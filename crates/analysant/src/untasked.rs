// SPDX-License-Identifier: MIT OR Apache-2.0
//! The directed-but-untasked check: a claim named in the documents that no plan line mentions.
//! "Mentions" is every word of the claim's key present in the line, after alias canonicalization
//! (the reference spike's lens, credited, `MIT OR Apache-2.0`). It does not judge whether a mention
//! is a task or a passing reference: that triage is a human's, and the finding says so.
//!
//! Every absence carries its search record and a SelfFind control (SPEC-STEP1 4.2). If the control
//! fails, the matcher is broken on this input and every absence is withheld, not reported as clean.

use crate::corpus::Corpus;
use analysant_core::{
    Basis, ClaimRef, ControlKind, ControlOutcome, ControlResult, Derivation, Disposition, Finding,
    FindingId, FindingKind, LanguagePack, LensId, SearchRecord, normalize,
};
use analysant_lexical::{AliasTable, MatchMode, TermIndex};
use std::collections::HashMap;

pub const LENS: &str = "untasked";

#[derive(Clone, Debug)]
pub enum Status {
    /// The claim has no key, so there is nothing to search for. Counted, never a finding.
    NoKey,
    /// At least one plan line mentions it.
    Mentioned {
        hits: usize,
    },
    Absent(Box<Finding>),
    /// Would have been a finding; withheld because its control failed or could not be run.
    Withheld {
        reason: String,
    },
}

#[derive(Clone, Debug)]
pub struct Outcome {
    /// Position in `Corpus::claims()` order.
    pub index: usize,
    pub status: Status,
}

#[derive(Clone, Debug)]
pub struct UntaskedRun {
    pub plan_lines: usize,
    pub outcomes: Vec<Outcome>,
}

impl UntaskedRun {
    pub fn absent(&self) -> impl Iterator<Item = (usize, &Finding)> {
        self.outcomes.iter().filter_map(|o| match &o.status {
            Status::Absent(f) => Some((o.index, f.as_ref())),
            _ => None,
        })
    }

    pub fn mentioned(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o.status, Status::Mentioned { .. }))
            .count()
    }

    pub fn withheld(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o.status, Status::Withheld { .. }))
            .count()
    }
}

/// Check every keyed claim against `plan_lines`.
pub fn directed_but_untasked(
    pack: &dyn LanguagePack,
    corpus: &Corpus,
    plan_lines: &[String],
    aliases: &AliasTable,
    mode: MatchMode,
) -> UntaskedRun {
    directed_but_untasked_with(pack, corpus, plan_lines, aliases, mode, false, &[])
}

