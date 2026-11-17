//! Slice 2: capability-name extraction (the named-list noise).
//!
//! Slice 1 left the named list at 64 absent, roughly half fragment
//! noise: sentences, verb-led fragments, versioned statements, and
//! property statements wearing a capability name's shape. A capability
//! name is a noun phrase naming a thing the system has or does
//! ("Pattern harvesting", "lookahead window", "Cache management"); a
//! fragment is a sentence, a clause, or a property statement
//! ("The pick now runs inside the executor", "populated at startup",
//! "v1.3 refines landed-vs-intent ..."). This module tells them apart
//! deterministically — lexicons plus structure, no ML — and every
//! verdict names the rule that fired, so the classification explains
//! itself. Lexicons are data tuned to this corpus; they rot like any
//! data, and the finite-verb list holds only verbs actually observed
//! plus auxiliaries — padding it with unobserved verbs would be
//! speculative data.
//!
//! The check runs on the text with parentheticals stripped: a
//! parenthetical is a tag or gloss ("(keep-per-device)", "(default)"),
//! not the phrase's grammar, so a verb inside one must not fire
//! ("crowding-distance cap (keep-per-device)" is a name, and
//! "Shallow (just-in-time)" is a mode, not a sentence).
//!
//! Known limit, stated plainly: a bare noun-phrase property statement
//! ("hardware-independent and portable", "recipe in two inverse
//! directions — both mandatory") is indistinguishable from a name
//! without semantics. Those survive as names and are listed as known
//! misses in the report. The one exception the rules do catch is a
//! period-terminated phrase with a temporal preposition ("Immutable
//! nodes after creation."): the period marks it as a written sentence,
//! and the temporal preposition marks it as a property in time, not a
//! coined term. A bare "-ing" opening is checked only on a standalone
//! first token, never inside a hyphenated compound: "crowding-distance"
//! and "user-facing" are nouns, "Loading" is a gerund.

use crate::pragmatic::{contains_phrase, IMPERATIVES};

/// The rule that makes a named claim a fragment, if any. `None` means
/// the text reads as a capability name.
pub fn fragment_reason(text: &str) -> Option<&'static str> {
    let stripped = strip_parentheticals(text);
    let lower = stripped.to_lowercase();
    let words = split_words(&lower);
    if words.is_empty() {
        return None;
    }
    if is_version_opening(&words) {
        return Some("version-opening");
    }
    if lower.contains('→') || lower.contains("->") {
        return Some("arrow");
    }
    if is_quantifier_opening(&words) {
        return Some("quantifier-opening");
    }
    if is_verb_opening(&lower, &words) {
        return Some("verb-opening");
    }
    if FINITE_VERBS.iter().any(|v| contains_phrase(&lower, v)) {
        return Some("finite-verb");
    }
    if contains_phrase(&lower, "not yet") {
        return Some("not-yet");
    }
    if is_per_fragment(&words) {
        return Some("per-fragment");
    }
    if has_predicative_participle(&words) {
        return Some("predicative-participle");
    }
    if ends_with_temporal_tail(&words) {
        return Some("temporal-tail");
    }
    if is_sentence_period(&lower, &words) {
        return Some("sentence-period");
    }
    None
}

/// Remove `(...)` groups (non-nested in this corpus, but depth-tracked
/// anyway). The parenthetical is a tag or gloss, not grammar.
fn strip_parentheticals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth: usize = 0;
    for ch in text.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {
                if depth == 0 {
                    out.push(ch);
                }
            }
        }
    }
    out
}

/// Words of a lowercased string: split on anything non-alphanumeric.
fn split_words(lower: &str) -> Vec<&str> {
    lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect()
}

/// First whitespace token, lowercased, stripped of edge punctuation.
fn first_token(lower: &str) -> &str {
    lower
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric())
}

