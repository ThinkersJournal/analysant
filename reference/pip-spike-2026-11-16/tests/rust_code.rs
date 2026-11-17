//! Slice 6 tests: the Rust language pack — code as ground truth.
//!
//! Spec first, from the real cases in the Fuel tree at the pinned commit:
//! - `pick_route` is a production fn called from production code
//!   (`RouteCache::resolve`), so the lens calls it BUILT — while the audit
//!   rates it STUB (not called on the realize path; its only caller is
//!   itself never called). The disagreement is the point: the lens sees
//!   production-vs-test provenance, not reachability from the realize path.
//! - `RouteCache` / `LOOKAHEAD_K` / `default_rules` /
//!   `capability_gated_rules` are production definitions with no
//!   production use (only tests, re-exports, doc comments) — STUB,
//!   agreeing with the audit's predicate (non-test callers: 0).
//! - `PlanStore` exists only in comments — ABSENT, comments-only.
//! - `.coupling` is read in exactly one place, a test asserting it is
//!   empty — STUB (the audit's STUB), not TEST-ONLY: the field is defined
//!   in production; struct-literal `coupling: Vec::new()` initializations
//!   are not reads.
//! - `CancellationToken` / `cancel_token` / `Op::Observe` / `Op::Tap`:
//!   0 hits anywhere — ABSENT.
//! - The `Model` trait's three impls are all scaffolding: `LocalModel`
//!   in a `tests/` directory, `FixtureModel` and `UmbrellaModel` in
//!   crates ending in `-fixture` — all TEST-ONLY.
//! - "local judge baseline initialization" does NOT match the `Judge`
//!   type: a one-word code name cannot cover a four-word doc key.

use doc_analysis_spike::claim::{Claim, ClaimKind, PragmaticRole};
use doc_analysis_spike::rust_code::*;
use std::sync::OnceLock;

fn claim(kind: ClaimKind, key: &str, text: &str) -> Claim {
    Claim {
        id: "test.md:1".to_string(),
        key: key.to_string(),
        text: text.to_string(),
        source_file: "test.md".to_string(),
        line: 1,
        section: String::new(),
        kind,
        role: PragmaticRole::Name,
    }
}

fn snippet(path: &str, text: &str) -> CodeIndex {
    let mut index = CodeIndex::default();
    index.add_source(path, text);
    index
}

// --- identifier splitting ---

#[test]
fn split_snake_and_camel_and_screaming() {
    assert_eq!(split_ident_words("pick_route"), vec!["pick", "route"]);
    assert_eq!(split_ident_words("RuleRegistry"), vec!["rule", "registry"]);
    assert_eq!(split_ident_words("LOOKAHEAD_K"), vec!["lookahead", "k"]);
    assert_eq!(
        split_ident_words("CouplingAdjustment"),
        vec!["coupling", "adjustment"]
    );
    assert_eq!(split_ident_words("coupling"), vec!["coupling"]);
    assert_eq!(split_ident_words("Op"), vec!["op"]);
    assert_eq!(split_ident_words("fuel_graph"), vec!["fuel", "graph"]);
}

// --- comment stripping ---

#[test]
fn line_comments_do_not_create_mentions() {
    let index = snippet(
        "fuel-graph/src/opt.rs",
        "pub fn pick_route() {}\n// pick_route( is called only by tests\nlet x = 1; // pick_route( again\n",
    );
    // One production definition, zero production calls: the two comment
    // mentions must not count.
    let item = index
        .items
        .iter()
        .find(|i| i.name == "pick_route")
        .expect("definition recorded");
    assert!(!item.test_only);
    let (prod, test) = index.uses_of("pick_route", CodeItemKind::Fn);
    assert_eq!(prod, 0, "comment mentions are not uses");
    assert_eq!(test, 0);
}

#[test]
fn string_contents_do_not_create_mentions() {
    let index = snippet(
        "fuel-graph/src/opt.rs",
        "pub fn pick_route() {}\nlet s = \"pick_route( called\";\nlet t = \"// pick_route\";\n",
    );
    let (prod, _) = index.uses_of("pick_route", CodeItemKind::Fn);
    assert_eq!(prod, 0, "mentions inside string literals are not uses");
}

