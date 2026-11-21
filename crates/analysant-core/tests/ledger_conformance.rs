// SPDX-License-Identifier: MIT OR Apache-2.0
//! The ledger-writer conformance kit (SPEC-STEP1 1.9, "Ledger writer: EditAnt"). A tool that writes
//! `.analysant/ledger.jsonl` must produce files this crate reads back unchanged, and must make
//! the ids the fixture pins. The golden file is `tests/fixtures/ledger-conformance/canonical.jsonl`;
//! `examples/ledger_writer.rs` is a reference writer that produces it byte for byte.

#![allow(dead_code)]

use analysant_core::ledger_file::{
    LedgerFileError, UnitKindName, build, history, merge_text, parse, render, state,
};
use analysant_core::{UnitId, UnitKind, content_id};

#[path = "../examples/ledger_writer.rs"]
mod writer;

const CANONICAL: &str = include_str!("fixtures/ledger-conformance/canonical.jsonl");
const ESCAPING: &str = include_str!("fixtures/ledger-conformance/escaping.jsonl");
const REWRITE: &str = include_str!("fixtures/ledger-conformance/rewrite.jsonl");

/// R2: a sentence deleted and written again gets a NEW id (generation 1), so an id is never reused.
/// The ids come from `reidentify`, which reads the file's state; `claim.unit.id` from a throwaway
/// `Ledger::default()` is generation 0 and is never what a writer records for a re-written sentence.
#[test]
fn a_sentence_deleted_and_written_again_gets_a_new_id_in_the_rewrite_golden() {
    assert_eq!(writer::rewrite(), REWRITE);
    let events = parse(REWRITE).unwrap();
    assert_eq!(render(&events), REWRITE);
    let text = "The intake service validates every request.";
    let path = vec!["Plan".to_string()];
    let first = content_id(UnitKind::Claim, "a.md", &path, text, 0, 0);
    let again = content_id(UnitKind::Claim, "a.md", &path, text, 0, 1);
    assert_ne!(first, again);
    let units: Vec<UnitId> = events
        .iter()
        .filter_map(|e| match e {
            analysant_core::ledger_file::Event::Minted { text: t, .. } if t == text => e.unit(),
            _ => None,
        })
        .collect();
    assert_eq!(
        units.len(),
        2,
        "the sentence is minted twice (the first is gone)"
    );
    assert!(units.contains(&first) && units.contains(&again));
    // The first one is gone; the rewritten one is live.
    let st = state(&events).unwrap();
    assert!(st.iter().find(|s| s.unit == first).unwrap().gone);
    assert!(!st.iter().find(|s| s.unit == again).unwrap().gone);
}

/// G7: the second golden file. Non-ASCII, quote, backslash, `/`, tab, newline, U+001F, U+007F, U+2028
/// and an astral character, in the text, the section path and the document path.
#[test]
fn the_escaping_golden_is_what_the_reference_writer_writes_and_survives_a_round_trip() {
    assert_eq!(writer::escaping(), ESCAPING);
    let events = parse(ESCAPING).unwrap();
    assert_eq!(render(&events), ESCAPING);
    // Raw, not escaped: non-ASCII, `/`, U+007F and U+2028 appear as themselves in the file.
    for raw in [
        "Caf\u{e9}",
        "\u{65e5}\u{672c}",
        "1/2",
        "\u{7f}",
        "\u{2028}",
        "\u{1f600}",
    ] {
        assert!(ESCAPING.contains(raw), "{raw:?} must be written raw");
    }
    // Escaped: the short forms and \u001f (lower-case hex), never \u000a or \u0009.
    for esc in ["\\t", "\\n", "\\\"", "\\\\", "\\u001f"] {
        assert!(ESCAPING.contains(esc), "{esc} must be written as an escape");
    }
    assert!(!ESCAPING.contains("\\u000a") && !ESCAPING.contains("\\u0009"));
}

/// G7: a reader takes any valid JSON escaping and normalises it on the next write.
#[test]
fn a_reader_accepts_other_valid_escapings_and_rewrites_them_canonically() {
    let other = ESCAPING
        .replace("Caf\u{e9}", "Caf\\u00e9")
        .replace("1/2", "1\\/2");
    assert_ne!(other, ESCAPING);
    assert_eq!(render(&parse(&other).unwrap()), ESCAPING);
}

/// G9: CRLF line ends (Windows autocrlf) are read; a write always uses LF.
#[test]
fn crlf_line_endings_are_read_and_a_write_is_lf() {
    let crlf = CANONICAL.replace('\n', "\r\n");
    assert_eq!(render(&parse(&crlf).unwrap()), CANONICAL);
    assert_eq!(merge_text(&crlf, CANONICAL).unwrap().text, CANONICAL);
}

/// G4: revision numbers. `minted` is revision 1; only `revised` raises it, to the unit's latest
/// revision plus one; `moved` and `gone` carry the revision the unit has when they are written.
#[test]
fn the_golden_file_obeys_the_revision_rules() {
    use analysant_core::ledger_file::Event;
    let events = parse(CANONICAL).unwrap();
    let mut latest: std::collections::BTreeMap<String, u32> = Default::default();
    for ev in &events {
        let unit = ev.unit_text().to_string();
        match ev {
            Event::Minted { revision, .. } => {
                assert_eq!(*revision, 1);
                latest.insert(unit, 1);
            }
            Event::Revised { revision, .. } => {
                let cur = latest.get_mut(&unit).unwrap();
                assert_eq!(*revision, *cur + 1, "revised = latest + 1");
                *cur = *revision;
            }
            Event::Moved { revision, .. } | Event::Gone { revision, .. } => {
                assert_eq!(
                    *revision, latest[&unit],
                    "moved/gone carry the current revision"
                );
            }
        }
    }
}

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
