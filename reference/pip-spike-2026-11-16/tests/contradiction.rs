//! Slice 5 (doc-vs-doc contradiction lens) tests, written against the
//! spec derived from the real cases before the implementation.
//!
//! A contradiction is a PAIR of statements about the same capability
//! where one side negates it and the other asserts it. Statements come
//! from headings (a document's own position) and from extracted claims
//! (everything except code identifiers and status notes). Polarity is
//! deterministic: temporal markers first ("not yet", dates, retractions),
//! then negation markers ("not supported", "cannot", "no <capability>"),
//! else positive.
//!
//! The first case, from the ground-truth audit §6 item 7: `06`
//! "cancellation not supported" vs `15` C-2 "can cancel". Side A is the
//! heading `06-runtime.md:71` "Cancellation is not supported"; side B
//! is the heading `15-consumer-contract.md:91` "C-2 — Bounded quantum
//! and cancellation". The C-2 paragraph ("can cancel in flight") yields
//! no claim under the prose parser, so the lens MUST read headings —
//! that is why headings are statements here.

use doc_analysis_spike::claim::{Claim, ClaimKind, PragmaticRole};
use doc_analysis_spike::contradiction::{
    find_contradictions, polarity, same_family, statements_from_claims, statements_from_headings,
    Polarity, Statement,
};

fn heading(file: &str, line: usize, title: &str) -> Statement {
    // Place the heading on the requested line so its id is realistic.
    let text = format!("{}# {title}\n", "\n".repeat(line - 1));
    statements_from_headings(file, &text)
        .into_iter()
        .find(|s| s.line == line)
        .unwrap_or_else(|| panic!("no heading statement for {title:?}"))
}

// --- The headline case ---

#[test]
fn cancellation_pair_from_headings() {
    // Real headings: 06-runtime.md:71 and 15-consumer-contract.md:91.
    let a = heading("06-runtime.md", 71, "Cancellation is not supported");
    assert_eq!(a.polarity, Polarity::Negated);
    let b = heading(
        "15-consumer-contract.md",
        91,
        "C-2 — Bounded quantum and cancellation",
    );
    assert_eq!(b.polarity, Polarity::Positive);
    let pairs = find_contradictions(&[a, b]);
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].shared, vec!["cancellation".to_string()]);
    assert_eq!(pairs[0].negated_by, Some("not supported".to_string()));
}

// --- Morphology: the capability family ---

#[test]
fn cancel_family_morphology() {
    // cancel / cancellation / cancelled / cancels / cancelling are one
    // family; the lens must see them as the same capability.
    assert!(same_family("cancel", "cancellation"));
    assert!(same_family("cancel", "cancelled"));
    assert!(same_family("cancel", "cancels"));
    assert!(same_family("cancelling", "cancellation"));
    assert!(same_family("cancelled", "cancellation"));
}

#[test]
fn morphology_does_not_overmerge() {
    // A shared prefix is not a family when the shorter token is short:
    // "state" (5) is not the family of "statement".
    assert!(!same_family("state", "statement"));
    // Unrelated words sharing a first letter are not a family.
    assert!(!same_family("cancel", "capacity"));
    assert!(!same_family("tolerance", "token"));
}

#[test]
fn plurals_are_one_family() {
    // Differing only by a trailing "s" is one family, whatever the
    // length: graph/graphs, token/tokens, op/ops.
    assert!(same_family("graph", "graphs"));
    assert!(same_family("token", "tokens"));
    assert!(same_family("op", "ops"));
    assert!(same_family("persist", "persistence"));
}

// --- Polarity: temporal is not negation ---

#[test]
fn not_yet_is_temporal_not_negated() {
    // Real text, 04-optimization.md:361: "Concurrent optimize-and-
    // execute is not yet landed." A "must support X" vs "X not yet
    // built" pair is directed-but-untasked, not a contradiction.
    let (pol, _) = polarity("Concurrent optimize-and-execute is not yet landed.");
    assert_eq!(pol, Polarity::Temporal);
    let (pol, _) = polarity("Per-decision-point alternatives are now landed.");
    assert_eq!(pol, Polarity::Positive);
}