#[test]
fn block_comments_are_stripped_across_lines() {
    let index = snippet(
        "fuel-graph/src/opt.rs",
        "pub fn pick_route() {}\n/*\n  pick_route( in a block comment\n*/\nlet y = pick_route();\n",
    );
    let (prod, _) = index.uses_of("pick_route", CodeItemKind::Fn);
    assert_eq!(prod, 1, "the real call after the block counts");
}

// --- definitions ---

#[test]
fn public_items_are_recorded_by_kind() {
    let index = snippet(
        "fuel-graph/src/opt.rs",
        "pub struct RuleRegistry { rules: Vec<Rule> }\npub enum Op { Branch { x: u8 } }\npub trait Rule { fn name(&self) -> &str; }\npub type NodeMap = HashMap<u32, u32>;\npub const LOOKAHEAD_K: usize = 3;\npub static COUNT: usize = 0;\n",
    );
    let names: Vec<(&str, CodeItemKind)> = index
        .items
        .iter()
        .map(|i| (i.name.as_str(), i.kind))
        .collect();
    assert!(names.contains(&("RuleRegistry", CodeItemKind::Struct)));
    assert!(names.contains(&("Op", CodeItemKind::Enum)));
    assert!(names.contains(&("Rule", CodeItemKind::Trait)));
    assert!(names.contains(&("NodeMap", CodeItemKind::Type)));
    assert!(names.contains(&("LOOKAHEAD_K", CodeItemKind::Const)));
    assert!(names.contains(&("COUNT", CodeItemKind::Static)));
}

#[test]
fn enum_variants_and_struct_fields_are_recorded() {
    let index = snippet(
        "fuel-graph/src/lib.rs",
        "pub enum Op {\n    Branch { reconverge_at: NodeId },\n    Scan,\n}\npub struct Candidate {\n    pub coupling: Vec<u8>,\n    name: String,\n}\n",
    );
    let variant = index
        .items
        .iter()
        .find(|i| i.name == "Branch" && i.kind == CodeItemKind::Variant)
        .expect("Branch variant recorded");
    assert_eq!(variant.qualified.as_deref(), Some("Op"));
    let field = index
        .items
        .iter()
        .find(|i| i.name == "coupling" && i.kind == CodeItemKind::Field)
        .expect("coupling field recorded");
    assert_eq!(field.qualified.as_deref(), Some("Candidate"));
    // The private `name` field is not a public item.
    assert!(!index.items.iter().any(|i| i.name == "name"));
}

#[test]
fn impl_blocks_record_the_implementing_type_and_trait() {
    let index = snippet(
        "fuel-model-core/src/lib.rs",
        "pub trait Model {}\nimpl Model for LocalModel {}\nimpl Rule for FusionRule {}\nimpl RuleRegistry {}\n",
    );
    let model_impl = index
        .items
        .iter()
        .find(|i| i.kind == CodeItemKind::Impl && i.name == "LocalModel")
        .expect("impl recorded");
    assert_eq!(model_impl.qualified.as_deref(), Some("Model"));
    let rule_impl = index
        .items
        .iter()
        .find(|i| i.kind == CodeItemKind::Impl && i.name == "FusionRule")
        .expect("impl recorded");
    assert_eq!(rule_impl.qualified.as_deref(), Some("Rule"));
    let inherent = index
        .items
        .iter()
        .find(|i| i.kind == CodeItemKind::Impl && i.name == "RuleRegistry")
        .expect("inherent impl recorded");
    assert_eq!(inherent.qualified, None);
}

#[test]
fn methods_inside_impl_blocks_are_qualified() {
    let index = snippet(
        "fuel-dispatch/src/ranker/route_picker.rs",
        "pub struct RouteCache {}\nimpl RouteCache {\n    pub fn resolve(&mut self) {}\n    fn helper(&self) {}\n}\n",
    );
    let resolve = index
        .items
        .iter()
        .find(|i| i.name == "resolve")
        .expect("method recorded");
    assert_eq!(resolve.qualified.as_deref(), Some("RouteCache"));
    // A plain fn inside an impl block is a method, recorded qualified.
    let helper = index
        .items
        .iter()
        .find(|i| i.name == "helper")
        .expect("private method recorded");
    assert_eq!(helper.qualified.as_deref(), Some("RouteCache"));
}

