// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `en` language pack: English prose. Its tokenizer parameters, word lists and thresholds are
//! data files under `data/en/` (SPEC-STEP1 2.3), embedded at build time; no word list is in this file.
//!
//! `extract` implements structural shapes only (a list item, a table's first cell, an emphasised
//! name, a code identifier). Deciding that a list item is a directive is the pragmatic lens, later.

use crate::model::{Form, Span};
use crate::pack::{
    Block, BlockKind, LanguageKind, LanguagePack, LexiconSet, Normalizer, PackId, RawClaim, Token,
    TokenClass, normalize,
};
use std::collections::HashSet;
use std::ops::Range;

const STOPWORDS: &str = include_str!("../data/en/stopwords.txt");
const LEADING_ARTICLES: &str = include_str!("../data/en/leading-articles.txt");
const NAME_LEAD_WORDS: &str = include_str!("../data/en/name-lead-words.txt");
const NAME_REJECT_PUNCT: &str = include_str!("../data/en/name-reject-punct.txt");
const IDENTIFIER_MARKERS: &str = include_str!("../data/en/identifier-markers.txt");
const ABBREVIATIONS: &str = include_str!("../data/en/abbreviations.txt");
const PARAMS: &str = include_str!("../data/en/params.txt");

const STATUS_MARKERS: &str = include_str!("../data/en/status-markers.txt");
const OUTCOME_MARKERS: &str = include_str!("../data/en/outcome-markers.txt");
const IMPERATIVES: &str = include_str!("../data/en/imperatives.txt");
const MODALS: &str = include_str!("../data/en/modals.txt");
const DESCRIPTIVE_SECTIONS: &str = include_str!("../data/en/descriptive-sections.txt");
const SENTENCE_MARKERS: &str = include_str!("../data/en/sentence-markers.txt");
const SENTENCE_OPENINGS: &str = include_str!("../data/en/sentence-openings.txt");
const FINITE_VERBS: &str = include_str!("../data/en/finite-verbs.txt");
const PREPOSITIONS: &str = include_str!("../data/en/prepositions.txt");
const PARTICIPLES: &str = include_str!("../data/en/participles.txt");
const TEMPORAL_PREPOSITIONS: &str = include_str!("../data/en/temporal-prepositions.txt");
const DIRECTIVE_PREFIXES: &str = include_str!("../data/en/directive-prefixes.txt");
const DESCRIPTION_OPENERS: &str = include_str!("../data/en/description-openers.txt");
const COPULAS: &str = include_str!("../data/en/copulas.txt");
const QUANTIFIER_OPENERS: &str = include_str!("../data/en/quantifier-openers.txt");
const TEMPORAL_TAILS: &str = include_str!("../data/en/temporal-tails.txt");
const RULE_OPENERS: &str = include_str!("../data/en/rule-openers.txt");
const NON_WORK_DOCUMENTS: &str = include_str!("../data/en/non-work-documents.txt");
const WORK_DOCUMENTS: &str = include_str!("../data/en/work-documents.txt");
const STANDING_RULE_OPENERS: &str = include_str!("../data/en/standing-rule-openers.txt");
const OPEN_WORK_MARKERS: &str = include_str!("../data/en/open-work-markers.txt");
const CHECKBOX_OPENERS: &str = include_str!("../data/en/checkbox-openers.txt");
const CLOSED_DOCUMENT_MARKERS: &str = include_str!("../data/en/closed-document-markers.txt");
const CLAUSE_JOINERS: &str = include_str!("../data/en/clause-joiners.txt");
const STATUS_LINE_WORDS: &str = include_str!("../data/en/status-line-words.txt");
const ADVERB_SUFFIXES: &str = include_str!("../data/en/adverb-suffixes.txt");
const GERUND_SUFFIXES: &str = include_str!("../data/en/gerund-suffixes.txt");
const NOT_YET_PHRASES: &str = include_str!("../data/en/not-yet-phrases.txt");
const NEGATION_BAD_WORDS: &str = include_str!("../data/en/negation-bad-words.txt");
const NEGATION_PHRASES: &str = include_str!("../data/en/negation-phrases.txt");
const NEGATION_WORDS: &str = include_str!("../data/en/negation-words.txt");
const TEMPORAL_PHRASES: &str = include_str!("../data/en/temporal-phrases.txt");
const TEMPORAL_WORDS: &str = include_str!("../data/en/temporal-words.txt");
const REFUSAL_WORDS: &str = include_str!("../data/en/refusal-words.txt");
const REFUSAL_EXEMPT_WORDS: &str = include_str!("../data/en/refusal-exempt-words.txt");
const CONTRAST_SEPARATORS: &str = include_str!("../data/en/contrast-separators.txt");
const PAIRING_STOPWORDS: &str = include_str!("../data/en/pairing-stopwords.txt");

