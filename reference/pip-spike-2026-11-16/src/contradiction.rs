//! Slice 5: the doc-vs-doc contradiction lens.
//!
//! The directed-but-untasked check cannot see a conflict: it only asks
//! whether the ROADMAP mentions a capability. Two architecture documents
//! can disagree with each other — one says a capability is not
//! supported, another requires it — and no ROADMAP row is involved.
//! The first case is the audit's §6 item 7: `06` "cancellation not
//! supported" vs `15` C-2 "can cancel".
//!
//! A contradiction is a PAIR of statements about the same capability
//! where one side negates it and the other asserts it. Statements come
//! from two places, because the prose parser alone cannot see either
//! side of the headline case:
//!
//! * Headings: a document's own position ("Cancellation is not
//!   supported"). The C-2 paragraph ("can cancel in flight") yields no
//!   claim under the prose parser — no list item, no bold span — so
//!   without headings the positive side would not exist.
//! * Extracted claims: everything except code identifiers (a code
//!   observation, not an assertion) and status notes (a retraction is
//!   temporal, not a position).
//!
//! Polarity is deterministic — lexicons plus structure, no ML — in
//! this order: temporal markers first ("not yet", dates, retractions),
//! then negation markers ("not supported", "cannot", "no <capability>"),
//! else positive. Every verdict names the rule that fired. Three
//! scoping rules keep polarity honest, all learned from the first
//! v8 run's 722 mostly-noise pairs:
//!
//! * The head window (claims only): polarity is read from the first
//!   16 tokens of the claim text — the head plus the first clause. A
//!   negation deep in a 200-word gloss ("never silent", "no
//!   materialized attention matrix") does not govern the head's
//!   capability. Headings use the full title; they are short.
//! * Negation of a bad thing is positive: "does not break", "never
//!   panics", "never silent" assert the absence of something bad, not
//!   the absence of a capability. When the word after a negation
//!   marker is in BAD_WORDS, the statement is Positive.
//! * Contrast split (headings only): "Dispatch: runs, not single
//!   ops" asserts runs and denies single ops, so one heading yields
//!   two statements. Claims do NOT split — ", not " inside a long
//!   gloss is contrastive prose, not a position, and splitting claims
//!   was the largest single noise source in the first v8 run (399
//!   split-half sides).
//!
//! Pairing is deliberately strict, because sharing a word is not a
//! contradiction:
//!
//! * Different files (a document contradicting itself is a different
//!   finding, not reported here).
//! * Opposite polarity: Negated vs Positive. Temporal statements never
//!   pair — "must support X" vs "X not yet built" is
//!   directed-but-untasked, not a contradiction.
//! * The smaller subject must be family-contained in the larger one:
//!   every token of the smaller subject has a same-family token in the
//!   larger subject. "Tolerance budgets are not supported" vs
//!   "tolerance configuration" shares "tolerance" but neither contains
//!   the other — budgets are not configuration — so it does not pair.
//! * The shared tokens must be strong enough: at least one token of 6+
//!   letters, or at least two tokens. A single short token ("graph",
//!   "runs", "crate") cannot carry a contradiction.
//! * The positive side must have at least two subject tokens: a bare
//!   name ("accuracy", "storage") asserts almost nothing, and
//!   single-token positive sides were a major false-pair source.
//! * Pairing stopwords are removed from subjects first: stance words
//!   ("supported", "not"), light verbs ("use", "make"), structural
//!   words ("fuel", "crate", "document", "contract", "backend"), and
//!   document-state words ("verified", "settled") identify nothing, so
//!   "X is not supported" vs "Y is supported" share nothing to pair
//!   on. Tokens containing digits ("qwen35", "conv2d") are dropped
//!   too — they are versions and identifiers, not capabilities.
//!
//! Known limits, stated plainly: negation scope is statement-level,
//! not token-level — a gloss that negates Z while the head names X is
//! read as a negation of X when the marker falls inside the head
//! window. "cancel" vs "cancellation" meet by the family rule below;
//! truly different words for one capability (a synonym the alias table
//! does not hold) do not meet at all; roots shorter than 6 letters
//! with a consonant mutation ("admit" / "admission") do not meet.

use crate::claim::{Claim, ClaimKind, PragmaticRole};
use crate::normalize::{content_words, normalize, STOPWORDS};
use crate::pragmatic::{contains_phrase, has_date, strip_backtick_spans};
use std::collections::HashMap;

