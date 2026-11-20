// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant-semantic`: which *thing* a term names, in this context (SPEC-STEP1 3.4, 3.5).
//!
//! A [`ConceptStore`] holds concepts (a canonical name, other forms of it, and, for an overloaded word,
//! a sense table: each sense has cues that must co-occur and cues that exclude it), in layers
//! (language pack < portfolio < project < document; a later layer replaces an earlier concept of the
//! same id). It resolves a term to a concept and, where the word is overloaded, to one sense or to
//! "ambiguous", never to a guess; it canonicalizes text through the forms; it relates two terms from the
//! tables alone. Everything here is deterministic and says which table entries it used.
//!
//! **Scope, stated.** This is the *consumer's shape* of a concept store (SPEC-STEP1 section 3, PM notice
//! 2026-11-18): the field libraries (SemanticAnt, OntologicAnt) own semantics proper, and this crate may
//! become a thin adapter over SemanticAnt. It therefore has no taxonomy: `Broader` and `Narrower` are not
//! produced, and an unrecorded relation is `Undecided`, not `Unrelated`. `subject_of` (what a claim is
//! about) waits for its first user, the contradiction lens (slice 4).
//!
//! **File form.** Plain lines (`# analysant-concepts v1`), diffable and commentable; the TOML of 3.5
//! waits until a slice needs fields the lines cannot carry (the same ruling as slice 1's lexicons):
//!
//! ```text
//! concept budget
//! status: active
//! sense budget-tolerance | numerical error tolerance | cues: tolerance, epsilon | excludes: vram
//! sense budget-resource | per-device resource limit | cues: vram, memory, device
//! evidence: spike-report-2026-11-16.md:334
//! test not-same: budget tolerance | budget memory
//! ```

use analysant_core::{LanguagePack, NormalForm, Span, TableError, Token, normalize};
use analysant_lexical::{AliasTable, Canonical};
use semanticant_core::{Reason, Verdict};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConceptId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SenseId(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Active,
    /// Proposed by a tool; affects nothing until a human sets it active (3.5, point 3).
    Proposed,
    Retired,
}

/// A phrase as folded tokens.
type Phrase = Vec<String>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sense {
    pub id: SenseId,
    pub gloss: String,
    pub cues: Vec<Phrase>,
    pub excludes: Vec<Phrase>,
}

/// An executable check on the table itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConceptTest {
    pub same: bool,
    pub phrases: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Concept {
    pub id: ConceptId,
    /// The layer it came from.
    pub layer: String,
    pub canonical: Phrase,
    pub forms: Vec<Phrase>,
    pub senses: Vec<Sense>,
    pub status: Status,
    pub evidence: Vec<String>,
    pub tests: Vec<ConceptTest>,
}

impl Concept {
    fn names(&self) -> impl Iterator<Item = &Phrase> {
        std::iter::once(&self.canonical).chain(self.forms.iter())
    }
}

/// One layer's text. Layers are loaded lowest precedence first.
#[derive(Clone, Debug)]
pub struct LayerSource {
    pub name: String,
    pub text: String,
}

/// What surrounds a term: the folded words near it. Cues are matched here, whole words only.
#[derive(Clone, Debug, Default)]
pub struct Context {
    pub words: Vec<String>,
}

