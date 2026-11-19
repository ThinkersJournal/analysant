// SPDX-License-Identifier: MIT OR Apache-2.0
//! Alias canonicalization: a flat alias layer first (SPEC-STEP1 slice 1). The scoped, sensed,
//! TOML-stored concept layers belong to `analysant-semantic`.
//!
//! The file form is the reference spike's: one group per line, `a | b | c`, first phrase canonical,
//! `#` comments. The replacement rule (every alias phrase, as a token subsequence, becomes the
//! canonical phrase; groups in file order) is the spike's, credited, `MIT OR Apache-2.0`.

use analysant_core::{LanguagePack, NormalForm, Span, Token, TokenClass, normalize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasGroup {
    /// The canonical phrase's tokens (folded).
    pub canonical: Vec<String>,
    /// Each alias phrase's tokens (folded).
    pub aliases: Vec<Vec<String>>,
}

impl AliasGroup {
    /// The canonical phrase, for records.
    pub fn name(&self) -> String {
        self.canonical.join(" ")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AliasTable {
    groups: Vec<AliasGroup>,
}

/// A canonicalized phrase and which groups changed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canonical {
    pub form: NormalForm,
    /// The canonical names of the groups that fired, in firing order, once each.
    pub applied: Vec<String>,
}

impl AliasTable {
    /// Parse the alias file format with the pack that will read the text it is applied to.
    pub fn parse(pack: &dyn LanguagePack, text: &str) -> Self {
        let mut groups = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let phrases: Vec<Vec<String>> = line
                .split('|')
                .map(|p| {
                    normalize(pack, p)
                        .tokens
                        .into_iter()
                        .map(|t| t.norm)
                        .collect::<Vec<_>>()
                })
                .filter(|p| !p.is_empty())
                .collect();
            if phrases.len() >= 2 {
                groups.push(AliasGroup {
                    canonical: phrases[0].clone(),
                    aliases: phrases[1..].to_vec(),
                });
            }
        }
        Self { groups }
    }

    pub fn groups(&self) -> &[AliasGroup] {
        &self.groups
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Replace every alias phrase in `nf` with its group's canonical phrase.
    pub fn canonicalize(&self, pack: &dyn LanguagePack, nf: &NormalForm) -> Canonical {
        let mut toks: Vec<Token> = nf.tokens.clone();
        let mut applied: Vec<String> = Vec::new();
        for group in &self.groups {
            for alias in &group.aliases {
                let mut i = 0;
                while i + alias.len() <= toks.len() {
                    let hit = toks[i..i + alias.len()]
                        .iter()
                        .zip(alias)
                        .all(|(t, a)| t.norm == *a);
                    if !hit {
                        i += 1;
                        continue;
                    }
                    let start = toks[i].span.start;
                    let end = toks[i + alias.len() - 1].span.end;
                    let canon: Vec<Token> = group
                        .canonical
                        .iter()
                        .map(|w| Token {
                            text: w.clone(),
                            span: start..end,
                            norm: w.clone(),
                            class: TokenClass::Word,
                        })
                        .collect();
                    let advance = canon.len();
                    toks.splice(i..i + alias.len(), canon);
                    i += advance;
                    let name = group.name();
                    if !applied.contains(&name) {
                        applied.push(name);
                    }
                }
            }
        }
        let norm = pack.normalizer();
        Canonical {
            form: NormalForm::from_tokens(toks, |w| norm.is_stopword(w)),
            applied,
        }
    }
}

/// Tokenize text with a pack. The span is the whole text.
pub fn tokenize(pack: &dyn LanguagePack, text: &str) -> Vec<Token> {
    pack.tokenize(
        &Span {
            start: 0,
            end: text.len(),
            ..Span::default()
        },
        text,
    )
}