/// What a statement does about its capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Asserts, requires, or provides the capability.
    Positive,
    /// Denies the capability ("not supported", "cannot", "no X").
    Negated,
    /// About time, not truth: "not yet", dated, retracted. Never pairs.
    Temporal,
}

/// Where a statement came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementSource {
    /// A heading: the document's own position.
    Heading,
    /// An extracted claim, of this kind.
    Claim(ClaimKind),
}

/// One side of a possible contradiction: a position a document takes
/// about a capability, with stable identity (file:line, plus a
/// `#assert` / `#deny` suffix when one line splits in two).
#[derive(Debug, Clone)]
pub struct Statement {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub text: String,
    pub section: String,
    /// The capability the statement is about: content tokens of the
    /// claim's key (or the heading's title), pairing stopwords
    /// removed. Never empty — a statement with no subject is dropped.
    pub subject: Vec<String>,
    pub polarity: Polarity,
    /// The marker that fired for a Negated (or Temporal) statement:
    /// a lexicon phrase, or "contrast" for the denied half of an
    /// "X, not Y" heading. None for Positive statements.
    pub negated_by: Option<String>,
    pub source: StatementSource,
}

/// A contradiction: two statements, different files, opposite
/// polarity, about the same capability (the smaller subject
/// family-contained in the larger one).
#[derive(Debug, Clone)]
pub struct Contradiction {
    pub negated: Statement,
    pub positive: Statement,
    /// The shared family tokens, in the smaller subject's order.
    pub shared: Vec<String>,
    /// The marker that fired on the negated side.
    pub negated_by: Option<String>,
}

/// Polarity of a text, with the marker that fired. Temporal first:
/// "not yet landed" contains "not" but is a deferral, not a denial;
/// a dated heading is a record, not a position. Then negation
/// markers — except when the marker negates a bad thing ("does not
/// break", "never panics"), which is a positive assertion, not a
/// capability denial.
pub fn polarity(text: &str) -> (Polarity, Option<&'static str>) {
    let lower = text.to_lowercase();
    let prose = strip_backtick_spans(&lower);
    if has_date(&prose) {
        return (Polarity::Temporal, Some("dated"));
    }
    for marker in TEMPORAL_PHRASES {
        if contains_phrase(&prose, marker) {
            return (Polarity::Temporal, Some(marker));
        }
    }
    for word in TEMPORAL_WORDS {
        if contains_standalone_word(&prose, word) {
            return (Polarity::Temporal, Some(word));
        }
    }
    // Token stream (split on non-alphanumerics) for phrase markers,
    // so the word after the marker can be checked for the bad-thing
    // flip. "can't" splits to [can, t], matching the phrase as
    // written.
    let tokens: Vec<&str> = prose
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    for marker in NEGATION_PHRASES {
        if let Some(end) = phrase_end(&tokens, marker) {
            if negates_bad_thing(tokens.get(end)) {
                return (Polarity::Positive, None);
            }
            return (Polarity::Negated, Some(marker));
        }
    }
    if let Some(next) = word_after_standalone(&prose, "never") {
        if BAD_WORDS.contains(&next.as_str()) {
            return (Polarity::Positive, None);
        }
        return (Polarity::Negated, Some("never"));
    }
    if let Some(word) = refused_capability(&prose) {
        if BAD_WORDS.contains(&word.as_str()) {
            return (Polarity::Positive, None);
        }
        return (Polarity::Negated, Some("no <capability>"));
    }
    (Polarity::Positive, None)
}

/// End index (one past the last matched token) of the first window
/// where `phrase` matches `tokens`, or None. Phrase tokens come
/// from splitting on non-alphanumerics, the same as `tokens`.
fn phrase_end(tokens: &[&str], phrase: &str) -> Option<usize> {
    let needle: Vec<&str> = phrase
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    if needle.is_empty() || needle.len() > tokens.len() {
        return None;
    }
    tokens
        .windows(needle.len())
        .position(|w| w == needle.as_slice())
        .map(|i| i + needle.len())
}

/// The word after a standalone whitespace-delimited occurrence of
/// `word`, trimmed of punctuation — or None when `word` does not
/// occur standalone. Whitespace splitting keeps hyphenated
/// compounds whole: "never-panics" has no standalone "never".
fn word_after_standalone(lower: &str, word: &str) -> Option<String> {
    let words: Vec<&str> = lower
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|t| !t.is_empty())
        .collect();
    for (i, w) in words.iter().enumerate() {
        if *w == word {
            return words.get(i + 1).map(|w| w.to_string());
        }
    }
    None
}

