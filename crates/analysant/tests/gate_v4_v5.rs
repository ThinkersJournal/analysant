// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gate A, slice 2 (SPEC-STEP1 5.1, section 6): the pragmatic lens, given the reference project only as
//! data, reproduces the spike's runs v4 (status, directive and description rules: absent 212 to 105)
//! and v5 (plus the fragment rules for names: 73). Findings AND the set-aside list are compared with
//! the run files line for line, in order.

use analysant::{
    Corpus, Partition, UntaskedRun, classify_corpus, directed_but_untasked, extract_corpus,
    partition,
};
use analysant_core::{English, Form, Kind, Ledger, Markdown};
use analysant_lexical::{AliasTable, MatchMode};
use analysant_pragmatic::{Config, PragmaticLens};
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
    aliases: String,
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
    Fixture {
        files: names
            .iter()
            .map(|n| (n.clone(), read(dir.join(n))))
            .collect(),
        plan: read(root.join("corpus/fuel/ROADMAP.md"))
            .lines()
            .map(str::to_string)
            .collect(),
        aliases: read(root.join("aliases.txt")),
    }
}

fn run_file(name: &str) -> String {
    read(reference().join("runs").join(name)).replace('\r', "")
}

fn section(text: &str, heading: &str) -> Vec<String> {
    let mut lines = text.lines().skip_while(|l| !l.starts_with(heading));
    assert!(lines.next().is_some(), "{heading} not in the run file");
    lines
        .take_while(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

struct Stage {
    corpus: Corpus,
    run: UntaskedRun,
}

fn stage(fragment_rules: bool) -> Stage {
    let fx = fixture();
    let en = English::new();
    let mut corpus = extract_corpus(&Markdown, &en, &mut Ledger::default(), &fx.files);
    let lens = PragmaticLens::new(&en, Config { fragment_rules }).expect("en carries the data");
    classify_corpus(&mut corpus, &lens);
    let aliases = AliasTable::parse(&en, &fx.aliases);
    let run = directed_but_untasked(&en, &corpus, &fx.plan, &aliases, MatchMode::TokenSubset);
    Stage { corpus, run }
}

fn cat(form: &Form) -> usize {
    match form {
        Form::ListItem => 0,
        Form::TableCell | Form::Bold => 1,
        Form::CodeSpan => 2,
        other => panic!("no such form in this corpus: {other:?}"),
    }
}

fn finding_line(corpus: &Corpus, idx: usize) -> String {
    let claims: Vec<_> = corpus.claims().collect();
    let (doc, c) = claims[idx];
    format!(
        "  {}:{}  [{}] {}",
        doc.path,
        c.unit.source.span.line_start,
        doc.section_path(c).join(" > "),
        c.text
    )
}

fn set_aside_line(corpus: &Corpus, idx: usize) -> String {
    let claims: Vec<_> = corpus.claims().collect();
    let (doc, c) = claims[idx];
    let what = if c.form == Form::ListItem {
        "Directive"
    } else {
        "Named"
    };
    let role = match c.kind {
        Kind::Description => "Description",
        Kind::Report => "StatusNote",
        other => panic!("not set aside: {other:?}"),
    };
    format!(
        "  {}:{}  [{what} / {role}] [{}] {}",
        doc.path,
        c.unit.source.span.line_start,
        doc.section_path(c).join(" > "),
        c.text
    )
}

fn check(fragment_rules: bool, run_name: &str, absent: usize, per_cat: [usize; 3], aside: usize) {
    let st = stage(fragment_rules);
    let want = run_file(run_name);
    let Partition {
        findings,
        set_aside,
    } = partition(&st.corpus, &st.run);

    // The header numbers, read from the run file and compared to ours.
    assert!(
        want.contains(&format!(
            "findings: {absent} absent ({} directives, {} named, {} identifiers), 346 mentioned",
            per_cat[0], per_cat[1], per_cat[2]
        )),
        "the run file's own header"
    );
    assert_eq!(st.run.mentioned(), 346);
    assert_eq!(st.run.withheld(), 0);
    assert_eq!(findings.len(), absent);
    let mut by_cat = [0usize; 3];
    let claims: Vec<_> = st.corpus.claims().collect();
    for (i, _) in &findings {
        by_cat[cat(&claims[*i].1.form)] += 1;
    }
    assert_eq!(by_cat, per_cat);
    assert_eq!(set_aside.len(), aside);
    assert!(want.contains(&format!("set aside by the pragmatic lens: {aside} (")));

    // The finding lists, line for line and in order, against the run's own.
    for (c, heading) in [
        "ABSENT DIRECTIVES",
        "ABSENT NAMED CAPABILITIES",
        "ABSENT CODE IDENTIFIERS",
    ]
    .iter()
    .enumerate()
    {
        let want_lines = section(&want, heading);
        let ours: Vec<String> = findings
            .iter()
            .filter(|(i, _)| cat(&claims[*i].1.form) == c)
            .map(|(i, _)| finding_line(&st.corpus, *i))
            .collect();
        assert_eq!(want_lines.len(), ours.len(), "{heading}: count");
        for (n, (w, g)) in want_lines.iter().zip(&ours).enumerate() {
            assert_eq!(w, g, "{heading}: entry {n}");
        }
    }

    // The set-aside list: every claim the lens set aside, and the role it gave, as the run lists it.
    let want_aside = section(&want, "SET ASIDE BY THE PRAGMATIC LENS");
    let ours: Vec<String> = set_aside
        .iter()
        .map(|(i, _)| set_aside_line(&st.corpus, *i))
        .collect();
    assert_eq!(want_aside.len(), ours.len(), "set aside: count");
    for (n, (w, g)) in want_aside.iter().zip(&ours).enumerate() {
        assert_eq!(w, g, "set aside: entry {n}");
    }
}

#[test]
fn reproduces_run_v4() {
    check(false, "fuel-2026-11-16-v4.txt", 105, [7, 64, 34], 107);
}

#[test]
fn reproduces_run_v5() {
    check(true, "fuel-2026-11-16-v5.txt", 73, [7, 32, 34], 139);
}

#[test]
fn every_classification_names_its_rule() {
    let fx = fixture();
    let en = English::new();
    let mut corpus = extract_corpus(&Markdown, &en, &mut Ledger::default(), &fx.files);
    let lens = PragmaticLens::new(&en, Config::default()).unwrap();
    let derivations = classify_corpus(&mut corpus, &lens);
    assert_eq!(derivations.len(), 558);
    for d in &derivations {
        let d = d.as_ref().expect("every slice-1 form has a rule");
        assert!(!d.rule.is_empty());
        assert_eq!(d.lens.0, "pragmatic");
    }
    assert!(corpus.claims().all(|(_, c)| c.kind != Kind::Unclassified));
}

#[test]
fn a_pack_without_the_data_is_refused_not_guessed() {
    use analysant_core::{
        Block, LanguageKind, LanguagePack, LexiconSet, Normalizer, PackId, RawClaim, Span, Token,
    };
    use std::ops::Range;
    struct Bare(English, LexiconSet);
    impl LanguagePack for Bare {
        fn id(&self) -> PackId {
            self.0.id()
        }
        fn script_kind(&self) -> LanguageKind {
            self.0.script_kind()
        }
        fn tokenize(&self, s: &Span, t: &str) -> Vec<Token> {
            self.0.tokenize(s, t)
        }
        fn sentences(&self, t: &str) -> Vec<Range<usize>> {
            self.0.sentences(t)
        }
        fn normalizer(&self) -> &dyn Normalizer {
            self.0.normalizer()
        }
        fn lexicons(&self) -> &LexiconSet {
            &self.1
        }
        fn extract(&self, b: &Block) -> Vec<RawClaim> {
            self.0.extract(b)
        }
    }
    let bare = Bare(English::new(), LexiconSet::default());
    let err = PragmaticLens::new(&bare, Config::default())
        .err()
        .expect("refused");
    assert!(err.0.contains(&"imperatives".to_string()));
    assert!(err.0.contains(&"param copula_window".to_string()));
    // Positive control: the real pack is accepted.
    assert!(PragmaticLens::new(&English::new(), Config::default()).is_ok());
}

#[test]
fn rules_fire_in_the_spikes_order() {
    let en = English::new();
    let lens = PragmaticLens::new(&en, Config::default()).unwrap();
    let mut ledger = Ledger::default();
    let mut doc = |text: &str| {
        let files = vec![("t.md".to_string(), format!("# T\n\n- {text}\n"))];
        let mut c = extract_corpus(&Markdown, &en, &mut ledger, &files);
        classify_corpus(&mut c, &lens);
        let claims: Vec<_> = c.claims().map(|(_, c)| c.kind).collect();
        claims[0]
    };
    // A dated retraction that also names a directive is a status note: status wins.
    assert_eq!(
        doc("Build the old graph (retired 2026-01-02)"),
        Kind::Report
    );
    assert_eq!(doc("Build the multi-path graph"), Kind::Directive);
    assert_eq!(doc("The runtime is the owner of caches"), Kind::Description);
    // Word boundary: "shall" must not fire inside "shallow", and a bare item defaults to a directive.
    assert_eq!(
        doc("shallow copies of the thing everywhere"),
        Kind::Directive
    );
    // A code token is not a decision (`Rejected`), a standalone word is.
    assert_eq!(doc("handle the `Rejected` variant"), Kind::Directive);
    assert_eq!(doc("memoization was declined"), Kind::Report);
}
