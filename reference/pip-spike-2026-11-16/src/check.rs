//! The one lens the spike implements: directed-but-untasked.
//!
//! A claim from the architecture docs is a finding when no ROADMAP row
//! mentions it. "Mentions" is token-subset containment on the normalized,
//! alias-canonicalized form (see `normalize`): every word of the claim's
//! key appears in that row. Every finding is explainable: these words
//! appear in that row, or in none of them. The check does not judge
//! whether a mention is a task or a passing reference — that triage is a
//! human's job, and the report says so.

use crate::claim::Claim;
use crate::normalize::{canonicalize, contains_token_subset, normalize, AliasGroup};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingKind {
    /// Named in the architecture, in no ROADMAP row.
    Absent,
    /// Named in the architecture and in at least one ROADMAP row.
    Mentioned,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub claim: Claim,
    pub kind: FindingKind,
    /// How many ROADMAP lines mention the claim's key.
    pub roadmap_hits: usize,
}

/// Run the directed-but-untasked check over architecture claims.
/// `roadmap_lines` are the ROADMAP's raw lines; they are normalized and
/// alias-canonicalized here, as are the claim keys.
pub fn directed_but_untasked(
    directives: &[Claim],
    roadmap_lines: &[String],
    aliases: &[AliasGroup],
) -> Vec<Finding> {
    let normalized_roadmap: Vec<String> = roadmap_lines
        .iter()
        .map(|l| canonicalize(&normalize(l), aliases))
        .collect();
    let mut out = Vec::new();
    for claim in directives {
        if claim.key.is_empty() {
            continue;
        }
        let key = canonicalize(&claim.key, aliases);
        let hits = normalized_roadmap
            .iter()
            .filter(|line| contains_token_subset(line, &key))
            .count();
        out.push(Finding {
            claim: claim.clone(),
            kind: if hits == 0 {
                FindingKind::Absent
            } else {
                FindingKind::Mentioned
            },
            roadmap_hits: hits,
        });
    }
    out
}
