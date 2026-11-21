// SPDX-License-Identifier: MIT OR Apache-2.0
//! A reference ledger writer: what a writing tool (EditAnt, SPEC-STEP1 1.9 "Ledger writer: EditAnt")
//! does with this crate. It mints one claim with `content_id`, records a rewording, a move and a
//! removal, and prints the file. The output is byte-for-byte
//! `tests/fixtures/ledger-conformance/canonical.jsonl`; the test `ledger_conformance` holds it there.
//!
//! Run: `cargo run -p analysant --example ledger_writer`

use analysant::ledger_file::{UnitKindName, build, render};
use analysant_core::{UnitKind, content_id};

fn main() {
    print!("{}", canonical());
}

/// The canonical file: one claim minted, reworded, moved and then removed, plus a second claim that
/// stays. A writer appends events in this shape and never edits or deletes an earlier one.
pub fn canonical() -> String {
    let path = |p: &[&str]| p.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    let a = content_id(
        UnitKind::Claim,
        "a.md",
        &path(&["Top", "Sub"]),
        "The cache is cold.",
        0,
        0,
    );
    let b = content_id(
        UnitKind::Claim,
        "a.md",
        &path(&["Top"]),
        "The queue is empty.",
        0,
        0,
    );
    let events = vec![
        build::minted(
            a,
            UnitKindName::Claim,
            "a.md",
            &path(&["Top", "Sub"]),
            "The cache is cold.",
        ),
        build::revised(a, "a.md", 2, "The cache is warm."),
        build::moved(a, "a.md", 2, &path(&["Top", "Other"])),
        build::gone(a, "a.md", 2),
        build::minted(
            b,
            UnitKindName::Claim,
            "a.md",
            &path(&["Top"]),
            "The queue is empty.",
        ),
    ];
    render(&events)
}
