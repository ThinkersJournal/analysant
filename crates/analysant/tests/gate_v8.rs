// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gate A, slice 4 (SPEC-STEP1 5.1, section 6): the contradiction lens, given the reference project only
//! as data (its corpus, its alias file, and `.analysant/project.toml` with its seed sense table).
//!
//! Part 1 reproduces the spike's run v8, 37 pairs, line for line and in order, with no sense table.
//! Part 2 loads the seed sense table and checks that exactly the 21 pairs the spike's report triaged as
//! "one generic word carrying the pair" are suppressed (each with its reason), the 16 others stand, and
//! the cancellation pair, the one real contradiction, is still found.

use analysant::{ProjectConfig, classify_corpus, contradiction, extract_corpus};
use analysant_core::{BlockKind, English, Form, FormatAdapter, Ledger, Markdown};
use analysant_logical::{
    Config, Contradiction, ContradictionLens, Findings, Statement, StatementSource,
};
use analysant_pragmatic::{Config as PragmaticConfig, PragmaticLens};
use analysant_semantic::{ConceptStore, LayerSource};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

mod common;
use common::{read, reference, run_file};

fn project_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fuel-project")
}

fn project() -> ProjectConfig {
    ProjectConfig::parse(&read(project_dir().join(".analysant/project.toml")))
        .expect("the fixture's project.toml is valid")
}

/// The wide corpus as run v8 read it: every file of `arch-wide/`, doc 15 included.
fn files() -> Vec<(String, String)> {
    let dir = reference().join("corpus/fuel/arch-wide");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md"))
        .collect();
    names.sort();
    names
        .iter()
        .map(|n| (n.clone(), read(dir.join(n))))
        .collect()
}

/// A heading's section as the spike printed it: the heading stack *before* the new heading closes the
/// headings it replaces, so a heading's "section" also lists the sibling it follows (SPEC-STEP1 slice 1,
/// deviation 7: the engine's section is the real parents; this rebuilds the spike's string only to
/// compare a printed line).
fn spike_heading_sections(files: &[(String, String)]) -> HashMap<(String, usize), String> {
    let mut out = HashMap::new();
    for (path, text) in files {
        let mut stack: Vec<(u8, String)> = Vec::new();
        for b in Markdown.blocks(text) {
            let BlockKind::Heading { level } = b.kind else {
                continue;
            };
            let section = stack
                .iter()
                .map(|(_, t)| t.as_str())
                .collect::<Vec<_>>()
                .join(" > ");
            out.insert((path.clone(), b.line as usize), section);
            while stack.last().is_some_and(|(l, _)| *l >= level) {
                stack.pop();
            }
            stack.push((level, b.text.clone()));
        }
    }
    out
}

fn source_label(s: &StatementSource) -> &'static str {
    match s {
        StatementSource::Heading => "heading",
        StatementSource::Claim(Form::ListItem) => "directive claim",
        StatementSource::Claim(Form::TableCell | Form::Bold) => "named claim",
        StatementSource::Claim(other) => panic!("not a statement source in this corpus: {other:?}"),
    }
}

/// The spike's printed form of one pair (three lines and a blank).
fn pair_lines(c: &Contradiction, sections: &HashMap<(String, usize), String>) -> Vec<String> {
    let section = |s: &Statement| match s.source {
        StatementSource::Heading => sections[&(s.file.clone(), s.line)].clone(),
        StatementSource::Claim(_) => s.section.clone(),
    };
    vec![
        format!(
            "  NEG {}  [{}] \"{}\" — negated by \"{}\"",
            c.negated.id,
            section(&c.negated),
            c.negated.text,
            c.negated_by.as_deref().unwrap_or("negated")
        ),
        format!(
            "  POS {}  [{}] \"{}\" — {} (no negation marker)",
            c.positive.id,
            section(&c.positive),
            c.positive.text,
            source_label(&c.positive.source)
        ),
        format!("  shared: {}", c.shared.join(", ")),
        String::new(),
    ]
}

struct Stage {
    files: Vec<(String, String)>,
    statements: Vec<Statement>,
}

fn stage(en: &English, lens: &ContradictionLens<'_>) -> Stage {
    let files = files();
    let mut corpus = extract_corpus(&Markdown, en, &mut Ledger::default(), &files);
    let prag = PragmaticLens::new(en, PragmaticConfig::default()).expect("en carries the data");
    classify_corpus(&mut corpus, &prag);
    assert_eq!(corpus.claims().count(), 3846, "the run's own claim count");
    let statements = contradiction::statements(&corpus, lens);
    Stage { files, statements }
}

fn lens(en: &English) -> ContradictionLens<'_> {
    ContradictionLens::new(
        en,
        Config {
            extra_pairing_stopwords: project().pairing_stopwords,
        },
    )
    .expect("en carries the data")
}

fn run_lines() -> Vec<String> {
    let want = run_file("fuel-2026-11-16-v8.txt");
    assert!(want.contains("contradictions: 37"));
    let mut lines: Vec<String> = want
        .lines()
        .skip_while(|l| !l.starts_with("CONTRADICTIONS"))
        .skip(1)
        .map(str::to_string)
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

fn printed(f: &[Contradiction], sections: &HashMap<(String, usize), String>) -> Vec<String> {
    let mut lines: Vec<String> = f.iter().flat_map(|c| pair_lines(c, sections)).collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

fn store<'a>(en: &'a English) -> ConceptStore<'a> {
    let cfg = project();
    let layers: Vec<LayerSource> = cfg
        .concepts
        .iter()
        .map(|p| LayerSource {
            name: p.clone(),
            text: read(project_dir().join(p)),
        })
        .collect();
    ConceptStore::load(en, &layers).expect("the seed sense table loads")
}

