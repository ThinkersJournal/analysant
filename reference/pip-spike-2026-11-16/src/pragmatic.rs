//! Slice 1: pragmatic classification (the precision bottleneck).
//!
//! The spike's directed-but-untasked check was drowned by extraction
//! noise: most "absent directives" are crate descriptions
//! (`**fuel-core-types**: dtypes, shapes, ...`) and most "absent named
//! capabilities" are status notes (`As-built status (2026-07-29)`) and
//! rules (`Rule 1: dependencies flow downward only`). Telling a directive
//! from a description is the pragmatic lens: what the text is *doing*,
//! not what it says. This module is deterministic — lexicons plus
//! structure, no ML — so every classification explains itself: the rule
//! that fired is the reason. Lexicons are data in this file (the same
//! seam as the alias table); growing them is data work, and it rots like
//! any data.
//!
//! Order matters: status first (a dated retraction is a status note
//! even when it names a capability), then directive signals, then
//! description signals, then the default. For list items the default is
//! Directive — a bare item in architecture docs is presumed directed,
//! and recall is bought there; precision comes from the positive
//! description/status signals. For bold/table names the default is Name.

use crate::claim::{Claim, ClaimKind, PragmaticRole};

/// A claim counts toward the directed-but-untasked report only when its
/// pragmatic role matches its kind: directives must actually direct,
/// named capabilities must actually name. Descriptions and status notes
/// are set aside (counted, not listed). Identifiers are code facts and
/// always count.
pub fn is_reportable(claim: &Claim) -> bool {
    match claim.kind {
        ClaimKind::Directive => claim.role == PragmaticRole::Directive,
        ClaimKind::Named => claim.role == PragmaticRole::Name,
        ClaimKind::Identifier => true,
        ClaimKind::Assertion => false,
    }
}

/// Classify a list item: Directive, Description, or StatusNote.
pub fn classify_list_item(text: &str, section: &str) -> PragmaticRole {
    let text = text.trim();
    if is_status_note(text) {
        return PragmaticRole::StatusNote;
    }
    if is_directive(text) {
        return PragmaticRole::Directive;
    }
    if is_description_item(text, section) {
        return PragmaticRole::Description;
    }
    PragmaticRole::Directive
}

/// Classify a bold phrase or table first-cell: Name, Description, or
/// StatusNote. Slice 2: a phrase that reads as a sentence, a clause,
/// or a property statement is a fragment wearing a name's shape, so it
/// is a Description, not a Name (see `names::fragment_reason`).
pub fn classify_name(text: &str, section: &str) -> PragmaticRole {
    let text = text.trim();
    if is_status_note(text) {
        return PragmaticRole::StatusNote;
    }
    if is_sentence_or_rule(text) || is_descriptive_section(section) {
        return PragmaticRole::Description;
    }
    if crate::names::fragment_reason(text).is_some() {
        return PragmaticRole::Description;
    }
    PragmaticRole::Name
}

/// Status markers: dated, as-built, retired, superseded, gone, "no
/// longer". A dated retraction names a capability only to say it is not
/// there (or not there any more) — it is not a directive to build it.
const STATUS_MARKERS: &[&str] = &[
    "as-built",
    "as built",
    "no longer",
    "retired",
    "superseded",
    "deleted",
    "is gone",
    "gone",
    "struck",
    "survived",
    "currently",
    "was a ",
    "was the ",
    "was deleted",
    "listed here as current until",
];

/// Decision-outcome markers (slice 4, genre handling): the past
/// participle of a decision — rejected, withdrawn, declined. A claim
/// recording one is a status note about a decision, not a directive.
/// "superseded" is already a status marker above and stays there;
/// the slice-1 weakness (it also fires on domain vocabulary,
/// "superseded nodes are left orphaned") is preserved, not fixed here.
///
/// The marker must be a standalone word in the prose, not a
/// hyphenated compound and not code: "hard-rejected when nonzero"
/// is what validate() does to bad input, `` `Rejected` `` is an enum
/// variant, and the bare verb in "verify→adopt/reject" is a process
/// step, not an outcome — none of those is a decision. The section
/// header is deliberately NOT checked: both outcome-sections in the
/// log contain live design (the derived-weights variant under
/// "save-all-activations rejected", the invariance-declaration
/// replacement under "memoization is DECLINED"), so a header is not a
/// verdict on everything under it.
const OUTCOME_MARKERS: &[&str] = &["rejected", "withdrawn", "declined"];

