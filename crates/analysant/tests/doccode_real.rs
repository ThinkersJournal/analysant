// SPDX-License-Identifier: MIT OR Apache-2.0
//! The doc-vs-code lens on the reference project's real code (SPEC-STEP1 5.1 item 3): the narrow corpus of the
//! slice 1 gate against the code at the pinned commit. The tree is not in this repository: set
//! `ANALYSANT_REFERENCE_CODE` to an extract or a plain checkout of it (CI does, see `reference-gate`). Without it the
//! tests say so and return locally; with `ANALYSANT_REQUIRE_REFERENCE` set (CI) a missing tree fails instead.
//! The same pattern as `analysant-rust`'s `gate_rows`; there is no second way to skip.

use analysant::doccode::{DocCodeRun, doc_vs_code};
use analysant::{Corpus, extract_corpus};
use analysant_core::{English, Ledger, Markdown};
use analysant_rust::{CodeIndex, RustPack};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

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

fn index() -> Option<CodeIndex> {
    let required = std::env::var_os("ANALYSANT_REQUIRE_REFERENCE").is_some();
    let Some(root) = std::env::var_os("ANALYSANT_REFERENCE_CODE") else {
        assert!(
            !required,
            "ANALYSANT_REQUIRE_REFERENCE is set but ANALYSANT_REFERENCE_CODE is not"
        );
        eprintln!("SKIPPED doccode_real: ANALYSANT_REFERENCE_CODE is not set");
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
    Some(index)
}

fn corpus() -> Corpus {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/pip-spike-2026-11-16/corpus/fuel/arch-run");
    let files: Vec<(String, String)> = ["02-layers.md", "03-ir.md", "06-runtime.md"]
        .iter()
        .map(|n| {
            (
                (*n).to_string(),
                fs::read_to_string(dir.join(n)).expect("the reference corpus"),
            )
        })
        .collect();
    extract_corpus(&Markdown, &English::new(), &mut Ledger::default(), &files)
}

fn run() -> Option<&'static (Corpus, DocCodeRun)> {
    static RUN: OnceLock<Option<(Corpus, DocCodeRun)>> = OnceLock::new();
    RUN.get_or_init(|| {
        let idx = index()?;
        let c = corpus();
        let r = doc_vs_code(&c, &idx);
        eprintln!(
            "doccode_real: files {}, items {}, named {}, findings {}, withheld {}, resolved {}",
            r.files,
            r.items,
            r.named(),
            r.findings().count(),
            r.withheld(),
            r.resolved()
        );
        for (_, f) in r.findings() {
            eprintln!("  {} | {}", f.kind.label(), f.statement);
        }
        Some((c, r))
    })
    .as_ref()
}

#[test]
fn the_real_run_is_not_withheld_wholesale() {
    let Some((_, r)) = run() else { return };
    assert_eq!(r.files, 951, "the tree at the pinned commit");
    assert!(r.named() > 0);
}

#[test]
fn the_audit_rows_the_spike_checked_by_hand_come_out_as_the_pack_says() {
    let Some((_, r)) = run() else { return };
    let by = |q: &str| {
        r.findings()
            .map(|(_, f)| f)
            .find(|f| f.search.query.key == q)
            .unwrap_or_else(|| panic!("no finding for {q}"))
    };
    // Row 4: `PlanStore` has comment mentions and no definition.
    let plan_store = by("PlanStore");
    assert_eq!(plan_store.kind.label(), "absent");
    assert_eq!(plan_store.search.matched, 0);
    assert!(
        plan_store.search.population > 33_000,
        "{:?}",
        plan_store.search
    );
    // Row 2: `pick_route`'s caller shares its bare name with 40 other production definitions: unsettled, not "built".
    let pick = by("pick_route");
    assert_eq!(pick.kind.label(), "shared-name");
    assert!(
        pick.statement
            .contains("shares its name with 40 other definitions"),
        "{}",
        pick.statement
    );
    // Every finding is vouched for by a different claim that resolves, never by its own subject.
    for (_, f) in r.findings() {
        assert_eq!(f.control.kind, analysant_core::ControlKind::KnownPresent);
        assert_eq!(f.control.outcome, analysant_core::ControlOutcome::Found);
        assert_ne!(f.control.item, f.claims[0]);
    }
}

#[test]
fn the_counts_at_the_pinned_commit() {
    // A regression pin, measured on 2026-11-20 at the reference tree's pinned commit over the three narrow
    // corpus documents; it is not an oracle, and Gate B (not this) is what says whether the findings are true.
    let Some((_, r)) = run() else { return };
    assert_eq!(
        (r.named(), r.findings().count(), r.withheld(), r.resolved()),
        (133, 56, 0, 77)
    );
}

#[test]
fn the_counts_per_kind_at_the_pinned_commit() {
    let Some((_, r)) = run() else { return };
    let n = |k: &str| r.findings().filter(|(_, f)| f.kind.label() == k).count();
    assert_eq!(
        [
            n("absent"),
            n("tests-only"),
            n("uncalled"),
            n("dead-callers"),
            n("shared-name")
        ],
        [41, 4, 5, 3, 3]
    );
}
