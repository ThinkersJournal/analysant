// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gate A, slice 1 (SPEC-STEP1 5.1, section 6): the generic crates, given the reference project only
//! as data, reproduce the spike's run v3. The fixtures are the spike's corpus, run file and alias file
//! at `reference/pip-spike-2026-11-16` (D9: not duplicated).
//!
//! The project's vocabulary lives in THIS file, which is a test: the map from a claim's form to the
//! spike's report categories is a report template, not engine behaviour (5.1, item 4).

use analysant::{directed_but_untasked, extract_corpus};
use analysant_core::{English, Form, Ledger, Markdown};
use analysant_lexical::{AliasTable, MatchMode};
use std::fs;
use std::path::PathBuf;

fn reference() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/pip-spike-2026-11-16")
}

fn read(p: PathBuf) -> String {
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

struct Fixture {
    files: Vec<(String, String)>,
    plan: Vec<String>,
    aliases_text: String,
    v3: String,
}

fn fixture() -> Fixture {
    let root = reference();
    let dir = root.join("corpus/fuel/arch-run");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md"))
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["02-layers.md", "03-ir.md", "06-runtime.md"],
        "the narrow corpus"
    );
    let files = names
        .iter()
        .map(|n| (n.clone(), read(dir.join(n))))
        .collect();
    Fixture {
        files,
        plan: read(root.join("corpus/fuel/ROADMAP.md"))
            .lines()
            .map(str::to_string)
            .collect(),
        aliases_text: read(root.join("aliases.txt")),
        v3: read(root.join("runs/fuel-2026-11-16-v3.txt")).replace('\r', ""),
    }
}

/// The spike's three report categories, by form.
fn category(form: &Form) -> usize {
    match form {
        Form::ListItem => 0,
        Form::TableCell | Form::Bold => 1,
        Form::CodeSpan => 2,
        other => panic!("slice 1 extracts no {other:?}"),
    }
}

const HEADINGS: [&str; 3] = [
    "ABSENT DIRECTIVES",
    "ABSENT NAMED CAPABILITIES",
    "ABSENT CODE IDENTIFIERS",
];

