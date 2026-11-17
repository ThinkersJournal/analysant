//! Slice 4 (genre handling) tests, written against the spec derived
//! from the v6 run before the implementation.
//!
//! Two mechanisms:
//!
//! 1. The decision-outcome filter: a claim recording a rejected /
//!    superseded / withdrawn / declined outcome is a status note about
//!    a decision, not a directive. The marker is the past participle
//!    as a standalone word in the prose — not a hyphenated compound
//!    ("hard-rejected" is validation behaviour), not a backticked code
//!    token (`` `Rejected` `` is an enum variant), and not the bare
//!    verb ("verify→adopt/reject" is a process with a reject step).
//!    A marker in the section header alone does NOT set an item aside:
//!    both outcome-sections in the log contain live design (the
//!    derived-weights variant, the invariance-declaration replacement),
//!    so the section is not a verdict on everything under it.
//!
//! 2. Note-to-row ownership: a design note is owned by a ROADMAP row
//!    when its title names the row ("ROADMAP row 157") and the row
//!    exists, or when a row's text cites the note's path
//!    (`design-notes/<file>`). The citing row is the nearest preceding
//!    `N. **` row-start line — rows can span several lines, and the
//!    ROADMAP also contains numbered sub-lists ("1. Add ...") that are
//!    not rows, so the bold marker is part of the pattern.

use doc_analysis_spike::claim::PragmaticRole;
use doc_analysis_spike::ownership::{first_heading, owner_for};
use doc_analysis_spike::pragmatic::{classify_list_item, classify_name};

const LOG_SECTION_0509: &str = "Decisions log > 2026-05-09 — Architecture set v1.0 established";
const LOG_SECTION_REJECTED_ARTIFACT: &str =
    "Decisions log > 2026-06-08 — Runtime-snapshot persistence artifact (L3); save-all-activations rejected";
const LOG_SECTION_INGESTION: &str =
    "Decisions log > 2026-07-14 — Candidate-kernel ingestion service (Spec B): verify→adopt/reject + bounded idle-aware queue";

// --- Mechanism 1: the decision-outcome filter ---

#[test]
fn rejected_option_is_a_status_note() {
    // Real text, decisions-log:127.
    let item = "*Continue piecemeal phase work without consolidation.* Audit identified compounding inconsistencies; rejected.";
    assert_eq!(
        classify_list_item(item, LOG_SECTION_0509),
        PragmaticRole::StatusNote
    );
}

#[test]
fn rejected_uppercase_is_a_status_note() {
    // Real text, decisions-log:1483.
    let item = "**A `decline_pending!` macro** — REJECTED. A macro can record a reason but cannot observe the capability arriving, so it produces better *documentation* of a hazard. Documentation is not a control; this is the same finding as the runtime-skip program (GAP-157), where a skip that declared itself in prose was invisible.";
    assert_eq!(
        classify_list_item(item, "Decisions log > 2026-08-12 — A decline carries TWO axes: what KIND of token it is, and WHY the capability is absent. They compose as two fields and never merge."),
        PragmaticRole::StatusNote
    );
}

#[test]
fn hyphenated_rejected_is_validation_behaviour_not_an_outcome() {
    // Real text, decisions-log:746. "hard-rejected when nonzero" is what
    // validate() does to bad input, not a decision to reject something.
    let item = "**`SeamHello.reserved1`** — the 6 alignment-padding bytes between `profiles` (offset 42) and `capabilities` (offset 48) were implicit `#[repr(C)]` padding Rust neither guarantees zeroed nor lets a reader inspect. Made explicit as a managed `reserved1: [u8; 6]` field: zeroed by `advertise()`, hard-rejected when nonzero by `validate()` (new `SeamError::ReservedNonZero`), offset asserted in the frozen-layout test. Layout-preserving (still 56 bytes).";
    assert_eq!(
        classify_list_item(
            item,
            "Decisions log > 2026-07-14 — KISS conformance pass: four seam/verify latent-bug fixes + adopt-from-KISS goals"
        ),
        PragmaticRole::Directive
    );
}