#[test]
fn not_yet_does_not_pair_with_a_requirement() {
    let a = heading(
        "04-optimization.md",
        361,
        "Concurrent optimize-and-execute is not yet landed",
    );
    assert_eq!(a.polarity, Polarity::Temporal);
    let b = heading(
        "15-consumer-contract.md",
        50,
        "Concurrent optimize-and-execute",
    );
    assert_eq!(b.polarity, Polarity::Positive);
    assert!(find_contradictions(&[a, b]).is_empty());
}

#[test]
fn dated_heading_is_temporal() {
    // A dated heading is a record, not a position: the log's
    // "2026-07-24 — conv2d are NOT a primitive-basis gap" entry is a
    // correction record, and the date rule keeps it out of pairing even
    // though it contains "are NOT".
    let (pol, _) = polarity("2026-07-24 — conv2d are NOT a primitive-basis gap");
    assert_eq!(pol, Polarity::Temporal);
}

#[test]
fn retraction_words_are_temporal() {
    // "no longer", "superseded", "retired", "withdrawn", "rejected":
    // a retraction says it WAS and now is not — temporal, not a
    // logical negation of a capability another doc asserts.
    for text in [
        "That is no longer true and was misleading",
        "Superseded nodes are left orphaned",
        "The old registry is retired",
        "The proposal was withdrawn",
    ] {
        assert_eq!(polarity(text).0, Polarity::Temporal, "{text}");
    }
}

// --- Polarity: negation markers ---

#[test]
fn negation_markers() {
    for text in [
        "Cancellation is not supported",
        "Most backends can't cleanly cancel queued work",
        "The runtime cannot preempt a running kernel",
        "No fairness guarantee is provided",
        "Attention scores are not obtainable on the fused arm",
        "There is no batching policy",
        "This is not possible on the current backend",
    ] {
        assert_eq!(polarity(text).0, Polarity::Negated, "{text}");
    }
}

#[test]
fn hyphenated_never_is_a_property_not_a_negation() {
    // "never-panics" is a compound adjective naming a property
    // (decompose is total, never-panics); "never panics" as two words
    // is a negation. The marker must be a standalone word.
    let (pol, _) = polarity("decompose is total, never-panics");
    assert_eq!(pol, Polarity::Positive);
    let (pol, _) = polarity("the runtime never replays a dispatched run");
    assert_eq!(pol, Polarity::Negated);
}

#[test]
fn no_longer_is_temporal_not_a_no_refusal() {
    // "no longer" is caught by the temporal rule before the "no" +
    // word rule ever sees it.
    let (pol, _) = polarity("fuel-core-types no longer exists");
    assert_eq!(pol, Polarity::Temporal);
}

// --- Statements: what counts as a side ---

fn named_claim(file: &str, line: usize, text: &str, role: PragmaticRole) -> Claim {
    Claim {
        id: format!("{file}:{line}"),
        key: doc_analysis_spike::normalize::normalize(text),
        text: text.to_string(),
        source_file: file.to_string(),
        line,
        section: String::new(),
        kind: ClaimKind::Named,
        role,
    }
}

#[test]
fn identifiers_are_not_statements() {
    // Real case: `cancel_token` (audit row 64: 0 hits) is a code
    // observation, not an assertion — "no cancel" vs "cancel_token
    // absent" must not pair.
    let id_claim = Claim {
        id: "06-runtime.md:64".to_string(),
        key: "cancel token".to_string(),
        text: "cancel_token".to_string(),
        source_file: "06-runtime.md".to_string(),
        line: 64,
        section: String::new(),
        kind: ClaimKind::Identifier,
        role: PragmaticRole::Name,
    };
    assert!(statements_from_claims(&[id_claim]).is_empty());
}

