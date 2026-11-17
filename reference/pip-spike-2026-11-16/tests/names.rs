//! Tests for slice 2: capability-name extraction.
//!
//! The named list's fragment noise is sentences, verb-led fragments,
//! versioned statements, and property statements wearing a capability
//! name's shape. A name is a noun phrase naming a thing; a fragment
//! is a sentence, a clause, or a property statement. Every verdict
//! names the rule that fired. Written against the spec (derived from
//! the v4 run's 64 absent named claims) before the implementation.

use doc_analysis_spike::claim::{ClaimKind, PragmaticRole};
use doc_analysis_spike::names::fragment_reason;
use doc_analysis_spike::parse::extract_claims;
use doc_analysis_spike::pragmatic::{classify_name, is_reportable};

fn is_name(text: &str) -> bool {
    classify_name(text, "The optimized form") == PragmaticRole::Name
}

fn is_fragment(text: &str) -> bool {
    classify_name(text, "The optimized form") == PragmaticRole::Description
}

#[test]
fn finite_verb_makes_a_fragment() {
    assert_eq!(
        fragment_reason("The pick now runs inside the executor"),
        Some("finite-verb")
    );
    assert_eq!(
        fragment_reason("ships a bundled baseline Judge dataset in-package"),
        Some("finite-verb")
    );
    assert_eq!(
        fragment_reason("Per-call overrides don't mutate the IR."),
        Some("finite-verb")
    );
    assert_eq!(
        fragment_reason("matmul role-vector op_attrs serialize/resolve live in both directions"),
        Some("finite-verb")
    );
    assert_eq!(
        fragment_reason("multiple alternative routes that diverge and reconverge"),
        Some("finite-verb")
    );
    assert_eq!(
        fragment_reason("Backend kernel-revision hash changed"),
        Some("finite-verb")
    );
    assert_eq!(
        fragment_reason("Tolerance configuration changed"),
        Some("finite-verb")
    );
    assert!(is_fragment("The pick now runs inside the executor"));
}

#[test]
fn verb_opening_makes_a_fragment() {
    assert_eq!(
        fragment_reason("populated at startup"),
        Some("verb-opening")
    );
    assert_eq!(
        fragment_reason("retained as a permanent artifact"),
        Some("verb-opening")
    );
    assert_eq!(
        fragment_reason("keep the per-device Pareto frontier (crowding-capped)"),
        Some("verb-opening")
    );
    assert_eq!(
        fragment_reason("Loading a .fuel on changed hardware"),
        Some("verb-opening")
    );
    assert_eq!(
        fragment_reason("write-new-file-and-swap"),
        Some("verb-opening")
    );
    assert!(is_fragment("populated at startup"));
    assert!(is_fragment("Loading a .fuel on changed hardware"));
}

#[test]
fn hyphenated_compounds_are_names_not_fragments() {
    // The first hyphen-segment is a noun ("Fuel", "mmap", "length",
    // "build-time"), so the verb-opening rule does not fire — and the
    // participle inside the compound is attributive, not predicative.
    assert_eq!(fragment_reason("Fuel-orchestrated JIT registration"), None);
    assert_eq!(fragment_reason("mmap-backed zero-copy view"), None);
    assert_eq!(fragment_reason("length-prefixed positional-blob"), None);
    assert_eq!(fragment_reason("build-time-closed Rust enum"), None);
    assert_eq!(
        fragment_reason("Fuel-orchestrated runtime kernel registration"),
        None
    );
    assert!(is_name("Fuel-orchestrated JIT registration"));
    assert!(is_name("mmap-backed zero-copy view"));
}

#[test]
fn parenthetical_verbs_do_not_fire() {
    // A verb inside a parenthetical tag is not the phrase's grammar.
    assert_eq!(
        fragment_reason("crowding-distance cap (keep-per-device)"),
        None
    );
    assert_eq!(fragment_reason("Shallow (just-in-time)"), None);
    assert_eq!(fragment_reason("Bounded (default)"), None);
    assert!(is_name("crowding-distance cap (keep-per-device)"));
    assert!(is_name("Shallow (just-in-time)"));
    assert!(is_name("Bounded (default)"));
}

#[test]
fn version_arrow_and_quantifier_openings_are_fragments() {
    assert_eq!(
        fragment_reason("v1.3 refines landed-vs-intent and salvages two durable facts"),
        Some("version-opening")
    );
    assert_eq!(fragment_reason("Disk → host RAM"), Some("arrow"));
    assert_eq!(fragment_reason("Host RAM → device VRAM"), Some("arrow"));
    assert_eq!(
        fragment_reason("few decision points (branch points)"),
        Some("quantifier-opening")
    );
    assert_eq!(
        fragment_reason("only localized re-optimization"),
        Some("quantifier-opening")
    );
    assert_eq!(
        fragment_reason("No silent runtime fallback to a different op."),
        Some("quantifier-opening")
    );
    assert_eq!(
        fragment_reason("no dedicated mask op"),
        Some("quantifier-opening")
    );
    assert_eq!(fragment_reason("lazily"), Some("quantifier-opening"));
}

