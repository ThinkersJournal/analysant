// SPDX-License-Identifier: MIT OR Apache-2.0
//! One content-based id scheme for every unit kind (pm-ants ruling 2026-11-20, SPEC-STEP1 1.4):
//! the same content gets the same id in any ledger, in any mint order; duplicates are told apart
//! by occurrence; the unit kind is part of the id.

use analysant_core::{Ledger, UnitKind, content_id};

fn path(p: &[&str]) -> Vec<String> {
    p.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn the_same_content_gets_the_same_id_in_two_separate_ledgers() {
    let mut a = Ledger::default();
    let mut b = Ledger::default();
    let ia = a.mint(
        UnitKind::Claim,
        "a.md",
        &path(&["Top"]),
        "The cache is cold.",
    );
    let ib = b.mint(
        UnitKind::Claim,
        "a.md",
        &path(&["Top"]),
        "The cache is cold.",
    );
    assert_eq!(ia, ib);
}

#[test]
fn mint_order_of_different_documents_does_not_change_an_id() {
    let mut first = Ledger::default();
    let x1 = first.mint(UnitKind::Claim, "x.md", &path(&["S"]), "Alpha holds.");
    let y1 = first.mint(UnitKind::Claim, "y.md", &path(&["S"]), "Beta holds.");
    let mut second = Ledger::default();
    let y2 = second.mint(UnitKind::Claim, "y.md", &path(&["S"]), "Beta holds.");
    let x2 = second.mint(UnitKind::Claim, "x.md", &path(&["S"]), "Alpha holds.");
    assert_eq!((x1, y1), (x2, y2));
}

#[test]
fn identical_sentences_in_one_section_are_told_apart_by_occurrence() {
    let mut l = Ledger::default();
    let one = l.mint(UnitKind::Claim, "a.md", &path(&["S"]), "Same words.");
    let two = l.mint(UnitKind::Claim, "a.md", &path(&["S"]), "Same words.");
    assert_ne!(one, two);
    let mut again = Ledger::default();
    assert_eq!(
        one,
        again.mint(UnitKind::Claim, "a.md", &path(&["S"]), "Same words.")
    );
    assert_eq!(two, {
        again.mint(UnitKind::Claim, "a.md", &path(&["S"]), "Same words.")
    });
}

#[test]
fn the_unit_kind_is_part_of_the_id() {
    let mut l = Ledger::default();
    let claim = l.mint(UnitKind::Claim, "a.md", &path(&["S"]), "Intro");
    let section = l.mint(UnitKind::Section, "a.md", &path(&["S"]), "Intro");
    assert_ne!(claim, section);
}

#[test]
fn content_id_is_the_function_the_ledger_uses() {
    let mut l = Ledger::default();
    let minted = l.mint(UnitKind::Claim, "a.md", &path(&["S", "T"]), "Text.");
    assert_eq!(
        minted,
        content_id(UnitKind::Claim, "a.md", &path(&["S", "T"]), "Text.", 0, 0)
    );
    assert_ne!(
        content_id(UnitKind::Claim, "a.md", &path(&["S"]), "Text.", 0, 0),
        content_id(UnitKind::Claim, "a.md", &path(&["S"]), "Text.", 0, 1),
        "a later generation of the same content is a different id"
    );
}

/// A stability vector: EditAnt depends on this exact output (SPEC-STEP1 1.9, "Ledger writer: EditAnt").
/// A change to it changes every id ever written to a ledger file, so it is a Breaking change.
#[test]
fn content_id_stability_vector() {
    let id = content_id(
        UnitKind::Claim,
        "a.md",
        &path(&["Top", "Sub"]),
        "The cache is cold.",
        0,
        0,
    );
    // Checked against an independent implementation (Python, two FNV-1a 64-bit hashes, little-endian counters).
    assert_eq!(
        format!("{id:?}"),
        "UnitId(289813204739674426619144360218466435026)"
    );
}
