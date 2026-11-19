// SPDX-License-Identifier: MIT OR Apache-2.0
//! Word-boundary phrase lookup against a lexicon. Never substring: "shall" must not hit "shallow"
//! (the reference spike's rule, credited). Matching is on whole tokens, so the boundary is the
//! tokenizer's, not a character test.

use analysant_core::{LanguagePack, NormalForm, normalize};
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconHit {
    pub lexicon: String,
    /// Index of the entry in its lexicon.
    pub entry: usize,
    /// Byte range in the text `text` was made from.
    pub span: Range<usize>,
}

/// Every occurrence of every entry of `lexicon` in `text`, entry by entry, left to right.
pub fn lookup(pack: &dyn LanguagePack, lexicon: &str, text: &NormalForm) -> Vec<LexiconHit> {
    let mut hits = Vec::new();
    for (entry, phrase) in pack.lexicons().entries(lexicon).iter().enumerate() {
        let want = normalize(pack, phrase);
        let n = want.tokens.len();
        if n == 0 || n > text.tokens.len() {
            continue;
        }
        for w in text.tokens.windows(n) {
            if w.iter().zip(&want.tokens).all(|(a, b)| a.norm == b.norm) {
                hits.push(LexiconHit {
                    lexicon: lexicon.to_string(),
                    entry,
                    span: w[0].span.start..w[n - 1].span.end,
                });
            }
        }
    }
    hits
}
