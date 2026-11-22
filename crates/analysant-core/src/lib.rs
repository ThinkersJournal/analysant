// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant-core`: the claim model, the two seams (format adapter, language pack), findings with
//! their search records and controls, the in-memory ledger, and the `en` pack. See docs/SPEC-STEP1.md.
//!
//! Depends on `serde` and `serde_json` only (the ledger file format); no other dependency. Core never classifies a claim.
//!
//! **API stability (pre-1.0).** The pinned surface is `content_id`, `UnitKind` and `UnitId` (and `UnitId`'s
//! `Display`/`parse`): `content_id`'s output is pinned by the test `content_id_stability_vector` and changes
//! only as a Breaking release. The ledger **writer flow** (`extract_document`, `identity::observations` and
//! `identity::reidentify`, `ledger_file::state` and `ledger_file::write_file`) is public and is what an editor
//! integration calls: the ledger *file format* is its contract (versioned, pinned by the goldens in
//! `tests/fixtures/ledger-conformance`, rules in `docs/LEDGER-WRITER-CONFORMANCE.md`), and a change to the format
//! or to those signatures ships as a Breaking release (the second number before 1.0) with a CHANGELOG entry,
//! never silently inside a patch. It is not frozen: it may change in any 0.n minor release, like everything else
//! here (lens, finding, language-pack and format-adapter seams, the English pack).
//!
//! **What a unit id is.** An identity, not an integrity check: two 64-bit FNV-1a hashes in a `u128`, not
//! cryptographic; collisions are possible and can be constructed on purpose, and no claim is made against
//! an adversary.

pub mod corpus;
pub mod en;
pub mod finding;
pub mod identity;
pub mod ledger;
pub mod ledger_file;
pub mod lens;
#[cfg(feature = "markdown")]
pub mod markdown;
pub mod model;
pub mod pack;

pub use corpus::{Corpus, Document, extract_corpus, extract_document};
pub use en::English;
pub use finding::{
    Basis, ControlKind, ControlOutcome, ControlResult, Disposition, Finding, FindingId,
    FindingKind, QueryDescription, ScopeDescription, SearchRecord,
};
pub use ledger::{Ledger, LedgerEvent, content_id};
pub use lens::{Cost, LensSpec};
#[cfg(feature = "markdown")]
pub use markdown::Markdown;
pub use model::*;
pub use pack::{
    Block, BlockKind, FormatAdapter, LanguageKind, LanguagePack, LexiconSet, NormalForm,
    Normalizer, PackId, RawClaim, TableError, Token, TokenClass, normalize,
};
