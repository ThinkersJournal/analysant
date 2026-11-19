// SPDX-License-Identifier: MIT OR Apache-2.0
//! The cross-document term index: normalized token to entries. It replaces the reference spike's
//! scan of every claim against every line (SPEC-STEP1 3.2) and returns the search record with
//! every answer, so a null always comes with its denominator.

use crate::matching::{MatchEvidence, MatchMode, matches};
use analysant_core::{
    Claim, ClaimRef, NormalForm, QueryDescription, ScopeDescription, SearchRecord,
};
use std::collections::HashMap;

pub struct TermIndex<K> {
    label: String,
    entries: Vec<(K, NormalForm)>,
    postings: HashMap<String, Vec<usize>>,
}

pub struct QueryResult<K> {
    /// Matching entries in insertion order, each with the evidence that matched it.
    pub hits: Vec<(K, MatchEvidence)>,
    pub search: SearchRecord,
}

impl<K: Clone> TermIndex<K> {
    /// Index `items`. `label` names what the index covers, for the search record.
    pub fn build(label: &str, items: impl IntoIterator<Item = (K, NormalForm)>) -> Self {
        let entries: Vec<(K, NormalForm)> = items.into_iter().collect();
        let mut postings: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, (_, nf)) in entries.iter().enumerate() {
            for t in &nf.tokens {
                let list = postings.entry(t.norm.clone()).or_default();
                // Entries are visited in order, so a repeat token is always the last item.
                if list.last() != Some(&idx) {
                    list.push(idx);
                }
            }
        }
        Self {
            label: label.to_string(),
            entries,
            postings,
        }
    }

    /// How many entries are in scope: the denominator of every null.
    pub fn population(&self) -> usize {
        self.entries.len()
    }

    pub fn query(&self, needle: &NormalForm, mode: MatchMode) -> QueryResult<K> {
        let hits = self
            .candidates(needle)
            .into_iter()
            .filter_map(|idx| {
                let (key, hay) = &self.entries[idx];
                matches(hay, needle, mode).map(|ev| (key.clone(), ev))
            })
            .collect::<Vec<_>>();
        let search = SearchRecord {
            looked_in: ScopeDescription(self.label.clone()),
            population: self.entries.len() as u64,
            query: QueryDescription {
                key: needle.key.clone(),
                mode: mode.name().to_string(),
                aliases_applied: Vec::new(),
            },
            matched: hits.len() as u64,
            excluded: Vec::new(),
        };
        QueryResult { hits, search }
    }

    /// Entries holding every needle token: a superset of what any mode can match.
    fn candidates(&self, needle: &NormalForm) -> Vec<usize> {
        if needle.tokens.is_empty() {
            return Vec::new();
        }
        let mut lists: Vec<&Vec<usize>> = Vec::new();
        for t in &needle.tokens {
            match self.postings.get(&t.norm) {
                Some(l) => lists.push(l),
                None => return Vec::new(),
            }
        }
        lists.sort_by_key(|l| l.len());
        let mut result: Vec<usize> = lists[0].clone();
        for other in &lists[1..] {
            result.retain(|i| other.binary_search(i).is_ok());
            if result.is_empty() {
                break;
            }
        }
        result
    }
}

impl TermIndex<ClaimRef> {
    /// Index claims by their normal forms.
    pub fn from_claims(label: &str, claims: &[Claim]) -> Self {
        Self::build(
            label,
            claims
                .iter()
                .map(|c| (ClaimRef::current(c.unit.id), c.normal.clone())),
        )
    }
}
