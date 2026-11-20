// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant-pragmatic`: what a claim's text is *doing* (SPEC-STEP1 section 6, slice 2), as opposed
//! to what it says. A list item that begins "Build the graph" directs work; one that begins
//! "**fuel-core-types**: dtypes, shapes" describes a crate; one dated "As-built status (2026-07-29)"
//! reports. The lens is deterministic (word lists plus structure, no model), so every verdict names the
//! rule that fired, and that rule is the reason.
//!
//! The rules are the reference spike's (`reference/pip-spike-2026-11-16/src/pragmatic.rs` and
//! `names.rs`, `MIT OR Apache-2.0`, credited): status first (a dated retraction is a status note even
//! when it names a capability), then directive signals, then description signals, then the default. For
//! a list item the default is a directive (a bare item in a plan-like document is presumed directed);
//! for a name the default is a name. The word lists are the language pack's data (`pack.lexicons()`),
//! never this file's. The *shapes* the rules test (a leading bold span, a `Name:` head, a YYYY-MM-DD
//! date, a parenthetical) are Markdown-and-English conventions, recorded as such in SPEC-STEP1
//! section 9.
//!
//! The lens assigns [`Kind`]; core never does. It reads no plan and no other document.

use analysant_core::{Claim, Derivation, Form, Kind, LanguagePack, LensId, Span};

pub const LENS: &str = "pragmatic";

/// Lexicons the lens reads. A pack that does not carry them cannot be used with it.
const REQUIRED_LEXICONS: &[&str] = &[
    "status-markers",
    "outcome-markers",
    "imperatives",
    "modals",
    "descriptive-sections",
    "sentence-markers",
    "sentence-openings",
    "finite-verbs",
    "prepositions",
    "participles",
    "temporal-prepositions",
    "directive-prefixes",
    "description-openers",
    "copulas",
    "quantifier-openers",
    "temporal-tails",
    "rule-openers",
    "not-yet-phrases",
    "gerund-suffixes",
    "adverb-suffixes",
    "leading-articles",
];

/// Which rule groups run. Each group is one slice of the reference spike, kept separable so the oracle
/// can be reproduced stage by stage (run v4, then run v5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// A bold phrase or table cell that reads as a sentence, a clause or a property statement is a
    /// description, not a name (the spike's "fragment" rules, its slice 2).
    pub fragment_rules: bool,
    /// A claim recording the outcome of a decision (rejected, withdrawn, declined) is a status note, not
    /// a directive (the spike's genre handling, its slice 4). Off reproduces the runs before it (v6).
    pub decision_outcomes: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fragment_rules: true,
            decision_outcomes: true,
        }
    }
}

/// The pack does not carry what the lens needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingData(pub Vec<String>);

impl std::fmt::Display for MissingData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pack lacks data the pragmatic lens reads: {}",
            self.0.join(", ")
        )
    }
}

impl std::error::Error for MissingData {}

/// A verdict: the kind, and the rule and table entries that produced it. Never an empty rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Classification {
    pub kind: Kind,
    pub derivation: Derivation,
}

pub struct PragmaticLens<'a> {
    pack: &'a dyn LanguagePack,
    config: Config,
    copula_window: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Directive,
    Description,
    Status,
    Name,
}

impl Role {
    fn kind(self) -> Kind {
        match self {
            Role::Directive => Kind::Directive,
            Role::Description => Kind::Description,
            Role::Status => Kind::Report,
            Role::Name => Kind::Name,
        }
    }
}

struct Verdict {
    role: Role,
    rule: &'static str,
    entries: Vec<String>,
}

impl Verdict {
    fn new(role: Role, rule: &'static str) -> Self {
        Self {
            role,
            rule,
            entries: Vec::new(),
        }
    }

    fn with(role: Role, rule: &'static str, entry: impl Into<String>) -> Self {
        Self {
            role,
            rule,
            entries: vec![entry.into()],
        }
    }
}

impl<'a> PragmaticLens<'a> {
    pub fn new(pack: &'a dyn LanguagePack, config: Config) -> Result<Self, MissingData> {
        let lex = pack.lexicons();
        let mut missing: Vec<String> = REQUIRED_LEXICONS
            .iter()
            .filter(|n| lex.entries(n).is_empty())
            .map(|n| (*n).to_string())
            .collect();
        let copula_window = lex.param("copula_window");
        if copula_window.is_none() {
            missing.push("param copula_window".into());
        }
        if !missing.is_empty() {
            return Err(MissingData(missing));
        }
        Ok(Self {
            pack,
            config,
            copula_window: copula_window.unwrap_or(0),
        })
    }

