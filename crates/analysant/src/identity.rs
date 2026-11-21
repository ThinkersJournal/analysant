// SPDX-License-Identifier: MIT OR Apache-2.0
//! Identity that survives edits (SPEC-STEP1 1.4, step 2): align a fresh extraction of one document
//! to the ledger's last state for it.
//!
//! The order, each step deterministic and each result says which rule fired:
//! 1. **Exact.** The same normalized text. Under the same section path it is the same claim; under
//!    another it is the same claim *moved* (a `moved` event). Where the old side and the new side hold
//!    different numbers of identical sentences in one section (a sentence duplicated, or one of two
//!    deleted) nothing can say which is which: that is **ambiguous**, not guessed.
//! 2. **Edited in place.** Same section path, token-set similarity at or above the caller's
//!    threshold, and the best match for each other (a tie is ambiguous): the same id, `revision + 1`.
//! 3. **Otherwise** a new unit is minted for a new sentence, and an old one with no match is marked
//!    `gone`, never silently dropped.
//!
//! An ambiguity assigns nothing: the old units keep their ids and no event is proposed, until a human
//! or an edit operation decides (1.4). The result is a *proposal* of events; this module writes
//! nothing (SPEC 1.9: only the writing tool owns the file).

use crate::ledger_file::{Event, UnitKindName, UnitState, build, mint_key};
use analysant_core::{
    Basis, ClaimRef, ControlKind, ControlOutcome, ControlResult, Derivation, Disposition, Finding,
    FindingId, FindingKind, LanguagePack, LensId, QueryDescription, ScopeDescription, SearchRecord,
    UnitId, UnitKind, content_id, normalize,
};
use std::collections::{BTreeMap, BTreeSet};

/// The lens id identity findings carry.
pub const LENS: &str = "identity";
/// The finding kind for an alignment nothing can decide.
pub const AMBIGUOUS_KIND: &str = "identity:ambiguous";

/// One claim as a fresh extraction found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub section_path: Vec<String>,
    pub text: String,
    /// The normal form's key: what "the same text" compares.
    pub key: String,
    /// The normal form's token set: what "similar" compares.
    pub tokens: BTreeSet<String>,
}

impl Observation {
    pub fn new(pack: &dyn LanguagePack, section_path: Vec<String>, text: &str) -> Self {
        let nf = normalize(pack, text);
        Self {
            section_path,
            text: text.to_string(),
            key: nf.key.clone(),
            tokens: nf.tokens.iter().map(|t| t.norm.clone()).collect(),
        }
    }
}

/// The edit threshold, as a share of 1000 so no float comparison decides identity. The pack
/// configures it (1.4); this module has no default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityParams {
    pub edit_similarity_permille: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum How {
    Exact,
    Moved,
    Edited { similarity_permille: u32 },
    Minted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    /// Index into the observations passed in.
    pub observation: usize,
    pub unit: UnitId,
    pub how: How,
}

/// A tie the rules do not break. The old units keep their ids; the observations get none yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ambiguity {
    pub document: String,
    pub old: Vec<UnitId>,
    /// Indexes into the observations passed in.
    pub new: Vec<usize>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reidentified {
    /// One per decided observation, in observation order.
    pub assignments: Vec<Assignment>,
    /// The events a writing tool would append: minted, revised, moved, gone.
    pub events: Vec<Event>,
    pub ambiguities: Vec<Ambiguity>,
    pub gone: Vec<UnitId>,
}

struct Old {
    unit: UnitId,
    path: Vec<String>,
    revision: u32,
    key: String,
    tokens: BTreeSet<String>,
}

fn similarity_permille(a: &BTreeSet<String>, b: &BTreeSet<String>) -> u32 {
    let union = a.union(b).count();
    if union == 0 {
        return 0;
    }
    (a.intersection(b).count() * 1000 / union) as u32
}

/// A minted unit's id: a function of the content and of how many units with that content the ledger
/// has already seen, so a sentence deleted and written again gets a new id (an id is never reused),
/// and two branches that mint the same new sentence mint the same id (the union dedups it).
fn mint_id(
    document: &str,
    path: &[String],
    text: &str,
    occurrence: u64,
    generation: u64,
) -> UnitId {
    content_id(
        UnitKind::Claim,
        document,
        path,
        text,
        occurrence,
        generation,
    )
}

