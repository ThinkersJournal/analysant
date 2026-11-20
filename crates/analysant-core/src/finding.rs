// SPDX-License-Identifier: MIT OR Apache-2.0
//! Findings carry what was searched and a control (SPEC-STEP1 4.2): the portfolio's rule "state the
//! control beside the null", made a type so it cannot be omitted.

use crate::model::{ClaimRef, Derivation, LensId};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FindingId(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FindingKind {
    /// Named in the documents, in no plan row.
    Untasked,
    Other(String),
}

impl FindingKind {
    /// The kind as the string used in reports and in `FindingId::compute`.
    pub fn label(&self) -> &str {
        match self {
            Self::Untasked => "untasked",
            Self::Other(s) => s,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Basis {
    Deterministic,
    ModelJudged { model: String, input_hash: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disposition {
    Open,
    Dismissed { reason: String },
    Accepted,
    Fixed { reference: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeDescription(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryDescription {
    /// The normalized key, after aliases.
    pub key: String,
    /// Which match mode.
    pub mode: String,
    /// The alias groups that changed the query or the haystack.
    pub aliases_applied: Vec<String>,
}

/// What was looked at, so the null is a number and not a feeling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRecord {
    pub looked_in: ScopeDescription,
    /// How many units were in scope: the denominator.
    pub population: u64,
    pub query: QueryDescription,
    /// The null is `matched == 0`, stated as a number.
    pub matched: u64,
    /// What was set aside, and how many, so exclusions are visible.
    pub excluded: Vec<(String, u64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    /// A sample claim from the same population searched for by its own key in its own text.
    SelfFind,
    /// An item the project declares present.
    KnownPresent,
    DeclaredPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlOutcome {
    Found,
    /// The finding is INVALID and is withheld.
    NotFound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlResult {
    pub kind: ControlKind,
    /// The control's own subject. Never the null's subject.
    pub item: ClaimRef,
    pub outcome: ControlOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Stable: a hash of (lens, kind, claim ids, key); survives re-runs.
    pub id: FindingId,
    pub lens: LensId,
    pub kind: FindingKind,
    pub claims: Vec<ClaimRef>,
    /// One sentence a reader can check.
    pub statement: String,
    /// Rules fired and table entries used, in order.
    pub derivation: Vec<Derivation>,
    pub basis: Basis,
    /// Required for any absence claim.
    pub search: SearchRecord,
    /// Required whenever `search` is.
    pub control: ControlResult,
    pub disposition: Disposition,
}

/// FNV-1a, 64 bits: a stable hash that needs no dependency. Not cryptographic.
pub fn fnv1a(parts: &[&[u8]]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for b in *part {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        // A separator no part contains, so ("ab","c") and ("a","bc") differ.
        h ^= 0xff;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

impl FindingId {
    /// The id of a finding: a hash of lens, kind, the sorted claim refs and the key.
    pub fn compute(lens: &LensId, kind: &str, claims: &[ClaimRef], key: &str) -> Self {
        let mut refs: Vec<String> = claims.iter().map(ToString::to_string).collect();
        refs.sort();
        let mut parts: Vec<&[u8]> = vec![lens.0.as_bytes(), kind.as_bytes()];
        parts.extend(refs.iter().map(|r| r.as_bytes()));
        parts.push(key.as_bytes());
        Self(format!("f-{:016x}", fnv1a(&parts)))
    }
}
