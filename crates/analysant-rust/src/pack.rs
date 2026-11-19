// SPDX-License-Identifier: MIT OR Apache-2.0
//! The pack: identifier tokens, the identifier-splitting normalizer, and the data files the rules read.

use analysant_core::{
    Block, LanguageKind, LanguagePack, LexiconSet, Normalizer, PackId, RawClaim, Span, Token,
    TokenClass,
};
use std::collections::HashSet;
use std::ops::Range;

const TEST_ATTRIBUTES: &str = include_str!("../data/rust/test-attributes.txt");
const TEST_CFG_PREDICATE: &str = include_str!("../data/rust/test-cfg-predicate.txt");
const CFG_ATTRIBUTE: &str = include_str!("../data/rust/cfg-attribute.txt");
const CFG_ALL: &str = include_str!("../data/rust/cfg-all.txt");
const CFG_ANY: &str = include_str!("../data/rust/cfg-any.txt");
const CFG_NOT: &str = include_str!("../data/rust/cfg-not.txt");
const TEST_PATH_SEGMENTS: &str = include_str!("../data/rust/test-path-segments.txt");
const TEST_CRATE_WORDS: &str = include_str!("../data/rust/test-crate-words.txt");
const PATH_SEPARATOR: &str = include_str!("../data/rust/path-separator.txt");
const ENTRY_POINTS: &str = include_str!("../data/rust/entry-points.txt");
const PARAMS: &str = include_str!("../data/rust/params.txt");

const REQUIRED_PARAMS: &[&str] = &["max_listed_sites"];