#[test]
fn reproduces_run_v8s_37_contradictions_line_for_line() {
    let en = English::new();
    let l = lens(&en);
    let st = stage(&en, &l);
    let f = l.find_contradictions(&st.statements, None);
    let sections = spike_heading_sections(&st.files);
    assert_eq!(f.pairs.len(), 37);
    assert!(f.suppressed.is_empty(), "no table, nothing suppressed");
    let want = run_lines();
    let ours = printed(&f.pairs, &sections);
    assert_eq!(want.len(), ours.len(), "lines");
    for (n, (w, g)) in want.iter().zip(&ours).enumerate() {
        assert_eq!(w, g, "line {n}");
    }
}

/// The five words the report triaged as "one generic word carrying the pair" (21 pairs).
const GENERIC: [&str; 5] = ["budget", "harvest", "lifecycle", "written", "network"];

fn is_generic(c: &Contradiction) -> bool {
    c.shared.len() == 1 && GENERIC.contains(&c.shared[0].as_str())
}

fn with_table(en: &English) -> (Findings, Findings) {
    let l = lens(en);
    let st = stage(en, &l);
    let store = store(en);
    assert!(store.run_tests().is_empty(), "the table passes its tests");
    (
        l.find_contradictions(&st.statements, None),
        l.find_contradictions(&st.statements, Some(&store)),
    )
}

#[test]
fn the_seed_sense_table_removes_exactly_the_21_generic_word_pairs() {
    let en = English::new();
    let (spike, ours) = with_table(&en);
    let key = |c: &Contradiction| (c.negated.id.clone(), c.positive.id.clone());
    let want_gone: Vec<_> = spike
        .pairs
        .iter()
        .filter(|c| is_generic(c))
        .map(key)
        .collect();
    assert_eq!(want_gone.len(), 21, "the report's count, from the run");
    // By word: budget 11, harvest 4, lifecycle 3, written 2, network 1.
    let count = |w: &str| {
        spike
            .pairs
            .iter()
            .filter(|c| is_generic(c) && c.shared[0] == w)
            .count()
    };
    assert_eq!(GENERIC.map(count), [11, 4, 3, 2, 1]);
    // Suppressed: exactly those 21, in the lens's order, each with a sense split naming its token.
    let gone: Vec<_> = ours.suppressed.iter().map(|s| key(&s.pair)).collect();
    assert_eq!(gone, want_gone);
    for s in &ours.suppressed {
        assert_eq!(s.splits.len(), 1);
        assert_eq!(s.splits[0].token, s.pair.shared[0]);
        assert_eq!(s.splits[0].rule, "different-senses");
        assert!(
            s.splits[0].note.contains("two senses of"),
            "{:?}",
            s.splits[0]
        );
    }
    // The 16 others stand, unchanged and in order.
    let want_stay: Vec<_> = spike
        .pairs
        .iter()
        .filter(|c| !is_generic(c))
        .map(key)
        .collect();
    assert_eq!(want_stay.len(), 16);
    assert_eq!(ours.pairs.iter().map(key).collect::<Vec<_>>(), want_stay);
}

#[test]
fn the_cancellation_pair_is_still_found_with_the_table_loaded() {
    let en = English::new();
    let (_, ours) = with_table(&en);
    let cancel: Vec<_> = ours
        .pairs
        .iter()
        .filter(|c| c.negated.text == "Cancellation is not supported")
        .collect();
    assert_eq!(cancel.len(), 1);
    assert_eq!(cancel[0].negated.id, "06-runtime.md:71");
    assert_eq!(cancel[0].positive.id, "15-consumer-contract.md:91");
    assert_eq!(cancel[0].shared, ["cancellation"]);
    assert_eq!(cancel[0].negated_by.as_deref(), Some("not supported"));
}

#[test]
fn a_context_the_table_cannot_decide_suppresses_nothing() {
    // Positive control for the direction of the error: with the senses' cues removed, every word is
    // ambiguous and all 37 pairs stand.
    let en = English::new();
    let l = lens(&en);
    let st = stage(&en, &l);
    let text = read(project_dir().join("senses.concepts"))
        .lines()
        .map(|line| {
            if line.starts_with("sense ") {
                // `cues: a, b` -> a cue no context contains.
                let head = line.split("| cues:").next().unwrap_or(line);
                format!("{head}| cues: zzzzunseen")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let store = ConceptStore::load(
        &en,
        &[LayerSource {
            name: "no-cues".into(),
            text,
        }],
    )
    .expect("loads");
    let f = l.find_contradictions(&st.statements, Some(&store));
    assert_eq!(f.pairs.len(), 37);
    assert!(f.suppressed.is_empty());
}

#[test]
fn the_project_file_carries_the_ownership_conventions_slice_3_took_by_hand() {
    let o = project().ownership.expect("declared");
    assert_eq!(o.row_word, "row");
    assert_eq!(o.row_start, ". **");
    assert_eq!(o.citation_prefix, "design-notes/");
}