#[test]
fn status_notes_are_not_statements() {
    // A retraction ("PlanStore is likewise gone") is a status report,
    // not a position to contradict.
    let gone = named_claim(
        "03-ir.md",
        100,
        "PlanStore is likewise gone",
        PragmaticRole::StatusNote,
    );
    assert!(statements_from_claims(&[gone]).is_empty());
}

#[test]
fn seven_clauses_refusals_do_not_pair() {
    // Real text, decisions-log:948. The "Seven clauses" directive
    // lists the matching refusals (no fairness/SLA, no admission, no
    // work lifecycle, no batching or eviction policy) — but its key is
    // "seven clauses", so the refusals are inside a summary whose
    // subject carries no capability. A positive statement about
    // fairness must not pair with it.
    let item = Claim {
        id: "10-decisions-log.md:948".to_string(),
        key: "seven clauses".to_string(),
        text: "**Seven clauses**: C-1 capacity advertisement, C-2 bounded quantum + cancellation, C-3 state externalization, C-4 measured cost and provenance, C-5 constraint admission, C-6 observation and intervention, C-7 declared mutation and in-process invalidation. Plus the matching refusals (no fairness/SLA, no admission, no work lifecycle, no multi-tenancy policy, no batching or eviction policy).".to_string(),
        source_file: "10-decisions-log.md".to_string(),
        line: 948,
        section: "Decisions log".to_string(),
        kind: ClaimKind::Directive,
        role: PragmaticRole::Description,
    };
    let fairness_positive = named_claim(
        "15-consumer-contract.md",
        260,
        "fairness",
        PragmaticRole::Name,
    );
    let stmts = statements_from_claims(&[item, fairness_positive]);
    assert!(find_contradictions(&stmts).is_empty());
    // And the refusal heading itself yields no statement: "What fuel
    // does NOT decide" is a document-scope heading, its subject empty
    // once document-state words are removed.
    assert!(
        statements_from_headings("15-consumer-contract.md", "# What fuel does NOT decide\n")
            .is_empty()
    );
}

#[test]
fn same_file_pairs_do_not_count() {
    // A document contradicting itself is a different finding — note
    // it if seen, do not report it as a cross-document contradiction.
    let a = heading("06-runtime.md", 71, "Cancellation is not supported");
    let b = heading("06-runtime.md", 80, "Cancellation");
    assert_eq!(b.polarity, Polarity::Positive);
    assert!(find_contradictions(&[a, b]).is_empty());
}

#[test]
fn supported_alone_does_not_pair() {
    // "X is not supported" vs "Y is supported" share only the word
    // "supported" — a stance word, not a capability. The pairing
    // stopwords remove it from both subjects, so there is nothing to
    // pair on.
    let a = heading("06-runtime.md", 10, "Foo is not supported");
    let b = heading("03-ir.md", 10, "Bar is supported");
    assert!(find_contradictions(&[a, b]).is_empty());
}

// --- Pairing: the subset rule and the strength rule ---

#[test]
fn subset_rule_blocks_partial_overlap() {
    // "tolerance budgets are not supported" vs "tolerance
    // configuration": they share "tolerance" but neither subject
    // contains the other — budgets are not configuration.
    let a = heading("06-runtime.md", 10, "Tolerance budgets are not supported");
    let b = heading("04-optimization.md", 10, "Tolerance configuration");
    assert!(find_contradictions(&[a, b]).is_empty());
}

#[test]
fn subset_rule_allows_either_direction() {
    // Negated side more specific than the positive side is still a
    // pair: the positive side's subject is contained in the negated
    // side's.
    let a = heading(
        "06-runtime.md",
        10,
        "Per-op tolerance budgets are not supported",
    );
    let b = heading("04-optimization.md", 10, "Tolerance budgets");
    assert_eq!(find_contradictions(&[a, b]).len(), 1);
}

#[test]
fn single_short_token_does_not_pair() {
    // "graph" (5 letters) alone cannot carry a contradiction: a
    // negated "Graph is not supported" and a positive "Graph" share
    // one token shorter than 6 letters, so the strength rule blocks
    // the pair.
    let a = heading("06-runtime.md", 10, "Graph is not supported");
    let b = heading("03-ir.md", 10, "Graph");
    assert!(find_contradictions(&[a, b]).is_empty());
}

