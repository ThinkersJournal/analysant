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

mod common;
use common::{cat, finding_line, read, reference, run_file, section, set_aside_line};

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

struct Stage {
    corpus: Corpus,
    run: UntaskedRun,
}

fn stage(fragment_rules: bool) -> Stage {
    let fx = fixture();
    let en = English::new();
    let mut corpus = extract_corpus(&Markdown, &en, &mut Ledger::default(), &fx.files);
    let lens = PragmaticLens::new(
        &en,
        Config {
            fragment_rules,
            decision_outcomes: false,
        },
    )
    .expect("en carries the data");
    classify_corpus(&mut corpus, &lens);
    let aliases = AliasTable::parse(&en, &fx.aliases);
    let run = directed_but_untasked(&en, &corpus, &fx.plan, &aliases, MatchMode::TokenSubset);
    Stage { corpus, run }
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

/// The corpus does not exercise every lexicon entry, so the gate cannot pin them all. Each rule group
/// is pinned here by an input that would take a DIFFERENT verdict without that rule (sabotage-checked:
/// dropping the entry from its lexicon turns the assertion red). A bare list item defaults to a
/// directive, so directive rules are pinned under a descriptive heading, where the default is a
/// description; status rules are pinned on text that would otherwise be a directive.
#[test]
fn each_rule_group_is_pinned_by_a_synthetic_input() {
    let en = English::new();
    let lens = PragmaticLens::new(&en, Config::default()).unwrap();
    let kind_of = |md: String, form: Form| {
        let files = vec![("t.md".to_string(), md)];
        let mut c = extract_corpus(&Markdown, &en, &mut Ledger::default(), &files);
        classify_corpus(&mut c, &lens);
        c.claims()
            .find(|(_, c)| c.form == form)
            .map(|(_, c)| c.kind)
    };
    let item = |text: &str| {
        kind_of(
            format!(
                "# T

- {text}
"
            ),
            Form::ListItem,
        )
        .unwrap()
    };
    let described = |text: &str| {
        kind_of(
            format!(
                "# See also

- {text}
"
            ),
            Form::ListItem,
        )
        .unwrap()
    };
    let name = |text: &str| {
        kind_of(
            format!(
                "# T

| H | I |
|---|---|
| {text} | x |
"
            ),
            Form::TableCell,
        )
    };

    // status markers beat a leading imperative
    for m in [
        "as-built",
        "no longer",
        "retired",
        "superseded",
        "deleted",
        "gone",
        "struck",
        "survived",
        "currently",
    ] {
        assert_eq!(item(&format!("add widgets, {m}")), Kind::Report, "{m}");
    }
    assert_eq!(item("add widgets, was a stopgap"), Kind::Report);
    // outcome markers, and a date
    for m in ["rejected", "withdrawn", "declined"] {
        assert_eq!(item(&format!("add widgets {m}")), Kind::Report, "{m}");
    }
    assert_eq!(item("add widgets on 2026-01-02"), Kind::Report);
    // modals, imperatives and prefixes beat the descriptive section they sit under
    for m in [
        "must",
        "should",
        "shall",
        "needs to",
        "need to",
        "has to",
        "have to",
        "required to",
    ] {
        assert_eq!(
            described(&format!("the pool {m} close early")),
            Kind::Directive,
            "{m}"
        );
    }
    for v in ["add", "build", "write", "use", "verify"] {
        assert_eq!(described(&format!("{v} widgets")), Kind::Directive, "{v}");
    }
    for p in ["TODO widgets", "[ ] widgets", "[x] widgets"] {
        assert_eq!(described(p), Kind::Directive, "{p}");
    }
    // description signals, outside a descriptive section (a bare item would be a directive)
    assert_eq!(described("widgets and gadgets"), Kind::Description);
    assert_eq!(item("[widgets](x.md) and gadgets"), Kind::Description);
    assert_eq!(item("rule seven holds always"), Kind::Description);
    for o in ["the", "a", "an", "this", "these", "that", "each", "every"] {
        assert_eq!(
            item(&format!("{o} pool is large")),
            Kind::Description,
            "{o}"
        );
    }
    assert_eq!(item("each pool are large"), Kind::Description);
    // names: finite verbs and openings are sentences, each fragment rule is a fragment
    for w in [
        "is", "are", "was", "does", "will", "has", "must", "should", "cannot",
    ] {
        assert_eq!(
            name(&format!("pool {w} large")),
            Some(Kind::Description),
            "{w}"
        );
    }
    for o in [
        "before start",
        "between pools",
        "after start",
        "during start",
        "until start",
        "when ready",
        "if ready",
        "while idle",
        "rule seven",
        "not at all useful",
    ] {
        assert_eq!(name(o), Some(Kind::Description), "{o}");
    }
    for n in [
        "v2 layout",
        "few pools",
        "only pools",
        "no pools",
        "never pools",
        "lazily",
        "populated early",
        "loading data",
        "not yet pools",
    ] {
        assert_eq!(name(n), Some(Kind::Description), "{n}");
    }
    assert_eq!(name("keep the pool"), Some(Kind::Description));
    assert_eq!(name("pool runs hot"), Some(Kind::Description));
    assert_eq!(name("per ending device"), Some(Kind::Description));
    assert_eq!(name("per pool with sets"), Some(Kind::Description));
    assert_eq!(name("pool refined for cells"), Some(Kind::Description));
    for t in [
        "at startup",
        "each pass",
        "per token",
        "per input",
        "mid realize",
        "at dispatch",
        "at runtime",
        "per pass",
        "at finalize",
        "at realize",
    ] {
        assert_eq!(
            name(&format!("pool work {t}")),
            Some(Kind::Description),
            "{t}"
        );
    }
    for w in ["after", "before", "during", "until", "when", "while"] {
        assert_eq!(
            name(&format!("Pool state {w} start.")),
            Some(Kind::Description),
            "{w}"
        );
    }
    // every descriptive section title, a status marker the corpus never reaches
    for sec in [
        "see also",
        "where this lives",
        "crate boundaries",
        "what lives where",
        "what's not in",
        "quick reference",
    ] {
        let md = format!(
            "# {sec}

- widgets and gadgets
"
        );
        assert_eq!(
            kind_of(md, Form::ListItem),
            Some(Kind::Description),
            "{sec}"
        );
    }
    assert_eq!(
        item("add widgets, listed here as current until then"),
        Kind::Report
    );
    // finite verbs that no sentence marker shadows
    for w in [
        "be",
        "been",
        "do",
        "did",
        "don't",
        "can",
        "could",
        "may",
        "might",
        "need",
        "needs",
        "had",
        "runs",
        "ships",
        "refines",
        "salvages",
        "changes",
        "changed",
        "live",
        "diverge",
        "reconverge",
    ] {
        assert_eq!(
            name(&format!("pool {w} thing")),
            Some(Kind::Description),
            "{w}"
        );
    }
    // each preposition after a predicative participle; a participle late in the phrase
    for p in [
        "at", "in", "on", "for", "with", "per", "from", "to", "by", "as", "after", "before",
        "during", "into", "over", "under", "of",
    ] {
        assert_eq!(
            name(&format!("pool refined {p} cells")),
            Some(Kind::Description),
            "{p}"
        );
    }
    assert_eq!(name("pool used at cells"), Some(Kind::Description));
    assert_eq!(name("none thing"), Some(Kind::Description));
    // and plain names stay names
    assert_eq!(name("Pattern harvesting"), Some(Kind::Name));
    assert_eq!(name("Cache management"), Some(Kind::Name));
    assert_eq!(name("per-tier free memory"), Some(Kind::Name));
}
