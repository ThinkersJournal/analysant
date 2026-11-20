// SPDX-License-Identifier: MIT OR Apache-2.0
//! The ledger file (SPEC-STEP1 1.9, D3, D7): `.analysant/ledger.jsonl`, one JSON record per unit
//! event, committed to the analysed project and merged by union on unique event ids.
//!
//! **Who writes it.** Only a writing tool (SPEC 1.9) calls [`write_file`]; analysis code reads the
//! file ([`read_file`], [`parse`]) and gets back events to *propose* ([`crate::identity`]). Nothing
//! here is called by a lens.
//!
//! **Why an event id is a hash of the event.** Two branches that record the same event (the same
//! sentence minted, the same edit) produce the same id, so the union dedups them; two different
//! events never share one. The id is checked on every read, so a hand-edited line is refused, not
//! trusted. **Old text is never deleted:** a `revised` event carries the new text and the earlier
//! events keep the old.
//!
//! **Order.** Events are kept in one canonical order, `(unit, revision, kind rank, event id)`, so a
//! merge written from either side is byte-identical.

use analysant_core::finding::fnv1a;
use analysant_core::{Revision, UnitId, UnitKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// Where the file lives, relative to the project root (D7).
pub const LEDGER_PATH: &str = ".analysant/ledger.jsonl";

/// Which kind of unit an event is about. Re-identification (1.4) records claims only so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitKindName {
    Document,
    Section,
    Block,
    Claim,
}

impl From<UnitKind> for UnitKindName {
    fn from(k: UnitKind) -> Self {
        match k {
            UnitKind::Document => Self::Document,
            UnitKind::Section => Self::Section,
            UnitKind::Block => Self::Block,
            UnitKind::Claim => Self::Claim,
        }
    }
}

/// One ledger record. `id` is [`Event::compute_id`] of the rest; `unit` is the unit id as `Display`
/// prints it; `text_hash` is 16 hex digits of FNV-1a over the text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    /// First sighting, revision 1.
    Minted {
        id: String,
        unit: String,
        kind: UnitKindName,
        document: String,
        section_path: Vec<String>,
        text: String,
        text_hash: String,
        revision: u32,
    },
    /// The same proposition, reworded: same unit, `revision + 1`, the new text.
    Revised {
        id: String,
        unit: String,
        document: String,
        revision: u32,
        text: String,
        text_hash: String,
    },
    /// The unit now sits under another section path; text and revision unchanged.
    Moved {
        id: String,
        unit: String,
        document: String,
        revision: u32,
        section_path: Vec<String>,
    },
    /// No longer in the document. Never dropped silently, never deleted from the ledger.
    Gone {
        id: String,
        unit: String,
        document: String,
        revision: u32,
    },
}

/// Hash of a text, as stored.
pub fn text_hash(text: &str) -> String {
    format!("{:016x}", fnv1a(&[text.as_bytes()]))
}

impl Event {
    fn rank(&self) -> u8 {
        match self {
            Self::Minted { .. } => 0,
            Self::Revised { .. } => 1,
            Self::Moved { .. } => 2,
            Self::Gone { .. } => 3,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::Minted { id, .. }
            | Self::Revised { id, .. }
            | Self::Moved { id, .. }
            | Self::Gone { id, .. } => id,
        }
    }

    pub fn unit_text(&self) -> &str {
        match self {
            Self::Minted { unit, .. }
            | Self::Revised { unit, .. }
            | Self::Moved { unit, .. }
            | Self::Gone { unit, .. } => unit,
        }
    }

    pub fn unit(&self) -> Option<UnitId> {
        UnitId::parse(self.unit_text())
    }

    pub fn revision(&self) -> u32 {
        match self {
            Self::Minted { revision, .. }
            | Self::Revised { revision, .. }
            | Self::Moved { revision, .. }
            | Self::Gone { revision, .. } => *revision,
        }
    }

    pub fn document(&self) -> &str {
        match self {
            Self::Minted { document, .. }
            | Self::Revised { document, .. }
            | Self::Moved { document, .. }
            | Self::Gone { document, .. } => document,
        }
    }

