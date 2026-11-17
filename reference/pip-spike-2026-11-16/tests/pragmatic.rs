//! Tests for slice 1: pragmatic classification.
//!
//! The spike's precision bottleneck is extraction, not matching: most
//! "absent directives" are crate descriptions and most "absent named
//! capabilities" are status notes and rules. The pragmatic lens tells
//! what the text is doing — directive vs description vs status note vs
//! naming — using deterministic lexicons and structure only. No ML.
//! Written against the spec before the implementation.

use doc_analysis_spike::claim::{ClaimKind, PragmaticRole};
use doc_analysis_spike::parse::extract_claims;
use doc_analysis_spike::pragmatic::{classify_list_item, classify_name, is_reportable};

#[test]
fn imperative_list_item_is_a_directive() {
    assert_eq!(
        classify_list_item("build the multi-path DAG", "The optimized form"),
        PragmaticRole::Directive
    );
    assert_eq!(
        classify_list_item("Add a tolerance budget per op", "Tolerance metadata"),
        PragmaticRole::Directive
    );
}

#[test]
fn modal_list_item_is_a_directive() {
    assert_eq!(
        classify_list_item(
            "the runtime must support multi-path DAG",
            "The optimized form"
        ),
        PragmaticRole::Directive
    );
    assert_eq!(
        classify_list_item(
            "Realize should restart re-optimization from the base map",
            "The base map"
        ),
        PragmaticRole::Directive
    );
}

#[test]
fn crate_description_list_item_is_not_a_directive() {
    assert_eq!(
        classify_list_item(
            "**fuel-core-types**: dtypes, shapes, layouts, errors, the dispatch-key types.",
            "Crate boundaries inside Foundation"
        ),
        PragmaticRole::Description
    );
    assert_eq!(
        classify_list_item(
            "**fuel-dispatch**: the binding-table catalog, `KernelRef` ABI, dispatch wrappers.",
            "Crate boundaries inside Foundation"
        ),
        PragmaticRole::Description
    );
}

#[test]
fn bold_term_plus_gloss_is_a_description() {
    assert_eq!(
        classify_list_item(
            "**Side-table metadata.** Information derivable from the graph lives in side tables.",
            "The DAG"
        ),
        PragmaticRole::Description
    );
}

#[test]
fn status_note_list_item_is_a_status_note() {
    assert_eq!(
        classify_list_item(
            "As-built status (2026-07-29).",
            "Crate boundaries inside Foundation"
        ),
        PragmaticRole::StatusNote
    );
    assert_eq!(
        classify_list_item(
            "**PlanStore** is likewise gone and was listed here as current until 2026-08-28.",
            "Where this lives in code"
        ),
        PragmaticRole::StatusNote
    );
    assert_eq!(
        classify_list_item(
            "The former fuel-graph-router and fuel-graph-executor are retired.",
            "Where this lives in code"
        ),
        PragmaticRole::StatusNote
    );
}

#[test]
fn rule_statement_is_a_description_not_a_directive() {
    assert_eq!(
        classify_list_item(
            "Rule 1: dependencies flow downward only.",
            "Two architectural rules that keep the layering clean"
        ),
        PragmaticRole::Description
    );
}

#[test]
fn cross_reference_list_item_is_a_description() {
    assert_eq!(
        classify_list_item(
            "[04-optimization](04-optimization.md) — produces the multi-path graph.",
            "See also"
        ),
        PragmaticRole::Description
    );
}

#[test]
fn substring_lookalikes_do_not_fire() {
    // "shall" is a prefix of "shallow"; "gone" sits inside "undergone".
    // Lexicon lookups are word-boundary matches, so neither fires.
    assert_eq!(
        classify_list_item(
            "**Shallow (just-in-time)**: commit the next run only as a slot frees.",
            "Dispatch: runs, not single ops"
        ),
        PragmaticRole::Description
    );
    assert_ne!(
        classify_list_item("The design has undergone review.", "The optimized form"),
        PragmaticRole::StatusNote
    );
}

#[test]
fn parenthetical_gloss_after_bold_term_is_a_description() {
    assert_eq!(
        classify_list_item(
            "**Shared** (weights/constants — `Op::Const`): one storage for the whole model.",
            "Storage classes and sessions"
        ),
        PragmaticRole::Description
    );
    assert_eq!(
        classify_list_item(
            "**Strategic decisions** (placement, fusion, kernel-variant choice) — the optimizer's.",
            "The runtime's responsibilities"
        ),
        PragmaticRole::Description
    );
    // But a `(see ...)` parenthetical is a cross-reference, not a gloss.
    assert_eq!(
        classify_list_item(
            "**Pattern harvesting against canonical form** (see [08-pattern-harvest](08-pattern-harvest.md))",
            "The base map"
        ),
        PragmaticRole::Directive
    );
}