    /// Classify one claim given the heading path it sits under (outermost first, joined by " > ").
    /// `None` for a form this lens has no rules for (it never guesses).
    pub fn classify(&self, claim: &Claim, section: &str) -> Option<Classification> {
        let verdict = match claim.form {
            Form::ListItem => self.list_item(&claim.text, section),
            Form::TableCell | Form::Bold => self.name(&claim.text, section),
            // A code identifier is a code fact: it names, and is always reportable.
            Form::CodeSpan => Verdict::new(Role::Name, "code identifier"),
            _ => return None,
        };
        Some(Classification {
            kind: verdict.role.kind(),
            derivation: Derivation {
                lens: LensId(LENS.to_string()),
                rule: verdict.rule.to_string(),
                table_entries: verdict.entries,
            },
        })
    }

    // ---- word lists and matching -------------------------------------------------------------

    fn list(&self, name: &str) -> &[String] {
        self.pack.lexicons().entries(name)
    }

    fn fold(&self, text: &str) -> String {
        self.pack.normalizer().fold_case(text)
    }

    fn norms(&self, folded: &str) -> Vec<String> {
        self.pack
            .tokenize(&Span::default(), folded)
            .into_iter()
            .map(|t| t.norm)
            .collect()
    }

    /// Word-boundary phrase match, never substring: "shall" must not hit "shallow", "gone" must not hit
    /// "undergone". Returns the first matching entry.
    fn phrase_in(&self, words: &[String], lexicon: &str) -> Option<String> {
        self.list(lexicon)
            .iter()
            .find(|entry| {
                let want = self.norms(&self.fold(entry));
                !want.is_empty()
                    && want.len() <= words.len()
                    && words.windows(want.len()).any(|w| w == want.as_slice())
            })
            .cloned()
    }

    fn in_list(&self, lexicon: &str, word: &str) -> bool {
        self.list(lexicon).iter().any(|e| e == word)
    }

    fn ends_with_any(&self, lexicon: &str, word: &str) -> bool {
        self.list(lexicon)
            .iter()
            .any(|s| word.ends_with(s.as_str()))
    }

    // ---- list items --------------------------------------------------------------------------

    fn list_item(&self, text: &str, section: &str) -> Verdict {
        let text = text.trim();
        if let Some(v) = self.status_note(text) {
            return v;
        }
        if let Some(v) = self.directive(text) {
            return v;
        }
        if let Some(v) = self.description_item(text, section) {
            return v;
        }
        Verdict::new(
            Role::Directive,
            "default: a bare list item is presumed directed",
        )
    }

    fn status_note(&self, text: &str) -> Option<Verdict> {
        let lower = self.fold(text);
        let words = self.norms(&lower);
        if let Some(m) = self.phrase_in(&words, "status-markers") {
            return Some(Verdict::with(Role::Status, "status marker", m));
        }
        if self.config.decision_outcomes && self.has_decision_outcome(&lower) {
            return Some(Verdict::new(Role::Status, "decision outcome"));
        }
        has_date(&lower).then(|| Verdict::new(Role::Status, "dated (YYYY-MM-DD)"))
    }

    /// An outcome word as a standalone whitespace-delimited word outside backtick spans: a code token
    /// (`Rejected`) is not a decision, and a hyphenated compound ("hard-rejected") stays whole.
    fn has_decision_outcome(&self, lower: &str) -> bool {
        strip_backtick_spans(lower).split_whitespace().any(|token| {
            let word = token.trim_matches(|c: char| !c.is_alphanumeric());
            self.in_list("outcome-markers", word)
        })
    }

    fn directive(&self, text: &str) -> Option<Verdict> {
        let lower = self.fold(text);
        for p in self.list("directive-prefixes") {
            if lower.starts_with(p.as_str()) {
                return Some(Verdict::with(
                    Role::Directive,
                    "directive prefix",
                    p.clone(),
                ));
            }
        }
        let words = self.norms(&lower);
        if let Some(m) = self.phrase_in(&words, "modals") {
            return Some(Verdict::with(Role::Directive, "modal", m));
        }
        let stripped = strip_leading_span(&lower);
        let first = stripped
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(|c: char| !c.is_alphanumeric());
        self.in_list("imperatives", first)
            .then(|| Verdict::with(Role::Directive, "leading imperative", first))
    }