/// Split one identifier into lowercase words: snake_case on `_`, CamelCase on a lower-to-upper
/// boundary, SCREAMING_SNAKE on `_`. `pick_route` -> [pick, route]; `RuleRegistry` -> [rule, registry].
/// The spike's rule, reused as an idea (SPEC-STEP1 2.6).
pub fn split_ident_words(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut prev_lower_or_digit = false;
    for ch in name.chars() {
        if ch == '_' {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            prev_lower_or_digit = false;
        } else if ch.is_uppercase() {
            if prev_lower_or_digit && !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            cur.extend(ch.to_lowercase());
            prev_lower_or_digit = false;
        } else {
            cur.push(ch);
            prev_lower_or_digit = ch.is_lowercase() || ch.is_ascii_digit();
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

struct IdentNormalizer;

impl Normalizer for IdentNormalizer {
    fn fold_case(&self, text: &str) -> String {
        text.to_lowercase()
    }
    fn is_leading_article(&self, _norm: &str) -> bool {
        false
    }
    fn is_stopword(&self, _norm: &str) -> bool {
        false
    }
}

/// The Rust pack. Construction checks that every data file the rules read is present and every
/// required parameter is set; a missing one is an error, never a silent default.
pub struct RustPack {
    lexicons: LexiconSet,
    normalizer: IdentNormalizer,
    test_attributes: HashSet<String>,
    test_predicates: HashSet<String>,
    cfg_attribute: HashSet<String>,
    cfg_all: HashSet<String>,
    cfg_any: HashSet<String>,
    cfg_not: HashSet<String>,
    test_path_segments: HashSet<String>,
    test_crate_words: Vec<String>,
    entry_points: HashSet<String>,
}

impl Default for RustPack {
    fn default() -> Self {
        Self::new()
    }
}

impl RustPack {
    /// # Panics
    /// If a data file embedded in this crate is malformed (a build-time defect, caught by the tests).
    pub fn new() -> Self {
        let mut lexicons = LexiconSet::default();
        for (name, text) in [
            ("test-attributes", TEST_ATTRIBUTES),
            ("test-cfg-predicate", TEST_CFG_PREDICATE),
            ("cfg-attribute", CFG_ATTRIBUTE),
            ("cfg-all", CFG_ALL),
            ("cfg-any", CFG_ANY),
            ("cfg-not", CFG_NOT),
            ("test-path-segments", TEST_PATH_SEGMENTS),
            ("test-crate-words", TEST_CRATE_WORDS),
            ("entry-points", ENTRY_POINTS),
            ("path-separator", PATH_SEPARATOR),
        ] {
            lexicons.add_lexicon(name, text);
        }
        lexicons
            .add_params("rust params", PARAMS)
            .unwrap_or_else(|e| panic!("{e}"));
        for p in REQUIRED_PARAMS {
            assert!(
                lexicons.param(p).is_some(),
                "rust pack: parameter {p} missing"
            );
        }
        let set = |n: &str| -> HashSet<String> { lexicons.entries(n).iter().cloned().collect() };
        let pack = Self {
            test_attributes: set("test-attributes"),
            test_predicates: set("test-cfg-predicate"),
            cfg_attribute: set("cfg-attribute"),
            cfg_all: set("cfg-all"),
            cfg_any: set("cfg-any"),
            cfg_not: set("cfg-not"),
            test_path_segments: set("test-path-segments"),
            test_crate_words: lexicons.entries("test-crate-words").to_vec(),
            entry_points: set("entry-points"),
            normalizer: IdentNormalizer,
            lexicons,
        };
        for n in [
            "test-attributes",
            "test-cfg-predicate",
            "cfg-attribute",
            "cfg-all",
            "cfg-any",
            "cfg-not",
            "test-path-segments",
            "entry-points",
            "path-separator",
        ] {
            assert!(
                !pack.lexicons.entries(n).is_empty(),
                "rust pack: lexicon {n} is empty"
            );
        }
        pack
    }

    /// A path (relative, `/`-separated) is test code when a segment is a test directory or file, or
    /// when its first segment (the crate directory) contains a test-support word.
    pub fn is_test_path(&self, rel_path: &str) -> bool {
        let mut segments = rel_path.split('/');
        let first = segments.next();
        if first.is_some_and(|f| self.test_path_segments.contains(f)) {
            return true;
        }
        if segments.any(|s| self.test_path_segments.contains(s)) {
            return true;
        }
        first.is_some_and(|f| self.test_crate_words.iter().any(|w| f.contains(w.as_str())))
    }

    /// The separator in a qualified name (`Type` + separator + `member`).
    pub fn path_separator(&self) -> &str {
        self.lexicons
            .entries("path-separator")
            .first()
            .map_or("", String::as_str)
    }

    pub(crate) fn is_test_attribute(&self, name: &str) -> bool {
        self.test_attributes.contains(name)
    }
    pub(crate) fn is_test_predicate(&self, name: &str) -> bool {
        self.test_predicates.contains(name)
    }
    pub(crate) fn is_cfg_attribute(&self, name: &str) -> bool {
        self.cfg_attribute.contains(name)
    }
    pub(crate) fn is_cfg_all(&self, name: &str) -> bool {
        self.cfg_all.contains(name)
    }
    pub(crate) fn is_cfg_any(&self, name: &str) -> bool {
        self.cfg_any.contains(name)
    }
    pub(crate) fn is_cfg_not(&self, name: &str) -> bool {
        self.cfg_not.contains(name)
    }
    pub(crate) fn is_entry_point(&self, name: &str) -> bool {
        self.entry_points.contains(name)
    }
    pub(crate) fn max_listed_sites(&self) -> usize {
        self.lexicons.param("max_listed_sites").unwrap_or(1)
    }
}

impl LanguagePack for RustPack {
    fn id(&self) -> PackId {
        PackId {
            name: "rust",
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    fn script_kind(&self) -> LanguageKind {
        LanguageKind::Formal
    }

    /// Identifier tokens: each maximal run of alphanumerics and `_`, normalized to its words joined
    /// by a space (`pick_route` -> `pick route`).
    fn tokenize(&self, span: &Span, text: &str) -> Vec<Token> {
        let _ = span;
        let mut out = Vec::new();
        let mut start: Option<usize> = None;
        let push = |range: Range<usize>, out: &mut Vec<Token>| {
            let word = &text[range.clone()];
            let class = if word.chars().all(|c| c.is_ascii_digit()) {
                TokenClass::Number
            } else {
                TokenClass::Identifier
            };
            out.push(Token {
                text: word.to_string(),
                span: range,
                norm: split_ident_words(word).join(" "),
                class,
            });
        };
        for (i, ch) in text.char_indices() {
            let in_ident = ch.is_alphanumeric() || ch == '_';
            match (in_ident, start) {
                (true, None) => start = Some(i),
                (false, Some(s)) => {
                    push(s..i, &mut out);
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(s) = start {
            push(s..text.len(), &mut out);
        }
        out
    }

    /// An item is the unit, not a sentence: the whole text is one range.
    fn sentences(&self, text: &str) -> Vec<Range<usize>> {
        std::iter::once(0..text.len())
            .filter(|r| !r.is_empty())
            .collect()
    }

    fn normalizer(&self) -> &dyn Normalizer {
        &self.normalizer
    }

    fn lexicons(&self) -> &LexiconSet {
        &self.lexicons
    }

    /// A format block (Markdown) carries no Rust items; code items come from `CodeIndex::add_source`.
    fn extract(&self, _block: &Block) -> Vec<RawClaim> {
        Vec::new()
    }
}
