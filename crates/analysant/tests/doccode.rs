// SPDX-License-Identifier: MIT OR Apache-2.0
//! The doc-vs-code lens (SPEC-STEP1 5.1, slice 5's omitted "claim-to-code wiring"), written before the
//! code. A document names code items in code spans; the lens asks the Rust pack's `CodeIndex` what each
//! one is (`relate`) and reports only what a reader should look at, with the scope of the reachability
//! claim in the sentence, what was searched, and a control that is a *different* claim.
//!
//! Every case is a synthetic document and a synthetic crate, so the expected output is stated here, not
//! derived by running the lens.

use analysant::doccode::{DocCodeRun, DocCodeStatus, LENS, doc_vs_code, spec};
use analysant::{Corpus, extract_corpus};
use analysant_core::{
    ClaimRef, ControlKind, ControlOutcome, Cost, English, Finding, Ledger, LensId, Markdown,
};
use analysant_rust::{CodeIndex, RustPack};

const LIB: &str = "\
pub fn build_cache() {}
pub fn only_in_tests() {}
pub fn dead_helper() {}
pub fn dead_caller() { dead_helper() }
pub struct A;
pub struct B;
impl A { pub fn resolve(&self) { target_fn() } }
impl B { pub fn resolve(&self) {} }
pub fn target_fn() {}
pub fn run(b: B) { b.resolve() }
#[cfg(test)]
mod tests {
    use super::*;
    fn helper_in_tests() {}
    #[test]
    fn t() { only_in_tests(); dead_caller(); helper_in_tests(); }
}
";
const MAIN: &str = "fn main() { build_cache(); }\n";

const DOC: &str = "\
# Design

The path is `build_cache`, `ghost_fn`, `only_in_tests`, `dead_helper`, `target_fn` and
`helper_in_tests`. The words cache and router are prose.
";

fn corpus(docs: &[(&str, &str)]) -> Corpus {
    let files: Vec<(String, String)> = docs
        .iter()
        .map(|(p, t)| ((*p).to_string(), (*t).to_string()))
        .collect();
    extract_corpus(&Markdown, &English::new(), &mut Ledger::default(), &files)
}

fn index(sources: &[(&str, &str)]) -> CodeIndex {
    let mut i = CodeIndex::new(RustPack::new());
    for (p, t) in sources {
        i.add_source(p, t);
    }
    i
}

fn fixture_index() -> CodeIndex {
    index(&[("demo/src/lib.rs", LIB), ("demo/src/main.rs", MAIN)])
}

fn found(run: &DocCodeRun) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = run
        .findings()
        .map(|(_, f)| (f.search.query.key.clone(), f.kind.label().to_string()))
        .collect();
    v.sort();
    v
}

fn claim_id(c: &Corpus, text: &str) -> ClaimRef {
    let (_, claim) = c
        .claims()
        .find(|(_, cl)| cl.text == text)
        .unwrap_or_else(|| panic!("no claim {text}"));
    ClaimRef::current(claim.unit.id)
}

fn finding<'a>(run: &'a DocCodeRun, query: &str) -> &'a Finding {
    run.findings()
        .map(|(_, f)| f)
        .find(|f| f.search.query.key == query)
        .unwrap_or_else(|| panic!("no finding for {query}"))
}

#[test]
fn each_reach_has_its_own_kind_and_a_called_name_has_none() {
    let c = corpus(&[("design.md", DOC)]);
    let run = doc_vs_code(&c, &fixture_index());
    let expect: Vec<(String, String)> = [
        ("dead_helper", "dead-callers"),
        ("ghost_fn", "absent"),
        ("helper_in_tests", "tests-only"),
        ("only_in_tests", "uncalled"),
        ("target_fn", "shared-name"),
    ]
    .iter()
    .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
    .collect();
    assert_eq!(found(&run), expect);
    // `build_cache` is called from `main`: resolved, no finding. Prose words are not named code.
    assert_eq!(run.named(), 6, "six code names, no prose word");
    assert_eq!(run.withheld(), 0);
    let resolved: Vec<&str> = run
        .outcomes
        .iter()
        .filter(|o| matches!(o.status, DocCodeStatus::Resolved))
        .map(|o| o.query.as_str())
        .collect();
    assert_eq!(resolved, ["build_cache"]);
}

#[test]
fn a_missing_name_carries_what_was_searched_and_a_different_claim_as_control() {
    let c = corpus(&[("design.md", DOC)]);
    let run = doc_vs_code(&c, &fixture_index());
    let f = finding(&run, "ghost_fn");
    assert_eq!(f.lens, LensId(LENS.to_string()));
    assert_eq!(f.kind.label(), "absent");
    assert_eq!(f.claims, vec![claim_id(&c, "ghost_fn")]);
    assert_eq!(
        f.search.population, 13,
        "items in the code index: 12 in lib.rs counting the two resolve methods and the test fn, 1 in main.rs"
    );
    assert_eq!(f.search.matched, 0, "the null as a number");
    assert!(f.search.looked_in.0.contains("2 Rust files"), "{f:?}");
    assert!(f.statement.contains("ghost_fn"), "{}", f.statement);
    assert!(
        f.statement.contains("by bare name") && f.statement.contains("no call graph"),
        "the scope of the claim is in the sentence: {}",
        f.statement
    );
    // The control is another claim of the same corpus, a present one, never the subject.
    assert_eq!(f.control.kind, ControlKind::KnownPresent);
    assert_eq!(f.control.outcome, ControlOutcome::Found);
    assert_eq!(f.control.item, claim_id(&c, "build_cache"));
    assert_ne!(f.control.item, f.claims[0]);
}