/// The capability a standalone "no" refuses, or None: the word
/// after the first standalone "no" that is a non-stopword other
/// than "longer" ("no longer" is temporal, checked earlier). A
/// hyphenated "no-op" is one whitespace token, not a refusal.
fn refused_capability(lower: &str) -> Option<String> {
    let words: Vec<&str> = lower
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|t| !t.is_empty())
        .collect();
    for pair in words.windows(2) {
        if pair[0] == "no" && !STOPWORDS.contains(&pair[1]) && pair[1] != "longer" {
            return Some(pair[1].to_string());
        }
    }
    None
}

/// True when the word after a negation marker names a bad thing:
/// negating it asserts its absence, which is positive ("does not
/// break", "never panics", "no failures"). `word` is None when the
/// marker ends the text.
fn negates_bad_thing(word: Option<&&str>) -> bool {
    match word {
        Some(w) => BAD_WORDS.contains(w),
        None => false,
    }
}

/// Bad things whose negation is a positive assertion: verbs and
/// participles of breaking, failing, panicking, corrupting, losing,
/// and the like, plus adjectives of absence and silence. "support"
/// is deliberately absent: "does not support" denies a capability.
const BAD_WORDS: &[&str] = &[
    "break",
    "breaks",
    "breaking",
    "broken",
    "breakage",
    "fail",
    "fails",
    "failing",
    "failed",
    "failure",
    "failures",
    "panic",
    "panics",
    "panicking",
    "panicked",
    "crash",
    "crashes",
    "crashing",
    "crashed",
    "corrupt",
    "corrupts",
    "corrupting",
    "corrupted",
    "corruption",
    "lose",
    "loses",
    "losing",
    "lost",
    "loss",
    "losses",
    "leak",
    "leaks",
    "leaking",
    "leaked",
    "deadlock",
    "deadlocks",
    "deadlocking",
    "deadlocked",
    "hang",
    "hangs",
    "hanging",
    "hung",
    "destroy",
    "destroys",
    "destroying",
    "destroyed",
    "damage",
    "damages",
    "damaging",
    "damaged",
    "violate",
    "violates",
    "violating",
    "violated",
    "violation",
    "violations",
    "degrade",
    "degrades",
    "degrading",
    "degraded",
    "silent",
    "silently",
    "quiet",
    "quietly",
    "invisible",
    "hidden",
    "missing",
    "absent",
    "wrong",
    "incorrect",
    "mistaken",
    "invalid",
    "stale",
    "outdated",
    "obsolete",
    "harmful",
    "dangerous",
    "unsafe",
];

/// Temporal markers: deferrals, retractions, and status reports.
/// Checked before negation — "not yet" and "no longer" both contain
/// a negation word, and neither denies a capability.
const TEMPORAL_PHRASES: &[&str] = &[
    "not yet",
    "yet to",
    "not landed",
    "not built",
    "not implemented",
    "not shipped",
    "not done",
    "not started",
    "no longer",
    "superseded",
    "retired",
    "is gone",
    "was deleted",
];

/// Single-word temporal markers, matched as standalone words (the
/// slice-4 discipline: "hard-rejected" is validation behaviour, not a
/// decision — and here a hyphenated compound is a property, not a
/// retraction).
const TEMPORAL_WORDS: &[&str] = &["withdrawn", "rejected", "declined"];

/// Negation markers: phrases that deny a capability, matched with
/// word boundaries (contains_phrase). "is not" covers the copula
/// case ("Cancellation is not supported"); "not supported" covers
/// the bare case. There is deliberately no bare "not" + word rule:
/// "not only" and "not just" assert, and a bare "not" governs an
/// unknown scope — the contrast split below handles ", not Y".
const NEGATION_PHRASES: &[&str] = &[
    "not supported",
    "is not",
    "are not",
    "was not",
    "were not",
    "isn't",
    "aren't",
    "wasn't",
    "weren't",
    "cannot",
    "can't",
    "can not",
    "could not",
    "does not",
    "do not",
    "doesn't",
    "don't",
    "did not",
    "not possible",
    "not available",
    "unsupported",
    "no support",
    "not allowed",
    "not permitted",
    "not required",
    "not needed",
];

