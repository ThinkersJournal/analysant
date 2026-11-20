// SPDX-License-Identifier: MIT OR Apache-2.0
//! The shapes behind the seven validation rows, on small synthetic sources, so they run everywhere
//! (the reference tree is needed only for `gate_rows`). Each test names the row whose shape it holds.

use analysant_rust::{CodeIndex, Context, Reach, Relation, RustPack, split_ident_words};

fn index(files: &[(&str, &str)]) -> CodeIndex {
    let mut i = CodeIndex::new(RustPack::new());
    for (path, text) in files {
        i.add_source(path, text);
    }
    assert!(i.failures().is_empty(), "{:?}", i.failures());
    i
}

#[test]
fn identifiers_split_into_words() {
    assert_eq!(split_ident_words("pick_route"), ["pick", "route"]);
    assert_eq!(split_ident_words("RuleRegistry"), ["rule", "registry"]);
    assert_eq!(split_ident_words("LOOKAHEAD_K"), ["lookahead", "k"]);
    assert_eq!(
        split_ident_words("CouplingAdjustment"),
        ["coupling", "adjustment"]
    );
    assert_eq!(split_ident_words("Op"), ["op"]);
}

#[test]
fn row1_and_row7_a_name_nowhere_is_missing_with_no_mention() {
    let i = index(&[(
        "a/src/lib.rs",
        "pub fn other() {}\npub enum Op { Branch }\n",
    )]);
    let v = i.relate("CancellationToken");
    assert_eq!(v.relation, Relation::Missing);
    assert_eq!(
        v.mentions.code_production + v.mentions.code_test + v.mentions.comment,
        0
    );
    // A variant is found by its qualified name, and a missing one on the same enum is still missing.
    assert_eq!(i.relate("Op::Branch").relation, Relation::Implements);
    assert_eq!(i.relate("Op::Observe").relation, Relation::Missing);
    // The positive control of the nulls above: the same query path finds what is there.
    assert_eq!(i.relate("other").relation, Relation::Implements);
}

#[test]
fn row4_comments_alone_do_not_build_anything() {
    let src = "//! The PlanStore was deleted.\n// PlanStore memoization lives elsewhere now\npub fn f() {}\n/* PlanStore\n again */\nfn g() { let _s = \"PlanStore // not a comment\"; }\n";
    let i = index(&[("a/src/lib.rs", src)]);
    let v = i.relate("PlanStore");
    assert_eq!(v.relation, Relation::Missing);
    assert_eq!(
        v.mentions.comment, 3,
        "three comments, the string literal is not one"
    );
    assert_eq!(v.mentions.code_production, 0);
}

#[test]
fn row2_a_caller_that_nothing_calls_is_reported_not_built() {
    let src = "pub struct Cache;\nimpl Cache { pub fn resolve(&self) { pick_route() } }\npub fn pick_route() {}\n#[cfg(test)]\nmod tests { use super::*; #[test] fn t() { Cache.resolve(); } }\n";
    let i = index(&[("a/src/lib.rs", src)]);
    let v = i.relate("pick_route");
    assert_eq!(v.relation, Relation::Implements);
    match v.reach.unwrap() {
        Reach::OnlyFromUncalled { callers, .. } => {
            assert_eq!(callers[0].within.as_deref(), Some("Cache::resolve"));
        }
        other => panic!("{other:?}"),
    }
    // The control: a function called from `main` is plainly called.
    let j = index(&[("a/src/main.rs", "fn main() { go() }\nfn go() {}\n")]);
    assert!(matches!(j.relate("go").reach, Some(Reach::Called { .. })));
}