#[test]
fn structural_heading_yields_no_statement() {
    // Real heading, 02-layers.md:95: "What this document covers, and
    // one crate it names that does not exist". Every content word is
    // structural or a light verb ("what", "document", "covers",
    // "crate", "names", "exist"), so the subject is empty and there
    // is no statement at all — nothing to pair.
    let stmts = statements_from_headings(
        "02-layers.md",
        "# What this document covers, and one crate it names that does not exist\n",
    );
    assert!(stmts.is_empty());
}

// --- Contrast headings: "X, not Y" asserts X and denies Y ---

#[test]
fn contrast_heading_splits() {
    // Real heading, 06-runtime.md:57: "Dispatch: runs, not single
    // ops" asserts runs and denies single ops. One heading yields two
    // statements; the denied half pairs with a positive "single ops"
    // statement, the asserted half does not pair with a negated
    // "runs" statement (shared "runs" is 4 letters — too short alone).
    let stmts = statements_from_headings("06-runtime.md", "## Dispatch: runs, not single ops\n");
    assert_eq!(stmts.len(), 2);
    let denied = stmts
        .iter()
        .find(|s| s.polarity == Polarity::Negated)
        .unwrap();
    assert!(denied.subject.contains(&"single".to_string()));
    assert!(denied.subject.contains(&"ops".to_string()));
    let asserted = stmts
        .iter()
        .find(|s| s.polarity == Polarity::Positive)
        .unwrap();
    assert!(asserted.subject.contains(&"runs".to_string()));

    let single_ops = heading("03-ir.md", 20, "Single ops");
    let mut all = stmts.clone();
    all.push(single_ops);
    let pairs = find_contradictions(&all);
    assert_eq!(pairs.len(), 1);
    assert!(pairs[0].shared.contains(&"single".to_string()));

    let runs_denied = heading("04-optimization.md", 30, "Runs are not supported");
    let mut all2 = stmts;
    all2.push(runs_denied);
    assert!(find_contradictions(&all2).is_empty());
}

#[test]
fn document_state_headings_are_not_statements() {
    // "Not verified", "What this note does not settle", "What fuel
    // does NOT decide": headings about the document's own state, not
    // about a capability. Their subjects are empty once document-state
    // words are removed.
    for title in [
        "Not verified",
        "What this note does not settle",
        "What fuel does NOT decide",
        "What I did not do",
    ] {
        let stmts =
            statements_from_headings("2026-11-14-qwen35-27b-plan.md", &format!("# {title}\n"));
        assert!(stmts.is_empty(), "{title}");
    }
}

// --- Redesign (second v8 run): the head window, the bad-thing flip,
// no claim splits, the positive side needing two tokens ---

fn directive_claim(file: &str, line: usize, key: &str, text: &str, role: PragmaticRole) -> Claim {
    Claim {
        id: format!("{file}:{line}"),
        key: key.to_string(),
        text: text.to_string(),
        source_file: file.to_string(),
        line,
        section: String::new(),
        kind: ClaimKind::Directive,
        role,
    }
}

#[test]
fn polarity_comes_from_the_head_not_the_gloss() {
    // Real text, 03-ir.md:17: the head asserts the graph IS the
    // model; the gloss, 45 words in, says "the graph itself is not
    // rebuilt per input". The first v8 run read the whole gloss and
    // marked the claim Negated; the head window reads only the head
    // plus the first clause, so the claim is Positive.
    let claim = directive_claim(
        "03-ir.md",
        17,
        "the graph is the model independent of any input",
        "**The graph is the model, independent of any input.** It is built when a model is loaded or imported — **not** when a forward pass runs — and is ideally fully optimized before the first input arrives. Inputs and per-session state flow *through* the persistent graph (via storage; see below); the graph itself is not rebuilt per input, per token, or per session.",
        PragmaticRole::Description,
    );
    let stmts = statements_from_claims(&[claim]);
    assert_eq!(stmts.len(), 1);
    assert_eq!(stmts[0].polarity, Polarity::Positive);
    // And the reverse: a negation right after the head still fires.
    let denied = directive_claim(
        "06-runtime.md",
        71,
        "cancel in flight",
        "**Cancel in flight** — not supported on CUDA",
        PragmaticRole::Description,
    );
    let stmts = statements_from_claims(&[denied]);
    assert_eq!(stmts.len(), 1);
    assert_eq!(stmts[0].polarity, Polarity::Negated);
}

