// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant-logical`: the contradiction lens (SPEC-STEP1 section 6, slice 4).
//!
//! A contradiction is a *pair* of statements about the same capability, in different files, where one
//! denies it and the other asserts it. Statements come from headings (a document's own position) and
//! from claims; polarity is deterministic (lexicons plus structure), in this order: temporal markers
//! first ("not yet", dated, retracted), then negation markers ("not supported", "cannot", "no X"), else
//! positive, and every verdict names the rule that fired. The scoping rules that keep polarity honest
//! (a head window for claims, the negation of a bad thing is positive, a contrast split for headings
//! only) and the strict pairing rules (smaller subject family-contained in the larger; shared tokens
//! strong enough; the positive side carrying at least two subject tokens) are the reference spike's,
//! reproduced exactly; the spike's doc comment on `contradiction.rs:1-79` is their rationale.
//!
//! **What is new here: sense-aware suppression.** The spike's dominant failure was one generic word
//! carrying a pair across unrelated meanings ("budget", "harvest"). Given a
//! [`analysant_semantic::ConceptStore`], a shared token is *sense-split* when both sides' contexts
//! resolve it to different senses of one concept; a pair whose remaining shared tokens are no longer
//! strong enough is *suppressed*, and listed with the reason, never silently dropped. A token the
//! store cannot decide ("ambiguous") splits nothing: a wrong table entry silently removes findings, so
//! the lens leans to keeping the pair.
//!
//! All word lists are pack data (`analysant-core/data/en/`); project words (a pairing stopword that is
//! the project's own name) arrive in [`Config`].
//!
//! Deviation, stated: polarity reads the text split on non-alphanumeric characters, as the spike does,
//! not through the pack's tokenizer. A pack token like `can't` would not match the phrase list's
//! `can't` the way the spike's split (`can`, `t`) does, and the gate is line-for-line.

use analysant_core::{Claim, Form, Kind, LanguagePack, normalize};
use analysant_pragmatic::{has_date, strip_backtick_spans};
use analysant_semantic::{ConceptStore, Context, TermRelation};
use std::collections::{BTreeMap, HashSet};

/// Name of this lens in a derivation.
pub const LENS: &str = "logical";

/// Lexicons the lens reads. A pack that does not carry them cannot be used with it.
const REQUIRED_LEXICONS: &[&str] = &[
    "negation-phrases",
    "negation-bad-words",
    "negation-words",
    "refusal-words",
    "refusal-exempt-words",
    "temporal-phrases",
    "temporal-words",
    "contrast-separators",
    "pairing-stopwords",
    "stopwords",
    "leading-articles",
];

const REQUIRED_PARAMS: &[&str] = &[
    "head_window_tokens",
    "family_prefix_min",
    "strong_token_min",
    "strong_shared_min",
    "positive_subject_min",
    "subject_token_min_chars",
];

/// The pack does not carry what the lens needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingData(pub Vec<String>);

impl std::fmt::Display for MissingData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the language pack lacks what the contradiction lens reads: {}",
            self.0.join(" + ")
        )
    }
}

impl std::error::Error for MissingData {}

/// What a project adds to the pack's data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// Words that identify nothing in this project's corpus (its own name, say), removed from every
    /// subject before pairing, on top of the pack's list.
    pub extra_pairing_stopwords: Vec<String>,
    /// Leave Name-kind claims (a bare identifier, clause id or label) out of the statements: they assert nothing a denial could
    /// contradict. Off by default (the Gate A oracle runs include them); a Gate B tuning option.
    pub skip_name_claims: bool,
    /// Pair a denial with a positive statement only when the positive statement's text carries the denied word (the word the
    /// negation marker governs: "can't *run*", "no *spec*"). Off by default (the Gate A oracle runs predate it); a Gate B tuning
    /// option for the named noise class "the denied token is absent from the positive side".
    pub require_denied_token: bool,
    /// A shared word that occurs in more than this whole percent of the corpus's files identifies nothing (the project's own name, a
    /// topic word every document uses): it is set aside before the pair's strength is judged. `None` leaves every shared word counting.
    /// Off by default; a Gate B option for the noise class "one generic word carrying the pair".
    pub generic_df_percent: Option<u32>,
    /// A pair needs at least this many shared words that are not generic (see `generic_df_percent`); `None` leaves the pair's own strength
    /// rule alone. Off by default; a Gate B option for pairs held by a single shared word.
    pub min_shared_tokens: Option<usize>,
}

