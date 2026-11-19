// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant-rust`: the Rust language pack (SPEC-STEP1 2.6, slice 5). Code is the ground truth a
//! document's claims are checked against: this crate reads `.rs` files with `syn` into items (what is
//! defined), uses (where each name appears in code, and whether that place is production or test code)
//! and comment mentions (where a name appears only in prose), and answers one question per name:
//! `Implements`, `Tests` or `Missing`, with the evidence and the *scope of the reachability claim*
//! attached (section 5.1, "verdict vocabulary").
//!
//! The reachability claim is deliberately small and named: use counting is **by bare name**, and
//! "called" looks one hop up the chain (a production caller that is itself never called in production is
//! reported as such, not as a bare "built"). There is no call graph and no macro expansion.
//! Every word list the rules read is a data file under `data/rust/`.

mod cfgeval;
mod comments;
mod index;
mod pack;
mod relate;

pub use index::{CodeIndex, CodeItem, CodeItemKind, Context, ItemId, ParseFailure, Use};
pub use pack::{RustPack, split_ident_words};
pub use relate::{Mentions, Reach, Relation, Scope, Site, Verdict};