/// The absent lines of one category in run v3, verbatim.
fn v3_section(v3: &str, heading: &str) -> Vec<String> {
    let mut lines = v3.lines().skip_while(|l| !l.starts_with(heading));
    assert!(lines.next().is_some(), "{heading} not in v3");
    lines
        .take_while(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

struct Ours {
    claims: [usize; 3],
    absent: [Vec<String>; 3],
    mentioned: usize,
    withheld: usize,
    keys: Vec<(String, u32, String)>,
}

fn run(aliases_text: &str, fx: &Fixture) -> Ours {
    let en = English::new();
    let mut ledger = Ledger::default();
    let corpus = extract_corpus(&Markdown, &en, &mut ledger, &fx.files);
    let aliases = AliasTable::parse(&en, aliases_text);
    let result = directed_but_untasked(&en, &corpus, &fx.plan, &aliases, MatchMode::TokenSubset);
    let claims: Vec<_> = corpus.claims().collect();
    let mut out = Ours {
        claims: [0; 3],
        absent: [vec![], vec![], vec![]],
        mentioned: result.mentioned(),
        withheld: result.withheld(),
        keys: Vec::new(),
    };
    for (doc, c) in &claims {
        out.claims[category(&c.form)] += 1;
        out.keys.push((
            doc.path.clone(),
            c.unit.source.span.line_start,
            c.normal.key.clone(),
        ));
    }
    for (idx, finding) in result.absent() {
        let (doc, c) = claims[idx];
        assert_eq!(finding.claims.len(), 1);
        let section = doc.section_path(c).join(" > ");
        out.absent[category(&c.form)].push(format!(
            "  {}:{}  [{}] {}",
            doc.path, c.unit.source.span.line_start, section, c.text
        ));
    }
    out
}

#[test]
fn reproduces_run_v3() {
    let fx = fixture();
    let ours = run(&fx.aliases_text, &fx);

    // The header of v3, read from the run file, not typed here.
    let header = fx.v3.lines().next().unwrap();
    assert_eq!(
        header,
        "claims extracted: 558 (257 named, 104 directives, 197 identifiers)"
    );
    assert_eq!(ours.claims, [104, 257, 197]);
    assert_eq!(ours.claims.iter().sum::<usize>(), 558);
    assert_eq!(fx.plan.len(), 3351, "plan lines");
    assert!(fx.v3.contains("roadmap lines: 3351"));
    assert!(fx.v3.contains(
        "findings: 212 absent (75 directives, 103 named, 34 identifiers), 346 mentioned"
    ));
    assert_eq!(ours.mentioned, 346);
    assert_eq!(ours.withheld, 0);
    assert_eq!(
        [
            ours.absent[0].len(),
            ours.absent[1].len(),
            ours.absent[2].len()
        ],
        [75, 103, 34]
    );

    // The finding sets, line for line and in order, against v3's own lists.
    for (cat, heading) in HEADINGS.iter().enumerate() {
        let want = v3_section(&fx.v3, heading);
        assert_eq!(want.len(), ours.absent[cat].len(), "{heading}: count");
        for (i, (w, g)) in want.iter().zip(&ours.absent[cat]).enumerate() {
            assert_eq!(w, g, "{heading}: entry {i}");
        }
    }
}

#[test]
fn every_absence_carries_a_search_record_and_a_passing_control() {
    let fx = fixture();
    let en = English::new();
    let mut ledger = Ledger::default();
    let corpus = extract_corpus(&Markdown, &en, &mut ledger, &fx.files);
    let aliases = AliasTable::parse(&en, &fx.aliases_text);
    let run = directed_but_untasked(&en, &corpus, &fx.plan, &aliases, MatchMode::TokenSubset);
    let mut n = 0;
    for (_, f) in run.absent() {
        n += 1;
        assert_eq!(f.search.matched, 0, "the null is stated as a number");
        assert_eq!(f.search.population, 3351);
        assert_eq!(f.search.query.mode, "token-subset");
        assert!(!f.search.query.key.is_empty());
        assert_eq!(f.control.outcome, analysant_core::ControlOutcome::Found);
        assert_ne!(
            f.control.item, f.claims[0],
            "a control must not be the null's own subject"
        );
        assert!(!f.derivation.is_empty());
    }
    assert_eq!(n, 212);
}

#[test]
fn the_alias_layer_is_applied_and_recorded() {
    let fx = fixture();
    let en = English::new();
    let mut ledger = Ledger::default();
    let corpus = extract_corpus(&Markdown, &en, &mut ledger, &fx.files);
    let aliases = AliasTable::parse(&en, &fx.aliases_text);
    assert_eq!(aliases.groups().len(), 1);
    let with = directed_but_untasked(&en, &corpus, &fx.plan, &aliases, MatchMode::TokenSubset);
    let without = directed_but_untasked(
        &en,
        &corpus,
        &fx.plan,
        &AliasTable::default(),
        MatchMode::TokenSubset,
    );
    // Whatever the alias layer changes, it changes in the direction the spike says: it can only
    // remove absences (it merges two names), never add one.
    assert!(without.absent().count() >= with.absent().count());
    for (_, f) in with.absent() {
        assert!(
            f.derivation[0]
                .table_entries
                .iter()
                .all(|e| e == "multi path dag")
        );
    }
}

#[test]
fn finding_ids_are_stable_across_runs_and_distinct_within_one() {
    let fx = fixture();
    let a = run(&fx.aliases_text, &fx);
    let b = run(&fx.aliases_text, &fx);
    assert_eq!(a.keys, b.keys);
    let en = English::new();
    let ids = |ledger: &mut Ledger| {
        let corpus = extract_corpus(&Markdown, &en, ledger, &fx.files);
        let aliases = AliasTable::parse(&en, &fx.aliases_text);
        let r = directed_but_untasked(&en, &corpus, &fx.plan, &aliases, MatchMode::TokenSubset);
        r.absent().map(|(_, f)| f.id.clone()).collect::<Vec<_>>()
    };
    let one = ids(&mut Ledger::default());
    assert_eq!(
        one,
        ids(&mut Ledger::default()),
        "same inputs, same ledger salt: same ids"
    );
    let unique: std::collections::HashSet<_> = one.iter().collect();
    assert_eq!(unique.len(), one.len(), "no two findings share an id");
}

#[test]
fn claim_keys_match_the_spikes_own_extraction() {
    // (file, line, key) for all 558 claims, in order, against what the spike's extractor produced.
    let fx = fixture();
    let ours = run(&fx.aliases_text, &fx);
    let golden = read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v3-keys.tsv"));
    let want: Vec<(String, u32, String)> = golden
        .lines()
        .map(|l| {
            let mut f = l.splitn(3, '\t');
            let file = f.next().unwrap().to_string();
            let line = f.next().unwrap().parse().unwrap();
            (file, line, f.next().unwrap_or("").to_string())
        })
        .collect();
    assert_eq!(want.len(), 558);
    assert_eq!(ours.keys.len(), want.len());
    for (i, (w, g)) in want.iter().zip(&ours.keys).enumerate() {
        assert_eq!(w, g, "claim {i}");
    }
}

#[test]
fn a_broken_matcher_withholds_every_absence_instead_of_reporting_them() {
    use analysant_core::{
        Block, LanguageKind, LanguagePack, LexiconSet, Normalizer, PackId, RawClaim, Span, Token,
    };
    use std::ops::Range;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// English, except every token's folded form differs on every call: a tokenizer/normalizer
    /// that cannot agree with itself. No claim can be found, not even in its own text.
    struct Unstable(English, AtomicUsize);
    impl LanguagePack for Unstable {
        fn id(&self) -> PackId {
            self.0.id()
        }
        fn script_kind(&self) -> LanguageKind {
            self.0.script_kind()
        }
        fn tokenize(&self, s: &Span, t: &str) -> Vec<Token> {
            let mut toks = self.0.tokenize(s, t);
            for tok in &mut toks {
                tok.norm = format!("{}#{}", tok.norm, self.1.fetch_add(1, Ordering::Relaxed));
            }
            toks
        }
        fn sentences(&self, t: &str) -> Vec<Range<usize>> {
            self.0.sentences(t)
        }
        fn normalizer(&self) -> &dyn Normalizer {
            self.0.normalizer()
        }
        fn lexicons(&self) -> &LexiconSet {
            self.0.lexicons()
        }
        fn extract(&self, b: &Block) -> Vec<RawClaim> {
            self.0.extract(b)
        }
    }

    let fx = fixture();
    let pack = Unstable(English::new(), AtomicUsize::new(0));
    let mut ledger = Ledger::default();
    let corpus = extract_corpus(&Markdown, &pack, &mut ledger, &fx.files);
    let run = directed_but_untasked(
        &pack,
        &corpus,
        &fx.plan,
        &AliasTable::default(),
        MatchMode::TokenSubset,
    );
    assert_eq!(run.absent().count(), 0, "no absence may be reported");
    assert!(run.withheld() > 0, "the absences are withheld, and counted");
    // Positive control for this test: the same corpus on the stable pack does report absences.
    let en = English::new();
    let ok = extract_corpus(&Markdown, &en, &mut Ledger::default(), &fx.files);
    let good = directed_but_untasked(
        &en,
        &ok,
        &fx.plan,
        &AliasTable::default(),
        MatchMode::TokenSubset,
    );
    assert!(good.absent().count() > 0);
    assert_eq!(good.withheld(), 0);
}
