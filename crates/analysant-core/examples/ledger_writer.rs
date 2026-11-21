// SPDX-License-Identifier: MIT OR Apache-2.0
//! A reference ledger writer: what a writing tool (EditAnt, SPEC-STEP1 1.9 "Ledger writer: EditAnt")
//! does with this crate. It mints one claim with `content_id`, records a rewording, a move and a
//! removal, and prints the file. The output is byte-for-byte
//! `tests/fixtures/ledger-conformance/canonical.jsonl`; the test `ledger_conformance` holds it there.
//!
//! Run: `cargo run -p analysant-core --example ledger_writer`

use analysant_core::ledger_file::{UnitKindName, build, render};
use analysant_core::{UnitKind, content_id};

fn main() {
    // `ledger_writer` prints the canonical file; `ledger_writer escaping` prints the escaping file.
    if std::env::args().nth(1).as_deref() == Some("escaping") {
        print!("{}", escaping());
    } else {
        print!("{}", canonical());
    }
}

/// The escaping file: one claim whose text and path carry every character JSON writes specially,
/// and non-ASCII. The writer emits what `serde_json` emits: `"` and backslash escaped; backspace,
/// form feed, newline, carriage return and tab as `\b \f \n \r \t`; any other control character
/// below U+0020 as `\u00xx` (lower-case hex); U+007F, `/`, U+2028 and every non-ASCII character raw,
/// in UTF-8 (never `\u` escapes for them, no surrogate pairs). A reader accepts any valid JSON
/// escaping; a file a writer produces is byte-identical to the golden file for this content.
pub fn escaping() -> String {
    let path = vec![
        "Caf\u{e9} \u{65e5}\u{672c}".to_string(),
        "a/b \"q\" \\".to_string(),
    ];
    let text = "Tab\there, \"quote\", back\\slash, 1/2, nl\nx, \u{1f}unit-sep, \u{7f}del, \u{2028}ls, emoji \u{1f600}.";
    let u = content_id(UnitKind::Claim, "d/\u{e9}.md", &path, text, 0, 0);
    render(&[build::minted(
        u,
        UnitKindName::Claim,
        "d/\u{e9}.md",
        &path,
        text,
    )])
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