/// Align `observed` (one document, reading order) to the live claims of `document` in `prior`.
pub fn reidentify(
    pack: &dyn LanguagePack,
    document: &str,
    prior: &[UnitState],
    observed: &[Observation],
    params: IdentityParams,
) -> Reidentified {
    let olds: Vec<Old> = prior
        .iter()
        .filter(|s| s.document == document && !s.gone && s.kind == UnitKindName::Claim)
        .map(|s| {
            let nf = normalize(pack, &s.text);
            Old {
                unit: s.unit,
                path: s.section_path.clone(),
                revision: s.revision.0,
                key: nf.key.clone(),
                tokens: nf.tokens.iter().map(|t| t.norm.clone()).collect(),
            }
        })
        .collect();

    let mut old_taken = vec![false; olds.len()];
    let mut new_taken = vec![false; observed.len()];
    let mut decided: BTreeMap<usize, (usize, How)> = BTreeMap::new();
    let mut ambiguities: Vec<Ambiguity> = Vec::new();

    // 1. Exact.
    let keys: BTreeSet<&str> = olds.iter().map(|o| o.key.as_str()).collect();
    for key in keys {
        let o_idx: Vec<usize> = (0..olds.len()).filter(|&i| olds[i].key == key).collect();
        let n_idx: Vec<usize> = (0..observed.len())
            .filter(|&j| observed[j].key == key)
            .collect();
        if n_idx.is_empty() {
            continue;
        }
        let mut by_path: BTreeMap<&Vec<String>, (Vec<usize>, Vec<usize>)> = BTreeMap::new();
        for &i in &o_idx {
            by_path.entry(&olds[i].path).or_default().0.push(i);
        }
        for &j in &n_idx {
            by_path
                .entry(&observed[j].section_path)
                .or_default()
                .1
                .push(j);
        }
        let (mut pool_o, mut pool_n) = (Vec::new(), Vec::new());
        for (o, n) in by_path.values() {
            if !o.is_empty() && o.len() == n.len() {
                for (&i, &j) in o.iter().zip(n) {
                    old_taken[i] = true;
                    new_taken[j] = true;
                    decided.insert(j, (i, How::Exact));
                }
            } else {
                pool_o.extend(o);
                pool_n.extend(n);
            }
        }
        if pool_o.len() == 1 && pool_n.len() == 1 {
            let (i, j) = (pool_o[0], pool_n[0]);
            old_taken[i] = true;
            new_taken[j] = true;
            decided.insert(j, (i, How::Moved));
        } else if !pool_o.is_empty() && !pool_n.is_empty() {
            for &i in &pool_o {
                old_taken[i] = true;
            }
            for &j in &pool_n {
                new_taken[j] = true;
            }
            ambiguities.push(Ambiguity {
                document: document.to_string(),
                old: pool_o.iter().map(|&i| olds[i].unit).collect(),
                new: pool_n.clone(),
                reason: format!(
                    "{} old and {} new claims have the same text; nothing says which is which",
                    pool_o.len(),
                    pool_n.len()
                ),
            });
        }
    }

    // 2. Edited in place: same section path, similar, and each the other's best.
    let mut edges: Vec<(u32, usize, usize)> = Vec::new();
    for (i, o) in olds.iter().enumerate().filter(|(i, _)| !old_taken[*i]) {
        for (j, n) in observed.iter().enumerate().filter(|(j, _)| !new_taken[*j]) {
            if o.path == n.section_path {
                let s = similarity_permille(&o.tokens, &n.tokens);
                if s >= params.edit_similarity_permille {
                    edges.push((s, i, j));
                }
            }
        }
    }
    edges.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    while let Some(&(s, i, j)) = edges.first() {
        let tied: Vec<(u32, usize, usize)> = edges
            .iter()
            .copied()
            .filter(|&(s2, i2, j2)| s2 == s && (i2 == i || j2 == j) && (i2, j2) != (i, j))
            .collect();
        if tied.is_empty() {
            old_taken[i] = true;
            new_taken[j] = true;
            decided.insert(
                j,
                (
                    i,
                    How::Edited {
                        similarity_permille: s,
                    },
                ),
            );
            edges.retain(|&(_, i2, j2)| i2 != i && j2 != j);
        } else {
            let mut o: BTreeSet<usize> = BTreeSet::from([i]);
            let mut n: BTreeSet<usize> = BTreeSet::from([j]);
            for &(_, i2, j2) in &tied {
                o.insert(i2);
                n.insert(j2);
            }
            for &x in &o {
                old_taken[x] = true;
            }
            for &x in &n {
                new_taken[x] = true;
            }
            ambiguities.push(Ambiguity {
                document: document.to_string(),
                old: o.iter().map(|&x| olds[x].unit).collect(),
                new: n.iter().copied().collect(),
                reason: format!(
                    "{} old and {} new claims are equally similar ({}/1000); nothing says which was edited",
                    o.len(),
                    n.len(),
                    s
                ),
            });
            edges.retain(|&(_, i2, j2)| !o.contains(&i2) && !n.contains(&j2));
        }
    }

    // 3. Mint what is new, mark gone what is unmatched; ambiguous ones stay as they are.
    let ambiguous_old: BTreeSet<UnitId> = ambiguities
        .iter()
        .flat_map(|a| a.old.iter().copied())
        .collect();
    let ambiguous_new: BTreeSet<usize> = ambiguities
        .iter()
        .flat_map(|a| a.new.iter().copied())
        .collect();

    let mut generation: BTreeMap<u64, u64> = BTreeMap::new();
    for s in prior.iter().filter(|s| s.document == document) {
        *generation.entry(s.mint_key).or_default() += 1;
    }
    let mut occurrence: BTreeMap<u64, u64> = BTreeMap::new();

    let mut events = Vec::new();
    let mut assignments = Vec::new();
    for (j, obs) in observed.iter().enumerate() {
        if let Some((i, how)) = decided.get(&j) {
            let o = &olds[*i];
            match how {
                How::Exact => {}
                How::Moved => {
                    events.push(build::moved(
                        o.unit,
                        document,
                        o.revision,
                        &obs.section_path,
                    ));
                }
                How::Edited { .. } => {
                    events.push(build::revised(o.unit, document, o.revision + 1, &obs.text));
                }
                How::Minted => {}
            }
            assignments.push(Assignment {
                observation: j,
                unit: o.unit,
                how: how.clone(),
            });
        } else if !ambiguous_new.contains(&j) {
            let key = mint_key(document, &obs.section_path, &obs.text);
            let occ = occurrence.entry(key).or_default();
            let gen_seen = generation.get(&key).copied().unwrap_or(0);
            let unit = mint_id(document, &obs.section_path, &obs.text, *occ, gen_seen);
            *occ += 1;
            events.push(build::minted(
                unit,
                UnitKindName::Claim,
                document,
                &obs.section_path,
                &obs.text,
            ));
            assignments.push(Assignment {
                observation: j,
                unit,
                how: How::Minted,
            });
        }
    }
    let mut gone = Vec::new();
    for (i, o) in olds.iter().enumerate() {
        if !old_taken[i] && !ambiguous_old.contains(&o.unit) {
            events.push(build::gone(o.unit, document, o.revision));
            gone.push(o.unit);
        }
    }
    Reidentified {
        assignments,
        events,
        ambiguities,
        gone,
    }
}