#[test]
fn negation_of_a_bad_thing_is_positive() {
    // "does not break", "never panics", "never silent", "is not
    // broken", "no failures": each asserts the absence of something
    // bad, not the absence of a capability.
    for text in [
        "a cure that does not break",
        "the runtime never panics",
        "unresolvable names are never silent",
        "the store is not broken",
        "no failures observed",
        "the system cannot fail",
    ] {
        assert_eq!(polarity(text).0, Polarity::Positive, "{text}");
    }
    // Controls: the same markers with a neutral word after them are
    // genuine negations.
    for text in [
        "the backend does not support cancellation",
        "the 4070 cannot hold the weights",
        "the runtime never replays a dispatched run",
        "there is no batching policy",
    ] {
        assert_eq!(polarity(text).0, Polarity::Negated, "{text}");
    }
}

#[test]
fn claim_contrast_does_not_split() {
    // Real shape, 02-layers.md:176: ", not " inside a long gloss is
    // contrastive prose, not a position. A claim yields exactly one
    // statement — no #assert / #deny halves — and its polarity comes
    // from the head window, so the later "aren't" never fires.
    let claim = directive_claim(
        "02-layers.md",
        176,
        "fuel model one architecture per crate",
        "**fuel-model-* — one architecture per crate.** Generic building blocks (RoPE, RMSNorm, GQA attention, SwiGLU MLP) stay in **fuel-nn**, not in a per-family crate; a shared-component crate is extracted only on *real* duplication across ≥2 model crates.",
        PragmaticRole::Description,
    );
    let stmts = statements_from_claims(&[claim]);
    assert_eq!(stmts.len(), 1);
    assert!(!stmts[0].id.contains('#'));
    assert_eq!(stmts[0].polarity, Polarity::Positive);
}

#[test]
fn positive_side_needs_two_subject_tokens() {
    // A bare name ("Cancellation") asserts almost nothing: the
    // negated "Cancellation is not supported" does not pair with it,
    // but does pair with the three-token C-2 heading.
    let a = heading("06-runtime.md", 71, "Cancellation is not supported");
    let bare = heading("15-consumer-contract.md", 91, "Cancellation");
    assert_eq!(bare.subject.len(), 1);
    assert!(find_contradictions(&[a.clone(), bare]).is_empty());
    let full = heading(
        "15-consumer-contract.md",
        91,
        "C-2 — Bounded quantum and cancellation",
    );
    assert_eq!(find_contradictions(&[a, full]).len(), 1);
}

#[test]
fn digit_tokens_are_not_capabilities() {
    // "qwen35", "27b", "conv2d" are versions and identifiers, not
    // capabilities — they must not carry a pair.
    assert_eq!(
        doc_analysis_spike::contradiction::subject_tokens("qwen35 27b weights"),
        vec!["weights".to_string()]
    );
    assert_eq!(
        doc_analysis_spike::contradiction::subject_tokens("conv2d kernels"),
        vec!["kernels".to_string()]
    );
}

#[test]
fn contract_and_backend_are_structural() {
    // "contract" and "backend" appear everywhere in this codebase
    // and identify nothing on their own: the first v8 run paired on
    // "backend" 81 times.
    assert!(doc_analysis_spike::contradiction::subject_tokens("the backend contract").is_empty());
    assert_eq!(
        doc_analysis_spike::contradiction::subject_tokens("backend cancellation token"),
        vec!["cancellation".to_string(), "token".to_string()]
    );
}