#[test]
fn backticked_rejected_is_code_not_an_outcome() {
    // Real text, decisions-log:768. `Rejected` is an enum variant the
    // ingestion service returns, not a decision recorded in prose.
    let item = "`ingest_one` → `adopt_runtime_fused` on Pass (never `.expect()` — a Pass with no decompose or a registration failure is `Rejected`, honoring never-panic) / `RejectionReport` + `ProviderFeedback::on_rejected` on Fail.";
    assert_eq!(
        classify_list_item(item, LOG_SECTION_INGESTION),
        PragmaticRole::Directive
    );
}

#[test]
fn bare_verb_reject_is_a_process_step_not_an_outcome() {
    // Real text, decisions-log:769. "verify→adopt/reject" names the
    // service's pipeline; the service itself is live work, and the
    // bare verb "reject" is not the outcome participle "rejected".
    let item = "`IngestionService` — a bounded `sync_channel` + one idle-aware serial worker (defers to GPU-idle via `inflight_count`), so ingestion never swamps live inference; backpressure notifies the provider; the worker survives a panicking verify AND a panicking provider callback.";
    assert_eq!(
        classify_list_item(item, LOG_SECTION_INGESTION),
        PragmaticRole::Directive
    );
}

#[test]
fn outcome_word_in_section_alone_does_not_set_aside() {
    // Real text, decisions-log:245. The section header carries the
    // rejected option ("save-all-activations rejected"), but this item
    // is live design under it — the derived-weights variant. A section
    // header is not a verdict on everything under it.
    let item = "The genuinely launch-relevant precompute case (input-independent derived values not already constant-folded — e.g. dequantized weights) is an optional **derived-weights** variant of the *model* artifact, not an activation snapshot.";
    assert_eq!(
        classify_list_item(item, LOG_SECTION_REJECTED_ARTIFACT),
        PragmaticRole::Directive
    );
}

#[test]
fn judge_baseline_survives_the_outcome_filter() {
    // Real text, decisions-log:115. The guard case: the outcome filter
    // operates on the log, and this audit-ABSENT item lives in the log.
    // The list-item twin is a definition head (set aside as a
    // description, as in v6); the bold-span name is the finding.
    let item = "**Local Judge baseline initialization.** The Judge can optionally start from a community-aggregated profile for the user's hardware fingerprint before refining with local measurements. (06-runtime.)";
    assert_eq!(
        classify_list_item(
            item,
            "Decisions log > 2026-05-09 — Architecture set v1.0 established"
        ),
        PragmaticRole::Description
    );
    assert_eq!(
        classify_name(
            "Local Judge baseline initialization.",
            "Decisions log > 2026-05-09 — Architecture set v1.0 established"
        ),
        PragmaticRole::Name
    );
}

#[test]
fn declined_and_withdrawn_are_outcomes() {
    assert_eq!(
        classify_name("declined", "Decisions log > 2026-08-07 — Runtime value memoization is DECLINED; invariance is a declaration, not a cache"),
        PragmaticRole::StatusNote
    );
    assert_eq!(
        classify_name(
            "withdrawn as wrong",
            "Decisions log > 2026-06-08 — Consumer contract established"
        ),
        PragmaticRole::StatusNote
    );
    // Real sentence, decisions-log:1140.
    assert_eq!(
        classify_list_item(
            "*Value memoization* (runtime cache, equality-keyed) is declined above.",
            "Decisions log > 2026-08-07 — Runtime value memoization is DECLINED; invariance is a declaration, not a cache"
        ),
        PragmaticRole::StatusNote
    );
}

#[test]
fn superseded_domain_word_still_a_status_note() {
    // The slice-1 known weakness, preserved, not fixed here.
    assert_eq!(
        classify_name(
            "superseded nodes are left orphaned",
            "Intermediate representation"
        ),
        PragmaticRole::StatusNote
    );
}

// --- Mechanism 2: note-to-row ownership ---

