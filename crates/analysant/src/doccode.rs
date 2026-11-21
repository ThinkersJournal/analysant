// SPDX-License-Identifier: MIT OR Apache-2.0
//! The doc-vs-code lens (SPEC-STEP1 5.1, "verdict vocabulary"): a document names a code item in a code span;
//! the Rust pack's `CodeIndex` says what the tree has under that name; the lens reports what a reader should
//! look at and nothing else. A name that is defined and called says nothing (`DocCodeStatus::Resolved`).
//!
//! What it does not claim is in every sentence: uses are counted by bare name, the chain is followed one hop,
//! and there is no call graph. A report template, not this lens, maps these kinds onto a project's own words
//! (built, stub, absent).
//!
//! Every finding carries what was searched and a control, and the control is a **different claim**: another
//! name of the same corpus that the same index resolves to a production definition (`KnownPresent`). With no
//! such claim, or with no Rust item indexed at all, a name that would have been reported is *withheld*, because
//! "absent from an empty index" and "absent from the tree" cannot be told apart.

use crate::corpus::Corpus;
use crate::tool::LensOutput;
use analysant_core::{
    Basis, ClaimRef, ControlKind, ControlOutcome, ControlResult, Cost, Derivation, Disposition,
    Finding, FindingId, FindingKind, Form, LensId, LensSpec, QueryDescription, ScopeDescription,
    SearchRecord,
};
use analysant_rust::{CodeIndex, Reach, Relation, Site, Verdict};

pub const LENS: &str = "doc-vs-code";

/// The kinds, in the order the lens declares them. One string so the labels are data, not word literals.
const KIND_LABELS: &str = "absent tests-only uncalled dead-callers shared-name";

/// What every sentence says about how far its claim reaches.
const SCOPE_TAIL: &str = "matching is exact and case-sensitive, uses are counted by bare name, one hop up the chain, with no call graph";

/// The finding kinds this lens can produce, as `FindingKind::label` strings.
pub fn kinds() -> Vec<String> {
    KIND_LABELS.split_whitespace().map(str::to_string).collect()
}

fn kind(i: usize) -> String {
    KIND_LABELS
        .split_whitespace()
        .nth(i)
        .unwrap_or_default()
        .to_string()
}

/// What the doc-vs-code lens declares about itself (SPEC-STEP1 4.1).
pub fn spec() -> LensSpec {
    LensSpec {
        id: LensId(LENS.to_string()),
        description: "A code item a document names that the tree does not define, defines only in tests, or defines but nothing calls; reach is by bare name and one hop, not a call graph."
            .into(),
        requires: Vec::new(),
        cost: Cost::Cheap,
        reads_code: true,
        kinds: kinds(),
    }
}

#[derive(Clone, Debug)]
pub enum DocCodeStatus {
    /// Defined in production and called: nothing to say.
    Resolved,
    Finding(Box<Finding>),
    /// Would have been a finding; withheld because no control could vouch for the search.
    Withheld {
        reason: String,
    },
}

#[derive(Clone, Debug)]
pub struct DocCodeOutcome {
    /// Position in `Corpus::claims()` order.
    pub index: usize,
    /// The name looked up, in the form `CodeIndex::relate` takes.
    pub query: String,
    pub status: DocCodeStatus,
}

#[derive(Clone, Debug)]
pub struct DocCodeRun {
    /// Rust files in the index.
    pub files: usize,
    /// Items in the index: the population searched.
    pub items: usize,
    /// One outcome per claim that names a code item, in reading order.
    pub outcomes: Vec<DocCodeOutcome>,
}

impl DocCodeRun {
    pub fn findings(&self) -> impl Iterator<Item = (usize, &Finding)> {
        self.outcomes.iter().filter_map(|o| match &o.status {
            DocCodeStatus::Finding(f) => Some((o.index, f.as_ref())),
            _ => None,
        })
    }

    /// Claims that name a code item.
    pub fn named(&self) -> usize {
        self.outcomes.len()
    }

