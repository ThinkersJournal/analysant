// SPDX-License-Identifier: MIT OR Apache-2.0
//! What a lens declares about itself (SPEC-STEP1 4.1): plain data, so a later tool project can publish one tool per lens without
//! lens-specific glue. Std only; core takes no MCP dependency and no JSON-schema dependency (the spec's `schemars` is a PROPOSAL,
//! not built in slice 6 part 1).

use crate::model::LensId;

/// How expensive a lens is to run, so a caller can order or skip them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cost {
    /// Linear in the corpus, no model call.
    Cheap,
    /// Wants the whole corpus at once (pairing, indexing).
    Batch,
}

/// A lens's declared shape. A lens has no side effects by construction: it returns values and writes nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LensSpec {
    pub id: LensId,
    /// One sentence: what finding it produces and what it does not claim.
    pub description: String,
    /// Lenses whose output this one reads.
    pub requires: Vec<LensId>,
    pub cost: Cost,
    /// Whether it needs a language pack for code, not only prose.
    pub reads_code: bool,
    /// The finding kinds it can produce, as `FindingKind::label` strings.
    pub kinds: Vec<String>,
}