/// Parameters the extraction rules read. Checked once, at construction.
const REQUIRED_PARAMS: &[&str] = &[
    "item_min_chars",
    "cell_min_chars",
    "name_min_chars",
    "name_max_chars",
    "name_max_words",
    "shouting_percent",
    "ident_max_chars",
    "head_max_content_words",
    "head_bold_min_words",
];

struct EnNormalizer {
    stop: HashSet<String>,
    articles: HashSet<String>,
}

impl Normalizer for EnNormalizer {
    fn fold_case(&self, text: &str) -> String {
        text.to_lowercase()
    }

    fn is_leading_article(&self, norm: &str) -> bool {
        self.articles.contains(norm)
    }

    fn is_stopword(&self, norm: &str) -> bool {
        self.stop.contains(norm)
    }
}

/// English.
pub struct English {
    normalizer: EnNormalizer,
    lexicons: LexiconSet,
}

impl Default for English {
    fn default() -> Self {
        Self::new()
    }
}

impl English {
    /// Load the pack from its embedded data. The data ships with the crate and is tested, so a
    /// malformed table is a build defect, reported by panic with the table and line.
    pub fn new() -> Self {
        let mut lexicons = LexiconSet::default();
        lexicons.add_lexicon("stopwords", STOPWORDS);
        lexicons.add_lexicon("leading-articles", LEADING_ARTICLES);
        lexicons.add_lexicon("name-lead-words", NAME_LEAD_WORDS);
        lexicons.add_lexicon("name-reject-punct", NAME_REJECT_PUNCT);
        lexicons.add_lexicon("identifier-markers", IDENTIFIER_MARKERS);
        lexicons.add_lexicon("abbreviations", ABBREVIATIONS);
        lexicons.add_lexicon("status-markers", STATUS_MARKERS);
        lexicons.add_lexicon("outcome-markers", OUTCOME_MARKERS);
        lexicons.add_lexicon("imperatives", IMPERATIVES);
        lexicons.add_lexicon("modals", MODALS);
        lexicons.add_lexicon("descriptive-sections", DESCRIPTIVE_SECTIONS);
        lexicons.add_lexicon("sentence-markers", SENTENCE_MARKERS);
        lexicons.add_lexicon("sentence-openings", SENTENCE_OPENINGS);
        lexicons.add_lexicon("finite-verbs", FINITE_VERBS);
        lexicons.add_lexicon("prepositions", PREPOSITIONS);
        lexicons.add_lexicon("participles", PARTICIPLES);
        lexicons.add_lexicon("temporal-prepositions", TEMPORAL_PREPOSITIONS);
        lexicons.add_lexicon("directive-prefixes", DIRECTIVE_PREFIXES);
        lexicons.add_lexicon("description-openers", DESCRIPTION_OPENERS);
        lexicons.add_lexicon("copulas", COPULAS);
        lexicons.add_lexicon("quantifier-openers", QUANTIFIER_OPENERS);
        lexicons.add_lexicon("temporal-tails", TEMPORAL_TAILS);
        lexicons.add_lexicon("rule-openers", RULE_OPENERS);
        lexicons.add_lexicon("non-work-documents", NON_WORK_DOCUMENTS);
        lexicons.add_lexicon("work-documents", WORK_DOCUMENTS);
        lexicons.add_lexicon("standing-rule-openers", STANDING_RULE_OPENERS);
        lexicons.add_lexicon("open-work-markers", OPEN_WORK_MARKERS);
        lexicons.add_lexicon("checkbox-openers", CHECKBOX_OPENERS);
        lexicons.add_lexicon("closed-document-markers", CLOSED_DOCUMENT_MARKERS);
        lexicons.add_lexicon("clause-joiners", CLAUSE_JOINERS);
        lexicons.add_lexicon("status-line-words", STATUS_LINE_WORDS);
        lexicons.add_lexicon("adverb-suffixes", ADVERB_SUFFIXES);
        lexicons.add_lexicon("gerund-suffixes", GERUND_SUFFIXES);
        lexicons.add_lexicon("not-yet-phrases", NOT_YET_PHRASES);
        lexicons.add_lexicon("negation-bad-words", NEGATION_BAD_WORDS);
        lexicons.add_lexicon("negation-phrases", NEGATION_PHRASES);
        lexicons.add_lexicon("negation-words", NEGATION_WORDS);
        lexicons.add_lexicon("temporal-phrases", TEMPORAL_PHRASES);
        lexicons.add_lexicon("temporal-words", TEMPORAL_WORDS);
        lexicons.add_lexicon("refusal-words", REFUSAL_WORDS);
        lexicons.add_lexicon("refusal-exempt-words", REFUSAL_EXEMPT_WORDS);
        lexicons.add_lexicon("contrast-separators", CONTRAST_SEPARATORS);
        lexicons.add_lexicon("pairing-stopwords", PAIRING_STOPWORDS);
        if let Err(e) = lexicons.add_params("params", PARAMS) {
            panic!("en pack data is malformed: {e}");
        }
        for name in REQUIRED_PARAMS {
            assert!(
                lexicons.param(name).is_some(),
                "en pack data lacks parameter `{name}`"
            );
        }
        let set =
            |name: &str| -> HashSet<String> { lexicons.entries(name).iter().cloned().collect() };
        let normalizer = EnNormalizer {
            stop: set("stopwords"),
            articles: set("leading-articles"),
        };
        Self {
            normalizer,
            lexicons,
        }
    }