    pub fn withheld(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o.status, DocCodeStatus::Withheld { .. }))
            .count()
    }

    pub fn resolved(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o.status, DocCodeStatus::Resolved))
            .count()
    }

    /// This run in the shape `health_report` reads. Claims that name no code item are not this lens's population.
    pub fn output(&self) -> LensOutput {
        LensOutput {
            spec: spec(),
            findings: self.findings().map(|(_, f)| f.clone()).collect(),
            population: self.outcomes.len() as u64,
            withheld: self.withheld() as u64,
            set_aside: Vec::new(),
            not_analysed: Vec::new(),
            null_checks: Vec::new(),
        }
    }
}

/// The forms that name a thing; a list item or a sentence is never read as a code name.
fn names_a_thing(form: &Form) -> bool {
    matches!(form, Form::CodeSpan | Form::Bold | Form::TableCell)
}

fn site_text(s: &Site) -> String {
    format!("{}:{}", s.file, s.line)
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// The index of the finding kind a verdict is reported as, or `None` when it is called and says nothing.
fn classify(v: &Verdict) -> Option<usize> {
    match (&v.relation, &v.reach) {
        (Relation::Missing, _) => Some(0),
        (Relation::Tests, _) => Some(1),
        (Relation::Implements, Some(Reach::Uncalled { .. })) => Some(2),
        (Relation::Implements, Some(Reach::OnlyFromUncalled { .. })) => Some(3),
        (Relation::Implements, Some(Reach::CalledIfSharedNameResolves { .. })) => Some(4),
        (Relation::Implements, _) => None,
    }
}

fn statement(
    claim_text: &str,
    place: &str,
    query: &str,
    v: &Verdict,
    files: usize,
    class: usize,
) -> String {
    let head = format!("\"{claim_text}\" ({place}) names `{query}`");
    let defined_at = v
        .definitions
        .first()
        .map(site_text)
        .unwrap_or_else(|| "-".into());
    let body = match (class, &v.reach) {
        (0, _) => format!(", and no definition of it is in the {files} Rust files searched"),
        (1, _) if v.definitions.is_empty() => {
            ", which no file defines and which is used only in test code".to_string()
        }
        (1, _) => format!(", which is defined or used only in test code ({defined_at})"),
        (2, Some(Reach::Uncalled { test_uses })) => format!(
            ", which is defined in production code ({defined_at}) but has no production caller; {test_uses} test use{}",
            plural(*test_uses)
        ),
        (3, Some(Reach::OnlyFromUncalled { callers, total })) => {
            let who: Vec<String> = callers
                .iter()
                .filter_map(|c| c.within.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            format!(
                ", whose {total} production caller{} ({}) nothing calls in production",
                plural(*total),
                if who.is_empty() {
                    "at the top level".to_string()
                } else {
                    who.join(&[',', ' '].iter().collect::<String>())
                }
            )
        }
        (
            4,
            Some(Reach::CalledIfSharedNameResolves {
                sharing_definitions,
                ..
            }),
        ) => format!(
            "; its production callers are reached only by bare name, and that name shares its name with {sharing_definitions} other definition{}, so the call may be to one of them",
            plural(*sharing_definitions)
        ),
        _ => String::new(),
    };
    format!("{head}{body}; {SCOPE_TAIL}")
}

fn definition_ref(v: &Verdict) -> String {
    v.definitions
        .first()
        .map_or_else(|| "-".to_string(), site_text)
}

/// The text a finding's `refs` column carries: where the definition is, or `-`.
pub fn code_ref(f: &Finding) -> String {
    f.derivation
        .first()
        .and_then(|d| d.table_entries.first().cloned())
        .unwrap_or_else(|| "-".into())
}

/// Check every claim that names a code item against `index`.
pub fn doc_vs_code(corpus: &Corpus, index: &CodeIndex) -> DocCodeRun {
    let claims: Vec<_> = corpus.claims().collect();
    let named: Vec<(usize, String)> = claims
        .iter()
        .enumerate()
        .filter(|(_, (_, c))| names_a_thing(&c.form))
        .filter_map(|(i, (_, c))| index.code_reference(&c.text).map(|q| (i, q)))
        .collect();
    let verdicts: Vec<Verdict> = named.iter().map(|(_, q)| index.relate(q)).collect();
    let files = index.file_count();
    let items = index.item_count();

    // A control is another claim that the same index resolves to a production definition that is called.
    let control_pos = |subject: usize| -> Option<usize> {
        named.iter().enumerate().position(|(p, (i, _))| {
            *i != subject
                && verdicts[p].relation == Relation::Implements
                && matches!(verdicts[p].reach, Some(Reach::Called { .. }))
        })
    };

    let lens = LensId(LENS.to_string());
    let mut outcomes = Vec::with_capacity(named.len());
    for (p, (i, query)) in named.iter().enumerate() {
        let (doc, claim) = claims[*i];
        let v = &verdicts[p];
        let Some(class) = classify(v) else {
            outcomes.push(DocCodeOutcome {
                index: *i,
                query: query.clone(),
                status: DocCodeStatus::Resolved,
            });
            continue;
        };
        let withheld = |reason: &str| DocCodeOutcome {
            index: *i,
            query: query.clone(),
            status: DocCodeStatus::Withheld {
                reason: reason.to_string(),
            },
        };
        if items == 0 {
            outcomes.push(withheld(
                "no Rust items were indexed, so a name cannot be told absent from the tree",
            ));
            continue;
        }
        let Some(cp) = control_pos(*i) else {
            outcomes.push(withheld(
                "no other claim of the corpus resolves to a called production definition, so there is no control",
            ));
            continue;
        };
        let (control_claim_index, control_query) = &named[cp];
        let control = ControlResult {
            kind: ControlKind::KnownPresent,
            item: ClaimRef::current(claims[*control_claim_index].1.unit.id),
            outcome: if index.relate(control_query).relation == Relation::Implements {
                ControlOutcome::Found
            } else {
                ControlOutcome::NotFound
            },
        };
        if control.outcome == ControlOutcome::NotFound {
            outcomes.push(withheld("the control claim no longer resolves"));
            continue;
        }
        let claim_ref = ClaimRef::current(claim.unit.id);
        let kind_label = kind(class);
        let line = claim.unit.source.span.line_start;
        let place = format!("{}:{line}", doc.path);
        let mut excluded: Vec<(String, u64)> = Vec::new();
        for (what, n) in [
            ("mentions in comments", v.mentions.comment),
            ("mentions in production code", v.mentions.code_production),
            ("mentions in test code", v.mentions.code_test),
        ] {
            if class == 0 && n > 0 {
                excluded.push((what.to_string(), n as u64));
            }
        }
        let search = SearchRecord {
            looked_in: ScopeDescription(format!("{files} Rust files, {items} items")),
            population: items as u64,
            query: QueryDescription {
                key: query.clone(),
                mode: "exact name, case-sensitive; uses counted by bare name".into(),
                aliases_applied: Vec::new(),
            },
            matched: v.definitions.len() as u64,
            excluded,
        };
        outcomes.push(DocCodeOutcome {
            index: *i,
            query: query.clone(),
            status: DocCodeStatus::Finding(Box::new(Finding {
                id: FindingId::compute(&lens, &kind_label, &[claim_ref], query),
                lens: lens.clone(),
                kind: FindingKind::Other(kind_label.clone()),
                claims: vec![claim_ref],
                statement: statement(&claim.text, &place, query, v, files, class),
                derivation: vec![Derivation {
                    lens: lens.clone(),
                    rule: format!(
                        "CodeIndex::relate({query}): {:?}, reach {:?}",
                        v.relation,
                        v.reach.as_ref().map(reach_label)
                    ),
                    table_entries: vec![definition_ref(v)],
                }],
                basis: Basis::Deterministic,
                search,
                control,
                disposition: Disposition::Open,
            })),
        });
    }
    DocCodeRun {
        files,
        items,
        outcomes,
    }
}

/// The reach as a label for the derivation: the words of the reach's own name, from one string of labels.
const REACH_LABELS: &str = "called called-if-shared-name-resolves only-from-uncalled uncalled";

fn reach_label(r: &Reach) -> &'static str {
    let i = match r {
        Reach::Called { .. } => 0,
        Reach::CalledIfSharedNameResolves { .. } => 1,
        Reach::OnlyFromUncalled { .. } => 2,
        Reach::Uncalled { .. } => 3,
    };
    REACH_LABELS.split_whitespace().nth(i).unwrap_or_default()
}
