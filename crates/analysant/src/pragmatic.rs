// SPDX-License-Identifier: MIT OR Apache-2.0
//! Applying the pragmatic lens to a corpus, and the partition the directed-but-untasked report makes
//! with it: an absence is a *finding* only when the claim does what its form claims; otherwise it is
//! *set aside*, counted and listed, never silently dropped (the lens stays checkable).

use crate::corpus::Corpus;
use crate::untasked::UntaskedRun;
use analysant_core::{Derivation, Finding};
use analysant_pragmatic::{PragmaticLens, is_reportable};

/// Set `kind` on every claim the lens has rules for. Returns each classified claim's derivation, in
/// `Corpus::claims()` order (`None` where the lens has no rule for the form).
pub fn classify_corpus(corpus: &mut Corpus, lens: &PragmaticLens<'_>) -> Vec<Option<Derivation>> {
    let mut derivations = Vec::new();
    for doc in &mut corpus.documents {
        let sections: Vec<String> = doc
            .claims
            .iter()
            .map(|c| doc.section_path(c).join(" > "))
            .collect();
        for (claim, section) in doc.claims.iter_mut().zip(&sections) {
            match lens.classify(claim, section) {
                Some(v) => {
                    claim.kind = v.kind;
                    derivations.push(Some(v.derivation));
                }
                None => derivations.push(None),
            }
        }
    }
    derivations
}

/// An untasked run split by the pragmatic lens.
pub struct Partition<'a> {
    /// Absent and reportable: the findings.
    pub findings: Vec<(usize, &'a Finding)>,
    /// Absent but not doing what its form claims.
    pub set_aside: Vec<(usize, &'a Finding)>,
}

/// Split the absences of `run` (indices into `Corpus::claims()`) by `is_reportable`.
pub fn partition<'a>(corpus: &Corpus, run: &'a UntaskedRun) -> Partition<'a> {
    let claims: Vec<_> = corpus.claims().collect();
    let (findings, set_aside) = run.absent().partition(|(i, _)| {
        let (_, c) = claims[*i];
        is_reportable(&c.form, c.kind)
    });
    Partition {
        findings,
        set_aside,
    }
}
