//! doc-analysis-spike: the first slices of the documentation tooling.
//!
//! Two lenses: cross-document discrepancy of the directed-but-untasked
//! kind (architecture docs name capabilities; the ROADMAP is the
//! working plan), and doc-vs-doc contradiction (two documents disagree
//! about the same capability, one denying it and one asserting it).
//! Everything here is deterministic: structure parsing, lexicon-based
//! pragmatic classification, token-subset matching. No ML, no
//! embeddings. See the requirements note
//! (`llm-documentation-tooling-requirements-2026-11-16.md`) for the design.

pub mod check;
pub mod claim;
pub mod contradiction;
pub mod names;
pub mod normalize;
pub mod ownership;
pub mod parse;
pub mod pragmatic;
pub mod rust_code;