/// What a statement does about its capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    /// Asserts, requires, or provides the capability.
    Positive,
    /// Denies the capability ("not supported", "cannot", "no X").
    Negated,
    /// About time, not truth: "not yet", dated, retracted. Never pairs.
    Temporal,
}

/// Where a statement came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementSource {
    /// A heading: the document's own position.
    Heading,
    /// An extracted claim, of this form.
    Claim(Form),
}

/// One side of a possible contradiction, with stable identity (`file:line`, plus `#assert` / `#deny`
/// when one heading line splits in two).
#[derive(Clone, Debug)]
pub struct Statement {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub text: String,
    pub section: String,
    /// The capability the statement is about. Never empty: a statement with no subject is dropped.
    pub subject: Vec<String>,
    pub polarity: Polarity,
    /// The marker that fired for a Negated (or Temporal) statement; `None` for Positive.
    pub negated_by: Option<String>,
    pub source: StatementSource,
    /// The words around the statement (its text and its section), for sense resolution.
    pub context: Context,
}

/// A claim with the place it sits in: the lens needs the file and the heading path, which the claim
/// itself does not carry.
#[derive(Clone, Copy, Debug)]
pub struct Located<'a> {
    pub file: &'a str,
    /// The headings above the claim, outermost first, joined by " > ".
    pub section: &'a str,
    pub claim: &'a Claim,
}

/// Two statements, different files, opposite polarity, about the same capability.
#[derive(Clone, Debug)]
pub struct Contradiction {
    pub negated: Statement,
    pub positive: Statement,
    /// The shared family tokens, in the smaller subject's order.
    pub shared: Vec<String>,
    /// The marker that fired on the negated side.
    pub negated_by: Option<String>,
}

/// A shared token the concept store reads as two different meanings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SenseSplit {
    pub token: String,
    /// The store's rule that decided (`different-senses`) and what it said.
    pub rule: &'static str,
    pub note: String,
}

/// A pair the pairing rules found and the sense table removed.
#[derive(Clone, Debug)]
pub struct Suppressed {
    pub pair: Contradiction,
    pub splits: Vec<SenseSplit>,
}

/// The lens's output: the pairs that stand, and the pairs the sense table removed.
#[derive(Clone, Debug, Default)]
pub struct Findings {
    pub pairs: Vec<Contradiction>,
    pub suppressed: Vec<Suppressed>,
}

pub struct ContradictionLens<'a> {
    pack: &'a dyn LanguagePack,
    pairing_stop: HashSet<String>,
    bad: HashSet<String>,
    head_window: usize,
    family_prefix_min: usize,
    strong_token_min: usize,
    strong_shared_min: usize,
    positive_subject_min: usize,
    subject_token_min_chars: usize,
    skip_name_claims: bool,
    require_denied_token: bool,
    generic_df_percent: Option<u32>,
    min_shared_tokens: Option<usize>,
}

type Key = (String, usize, String, usize);

impl<'a> ContradictionLens<'a> {
    pub fn new(pack: &'a dyn LanguagePack, config: Config) -> Result<Self, MissingData> {
        let lex = pack.lexicons();
        let mut missing: Vec<String> = REQUIRED_LEXICONS
            .iter()
            .filter(|n| lex.entries(n).is_empty())
            .map(|n| (*n).to_string())
            .collect();
        missing.extend(
            REQUIRED_PARAMS
                .iter()
                .filter(|p| lex.param(p).is_none())
                .map(|p| format!("param {p}")),
        );
        if !missing.is_empty() {
            return Err(MissingData(missing));
        }
        let p = |name: &str| lex.param(name).unwrap_or(0);
        let mut pairing_stop: HashSet<String> =
            lex.entries("pairing-stopwords").iter().cloned().collect();
        pairing_stop.extend(config.extra_pairing_stopwords.iter().cloned());
        Ok(Self {
            pack,
            pairing_stop,
            bad: lex.entries("negation-bad-words").iter().cloned().collect(),
            head_window: p("head_window_tokens"),
            family_prefix_min: p("family_prefix_min"),
            strong_token_min: p("strong_token_min"),
            strong_shared_min: p("strong_shared_min"),
            positive_subject_min: p("positive_subject_min"),
            subject_token_min_chars: p("subject_token_min_chars"),
            skip_name_claims: config.skip_name_claims,
            require_denied_token: config.require_denied_token,
            generic_df_percent: config.generic_df_percent,
            min_shared_tokens: config.min_shared_tokens,
        })
    }

