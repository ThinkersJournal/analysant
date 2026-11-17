//! The claim model (spike slice).
//!
//! A claim is the unit the analysis works on. It has stable identity
//! (source file + line), text, a normalized key for matching, the section
//! it sits under, and a kind. Nothing here depends on language: the model
//! is the same whether the claim came from English prose or a Rust
//! signature. Language-specific work lives in the parsers, not the model.

/// What the text is doing (the pragmatic lens, minimal for the spike).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimKind {
    /// A directive: something to build or do ("build the multi-path DAG").
    Directive,
    /// An assertion: something claimed to be true ("the plan IS the graph").
    Assertion,
    /// A named capability: a bold phrase or table component.
    Named,
    /// A code identifier from a backtick span (`Op::Branch`).
    /// Reported separately: identifiers are code facts, not directives,
    /// so an identifier absent from the ROADMAP is weak evidence.
    Identifier,
}

/// What a claim's text is doing, per the pragmatic lens (slice 1).
/// Assigned by `pragmatic::classify_*` at extraction time; the
/// directed-but-untasked report counts only claims whose role matches
/// their kind (see `pragmatic::is_reportable`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PragmaticRole {
    /// The text directs work: build, do, or support this.
    Directive,
    /// The text describes or asserts what is (a definition, a rule, a
    /// cross-reference): not a directive, not a name.
    Description,
    /// The text reports status or history: as-built, dated, retired,
    /// superseded, gone.
    StatusNote,
    /// The text names a capability (a naming act, not a sentence).
    Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// Stable identity: `file:line` of first occurrence.
    pub id: String,
    /// Raw text as written.
    pub text: String,
    /// Normalized key used for matching (see `normalize`).
    pub key: String,
    pub source_file: String,
    pub line: usize,
    /// Heading path the claim sits under, e.g. "The optimized form".
    pub section: String,
    pub kind: ClaimKind,
    /// What the text is doing (pragmatic lens, slice 1).
    pub role: PragmaticRole,
}