    fn p(&self, name: &str) -> usize {
        // Presence is checked in `new`.
        self.lexicons.param(name).unwrap_or(0)
    }

    fn rejects_as_name_punctuation(&self, span: &str) -> bool {
        self.lexicons
            .entries("name-reject-punct")
            .iter()
            .any(|p| span.contains(p.as_str()))
    }

    /// An emphasised span that reads as a name: not a sentence, not a clause, not shouting.
    fn looks_like_name(&self, span: &str) -> bool {
        let s = span.trim();
        if s.is_empty() || self.rejects_as_name_punctuation(s) {
            return false;
        }
        let words: Vec<&str> = s.split_whitespace().collect();
        let Some(first) = words.first() else {
            return false;
        };
        if words.len() > self.p("name_max_words") {
            return false;
        }
        let opens_lowercase = first.chars().next().is_some_and(char::is_lowercase);
        if opens_lowercase {
            let folded = self.normalizer.fold_case(first);
            if self.lexicons.entries("name-lead-words").contains(&folded) {
                return false;
            }
        }
        let letters = s.chars().filter(|c| c.is_alphabetic()).count();
        if letters == 0 {
            return false;
        }
        let upper = s
            .chars()
            .filter(|c| c.is_alphabetic() && c.is_uppercase())
            .count();
        upper * 100 <= letters * self.p("shouting_percent")
    }

    /// A code span that reads as an identifier rather than prose.
    fn looks_like_identifier(&self, span: &str) -> bool {
        let s = span.trim();
        if s.is_empty() || s.contains(' ') || s.chars().count() > self.p("ident_max_chars") {
            return false;
        }
        self.lexicons
            .entries("identifier-markers")
            .iter()
            .any(|m| s.contains(m.as_str()))
            || (s.chars().any(char::is_uppercase) && s.chars().any(char::is_lowercase))
    }

