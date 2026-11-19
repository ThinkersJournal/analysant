// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gate A, slice 5 (SPEC-STEP1 5.1 item 3, section 6): the seven validation rows of the spike report
//! (`spike-report-2026-11-16.md:463-471`) against the reference project's code at the pinned commit.
//!
//! The tree is not in this repository. Point `ANALYSANT_REFERENCE_CODE` at an extract of it
//! (`git archive b0d74efa | tar -x -C <dir>`); without it the test says so and returns locally (CI sets `ANALYSANT_REQUIRE_REFERENCE=1`, where
//! an unset variable or a missing tree fails instead; a plain checkout works too), and the
//! synthetic rows in `tests/shapes.rs` still run. The expectations below are the audit's rows, each one
//! stated as what the engine must say: four rows agree with the audit, and the three where the spike
//! disagreed must report the disagreement explicitly (reach, not a bare "built").

use analysant_rust::{CodeIndex, Context, Reach, Relation, RustPack};
use std::fs;
use std::path::{Path, PathBuf};

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().filter_map(Result::ok).collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn load() -> Option<&'static CodeIndex> {
    static INDEX: std::sync::OnceLock<Option<CodeIndex>> = std::sync::OnceLock::new();
    INDEX.get_or_init(load_once).as_ref()
}

fn load_once() -> Option<CodeIndex> {
    // CI sets ANALYSANT_REQUIRE_REFERENCE=1: there an unset variable or a missing tree fails, never skips.
    let required = std::env::var_os("ANALYSANT_REQUIRE_REFERENCE").is_some();
    let Some(root) = std::env::var_os("ANALYSANT_REFERENCE_CODE") else {
        assert!(
            !required,
            "ANALYSANT_REQUIRE_REFERENCE is set but ANALYSANT_REFERENCE_CODE is not"
        );
        eprintln!("SKIPPED gate_rows: ANALYSANT_REFERENCE_CODE is not set");
        return None;
    };
    let root = PathBuf::from(root);
    assert!(
        !required || root.is_dir(),
        "the reference tree is missing at {}",
        root.display()
    );
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    let mut index = CodeIndex::new(RustPack::new());
    for f in &files {
        let rel = f
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        index.add_source(&rel, &fs::read_to_string(f).unwrap());
    }
    eprintln!(
        "gate_rows: {} files, {} items, {} parse failures",
        index.file_count(),
        index.item_count(),
        index.failures().len()
    );
    Some(index)
}

#[test]
fn the_reference_tree_is_read_whole() {
    let Some(index) = load() else { return };
    assert_eq!(index.file_count(), 951, "the spike scanned 951 files");
    for f in index.failures() {
        eprintln!("parse failure: {} {}", f.file, f.reason);
    }
    assert!(index.failures().is_empty(), "every file parses");
}

#[test]
fn row1_cancellation_is_missing() {
    let Some(index) = load() else { return };
    for name in ["CancellationToken", "cancel_token"] {
        let v = index.relate(name);
        assert_eq!(v.relation, Relation::Missing, "{name}");
        assert_eq!(
            v.mentions.code_production + v.mentions.code_test,
            0,
            "{name} has no code mention"
        );
    }
}

#[test]
fn row2_pick_route_is_reached_only_from_dead_code() {
    let Some(index) = load() else { return };
    let v = index.relate("pick_route");
    eprintln!("{v:#?}");
    assert_eq!(v.relation, Relation::Implements);
    // The disagreement is reported, not hidden: its only production caller is `RouteCache::resolve`,
    // whose own callers cannot be told from the other `resolve` definitions by name (the audit knows
    // they are all in tests). The engine says so instead of answering built.
    match v.reach.as_ref().unwrap() {
        Reach::CalledIfSharedNameResolves {
            callers,
            sharing_definitions,
            ..
        } => {
            assert!(
                callers
                    .iter()
                    .any(|c| c.within.as_deref() == Some("RouteCache::resolve")),
                "{callers:?}"
            );
            assert!(*sharing_definitions >= 1);
        }
        other => panic!("expected the unsettled shared-name reach, got {other:?}"),
    }
    let rc = index.relate("RouteCache");
    assert_eq!(rc.relation, Relation::Implements);
    let lk = index.relate("LOOKAHEAD_K");
    assert_eq!(lk.relation, Relation::Implements);
    assert!(
        matches!(lk.reach, Some(Reach::Uncalled { .. })),
        "{:?}",
        lk.reach
    );
}

