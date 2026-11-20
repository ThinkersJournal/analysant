// SPDX-License-Identifier: MIT OR Apache-2.0
//! The doc-vs-code relation (SPEC-STEP1 5.1, "verdict vocabulary"): `Implements`, `Tests` or `Missing`,
//! with the evidence and the scope of the reachability claim attached. A report template, not the
//! engine, maps these onto a project's own words (built, stub, absent).

use crate::index::{CodeIndex, CodeItem, CodeItemKind, Context, ItemId, Use};
use crate::pack::split_ident_words;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    /// A production definition exists. `Verdict::reach` says whether anything calls it.
    Implements,
    /// No production definition; a definition or use exists only in test code.
    Tests,
    /// No definition anywhere. `Verdict::mentions` says whether the name is still in code or comments.
    Missing,
}

/// How far a reachability claim reaches. There is one scope today and it is named so no report can
/// read it as more: uses are counted by bare name, and the chain is followed one hop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    NameBasedOneHop,
}

/// A place in the code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub file: String,
    pub line: u32,
    pub context: Context,
    /// The definition's qualified name, or for a use the item that holds it.
    pub within: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reach {
    /// A production use sits at the top level or in an item that is itself used in production under
    /// a name only it has (or is an entry point or a trait-impl method, whose callers cannot be seen).
    Called { callers: Vec<Site>, total: usize },
    /// Every live production caller is live only because its *bare name* is used in production, and
    /// other definitions share that name: the call may be to one of them. With no call graph this
    /// cannot be settled, so it is reported as unsettled rather than as `Called` or dead.
    CalledIfSharedNameResolves {
        callers: Vec<Site>,
        total: usize,
        sharing_definitions: usize,
    },
    /// Every production use sits in an item that nothing uses in production: dead code calling it.
    OnlyFromUncalled { callers: Vec<Site>, total: usize },
    /// No production use. `test_uses` counts the uses in test code.
    Uncalled { test_uses: usize },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mentions {
    /// Occurrences in production code other than a definition or a `use` line.
    pub code_production: usize,
    pub code_test: usize,
    /// Occurrences in comments, doc comments included.
    pub comment: usize,
    pub comment_sites: Vec<Site>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub query: String,
    pub relation: Relation,
    pub scope: Scope,
    /// Production definitions first, then test ones; at most `max_listed_sites` of each.
    pub definitions: Vec<Site>,
    pub reach: Option<Reach>,
    pub mentions: Mentions,
}

impl CodeIndex {
    /// The relation for a name: `pick_route`, or `Type::member` to pin a method, variant or field.
    /// Matching is exact and case-sensitive; uses are counted by the last segment.
    pub fn relate(&self, query: &str) -> Verdict {
        let sep = self.pack().path_separator();
        let (owner, name) = match query.rsplit_once(sep) {
            Some((o, n)) => (Some(o.rsplit(sep).next().unwrap_or(o)), n),
            None => (None, query),
        };
        let defs: Vec<ItemId> = self
            .by_name
            .get(name)
            .map(|ids| {
                ids.iter()
                    .copied()
                    .filter(|id| {
                        owner.is_none_or(|o| {
                            self.items[id.0]
                                .qualified
                                .rsplit_once(sep)
                                .is_some_and(|(q, _)| q == o)
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.verdict(query, name, &defs)
    }

    /// The relation for every identifier whose words are exactly `words` (`pick route` finds
    /// `pick_route` and `PickRoute`); the strongest relation among them wins.
    pub fn relate_words(&self, words: &[String]) -> Verdict {
        let wanted = words.join(" ");
        let mut best: Option<Verdict> = None;
        for name in self.by_name.keys() {
            if split_ident_words(name).join(" ") == wanted {
                let v = self.relate(name);
                let rank = |r: Relation| match r {
                    Relation::Implements => 0,
                    Relation::Tests => 1,
                    Relation::Missing => 2,
                };
                let better = best
                    .as_ref()
                    .is_none_or(|b| (rank(v.relation), &v.query) < (rank(b.relation), &b.query));
                if better {
                    best = Some(v);
                }
            }
        }
        best.unwrap_or_else(|| self.relate(&wanted))
    }

    fn verdict(&self, query: &str, name: &str, defs: &[ItemId]) -> Verdict {
        let max = self.pack().max_listed_sites();
        let prod_defs: Vec<ItemId> = defs
            .iter()
            .copied()
            .filter(|d| self.items[d.0].context == Context::Production)
            .collect();
        let test_defs: Vec<ItemId> = defs
            .iter()
            .copied()
            .filter(|d| self.items[d.0].context == Context::Test)
            .collect();
        let kinds: Vec<CodeItemKind> = defs.iter().map(|d| self.items[d.0].kind).collect();
        let all_uses = self.uses.get(name).map(Vec::as_slice).unwrap_or(&[]);
        let counted: Vec<&Use> = all_uses
            .iter()
            .filter(|u| kinds.is_empty() || kinds.iter().any(|k| counts(*k, u)))
            .filter(|u| !u.enclosing.is_some_and(|e| defs.contains(&e)))
            .collect();
        let prod_uses: Vec<&Use> = counted
            .iter()
            .copied()
            .filter(|u| u.context == Context::Production)
            .collect();
        let test_uses = counted.len() - prod_uses.len();

        let comment_hits = self.comments.get(name).map(Vec::as_slice).unwrap_or(&[]);
        let mentions = Mentions {
            code_production: prod_uses.len(),
            code_test: test_uses,
            comment: comment_hits.len(),
            comment_sites: comment_hits
                .iter()
                .take(max)
                .map(|(f, l)| Site {
                    file: self.files[*f as usize].clone(),
                    line: *l,
                    context: Context::Production,
                    within: None,
                })
                .collect(),
        };
        let site_of_def = |d: &ItemId| {
            let it = &self.items[d.0];
            Site {
                file: self.files[it.file as usize].clone(),
                line: it.line,
                context: it.context,
                within: Some(it.qualified.clone()),
            }
        };
        let mut definitions: Vec<Site> = prod_defs.iter().take(max).map(site_of_def).collect();
        definitions.extend(test_defs.iter().take(max).map(site_of_def));

        let (relation, reach) = if !prod_defs.is_empty() {
            (
                Relation::Implements,
                Some(self.reach(&prod_uses, test_uses, max)),
            )
        } else if !test_defs.is_empty() || (test_uses > 0 && prod_uses.is_empty()) {
            (Relation::Tests, None)
        } else {
            (Relation::Missing, None)
        };
        Verdict {
            query: query.to_string(),
            relation,
            scope: Scope::NameBasedOneHop,
            definitions,
            reach,
            mentions,
        }
    }

    fn reach(&self, prod_uses: &[&Use], test_uses: usize, max: usize) -> Reach {
        if prod_uses.is_empty() {
            return Reach::Uncalled { test_uses };
        }
        let site = |u: &Use| Site {
            file: self.files[u.file as usize].clone(),
            line: u.line,
            context: u.context,
            within: u.enclosing.map(|e| self.items[e.0].qualified.clone()),
        };
        let mut certain: Vec<&Use> = Vec::new();
        let mut shared: Vec<&Use> = Vec::new();
        let mut sharing = 0;
        for u in prod_uses {
            match u.enclosing.map_or(Liveness::Certain, |e| self.liveness(e)) {
                Liveness::Certain => certain.push(u),
                Liveness::Shared(n) => {
                    sharing = sharing.max(n);
                    shared.push(u);
                }
                Liveness::Dead => {}
            }
        }
        let list = |us: &[&Use]| us.iter().take(max).map(|u| site(u)).collect::<Vec<_>>();
        if !certain.is_empty() {
            Reach::Called {
                callers: list(&certain),
                total: certain.len(),
            }
        } else if !shared.is_empty() {
            Reach::CalledIfSharedNameResolves {
                callers: list(&shared),
                total: shared.len(),
                sharing_definitions: sharing,
            }
        } else {
            Reach::OnlyFromUncalled {
                callers: prod_uses.iter().take(max).map(|u| site(u)).collect(),
                total: prod_uses.len(),
            }
        }
    }

    /// Whether an item is called: an entry point or a trait-impl method is (its callers cannot be
    /// seen); otherwise it needs a production use of its bare name, of the kind that counts for it,
    /// from outside its own body. When other production definitions carry the same bare name that use
    /// may belong to one of them, and the answer is `Shared`.
    fn liveness(&self, id: ItemId) -> Liveness {
        let it: &CodeItem = &self.items[id.0];
        if it.trait_impl || self.pack().is_entry_point(&it.name) {
            return Liveness::Certain;
        }
        let used = self.uses.get(&it.name).is_some_and(|us| {
            us.iter().any(|u| {
                u.context == Context::Production && counts(it.kind, u) && u.enclosing != Some(id)
            })
        });
        if !used {
            return Liveness::Dead;
        }
        let others = self.by_name.get(&it.name).map_or(0, |ids| {
            ids.iter()
                .filter(|o| **o != id && self.items[o.0].context == Context::Production)
                .count()
        });
        if others == 0 {
            Liveness::Certain
        } else {
            Liveness::Shared(others)
        }
    }
}

enum Liveness {
    Certain,
    Shared(usize),
    Dead,
}

/// Which occurrences count as a use of an item of this kind: a field is read with a dot and no
/// call, a function is any occurrence that is not a dotted field read, everything else any occurrence.
fn counts(kind: CodeItemKind, u: &Use) -> bool {
    match kind {
        CodeItemKind::Field => u.dot && !u.call,
        CodeItemKind::Fn => !(u.dot && !u.call),
        _ => true,
    }
}