    /// The phrase a list item is keyed by: its leading emphasised name when that has enough words,
    /// otherwise its first content words.
    fn head_of(&self, item: &str, strong: &[String]) -> String {
        if let Some(first) = strong.first() {
            let span = first.trim();
            if span.split_whitespace().count() >= self.p("head_bold_min_words")
                && !normalize(self, span).is_empty()
            {
                return span.to_string();
            }
        }
        normalize(self, item)
            .content
            .into_iter()
            .take(self.p("head_max_content_words"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl LanguagePack for English {
    fn id(&self) -> PackId {
        PackId {
            name: "en",
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    fn script_kind(&self) -> LanguageKind {
        LanguageKind::Natural
    }

    /// Runs of alphanumeric characters. Everything else separates tokens and is dropped.
    fn tokenize(&self, _span: &Span, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut start: Option<usize> = None;
        let push = |from: usize, to: usize, tokens: &mut Vec<Token>| {
            let word = &text[from..to];
            let class = if word.chars().all(|c| c.is_ascii_digit()) {
                TokenClass::Number
            } else {
                TokenClass::Word
            };
            tokens.push(Token {
                text: word.to_string(),
                span: from..to,
                norm: self.normalizer.fold_case(word),
                class,
            });
        };
        for (idx, ch) in text.char_indices() {
            match (ch.is_alphanumeric(), start) {
                (true, None) => start = Some(idx),
                (false, Some(from)) => {
                    push(from, idx, &mut tokens);
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(from) = start {
            push(from, text.len(), &mut tokens);
        }
        tokens
    }

    /// A sentence ends at `.`, `!` or `?` followed by whitespace or the end, unless the word before a
    /// full stop is a listed abbreviation. Ranges exclude surrounding whitespace.
    fn sentences(&self, text: &str) -> Vec<Range<usize>> {
        let mut out = Vec::new();
        let mut from = 0usize;
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        for (i, &(idx, ch)) in chars.iter().enumerate() {
            if !matches!(ch, '.' | '!' | '?') {
                continue;
            }
            let end = idx + ch.len_utf8();
            let at_boundary = chars
                .get(i + 1)
                .is_none_or(|&(_, next)| next.is_whitespace());
            if !at_boundary {
                continue;
            }
            if ch == '.' {
                let word_start = text[..idx]
                    .rfind(|c: char| c.is_whitespace())
                    .map_or(0, |p| p + 1);
                let word = self.normalizer.fold_case(&text[word_start..idx]);
                if self.lexicons.entries("abbreviations").contains(&word) {
                    continue;
                }
            }
            let piece = &text[from..end];
            let lead = piece.len() - piece.trim_start().len();
            if !piece.trim().is_empty() {
                out.push(from + lead..end);
            }
            from = end;
        }
        let tail = &text[from..];
        if !tail.trim().is_empty() {
            let lead = tail.len() - tail.trim_start().len();
            out.push(from + lead..from + tail.trim_end().len());
        }
        out
    }

    fn normalizer(&self) -> &dyn Normalizer {
        &self.normalizer
    }

    fn lexicons(&self) -> &LexiconSet {
        &self.lexicons
    }

    fn extract(&self, block: &Block) -> Vec<RawClaim> {
        if !matches!(block.kind, BlockKind::Content) {
            return Vec::new();
        }
        let mut out = Vec::new();
        let raw = |form: Form, text: String, head: Option<String>| RawClaim {
            form,
            text,
            head,
            span: block.span,
        };
        if let Some(item) = &block.list_item {
            let item = item.trim();
            if item.chars().count() >= self.p("item_min_chars") {
                let head = self.head_of(item, &block.strong);
                out.push(raw(Form::ListItem, item.to_string(), Some(head)));
            }
        }
        if let Some(cell) = &block.table_cell {
            let cell = cell.trim();
            if cell.chars().count() >= self.p("cell_min_chars") {
                out.push(raw(Form::TableCell, cell.to_string(), None));
            }
        }
        for span in &block.strong {
            let text = span.trim();
            let len = text.chars().count();
            if self.looks_like_name(span)
                && (self.p("name_min_chars")..=self.p("name_max_chars")).contains(&len)
            {
                out.push(raw(Form::Bold, text.to_string(), None));
            }
        }
        for span in &block.code {
            if self.looks_like_identifier(span) {
                out.push(raw(Form::CodeSpan, span.trim().to_string(), None));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_alphanumeric_runs_and_folds_case() {
        let en = English::new();
        let t = en.tokenize(&Span::default(), "Multi-path DAG, v2!");
        let norms: Vec<&str> = t.iter().map(|t| t.norm.as_str()).collect();
        assert_eq!(norms, ["multi", "path", "dag", "v2"]);
        assert_eq!(t[0].span, 0..5);
    }

    #[test]
    fn normalize_drops_one_leading_article_and_keeps_stopwords_in_the_key() {
        let en = English::new();
        let n = normalize(&en, "The graph is the model");
        assert_eq!(n.key, "graph is the model");
        assert_eq!(n.content, ["graph", "model"]);
        assert_eq!(normalize(&en, "A the x").key, "the x");
    }

    #[test]
    fn sentences_respect_abbreviations() {
        let en = English::new();
        let text = "Use a cache, e.g. an LRU. It is fast! Ok";
        let got: Vec<&str> = en.sentences(text).into_iter().map(|r| &text[r]).collect();
        assert_eq!(got, ["Use a cache, e.g. an LRU.", "It is fast!", "Ok"]);
    }

    #[test]
    fn name_rules() {
        let en = English::new();
        assert!(en.looks_like_name("Pattern harvest"));
        assert!(!en.looks_like_name("and then it ran"));
        assert!(!en.looks_like_name("One, two"));
        assert!(!en.looks_like_name("ALL CAPS SHOUT"));
        assert!(!en.looks_like_name("one two three four five six seven eight nine"));
        assert!(en.looks_like_identifier("pick_route"));
        assert!(en.looks_like_identifier("RuleRegistry"));
        assert!(!en.looks_like_identifier("plain"));
        assert!(!en.looks_like_identifier("two words"));
    }

    #[test]
    fn shipped_data_is_complete() {
        let en = English::new();
        for name in [
            "stopwords",
            "leading-articles",
            "name-lead-words",
            "name-reject-punct",
            "identifier-markers",
            "abbreviations",
        ] {
            assert!(
                !en.lexicons().entries(name).is_empty(),
                "lexicon {name} is empty"
            );
        }
        assert_eq!(en.lexicons().entries("name-reject-punct")[0], ", ");
    }
}
