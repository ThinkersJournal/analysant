// SPDX-License-Identifier: MIT OR Apache-2.0
//! Documents to claims: a format adapter finds the structure, a language pack finds the claims, the
//! ledger mints their identities. No claim is classified here (kind stays `Unclassified`).

use crate::{
    BlockKind, Claim, EpistemicStatus, ExtractorId, Form, FormatAdapter, Kind, LanguagePack,
    Ledger, Provenance, Revision, SourceRef, Span, Unit, UnitId, UnitKind, normalize,
};
use std::collections::{HashMap, HashSet};

/// One document: its unit, its section units, and the claims in it, in reading order.
#[derive(Clone, Debug)]
pub struct Document {
    pub path: String,
    pub unit: Unit,
    pub sections: Vec<Unit>,
    pub claims: Vec<Claim>,
}

impl Document {
    /// The headings above a claim, outermost first.
    pub fn section_path(&self, claim: &Claim) -> Vec<&str> {
        let by_id: HashMap<UnitId, &Unit> = self.sections.iter().map(|s| (s.id, s)).collect();
        let mut path = Vec::new();
        let mut at = claim.unit.parent;
        while let Some(id) = at {
            let Some(sec) = by_id.get(&id) else { break };
            path.push(sec.heading.as_deref().unwrap_or(""));
            at = sec.parent;
        }
        path.reverse();
        path
    }
}

#[derive(Clone, Debug, Default)]
pub struct Corpus {
    pub documents: Vec<Document>,
}

impl Corpus {
    /// Every claim, document by document, in reading order. The order outcomes are reported in.
    pub fn claims(&self) -> impl Iterator<Item = (&Document, &Claim)> {
        self.documents
            .iter()
            .flat_map(|d| d.claims.iter().map(move |c| (d, c)))
    }
}

/// Forms that name a thing; one claim per normalized key per document ("a capability named fifty times
/// is one claim"). A list item is never deduplicated: each is its own direction.
fn names_a_thing(form: &Form) -> bool {
    matches!(form, Form::TableCell | Form::Bold | Form::CodeSpan)
}

/// Extract one document.
pub fn extract_document(
    format: &dyn FormatAdapter,
    pack: &dyn LanguagePack,
    ledger: &mut Ledger,
    path: &str,
    text: &str,
) -> Document {
    let extracted_by = ExtractorId {
        format: format.id().name.to_string(),
        format_version: format.id().version.to_string(),
        pack: pack.id().name.to_string(),
        pack_version: pack.id().version.to_string(),
    };
    let whole = Span {
        start: 0,
        end: text.len(),
        line_start: 1,
        line_end: text.lines().count().max(1) as u32,
    };
    let source = |span: Span| SourceRef {
        repo: None,
        path: path.to_string(),
        rev: None,
        span,
    };
    let doc_id = ledger.mint(UnitKind::Document, path, &[], text);
    let doc_unit = Unit {
        id: doc_id,
        kind: UnitKind::Document,
        parent: None,
        ordinal: 0,
        heading: None,
        span: whole,
        source: source(whole),
    };

    let mut sections: Vec<Unit> = Vec::new();
    let mut section_ids: HashMap<Vec<String>, UnitId> = HashMap::new();
    let mut claims: Vec<Claim> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for block in format.blocks(text) {
        if let BlockKind::Heading { .. } = block.kind {
            let mut full = block.section.clone();
            full.push(block.text.clone());
            let parent = section_ids.get(&block.section).copied().unwrap_or(doc_id);
            let id = ledger.mint(UnitKind::Section, path, &full, &block.text);
            sections.push(Unit {
                id,
                kind: UnitKind::Section,
                parent: Some(parent),
                ordinal: sections.len() as u32,
                heading: Some(block.text.clone()),
                span: block.span,
                source: source(block.span),
            });
            section_ids.insert(full, id);
            continue;
        }
        let parent = section_ids.get(&block.section).copied().unwrap_or(doc_id);
        for raw in pack.extract(&block) {
            let normal = normalize(pack, raw.head.as_deref().unwrap_or(&raw.text));
            if names_a_thing(&raw.form) && (normal.is_empty() || !seen.insert(normal.key.clone())) {
                continue;
            }
            let id = ledger.mint(UnitKind::Claim, path, &block.section, &raw.text);
            claims.push(Claim {
                unit: Unit {
                    id,
                    kind: UnitKind::Claim,
                    parent: Some(parent),
                    ordinal: claims.len() as u32,
                    heading: None,
                    span: raw.span,
                    source: source(raw.span),
                },
                revision: Revision(1),
                text: raw.text,
                form: raw.form,
                kind: Kind::Unclassified,
                normal,
                status: EpistemicStatus::Unassessed,
                provenance: Provenance {
                    source: source(raw.span),
                    extracted_by: extracted_by.clone(),
                    extracted_at: None,
                    asserted_at: None,
                    author: None,
                },
            });
        }
    }
    Document {
        path: path.to_string(),
        unit: doc_unit,
        sections,
        claims,
    }
}

/// Extract documents in the order given; the caller sorts, so the order is the caller's choice.
pub fn extract_corpus(
    format: &dyn FormatAdapter,
    pack: &dyn LanguagePack,
    ledger: &mut Ledger,
    files: &[(String, String)],
) -> Corpus {
    Corpus {
        documents: files
            .iter()
            .map(|(path, text)| extract_document(format, pack, ledger, path, text))
            .collect(),
    }
}