#[test]
fn a_function_used_only_by_tests_and_one_whose_name_is_shared_say_what_they_could_not_settle() {
    let c = corpus(&[("design.md", DOC)]);
    let run = doc_vs_code(&c, &fixture_index());
    let u = finding(&run, "only_in_tests");
    assert!(
        u.statement.contains("no production caller"),
        "{}",
        u.statement
    );
    assert!(u.statement.contains("1 test"), "{}", u.statement);
    let s = finding(&run, "target_fn");
    assert!(
        s.statement
            .contains("shares its name with 1 other definition"),
        "{}",
        s.statement
    );
    let t = finding(&run, "helper_in_tests");
    assert!(t.statement.contains("only in test code"), "{}", t.statement);
    let d = finding(&run, "dead_helper");
    assert!(
        d.statement.contains("dead_caller") || d.statement.contains("nothing calls"),
        "{}",
        d.statement
    );
}

#[test]
fn without_a_claim_that_resolves_nothing_is_reported_missing() {
    // Only the missing name is named: no control can be found, so the absence is withheld, not reported.
    let c = corpus(&[("design.md", "# D\n\nSee `ghost_fn` for details.\n")]);
    let run = doc_vs_code(&c, &fixture_index());
    assert_eq!(run.findings().count(), 0);
    assert_eq!(run.withheld(), 1);
    match &run.outcomes[0].status {
        DocCodeStatus::Withheld { reason } => {
            assert!(reason.contains("control"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
    // Control for the control: add a claim that does resolve, and the same absence is reported.
    let c2 = corpus(&[(
        "design.md",
        "# D\n\nSee `ghost_fn` and `build_cache` for details.\n",
    )]);
    let run2 = doc_vs_code(&c2, &fixture_index());
    assert_eq!(
        found(&run2),
        [("ghost_fn".to_string(), "absent".to_string())]
    );
}

#[test]
fn an_empty_code_index_withholds_everything_it_would_have_called_missing() {
    let c = corpus(&[("design.md", DOC)]);
    let empty = index(&[]);
    let run = doc_vs_code(&c, &empty);
    assert_eq!(run.findings().count(), 0);
    assert_eq!(run.withheld(), 6);
    match &run.outcomes[0].status {
        DocCodeStatus::Withheld { reason } => {
            assert!(reason.contains("no Rust items"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn paths_are_trimmed_to_what_the_code_defines_and_std_paths_are_not_ours() {
    let c = corpus(&[(
        "design.md",
        "# D\n\nUse `crate::build_cache`, `std::fs::read`, `A::resolve` and `B::nothing` here.\n",
    )]);
    let run = doc_vs_code(&c, &fixture_index());
    // `crate::build_cache` resolves; `A::resolve` is called only by `run`, which nothing calls;
    // `std::...` is not named code of this tree;
    // `B::nothing` is a member that does not exist.
    assert_eq!(
        found(&run),
        [
            ("A::resolve".to_string(), "dead-callers".to_string()),
            ("B::nothing".to_string(), "absent".to_string()),
        ]
    );
    assert_eq!(run.named(), 3);
}

#[test]
fn ids_are_stable_across_runs_and_differ_by_kind() {
    let c = corpus(&[("design.md", DOC)]);
    let a = doc_vs_code(&c, &fixture_index());
    let b = doc_vs_code(&c, &fixture_index());
    let ids =
        |r: &DocCodeRun| -> Vec<String> { r.findings().map(|(_, f)| f.id.0.clone()).collect() };
    assert_eq!(ids(&a), ids(&b));
    let mut sorted = ids(&a);
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 5, "five findings, five ids");
}

#[test]
fn the_lens_spec_declares_its_kinds_and_that_it_reads_code() {
    let s = spec();
    assert_eq!(s.id, LensId("doc-vs-code".to_string()));
    assert_eq!(s.cost, Cost::Cheap);
    assert!(s.reads_code);
    assert!(s.requires.is_empty());
    assert_eq!(
        s.kinds,
        [
            "absent",
            "tests-only",
            "uncalled",
            "dead-callers",
            "shared-name"
        ]
    );
}

#[test]
fn the_report_input_counts_what_was_examined_and_withheld() {
    let c = corpus(&[("design.md", DOC)]);
    let out = doc_vs_code(&c, &fixture_index()).output();
    assert_eq!(out.spec.id, LensId("doc-vs-code".to_string()));
    assert_eq!(out.findings.len(), 5);
    assert_eq!(out.population, 6);
    assert_eq!(out.withheld, 0);
}

#[test]
fn a_name_used_only_in_tests_with_no_definition_says_so_instead_of_naming_a_place() {
    let code = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() { phantom_fn(); }\n}\npub fn build_cache() {}\nfn main() { build_cache(); }\n";
    let idx = index(&[("demo/src/lib.rs", code)]);
    let c = corpus(&[("design.md", "# D\n\nSee `phantom_fn` and `build_cache`.\n")]);
    let run = doc_vs_code(&c, &idx);
    let f = finding(&run, "phantom_fn");
    assert_eq!(f.kind.label(), "tests-only");
    assert!(
        f.statement
            .contains("which no file defines and which is used only in test code"),
        "{}",
        f.statement
    );
    assert!(!f.statement.contains("(-)"), "{}", f.statement);
}