/// True when the text opens with a version marker (`v1.3 ...`).
fn is_version_opening(words: &[&str]) -> bool {
    let first = words.first().copied().unwrap_or("");
    first.len() >= 2
        && first.starts_with('v')
        && first
            .chars()
            .nth(1)
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
}

/// Openings that quantify or negate rather than name: "few", "only",
/// "no", "not", "never", "none" — or a lone adverb ("lazily").
fn is_quantifier_opening(words: &[&str]) -> bool {
    let first = words.first().copied().unwrap_or("");
    matches!(first, "no" | "few" | "only" | "not" | "never" | "none")
        || (words.len() == 1 && first.ends_with("ly"))
}

/// True when the text opens with a verb. Three shapes, each with a
/// guard against verb-words used as nouns (architecture docs are full
/// of them: "Cache management", "Profile data"):
/// - a participle ("populated at startup", "retained as ...") — fires
///   unconditionally; a participle opening a name ("Bounded") is a
///   different word, so word-boundary matching keeps it safe.
/// - a standalone gerund ("Loading a .fuel ...") — fires only on a
///   standalone first token, never inside a hyphenated compound
///   ("crowding-distance", "user-facing" are nouns).
/// - an imperative — fires only when the second word is an article
///   ("keep the per-device ...": a clause with an object) or when the
///   first token is a hyphenated verb phrase with no participle segment
///   ("write-new-file-and-swap"). An imperative followed directly by a
///   noun ("Cache management", "Profile data") is a compound name, not
///   a clause, so it does not fire. An imperative followed by an
///   adjective ("orchestrate eager collectives above") is a clause the
///   rules cannot tell from a compound, so it survives as a known miss.
fn is_verb_opening(lower: &str, words: &[&str]) -> bool {
    let token = first_token(lower);
    if token.is_empty() {
        return false;
    }
    let segments: Vec<&str> = token.split('-').collect();
    let first = segments[0];
    if PARTICIPLES.contains(&first) {
        return true;
    }
    if segments.len() == 1 {
        if first.ends_with("ing") {
            return true;
        }
        if IMPERATIVES.contains(&first) {
            let second = words.get(1).copied().unwrap_or("");
            return matches!(second, "the" | "a" | "an");
        }
        return false;
    }
    // Hyphenated first token: a verb phrase only if the first segment
    // is an imperative and no later segment is a participle
    // ("build-time-closed" is a compound adjective, not a command).
    IMPERATIVES.contains(&first) && !segments[1..].iter().any(|s| PARTICIPLES.contains(s))
}

/// Finite verbs: auxiliaries plus verbs actually observed in the
/// corpus's fragment noise. Word-boundary matched, so "resolution"
/// does not fire on "resolve" (which is not listed anyway) and
/// "cannot" does not fire on "can".
const FINITE_VERBS: &[&str] = &[
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "do",
    "does",
    "did",
    "don't",
    "doesn't",
    "can't",
    "cannot",
    "will",
    "would",
    "has",
    "have",
    "had",
    "must",
    "should",
    "shall",
    "can",
    "could",
    "may",
    "might",
    "need",
    "needs",
    "runs",
    "ships",
    "refines",
    "salvages",
    "changes",
    "changed",
    "live",
    "diverge",
    "reconverge",
];

/// Prepositions, for the per-fragment and predicative-participle rules.
const PREPOSITIONS: &[&str] = &[
    "at", "in", "on", "for", "with", "per", "from", "to", "by", "as", "after", "before", "during",
    "into", "over", "under", "of",
];

/// True when the text opens with "per" and reads as a fragment rather
/// than a quantity: the word after "per" is a participle or gerund
/// ("per ending device"), or a second preposition follows ("per
/// decision point with merged path sets", "per-decision-point during
/// the dispatch walk"). "per-tier free memory" is a name: "tier" is a
/// noun and no second preposition follows.
fn is_per_fragment(words: &[&str]) -> bool {
    if words.first().copied().unwrap_or("") != "per" || words.len() < 2 {
        return false;
    }
    let second = words[1];
    if second.ends_with("ing") || PARTICIPLES.contains(&second) {
        return true;
    }
    words[2..].iter().any(|w| PREPOSITIONS.contains(w))
}