// --- provenance ---

#[test]
fn provenance_tests_dir_cfg_test_and_fixture_crates() {
    let in_tests = snippet(
        "fuel-model-core/tests/registry.rs",
        "impl Model for LocalModel {}\n",
    );
    assert!(in_tests.items.iter().all(|i| i.test_only));

    let in_cfg_test = snippet(
        "fuel-graph/src/opt.rs",
        "#[cfg(test)]\nmod tests {\n    fn helper() {}\n    pub fn also_helper() {}\n}\n",
    );
    assert!(
        !in_cfg_test.items.is_empty(),
        "items inside cfg(test) are recorded"
    );
    assert!(in_cfg_test.items.iter().all(|i| i.test_only));

    let in_fixture = snippet(
        "fuel-model-fixture/src/lib.rs",
        "impl Model for FixtureModel {}\n",
    );
    assert!(in_fixture.items.iter().all(|i| i.test_only));

    let in_fixtures_plural = snippet(
        "fuel-correctness-fixtures/src/capture.rs",
        "pub fn capture_one() {}\n",
    );
    assert!(in_fixtures_plural.items.iter().all(|i| i.test_only));

    let in_test_support = snippet("fuel-test-support/src/lib.rs", "pub fn dev_or_skip() {}\n");
    assert!(in_test_support.items.iter().all(|i| i.test_only));

    let production = snippet("fuel-graph/src/opt.rs", "pub fn lower_to_base_map() {}\n");
    assert!(production.items.iter().all(|i| !i.test_only));
}

#[test]
fn fixture_suffixed_name_is_test_only_even_in_a_plain_file() {
    // A name ending in `Fixture` is scaffolding wherever it is
    // defined. (The real `FixtureModel` is caught by the crate rule,
    // not this one — it ends in `Model`, not `Fixture`.)
    let index = snippet("fuel-graph/src/opt.rs", "pub struct GraphFixture {}\n");
    assert!(index.items.iter().all(|i| i.test_only));
}

#[test]
fn test_attribute_marks_the_fn_even_outside_a_test_module() {
    let index = snippet("fuel-graph/src/opt.rs", "#[test]\nfn my_test() {}\n");
    let item = index
        .items
        .iter()
        .find(|i| i.name == "my_test")
        .expect("test fn recorded");
    assert!(item.test_only);
    assert_eq!(item.kind, CodeItemKind::Fn);
}

// --- mentions / uses ---

#[test]
fn re_export_lines_do_not_count_as_uses() {
    let index = snippet(
        "fuel-dispatch/src/ranker/mod.rs",
        "pub use route_picker::{LOOKAHEAD_K, RouteCache, pick_route};\n",
    );
    assert_eq!(index.uses_of("LOOKAHEAD_K", CodeItemKind::Const), (0, 0));
    assert_eq!(index.uses_of("RouteCache", CodeItemKind::Struct), (0, 0));
    assert_eq!(index.uses_of("pick_route", CodeItemKind::Fn), (0, 0));
}

#[test]
fn definition_lines_do_not_count_as_uses() {
    let index = snippet("fuel-graph/src/opt.rs", "pub fn pick_route() {}\n");
    assert_eq!(index.uses_of("pick_route", CodeItemKind::Fn), (0, 0));
}

