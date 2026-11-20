// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant`: document analysis for plans, specs and code. Slice 1 reads claims out of documents
//! (`corpus`) and checks them against a plan (`untasked`), on `analysant-core` and
//! `analysant-lexical`. See docs/SPEC-STEP1.md.

pub mod contradiction;
pub mod corpus;
pub mod gate_b;
pub mod identity;
pub mod judging;
pub mod ledger_file;
pub mod ownership;
pub mod pragmatic;
pub mod produce;
pub mod project;
pub mod tool;
pub mod untasked;

pub use corpus::{Corpus, Document, extract_corpus, extract_document};
pub use ownership::{OwnedPartition, Owners, OwnershipConfig, owner_for, owners, partition_owned};
pub use pragmatic::{Partition, classify_corpus, partition};
pub use project::{ProjectConfig, ProjectError};
pub use tool::{
    ClaimDetail, Coverage, ExplainError, Explanation, HealthParams, KindCount, KindStatus,
    LensOutput, NullCheck, Report, Silence, TopFinding, explain, health_report,
};
pub use untasked::{LENS, Outcome, Status, UntaskedRun, directed_but_untasked};