/// True when `word` appears as a standalone whitespace-delimited word
/// in the (lowercased, backtick-stripped) text. Splitting on
/// whitespace — not on every non-alphanumeric — keeps hyphenated
/// compounds whole: "never-panics" is a property, "never panics" is
/// a negation; "hard-rejected" is validation behaviour.
fn contains_standalone_word(lower: &str, word: &str) -> bool {
    lower
        .split_whitespace()
        .any(|token| token.trim_matches(|c: char| !c.is_alphanumeric()) == word)
}

/// Two tokens name the same capability family: identical, differing
/// only by a trailing "s", or sharing a common prefix of 6+ letters
/// ("cancel" / "cancelling" / "cancellation" share "cancell";
/// "persist" / "persistence" share "persist"). The length floor is
/// what keeps the rule from overmerging short words: "state" and
/// "statement" share only 5 letters, so they are not a family. Known
/// miss, stated plainly: roots shorter than 6 letters with a
/// consonant mutation ("admit" / "admission" share 4) do not meet.
pub fn same_family(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if long.len() == short.len() + 1 && long.starts_with(short) && long.ends_with('s') {
        return true;
    }
    let common = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    common >= 6
}

/// Words that identify nothing, removed from subjects before pairing:
/// stance words (the polarity itself), light verbs, interrogatives
/// and numbers, structural/project words that appear everywhere, and
/// document-state words (a heading about the document's own state is
/// not a statement about a capability). Lexicons are data and rot
/// like any data — this list is the slice-5 instance of that.
const PAIRING_STOPWORDS: &[&str] = &[
    // Auxiliaries and modals.
    "do",
    "does",
    "did",
    "done",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "has",
    "have",
    "had",
    "will",
    "would",
    "can",
    "could",
    "shall",
    "should",
    "may",
    "might",
    "must",
    "ought",
    // Polarity words.
    "not",
    "no",
    "never",
    "cannot",
    "cant",
    "doesn",
    "don",
    "isn",
    "aren",
    "wasn",
    "weren",
    "didn",
    "couldn",
    "wouldn",
    "shouldn",
    "unsupported",
    "supported",
    "support",
    "supports",
    "supporting",
    "available",
    "required",
    "require",
    "requires",
    "requiring",
    "provides",
    "provide",
    "provided",
    "providing",
    // Light verbs.
    "get",
    "gets",
    "got",
    "make",
    "makes",
    "made",
    "take",
    "takes",
    "took",
    "put",
    "puts",
    "set",
    "sets",
    "give",
    "gives",
    "gave",
    "keep",
    "keeps",
    "kept",
    "use",
    "uses",
    "used",
    "using",
    "go",
    "goes",
    "went",
    "come",
    "comes",
    "came",
    "say",
    "says",
    "said",
    "see",
    "sees",
    "saw",
    "know",
    "knows",
    "known",
    "think",
    "thinks",
    "seem",
    "seems",
    "become",
    "becomes",
    "became",
    "exist",
    "exists",
    "existing",
    "cover",
    "covers",
    "covered",
    "name",
    "names",
    "named",
    "hold",
    "holds",
    "holding",
    "call",
    "calls",
    "called",
    "mean",
    "means",
    "meant",
    "help",
    "helps",
    "let",
    "lets",
    "show",
    "shows",
    "showed",
    "find",
    "finds",
    "found",
    "need",
    "needs",
    "needed",
    "want",
    "wants",
    "like",
    "likes",
    "work",
    "works",
    "worked",
    "happen",
    "happens",
    "include",
    "includes",
    "included",
    "contain",
    "contains",
    "supply",
    "supplies",
    "supplied",
    // Interrogatives, quantifiers, numbers, connectives.
    "what",
    "where",
    "when",
    "why",
    "how",
    "which",
    "who",
    "whom",
    "whose",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "first",
    "second",
    "third",
    "each",
    "every",
    "all",
    "any",
    "some",
    "none",
    "both",
    "either",
    "neither",
    "per",
    "via",
    "than",
    "then",
    "also",
    "only",
    "just",
    "even",
    "still",
    "yet",
    "already",
    "now",
    "today",
    // Structural and project words.
    "fuel",
    "architecture",
    "document",
    "documents",
    "section",
    "sections",
    "overview",
    "summary",
    "introduction",
    "notes",
    "note",
    "part",
    "parts",
    "chapter",
    "appendix",
    "version",
    "versions",
    "rule",
    "rules",
    "status",
    "consumer",
    "consumers",
    "contract",
    "contracts",
    "interface",
    "interfaces",
    "backend",
    "backends",
    "repository",
    "workspace",
    "crate",
    "crates",
    "file",
    "files",
    "line",
    "lines",
    "example",
    "examples",
    // Document-state words.
    "verified",
    "verify",
    "verifies",
    "settled",
    "settle",
    "settles",
    "covered",
    "decided",
    "decide",
    "decides",
    "measured",
    "measure",
    "measures",
    "documented",
    "asserted",
    "declared",
    "pinned",
    "authored",
];

