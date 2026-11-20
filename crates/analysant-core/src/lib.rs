// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant-core`: the claim model, the two seams (format adapter, language pack), findings with
//! their search records and controls, the in-memory ledger, and the `en` pack. See docs/SPEC-STEP1.md.
//!
//! Std only: no dependencies. Core never classifies a claim.

pub mod en;
pub mod finding;
pub mod ledger;
pub mod lens;
#[cfg(feature = "markdown")]
pub mod markdown;
pub mod model;
pub mod pack;

pub use en::English;
pub use finding::{
    Basis, ControlKind, ControlOutcome, ControlResult, Disposition, Finding, FindingId,
    FindingKind, QueryDescription, ScopeDescription, SearchRecord,
};
pub use ledger::{Ledger, LedgerEvent};
pub use lens::{Cost, LensSpec};
#[cfg(feature = "markdown")]
pub use markdown::Markdown;
pub use model::*;
pub use pack::{
    Block, BlockKind, FormatAdapter, LanguageKind, LanguagePack, LexiconSet, NormalForm,
    Normalizer, PackId, RawClaim, TableError, Token, TokenClass, normalize,
};
