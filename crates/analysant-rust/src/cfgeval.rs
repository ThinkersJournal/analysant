// SPDX-License-Identifier: MIT OR Apache-2.0
//! Evaluating a `cfg` predicate for one question: is this code compiled in a production build?
//! The test predicate is false; every other predicate (a feature, a target) is assumed true, because
//! the scan cannot know the build, and assuming "absent" would hide production code.

use crate::pack::RustPack;
use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// True when the predicate (the tokens inside `cfg( ... )`) holds in a production build.
pub(crate) fn holds_in_production(pack: &RustPack, tokens: TokenStream) -> bool {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    eval(pack, &trees)
}

fn eval(pack: &RustPack, trees: &[TokenTree]) -> bool {
    match trees {
        [TokenTree::Ident(id)] => !pack.is_test_predicate(&id.to_string()),
        [TokenTree::Ident(id), TokenTree::Group(g)] if g.delimiter() == Delimiter::Parenthesis => {
            let name = id.to_string();
            let operands = split_commas(g.stream());
            let results: Vec<bool> = operands.iter().map(|o| eval(pack, o)).collect();
            if pack.is_cfg_all(&name) {
                results.iter().all(|r| *r)
            } else if pack.is_cfg_any(&name) {
                results.iter().any(|r| *r)
            } else if pack.is_cfg_not(&name) {
                !results.first().copied().unwrap_or(true)
            } else {
                true
            }
        }
        _ => true,
    }
}

fn split_commas(tokens: TokenStream) -> Vec<Vec<TokenTree>> {
    let mut out: Vec<Vec<TokenTree>> = vec![Vec::new()];
    for tt in tokens {
        match &tt {
            TokenTree::Punct(p) if p.as_char() == ',' => out.push(Vec::new()),
            _ => out.last_mut().expect("never empty").push(tt),
        }
    }
    out.retain(|v| !v.is_empty());
    out
}