    fn entries(&self, name: &str) -> &[String] {
        self.pack.lexicons().entries(name)
    }

    /// Polarity of a text, with the marker that fired. Temporal first, then negation, except that the
    /// negation of a bad thing ("does not break", "never panics") is a positive assertion.
    pub fn polarity(&self, text: &str) -> (Polarity, Option<String>) {
        let lower = self.pack.normalizer().fold_case(text);
        let prose = strip_backtick_spans(&lower);
        if has_date(&prose) {
            return (Polarity::Temporal, Some("date".to_string()));
        }
        for marker in self.entries("temporal-phrases") {
            if contains_phrase(&prose, marker) {
                return (Polarity::Temporal, Some(marker.clone()));
            }
        }
        for word in self.entries("temporal-words") {
            if contains_standalone_word(&prose, word) {
                return (Polarity::Temporal, Some(word.clone()));
            }
        }
        let tokens: Vec<&str> = alnum_words(&prose);
        for marker in self.entries("negation-phrases") {
            if let Some(end) = phrase_end(&tokens, marker) {
                if tokens.get(end).is_some_and(|w| self.bad.contains(*w)) {
                    return (Polarity::Positive, None);
                }
                return (Polarity::Negated, Some(marker.clone()));
            }
        }
        for word in self.entries("negation-words") {
            if let Some(next) = word_after_standalone(&prose, word) {
                if self.bad.contains(&next) {
                    return (Polarity::Positive, None);
                }
                return (Polarity::Negated, Some(word.clone()));
            }
        }
        for word in self.entries("refusal-words") {
            if let Some(refused) = self.refused_capability(&prose, word) {
                if self.bad.contains(&refused) {
                    return (Polarity::Positive, None);
                }
                return (Polarity::Negated, Some(format!("{word} <capability>")));
            }
        }
        (Polarity::Positive, None)
    }

    /// The subject tokens that occur in more than `generic_df_percent` of the files the statements come from (empty when off).
    fn generic_tokens(&self, statements: &[Statement]) -> HashSet<String> {
        let Some(percent) = self.generic_df_percent else {
            return HashSet::new();
        };
        let mut files: HashSet<&str> = HashSet::new();
        let mut df: std::collections::HashMap<&str, HashSet<&str>> =
            std::collections::HashMap::new();
        for s in statements {
            files.insert(s.file.as_str());
            for t in &s.subject {
                df.entry(t.as_str()).or_default().insert(s.file.as_str());
            }
        }
        let total = files.len();
        df.into_iter()
            .filter(|(_, in_files)| in_files.len() * 100 > percent as usize * total)
            .map(|(t, _)| t.to_string())
            .collect()
    }

    /// The content word a negation governs: the word after a refusal word (`no <capability>`), or the first non-stopword after the
    /// marker phrase. `None` when there is none, or when it could never be a subject token (a pairing stopword, a digit word).
    fn denied_token(&self, neg: &Statement) -> Option<String> {
        let marker = neg.negated_by.as_deref()?;
        let norm = self.pack.normalizer();
        let prose = strip_backtick_spans(&norm.fold_case(&self.head_window(&neg.text)));
        let word = if let Some(refusal) = marker.strip_suffix(" <capability>") {
            self.refused_capability(&prose, refusal)?
        } else {
            let tokens = alnum_words(&prose);
            let end = phrase_end(&tokens, marker)?;
            tokens[end..]
                .iter()
                .find(|w| !norm.is_stopword(w))?
                .to_string()
        };
        self.subject_tokens(&word).into_iter().next()
    }

    /// The capability a standalone refusal word refuses: the word after it that is neither a stopword
    /// nor an exempt word ("no longer" is temporal, checked earlier).
    fn refused_capability(&self, lower: &str, refusal: &str) -> Option<String> {
        let words = whitespace_words(lower);
        let norm = self.pack.normalizer();
        let exempt = self.entries("refusal-exempt-words");
        words
            .windows(2)
            .find(|pair| {
                pair[0] == refusal && !norm.is_stopword(&pair[1]) && !exempt.contains(&pair[1])
            })
            .map(|pair| pair[1].clone())
    }

