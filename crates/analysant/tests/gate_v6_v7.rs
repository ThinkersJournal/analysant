// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gate A, slice 3 (SPEC-STEP1 5.1, section 6): genre handling, given the reference project only as data
//! (the wide corpus without doc 15, its plan, its alias file, and its two conventions as an
//! `OwnershipConfig`). Run v6 is the wide corpus with the pragmatic lens and no genre handling (1164
//! absent); run v7 adds the decision-outcome filter and note-to-row ownership (688 absent, 449 owned).
//! Every list is compared with the run file line for line, in order.

use analysant::{
    Corpus, OwnershipConfig, UntaskedRun, classify_corpus, directed_but_untasked, extract_corpus,
    owners, partition_owned,
};
use analysant_core::{English, Ledger, Markdown};
use analysant_lexical::{AliasTable, MatchMode};
use analysant_pragmatic::{Config, PragmaticLens};
use std::collections::HashMap;
use std::fs;

mod common;
use common::{cat, finding_line, read, reference, run_file, section, set_aside_line};

/// The wide corpus as run v6 and v7 read it: every file of `arch-wide/` but doc 15 (added for v8).
fn files() -> Vec<(String, String)> {
    let dir = reference().join("corpus/fuel/arch-wide");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md") && n != "15-consumer-contract.md")
        .collect();
    names.sort();
    names
        .iter()
        .map(|n| (n.clone(), read(dir.join(n))))
        .collect()
}

fn plan() -> Vec<String> {
    read(reference().join("corpus/fuel/ROADMAP.md"))
        .lines()
        .map(str::to_string)
        .collect()
}

/// The reference project's two conventions, as data.
fn config() -> OwnershipConfig {
    OwnershipConfig {
        row_word: "row".into(),
        row_start: ". **".into(),
        citation_prefix: "design-notes/".into(),
    }
}

struct Stage {
    files: Vec<(String, String)>,
    plan: Vec<String>,
    corpus: Corpus,
    run: UntaskedRun,
}

fn stage(decision_outcomes: bool) -> Stage {
    let files = files();
    let plan = plan();
    let en = English::new();
    let mut corpus = extract_corpus(&Markdown, &en, &mut Ledger::default(), &files);
    let lens = PragmaticLens::new(
        &en,
        Config {
            fragment_rules: true,
            decision_outcomes,
        },
    )
    .expect("en carries the data");
    classify_corpus(&mut corpus, &lens);
    let aliases = AliasTable::parse(&en, &read(reference().join("aliases.txt")));
    let run = directed_but_untasked(&en, &corpus, &plan, &aliases, MatchMode::TokenSubset);
    Stage {
        files,
        plan,
        corpus,
        run,
    }
}

fn owner_map(st: &Stage) -> HashMap<String, usize> {
    owners(&Markdown, &English::new(), &config(), &st.plan, &st.files)
}

fn compare(what: &str, want: &[String], ours: &[String]) {
    assert_eq!(want.len(), ours.len(), "{what}: count");
    for (n, (w, g)) in want.iter().zip(ours).enumerate() {
        assert_eq!(w, g, "{what}: entry {n}");
    }
}

/// The findings and set-aside lists against `want`'s, and the totals against its header.
fn check_absent<'a>(
    st: &'a Stage,
    want: &str,
    owners: &HashMap<String, usize>,
    absent: usize,
    per_cat: [usize; 3],
    aside: usize,
) -> analysant::OwnedPartition<'a> {
    let p = partition_owned(&st.corpus, &st.run, owners);
    assert!(
        want.contains(&format!(
            "findings: {absent} absent ({} directives, {} named, {} identifiers), 2044 mentioned",
            per_cat[0], per_cat[1], per_cat[2]
        )),
        "the run file's own header"
    );
    assert_eq!(st.run.mentioned(), 2044);
    assert_eq!(st.run.withheld(), 0);
    assert_eq!(p.findings.len(), absent);
    assert_eq!(p.set_aside.len(), aside);
    assert!(want.contains(&format!("set aside by the pragmatic lens: {aside} (")));
    let claims: Vec<_> = st.corpus.claims().collect();
    let mut by_cat = [0usize; 3];
    for (i, _) in &p.findings {
        by_cat[cat(&claims[*i].1.form)] += 1;
    }
    assert_eq!(by_cat, per_cat);
    for (c, heading) in [
        "ABSENT DIRECTIVES",
        "ABSENT NAMED CAPABILITIES",
        "ABSENT CODE IDENTIFIERS",
    ]
    .iter()
    .enumerate()
    {
        let ours: Vec<String> = p
            .findings
            .iter()
            .filter(|(i, _)| cat(&claims[*i].1.form) == c)
            .map(|(i, _)| finding_line(&st.corpus, *i))
            .collect();
        compare(heading, &section(want, heading), &ours);
    }
    let ours: Vec<String> = p
        .set_aside
        .iter()
        .map(|(i, _)| set_aside_line(&st.corpus, *i))
        .collect();
    compare(
        "set aside",
        &section(want, "SET ASIDE BY THE PRAGMATIC LENS"),
        &ours,
    );
    p
}