    fn description_item(&self, text: &str, section: &str) -> Option<Verdict> {
        if self.is_descriptive_section(section) {
            return Some(Verdict::new(Role::Description, "descriptive section"));
        }
        if text.trim_start().starts_with('[') {
            return Some(Verdict::new(Role::Description, "link-led cross-reference"));
        }
        let lower = self.fold(text.trim_start());
        for w in self.list("rule-openers") {
            if lower.starts_with(&format!("{w} ")) {
                return Some(Verdict::new(Role::Description, "states a rule"));
            }
        }
        if is_definition_head(text) {
            return Some(Verdict::new(Role::Description, "definition head"));
        }
        self.is_copula_opening(&lower)
            .then(|| Verdict::new(Role::Description, "copula opening"))
    }

    fn is_descriptive_section(&self, section: &str) -> bool {
        let lower = self.fold(section);
        self.list("descriptive-sections")
            .iter()
            .any(|s| lower.contains(s.as_str()))
    }

    /// Opens with an article, demonstrative or quantifier and states what something is within the
    /// first few words.
    fn is_copula_opening(&self, lower: &str) -> bool {
        if !self
            .list("description-openers")
            .iter()
            .any(|o| lower.starts_with(&format!("{o} ")))
        {
            return false;
        }
        lower
            .split_whitespace()
            .take(self.copula_window)
            .any(|w| self.in_list("copulas", w))
    }

    // ---- names -------------------------------------------------------------------------------

    fn name(&self, text: &str, section: &str) -> Verdict {
        let text = text.trim();
        if let Some(v) = self.status_note(text) {
            return v;
        }
        if let Some(v) = self.sentence_or_rule(text) {
            return v;
        }
        if self.is_descriptive_section(section) {
            return Verdict::new(Role::Description, "descriptive section");
        }
        if self.config.fragment_rules {
            if let Some(rule) = self.fragment_reason(text) {
                return Verdict::new(Role::Description, rule);
            }
        }
        Verdict::new(Role::Name, "default: a short phrase is a name")
    }

    fn sentence_or_rule(&self, text: &str) -> Option<Verdict> {
        let padded = format!(" {} ", self.fold(text));
        for m in self.list("sentence-markers") {
            if padded.contains(m.as_str()) {
                return Some(Verdict::with(Role::Description, "finite verb", m.trim()));
            }
        }
        let trimmed = self.fold(text.trim_start());
        for o in self.list("sentence-openings") {
            if trimmed.starts_with(o.as_str()) {
                return Some(Verdict::with(
                    Role::Description,
                    "clause or rule opening",
                    o.trim(),
                ));
            }
        }
        is_definition_head(text).then(|| Verdict::new(Role::Description, "definition head"))
    }

    /// The rule that makes a named claim a fragment, if any. `None` means the text reads as a name.
    /// Runs on the text with parentheticals stripped: a parenthetical is a tag or gloss, not the
    /// phrase's grammar.
    fn fragment_reason(&self, text: &str) -> Option<&'static str> {
        let stripped = strip_parentheticals(text);
        let lower = self.fold(&stripped);
        let words = self.norms(&lower);
        if words.is_empty() {
            return None;
        }
        let first = words[0].as_str();
        let is_version = first.chars().count() >= 2
            && first.starts_with('v')
            && first.chars().nth(1).is_some_and(|c| c.is_ascii_digit());
        if is_version {
            return Some("fragment: version opening");
        }
        if lower.contains('\u{2192}') || lower.contains("->") {
            return Some("fragment: arrow");
        }
        if self.in_list("quantifier-openers", first)
            || (words.len() == 1 && self.ends_with_any("adverb-suffixes", first))
        {
            return Some("fragment: quantifier opening");
        }
        if self.is_verb_opening(&lower, &words) {
            return Some("fragment: verb opening");
        }
        if self.phrase_in(&words, "finite-verbs").is_some() {
            return Some("fragment: finite verb");
        }
        if self.phrase_in(&words, "not-yet-phrases").is_some() {
            return Some("fragment: not yet");
        }
        if self.is_per_fragment(&words) {
            return Some("fragment: per-fragment");
        }
        if self.has_predicative_participle(&words) {
            return Some("fragment: predicative participle");
        }
        if self.ends_with_temporal_tail(&words) {
            return Some("fragment: temporal tail");
        }
        if lower.trim_end().ends_with('.')
            && words
                .iter()
                .any(|w| self.in_list("temporal-prepositions", w))
        {
            return Some("fragment: sentence period");
        }
        None
    }

    /// Three shapes, each guarded against verb-words used as nouns ("Cache management"): a participle
    /// opening, a standalone gerund (never inside a hyphenated compound), and an imperative followed
    /// by an article (a clause with an object) or a hyphenated verb phrase with no participle segment.
    fn is_verb_opening(&self, lower: &str, words: &[String]) -> bool {
        let token = lower
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(|c: char| !c.is_alphanumeric());
        if token.is_empty() {
            return false;
        }
        let segments: Vec<&str> = token.split('-').collect();
        let first = segments[0];
        if self.in_list("participles", first) {
            return true;
        }
        if segments.len() == 1 {
            if self.ends_with_any("gerund-suffixes", first) {
                return true;
            }
            if self.in_list("imperatives", first) {
                let second = words.get(1).map_or("", String::as_str);
                return self.in_list("leading-articles", second);
            }
            return false;
        }
        self.in_list("imperatives", first)
            && !segments[1..].iter().any(|s| self.in_list("participles", s))
    }

    /// Opens with "per" and reads as a fragment: a participle or gerund follows, or a second
    /// preposition does ("per-tier free memory" is a name: "tier" is a noun).
    fn is_per_fragment(&self, words: &[String]) -> bool {
        if words.len() < 2 || words[0] != "per" {
            return false;
        }
        let second = words[1].as_str();
        if self.ends_with_any("gerund-suffixes", second) || self.in_list("participles", second) {
            return true;
        }
        words[2..].iter().any(|w| self.in_list("prepositions", w))
    }

    /// A participle used predicatively: followed by a preposition or standing last. An attributive
    /// participle before its head noun ("The persisted graph") has a noun after it and does not fire.
    fn has_predicative_participle(&self, words: &[String]) -> bool {
        words.iter().enumerate().any(|(i, w)| {
            self.in_list("participles", w)
                && (i + 1 == words.len() || self.in_list("prepositions", &words[i + 1]))
        })
    }

    fn ends_with_temporal_tail(&self, words: &[String]) -> bool {
        self.list("temporal-tails").iter().any(|tail| {
            let t = self.norms(&self.fold(tail));
            !t.is_empty() && words.len() >= t.len() && words[words.len() - t.len()..] == t[..]
        })
    }
}

