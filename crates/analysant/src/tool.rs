// SPDX-License-Identifier: MIT OR Apache-2.0
//! The tool seam as a library (SPEC-STEP1 4.1, slice 6 part 1): what a lens declares (`analysant_core::LensSpec`), a short
//! `health_report` over lens outputs, and `explain` for one finding. No MCP, no JSON, no I/O: a later tool project publishes
//! these; nothing here knows it exists.
//!
//! `health_report` never says "no discrepancies" without the `SearchRecord` and `ControlResult` of the checks that produced the
//! silence (4.2). The type enforces it: a lens with nothing to show is [`Silence::Verified`], carrying those checks, or
//! [`Silence::Unverified`], carrying the reason it cannot be called clean. There is no third, bare "clean" state.

use crate::corpus::Corpus;
use analysant_core::{
    Basis, ClaimRef, ControlOutcome, ControlResult, Derivation, Disposition, Finding, FindingId,
    LensId, LensSpec, SearchRecord,
};
use std::collections::BTreeMap;

/// One check whose result was "nothing found": what was searched and the control that shows the search could have found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NullCheck {
    pub search: SearchRecord,
    pub control: ControlResult,
}

/// Everything a lens produced in one run, in the form the report reads.
#[derive(Clone, Debug)]
pub struct LensOutput {
    pub spec: LensSpec,
    pub findings: Vec<Finding>,
    /// Units the lens examined: the denominator for its coverage.
    pub population: u64,
    /// Findings the lens withheld because a control failed or could not be run.
    pub withheld: u64,
    /// Units set aside and why (never silently dropped).
    pub set_aside: Vec<(String, u64)>,
    /// What the lens did not analyse at all (a document class, a language), in words.
    pub not_analysed: Vec<String>,
    /// The checks behind an empty result. Empty when the lens recorded none.
    pub null_checks: Vec<NullCheck>,
}

/// The knobs of a report. The pass line is CireSnave's (D1) and has no default here.
#[derive(Clone, Debug, Default)]
pub struct HealthParams {
    /// How many findings the report lists (the size budget).
    pub top_k: usize,
    /// The precision, in percent, a kind must reach to be shown. `None`: no kind is hidden for precision.
    pub pass_line_percent: Option<u8>,
    /// Measured precision per `(lens id, kind label)`, in percent, from a Gate B pass. A kind absent here is *unmeasured*.
    pub measured_precision: BTreeMap<(String, String), u8>,
    /// Severity per kind label; a kind not listed is 0. Higher is listed first; ties by finding id.
    pub severity: BTreeMap<String, u32>,
}