    /// The subject of a text: normalized content tokens, pairing stopwords, single characters, and any
    /// token containing a digit or reading as a version (`v0`) removed, deduplicated in order.
    pub fn subject_tokens(&self, text: &str) -> Vec<String> {
        let norm = self.pack.normalizer();
        let lower = norm.fold_case(text);
        let mut words = alnum_words(&lower);
        if words.first().is_some_and(|w| norm.is_leading_article(w)) {
            words.remove(0);
        }
        let mut out: Vec<String> = Vec::new();
        for word in words {
            if norm.is_stopword(word)
                || word.chars().count() < self.subject_token_min_chars
                || word.chars().any(|c| c.is_ascii_digit())
                || is_version(word)
                || self.pairing_stop.contains(word)
                || out.iter().any(|w| w == word)
            {
                continue;
            }
            out.push(word.to_string());
        }
        out
    }

    fn context_of(&self, text: &str, section: &str) -> Context {
        Context::from_text(self.pack, &format!("{text} {section}"))
    }

    /// Split an "X, not Y" contrast into (asserted, denied). `None` when the text is temporal ("X, not
    /// yet Y" is a deferral) or has no contrast separator.
    fn split_contrast(&self, text: &str) -> Option<(String, String)> {
        if self.polarity(text).0 == Polarity::Temporal {
            return None;
        }
        let lower = self.pack.normalizer().fold_case(text);
        for sep in self.entries("contrast-separators") {
            if let Some(pos) = lower.find(sep.as_str()) {
                let (Some(left), Some(right)) = (text.get(..pos), text.get(pos + sep.len()..))
                else {
                    continue;
                };
                let (asserted, denied) = (left.trim().to_string(), right.trim().to_string());
                if !asserted.is_empty() && !denied.is_empty() {
                    return Some((asserted, denied));
                }
            }
        }
        None
    }