impl Context {
    pub fn from_text(pack: &dyn LanguagePack, text: &str) -> Self {
        Self {
            words: phrase_of(pack, text),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// The term names this concept, in this sense when the concept has a sense table.
    Concept {
        concept: ConceptId,
        sense: Option<SenseId>,
        /// The layer the concept came from, for the finding's record.
        layer: String,
    },
    /// The concept is overloaded and the context does not pick one sense (none matched, or several).
    Ambiguous {
        concept: ConceptId,
        senses: Vec<SenseId>,
    },
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TermRelation {
    Same,
    Broader,
    Narrower,
    Related,
    Unrelated,
}

/// A table test that failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestFailure {
    pub concept: ConceptId,
    pub layer: String,
    pub reason: String,
}

pub struct ConceptStore<'a> {
    pack: &'a dyn LanguagePack,
    concepts: Vec<Concept>,
}

fn tokens_of(pack: &dyn LanguagePack, text: &str) -> Vec<Token> {
    pack.tokenize(
        &Span {
            start: 0,
            end: text.len(),
            ..Span::default()
        },
        text,
    )
}

/// The folded words of a phrase. Not `normalize`: that drops a leading article, which a cue or a form
/// may legitimately be.
fn phrase_of(pack: &dyn LanguagePack, text: &str) -> Phrase {
    tokens_of(pack, text).into_iter().map(|t| t.norm).collect()
}

fn contains_phrase(words: &[String], phrase: &[String]) -> bool {
    !phrase.is_empty()
        && phrase.len() <= words.len()
        && words.windows(phrase.len()).any(|w| w == phrase)
}

impl<'a> ConceptStore<'a> {
    /// Load layers, lowest precedence first. A concept id in a later layer replaces the earlier one
    /// whole (a layer states what it means; it does not patch).
    pub fn load(pack: &'a dyn LanguagePack, layers: &[LayerSource]) -> Result<Self, TableError> {
        let mut concepts: Vec<Concept> = Vec::new();
        for layer in layers {
            for c in parse_layer(pack, layer)? {
                match concepts.iter().position(|e| e.id == c.id) {
                    Some(i) => concepts[i] = c,
                    None => concepts.push(c),
                }
            }
        }
        Ok(Self { pack, concepts })
    }

    pub fn concepts(&self) -> &[Concept] {
        &self.concepts
    }

    fn active(&self) -> impl Iterator<Item = &Concept> {
        self.concepts.iter().filter(|c| c.status == Status::Active)
    }

    /// Which concept and sense a term names, given what surrounds it.
    pub fn resolve(&self, term: &NormalForm, ctx: &Context) -> Resolution {
        // The term as written, and as its key (leading article dropped).
        let written: Phrase = term.tokens.iter().map(|t| t.norm.clone()).collect();
        let key: Phrase = term.key.split(' ').map(str::to_string).collect();
        let Some(c) = self
            .active()
            .find(|c| c.names().any(|n| *n == written || *n == key))
        else {
            return Resolution::Unknown;
        };
        self.resolve_concept(c, ctx)
    }

    fn resolve_concept(&self, c: &Concept, ctx: &Context) -> Resolution {
        if c.senses.is_empty() {
            return Resolution::Concept {
                concept: c.id.clone(),
                sense: None,
                layer: c.layer.clone(),
            };
        }
        let picked: Vec<&Sense> = c
            .senses
            .iter()
            .filter(|s| {
                s.cues.iter().any(|q| contains_phrase(&ctx.words, q))
                    && !s.excludes.iter().any(|q| contains_phrase(&ctx.words, q))
            })
            .collect();
        match picked.as_slice() {
            [one] => Resolution::Concept {
                concept: c.id.clone(),
                sense: Some(one.id.clone()),
                layer: c.layer.clone(),
            },
            // No cue matched: every sense is still possible. Several matched: those are.
            [] => Resolution::Ambiguous {
                concept: c.id.clone(),
                senses: c.senses.iter().map(|s| s.id.clone()).collect(),
            },
            many => Resolution::Ambiguous {
                concept: c.id.clone(),
                senses: many.iter().map(|s| s.id.clone()).collect(),
            },
        }
    }

    /// Replace every form of an active concept in `text` with the concept's canonical phrase, the same
    /// rule as the flat alias layer. A concept whose senses the context does not settle is left alone:
    /// merging two senses is the dangerous direction (it removes findings).
    pub fn canonicalize(&self, text: &NormalForm, ctx: &Context) -> Canonical {
        let mut lines = String::new();
        for c in self.active().filter(|c| !c.forms.is_empty()) {
            if !matches!(self.resolve_concept(c, ctx), Resolution::Concept { .. }) {
                continue;
            }
            lines.push_str(&c.canonical.join(" "));
            for f in &c.forms {
                lines.push_str(" | ");
                lines.push_str(&f.join(" "));
            }
            lines.push('\n');
        }
        AliasTable::parse(self.pack, &lines).canonicalize(self.pack, text)
    }

    /// How two terms relate, from the tables alone, both read in one context. See [`Self::relate_in`].
    pub fn relate(
        &self,
        a: &NormalForm,
        b: &NormalForm,
        ctx: &Context,
    ) -> Verdict<Option<TermRelation>> {
        self.relate_in(a, ctx, b, ctx)
    }