    /// The id the event must carry: 32 hex digits, two FNV-1a hashes (different prefixes) over the
    /// event's content in a fixed field order.
    pub fn compute_id(&self) -> String {
        let mut parts: Vec<Vec<u8>> = vec![
            vec![self.rank()],
            self.unit_text().as_bytes().to_vec(),
            self.revision().to_le_bytes().to_vec(),
            self.document().as_bytes().to_vec(),
        ];
        match self {
            Self::Minted {
                kind,
                section_path,
                text_hash,
                ..
            } => {
                parts.push(vec![*kind as u8]);
                parts.push(section_path.join("\u{1f}").into_bytes());
                parts.push(text_hash.as_bytes().to_vec());
            }
            Self::Revised { text_hash, .. } => parts.push(text_hash.as_bytes().to_vec()),
            Self::Moved { section_path, .. } => {
                parts.push(section_path.join("\u{1f}").into_bytes());
            }
            Self::Gone { .. } => {}
        }
        let refs: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
        let lo = fnv1a(&refs);
        let mut with_prefix: Vec<&[u8]> = vec![b"hi"];
        with_prefix.extend(refs);
        let hi = fnv1a(&with_prefix);
        format!("{hi:016x}{lo:016x}")
    }

    fn sort_key(&self) -> (u128, u32, u8, &str) {
        (
            self.unit().map_or(0, UnitId::raw),
            self.revision(),
            self.rank(),
            self.id(),
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LedgerFileError {
    /// Line number (1-based) and what serde said.
    Malformed(usize, String),
    /// The record's `id` is not the hash of its content: hand-edited or truncated.
    BadId {
        line: usize,
        id: String,
    },
    BadUnit {
        line: usize,
        unit: String,
    },
    /// The stored `text_hash` is not the hash of the stored `text`: the text was edited by hand.
    BadTextHash {
        line: usize,
        id: String,
    },
    /// A `revised`, `moved` or `gone` event for a unit no `minted` event introduces.
    OrphanEvent {
        id: String,
        unit: String,
    },
    /// Two different events under one id (a hash collision or a forged line).
    IdCollision(String),
    Io(String),
}

impl fmt::Display for LedgerFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(n, why) => write!(f, "ledger line {n}: {why}"),
            Self::BadId { line, id } => {
                write!(f, "ledger line {line}: id {id} is not the event's hash")
            }
            Self::BadUnit { line, unit } => {
                write!(f, "ledger line {line}: {unit:?} is not a unit id")
            }
            Self::BadTextHash { line, id } => {
                write!(
                    f,
                    "ledger line {line}: event {id}: text does not match its text_hash"
                )
            }
            Self::OrphanEvent { id, unit } => {
                write!(f, "event {id}: unit {unit} was never minted in this ledger")
            }
            Self::IdCollision(id) => write!(f, "two different events carry id {id}"),
            Self::Io(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for LedgerFileError {}

fn canonical(mut events: Vec<Event>) -> Vec<Event> {
    events.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    events
}

/// Read a ledger. Blank lines are skipped. A line that does not parse, whose id is not its content's
/// hash, or whose unit is not an id is an error naming the line. A repeated event (same id, same
/// content) is kept once. The result is in canonical order.
pub fn parse(text: &str) -> Result<Vec<Event>, LedgerFileError> {
    let mut by_id: BTreeMap<String, Event> = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        let ev: Event =
            serde_json::from_str(line).map_err(|e| LedgerFileError::Malformed(n, e.to_string()))?;
        if ev.unit().is_none() {
            return Err(LedgerFileError::BadUnit {
                line: n,
                unit: ev.unit_text().to_string(),
            });
        }
        if let Event::Minted {
            text, text_hash: h, ..
        }
        | Event::Revised {
            text, text_hash: h, ..
        } = &ev
        {
            if *h != text_hash(text) {
                return Err(LedgerFileError::BadTextHash {
                    line: n,
                    id: ev.id().to_string(),
                });
            }
        }
        if ev.compute_id() != ev.id() {
            return Err(LedgerFileError::BadId {
                line: n,
                id: ev.id().to_string(),
            });
        }
        match by_id.get(ev.id()) {
            Some(prev) if *prev != ev => {
                return Err(LedgerFileError::IdCollision(ev.id().to_string()));
            }
            Some(_) => {}
            None => {
                by_id.insert(ev.id().to_string(), ev);
            }
        }
    }
    Ok(canonical(by_id.into_values().collect()))
}

/// The file's text: one JSON record per line, canonical order, a final newline. Same events, same
/// bytes.
pub fn render(events: &[Event]) -> String {
    let mut out = String::new();
    for ev in canonical(events.to_vec()) {
        out.push_str(&serde_json::to_string(&ev).unwrap_or_default());
        out.push('\n');
    }
    out
}

/// Two revisions of one unit that disagree: the same unit, the same revision, different text. The
/// union keeps both (nothing is deleted); the later event id wins `state`, and a human decides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub unit: UnitId,
    pub revision: u32,
    pub event_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Merged {
    pub events: Vec<Event>,
    pub conflicts: Vec<Conflict>,
}

/// Merge by union on event id (D3). Commutative, associative and idempotent, because the result is
/// the set of events in canonical order.
pub fn merge(a: &[Event], b: &[Event]) -> Result<Merged, LedgerFileError> {
    let mut by_id: BTreeMap<String, Event> = BTreeMap::new();
    for ev in a.iter().chain(b) {
        match by_id.get(ev.id()) {
            Some(prev) if prev != ev => {
                return Err(LedgerFileError::IdCollision(ev.id().to_string()));
            }
            Some(_) => {}
            None => {
                by_id.insert(ev.id().to_string(), ev.clone());
            }
        }
    }
    let events = canonical(by_id.into_values().collect());
    let conflicts = conflicts_of(&events);
    Ok(Merged { events, conflicts })
}

fn conflicts_of(events: &[Event]) -> Vec<Conflict> {
    let mut by_key: BTreeMap<(u128, u32), Vec<&Event>> = BTreeMap::new();
    for ev in events {
        if matches!(ev, Event::Revised { .. }) {
            if let Some(u) = ev.unit() {
                by_key.entry((u.raw(), ev.revision())).or_default().push(ev);
            }
        }
    }
    by_key
        .into_iter()
        .filter(|(_, evs)| {
            let hashes: std::collections::BTreeSet<&str> = evs
                .iter()
                .filter_map(|e| match e {
                    Event::Revised { text_hash, .. } => Some(text_hash.as_str()),
                    _ => None,
                })
                .collect();
            hashes.len() > 1
        })
        .map(|((raw, revision), evs)| Conflict {
            unit: UnitId::from_raw(raw),
            revision,
            event_ids: evs.iter().map(|e| e.id().to_string()).collect(),
        })
        .collect()
}

/// A unit as the ledger last left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitState {
    pub unit: UnitId,
    pub kind: UnitKindName,
    pub document: String,
    pub section_path: Vec<String>,
    pub text: String,
    pub revision: Revision,
    pub gone: bool,
    /// Hash of (document, path, text) at minting, so a re-mint of the same content is told apart
    /// from the first (see `identity::mint_id`).
    pub mint_key: u64,
}

/// Fold the events into each unit's last state, in canonical order. A unit with no `minted` event
/// is an error, not a guess.
pub fn state(events: &[Event]) -> Result<Vec<UnitState>, LedgerFileError> {
    let mut units: BTreeMap<u128, UnitState> = BTreeMap::new();
    for ev in canonical(events.to_vec()) {
        let Some(unit) = ev.unit() else {
            return Err(LedgerFileError::BadUnit {
                line: 0,
                unit: ev.unit_text().to_string(),
            });
        };
        match &ev {
            Event::Minted {
                kind,
                document,
                section_path,
                text,
                revision,
                ..
            } => {
                units.insert(
                    unit.raw(),
                    UnitState {
                        unit,
                        kind: *kind,
                        document: document.clone(),
                        section_path: section_path.clone(),
                        text: text.clone(),
                        revision: Revision(*revision),
                        gone: false,
                        mint_key: mint_key(document, section_path, text),
                    },
                );
            }
            other => {
                let Some(s) = units.get_mut(&unit.raw()) else {
                    return Err(LedgerFileError::OrphanEvent {
                        id: other.id().to_string(),
                        unit: other.unit_text().to_string(),
                    });
                };
                match other {
                    Event::Revised { text, revision, .. } => {
                        s.text.clone_from(text);
                        s.revision = Revision(*revision);
                    }
                    Event::Moved { section_path, .. } => s.section_path.clone_from(section_path),
                    Event::Gone { .. } => s.gone = true,
                    Event::Minted { .. } => {}
                }
            }
        }
    }
    Ok(units.into_values().collect())
}

/// Every text a unit has had, oldest first: `(revision, text)`. The old text is never deleted.
pub fn history(events: &[Event], unit: UnitId) -> Vec<(Revision, String)> {
    canonical(events.to_vec())
        .into_iter()
        .filter(|e| e.unit() == Some(unit))
        .filter_map(|e| match e {
            Event::Minted { text, revision, .. } | Event::Revised { text, revision, .. } => {
                Some((Revision(revision), text))
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn mint_key(document: &str, section_path: &[String], text: &str) -> u64 {
    fnv1a(&[
        document.as_bytes(),
        section_path.join("\u{1f}").as_bytes(),
        text.as_bytes(),
    ])
}

/// Read the ledger at `path`. A missing file is an empty ledger (a project that has never been
/// analysed), not an error; any other failure is.
pub fn read_file(path: &Path) -> Result<Vec<Event>, LedgerFileError> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(LedgerFileError::Io(format!("{}: {e}", path.display()))),
    }
}

/// Write the ledger, creating `.analysant/` if needed. **Only the writing tool calls this** (1.9).
pub fn write_file(path: &Path, events: &[Event]) -> Result<(), LedgerFileError> {
    let io = |e: std::io::Error| LedgerFileError::Io(format!("{}: {e}", path.display()));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    std::fs::write(path, render(events)).map_err(io)
}

/// Build events with their ids filled in (for the writing tool and for tests).
pub mod build {
    use super::{Event, UnitKindName, text_hash};
    use analysant_core::UnitId;

    fn sealed(mut ev: Event) -> Event {
        let id = ev.compute_id();
        match &mut ev {
            Event::Minted { id: slot, .. }
            | Event::Revised { id: slot, .. }
            | Event::Moved { id: slot, .. }
            | Event::Gone { id: slot, .. } => *slot = id,
        }
        ev
    }

    pub fn minted(
        unit: UnitId,
        kind: UnitKindName,
        document: &str,
        section_path: &[String],
        text: &str,
    ) -> Event {
        sealed(Event::Minted {
            id: String::new(),
            unit: unit.to_string(),
            kind,
            document: document.to_string(),
            section_path: section_path.to_vec(),
            text: text.to_string(),
            text_hash: text_hash(text),
            revision: 1,
        })
    }

    pub fn revised(unit: UnitId, document: &str, revision: u32, text: &str) -> Event {
        sealed(Event::Revised {
            id: String::new(),
            unit: unit.to_string(),
            document: document.to_string(),
            revision,
            text: text.to_string(),
            text_hash: text_hash(text),
        })
    }

    pub fn moved(unit: UnitId, document: &str, revision: u32, section_path: &[String]) -> Event {
        sealed(Event::Moved {
            id: String::new(),
            unit: unit.to_string(),
            document: document.to_string(),
            revision,
            section_path: section_path.to_vec(),
        })
    }

    pub fn gone(unit: UnitId, document: &str, revision: u32) -> Event {
        sealed(Event::Gone {
            id: String::new(),
            unit: unit.to_string(),
            document: document.to_string(),
            revision,
        })
    }
}