/// Whether a claim counts toward the directed-but-untasked report: a list item must actually direct, a
/// name must actually name, a code identifier always counts. Descriptions and status notes are set
/// aside (counted and listed, never silently dropped). An unclassified claim is not reportable: it has
/// not been read.
pub fn is_reportable(form: &Form, kind: Kind) -> bool {
    match form {
        Form::ListItem => kind == Kind::Directive,
        Form::TableCell | Form::Bold => kind == Kind::Name,
        Form::CodeSpan => true,
        _ => false,
    }
}

// ---- shape helpers (Markdown and English conventions; see the module note) ----------------------

/// Remove `` `...` `` spans (matched pairs; an unmatched backtick leaves the rest as-is), so code
/// tokens cannot trigger a prose lexicon.
pub fn strip_backtick_spans(text: &str) -> String {
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

/// Remove `(...)` groups, depth-tracked.
fn strip_parentheticals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth: usize = 0;
    for ch in text.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

/// True when the text contains a YYYY-MM-DD date.
pub fn has_date(lower: &str) -> bool {
    let chars: Vec<char> = lower.chars().collect();
    chars.windows(10).any(|w| {
        let d = |i: usize| w[i].is_ascii_digit();
        d(0) && d(1) && d(2) && d(3) && w[4] == '-' && d(5) && d(6) && w[7] == '-' && d(8) && d(9)
    })
}

/// Strip one leading `**...**` or `` `...` `` span, unless it is a definition head (followed by ':' or
/// '.', a span whose own text ends in '.', or a parenthetical gloss but not a `(see ...)` reference).
fn strip_leading_span(lower: &str) -> &str {
    let text = lower.trim_start();
    for (open, close) in [("**", "**"), ("`", "`")] {
        if let Some(rest) = text.strip_prefix(open) {
            if let Some(end) = rest.find(close) {
                if rest[..end].trim_end().ends_with('.') {
                    return text;
                }
                let after = rest[end + close.len()..].trim_start();
                if after.starts_with(':') || after.starts_with('.') {
                    return text;
                }
                if let Some(inner) = after.strip_prefix('(').map(str::trim_start) {
                    if !inner.to_lowercase().starts_with("see ") && !inner.is_empty() {
                        return text;
                    }
                }
                return after;
            }
        }
    }
    text
}

/// True when the text opens with a term followed by ':' or '.' and more prose: `**crate**: dtypes`,
/// `**Side-table metadata.** Information ...`, `**Shared** (weights ...): ...` (a gloss, not a
/// `(see ...)` reference), or a bare `Name: gloss`.
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
    if let Some(first) = text.split_whitespace().next() {
        if first.ends_with(':') && text.split_whitespace().count() > 1 {
            return true;
        }
    }
    false
}
