// SPDX-License-Identifier: MIT OR Apache-2.0
//! Slice 6 part 1 (SPEC-STEP1 4.1, 4.2): `LensSpec`, `health_report` and `explain`, asserted on exact values. The real lens output is
//! the untasked run of the slice 1 gate (the narrow corpus, 558 claims, 212 absent); the synthetic cases vary one thing at a time
//! on copies of those real findings. The Gate B pass line is CireSnave's (D1): every test passes it as a parameter.

use analysant::{
    Corpus, ExplainError, HealthParams, KindStatus, LensOutput, NullCheck, Silence, contradiction,
    directed_but_untasked, explain, extract_corpus, health_report, untasked,
};
use analysant_core::{
    ControlOutcome, Cost, English, FindingId, FindingKind, Ledger, LensId, LensSpec, Markdown,
};
use analysant_lexical::{AliasTable, MatchMode};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn reference() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/pip-spike-2026-11-16")
}

fn read(p: PathBuf) -> String {
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

/// The slice 1 gate's run: the corpus and the untasked output over it.
fn real() -> (Corpus, LensOutput, usize) {
    let root = reference();
    let dir = root.join("corpus/fuel/arch-run");
    let files: Vec<(String, String)> = ["02-layers.md", "03-ir.md", "06-runtime.md"]
        .iter()
        .map(|n| ((*n).to_string(), read(dir.join(n))))
        .collect();
    let plan: Vec<String> = read(root.join("corpus/fuel/ROADMAP.md"))
        .lines()
        .map(str::to_string)
        .collect();
    let en = English::new();
    let mut ledger = Ledger::default();
    let corpus = extract_corpus(&Markdown, &en, &mut ledger, &files);
    let aliases = AliasTable::parse(&en, &read(root.join("aliases.txt")));
    let run = directed_but_untasked(&en, &corpus, &plan, &aliases, MatchMode::TokenSubset);
    let no_key = run
        .outcomes
        .iter()
        .filter(|o| matches!(o.status, analysant::Status::NoKey))
        .count();
    (corpus, run.output(), no_key)
}

fn params(top_k: usize) -> HealthParams {
    HealthParams {
        top_k,
        ..HealthParams::default()
    }
}

fn precision(pairs: &[(&str, &str, u8)]) -> BTreeMap<(String, String), u8> {
    pairs
        .iter()
        .map(|(l, k, p)| (((*l).to_string(), (*k).to_string()), *p))
        .collect()
}

#[test]
fn the_specs_are_exact_plain_data() {
    assert_eq!(
        untasked::spec(),
        LensSpec {
            id: LensId("untasked".into()),
            description: "A claim named in the documents that no plan line mentions; whether the mention is a task is a human's call.".into(),
            requires: vec![],
            cost: Cost::Cheap,
            reads_code: false,
            kinds: vec!["untasked".into()],
        }
    );
    let c = contradiction::spec();
    assert_eq!(c.id, LensId("contradiction".into()));
    assert_eq!(c.cost, Cost::Batch);
    assert_eq!(c.kinds, vec!["contradiction".to_string()]);
    assert!(!c.reads_code && c.requires.is_empty());
}

#[test]
fn health_report_over_the_real_untasked_run() {
    let (_, out, no_key) = real();
    assert_eq!(out.findings.len(), 212, "slice 1's gate: 212 absent");
    let r = health_report(std::slice::from_ref(&out), &params(5));

    assert_eq!(r.counts.len(), 1);
    assert_eq!(r.counts[0].lens, LensId("untasked".into()));
    assert_eq!(r.counts[0].kind, "untasked");
    assert_eq!(r.counts[0].count, 212);
    assert_eq!(
        r.counts[0].status,
        KindStatus::Unmeasured,
        "no Gate B yet: said so"
    );

    assert_eq!(r.top.len(), 5, "the size budget");
    let mut by_id: Vec<&FindingId> = out.findings.iter().map(|f| &f.id).collect();
    by_id.sort();
    let top_ids: Vec<&FindingId> = r.top.iter().map(|t| &t.id).collect();
    assert_eq!(
        top_ids,
        by_id[..5].to_vec(),
        "equal severity: ordered by finding id"
    );
    assert!(
        r.top
            .iter()
            .all(|t| t.severity == 0 && t.kind == "untasked")
    );

    assert_eq!(r.finding_ids.len(), 212);
    assert!(
        r.finding_ids.windows(2).all(|w| w[0] < w[1]),
        "sorted, unique"
    );

    assert_eq!(r.coverage.len(), 1);
    assert_eq!(r.coverage[0].population, 558);
    assert_eq!(r.coverage[0].findings, 212);
    assert_eq!(r.coverage[0].withheld, 0);
    assert_eq!(
        r.coverage[0].set_aside,
        if no_key > 0 {
            vec![("claim has no key".to_string(), no_key as u64)]
        } else {
            vec![]
        }
    );
    assert!(r.silences.is_empty(), "it has findings to show");
    assert_eq!(r.kinds_below_pass_line, 0);
}

#[test]
fn a_kind_below_the_pass_line_is_not_shown_but_can_be_explained() {
    let (corpus, out, _) = real();
    let p = HealthParams {
        top_k: 5,
        pass_line_percent: Some(80),
        measured_precision: precision(&[("untasked", "untasked", 60)]),
        ..HealthParams::default()
    };
    let r = health_report(std::slice::from_ref(&out), &p);
    assert!(r.counts.is_empty() && r.top.is_empty() && r.finding_ids.is_empty());
    assert_eq!(r.kinds_below_pass_line, 1);
    // The lens is not clean: it has findings that are not shown, and the report says that, not "no discrepancies".
    assert_eq!(
        r.silences,
        vec![Silence::Unverified {
            lens: LensId("untasked".into()),
            reason: "the lens has findings, none of which is shown".into()
        }]
    );
    // Hidden from the report, still inspectable.
    let id = out.findings[0].id.clone();
    assert!(explain(&id, &[out], &corpus).is_ok());
}

#[test]
fn a_kind_at_or_above_the_line_is_shown_with_its_measured_figure() {
    let (_, out, _) = real();
    for (measured, line) in [(90u8, 80u8), (80, 80)] {
        let p = HealthParams {
            top_k: 1,
            pass_line_percent: Some(line),
            measured_precision: precision(&[("untasked", "untasked", measured)]),
            ..HealthParams::default()
        };
        let r = health_report(std::slice::from_ref(&out), &p);
        assert_eq!(r.counts[0].status, KindStatus::Shown { measured });
        assert_eq!(r.kinds_below_pass_line, 0);
    }
}

#[test]
fn severity_orders_the_top_and_top_k_zero_lists_none() {
    let (_, out, _) = real();
    let mut other = out.clone();
    other.spec.id = LensId("other".into());
    for f in &mut other.findings {
        f.lens = LensId("other".into());
        f.kind = FindingKind::Other("loud".into());
    }
    let p = HealthParams {
        top_k: 3,
        severity: [("loud".to_string(), 9)].into_iter().collect(),
        ..HealthParams::default()
    };
    let r = health_report(&[out.clone(), other.clone()], &p);
    assert_eq!(
        r.counts
            .iter()
            .map(|c| (c.kind.as_str(), c.count))
            .collect::<Vec<_>>(),
        [("untasked", 212), ("loud", 212)],
        "counts in output order"
    );
    assert!(r.top.iter().all(|t| t.kind == "loud" && t.severity == 9));
    assert_eq!(r.top.len(), 3);
    assert_eq!(r.finding_ids.len(), 424);
    assert!(health_report(&[out], &params(0)).top.is_empty());
}

/// An output with no findings and one recorded check, built from a real finding's own search record and control.
fn silent(base: &LensOutput) -> LensOutput {
    let f = &base.findings[0];
    let mut search = f.search.clone();
    search.matched = 0;
    let mut o = base.clone();
    o.findings.clear();
    o.withheld = 0;
    o.null_checks = vec![NullCheck {
        search,
        control: f.control.clone(),
    }];
    o
}

#[test]
fn silence_is_verified_only_with_its_search_and_a_found_control() {
    let (_, base, _) = real();
    let ok = silent(&base);
    let r = health_report(std::slice::from_ref(&ok), &params(5));
    assert_eq!(
        r.silences,
        vec![Silence::Verified {
            lens: LensId("untasked".into()),
            checks: ok.null_checks.clone()
        }]
    );
    assert_eq!(r.silences.len(), 1);
    assert_eq!(ok.null_checks[0].control.outcome, ControlOutcome::Found);
    assert_eq!(ok.null_checks[0].search.matched, 0);

    let unverified = |o: LensOutput, why: &str| {
        let r = health_report(std::slice::from_ref(&o), &params(5));
        assert_eq!(
            r.silences,
            vec![Silence::Unverified {
                lens: LensId("untasked".into()),
                reason: why.into()
            }]
        );
    };
    let mut no_checks = ok.clone();
    no_checks.null_checks.clear();
    unverified(
        no_checks,
        "the lens recorded no search and no control for its empty result",
    );
    let mut bad_control = ok.clone();
    bad_control.null_checks[0].control.outcome = ControlOutcome::NotFound;
    unverified(
        bad_control,
        "a control did not find its own subject, so the search cannot be trusted",
    );
    let mut withheld = ok.clone();
    withheld.withheld = 4;
    unverified(
        withheld,
        "findings were withheld because a control failed or could not be run",
    );
    let mut matched = ok;
    matched.null_checks[0].search.matched = 1;
    unverified(
        matched,
        "a recorded search matched something, so it does not show an absence",
    );
}

#[test]
fn explain_expands_one_finding_from_the_corpus() {
    let (corpus, out, _) = real();
    let f = &out.findings[0];
    let e = explain(&f.id, std::slice::from_ref(&out), &corpus).unwrap();
    assert_eq!(e.id, f.id);
    assert_eq!(e.kind, "untasked");
    assert_eq!(e.statement, f.statement);
    assert_eq!(e.derivation, f.derivation);
    assert_eq!(e.search, f.search);
    assert_eq!(e.control, f.control);
    assert_eq!(e.claims.len(), 1);
    let (doc, claim) = corpus
        .claims()
        .find(|(_, c)| c.unit.id == f.claims[0].id)
        .unwrap();
    assert_eq!(e.claims[0].claim, f.claims[0]);
    assert_eq!(e.claims[0].text.as_deref(), Some(claim.text.as_str()));
    assert_eq!(e.claims[0].path.as_deref(), Some(doc.path.as_str()));
    assert_eq!(
        e.claims[0].line_start,
        Some(claim.unit.source.span.line_start)
    );
    assert!(
        e.statement.contains(&claim.text),
        "the statement quotes the claim"
    );
}

#[test]
fn explain_says_when_it_cannot() {
    let (corpus, out, _) = real();
    let ghost = FindingId("f-0000000000000000".into());
    assert_eq!(
        explain(&ghost, std::slice::from_ref(&out), &corpus),
        Err(ExplainError::NotFound(ghost))
    );
    // A stale reference (the claim is not in the corpus given) is reported as missing, not invented.
    let id = out.findings[0].id.clone();
    let e = explain(&id, &[out], &Corpus::default()).unwrap();
    assert_eq!(e.claims[0].text, None);
    assert_eq!(e.claims[0].path, None);
    assert_eq!(e.claims[0].line_start, None);
}