#[test]
fn row2_a_caller_whose_name_is_shared_is_reported_unsettled() {
    let src = "pub struct A;\npub struct B;\nimpl A { pub fn resolve(&self) { target() } }\nimpl B { pub fn resolve(&self) {} }\npub fn target() {}\npub fn run(b: B) { b.resolve() }\n";
    let i = index(&[("a/src/lib.rs", src)]);
    match i.relate("target").reach.unwrap() {
        Reach::CalledIfSharedNameResolves {
            sharing_definitions,
            ..
        } => {
            assert_eq!(sharing_definitions, 1);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn row3_a_chain_from_test_only_callers_is_dead_and_says_where() {
    let src = "pub fn default_rules() { gated() }\npub fn gated() {}\n#[cfg(test)]\nmod tests { use super::*; #[test] fn t() { default_rules(); } }\n";
    let i = index(&[("a/src/lib.rs", src)]);
    let d = i.relate("default_rules");
    assert_eq!(d.reach, Some(Reach::Uncalled { test_uses: 1 }));
    assert!(matches!(
        i.relate("gated").reach,
        Some(Reach::OnlyFromUncalled { .. })
    ));
}

#[test]
fn row5_a_field_is_used_only_by_a_dotted_read() {
    let src = "pub struct C { pub coupling: Vec<u8> }\nimpl C { pub fn new() -> C { C { coupling: Vec::new() } } }\n#[cfg(test)]\nmod tests { use super::*; #[test] fn t() { assert!(C::new().coupling.is_empty()); } }\n";
    let i = index(&[("a/src/lib.rs", src)]);
    let v = i.relate("coupling");
    assert_eq!(v.relation, Relation::Implements);
    assert_eq!(
        v.reach,
        Some(Reach::Uncalled { test_uses: 1 }),
        "an initializer is not a read"
    );
}

#[test]
fn cfg_is_evaluated_not_matched_as_text() {
    let src = "#[cfg(all(test, feature = \"x\"))]\nfn only_test() {}\n#[cfg(any(feature = \"x\", test))]\nfn maybe_prod() {}\n#[cfg(not(test))]\nfn prod() {}\n#[test]\nfn t() {}\n";
    let i = index(&[("a/src/lib.rs", src)]);
    let ctx = |n: &str| i.items().iter().find(|it| it.name == n).unwrap().context;
    assert_eq!(ctx("only_test"), Context::Test);
    assert_eq!(ctx("maybe_prod"), Context::Production);
    assert_eq!(ctx("prod"), Context::Production);
    assert_eq!(ctx("t"), Context::Test);
}

#[test]
fn test_paths_come_from_pack_data() {
    let p = RustPack::new();
    assert!(p.is_test_path("a/tests/x.rs"));
    assert!(p.is_test_path("a-model-fixture/src/lib.rs"));
    assert!(p.is_test_path("a-test-support/src/lib.rs"));
    assert!(!p.is_test_path("a/src/lib.rs"));
}

#[test]
fn a_file_that_does_not_parse_is_recorded_not_skipped() {
    let mut i = CodeIndex::new(RustPack::new());
    i.add_source("a/src/bad.rs", "fn ( {");
    assert_eq!(i.failures().len(), 1);
    assert_eq!(i.file_count(), 1);
}

#[test]
fn words_find_an_identifier_by_its_parts() {
    let i = index(&[(
        "a/src/lib.rs",
        "pub fn pick_route() {}\npub struct RuleRegistry;\n",
    )]);
    let words = |s: &str| s.split(' ').map(str::to_string).collect::<Vec<_>>();
    assert_eq!(
        i.relate_words(&words("pick route")).relation,
        Relation::Implements
    );
    assert_eq!(
        i.relate_words(&words("rule registry")).relation,
        Relation::Implements
    );
    assert_eq!(
        i.relate_words(&words("route pick")).relation,
        Relation::Missing
    );
}

#[test]
fn a_reexport_is_not_a_use() {
    let i = index(&[(
        "a/src/lib.rs",
        "pub const LOOKAHEAD_K: usize = 4;\npub use self::LOOKAHEAD_K as K2;\n",
    )]);
    assert_eq!(
        i.relate("LOOKAHEAD_K").reach,
        Some(Reach::Uncalled { test_uses: 0 })
    );
}