/// As `directed_but_untasked`; with `skip_names` a Name-kind claim is set aside like a claim with no key (it states no work), and a
/// non-empty `only_kinds` sets aside every claim whose kind is not listed (descriptions and records of finished work state no task).
pub fn directed_but_untasked_with(
    pack: &dyn LanguagePack,
    corpus: &Corpus,
    plan_lines: &[String],
    aliases: &AliasTable,
    mode: MatchMode,
    skip_names: bool,
    only_kinds: &[analysant_core::Kind],
) -> UntaskedRun {
    let set_aside = |c: &analysant_core::Claim| {
        c.normal.is_empty()
            || (skip_names && c.kind == analysant_core::Kind::Name)
            || (!only_kinds.is_empty() && !only_kinds.contains(&c.kind))
    };
    let claims: Vec<_> = corpus.claims().collect();
    let index = TermIndex::build(
        &format!("{} plan lines", plan_lines.len()),
        plan_lines
            .iter()
            .enumerate()
            .map(|(i, l)| (i, aliases.canonicalize(pack, &normalize(pack, l)).form)),
    );

    // Control candidates: claims that have a key. One control per distinct control item, cached.
    let eligible: Vec<usize> = claims
        .iter()
        .enumerate()
        .filter(|(_, (_, c))| !set_aside(c))
        .map(|(i, _)| i)
        .collect();
    let mut control_cache: HashMap<usize, ControlOutcome> = HashMap::new();
    let mut self_find = |item: usize| -> ControlOutcome {
        *control_cache.entry(item).or_insert_with(|| {
            let (_, c) = claims[item];
            // The claim's own text, normalized afresh, against its own key: same matcher, same mode.
            let own = TermIndex::build(
                "the control claim's own text",
                [(
                    0usize,
                    aliases.canonicalize(pack, &normalize(pack, &c.text)).form,
                )],
            );
            let key = aliases.canonicalize(pack, &c.normal).form;
            if own.query(&key, mode).hits.len() == 1 {
                ControlOutcome::Found
            } else {
                ControlOutcome::NotFound
            }
        })
    };

    let mut outcomes = Vec::with_capacity(claims.len());
    for (i, (doc, claim)) in claims.iter().enumerate() {
        if set_aside(claim) {
            outcomes.push(Outcome {
                index: i,
                status: Status::NoKey,
            });
            continue;
        }
        let canon = aliases.canonicalize(pack, &claim.normal);
        let result = index.query(&canon.form, mode);
        if !result.hits.is_empty() {
            outcomes.push(Outcome {
                index: i,
                status: Status::Mentioned {
                    hits: result.hits.len(),
                },
            });
            continue;
        }

        // An absence. It needs a control that is not itself.
        let Some(&control_idx) = eligible.iter().find(|&&e| e != i) else {
            outcomes.push(Outcome {
                index: i,
                status: Status::Withheld {
                    reason: "no control claim other than the subject exists".into(),
                },
            });
            continue;
        };
        let control = ControlResult {
            kind: ControlKind::SelfFind,
            item: ClaimRef::current(claims[control_idx].1.unit.id),
            outcome: self_find(control_idx),
        };
        if control.outcome == ControlOutcome::NotFound {
            outcomes.push(Outcome {
                index: i,
                status: Status::Withheld {
                    reason:
                        "SelfFind control failed: the matcher cannot find a claim in its own text"
                            .into(),
                },
            });
            continue;
        }

        let lens = LensId(LENS.to_string());
        let claim_ref = ClaimRef::current(claim.unit.id);
        let mut search: SearchRecord = result.search;
        search.query.aliases_applied = canon.applied.clone();
        let line = claim.unit.source.span.line_start;
        outcomes.push(Outcome {
            index: i,
            status: Status::Absent(Box::new(Finding {
                id: FindingId::compute(&lens, "untasked", &[claim_ref], &canon.form.key),
                lens: lens.clone(),
                kind: FindingKind::Untasked,
                claims: vec![claim_ref],
                statement: format!(
                    "\"{}\" ({}:{}) is named in the documents, and none of {} plan lines contains all of: {}",
                    claim.text,
                    doc.path,
                    line,
                    plan_lines.len(),
                    canon.form.key
                ),
                derivation: vec![Derivation {
                    lens,
                    rule: "every key token present in a plan line, after alias canonicalization"
                        .into(),
                    table_entries: canon.applied,
                }],
                basis: Basis::Deterministic,
                search,
                control,
                disposition: Disposition::Open,
            })),
        });
    }
    UntaskedRun {
        plan_lines: plan_lines.len(),
        outcomes,
    }
}

/// What the untasked lens declares about itself (SPEC-STEP1 4.1).
pub fn spec() -> analysant_core::LensSpec {
    analysant_core::LensSpec {
        id: LensId(LENS.to_string()),
        description: "A claim named in the documents that no plan line mentions; whether the mention is a task is a human's call."
            .into(),
        requires: Vec::new(),
        cost: analysant_core::Cost::Cheap,
        reads_code: false,
        kinds: vec!["untasked".into()],
    }
}

impl UntaskedRun {
    /// This run in the shape `health_report` reads. `NoKey` claims are set aside and counted. The run keeps no search record for
    /// a claim that was mentioned, so an empty result carries no null checks and the report will not call it clean.
    pub fn output(&self) -> crate::tool::LensOutput {
        let no_key = self
            .outcomes
            .iter()
            .filter(|o| matches!(o.status, Status::NoKey))
            .count() as u64;
        crate::tool::LensOutput {
            spec: spec(),
            findings: self.absent().map(|(_, f)| f.clone()).collect(),
            population: self.outcomes.len() as u64,
            withheld: self.withheld() as u64,
            set_aside: if no_key > 0 {
                vec![("claim has no key".into(), no_key)]
            } else {
                Vec::new()
            },
            not_analysed: Vec::new(),
            null_checks: Vec::new(),
        }
    }
}
