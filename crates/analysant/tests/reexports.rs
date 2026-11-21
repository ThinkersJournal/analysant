// SPDX-License-Identifier: MIT OR Apache-2.0
//! `corpus` and `ledger_file` moved into `analysant-core` (kit gap G2: a core-only publish must be
//! enough for a writing tool). Every `analysant::` path that existed before the move still works,
//! and names the same items.

use analysant::corpus::{Corpus, Document, extract_corpus, extract_document};
use analysant::identity::{IdentityParams, Observation, reidentify};
use analysant::ledger_file::{
    Conflict, Event, LEDGER_FORMAT_VERSION, LEDGER_PATH, LedgerFileError, Merged, MergedText,
    UnitKindName, UnitState, build, history, merge, merge_text, parse, read_file, render, state,
    text_hash, write_file,
};

#[test]
fn the_old_paths_are_the_core_items() {
    // Same type: a value built through one path is accepted through the other.
    let ev: analysant_core::ledger_file::Event =
        build::revised(analysant_core::UnitId::from_raw(1), "a.md", 2, "t");
    let _: &Event = &ev;
    let _: fn(&[Event]) -> String = render;
    let d: Option<Document> = None;
    let c: Option<analysant_core::corpus::Document> = d;
    assert!(c.is_none());
    let _ = (
        Corpus::default(),
        LEDGER_FORMAT_VERSION,
        LEDGER_PATH,
        text_hash("x"),
    );
    let _ = (
        extract_corpus as fn(_, _, _, _) -> _,
        extract_document as fn(_, _, _, _, _) -> _,
    );
    let _: Option<(
        Conflict,
        Merged,
        MergedText,
        UnitState,
        UnitKindName,
        LedgerFileError,
    )> = None;
    let _ = (
        history, merge, merge_text, parse, read_file, state, write_file,
    );
    // The identity module moved too.
    let _ = reidentify;
    let _ = IdentityParams {
        edit_similarity_permille: 1,
    };
    let _: Option<Observation> = None;
    // And the crate-root re-exports that existed before.
    let _ = (analysant::extract_corpus, analysant::extract_document);
    let _: Option<analysant::Corpus> = None;
}
