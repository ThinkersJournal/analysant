// SPDX-License-Identifier: MIT OR Apache-2.0
//! Matching modes, named so a finding can say which one fired (SPEC-STEP1 3.2).
//!
//! Ideas credited to the reference spike (reference/pip-spike-2026-11-16, `MIT OR Apache-2.0`, same
//! licence): token-sequence containment and the looser any-order token subset, which exists on
//! purpose because word order is not part of a capability's identity.

use analysant_core::NormalForm;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MatchMode {
    /// A contiguous token run.
    Sequence,
    /// Every needle token is present in the haystack, in any order.
    TokenSubset,
    // `Family` (morphological family, shared long prefix) is specified in 3.2 and lands with the
    // contradiction lens, which is the first user that needs it.
}

impl MatchMode {
    /// The name a search record carries.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sequence => "sequence",
            Self::TokenSubset => "token-subset",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchEvidence {
    pub mode: MatchMode,
    /// Spans, in the haystack's own text, of the tokens that matched.
    pub matched: Vec<Range<usize>>,
    /// The rule that fired.
    pub rule: &'static str,
}

/// Does `hay` contain `needle` under `mode`? An empty needle matches nothing: "everything contains
/// nothing" would make every key present.
pub fn matches(hay: &NormalForm, needle: &NormalForm, mode: MatchMode) -> Option<MatchEvidence> {
    if needle.tokens.is_empty() {
        return None;
    }
    match mode {
        MatchMode::Sequence => {
            let n = needle.tokens.len();
            if n > hay.tokens.len() {
                return None;
            }
            hay.tokens.windows(n).find_map(|w| {
                w.iter()
                    .zip(&needle.tokens)
                    .all(|(a, b)| a.norm == b.norm)
                    .then(|| MatchEvidence {
                        mode,
                        matched: w.iter().map(|t| t.span.clone()).collect(),
                        rule: "contiguous token run",
                    })
            })
        }
        MatchMode::TokenSubset => {
            let mut matched = Vec::with_capacity(needle.tokens.len());
            for want in &needle.tokens {
                let found = hay.tokens.iter().find(|t| t.norm == want.norm)?;
                matched.push(found.span.clone());
            }
            Some(MatchEvidence {
                mode,
                matched,
                rule: "every needle token present, any order",
            })
        }
    }
}