/// The subject of a text: normalized content tokens, pairing
/// stopwords removed, single characters, pure numbers, version
/// tokens ("v0", "v1"), and any token containing a digit
/// ("qwen35", "conv2d" — versions and identifiers, not capabilities)
/// dropped, deduplicated in order.
pub fn subject_tokens(text: &str) -> Vec<String> {
    let norm = normalize(text);
    let mut out: Vec<String> = Vec::new();
    for word in content_words(&norm) {
        if word.chars().count() < 2 {
            continue;
        }
        if word.chars().any(|c| c.is_ascii_digit()) {
            continue;
        }
        if word.len() >= 2 && word.starts_with('v') && word[1..].chars().all(|c| c.is_ascii_digit())
        {
            continue;
        }
        if PAIRING_STOPWORDS.contains(&word) {
            continue;
        }
        if !out.iter().any(|w| w == word) {
            out.push(word.to_string());
        }
    }
    out
}

/// Split an "X, not Y" contrast into (asserted, denied). The comma
/// (or semicolon, or dash) before "not" is a clause boundary: the
/// heading asserts X and denies Y, and the two halves are separate
/// statements. Returns None when the text is temporal ("X, not yet
/// Y" is a deferral) or has no contrast separator. The split is
/// computed on the lowercased text and applied to the original at
/// the same byte offsets — lowercase is length-preserving for the
/// ASCII text and em-dashes in this corpus.
fn split_contrast(text: &str) -> Option<(String, String)> {
    if polarity(text).0 == Polarity::Temporal {
        return None;
    }
    let lower = text.to_lowercase();
    for sep in [", not ", "; not ", " — not ", " - not ", "— not "] {
        if let Some(pos) = lower.find(sep) {
            let asserted = text[..pos].trim().to_string();
            let denied = text[pos + sep.len()..].trim().to_string();
            if !asserted.is_empty() && !denied.is_empty() {
                return Some((asserted, denied));
            }
        }
    }
    None
}

