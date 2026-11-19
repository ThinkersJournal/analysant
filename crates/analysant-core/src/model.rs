// SPDX-License-Identifier: MIT OR Apache-2.0
//! The claim model: units, claims, relations, provenance (SPEC-STEP1 section 1).
//!
//! Core ships the *vocabulary*; the lens libraries own the *assignment*. A claim extracted by core
//! alone has `kind: Kind::Unclassified` and `status: EpistemicStatus::Unassessed`.

use crate::pack::NormalForm;
use std::fmt;

/// Identifies a unit across revisions. Opaque: 128 bits rendered as base32. Minted once by a
/// [`crate::ledger::Ledger`], never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnitId(u128);

impl UnitId {
    pub(crate) const fn from_raw(raw: u128) -> Self {
        Self(raw)
    }

    /// The raw 128 bits. For storage and hashing; do not interpret them.
    pub const fn raw(self) -> u128 {
        self.0
    }
}

const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

impl fmt::Display for UnitId {
    /// 26 base32 characters (130 bits, the top two are zero).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = [0u8; 26];
        let mut v = self.0;
        for slot in out.iter_mut().rev() {
            *slot = BASE32[(v & 31) as usize];
            v >>= 5;
        }
        f.write_str(std::str::from_utf8(&out).map_err(|_| fmt::Error)?)
    }
}

/// Which revision of a claim's text this is. Revision 1 is the first sighting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Revision(pub u32);

/// A byte range and a line range in one source revision. Lines are 1-based and inclusive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line_start: u32,
    pub line_end: u32,
}

/// Where a unit came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRef {
    pub repo: Option<String>,
    pub path: String,
    /// Commit sha, when known.
    pub rev: Option<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Document,
    Section,
    Block,
    Claim,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub parent: Option<UnitId>,
    /// Position among siblings: a locator, not identity.
    pub ordinal: u32,
    /// For a section.
    pub heading: Option<String>,
    pub span: Span,
    pub source: SourceRef,
}

/// A stable reference to a claim: `an:<id>[@<rev>]`. Without `rev`, "whichever revision is current".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClaimRef {
    pub id: UnitId,
    pub rev: Option<Revision>,
}

impl ClaimRef {
    pub const fn current(id: UnitId) -> Self {
        Self { id, rev: None }
    }
}

impl fmt::Display for ClaimRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.rev {
            Some(r) => write!(f, "an:{}@{}", self.id, r.0),
            None => write!(f, "an:{}", self.id),
        }
    }
}

/// How a claim appears (structural; set by the format adapter and the language pack).
///
/// `Bold` and `CodeSpan` are additions to the spec's list (SPEC-STEP1 1.3): the spike's two
/// "named capability" and "code fact" shapes are spans inside a line, and neither is a sentence.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Form {
    Sentence,
    ListItem,
    TableRow,
    TableCell,
    Heading,
    Question,
    Bold,
    CodeSpan,
    Other(String),
}

/// What a claim does (speech act). The pragmatic lens assigns; the base libraries never do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Kind {
    Unclassified,
    Directive,
    Assertion,
    Question,
    Decision,
    Definition,
    Proposal,
    Report,
    Name,
    Description,
}

/// Evidence behind a status. Free text until the epistemic lens defines more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence(pub String);

/// A status, not a meaning; the epistemic lens assigns.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpistemicStatus {
    Unassessed,
    Verified { basis: Evidence },
    Asserted,
    Requested,
    Assumed,
    Disputed { by: Vec<ClaimRef> },
    Superseded { by: ClaimRef },
}

/// A format adapter or language pack, with its version, recorded on every claim it produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractorId {
    pub format: String,
    pub format_version: String,
    pub pack: String,
    pub pack_version: String,
}

/// An RFC 3339 instant supplied by the caller. Extraction is deterministic, so core never reads a clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timestamp(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub source: SourceRef,
    pub extracted_by: ExtractorId,
    pub extracted_at: Option<Timestamp>,
    /// When the document says it, if derivable. Core only carries it.
    pub asserted_at: Option<Timestamp>,
    /// Only when derivable; never guessed.
    pub author: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Claim {
    pub unit: Unit,
    pub revision: Revision,
    /// As written.
    pub text: String,
    pub form: Form,
    pub kind: Kind,
    /// From the language pack's normalizer. Never part of identity.
    pub normal: NormalForm,
    pub status: EpistemicStatus,
    pub provenance: Provenance,
}

/// Names a lens in a derivation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LensId(pub String);

/// The rule that fired, and the table entries it used. Never empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derivation {
    pub lens: LensId,
    pub rule: String,
    pub table_entries: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RelationKind {
    Supersedes,
    Contradicts,
    Implements,
    Tests,
    Missing,
    Elaborates,
    Supports,
    Attacks,
    Justifies,
    Concedes,
    Owns,
    Cites,
    Duplicates,
    Refines,
    /// A lens-private relation, namespaced `lens:name`.
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationState {
    /// By the document or an edit.
    Asserted,
    /// By a lens; affects nothing until accepted.
    Proposed,
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relation {
    pub from: ClaimRef,
    pub to: ClaimRef,
    pub kind: RelationKind,
    pub by: Derivation,
    pub state: RelationState,
}