    /// The first `head_window_tokens` whitespace-delimited tokens of a text: the head window for claim
    /// polarity.
    fn head_window(&self, text: &str) -> String {
        text.split_whitespace()
            .take(self.head_window)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Statements from claims: everything except code identifiers (a code observation, not an
    /// assertion) and status notes (a retraction is temporal, not a position). The subject comes from
    /// the claim's key; polarity from the first words of its text. Claims never split on a contrast.
    pub fn statements_from_claims(&self, claims: &[Located<'_>]) -> Vec<Statement> {
        let mut out = Vec::new();
        for l in claims {
            let c = l.claim;
            if c.form == Form::CodeSpan || c.kind == Kind::Report {
                continue;
            }
            if self.skip_name_claims && c.kind == Kind::Name {
                continue;
            }
            let subject = self.subject_tokens(&c.normal.key);
            if subject.is_empty() {
                continue;
            }
            let (polarity, negated_by) = self.polarity(&self.head_window(&c.text));
            let line = c.unit.source.span.line_start as usize;
            out.push(Statement {
                id: format!("{}:{line}", l.file),
                file: l.file.to_string(),
                line,
                text: c.text.clone(),
                section: l.section.to_string(),
                subject,
                polarity,
                negated_by,
                source: StatementSource::Claim(c.form.clone()),
                context: self.context_of(&c.text, l.section),
            });
        }
        out
    }

    /// Statements from one heading: it states the document's own position. A contrast heading yields
    /// two statements (the asserted half, the denied half); empty subjects are dropped.
    pub fn statements_from_heading(
        &self,
        file: &str,
        line: usize,
        title: &str,
        section: &str,
    ) -> Vec<Statement> {
        let make = |id: String, text: String, subject, polarity, negated_by| Statement {
            context: self.context_of(&text, section),
            id,
            file: file.to_string(),
            line,
            text,
            section: section.to_string(),
            subject,
            polarity,
            negated_by,
            source: StatementSource::Heading,
        };
        let mut out = Vec::new();
        if let Some((asserted, denied)) = self.split_contrast(title) {
            let subject = self.subject_tokens(&asserted);
            if !subject.is_empty() {
                let (pol, marker) = self.polarity(&asserted);
                out.push(make(
                    format!("{file}:{line}#assert"),
                    asserted,
                    subject,
                    pol,
                    marker,
                ));
            }
            let subject = self.subject_tokens(&denied);
            if !subject.is_empty() {
                out.push(make(
                    format!("{file}:{line}#deny"),
                    denied,
                    subject,
                    Polarity::Negated,
                    Some("contrast".to_string()),
                ));
            }
            return out;
        }
        let subject = self.subject_tokens(title);
        if !subject.is_empty() {
            let (pol, marker) = self.polarity(title);
            out.push(make(
                format!("{file}:{line}"),
                title.to_string(),
                subject,
                pol,
                marker,
            ));
        }
        out
    }

    /// Two tokens name the same capability family: identical, differing only by a trailing "s", or
    /// sharing a common prefix of `family_prefix_min`+ letters. The floor keeps the rule from
    /// overmerging short words ("state" and "statement").
    pub fn same_family(&self, a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }
        let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
        if long.len() == short.len() + 1 && long.starts_with(short) && long.ends_with('s') {
            return true;
        }
        let common = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
        common >= self.family_prefix_min
    }

    /// The smaller subject family-contained in the larger one: the matched tokens in the smaller
    /// subject's order, and whether the smaller one is `a`. An empty subject never matches.
    fn shared_subject(&self, a: &[String], b: &[String]) -> Option<(Vec<String>, bool)> {
        let small_is_a = a.len() <= b.len();
        let (small, large) = if small_is_a { (a, b) } else { (b, a) };
        if small.is_empty() {
            return None;
        }
        let mut shared = Vec::new();
        for tok in small {
            if large.iter().any(|t| self.same_family(tok, t)) {
                shared.push(tok.clone());
            } else {
                return None;
            }
        }
        Some((shared, small_is_a))
    }

    /// At least one token of `strong_token_min`+ letters, or at least `strong_shared_min` tokens.
    fn strong_enough(&self, shared: &[String]) -> bool {
        shared
            .iter()
            .any(|t| t.chars().count() >= self.strong_token_min)
            || shared.len() >= self.strong_shared_min
    }

    /// The shared tokens the store reads as different meanings on the two sides. A token is split when
    /// every family partner on the other side resolves to a different sense of one concept.
    fn sense_splits(
        &self,
        store: &ConceptStore<'_>,
        neg: &Statement,
        pos: &Statement,
        shared: &[String],
        small_is_neg: bool,
    ) -> Vec<SenseSplit> {
        let (small, large) = if small_is_neg { (neg, pos) } else { (pos, neg) };
        let mut splits = Vec::new();
        for tok in shared {
            let a = normalize(self.pack, tok);
            let mut first: Option<SenseSplit> = None;
            let mut all = true;
            for partner in large.subject.iter().filter(|u| self.same_family(tok, u)) {
                let b = normalize(self.pack, partner);
                let verdict = store.relate_in(&a, &small.context, &b, &large.context);
                if verdict.value == Some(TermRelation::Unrelated) {
                    if first.is_none() {
                        let why = verdict.reasons.first();
                        first = Some(SenseSplit {
                            token: tok.clone(),
                            rule: why.map_or("different-senses", |r| r.rule),
                            note: why.map(|r| r.note.clone()).unwrap_or_default(),
                        });
                    }
                } else {
                    all = false;
                }
            }
            if all {
                splits.extend(first);
            }
        }
        splits
    }

    /// Every contradiction among the statements: Negated vs Positive, different files, smaller subject
    /// family-contained in the larger, shared tokens strong enough, the positive side carrying at least
    /// `positive_subject_min` subject tokens. One pair per (negated line, positive line): the one sharing
    /// the most tokens wins. Sorted by negated file:line, then positive file:line.
    ///
    /// With a `store`, a pair whose shared tokens are sense-split, leaving too little to carry it, is
    /// moved to `suppressed` (with the splits) instead.
    pub fn find_contradictions(
        &self,
        statements: &[Statement],
        store: Option<&ConceptStore<'_>>,
    ) -> Findings {
        let generic = self.generic_tokens(statements);
        let mut best: BTreeMap<Key, Contradiction> = BTreeMap::new();
        let mut suppressed: BTreeMap<Key, Suppressed> = BTreeMap::new();
        for neg in statements
            .iter()
            .filter(|s| s.polarity == Polarity::Negated)
        {
            for pos in statements.iter().filter(|s| {
                s.polarity == Polarity::Positive && s.subject.len() >= self.positive_subject_min
            }) {
                if neg.file == pos.file {
                    continue;
                }
                if self.require_denied_token {
                    if let Some(denied) = self.denied_token(neg) {
                        let folded = self.pack.normalizer().fold_case(&pos.text);
                        if !alnum_words(&folded)
                            .iter()
                            .any(|w| self.same_family(&denied, w))
                        {
                            continue;
                        }
                    }
                }
                let Some((shared, small_is_neg)) = self.shared_subject(&neg.subject, &pos.subject)
                else {
                    continue;
                };
                if !self.strong_enough(&shared) {
                    continue;
                }
                if !generic.is_empty() || self.min_shared_tokens.is_some() {
                    let rest: Vec<String> = shared
                        .iter()
                        .filter(|t| !generic.contains(*t))
                        .cloned()
                        .collect();
                    if !generic.is_empty() && !self.strong_enough(&rest) {
                        continue;
                    }
                    if self.min_shared_tokens.is_some_and(|n| rest.len() < n) {
                        continue;
                    }
                }
                let key = (neg.file.clone(), neg.line, pos.file.clone(), pos.line);
                let cand = Contradiction {
                    negated: neg.clone(),
                    positive: pos.clone(),
                    shared: shared.clone(),
                    negated_by: neg.negated_by.clone(),
                };
                if let Some(store) = store {
                    let splits = self.sense_splits(store, neg, pos, &shared, small_is_neg);
                    let split_tokens: Vec<&String> = splits.iter().map(|s| &s.token).collect();
                    let rest: Vec<String> = shared
                        .iter()
                        .filter(|t| !split_tokens.contains(t))
                        .cloned()
                        .collect();
                    if !splits.is_empty() && !self.strong_enough(&rest) {
                        let better = suppressed
                            .get(&key)
                            .is_none_or(|p| p.pair.shared.len() < cand.shared.len());
                        if better {
                            suppressed.insert(key, Suppressed { pair: cand, splits });
                        }
                        continue;
                    }
                }
                let better = best
                    .get(&key)
                    .is_none_or(|prev| prev.shared.len() < cand.shared.len());
                if better {
                    best.insert(key, cand);
                }
            }
        }
        suppressed.retain(|k, _| !best.contains_key(k));
        Findings {
            pairs: best.into_values().collect(),
            suppressed: suppressed.into_values().collect(),
        }
    }
}

/// `v` followed by digits only ("v0", "v12").
fn is_version(word: &str) -> bool {
    word.len() >= 2 && word.starts_with('v') && word[1..].chars().all(|c| c.is_ascii_digit())
}

/// The words of a text split on every non-alphanumeric character.
fn alnum_words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Word-boundary phrase match: both sides split into alphanumeric words, then a contiguous check.
fn contains_phrase(lower: &str, phrase: &str) -> bool {
    let hay = alnum_words(lower);
    let needle = alnum_words(phrase);
    !needle.is_empty()
        && needle.len() <= hay.len()
        && hay.windows(needle.len()).any(|w| w == needle.as_slice())
}

/// One past the last matched token of the first window where `phrase` matches `tokens`.
fn phrase_end(tokens: &[&str], phrase: &str) -> Option<usize> {
    let needle = alnum_words(phrase);
    if needle.is_empty() || needle.len() > tokens.len() {
        return None;
    }
    tokens
        .windows(needle.len())
        .position(|w| w == needle.as_slice())
        .map(|i| i + needle.len())
}

/// Whitespace-delimited words trimmed of punctuation: hyphenated compounds stay whole ("never-panics"
/// has no standalone "never").
fn whitespace_words(lower: &str) -> Vec<String> {
    lower
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

fn contains_standalone_word(lower: &str, word: &str) -> bool {
    lower
        .split_whitespace()
        .any(|token| token.trim_matches(|c: char| !c.is_alphanumeric()) == word)
}

/// The word after the first standalone occurrence of `word`, or `None` (also when it ends the text).
fn word_after_standalone(lower: &str, word: &str) -> Option<String> {
    let words = whitespace_words(lower);
    let i = words.iter().position(|w| w == word)?;
    words.get(i + 1).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysant_core::English;

    fn lens(en: &English) -> ContradictionLens<'_> {
        ContradictionLens::new(en, Config::default()).expect("en carries the data")
    }

    #[test]
    fn polarity_follows_the_spikes_order_temporal_then_negation_then_positive() {
        let en = English::new();
        let l = lens(&en);
        let p = |t: &str| l.polarity(t);
        assert_eq!(
            p("Cancellation is not supported"),
            (Polarity::Negated, Some("not supported".into()))
        );
        assert_eq!(p("Cancellation can be requested").0, Polarity::Positive);
        // Temporal first: "not yet" contains a negation word but defers.
        assert_eq!(p("not yet built").0, Polarity::Temporal);
        assert_eq!(p("decided on 2026-11-02").0, Polarity::Temporal);
        // The negation of a bad thing is positive.
        assert_eq!(p("the kernel does not break").0, Polarity::Positive);
        assert_eq!(p("it never panics").0, Polarity::Positive);
        assert_eq!(
            p("it never retries"),
            (Polarity::Negated, Some("never".into()))
        );
        // A refusal word names its capability; a hyphenated compound is no refusal; "no longer" defers.
        assert_eq!(
            p("no network"),
            (Polarity::Negated, Some("no <capability>".into()))
        );
        assert_eq!(p("a no-op pass").0, Polarity::Positive);
        assert_eq!(p("no longer shipped").0, Polarity::Temporal);
        // Backticked code does not trigger prose rules.
        assert_eq!(p("the `is_not_ready` flag").0, Polarity::Positive);
    }

    #[test]
    fn subjects_drop_stance_words_numbers_versions_and_single_letters() {
        let en = English::new();
        let l = lens(&en);
        assert_eq!(
            l.subject_tokens("The cancellation is not supported v2 qwen35 x"),
            vec!["cancellation"]
        );
        let with_extra = ContradictionLens::new(
            &en,
            Config {
                extra_pairing_stopwords: vec!["acme".into()],
                ..Config::default()
            },
        )
        .expect("data");
        assert_eq!(
            with_extra.subject_tokens("acme scheduler"),
            vec!["scheduler"]
        );
        // Positive control: without the extra word it stays.
        assert_eq!(
            l.subject_tokens("acme scheduler"),
            vec!["acme", "scheduler"]
        );
    }

    #[test]
    fn family_needs_a_long_enough_shared_prefix_or_a_plural_s() {
        let en = English::new();
        let l = lens(&en);
        assert!(l.same_family("cancel", "cancels"));
        assert!(l.same_family("cancelling", "cancellation"));
        assert!(!l.same_family("state", "statement"));
        assert!(!l.same_family("admit", "admission"));
    }

    fn statement(l: &ContradictionLens<'_>, file: &str, line: usize, text: &str) -> Statement {
        let subject = l.subject_tokens(text);
        let (polarity, negated_by) = l.polarity(text);
        Statement {
            id: format!("{file}:{line}"),
            file: file.into(),
            line,
            text: text.into(),
            section: String::new(),
            subject,
            polarity,
            negated_by,
            source: StatementSource::Heading,
            context: l.context_of(text, ""),
        }
    }

    #[test]
    fn a_denial_and_an_assertion_in_two_files_pair_and_the_same_file_does_not() {
        let en = English::new();
        let l = lens(&en);
        let s = [
            statement(&l, "a.md", 1, "Scheduler cancellation is not supported"),
            statement(&l, "b.md", 2, "Scheduler cancellation in flight"),
            statement(&l, "a.md", 3, "Scheduler cancellation in flight"),
        ];
        let f = l.find_contradictions(&s, None);
        assert_eq!(f.pairs.len(), 1);
        assert_eq!(f.pairs[0].negated.id, "a.md:1");
        assert_eq!(f.pairs[0].positive.id, "b.md:2");
        assert_eq!(f.pairs[0].shared, vec!["scheduler", "cancellation"]);
        // The bare-name positive side (one subject token) never pairs.
        let s = [
            statement(&l, "a.md", 1, "Scheduler cancellation is not supported"),
            statement(&l, "b.md", 2, "Cancellation"),
        ];
        assert!(l.find_contradictions(&s, None).pairs.is_empty());
    }

    #[test]
    fn require_denied_token_drops_a_pair_whose_positive_side_lacks_the_denied_word() {
        let en = English::new();
        let strict = ContradictionLens::new(
            &en,
            Config {
                require_denied_token: true,
                ..Config::default()
            },
        )
        .expect("data");
        let plain = lens(&en);
        let mk = |l: &ContradictionLens<'_>, pos: &str| {
            [
                statement(l, "a.md", 1, "Scheduler cancellation cannot run"),
                statement(l, "b.md", 2, pos),
            ]
        };
        // The positive side never says "run": a shared topic, not a contradiction.
        let absent = "Scheduler cancellation";
        assert_eq!(
            plain
                .find_contradictions(&mk(&plain, absent), None)
                .pairs
                .len(),
            1
        );
        assert!(
            strict
                .find_contradictions(&mk(&strict, absent), None)
                .pairs
                .is_empty()
        );
        // Positive control: when the positive side carries the denied word, it still pairs.
        let present = "Scheduler cancellation can run";
        assert_eq!(
            strict
                .find_contradictions(&mk(&strict, present), None)
                .pairs
                .len(),
            1
        );
        // A stance-word denial ("is not supported") has no content denied word, so it is never filtered.
        let s = [
            statement(
                &strict,
                "a.md",
                1,
                "Scheduler cancellation is not supported",
            ),
            statement(&strict, "b.md", 2, absent),
        ];
        assert_eq!(strict.find_contradictions(&s, None).pairs.len(), 1);
    }

    #[test]
    fn generic_df_percent_sets_aside_words_every_document_uses() {
        let en = English::new();
        let strict = ContradictionLens::new(
            &en,
            Config {
                generic_df_percent: Some(60),
                ..Config::default()
            },
        )
        .expect("data");
        let plain = lens(&en);
        // "scheduler cancellation" is in all four files: it names the project's topic, not a shared fact.
        let all = |l: &ContradictionLens<'_>| {
            [
                statement(l, "a.md", 1, "Scheduler cancellation is not supported"),
                statement(l, "b.md", 2, "Scheduler cancellation in flight"),
                statement(l, "c.md", 3, "Scheduler cancellation guide"),
                statement(l, "d.md", 4, "Scheduler cancellation notes"),
            ]
        };
        assert!(
            !plain
                .find_contradictions(&all(&plain), None)
                .pairs
                .is_empty()
        );
        assert!(
            strict
                .find_contradictions(&all(&strict), None)
                .pairs
                .is_empty()
        );
        assert_eq!(strict.generic_tokens(&all(&strict)).len(), 2);
        assert!(plain.generic_tokens(&all(&plain)).is_empty());
        // Positive control: the same words in two of four files are specific enough to pair.
        let two = [
            statement(
                &strict,
                "a.md",
                1,
                "Scheduler cancellation is not supported",
            ),
            statement(&strict, "b.md", 2, "Scheduler cancellation in flight"),
            statement(&strict, "c.md", 3, "Unrelated guide"),
            statement(&strict, "d.md", 4, "Other notes"),
        ];
        assert_eq!(strict.find_contradictions(&two, None).pairs.len(), 1);
    }