#[test]
fn row3_rule_registry_chain() {
    let Some(index) = load() else { return };
    let d = index.relate("default_rules");
    eprintln!("{d:#?}");
    assert_eq!(d.relation, Relation::Implements);
    assert!(
        matches!(d.reach, Some(Reach::Uncalled { test_uses }) if test_uses >= 2),
        "callers are in test modules only: {:?}",
        d.reach
    );
    let c = index.relate("capability_gated_rules");
    eprintln!("{c:#?}");
    assert_eq!(c.relation, Relation::Implements);
    match c.reach.as_ref().unwrap() {
        Reach::OnlyFromUncalled { callers, .. } => {
            assert!(
                callers
                    .iter()
                    .any(|s| s.within.as_deref() == Some("RuleRegistry::default_rules")),
                "{callers:?}"
            );
        }
        other => panic!("expected the dead chain to be reported, got {other:?}"),
    }
    let f = index.relate("optimize_to_fixpoint");
    eprintln!("{f:#?}");
    assert_eq!(f.relation, Relation::Implements);
    match f.reach.as_ref().unwrap() {
        Reach::Called { callers, .. } => {
            assert!(
                callers
                    .iter()
                    .any(|s| s.within.as_deref() == Some("lower_to_base_map")),
                "{callers:?}"
            );
        }
        other => panic!("expected production callers named, got {other:?}"),
    }
}

#[test]
fn row4_plan_store_is_missing_with_comments_only() {
    let Some(index) = load() else { return };
    let v = index.relate("PlanStore");
    eprintln!("{v:#?}");
    assert_eq!(v.relation, Relation::Missing);
    assert_eq!(v.mentions.code_production + v.mentions.code_test, 0);
    assert_eq!(
        v.mentions.comment, 4,
        "exactly 4 hits in the tree, all in comments"
    );
}

#[test]
fn row5_coupling_field_is_read_only_in_a_test() {
    let Some(index) = load() else { return };
    let v = index.relate("coupling");
    // The audit's STUB, exactly: a production field (`candidate.rs:82`), one `.coupling` read, in a test.
    assert_eq!(v.relation, Relation::Implements);
    assert!(
        v.definitions
            .iter()
            .any(|d| d.file == "fuel-dispatch/src/ranker/candidate.rs" && d.line == 82)
    );
    assert_eq!(v.reach, Some(Reach::Uncalled { test_uses: 1 }));
    assert_eq!(v.mentions.code_production, 0);
}

#[test]
fn row6_judge_baseline_is_missing() {
    let Some(index) = load() else { return };
    let v = index.relate("JudgeBaseline");
    assert_eq!(v.relation, Relation::Missing, "{v:#?}");
    // A bare `Baseline` exists, but only as a struct in a benchmark file under `tests/`: test code,
    // never production, so it is `Tests` and cannot build the document's Judge baseline.
    let b = index.relate("Baseline");
    assert_eq!(b.relation, Relation::Tests, "{b:#?}");
    assert!(b.definitions.iter().all(|d| d.context == Context::Test));
    let p = index.relate("populate_dispatch_table");
    assert_eq!(p.relation, Relation::Implements);
    assert!(
        matches!(p.reach, Some(Reach::Uncalled { .. })),
        "{:?}",
        p.reach
    );
}

#[test]
fn row7_observe_and_tap_are_missing() {
    let Some(index) = load() else { return };
    for q in ["Op::Observe", "Op::Tap"] {
        let v = index.relate(q);
        assert_eq!(v.relation, Relation::Missing, "{q}");
        assert_eq!(v.mentions.code_production + v.mentions.code_test, 0, "{q}");
    }
}

#[test]
fn row6_populate_dispatch_table_is_a_production_fn_nothing_calls() {
    let Some(index) = load() else { return };
    let p = index.relate("populate_dispatch_table");
    assert_eq!(p.relation, Relation::Implements);
    assert!(
        p.definitions
            .iter()
            .any(|d| d.file == "fuel-tensor/src/judge/cache.rs")
    );
}