fn is_status_note(text: &str) -> bool {
    let lower = text.to_lowercase();
    if STATUS_MARKERS.iter().any(|m| contains_phrase(&lower, m)) {
        return true;
    }
    if has_decision_outcome(&lower) {
        return true;
    }
    has_date(&lower)
}

/// True when the (lowercased) text records a decision outcome: one of
/// OUTCOME_MARKERS as a standalone whitespace-delimited word, outside
/// backtick spans. Backtick spans are code, and code is another
/// language — a `` `Rejected` `` variant is not a decision. Splitting
/// on whitespace (not on every non-alphanumeric) keeps hyphenated
/// compounds whole, so "hard-rejected" never matches "rejected".
fn has_decision_outcome(lower: &str) -> bool {
    let prose = strip_backtick_spans(lower);
    prose.split_whitespace().any(|token| {
        let word = token.trim_matches(|c: char| !c.is_alphanumeric());
        OUTCOME_MARKERS.contains(&word)
    })
}

/// Remove `` `...` `` spans (matched pairs; an unmatched backtick
/// leaves the rest of the text as-is). Used so code tokens cannot
/// trigger a prose lexicon. Shared with the contradiction lens
/// (slice 5), which strips code before reading polarity.
pub(crate) fn strip_backtick_spans(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        match after_open.find('`') {
            Some(close) => rest = &after_open[close + 1..],
            None => {
                out.push_str(&rest[open..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Word-boundary phrase match: both sides split into alphanumeric
/// words, then a contiguous-subsequence check. Plain substring matching
/// misfires here — "shall" inside "shallow", "gone" inside "undergone"
/// — and the misfire is silent, so every lexicon lookup goes through
/// this helper.
pub(crate) fn contains_phrase(lower: &str, phrase: &str) -> bool {
    let hay: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let needle: Vec<&str> = phrase
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len())
        .any(|window| window == needle.as_slice())
}

/// True when the text contains a YYYY-MM-DD date. Shared with the
/// contradiction lens (slice 5): a dated heading is a record, not a
/// position, so it is temporal.
pub(crate) fn has_date(lower: &str) -> bool {
    let chars: Vec<char> = lower.chars().collect();
    chars.windows(10).any(|w| {
        w[0].is_ascii_digit()
            && w[1].is_ascii_digit()
            && w[2].is_ascii_digit()
            && w[3].is_ascii_digit()
            && w[4] == '-'
            && w[5].is_ascii_digit()
            && w[6].is_ascii_digit()
            && w[7] == '-'
            && w[8].is_ascii_digit()
            && w[9].is_ascii_digit()
    })
}

/// Imperative verbs that open a directive, and modal verbs that mark
/// one. The first content word of the item (after any leading bold/code
/// span, checkbox, or TODO marker) is checked against the imperative
/// list; modals are searched anywhere in the item.
pub(crate) const IMPERATIVES: &[&str] = &[
    "add",
    "allow",
    "apply",
    "audit",
    "authenticate",
    "authorize",
    "avoid",
    "benchmark",
    "branch",
    "build",
    "cache",
    "calculate",
    "cancel",
    "capture",
    "check",
    "choose",
    "clone",
    "commit",
    "compare",
    "compact",
    "compute",
    "confirm",
    "convert",
    "copy",
    "create",
    "decide",
    "decode",
    "decrypt",
    "define",
    "delegate",
    "deny",
    "derive",
    "design",
    "dispatch",
    "document",
    "emit",
    "encode",
    "encrypt",
    "enforce",
    "ensure",
    "evaluate",
    "evict",
    "execute",
    "expose",
    "filter",
    "fork",
    "free",
    "freeze",
    "generate",
    "grant",
    "group",
    "guard",
    "handle",
    "harvest",
    "implement",
    "index",
    "infer",
    "inspect",
    "introduce",
    "invalidate",
    "keep",
    "land",
    "load",
    "lock",
    "log",
    "make",
    "map",
    "measure",
    "merge",
    "model",
    "move",
    "normalize",
    "observe",
    "optimize",
    "parse",
    "pause",
    "persist",
    "pick",
    "plan",
    "prefer",
    "produce",
    "profile",
    "provide",
    "prune",
    "publish",
    "rank",
    "read",
    "record",
    "refresh",
    "reject",
    "reload",
    "remove",
    "rename",
    "represent",
    "require",
    "resolve",
    "resume",
    "retain",
    "retry",
    "return",
    "revert",
    "revoke",
    "route",
    "run",
    "save",
    "schedule",
    "select",
    "serialize",
    "ship",
    "sign",
    "sort",
    "split",
    "start",
    "stop",
    "store",
    "support",
    "test",
    "trace",
    "track",
    "translate",
    "treat",
    "use",
    "validate",
    "verify",
    "write",
];

const MODALS: &[&str] = &[
    "must",
    "should",
    "shall",
    "needs to",
    "need to",
    "has to",
    "have to",
    "required to",
    "must not",
    "should not",
];

fn is_directive(text: &str) -> bool {
    let lower = text.to_lowercase();
    if lower.starts_with("todo") || lower.starts_with("[ ]") || lower.starts_with("[x]") {
        return true;
    }
    if MODALS.iter().any(|m| contains_phrase(&lower, m)) {
        return true;
    }
    // First content word, after a leading bold/code span, checkbox, or
    // TODO marker. A leading bold span that is followed by ':' or '.' is
    // a definition head, not an imperative — handled by the description
    // rules, so only strip the span when it is not a definition head.
    let stripped = strip_leading_span(&lower);
    let first = stripped.split_whitespace().next().unwrap_or("");
    let first = first.trim_matches(|c: char| !c.is_alphanumeric());
    IMPERATIVES.contains(&first)
}

/// Strip one leading `**...**` or `` `...` `` span, unless it is a
/// definition head (followed by ':' or '.' plus more text, a span whose
/// own text ends in '.' — `**Side-table metadata.** Information ...` —
/// or a parenthetical gloss — `**Shared** (weights ...)`, but not a
/// `(see ...)` cross-reference). The gloss's first word is not the
/// item's first word.
fn strip_leading_span(lower: &str) -> &str {
    let text = lower.trim_start();
    for (open, close) in [("**", "**"), ("`", "`")] {
        if let Some(rest) = text.strip_prefix(open) {
            if let Some(end) = rest.find(close) {
                if rest[..end].trim_end().ends_with('.') {
                    return text; // period-terminated bold term: a definition head
                }
                let after = rest[end + close.len()..].trim_start();
                if after.starts_with(':') || after.starts_with('.') {
                    return text; // definition head: leave it for the description rules
                }
                if let Some(inner) = after.strip_prefix('(').map(str::trim_start) {
                    if !inner.to_lowercase().starts_with("see ") && !inner.is_empty() {
                        return text; // parenthetical gloss: a definition head
                    }
                }
                return after;
            }
        }
    }
    text
}

/// Sections that describe rather than direct: inventories, cross-refs,
/// and "where it lives" maps. An item under one of these is a
/// description of the design, not a directive to build something.
const DESCRIPTIVE_SECTIONS: &[&str] = &[
    "see also",
    "where this lives",
    "crate boundaries",
    "what lives where",
    "what's not in",
    "quick reference",
];

fn is_descriptive_section(section: &str) -> bool {
    let lower = section.to_lowercase();
    DESCRIPTIVE_SECTIONS.iter().any(|s| lower.contains(s))
}

fn is_description_item(text: &str, section: &str) -> bool {
    if is_descriptive_section(section) {
        return true;
    }
    // A link-led item is a cross-reference, not a directive.
    if text.trim_start().starts_with('[') {
        return true;
    }
    // `Rule N: ...` states a rule; it does not direct work.
    if text.trim_start().to_lowercase().starts_with("rule ") {
        return true;
    }
    // Definition head: a leading bold/code span (or a leading `Name:`)
    // followed by ':' or '.' and then gloss prose.
    if is_definition_head(text) {
        return true;
    }
    // `The X is Y` / `Each X is Y`: a copula sentence states what
    // something is, it does not direct work. (Modal items were already
    // caught as directives above, so "The runtime must ..." is safe.)
    if is_copula_opening(text) {
        return true;
    }
    false
}

/// True when the text opens with an article/demonstrative/quantifier
/// and states what something is within the first eight words.
fn is_copula_opening(text: &str) -> bool {
    let lower = text.trim_start().to_lowercase();
    const OPENERS: &[&str] = &[
        "the ", "a ", "an ", "this ", "these ", "that ", "each ", "every ",
    ];
    if !OPENERS.iter().any(|o| lower.starts_with(o)) {
        return false;
    }
    lower
        .split_whitespace()
        .take(8)
        .any(|w| w == "is" || w == "are")
}

/// True when the text opens with a term followed by ':' or '.' and more
/// prose: `**fuel-core-types**: dtypes, ...`, `**Side-table metadata.**
/// Information ...` (the '.' sits inside the bold span), `Rule 1:
/// dependencies ...`, `**Shared** (weights/constants ...): one storage
/// ...` (a parenthetical gloss after the term — but not `(see ...)`,
/// which is a cross-reference, not a gloss). A bold span ending in '.'
/// is read as a defined term plus gloss; an imperative hidden inside
/// such a span would be misread, which the set-aside listing makes
/// auditable.
fn is_definition_head(text: &str) -> bool {
    let text = text.trim_start();
    for (open, close) in [("**", "**"), ("`", "`")] {
        if let Some(rest) = text.strip_prefix(open) {
            if let Some(end) = rest.find(close) {
                if rest[..end].trim_end().ends_with('.')
                    && !rest[end + close.len()..].trim().is_empty()
                {
                    return true;
                }
                let after = rest[end + close.len()..].trim_start();
                if (after.starts_with(':') || after.starts_with('.')) && after.len() > 1 {
                    return true;
                }
                // Parenthetical gloss: `**Term** (gloss ...)` — but a
                // `(see ...)` parenthetical is a cross-reference.
                if let Some(inner) = after
                    .strip_prefix('(')
                    .map(|s| s.trim_start().to_lowercase())
                {
                    if !inner.starts_with("see ") && !inner.is_empty() {
                        return true;
                    }
                }
            }
        }
    }
    // Bare `Name: gloss` — the first token ends with ':'.
    if let Some(first) = text.split_whitespace().next() {
        if first.ends_with(':') && text.split_whitespace().count() > 1 {
            return true;
        }
    }
    false
}

/// Sentence markers: the bold phrase is a clause, not a name. A finite
/// verb (`is`, `are`, `cannot`, ...) or a rule/temporal opening means
/// the span asserts something rather than naming a capability.
const SENTENCE_MARKERS: &[&str] = &[
    " is ",
    " are ",
    " was ",
    " were ",
    " cannot ",
    " can't ",
    " does ",
    " do not ",
    " doesn't ",
    " will ",
    " would ",
    " has ",
    " have ",
    " must ",
    " should ",
];

const SENTENCE_OPENINGS: &[&str] = &[
    "rule ",
    "that is",
    "this is",
    "before ",
    "between ",
    "after ",
    "during ",
    "until ",
    "when ",
    "if ",
    "while ",
    "not at all",
];

fn is_sentence_or_rule(text: &str) -> bool {
    let lower = format!(" {} ", text.to_lowercase());
    if SENTENCE_MARKERS.iter().any(|m| lower.contains(m)) {
        return true;
    }
    let trimmed = text.trim_start().to_lowercase();
    if SENTENCE_OPENINGS.iter().any(|o| trimmed.starts_with(o)) {
        return true;
    }
    if is_definition_head(text) {
        return true;
    }
    false
}
