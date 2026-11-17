//! Tests for the spike, written against the spec before the implementation
//! was finished: the claim model, normalization, extraction, and the
//! directed-but-untasked check — including a miniature of the Fuel DAG
//! failure (a directed capability with no ROADMAP row).

use doc_analysis_spike::check::{directed_but_untasked, FindingKind};
use doc_analysis_spike::claim::ClaimKind;
use doc_analysis_spike::normalize::{
    canonicalize, contains_token_sequence, contains_token_subset, normalize, parse_aliases,
};
use doc_analysis_spike::parse::extract_claims;

#[test]
fn normalize_lowercases_strips_punctuation_and_drops_leading_article() {
    assert_eq!(normalize("Multi-Path DAG"), "multi path dag");
    assert_eq!(normalize("The Plan IS the Graph"), "plan is the graph");
    assert_eq!(normalize("Op::Branch"), "op branch");
    assert_eq!(normalize("  spaced   out  "), "spaced out");
}

#[test]
fn token_sequence_matching_is_exact_not_substring() {
    // "op branch" must not match inside "stop branch".
    assert!(contains_token_sequence("the op branch node", "op branch"));
    assert!(!contains_token_sequence(
        "the stop branch node",
        "op branch"
    ));
    assert!(!contains_token_sequence("branch", "op branch"));
    assert!(contains_token_sequence("multi path dag", "multi path dag"));
}

#[test]
fn token_subset_matching_ignores_word_order() {
    assert!(contains_token_subset("dag multi path", "multi path dag"));
    assert!(!contains_token_subset("multi path", "multi path dag"));
    assert!(!contains_token_subset("", "multi path dag"));
}

#[test]
fn aliases_canonicalize_both_sides() {
    let groups = parse_aliases("multi-path DAG | multi-path graph\n");
    assert_eq!(groups.len(), 1);
    assert_eq!(
        canonicalize("the multi path graph", &groups),
        "the multi path dag"
    );
    assert_eq!(
        canonicalize("optimized multi path graph", &groups),
        "optimized multi path dag"
    );
    // No group matches: unchanged.
    assert_eq!(canonicalize("base map", &groups), "base map");
}

#[test]
fn headings_set_section_and_fences_are_skipped() {
    let doc = "# The optimized form\n\nSome prose.\n\n```rust\nlet x = 1;\n```\n\n- build the multi-path DAG\n";
    let claims = extract_claims("03-ir.md", doc);
    // The fenced code line must not produce a claim; the list item must.
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].kind, ClaimKind::Directive);
    assert_eq!(claims[0].text, "build the multi-path DAG");
    assert_eq!(claims[0].section, "The optimized form");
    assert_eq!(claims[0].key, "build multi path dag");
}

#[test]
fn bold_phrases_become_named_claims_and_backticks_become_identifiers() {
    let doc = "The **multi-path graph** is the plan. See `Op::Branch`.\n\nLater the **multi-path graph** appears again, and `Op::Branch` too.\n";
    let claims = extract_claims("03-ir.md", doc);
    let named: Vec<_> = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Named)
        .collect();
    let identifiers: Vec<_> = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Identifier)
        .collect();
    assert_eq!(named.len(), 1);
    assert_eq!(named[0].key, "multi path graph");
    assert_eq!(identifiers.len(), 1);
    assert_eq!(identifiers[0].key, "op branch");
}

#[test]
fn bolded_sentences_are_not_capability_names() {
    let doc = "This is **a whole sentence, with commas, and it goes on.**\n\nAnd **AND THE NOTE WAS APPLIED TO ONE OF FOUR NAMES.**\n\nBut the **multi-path graph** is a name.\n";
    let claims = extract_claims("02-layers.md", doc);
    let named: Vec<_> = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Named)
        .collect();
    assert_eq!(named.len(), 1);
    assert_eq!(named[0].key, "multi path graph");
}

#[test]
fn table_first_cells_become_named_claims_and_headers_yield_nothing() {
    let doc = "| Component | Status |\n| --- | --- |\n| `Op::Branch` + builders | BUILT |\n| Placement fork | PARTIAL |\n";
    let claims = extract_claims("audit.md", doc);
    let named: Vec<_> = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Named)
        .collect();
    let identifiers: Vec<_> = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Identifier)
        .collect();
    // Header yields nothing. First row: the first-cell phrase (Named) and
    // the backtick identifier inside it (Identifier, different key).
    assert_eq!(named.len(), 2);
    assert!(named.iter().any(|c| c.key == "op branch builders"));
    assert!(named.iter().any(|c| c.key == "placement fork"));
    assert_eq!(identifiers.len(), 1);
    assert_eq!(identifiers[0].key, "op branch");
}

#[test]
fn dag_in_miniature_absent_when_roadmap_has_no_row() {
    let arch = "# Optimized form\n\nThe optimized form is the **multi-path graph**.\n\n- build the multi-path DAG\n";
    let roadmap = "# Roadmap\n\n- ship the base map\n- land the runtime picker\n";
    let claims = extract_claims("03-ir.md", arch);
    let roadmap_lines: Vec<String> = roadmap.lines().map(|l| l.to_string()).collect();
    let findings = directed_but_untasked(&claims, &roadmap_lines, &[]);
    let absent: Vec<_> = findings
        .iter()
        .filter(|f| f.kind == FindingKind::Absent)
        .collect();
    // Both the directive and the named capability are absent.
    assert!(absent.iter().any(|f| f.claim.key == "build multi path dag"));
    assert!(absent.iter().any(|f| f.claim.key == "multi path graph"));
}

#[test]
fn dag_in_miniature_mentioned_when_roadmap_gains_the_row() {
    let arch = "# Optimized form\n\nThe optimized form is the **multi-path graph**.\n\n- build the multi-path DAG\n";
    let roadmap = "# Roadmap\n\n- build the multi-path DAG the architecture describes\n- the multi-path graph is the plan\n";
    let claims = extract_claims("03-ir.md", arch);
    let roadmap_lines: Vec<String> = roadmap.lines().map(|l| l.to_string()).collect();
    let findings = directed_but_untasked(&claims, &roadmap_lines, &[]);
    assert!(findings.iter().all(|f| f.kind == FindingKind::Mentioned));
    assert!(findings.iter().any(|f| f.roadmap_hits == 1));
}

#[test]
fn alias_bridges_graph_and_dag() {
    // The architecture says "graph", the ROADMAP says "DAG": without the
    // alias group they are different keys; with it they are one claim.
    let groups = parse_aliases("multi-path DAG | multi-path graph\n");
    let arch = "The optimized form is the **multi-path graph**.\n";
    let roadmap = "- build the multi-path DAG the architecture describes\n";
    let claims = extract_claims("03-ir.md", arch);
    let roadmap_lines: Vec<String> = roadmap.lines().map(|l| l.to_string()).collect();
    let without = directed_but_untasked(&claims, &roadmap_lines, &[]);
    assert!(without.iter().all(|f| f.kind == FindingKind::Absent));
    let with = directed_but_untasked(&claims, &roadmap_lines, &groups);
    assert!(with.iter().all(|f| f.kind == FindingKind::Mentioned));
}