    /// How two terms relate, each read in its own context. The value is `Some(relation)` when the
    /// tables decide: `Same` for the same concept in the same sense (or the same words in no table),
    /// `Unrelated` for two senses of one overloaded word. It is `None`, with the reason, when the tables
    /// do not say: undecided is never "unrelated". The reasons are the rule that fired.
    pub fn relate_in(
        &self,
        a: &NormalForm,
        ca: &Context,
        b: &NormalForm,
        cb: &Context,
    ) -> Verdict<Option<TermRelation>> {
        let (ra, rb) = (self.resolve(a, ca), self.resolve(b, cb));
        match (&ra, &rb) {
            (
                Resolution::Concept {
                    concept: x,
                    sense: sx,
                    layer,
                },
                Resolution::Concept {
                    concept: y,
                    sense: sy,
                    ..
                },
            ) if x == y => {
                if sx == sy {
                    Verdict::new(Some(TermRelation::Same)).because(Reason::new(
                        "same-concept",
                        format!("both are concept {} (layer {layer})", x.0),
                    ))
                } else {
                    Verdict::new(Some(TermRelation::Unrelated)).because(Reason::new(
                        "different-senses",
                        format!(
                            "two senses of {}: {:?} and {:?}",
                            x.0,
                            sx.as_ref().map(|s| &s.0),
                            sy.as_ref().map(|s| &s.0)
                        ),
                    ))
                }
            }
            (Resolution::Unknown, Resolution::Unknown) if a.key == b.key => {
                Verdict::new(Some(TermRelation::Same))
                    .because(Reason::new("same-words", "the same words, in no table"))
            }
            (Resolution::Ambiguous { concept, .. }, _)
            | (_, Resolution::Ambiguous { concept, .. }) => {
                Verdict::new(None).because(Reason::new(
                    "ambiguous-sense",
                    format!(
                        "{} is overloaded and the context does not pick a sense",
                        concept.0
                    ),
                ))
            }
            _ => Verdict::new(None)
                .because(Reason::new("no-relation", "the tables record no relation")),
        }
    }

