// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant-lexical`: what are the words, and do these words match. Text in, hits out; it knows no
//! documents. Deterministic by contract (SPEC-STEP1 3.3). See docs/SPEC-STEP1.md section 3.

pub mod alias;
pub mod index;
pub mod lookup;
pub mod matching;

pub use alias::{AliasGroup, AliasTable, Canonical, tokenize};
pub use analysant_core::{NormalForm, Token, TokenClass, normalize};
pub use index::{QueryResult, TermIndex};
pub use lookup::{LexiconHit, lookup};
pub use matching::{MatchEvidence, MatchMode, matches};

#[cfg(test)]
mod tests {
    use super::*;
    use analysant_core::English;

    fn nf(en: &English, s: &str) -> NormalForm {
        normalize(en, s)
    }

    #[test]
    fn subset_ignores_order_sequence_does_not() {
        let en = English::new();
        let hay = nf(&en, "the multi path DAG is built");
        let key = nf(&en, "DAG multi path");
        assert!(matches(&hay, &key, MatchMode::TokenSubset).is_some());
        assert!(matches(&hay, &key, MatchMode::Sequence).is_none());
        assert!(matches(&hay, &nf(&en, "multi path"), MatchMode::Sequence).is_some());
    }

    #[test]
    fn empty_needle_matches_nothing() {
        let en = English::new();
        let hay = nf(&en, "anything");
        assert!(matches(&hay, &nf(&en, ""), MatchMode::TokenSubset).is_none());
        assert!(matches(&hay, &nf(&en, "the"), MatchMode::Sequence).is_none());
    }

    #[test]
    fn evidence_names_the_matched_tokens() {
        let en = English::new();
        let hay = nf(&en, "alpha beta gamma");
        let ev = matches(&hay, &nf(&en, "gamma alpha"), MatchMode::TokenSubset).unwrap();
        assert_eq!(ev.matched, vec![11..16, 0..5]);
        assert_eq!(ev.mode.name(), "token-subset");
    }

    #[test]
    fn alias_canonicalizes_phrases_and_reports_the_group() {
        let en = English::new();
        let table = AliasTable::parse(
            &en,
            "# c\nmulti path DAG | multi-path graph | graph of paths\n",
        );
        assert_eq!(table.groups().len(), 1);
        let got = table.canonicalize(&en, &nf(&en, "The loaded multi-path graph state"));
        assert_eq!(got.form.key, "loaded multi path dag state");
        assert_eq!(got.applied, ["multi path dag"]);
        let untouched = table.canonicalize(&en, &nf(&en, "a path graph"));
        assert_eq!(untouched.form.key, "path graph");
        assert!(untouched.applied.is_empty());
    }

    #[test]
    fn alias_lines_with_one_phrase_are_not_groups() {
        let en = English::new();
        assert!(AliasTable::parse(&en, "only one\n\n# x\n |  | \n").is_empty());
    }

    #[test]
    fn lookup_is_word_bounded() {
        let en = English::new();
        let mut set = analysant_core::LexiconSet::default();
        set.add_lexicon("modals", "shall\nmust not\n");
        // A pack with this lexicon: wrap English by building the lookup over its tokenizer.
        struct P<'a>(&'a English, analysant_core::LexiconSet);
        impl analysant_core::LanguagePack for P<'_> {
            fn id(&self) -> analysant_core::PackId {
                self.0.id()
            }
            fn script_kind(&self) -> analysant_core::LanguageKind {
                self.0.script_kind()
            }
            fn tokenize(&self, s: &analysant_core::Span, t: &str) -> Vec<Token> {
                self.0.tokenize(s, t)
            }
            fn sentences(&self, t: &str) -> Vec<std::ops::Range<usize>> {
                self.0.sentences(t)
            }
            fn normalizer(&self) -> &dyn analysant_core::Normalizer {
                self.0.normalizer()
            }
            fn lexicons(&self) -> &analysant_core::LexiconSet {
                &self.1
            }
            fn extract(&self, b: &analysant_core::Block) -> Vec<analysant_core::RawClaim> {
                self.0.extract(b)
            }
        }
        let p = P(&en, set);
        let text = normalize(&p, "It shall pass; a shallow check. You must not fail");
        let hits = lookup(&p, "modals", &text);
        let spans: Vec<(usize, Range<usize>)> =
            hits.iter().map(|h| (h.entry, h.span.clone())).collect();
        assert_eq!(spans, vec![(0, 3..8), (1, 36..44)]);
    }

    use std::ops::Range;

    #[test]
    fn index_agrees_with_a_brute_force_scan() {
        let en = English::new();
        let lines = [
            "alpha beta",
            "beta gamma delta",
            "gamma",
            "alpha gamma beta",
            "",
            "delta delta alpha",
            "beta alpha",
        ];
        let items: Vec<(usize, NormalForm)> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| (i, nf(&en, l)))
            .collect();
        let index = TermIndex::build("test lines", items.clone());
        for needle in [
            "alpha beta",
            "gamma",
            "delta alpha",
            "zeta",
            "",
            "beta alpha gamma",
        ] {
            let n = nf(&en, needle);
            for mode in [MatchMode::TokenSubset, MatchMode::Sequence] {
                let want: Vec<usize> = items
                    .iter()
                    .filter(|(_, h)| matches(h, &n, mode).is_some())
                    .map(|(i, _)| *i)
                    .collect();
                let got = index.query(&n, mode);
                let ids: Vec<usize> = got.hits.iter().map(|(i, _)| *i).collect();
                assert_eq!(ids, want, "needle {needle:?} mode {mode:?}");
                assert_eq!(got.search.matched as usize, want.len());
                assert_eq!(got.search.population, lines.len() as u64);
            }
        }
    }
}
