// SPDX-License-Identifier: MIT OR Apache-2.0
//! The contradiction lens applied to a corpus (SPEC-STEP1 section 6, slice 4): a document's headings
//! and its claims become statements, the lens pairs them. The lens itself is `analysant-logical`; this
//! module is the part that knows documents.

use crate::corpus::Corpus;
use analysant_core::UnitKind;
use analysant_logical::{ContradictionLens, Located, Statement};

/// Statements from every claim of the corpus, then from every heading, in reading order. The order is
/// part of the behaviour: when two candidate pairs share a line and tie on shared tokens, the one seen
/// first wins.
pub fn statements(corpus: &Corpus, lens: &ContradictionLens<'_>) -> Vec<Statement> {
    let sections: Vec<Vec<String>> = corpus
        .documents
        .iter()
        .map(|d| {
            d.claims
                .iter()
                .map(|c| d.section_path(c).join(" > "))
                .collect()
        })
        .collect();
    let located: Vec<Located<'_>> = corpus
        .documents
        .iter()
        .zip(&sections)
        .flat_map(|(d, secs)| {
            d.claims.iter().zip(secs).map(|(claim, section)| Located {
                file: &d.path,
                section,
                claim,
            })
        })
        .collect();
    let mut out = lens.statements_from_claims(&located);
    for d in &corpus.documents {
        for s in d.sections.iter().filter(|u| u.kind == UnitKind::Section) {
            let Some(title) = s.heading.as_deref() else {
                continue;
            };
            let parents = heading_parents(d, s.parent);
            out.extend(lens.statements_from_heading(
                &d.path,
                s.span.line_start as usize,
                title,
                &parents.join(" > "),
            ));
        }
    }
    out
}

fn heading_parents(
    d: &crate::corpus::Document,
    mut at: Option<analysant_core::UnitId>,
) -> Vec<String> {
    let mut path = Vec::new();
    while let Some(id) = at {
        let Some(sec) = d.sections.iter().find(|s| s.id == id) else {
            break;
        };
        path.push(sec.heading.clone().unwrap_or_default());
        at = sec.parent;
    }
    path.reverse();
    path
}