/// Past participles (regular and irregular), for the verb-opening and
/// predicative-participle rules. Word-boundary matched, so "Bounded"
/// (a mode name) does not fire on "bound".
const PARTICIPLES: &[&str] = &[
    "populated",
    "retained",
    "refined",
    "changed",
    "written",
    "loaded",
    "stored",
    "cached",
    "bound",
    "mapped",
    "built",
    "removed",
    "deleted",
    "closed",
    "prefixed",
    "backed",
    "orchestrated",
    "capped",
    "merged",
    "resolved",
    "persisted",
    "salvaged",
    "landed",
    "realized",
    "dispatched",
    "optimized",
    "compiled",
    "generated",
    "derived",
    "serialized",
    "registered",
    "fused",
    "pruned",
    "ranked",
    "sorted",
    "filtered",
    "indexed",
    "profiled",
    "traced",
    "tracked",
    "measured",
    "validated",
    "verified",
    "tested",
    "shipped",
    "published",
    "frozen",
    "locked",
    "paused",
    "resumed",
    "stopped",
    "started",
    "scheduled",
    "selected",
    "picked",
    "planned",
    "designed",
    "defined",
    "documented",
    "implemented",
    "introduced",
    "invalidated",
    "granted",
    "denied",
    "revoked",
    "signed",
    "encrypted",
    "decrypted",
    "encoded",
    "decoded",
    "converted",
    "cloned",
    "committed",
    "split",
    "moved",
    "copied",
    "renamed",
    "kept",
    "given",
    "taken",
    "made",
    "done",
    "gone",
    "seen",
    "known",
    "used",
];

/// True when a participle is used predicatively: a standalone token
/// followed by a preposition ("Profile data refined for cells",
/// "Memory-mapped at startup") or standing last ("Backend
/// kernel-revision hash changed"). An attributive participle before
/// its head noun ("The persisted graph", "The cached resolved route",
/// "Fuel-orchestrated JIT registration") has a noun after it, so it
/// does not fire.
fn has_predicative_participle(words: &[&str]) -> bool {
    for (i, w) in words.iter().enumerate() {
        if PARTICIPLES.contains(w) {
            let is_last = i + 1 == words.len();
            let next_is_prep = !is_last && PREPOSITIONS.contains(&words[i + 1]);
            if is_last || next_is_prep {
                return true;
            }
        }
    }
    false
}

/// Temporal tails: a phrase ending in one of these is a property in
/// time, not a name ("Memory-mapped at startup", "re-bound each
/// pass", "silent op substitution mid-realize").
const TEMPORAL_TAILS: &[&[&str]] = &[
    &["at", "startup"],
    &["each", "pass"],
    &["per", "token"],
    &["per", "input"],
    &["mid", "realize"],
    &["at", "dispatch"],
    &["at", "runtime"],
    &["per", "pass"],
    &["at", "finalize"],
    &["at", "realize"],
];

fn ends_with_temporal_tail(words: &[&str]) -> bool {
    TEMPORAL_TAILS
        .iter()
        .any(|tail| words.len() >= tail.len() && words[words.len() - tail.len()..] == **tail)
}

/// Temporal prepositions, for the sentence-period rule.
const TEMPORAL_PREPOSITIONS: &[&str] = &["after", "before", "during", "until", "when", "while"];

/// True when the text ends with a period and contains a temporal
/// preposition: a written sentence about a property in time
/// ("Immutable nodes after creation."), not a coined term. A
/// period-terminated name without one ("Path resolution at decision
/// points.", "Side-table metadata.") stays a name.
fn is_sentence_period(lower: &str, words: &[&str]) -> bool {
    lower.trim_end().ends_with('.') && words.iter().any(|w| TEMPORAL_PREPOSITIONS.contains(w))
}