#[test]
fn struct_literal_inits_are_not_field_reads() {
    // Mirrors the real `.coupling` case: the field is constructed with
    // `coupling: Vec::new()` in production code, but only *read* by a
    // test asserting it is empty. Inits must not count as reads.
    let index = snippet(
        "fuel-dispatch/src/ranker/candidate.rs",
        "pub struct Candidate { pub coupling: Vec<u8> }\nfn build() -> Candidate { Candidate { coupling: Vec::new() } }\n",
    );
    let (prod, test) = index.uses_of("coupling", CodeItemKind::Field);
    assert_eq!(prod, 0, "struct-literal init is not a field read");
    assert_eq!(test, 0);
    let index2 = snippet(
        "fuel-dispatch/src/ranker/candidate.rs",
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn empty() { assert!(c.coupling.is_empty()); }\n}\n",
    );
    let (prod2, test2) = index2.uses_of("coupling", CodeItemKind::Field);
    assert_eq!(prod2, 0);
    assert_eq!(test2, 1, "the test assert is a field read");
}

#[test]
fn use_statements_do_not_count_as_fn_calls() {
    // `use crate::ranker::pick_route;` imports the name; only
    // `pick_route(` calls it. The audit's grep was for `pick_route(`.
    let index = snippet(
        "fuel-dispatch/src/pipelined.rs",
        "use crate::ranker::pick_route;\n",
    );
    assert_eq!(index.uses_of("pick_route", CodeItemKind::Fn), (0, 0));
}

// --- matching ---

#[test]
fn matching_name_like_keys_to_code_names() {
    let mut index = CodeIndex::default();
    index.add_source(
        "fuel-graph/src/opt.rs",
        "pub struct RuleRegistry {}\npub fn optimize_to_fixpoint() {}\n",
    );
    index.add_source(
        "fuel-graph/src/registry.rs",
        "pub struct ToleranceBudget {}\n",
    );
    // "rule registry" covers RuleRegistry exactly.
    let v = doc_code_status(
        &claim(ClaimKind::Named, "rule registry", "Rule Registry"),
        &index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Stub, "defined, never used");
    // "tolerance budget" covers ToleranceBudget exactly.
    let v = doc_code_status(
        &claim(ClaimKind::Named, "tolerance budget", "Tolerance budget"),
        &index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Stub);
}

#[test]
fn one_word_code_name_cannot_cover_a_compound_doc_key() {
    // The Judge type exists; the bundled Judge *baseline* does not.
    // A single-word code name must not satisfy a multi-word doc key.
    let mut index = CodeIndex::default();
    index.add_source(
        "fuel-dispatch/src/judge.rs",
        "pub struct Judge {}\nimpl Judge { pub fn run(&self) {} }\n",
    );
    let v = doc_code_status(
        &claim(
            ClaimKind::Named,
            "local judge baseline initialization",
            "Local Judge baseline initialization.",
        ),
        &index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Absent);
}

#[test]
fn prose_key_does_not_match_an_unrelated_helper() {
    // `harvest_grads` is gradient harvesting in training, not pattern
    // harvesting. Its second word is not in the doc key.
    let mut index = CodeIndex::default();
    index.add_source(
        "fuel-training/src/training_loop.rs",
        "fn harvest_grads() {}\n",
    );
    let v = doc_code_status(
        &claim(
            ClaimKind::Directive,
            "pattern harvesting against canonical form",
            "Pattern harvesting against canonical form",
        ),
        &index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Absent);
}

#[test]
fn identifier_exact_match_prefers_the_qualified_item() {
    let mut index = CodeIndex::default();
    index.add_source(
        "fuel-graph/src/lib.rs",
        "pub enum Op { Branch { x: u8 }, Scan }\n",
    );
    let v = doc_code_status(
        &claim(ClaimKind::Identifier, "op branch", "Op::Branch"),
        &index,
        &[],
    );
    assert_eq!(
        v.status,
        CodeStatus::Stub,
        "enum defined, variant never used"
    );
    assert_eq!(v.matched.as_ref().map(|m| m.name.as_str()), Some("Branch"));
    assert_eq!(
        v.matched.as_ref().and_then(|m| m.qualified.as_deref()),
        Some("Op")
    );
}

// --- statuses ---