/// Whether a kind's findings are in the report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindStatus {
    /// Shown, and measured at or above the pass line.
    Shown { measured: u8 },
    /// Shown, but no precision has been measured for it (or no pass line is set): the reader is told.
    Unmeasured,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindCount {
    pub lens: LensId,
    pub kind: String,
    pub count: u64,
    pub status: KindStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopFinding {
    pub id: FindingId,
    pub lens: LensId,
    pub kind: String,
    pub severity: u32,
    pub statement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    pub lens: LensId,
    pub population: u64,
    /// Findings the lens produced, shown or not.
    pub findings: u64,
    pub withheld: u64,
    pub set_aside: Vec<(String, u64)>,
    pub not_analysed: Vec<String>,
}

/// Why a lens has nothing in the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Silence {
    /// Nothing found, with the checks that show the search could have found something.
    Verified {
        lens: LensId,
        checks: Vec<NullCheck>,
    },
    /// Nothing shown, and it cannot be called clean.
    Unverified { lens: LensId, reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub counts: Vec<KindCount>,
    /// At most `top_k`, by severity then id.
    pub top: Vec<TopFinding>,
    /// Ids of every shown finding, so `explain` can be asked about any of them.
    pub finding_ids: Vec<FindingId>,
    pub coverage: Vec<Coverage>,
    pub silences: Vec<Silence>,
    /// How many kinds were left out for being below the pass line. A count, not a list: they are not shown.
    pub kinds_below_pass_line: u64,
}

/// A short report over lens outputs.
pub fn health_report(outputs: &[LensOutput], params: &HealthParams) -> Report {
    let mut counts = Vec::new();
    let mut shown: Vec<(&Finding, u32)> = Vec::new();
    let mut coverage = Vec::new();
    let mut silences = Vec::new();
    let mut below = 0u64;

    for out in outputs {
        let lens = out.spec.id.clone();
        // Group by kind label, in label order, so the report is stable.
        let mut by_kind: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
        for f in &out.findings {
            by_kind.entry(f.kind.label()).or_default().push(f);
        }
        let mut shown_here = 0usize;
        for (kind, fs) in by_kind {
            let measured = params
                .measured_precision
                .get(&(lens.0.clone(), kind.to_string()))
                .copied();
            let status = match (params.pass_line_percent, measured) {
                (Some(line), Some(m)) if m < line => {
                    below += 1;
                    continue;
                }
                (_, Some(m)) => KindStatus::Shown { measured: m },
                (_, None) => KindStatus::Unmeasured,
            };
            shown_here += fs.len();
            counts.push(KindCount {
                lens: lens.clone(),
                kind: kind.to_string(),
                count: fs.len() as u64,
                status,
            });
            let sev = params.severity.get(kind).copied().unwrap_or(0);
            shown.extend(fs.into_iter().map(|f| (f, sev)));
        }
        coverage.push(Coverage {
            lens: lens.clone(),
            population: out.population,
            findings: out.findings.len() as u64,
            withheld: out.withheld,
            set_aside: out.set_aside.clone(),
            not_analysed: out.not_analysed.clone(),
        });
        if shown_here == 0 {
            silences.push(silence_of(out));
        }
    }

    shown.sort_by(|(a, sa), (b, sb)| sb.cmp(sa).then_with(|| a.id.cmp(&b.id)));
    let finding_ids = {
        let mut ids: Vec<FindingId> = shown.iter().map(|(f, _)| f.id.clone()).collect();
        ids.sort();
        ids
    };
    let top = shown
        .iter()
        .take(params.top_k)
        .map(|(f, sev)| TopFinding {
            id: f.id.clone(),
            lens: f.lens.clone(),
            kind: f.kind.label().to_string(),
            severity: *sev,
            statement: f.statement.clone(),
        })
        .collect();
    Report {
        counts,
        top,
        finding_ids,
        coverage,
        silences,
        kinds_below_pass_line: below,
    }
}

/// The silence of a lens with nothing shown: verified only if it recorded checks, every control found its subject, and the
/// lens had findings to hide for no reason (none withheld, none above zero).
fn silence_of(out: &LensOutput) -> Silence {
    let lens = out.spec.id.clone();
    let unverified = |reason: &str| Silence::Unverified {
        lens: lens.clone(),
        reason: reason.to_string(),
    };
    if !out.findings.is_empty() {
        return unverified("the lens has findings, none of which is shown");
    }
    if out.withheld > 0 {
        return unverified("findings were withheld because a control failed or could not be run");
    }
    if out.null_checks.is_empty() {
        return unverified("the lens recorded no search and no control for its empty result");
    }
    if out
        .null_checks
        .iter()
        .any(|c| c.control.outcome != ControlOutcome::Found)
    {
        return unverified(
            "a control did not find its own subject, so the search cannot be trusted",
        );
    }
    if out.null_checks.iter().any(|c| c.search.matched != 0) {
        return unverified("a recorded search matched something, so it does not show an absence");
    }
    Silence::Verified {
        lens,
        checks: out.null_checks.clone(),
    }
}

/// A claim a finding is about, with where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimDetail {
    pub claim: ClaimRef,
    /// `None` when the claim is not in the corpus given (a stale reference): said, not hidden.
    pub text: Option<String>,
    pub path: Option<String>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explanation {
    pub id: FindingId,
    pub lens: LensId,
    pub kind: String,
    pub statement: String,
    pub claims: Vec<ClaimDetail>,
    pub derivation: Vec<Derivation>,
    pub basis: Basis,
    pub search: SearchRecord,
    pub control: ControlResult,
    pub disposition: Disposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplainError {
    /// No finding with this id in the outputs given.
    NotFound(FindingId),
}

/// Expand one finding: its claims with text and locator, its derivation, its search record and control. Any finding of any
/// output can be explained, including a kind below the pass line (it can still be inspected).
pub fn explain(
    id: &FindingId,
    outputs: &[LensOutput],
    corpus: &Corpus,
) -> Result<Explanation, ExplainError> {
    let f = outputs
        .iter()
        .flat_map(|o| o.findings.iter())
        .find(|f| &f.id == id)
        .ok_or_else(|| ExplainError::NotFound(id.clone()))?;
    let claims = f
        .claims
        .iter()
        .map(|r| {
            let found = corpus.claims().find(|(_, c)| c.unit.id == r.id);
            ClaimDetail {
                claim: *r,
                text: found.map(|(_, c)| c.text.clone()),
                path: found.map(|(d, _)| d.path.clone()),
                line_start: found.map(|(_, c)| c.unit.source.span.line_start),
                line_end: found.map(|(_, c)| c.unit.source.span.line_end),
            }
        })
        .collect();
    Ok(Explanation {
        id: f.id.clone(),
        lens: f.lens.clone(),
        kind: f.kind.label().to_string(),
        statement: f.statement.clone(),
        claims,
        derivation: f.derivation.clone(),
        basis: f.basis.clone(),
        search: f.search.clone(),
        control: f.control.clone(),
        disposition: f.disposition.clone(),
    })
}
