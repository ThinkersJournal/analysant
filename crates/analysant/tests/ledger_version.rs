// SPDX-License-Identifier: MIT OR Apache-2.0
//! The ledger file's format version (SPEC-STEP1 1.9, "Ledger writer: EditAnt"): a first-line header
//! record that a reader requires and that never takes part in the events or their ids.

use analysant::ledger_file::{
    Event, LEDGER_FORMAT_VERSION, LedgerFileError, UnitKindName, build, history, merge_text, parse,
    render, state,
};
use analysant_core::UnitId;

const HEADER: &str = "{\"type\":\"format\",\"name\":\"analysant-ledger\",\"version\":1}";

fn two_events() -> Vec<Event> {
    let u = UnitId::from_raw(7);
    vec![
        build::minted(u, UnitKindName::Claim, "a.md", &[], "The cache is cold."),
        build::revised(u, "a.md", 2, "The cache is warm."),
    ]
}

fn other_branch() -> Vec<Event> {
    let v = UnitId::from_raw(9);
    vec![build::minted(
        v,
        UnitKindName::Claim,
        "b.md",
        &[],
        "The queue is empty.",
    )]
}

#[test]
fn the_current_version_is_one() {
    assert_eq!(LEDGER_FORMAT_VERSION, 1);
}

#[test]
fn render_writes_the_header_first_with_exact_bytes() {
    let text = render(&two_events());
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some(HEADER));
    assert_eq!(lines.count(), 2, "then exactly the two events");
    assert!(text.ends_with('\n'));
    // An empty ledger is just the header.
    assert_eq!(render(&[]), format!("{HEADER}\n"));
}

#[test]
fn the_header_is_not_an_event_and_does_not_change_ids_or_state() {
    let ev = two_events();
    let back = parse(&render(&ev)).unwrap();
    assert_eq!(back.len(), 2);
    let ids: Vec<&str> = ev.iter().map(Event::id).collect();
    let back_ids: Vec<&str> = back.iter().map(Event::id).collect();
    assert_eq!(ids, back_ids);
    assert_eq!(state(&back).unwrap().len(), 1);
    assert_eq!(history(&back, UnitId::from_raw(7)).len(), 2);
}

#[test]
fn a_non_empty_file_without_a_header_is_refused() {
    let text = render(&two_events());
    let headless: String = text.lines().skip(1).map(|l| format!("{l}\n")).collect();
    assert_eq!(
        parse(&headless),
        Err(LedgerFileError::MissingHeader { line: 1 })
    );
    // Controls: an empty or blank file is an empty ledger, and the same text with its header parses.
    assert_eq!(parse(""), Ok(Vec::new()));
    assert_eq!(parse("\n  \n"), Ok(Vec::new()));
    assert_eq!(parse(&text).unwrap().len(), 2);
}

#[test]
fn a_header_must_be_the_first_record() {
    let text = render(&two_events());
    let mut lines: Vec<&str> = text.lines().collect();
    lines.swap(0, 1);
    let swapped = format!("{}\n", lines.join("\n"));
    assert_eq!(
        parse(&swapped),
        Err(LedgerFileError::MissingHeader { line: 1 })
    );
}

#[test]
fn an_unknown_or_newer_version_is_refused_naming_both() {
    let text = render(&two_events());
    for found in [0_u32, 2, 99] {
        let other = text.replacen("\"version\":1", &format!("\"version\":{found}"), 1);
        assert_eq!(
            parse(&other),
            Err(LedgerFileError::UnsupportedVersion {
                found,
                supported: LEDGER_FORMAT_VERSION
            }),
            "version {found}"
        );
    }
    let msg = LedgerFileError::UnsupportedVersion {
        found: 2,
        supported: 1,
    }
    .to_string();
    assert!(msg.contains('2') && msg.contains('1'), "{msg}");
}

#[test]
fn a_wrong_format_name_or_extra_field_in_the_header_is_malformed() {
    let text = render(&two_events());
    let renamed = text.replacen("analysant-ledger", "something-else", 1);
    assert!(matches!(
        parse(&renamed),
        Err(LedgerFileError::Malformed(1, _))
    ));
    let extra = text.replacen("\"version\":1", "\"version\":1,\"colour\":\"red\"", 1);
    assert!(matches!(
        parse(&extra),
        Err(LedgerFileError::Malformed(1, _))
    ));
}

#[test]
fn a_repeated_identical_header_is_kept_once_and_a_different_one_is_refused() {
    // Two files concatenated by a textual merge repeat the header; an identical one is harmless.
    let a = render(&two_events());
    let b = render(&other_branch());
    let both = format!("{a}{b}");
    let parsed = parse(&both).unwrap();
    assert_eq!(parsed.len(), 3);
    let clash = format!("{a}{}", b.replacen("\"version\":1", "\"version\":2", 1));
    assert_eq!(
        parse(&clash),
        Err(LedgerFileError::UnsupportedVersion {
            found: 2,
            supported: 1
        })
    );
}

#[test]
fn merging_files_keeps_one_header_and_is_byte_identical_in_either_order() {
    let a = render(&two_events());
    let b = render(&other_branch());
    let ab = merge_text(&a, &b).unwrap();
    let ba = merge_text(&b, &a).unwrap();
    assert_eq!(ab.text, ba.text, "byte-identical");
    assert_eq!(ab.text.matches(HEADER).count(), 1, "one header");
    assert!(ab.text.starts_with(&format!("{HEADER}\n")), "kept first");
    assert_eq!(ab.text.lines().count(), 1 + 3);
    assert!(ab.conflicts.is_empty());
    // Idempotent: merging the result with either side changes nothing.
    assert_eq!(merge_text(&ab.text, &a).unwrap().text, ab.text);
    // An empty side contributes nothing (no header needed on a file that has no records).
    assert_eq!(merge_text("", &a).unwrap().text, a);
}

#[test]
fn merging_files_of_different_versions_is_refused_not_merged() {
    let a = render(&two_events());
    let b = render(&other_branch()).replacen("\"version\":1", "\"version\":2", 1);
    assert_eq!(
        merge_text(&a, &b),
        Err(LedgerFileError::VersionMismatch { a: 1, b: 2 })
    );
    assert_eq!(
        merge_text(&b, &a),
        Err(LedgerFileError::VersionMismatch { a: 2, b: 1 })
    );
    let msg = LedgerFileError::VersionMismatch { a: 1, b: 2 }.to_string();
    assert!(msg.contains('1') && msg.contains('2'), "{msg}");
}