#[test]
fn status_built_stub_test_only_absent() {
    // Built: production definition + production call.
    let mut index = CodeIndex::default();
    index.add_source(
        "fuel-graph/src/opt.rs",
        "pub fn lower_to_base_map() {}\nfn caller() { lower_to_base_map(); }\n",
    );
    let v = doc_code_status(
        &claim(ClaimKind::Named, "lower base map", "lower to base map"),
        &index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Built);

    // Test-only: definition only in a test module.
    let mut index = CodeIndex::default();
    index.add_source(
        "fuel-graph/src/opt.rs",
        "#[cfg(test)]\nmod tests {\n    pub fn test_only_helper() {}\n}\n",
    );
    let v = doc_code_status(
        &claim(ClaimKind::Named, "test only helper", "test only helper"),
        &index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::TestOnly);

    // Absent: no definition, no mentions at all.
    let v = doc_code_status(
        &claim(ClaimKind::Named, "cancellation token", "CancellationToken"),
        &index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Absent);
    assert!(!v.comments_only);
}

#[test]
fn comments_only_is_absent_with_the_note() {
    let mut index = CodeIndex::default();
    index.add_source(
        "fuel-dispatch/src/optimize.rs",
        "//! `PlanStore` and the legacy route-picking dispatch\n//! the ONLY path — the legacy `PlanStore` route-picking\n",
    );
    let v = doc_code_status(
        &claim(ClaimKind::Named, "plan store", "PlanStore"),
        &index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Absent);
    assert!(v.comments_only, "PlanStore is mentioned only in comments");
}

// --- integration: the real tree at the pinned commit ---

fn real_index() -> &'static CodeIndex {
    static INDEX: OnceLock<CodeIndex> = OnceLock::new();
    INDEX.get_or_init(|| {
        let root = std::path::Path::new("corpus/fuel/code");
        assert!(
            root.is_dir(),
            "corpus/fuel/code must exist — run the fetch first"
        );
        scan_code_dir(root)
    })
}

#[test]
fn real_cancellation_has_zero_hits() {
    let index = real_index();
    assert_eq!(index.definitions_of("CancellationToken").len(), 0);
    assert_eq!(index.definitions_of("cancel_token").len(), 0);
    assert_eq!(
        index.uses_of("CancellationToken", CodeItemKind::Struct),
        (0, 0)
    );
    assert_eq!(index.uses_of("cancel_token", CodeItemKind::Fn), (0, 0));
}

#[test]
fn real_observe_and_tap_have_zero_hits() {
    let index = real_index();
    // Op::Observe / Op::Tap: no variants, no mentions.
    assert!(!index.items.iter().any(|i| i.name == "Observe"));
    assert!(!index.items.iter().any(|i| i.name == "Tap"));
}

#[test]
fn real_plan_store_is_comments_only() {
    let index = real_index();
    assert_eq!(index.definitions_of("PlanStore").len(), 0);
    let v = doc_code_status(
        &claim(ClaimKind::Named, "plan store", "PlanStore"),
        index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Absent);
    assert!(v.comments_only);
}

#[test]
fn real_coupling_is_a_stub() {
    let index = real_index();
    // The field is defined in production; the only `.coupling` read is
    // the test asserting it is empty.
    let defs = index.definitions_of("coupling");
    assert!(!defs.is_empty());
    assert!(defs.iter().any(|d| !d.test_only));
    let (prod, test) = index.uses_of("coupling", CodeItemKind::Field);
    assert_eq!(prod, 0, "no production read of .coupling");
    assert_eq!(test, 1, "exactly the test assert");
    let v = doc_code_status(
        &claim(
            ClaimKind::Named,
            "conditional cost adjustments",
            "conditional cost adjustments",
        ),
        index,
        &[],
    );
    // The doc key is the mechanism, not the word "coupling": no code
    // item's name covers it, so the finding stays ABSENT in code — the
    // audit's STUB is about the `.coupling` field, checked above.
    assert_eq!(v.status, CodeStatus::Absent);
}

