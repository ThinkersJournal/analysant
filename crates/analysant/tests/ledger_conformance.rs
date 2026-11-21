// SPDX-License-Identifier: MIT OR Apache-2.0
//! The ledger-writer conformance kit (SPEC-STEP1 1.9, "Ledger writer: EditAnt"). A tool that writes
//! `.analysant/ledger.jsonl` must produce files this crate reads back unchanged, and must make
//! the ids the fixture pins. The golden file is `tests/fixtures/ledger-conformance/canonical.jsonl`;
//! `examples/ledger_writer.rs` is a reference writer that produces it byte for byte.

#![allow(dead_code)]

use analysant::ledger_file::{
    LedgerFileError, UnitKindName, build, history, merge_text, parse, render, state,
};
use analysant_core::{UnitId, UnitKind, content_id};

#[path = "../examples/ledger_writer.rs"]
mod writer;

const CANONICAL: &str = include_str!("fixtures/ledger-conformance/canonical.jsonl");

#[test]
fn the_reference_writer_reproduces_the_golden_file_byte_for_byte() {
    assert_eq!(writer::canonical(), CANONICAL);
}

#[test]
fn the_golden_file_reads_and_writes_back_unchanged() {
    let events = parse(CANONICAL).expect("the golden file parses");
    assert_eq!(events.len(), 5, "the header is not an event");
    assert_eq!(render(&events), CANONICAL, "render(parse(f)) == f");
}

#[test]
fn unit_ids_are_the_pinned_content_ids() {
    // The unit id of "The cache is cold." is content_id's published stability vector's cousin:
    // same function, document a.md, path Top/Sub, occurrence 0, generation 0.
    let path = vec!["Top".to_string(), "Sub".to_string()];
    let a = content_id(UnitKind::Claim, "a.md", &path, "The cache is cold.", 0, 0);
    assert_eq!(a.to_string(), "G2A77DBEZX5JUGV2OOOVSG7P6S");
    let events = parse(CANONICAL).unwrap();
    assert_eq!(events[0].unit(), Some(a));
}

#[test]
fn event_ids_are_pinned() {
    let ids: Vec<String> = parse(CANONICAL)
        .unwrap()
        .iter()
        .map(|e| e.id().to_string())
        .collect();
    assert_eq!(
        ids,
        [
            "2b55c072a9bce3dd261a07212533a033",
            "5541bf859ff881cd6a1a55ce585eebaf",
            "6cec8c7d56b2375d79ddd1f2454e265b",
            "3907d4293a0e0e8d088891cd50598ac7",
            "a77891b3be4158841bed56bf5120ca86",
        ]
    );
}

#[test]
fn state_and_history_of_the_golden_file() {
    let events = parse(CANONICAL).unwrap();
    let a = events[0].unit().unwrap();
    let h = history(&events, a);
    assert_eq!(h.len(), 2, "old text is kept: cold then warm");
    assert_eq!(h[0].1, "The cache is cold.");
    assert_eq!(h[1].1, "The cache is warm.");
    assert_eq!(
        state(&events).unwrap().len(),
        2,
        "two units; one of them is gone"
    );
}

#[test]
fn appending_an_event_is_a_union_with_the_old_file() {
    // A writer appends; it never rewrites. Appending = merging the old file with the new events.
    let u = UnitId::from_raw(42);
    let extra = render(&[build::minted(u, UnitKindName::Claim, "b.md", &[], "New.")]);
    let merged = merge_text(CANONICAL, &extra).unwrap();
    let back = parse(&merged.text).unwrap();
    assert_eq!(back.len(), 6);
    assert!(
        parse(CANONICAL)
            .unwrap()
            .iter()
            .all(|old| back.iter().any(|e| e.id() == old.id())),
        "every earlier event survives, unedited"
    );
    // Merging again changes nothing (idempotent), and either side order gives the same bytes.
    assert_eq!(merge_text(&merged.text, &extra).unwrap().text, merged.text);
    assert_eq!(merge_text(&extra, CANONICAL).unwrap().text, merged.text);
}

#[test]
fn a_writer_that_gets_one_of_these_wrong_is_refused_not_trusted() {
    let lines: Vec<&str> = CANONICAL.lines().collect();
    // No header.
    let no_header = lines[1..].join("\n");
    assert!(matches!(
        parse(&no_header),
        Err(LedgerFileError::MissingHeader { .. })
    ));
    // A hand-edited event (the id no longer matches the content).
    let edited = CANONICAL.replace("The cache is warm.", "The cache is hot.");
    assert!(parse(&edited).is_err());
    // An unknown field.
    let extra_field = CANONICAL.replacen("\"revision\":1}", "\"revision\":1,\"note\":\"x\"}", 1);
    assert!(parse(&extra_field).is_err());
    // A future format version names both versions.
    let v2 = CANONICAL.replacen("\"version\":1", "\"version\":2", 1);
    assert!(matches!(
        parse(&v2),
        Err(LedgerFileError::UnsupportedVersion { .. })
    ));
}