#[test]
fn per_fragments_but_per_quantities_are_names() {
    assert_eq!(fragment_reason("per ending device"), Some("per-fragment"));
    assert_eq!(
        fragment_reason("per-decision-point during the dispatch walk"),
        Some("per-fragment")
    );
    assert_eq!(
        fragment_reason("per decision point with merged path sets"),
        Some("per-fragment")
    );
    assert_eq!(fragment_reason("per-tier free memory"), None);
    assert!(is_fragment("per ending device"));
    assert!(is_name("per-tier free memory"));
}

#[test]
fn predicative_participle_is_a_fragment_attributive_is_a_name() {
    assert_eq!(
        fragment_reason("Profile data refined for cells"),
        Some("predicative-participle")
    );
    assert_eq!(
        fragment_reason("Memory-mapped at startup"),
        Some("predicative-participle")
    );
    // A participle before its head noun is attributive, not predicative.
    assert_eq!(
        fragment_reason("The persisted graph / optimization cache"),
        None
    );
    assert_eq!(fragment_reason("The cached resolved route"), None);
    assert!(is_fragment("Profile data refined for cells"));
    assert!(is_name("The persisted graph / optimization cache"));
}

#[test]
fn not_yet_and_temporal_tails_are_fragments() {
    assert_eq!(
        fragment_reason("the architecture's intent but not yet built"),
        Some("not-yet")
    );
    assert_eq!(fragment_reason("re-bound each pass"), Some("temporal-tail"));
    assert_eq!(
        fragment_reason("silent op *substitution* mid-realize"),
        Some("temporal-tail")
    );
}

#[test]
fn sentence_period_with_temporal_preposition_is_a_fragment() {
    assert_eq!(
        fragment_reason("Immutable nodes after creation."),
        Some("sentence-period")
    );
    // No trailing period: a name, not a sentence.
    assert_eq!(fragment_reason("Bisection during debugging"), None);
    // Trailing period but no temporal preposition: a name.
    assert_eq!(fragment_reason("Path resolution at decision points."), None);
    assert_eq!(fragment_reason("Side-table metadata."), None);
    assert!(is_fragment("Immutable nodes after creation."));
    assert!(is_name("Bisection during debugging"));
    assert!(is_name("Path resolution at decision points."));
}

#[test]
fn real_capability_names_stay_names() {
    for name in [
        "single source of truth",
        "lookahead window",
        "Cache management",
        "Pattern harvesting against canonical form",
        "user-facing form",
        "Strategic decisions",
        "Synchronization at join points.",
        "Re-optimization from canonical state.",
        "Per-decision-point atomic swap.",
        "Default — capacity shape + valid-count.",
        "Fallback — symbolic dim + realize-time specialization.",
        "cross-tier prefetch schedule",
        "unit marker",
        "storage class",
    ] {
        assert_eq!(fragment_reason(name), None, "{name}");
        assert!(is_name(name), "{name}");
    }
}

#[test]
fn known_misses_survive_as_names() {
    // Bare noun-phrase property statements are indistinguishable from
    // names without semantics. They survive, and the report says so —
    // these tests pin the current behavior so a later slice can see
    // it change.
    assert_eq!(fragment_reason("hardware-independent and portable"), None);
    assert_eq!(
        fragment_reason("recipe in two inverse directions — both mandatory"),
        None
    );
    // An imperative followed by an adjective ("orchestrate eager ...")
    // is a clause the rules cannot tell from a compound name: the
    // imperative guard only fires on an article object or a hyphenated
    // verb phrase, so this survives too.
    assert_eq!(fragment_reason("orchestrate eager collectives above"), None);
}

#[test]
fn mixed_doc_reports_names_and_sets_aside_fragments() {
    let doc = "# The base map\n\n- **Pattern harvesting against canonical form** (see [08-pattern-harvest](08-pattern-harvest.md))\n\nThe **lookahead window** guides the walk. **populated at startup** is how the cache fills. **Fuel-orchestrated JIT registration** is the mechanism. **The pick now runs inside the executor** for now.\n";
    let claims = extract_claims("06-runtime.md", doc);
    let named: Vec<_> = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Named)
        .collect();
    let reportable: Vec<_> = named.iter().filter(|c| is_reportable(c)).collect();
    let keys: Vec<&str> = reportable.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(
        keys,
        [
            "pattern harvesting against canonical form",
            "lookahead window",
            "fuel orchestrated jit registration",
        ]
    );
    assert!(named
        .iter()
        .any(|c| c.key == "populated at startup" && c.role == PragmaticRole::Description));
    assert!(named
        .iter()
        .any(|c| c.key == "pick now runs inside the executor"
            && c.role == PragmaticRole::Description));
}