#[test]
fn real_route_cache_and_lookahead_k_are_stubs() {
    let index = real_index();
    let v = doc_code_status(
        &claim(ClaimKind::Named, "route cache", "RouteCache"),
        index,
        &[],
    );
    assert_eq!(
        v.status,
        CodeStatus::Stub,
        "defined, resolve never called outside tests"
    );
    let v = doc_code_status(
        &claim(ClaimKind::Named, "lookahead k", "LOOKAHEAD_K"),
        index,
        &[],
    );
    assert_eq!(
        v.status,
        CodeStatus::Stub,
        "defined and re-exported, unused"
    );
}

#[test]
fn real_default_rules_is_a_stub() {
    let index = real_index();
    let v = doc_code_status(
        &claim(ClaimKind::Named, "default rules", "default_rules"),
        index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Stub, "all callers are test modules");
}

#[test]
fn real_capability_gated_rules_is_built_by_the_lens_rule() {
    // The lens calls this BUILT: production definition + a production
    // call site at fuel-graph/src/opt.rs:168, inside `default_rules`
    // itself. The audit's literal predicate ("non-test callers: 0")
    // is FALSE for this item — there is one non-test caller — but its
    // substance holds: that caller is `default_rules`, which is
    // itself only called from tests, so the chain is dead on the
    // realize path. Same class of disagreement as pick_route and
    // optimize_to_fixpoint: the lens measures production-reachability,
    // the audit measures reachability from the realize path.
    let index = real_index();
    let v = doc_code_status(
        &claim(
            ClaimKind::Named,
            "capability gated rules",
            "capability_gated_rules",
        ),
        index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Built);
    assert_eq!(
        v.matched.as_ref().map(|m| (m.file.as_str(), m.line)),
        Some(("fuel-graph/src/opt.rs", 183))
    );
}

#[test]
fn real_pick_route_is_built_by_the_lens_rule() {
    // The lens calls this BUILT: production definition + a production
    // call site inside `RouteCache::resolve`. The audit rates it STUB
    // because `resolve` is itself never called — dead production code.
    // This test pins the lens's verdict AND the disagreement; the report
    // sides with the audit (reachability beats provenance).
    let index = real_index();
    let v = doc_code_status(
        &claim(ClaimKind::Identifier, "pick route", "pick_route"),
        index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Built);
}

#[test]
fn real_optimize_to_fixpoint_is_built_by_the_lens_rule() {
    // Production definition + a production call site inside
    // `lower_to_base_map` (JIT paths, not realize). The audit groups it
    // under "ABSENT on realize"; the lens's BUILT means
    // production-reachable, a weaker claim — the report says so.
    let index = real_index();
    let v = doc_code_status(
        &claim(
            ClaimKind::Identifier,
            "optimize to fixpoint",
            "optimize_to_fixpoint",
        ),
        index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Built);
}

#[test]
fn real_model_impls_are_all_test_only() {
    let index = real_index();
    let impls: Vec<&CodeItem> = index
        .items
        .iter()
        .filter(|i| i.kind == CodeItemKind::Impl && i.qualified.as_deref() == Some("Model"))
        .collect();
    assert_eq!(impls.len(), 3, "LocalModel, FixtureModel, UmbrellaModel");
    assert!(
        impls.iter().all(|i| i.test_only),
        "every Model impl is test/fixture scaffolding"
    );
    let names: Vec<&str> = impls.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"LocalModel"));
    assert!(names.contains(&"FixtureModel"));
    assert!(names.contains(&"UmbrellaModel"));
}

#[test]
fn real_populate_dispatch_table_is_defined_but_never_called() {
    let index = real_index();
    let defs = index.definitions_of("populate_dispatch_table");
    assert_eq!(defs.len(), 1, "one production definition");
    assert!(!defs[0].test_only);
    let (prod, test) = index.uses_of("populate_dispatch_table", CodeItemKind::Fn);
    assert_eq!(prod, 0, "zero callers");
    assert_eq!(test, 0);
}

#[test]
fn real_judge_baseline_is_absent_in_code() {
    let index = real_index();
    let v = doc_code_status(
        &claim(
            ClaimKind::Named,
            "local judge baseline initialization",
            "Local Judge baseline initialization.",
        ),
        index,
        &[],
    );
    assert_eq!(v.status, CodeStatus::Absent);
}
