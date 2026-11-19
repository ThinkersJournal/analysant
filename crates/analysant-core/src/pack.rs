// SPDX-License-Identifier: MIT OR Apache-2.0
//! The two seams (SPEC-STEP1 section 2): a `FormatAdapter` supplies structure, a `LanguagePack`
//! supplies everything about reading the text of a span.
//!
//! Deviation from the sketch in 2.2 and 3.2, recorded in the spec: `Token`, `NormalForm` and
//! [`normalize`] live here, not in `analysant-lexical`, because `LanguagePack::extract` must be able
//! to build a claim's key and core cannot depend on lexical. `analysant-lexical` re-exports them.

use crate::model::Span;
use std::collections::BTreeMap;
use std::ops::Range;

/// A pack's name and its semantic version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackId {
    pub name: &'static str,
    pub version: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageKind {
    Natural,
    Formal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TokenClass {
    Word,
    Number,
    Identifier,
    Punct,
    Date,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// As written.
    pub text: String,
    /// Byte range in the text that was tokenized.
    pub span: Range<usize>,
    /// Case-folded by the pack's normalizer.
    pub norm: String,
    pub class: TokenClass,
}

/// A phrase reduced to what matching compares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NormalForm {
    /// Tokens after a leading article is dropped.
    pub tokens: Vec<Token>,
    /// The `norm` of every token, joined by one space.
    pub key: String,
    /// The `norm` of every token that is not a stopword.
    pub content: Vec<String>,
}

impl NormalForm {
    /// Build a normal form from tokens already in normal order.
    pub fn from_tokens(tokens: Vec<Token>, is_stopword: impl Fn(&str) -> bool) -> Self {
        let key = tokens
            .iter()
            .map(|t| t.norm.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let content = tokens
            .iter()
            .filter(|t| !is_stopword(&t.norm))
            .map(|t| t.norm.clone())
            .collect();
        Self {
            tokens,
            key,
            content,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

/// The parts of normalization that vary by language. Word lists are the pack's data.
pub trait Normalizer: Send + Sync {
    /// Case folding. A pack hook because the default (`to_lowercase`) is wrong for some languages.
    fn fold_case(&self, text: &str) -> String;
    /// True when a folded token, opening a phrase, is dropped from its key.
    fn is_leading_article(&self, norm: &str) -> bool;
    fn is_stopword(&self, norm: &str) -> bool;
}

/// Named word lists loaded from a pack's data files.
#[derive(Clone, Debug, Default)]
pub struct LexiconSet {
    lexicons: BTreeMap<String, Vec<String>>,
    params: BTreeMap<String, usize>,
}

/// A data file that could not be read as a lexicon or a params table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableError {
    pub table: String,
    pub line: usize,
    pub reason: String,
}

impl std::fmt::Display for TableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} line {}: {}", self.table, self.line, self.reason)
    }
}

impl std::error::Error for TableError {}

impl LexiconSet {
    /// Add a lexicon from text: one entry per line; blank lines and `#` lines are skipped; an entry
    /// wrapped in double quotes keeps its inner whitespace. Entries are stored as written, not folded.
    pub fn add_lexicon(&mut self, name: &str, text: &str) {
        let entries = text
            .lines()
            .filter_map(|raw| {
                let line = raw.trim_end_matches('\r');
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    return None;
                }
                if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
                    return Some(trimmed[1..trimmed.len() - 1].to_string());
                }
                Some(trimmed.to_string())
            })
            .collect();
        self.lexicons.insert(name.to_string(), entries);
    }

    /// Add integer parameters from text: `name = value` per line.
    pub fn add_params(&mut self, table: &str, text: &str) -> Result<(), TableError> {
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let err = |reason: &str| TableError {
                table: table.to_string(),
                line: idx + 1,
                reason: reason.to_string(),
            };
            let (name, value) = line
                .split_once('=')
                .ok_or_else(|| err("expected `name = value`"))?;
            let value: usize = value
                .trim()
                .parse()
                .map_err(|_| err("value is not a non-negative integer"))?;
            self.params.insert(name.trim().to_string(), value);
        }
        Ok(())
    }

    /// The entries of a lexicon, empty when there is none of that name.
    pub fn entries(&self, name: &str) -> &[String] {
        self.lexicons.get(name).map_or(&[], Vec::as_slice)
    }

    pub fn has_lexicon(&self, name: &str) -> bool {
        self.lexicons.contains_key(name)
    }

    /// A parameter; a missing one is the caller's error to report, never silently defaulted.
    pub fn param(&self, name: &str) -> Option<usize> {
        self.params.get(name).copied()
    }

    pub fn lexicon_names(&self) -> impl Iterator<Item = &str> {
        self.lexicons.keys().map(String::as_str)
    }
}

/// A claim as a pack found it: form, text and place. A pack NEVER sets kind or status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawClaim {
    pub form: crate::model::Form,
    /// As written.
    pub text: String,
    /// The phrase the claim's key is made from, when that is not the whole text (a list item is
    /// named by its head). `None` means: key the whole text.
    pub head: Option<String>,
    pub span: Span,
}

/// What a format adapter says a block is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Heading {
        level: u8,
    },
    /// Any other line outside a code block.
    Content,
    /// The line naming a table's columns: it names columns, not components.
    TableHeader,
    /// A fenced code block. Its language is the pack named by `info`; slice 1 has no such pack.
    CodeBlock {
        info: String,
    },
}

/// One unit of document structure, as a format adapter reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockKind,
    /// 1-based line number of the block's first line.
    pub line: u32,
    pub span: Span,
    /// The headings above this block, outermost first (not including the block itself).
    pub section: Vec<String>,
    /// The block's text: a heading's title, a content line, a code block's body.
    pub text: String,
    /// Text of the list item on this line, marker removed.
    pub list_item: Option<String>,
    /// First cell of a table row.
    pub table_cell: Option<String>,
    /// Emphasised spans on this line.
    pub strong: Vec<String>,
    /// Code spans on this line.
    pub code: Vec<String>,
}

/// The document-format axis: structure in, blocks out.
pub trait FormatAdapter: Send + Sync {
    fn id(&self) -> PackId;
    fn blocks(&self, text: &str) -> Vec<Block>;
}

/// The language axis.
pub trait LanguagePack: Send + Sync {
    fn id(&self) -> PackId;
    fn script_kind(&self) -> LanguageKind;
    fn tokenize(&self, span: &Span, text: &str) -> Vec<Token>;
    /// Sentence boundaries as byte ranges. Natural languages use rules held as data.
    fn sentences(&self, text: &str) -> Vec<Range<usize>>;
    fn normalizer(&self) -> &dyn Normalizer;
    fn lexicons(&self) -> &LexiconSet;
    /// Claims in one block: form, text and span only.
    fn extract(&self, block: &Block) -> Vec<RawClaim>;
}

/// Tokenize, drop a leading article, and fold. Deterministic.
pub fn normalize(pack: &dyn LanguagePack, text: &str) -> NormalForm {
    let span = Span {
        start: 0,
        end: text.len(),
        ..Span::default()
    };
    let mut tokens = pack.tokenize(&span, text);
    let norm = pack.normalizer();
    if tokens
        .first()
        .is_some_and(|t| norm.is_leading_article(&t.norm))
    {
        tokens.remove(0);
    }
    NormalForm::from_tokens(tokens, |w| norm.is_stopword(w))
}