    /// Run every entry's tests: `same` phrases must canonicalize to one form, `not-same` phrases must
    /// all differ. A table is tested like code (3.5, point 2).
    pub fn run_tests(&self) -> Vec<TestFailure> {
        let ctx = Context::default();
        let mut out = Vec::new();
        for c in &self.concepts {
            for t in &c.tests {
                let forms: Vec<String> = t
                    .phrases
                    .iter()
                    .map(|p| self.canonicalize(&normalize(self.pack, p), &ctx).form.key)
                    .collect();
                let all_equal = forms.windows(2).all(|w| w[0] == w[1]);
                let any_equal = forms
                    .iter()
                    .enumerate()
                    .any(|(i, f)| forms[i + 1..].contains(f));
                if if t.same { !all_equal } else { any_equal } {
                    out.push(TestFailure {
                        concept: c.id.clone(),
                        layer: c.layer.clone(),
                        reason: format!(
                            "{} {:?} canonicalized to {:?}",
                            if t.same { "same" } else { "not-same" },
                            t.phrases,
                            forms
                        ),
                    });
                }
            }
        }
        out
    }
}

fn parse_layer(pack: &dyn LanguagePack, layer: &LayerSource) -> Result<Vec<Concept>, TableError> {
    let err = |line: usize, reason: &str| TableError {
        table: layer.name.clone(),
        line,
        reason: reason.to_string(),
    };
    let phrases = |s: &str| -> Vec<Phrase> {
        s.split(['|', ','])
            .map(|p| phrase_of(pack, p))
            .filter(|p| !p.is_empty())
            .collect()
    };
    let mut out: Vec<Concept> = Vec::new();
    for (idx, raw) in layer.text.lines().enumerate() {
        let n = idx + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(id) = line.strip_prefix("concept ") {
            let id = id.trim();
            if id.is_empty() {
                return Err(err(n, "a concept needs an id"));
            }
            if out.iter().any(|c| c.id.0 == id) {
                return Err(err(n, "duplicate concept id in one layer"));
            }
            out.push(Concept {
                id: ConceptId(id.to_string()),
                layer: layer.name.clone(),
                canonical: phrase_of(pack, id),
                forms: Vec::new(),
                senses: Vec::new(),
                status: Status::Active,
                evidence: Vec::new(),
                tests: Vec::new(),
            });
            continue;
        }
        let Some(c) = out.last_mut() else {
            return Err(err(n, "a line before any `concept <id>`"));
        };
        if let Some(v) = line.strip_prefix("canonical:") {
            let p = phrase_of(pack, v);
            if p.is_empty() {
                return Err(err(n, "empty canonical"));
            }
            c.canonical = p;
        } else if let Some(v) = line.strip_prefix("forms:") {
            c.forms = phrases(v);
        } else if let Some(v) = line.strip_prefix("status:") {
            c.status = match v.trim() {
                "active" => Status::Active,
                "proposed" => Status::Proposed,
                "retired" => Status::Retired,
                _ => return Err(err(n, "status is active, proposed or retired")),
            };
        } else if let Some(v) = line.strip_prefix("evidence:") {
            c.evidence.push(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("sense ") {
            let mut parts = v.split('|').map(str::trim);
            let id = parts.next().unwrap_or("");
            let gloss = parts.next().unwrap_or("");
            if id.is_empty() || gloss.is_empty() {
                return Err(err(n, "a sense is `sense <id> | <gloss> | cues: ...`"));
            }
            let mut sense = Sense {
                id: SenseId(id.to_string()),
                gloss: gloss.to_string(),
                cues: Vec::new(),
                excludes: Vec::new(),
            };
            for p in parts {
                if let Some(q) = p.strip_prefix("cues:") {
                    sense.cues = phrases(q);
                } else if let Some(q) = p.strip_prefix("excludes:") {
                    sense.excludes = phrases(q);
                } else {
                    return Err(err(n, "a sense field is `cues:` or `excludes:`"));
                }
            }
            if sense.cues.is_empty() {
                return Err(err(n, "a sense needs at least one cue"));
            }
            c.senses.push(sense);
        } else if let Some(v) = line.strip_prefix("test same:") {
            c.tests.push(test(true, v));
        } else if let Some(v) = line.strip_prefix("test not-same:") {
            c.tests.push(test(false, v));
        } else {
            return Err(err(n, "unknown line"));
        }
    }
    Ok(out)
}

fn test(same: bool, v: &str) -> ConceptTest {
    ConceptTest {
        same,
        phrases: v.split('|').map(|p| p.trim().to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysant_core::English;

    const PORTFOLIO: &str = "\
# analysant-concepts v1
concept multi path dag
forms: multi-path graph
test same: multi-path graph | multi-path DAG
test not-same: path graph | multi-path DAG

concept budget
sense budget-tolerance | numerical error tolerance | cues: tolerance, epsilon | excludes: vram
sense budget-resource | per-device resource limit | cues: vram, memory, device
";

    fn store(en: &English) -> ConceptStore<'_> {
        ConceptStore::load(
            en,
            &[LayerSource {
                name: "portfolio".into(),
                text: PORTFOLIO.into(),
            }],
        )
        .unwrap()
    }

    fn nf(en: &English, s: &str) -> NormalForm {
        normalize(en, s)
    }

    #[test]
    fn a_form_canonicalizes_and_the_table_tests_pass() {
        let en = English::new();
        let s = store(&en);
        let c = s.canonicalize(&nf(&en, "the multi-path graph"), &Context::default());
        assert_eq!(c.form.key, "multi path dag");
        assert_eq!(c.applied, ["multi path dag"]);
        assert_eq!(s.run_tests(), []);
    }

    #[test]
    fn a_failing_table_test_is_reported() {
        let en = English::new();
        let text = "concept x\nforms: y\ntest not-same: x | y\n";
        let s = ConceptStore::load(
            &en,
            &[LayerSource {
                name: "p".into(),
                text: text.into(),
            }],
        )
        .unwrap();
        assert_eq!(s.run_tests().len(), 1);
    }

    #[test]
    fn an_overloaded_word_resolves_by_cue_or_stays_ambiguous() {
        let en = English::new();
        let s = store(&en);
        let t = nf(&en, "budget");
        let of = |text: &str| s.resolve(&t, &Context::from_text(&en, text));
        match of("the rounding tolerance of the epsilon") {
            Resolution::Concept { sense, .. } => assert_eq!(sense.unwrap().0, "budget-tolerance"),
            other => panic!("{other:?}"),
        }
        match of("the vram a device has") {
            Resolution::Concept { sense, .. } => assert_eq!(sense.unwrap().0, "budget-resource"),
            other => panic!("{other:?}"),
        }
        // No cue: ambiguous, with every sense. Both cues: ambiguous, with those two.
        assert!(
            matches!(of("nothing relevant"), Resolution::Ambiguous { senses, .. } if senses.len() == 2)
        );
        // An exclude knocks a sense out: "tolerance" cues one, "vram" excludes it and cues the other.
        match of("tolerance and vram") {
            Resolution::Concept { sense, .. } => assert_eq!(sense.unwrap().0, "budget-resource"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            s.resolve(&nf(&en, "unlisted"), &Context::default()),
            Resolution::Unknown
        );
    }

    #[test]
    fn two_senses_of_one_word_are_unrelated_and_unknown_is_undecided() {
        let en = English::new();
        let s = store(&en);
        let t = nf(&en, "budget");
        let tol = Context::from_text(&en, "tolerance");
        let mem = Context::from_text(&en, "memory");
        // Same word, different contexts: resolve each side in its own context, then compare.
        let (a, b) = (s.resolve(&t, &tol), s.resolve(&t, &mem));
        assert_ne!(a, b);
        let v = s.relate(&t, &t, &tol);
        assert_eq!(v.value, Some(TermRelation::Same));
        assert_eq!(v.reasons[0].rule, "same-concept");
        // Two senses of one word, each read in its own context: unrelated, and it says why.
        let v = s.relate_in(&t, &tol, &t, &mem);
        assert_eq!(v.value, Some(TermRelation::Unrelated));
        assert_eq!(v.reasons[0].rule, "different-senses");
        // Undecided is None with a reason, never Unrelated.
        let v = s.relate(&t, &nf(&en, "harvest"), &tol);
        assert_eq!((v.value, v.reasons[0].rule), (None, "no-relation"));
        let v = s.relate(&t, &t, &Context::default());
        assert_eq!((v.value, v.reasons[0].rule), (None, "ambiguous-sense"));
        let foo = nf(&en, "foo bar");
        let v = s.relate(&foo, &foo, &Context::default());
        assert_eq!(
            (v.value, v.reasons[0].rule),
            (Some(TermRelation::Same), "same-words")
        );
    }

    #[test]
    fn a_later_layer_replaces_a_concept_whole_and_proposed_entries_do_nothing() {
        let en = English::new();
        let s = ConceptStore::load(
            &en,
            &[
                LayerSource {
                    name: "portfolio".into(),
                    text: "concept alpha beta\nforms: x y\n".into(),
                },
                LayerSource {
                    name: "project".into(),
                    text: "concept alpha beta\nforms: p q\nconcept gamma delta\nstatus: proposed\nforms: eps zeta\n"
                        .into(),
                },
            ],
        )
        .unwrap();
        let ctx = Context::default();
        assert_eq!(s.canonicalize(&nf(&en, "x y"), &ctx).form.key, "x y");
        assert_eq!(s.canonicalize(&nf(&en, "p q"), &ctx).form.key, "alpha beta");
        assert_eq!(
            s.canonicalize(&nf(&en, "eps zeta"), &ctx).form.key,
            "eps zeta"
        );
        assert_eq!(
            s.concepts()
                .iter()
                .find(|c| c.id.0 == "alpha beta")
                .unwrap()
                .layer,
            "project"
        );
    }

    #[test]
    fn malformed_tables_name_the_line() {
        let en = English::new();
        let bad = |text: &str| {
            ConceptStore::load(
                &en,
                &[LayerSource {
                    name: "t".into(),
                    text: text.into(),
                }],
            )
            .err()
            .expect("refused")
        };
        assert_eq!(bad("forms: a\n").line, 1);
        assert_eq!(bad("concept a\nsense s | g\n").line, 2);
        assert_eq!(bad("concept a\nstatus: maybe\n").line, 2);
        assert_eq!(bad("concept a\nconcept a\n").line, 2);
    }
}