fn miniature_roadmap() -> Vec<String> {
    vec![
        "7. **Named `reference_function` + derived `audited_status`** — a precision bound must NAME the thing it bounds.".to_string(),
        "137. **Separation-of-concerns cleanup near the candidate seams (opened 2026-11-01).** For each seam row 136 marks \"needs cleanup first\", apply the method in the design note. **S3 ANALYSIS (2026-11-01, `docs/design-notes/2026-11-01-seam-s3-executor-backends.md`):** the executor reaches the backends through a closed enum.".to_string(),
        "157. **Multi-node placement pathfinder: build the multi-path DAG the architecture describes (GAP-385; opened 2026-11-15).**".to_string(),
    ]
}

#[test]
fn title_naming_an_existing_row_owns_the_note() {
    let text = "# Multi-node placement pathfinder (ROADMAP row 157, GAP-385)\n\nDesign, before any code.\n";
    assert_eq!(
        owner_for(
            "2026-11-15-multi-node-placement-pathfinder.md",
            text,
            &miniature_roadmap()
        ),
        Some(157)
    );
}

#[test]
fn title_naming_a_missing_row_owns_nothing() {
    let text = "# Plan (ROADMAP row 999)\n\nNothing tasks this.\n";
    assert_eq!(
        owner_for("2026-11-15-some-plan.md", text, &miniature_roadmap()),
        None
    );
}

#[test]
fn row_citing_the_note_owns_it() {
    let text = "# Seam S3: executor (fuel-tensor, fuel-dispatch) and the backends\n\nThe analysis row 137 executes.\n";
    assert_eq!(
        owner_for(
            "2026-11-01-seam-s3-executor-backends.md",
            text,
            &miniature_roadmap()
        ),
        Some(137)
    );
}

#[test]
fn citation_inside_a_multiline_row_belongs_to_the_enclosing_row() {
    // The ROADMAP's early rows span several lines, and it also contains
    // numbered sub-lists ("1. Add ...") that are not rows. The owner is
    // the nearest preceding `N. **` row-start, not the nearest number.
    let roadmap = vec![
        "7. **Named `reference_function` + derived `audited_status`** — a precision bound must NAME".to_string(),
        "the thing it bounds, so it cannot be authoritative without evaluating the candidate (seam widening — see".to_string(),
        "[`docs/design-notes/2026-07-23-kiss-corpus-verdict-seam-mismatch.md`](docs/design-notes/2026-07-23-kiss-corpus-verdict-seam-mismatch.md)).".to_string(),
        "8. **Next row.**".to_string(),
        "1. Add `StorageMap` to `Graph`. A numbered sub-list inside some other row's body.".to_string(),
    ];
    let text =
        "# KISS corpus exists — but `corpus_verdict`'s seam can't consume it (yet)\n\nThe mismatch.\n";
    assert_eq!(
        owner_for(
            "2026-07-23-kiss-corpus-verdict-seam-mismatch.md",
            text,
            &roadmap
        ),
        Some(7)
    );
}

#[test]
fn no_signal_owns_nothing() {
    let text = "# ARC-004 (fuel#680): what the 204 sites are, the cure per group, and what the splits look like after\n\nA cure note.\n";
    assert_eq!(
        owner_for("2026-11-14-arc004-cure.md", text, &miniature_roadmap()),
        None
    );
}

#[test]
fn title_and_citation_agreeing_give_the_same_row() {
    let text = "# ROADMAP row 137 step 6: move backend-gated code out of `fuel-dispatch` (read and plan, no code)\n\nThe plan.\n";
    let mut roadmap = miniature_roadmap();
    roadmap[1] = "137. **Separation-of-concerns cleanup near the candidate seams.** READ AND PLANNED 2026-11-03, `docs/design-notes/2026-11-03-row137-step6-plan.md`, no code.".to_string();
    assert_eq!(
        owner_for("2026-11-03-row137-step6-plan.md", text, &roadmap),
        Some(137)
    );
}

#[test]
fn first_heading_extracts_the_title() {
    assert_eq!(
        first_heading("# Multi-node placement pathfinder (ROADMAP row 157, GAP-385)\n\nBody.\n"),
        Some("Multi-node placement pathfinder (ROADMAP row 157, GAP-385)".to_string())
    );
    assert_eq!(first_heading("No headings here.\n"), None);
    // A level-2 heading first does not count as the title.
    assert_eq!(
        first_heading("## Subsection\n\n# Real title\n"),
        Some("Real title".to_string())
    );
}