/// The first `n` whitespace-delimited tokens of a text: the head
/// window for claim polarity. A claim's head states its position;
/// a negation deep in the gloss does not govern the head's
/// capability.
pub fn first_tokens(text: &str, n: usize) -> String {
    text.split_whitespace()
        .take(n)
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Statements from extracted claims: everything except code
/// identifiers (a code observation, not an assertion) and status
/// notes (a retraction is temporal, not a position). The subject
/// comes from the claim's key — the capability identity — while
/// polarity comes from the first 16 tokens of the text, the head
/// plus the first clause ("**X** — not supported on CUDA" is
/// Negated; a gloss that negates something else 200 words in is
/// not). Claims do NOT split on ", not ": contrastive prose inside
/// a long gloss is not a position — splitting claims was the
/// largest noise source in the first v8 run.
pub fn statements_from_claims(claims: &[Claim]) -> Vec<Statement> {
    let mut out = Vec::new();
    for claim in claims {
        if claim.kind == ClaimKind::Identifier || claim.role == PragmaticRole::StatusNote {
            continue;
        }
        let subject = subject_tokens(&claim.key);
        if subject.is_empty() {
            continue;
        }
        let (pol, marker) = polarity(&first_tokens(&claim.text, 16));
        out.push(Statement {
            id: claim.id.clone(),
            file: claim.source_file.clone(),
            line: claim.line,
            text: claim.text.clone(),
            section: claim.section.clone(),
            subject,
            polarity: pol,
            negated_by: marker.map(|m| m.to_string()),
            source: StatementSource::Claim(claim.kind.clone()),
        });
    }
    out
}

/// Statements from headings: a heading states the document's own
/// position. Same rules as claims (contrast split, empty subjects
/// dropped), with the heading's parent path as its section.
pub fn statements_from_headings(file: &str, text: &str) -> Vec<Statement> {
    let mut out = Vec::new();
    for heading in crate::parse::extract_headings(file, text) {
        if let Some((asserted, denied)) = split_contrast(&heading.title) {
            let (pol, marker) = polarity(&asserted);
            let subject = subject_tokens(&asserted);
            if !subject.is_empty() {
                out.push(Statement {
                    id: format!("{}:{}#assert", heading.file, heading.line),
                    file: heading.file.clone(),
                    line: heading.line,
                    text: asserted,
                    section: heading.section.clone(),
                    subject,
                    polarity: pol,
                    negated_by: marker.map(|m| m.to_string()),
                    source: StatementSource::Heading,
                });
            }
            let subject = subject_tokens(&denied);
            if !subject.is_empty() {
                out.push(Statement {
                    id: format!("{}:{}#deny", heading.file, heading.line),
                    file: heading.file.clone(),
                    line: heading.line,
                    text: denied,
                    section: heading.section.clone(),
                    subject,
                    polarity: Polarity::Negated,
                    negated_by: Some("contrast".to_string()),
                    source: StatementSource::Heading,
                });
            }
            continue;
        }
        let subject = subject_tokens(&heading.title);
        if subject.is_empty() {
            continue;
        }
        let (pol, marker) = polarity(&heading.title);
        out.push(Statement {
            id: format!("{}:{}", heading.file, heading.line),
            file: heading.file.clone(),
            line: heading.line,
            text: heading.title,
            section: heading.section,
            subject,
            polarity: pol,
            negated_by: marker.map(|m| m.to_string()),
            source: StatementSource::Heading,
        });
    }
    out
}

/// The smaller subject family-contained in the larger one, or None.
/// Returns the matched tokens, in the smaller subject's order. An
/// empty subject never matches.
fn shared_subject(a: &[String], b: &[String]) -> Option<Vec<String>> {
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if small.is_empty() {
        return None;
    }
    let mut shared = Vec::new();
    for tok in small {
        if large.iter().any(|t| same_family(tok, t)) {
            shared.push(tok.clone());
        } else {
            return None;
        }
    }
    Some(shared)
}

/// The shared tokens must be strong enough to carry a contradiction:
/// at least one token of 6+ letters, or at least two tokens. A
/// single short token ("graph", "runs") is not enough on its own.
fn strong_enough(shared: &[String]) -> bool {
    shared.iter().any(|t| t.chars().count() >= 6) || shared.len() >= 2
}

/// Find every contradiction among the statements: Negated vs
/// Positive, different files, smaller subject family-contained in
/// the larger one, shared tokens strong enough, and the positive
/// side carrying at least two subject tokens — a bare name
/// ("accuracy", "storage") asserts almost nothing. One pair per
/// (negated line, positive line) — if several claims on the same two
/// lines pair, the one sharing the most tokens wins. Output is
/// sorted by negated file:line, then positive file:line, so the run
/// is deterministic.
pub fn find_contradictions(statements: &[Statement]) -> Vec<Contradiction> {
    let mut best: HashMap<(String, usize, String, usize), Contradiction> = HashMap::new();
    for neg in statements
        .iter()
        .filter(|s| s.polarity == Polarity::Negated)
    {
        for pos in statements
            .iter()
            .filter(|s| s.polarity == Polarity::Positive && s.subject.len() >= 2)
        {
            if neg.file == pos.file {
                continue;
            }
            let Some(shared) = shared_subject(&neg.subject, &pos.subject) else {
                continue;
            };
            if !strong_enough(&shared) {
                continue;
            }
            let key = (neg.file.clone(), neg.line, pos.file.clone(), pos.line);
            let cand = Contradiction {
                negated: neg.clone(),
                positive: pos.clone(),
                shared,
                negated_by: neg.negated_by.clone(),
            };
            match best.get(&key) {
                Some(prev) if prev.shared.len() >= cand.shared.len() => {}
                _ => {
                    best.insert(key, cand);
                }
            }
        }
    }
    let mut out: Vec<Contradiction> = best.into_values().collect();
    out.sort_by(|a, b| {
        (
            &a.negated.file,
            a.negated.line,
            &a.positive.file,
            a.positive.line,
        )
            .cmp(&(
                &b.negated.file,
                b.negated.line,
                &b.positive.file,
                b.positive.line,
            ))
    });
    out
}