/// An ambiguity as a finding (`identity:ambiguous`). The shape requires a search record and a
/// control (SPEC 4.2); here the search is the alignment: `population` is the old live claims
/// considered, `matched` the old candidates, and the control is the first candidate found by its own
/// text, which is what the alignment's exact step does.
pub fn ambiguity_findings(
    pack: &dyn LanguagePack,
    result: &Reidentified,
    prior: &[UnitState],
    observed: &[Observation],
) -> Vec<Finding> {
    let lens = LensId(LENS.to_string());
    let mut out = Vec::new();
    for a in &result.ambiguities {
        let Some(first) = a.old.first().copied() else {
            continue;
        };
        let population = prior
            .iter()
            .filter(|s| s.document == a.document && !s.gone)
            .count() as u64;
        let refs: Vec<ClaimRef> = a.old.iter().map(|u| ClaimRef::current(*u)).collect();
        let sample = a
            .new
            .first()
            .and_then(|&j| observed.get(j))
            .map(|o| normalize(pack, &o.text).key)
            .unwrap_or_default();
        let locators: Vec<String> = a
            .new
            .iter()
            .filter_map(|&j| observed.get(j))
            .map(|o| format!("\"{}\" under {:?}", o.text, o.section_path))
            .collect();
        out.push(Finding {
            id: FindingId::compute(&lens, AMBIGUOUS_KIND, &refs, &sample),
            lens: lens.clone(),
            kind: FindingKind::Other(AMBIGUOUS_KIND.to_string()),
            claims: refs,
            statement: format!(
                "{}: {}. Candidates now: {}. The old ids are unchanged until a person or an edit decides.",
                a.document,
                a.reason,
                locators.join(" / ")
            ),
            derivation: vec![Derivation {
                lens: lens.clone(),
                rule: "ties are not resolved by guessing (SPEC 1.4)".into(),
                table_entries: Vec::new(),
            }],
            basis: Basis::Deterministic,
            search: SearchRecord {
                looked_in: ScopeDescription(format!("the ledger's live claims of {}", a.document)),
                population,
                query: QueryDescription {
                    key: sample,
                    mode: "identity alignment".into(),
                    aliases_applied: Vec::new(),
                },
                matched: a.old.len() as u64,
                excluded: Vec::new(),
            },
            control: ControlResult {
                kind: ControlKind::SelfFind,
                item: ClaimRef::current(first),
                outcome: ControlOutcome::Found,
            },
            disposition: Disposition::Open,
        });
    }
    out
}

/// The claims of an extracted document as observations, in reading order.
pub fn observations(pack: &dyn LanguagePack, doc: &crate::Document) -> Vec<Observation> {
    doc.claims
        .iter()
        .map(|c| {
            let path = doc.section_path(c).into_iter().map(String::from).collect();
            Observation::new(pack, path, &c.text)
        })
        .collect()
}