#[test]
fn reproduces_run_v6_the_wide_corpus_without_genre_handling() {
    let st = stage(false);
    let want = run_file("fuel-2026-11-16-v6.txt");
    assert!(want.contains("claims extracted: 3657 ("));
    assert_eq!(st.corpus.claims().count(), 3657);
    // No ownership: every reportable absence is a finding.
    let p = check_absent(&st, &want, &HashMap::new(), 1164, [290, 324, 550], 449);
    assert!(p.owned.is_empty());
}

#[test]
fn reproduces_run_v7_outcome_filter_and_ownership() {
    let st = stage(true);
    let owners = owner_map(&st);
    let want = run_file("fuel-2026-11-16-v7.txt");
    let p = check_absent(&st, &want, &owners, 688, [89, 224, 375], 476);
    assert!(
        want.contains("owned by a ROADMAP row: 449 (175 directives, 99 named, 175 identifiers)")
    );
    assert_eq!(p.owned.len(), 449);
    let claims: Vec<_> = st.corpus.claims().collect();
    let mut by_cat = [0usize; 3];
    for (i, _, _) in &p.owned {
        by_cat[cat(&claims[*i].1.form)] += 1;
    }
    assert_eq!(by_cat, [175, 99, 175]);
    let ours: Vec<String> = p
        .owned
        .iter()
        .map(|(i, _, row)| {
            let line = finding_line(&st.corpus, *i);
            // `  file:line  [section] text` -> `  file:line  [row N] [section] text`
            let (id, rest) = line.trim_start().split_once("  ").expect("id and text");
            format!("  {id}  [row {row}] {rest}")
        })
        .collect();
    compare("owned", &section(&want, "OWNED BY A ROADMAP ROW"), &ours);
}

#[test]
fn the_outcome_filter_moves_exactly_the_runs_27_claims_to_set_aside() {
    let (v6, v7) = (stage(false), stage(true));
    let aside = |s: &Stage| {
        partition_owned(&s.corpus, &s.run, &HashMap::new())
            .set_aside
            .len()
    };
    assert_eq!(aside(&v7) - aside(&v6), 27);
}

#[test]
fn ownership_follows_titles_and_citations_and_ignores_the_rest() {
    let st = stage(true);
    let en = English::new();
    let plan = [
        "Preamble citing design-notes/d.md before any row".to_string(),
        "1. **First row** cites notes/a.md".to_string(),
        "   continued, and cites design-notes/b.md on a later line".to_string(),
        "7. **Seventh row** also cites design-notes/b.md".to_string(),
        "9. Not a row: no bold marker, cites design-notes/c.md".to_string(),
        "1. Add a thing (a sub-list, not a row start)".to_string(),
    ];
    let cfg = config();
    let own =
        |path: &str, text: &str| analysant::owner_for(&Markdown, &en, &cfg, &plan, path, text);
    // (b) the lowest-numbered citing row, the citing line being a continuation of row 1.
    assert_eq!(own("b.md", "# Anything\n"), Some(1));
    // (a) a title naming an existing row wins over a citation.
    assert_eq!(own("b.md", "# Plan for Row 7 step 2\n"), Some(7));
    assert_eq!(own("x.md", "# Row7\n"), Some(7));
    // A line without the bold marker is a continuation of the nearest preceding row (here 7), not a row.
    assert_eq!(
        own(
            "c.md",
            "# Anything
"
        ),
        Some(7)
    );
    // A title naming a row that does not exist, a bare "row", and a citation before any row: none own.
    assert_eq!(own("x.md", "# Plan for row 99\n"), None);
    assert_eq!(own("x.md", "# The row below\n"), None);
    assert_eq!(
        own(
            "d.md",
            "# Anything
"
        ),
        None
    );
    // Only a level-1 heading is a title.
    assert_eq!(own("x.md", "## Row 7\n"), None);
    // Positive control: the real wide corpus has owned notes under the same function.
    assert!(owner_map(&st).len() >= 10);
}
