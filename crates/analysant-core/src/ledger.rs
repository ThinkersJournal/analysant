// SPDX-License-Identifier: MIT OR Apache-2.0
//! The in-memory ledger (SPEC-STEP1 1.9). Identity needs memory: every unit is minted here, once.
//! The file format and re-identification across edits (1.4) live in the `analysant` crate and mint
//! through the same function, `content_id`.

use crate::finding::fnv1a;
use crate::model::{Revision, UnitId, UnitKind};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerEvent {
    pub unit: UnitId,
    pub kind: UnitKind,
    pub document: String,
    pub section_path: Vec<String>,
    /// Hash of the unit's text at minting.
    pub text_hash: u64,
    pub revision: Revision,
}

/// A one-byte tag per kind (numbers, so no word is spelled in engine source).
fn kind_tag(kind: UnitKind) -> &'static [u8] {
    match kind {
        UnitKind::Document => &[1],
        UnitKind::Section => &[2],
        UnitKind::Block => &[3],
        UnitKind::Claim => &[4],
    }
}

/// The one id scheme for every unit kind: a function of the unit kind, document, section path and
/// text, of how many units with that same content the ledger had already minted (`occurrence`, which
/// tells identical sentences in one section apart) and of `generation` (how many earlier units of that
/// content were retired, so an id is never reused). It depends on nothing else: not on the order in
/// which documents are read, not on a counter, not on a salt. Two ledgers, two runs, two branches that
/// see the same content mint the same id, which is what makes the ledger file union-mergeable.
pub fn content_id(
    kind: UnitKind,
    document: &str,
    section_path: &[String],
    text: &str,
    occurrence: u64,
    generation: u64,
) -> UnitId {
    let path_text = section_path.join("\u{1f}");
    let occurrence = occurrence.to_le_bytes();
    let generation = generation.to_le_bytes();
    let parts: [&[u8]; 6] = [
        kind_tag(kind),
        document.as_bytes(),
        path_text.as_bytes(),
        text.as_bytes(),
        &occurrence,
        &generation,
    ];
    let lo = fnv1a(&parts);
    let hi_parts: [&[u8]; 7] = [
        b"hi", parts[0], parts[1], parts[2], parts[3], parts[4], parts[5],
    ];
    let hi = fnv1a(&hi_parts);
    UnitId::from_raw((u128::from(hi) << 64) | u128::from(lo))
}

/// Mints unit ids by `content_id` and records the event.
#[derive(Debug, Default)]
pub struct Ledger {
    seen: BTreeMap<u64, u64>,
    events: Vec<LedgerEvent>,
}

impl Ledger {
    pub fn mint(
        &mut self,
        kind: UnitKind,
        document: &str,
        section_path: &[String],
        text: &str,
    ) -> UnitId {
        let path_text = section_path.join("\u{1f}");
        let key = fnv1a(&[
            kind_tag(kind),
            document.as_bytes(),
            path_text.as_bytes(),
            text.as_bytes(),
        ]);
        let occurrence = self.seen.entry(key).or_default();
        let id = content_id(kind, document, section_path, text, *occurrence, 0);
        *occurrence += 1;
        self.events.push(LedgerEvent {
            unit: id,
            kind,
            document: document.to_string(),
            section_path: section_path.to_vec(),
            text_hash: fnv1a(&[text.as_bytes()]),
            revision: Revision(1),
        });
        id
    }

    pub fn events(&self) -> &[LedgerEvent] {
        &self.events
    }
}