#[test]
fn copula_opening_is_a_description() {
    assert_eq!(
        classify_list_item(
            "The **user-facing form** is what Tensor ops or an import produce.",
            "How the user-facing form, base map, and optimized graph relate"
        ),
        PragmaticRole::Description
    );
    assert_eq!(
        classify_list_item(
            "Each ready run is dispatched to an available slot on its assigned backend.",
            "Data parallelism: independent runs run concurrently"
        ),
        PragmaticRole::Description
    );
    // Modal still wins over the copula opening.
    assert_eq!(
        classify_list_item(
            "The runtime must support multi-path DAG",
            "The optimized form"
        ),
        PragmaticRole::Directive
    );
}

#[test]
fn bare_capability_fragment_stays_a_directive() {
    // A list item that names a capability with no descriptive signal is
    // presumed directed: the architecture lists it as part of the design.
    // (Recall is bought here; precision comes from the positive signals.)
    assert_eq!(
        classify_list_item(
            "**Pattern harvesting against canonical form** (see [08-pattern-harvest](08-pattern-harvest.md))",
            "The base map"
        ),
        PragmaticRole::Directive
    );
}

#[test]
fn capability_name_stays_a_name() {
    assert_eq!(
        classify_name("multi-path graph", "The optimized form"),
        PragmaticRole::Name
    );
    assert_eq!(
        classify_name("lookahead window", "Dispatch: runs, not single ops"),
        PragmaticRole::Name
    );
    assert_eq!(
        classify_name(
            "The persisted graph / optimization cache",
            "What the runtime persists"
        ),
        PragmaticRole::Name
    );
}

#[test]
fn version_change_note_is_a_status_note() {
    assert_eq!(
        classify_name(
            "v0.6 was a MAJOR core-claim change",
            "Intermediate representation"
        ),
        PragmaticRole::StatusNote
    );
    assert_eq!(
        classify_name(
            "As-built status (2026-07-29)",
            "Crate boundaries inside Foundation"
        ),
        PragmaticRole::StatusNote
    );
}

#[test]
fn sentence_bold_phrase_is_a_description_not_a_name() {
    assert_eq!(
        classify_name(
            "cannot become a new primitive at runtime",
            "The op vocabulary"
        ),
        PragmaticRole::Description
    );
    assert_eq!(
        classify_name(
            "That is no longer true and was misleading",
            "Crate boundaries inside Foundation"
        ),
        PragmaticRole::StatusNote
    );
    assert_eq!(
        classify_name(
            "Rule 1: dependencies flow downward only",
            "Two architectural rules that keep the layering clean"
        ),
        PragmaticRole::Description
    );
}

#[test]
fn temporal_fragment_is_a_description_not_a_name() {
    assert_eq!(
        classify_name("before finalize-to-disk", "The DAG"),
        PragmaticRole::Description
    );
    assert_eq!(
        classify_name("between optimization rounds", "The DAG"),
        PragmaticRole::Description
    );
}

#[test]
fn reportable_keeps_directives_and_names_drops_descriptions() {
    let doc = "# The optimized form\n\n- build the multi-path DAG\n- **fuel-core-types**: dtypes, shapes, layouts.\n\nThe **multi-path graph** is the plan. **As-built status (2026-07-29).**\n";
    let claims = extract_claims("03-ir.md", doc);
    let reportable: Vec<_> = claims.iter().filter(|c| is_reportable(c)).collect();
    // The directive and the capability name survive, and so does the
    // crate name itself (a genuine component name, extracted from the
    // bold span inside the description item). The description item and
    // the status note are set aside.
    assert_eq!(reportable.len(), 3);
    assert!(reportable.iter().any(|c| c.key == "build multi path dag"));
    assert!(reportable.iter().any(|c| c.key == "multi path graph"));
    assert!(reportable.iter().any(|c| c.key == "fuel core types"));
    assert!(claims
        .iter()
        .any(|c| c.kind == ClaimKind::Directive && c.role == PragmaticRole::Description));
    assert!(claims
        .iter()
        .any(|c| c.kind == ClaimKind::Named && c.role == PragmaticRole::StatusNote));
}