    #[test]
    fn min_shared_tokens_drops_a_pair_held_by_too_few_shared_words() {
        let en = English::new();
        let with = |n: Option<usize>| {
            ContradictionLens::new(
                &en,
                Config {
                    min_shared_tokens: n,
                    ..Config::default()
                },
            )
            .expect("data")
        };
        let pair = |l: &ContradictionLens<'_>| {
            [
                statement(l, "a.md", 1, "Scheduler cancellation is not supported"),
                statement(l, "b.md", 2, "Scheduler cancellation in flight"),
            ]
        };
        // The pair shares two words (scheduler, cancellation).
        let (off, two, three) = (with(None), with(Some(2)), with(Some(3)));
        assert_eq!(off.find_contradictions(&pair(&off), None).pairs.len(), 1);
        assert_eq!(two.find_contradictions(&pair(&two), None).pairs.len(), 1);
        assert!(
            three
                .find_contradictions(&pair(&three), None)
                .pairs
                .is_empty()
        );
    }

    #[test]
    fn a_missing_lexicon_is_reported_not_defaulted() {
        struct Bare(English);
        impl LanguagePack for Bare {
            fn id(&self) -> analysant_core::PackId {
                self.0.id()
            }
            fn script_kind(&self) -> analysant_core::LanguageKind {
                self.0.script_kind()
            }
            fn tokenize(&self, s: &analysant_core::Span, t: &str) -> Vec<analysant_core::Token> {
                self.0.tokenize(s, t)
            }
            fn sentences(&self, t: &str) -> Vec<std::ops::Range<usize>> {
                self.0.sentences(t)
            }
            fn normalizer(&self) -> &dyn analysant_core::Normalizer {
                self.0.normalizer()
            }
            fn lexicons(&self) -> &analysant_core::LexiconSet {
                static EMPTY: std::sync::OnceLock<analysant_core::LexiconSet> =
                    std::sync::OnceLock::new();
                EMPTY.get_or_init(analysant_core::LexiconSet::default)
            }
            fn extract(&self, b: &analysant_core::Block) -> Vec<analysant_core::RawClaim> {
                self.0.extract(b)
            }
        }
        let bare = Bare(English::new());
        let Err(MissingData(m)) = ContradictionLens::new(&bare, Config::default()) else {
            panic!("an empty pack must be refused")
        };
        assert!(m.contains(&"negation-phrases".to_string()));
        assert!(m.contains(&"param head_window_tokens".to_string()));
    }
}
