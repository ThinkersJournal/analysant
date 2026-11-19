// SPDX-License-Identifier: MIT OR Apache-2.0
//! The in-memory ledger (SPEC-STEP1 1.9). Identity needs memory: every unit is minted here, once.
//! The file format, and re-identification across edits (1.4), are later slices.

use crate::finding::fnv1a;
use crate::model::{Revision, UnitId, UnitKind};

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

/// Mints unit ids and records the event. Ids are a function of the ledger's own counter and a salt, so
/// a run over the same inputs in the same order mints the same ids, and no two units in one ledger
/// share an id.
#[derive(Debug, Default)]
pub struct Ledger {
    salt: u64,
    next: u64,
    events: Vec<LedgerEvent>,
}

impl Ledger {
    /// A ledger whose ids differ from another ledger's when the salts differ.
    pub fn with_salt(salt: u64) -> Self {
        Self {
            salt,
            ..Self::default()
        }
    }

    pub fn mint(
        &mut self,
        kind: UnitKind,
        document: &str,
        section_path: &[String],
        text: &str,
    ) -> UnitId {
        self.next += 1;
        let hi = fnv1a(&[&self.salt.to_le_bytes(), &self.next.to_le_bytes()]);
        let lo = self.next;
        let id = UnitId::from_raw((u128::from(hi) << 64) | u128::from(lo));
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
