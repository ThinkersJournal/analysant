// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gate B round 4 selections (docs/GATE-B-ROUND4-DESIGN.md), both OFF by default and applied after the lenses, so the lenses and
//! Gate A are untouched: `live_open_boxes` for the untasked findings, `minimal_pair` for the contradiction pairs.

use analysant_core::{LanguagePack, normalize};
use std::collections::{BTreeSet, HashMap, HashSet};

/// What `live_open_boxes` knows about the repository: tracked paths, which documents are closed, and the plan's content words.
pub struct LiveContext {
    paths: HashSet<String>,
    names: HashSet<String>,
    closed: HashMap<String, bool>,
    plan: Vec<HashSet<String>>,
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

impl LiveContext {
    pub fn new(
        pack: &dyn LanguagePack,
        tracked: &BTreeSet<&str>,
        files: &[(String, String)],
        plan_lines: &[String],
    ) -> Self {
        let lex = pack.lexicons();
        let markers = lex.entries("closed-document-markers");
        let status = lex.entries("status-line-words");
        let closed = files
            .iter()
            .map(|(path, text)| (path.clone(), is_closed(text, markers, status)))
            .collect();
        let plan = plan_lines
            .iter()
            .map(|l| normalize(pack, l).content.into_iter().collect())
            .collect();
        Self {
            paths: tracked.iter().map(|p| p.to_lowercase()).collect(),
            names: tracked
                .iter()
                .filter_map(|p| p.rsplit('/').next())
                .map(str::to_lowercase)
                .collect(),
            closed,
            plan,
        }
    }

    /// Whether the document opens with a closed-document marker (superseded, complete, ...).
    pub fn is_closed_doc(&self, path: &str) -> bool {
        self.closed.get(path).copied().unwrap_or(false)
    }

    /// Is the claim an unticked box in a live document, with no named file that exists, and not half-covered by one plan line.
    pub fn keeps(&self, pack: &dyn LanguagePack, path: &str, text: &str, key: &str) -> bool {
        let lex = pack.lexicons();
        let lower = text.trim_start_matches(['-', '*', '+', ' ']).to_lowercase();
        let open_box = lex.entries("checkbox-openers").first();
        if !open_box.is_some_and(|b| lower.starts_with(b.as_str())) {
            return false;
        }
        let path_l = path.replace('\\', "/").to_lowercase();
        if lex
            .entries("non-work-documents")
            .iter()
            .any(|e| path_l.contains(e.as_str()))
            || self.closed.get(path).copied().unwrap_or(false)
        {
            return false;
        }
        if self.names_existing_file(&lower) {
            return false;
        }
        let key_words: Vec<&str> = key
            .split_whitespace()
            .filter(|w| !pack.normalizer().is_stopword(w))
            .collect();
        if key_words.len() >= 2 {
            let need = key_words.len().div_ceil(2).max(2);
            let covered = self
                .plan
                .iter()
                .any(|line| key_words.iter().filter(|w| line.contains(**w)).count() >= need);
            if covered {
                return false;
            }
        }
        true
    }

    /// A token of the item that looks like a path or file name (it holds `/`, or ends in a dot and 1-5 letters or digits) and
    /// resolves to a tracked path or to the name of one.
    fn names_existing_file(&self, text: &str) -> bool {
        text.split(|c: char| {
            c.is_whitespace() || matches!(c, '`' | '(' | ')' | ',' | ';' | '"' | '\'')
        })
        .map(|t| t.trim_matches(['.', ':', '*']).trim_start_matches("./"))
        .filter(|t| t.contains('/') || has_extension(t))
        .any(|t| self.paths.contains(t) || self.names.contains(t.rsplit('/').next().unwrap_or(t)))
    }
}

fn has_extension(t: &str) -> bool {
    t.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty()
            && (1..=5).contains(&ext.len())
            && ext.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

/// A closed-document marker in a heading, a quote line or a status line, within the first 20 lines.
fn is_closed(text: &str, markers: &[String], status: &[String]) -> bool {
    text.lines().take(20).any(|l| {
        let t = l.trim_start();
        let w = words(t);
        (t.starts_with('#') || t.starts_with('>') || w.iter().any(|x| status.contains(x)))
            && w.iter().any(|x| markers.contains(x))
    })
}

/// Two verb forms of one verb: one begins with the other, and the shorter is at least three letters.
fn same_verb(a: &str, b: &str) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    short.len() >= 3 && long.starts_with(short)
}

/// `minimal_pair`: the negated side is `<head> <marker> <verb> <object>`; the positive side holds the same head words, the same verb
/// (to its stem) and the same object words within six words after the verb. Both are single clauses in live documents. A refusal
/// marker with no verb (`no <capability>`) is not a pair.
pub fn minimal_pair(
    pack: &dyn LanguagePack,
    negated: &str,
    negated_by: Option<&str>,
    positive: &str,
    live: impl Fn(bool) -> bool,
) -> bool {
    if !live(true) || !live(false) {
        return false;
    }
    let Some(marker) = negated_by else {
        return false;
    };
    if marker.ends_with("<capability>") {
        return false;
    }
    let norm = pack.normalizer();
    let joiners = pack.lexicons().entries("clause-joiners");
    let neg = words(negated);
    let pos = words(positive);
    if neg.iter().chain(&pos).any(|w| joiners.contains(w)) {
        return false;
    }
    let m = words(marker);
    let Some(at) = neg.windows(m.len()).position(|w| w == m.as_slice()) else {
        return false;
    };
    let head: Vec<&String> = neg[..at].iter().filter(|w| !norm.is_stopword(w)).collect();
    let mut rest = neg[at + m.len()..].iter().filter(|w| !norm.is_stopword(w));
    let (Some(verb), object) = (rest.next(), rest.take(3).collect::<Vec<_>>()) else {
        return false;
    };
    if head.len() < 2 || object.is_empty() {
        return false;
    }
    if !head.iter().all(|h| pos.contains(h)) {
        return false;
    }
    let Some(v_at) = pos.iter().position(|w| same_verb(w, verb)) else {
        return false;
    };
    let window = &pos[v_at + 1..pos.len().min(v_at + 7)];
    object.iter().all(|o| window.contains(o))
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysant_core::English;

    fn pair(neg: &str, by: &str, pos: &str) -> bool {
        minimal_pair(&English::new(), neg, Some(by), pos, |_| true)
    }

    #[test]
    fn minimal_pair_needs_the_same_head_verb_and_object() {
        assert!(pair(
            "The job scheduler does not support streaming responses.",
            "does not",
            "The job scheduler supports streaming responses for every job."
        ));
        // Same head and object, a different verb: not a pair.
        assert!(!pair(
            "The job scheduler does not support streaming responses.",
            "does not",
            "The job scheduler falls back to streaming responses."
        ));
        // A clause joiner: not a single clause.
        assert!(!pair(
            "The job scheduler does not support streaming responses.",
            "does not",
            "The job scheduler supports streaming responses but slowly."
        ));
        // A bare label has no head.
        assert!(!pair(
            "Not supported streaming responses",
            "not",
            "Streaming responses supported"
        ));
    }

    #[test]
    fn a_closed_or_missing_side_is_not_a_pair() {
        assert!(!minimal_pair(
            &English::new(),
            "The job scheduler does not support streaming responses.",
            Some("does not"),
            "The job scheduler supports streaming responses.",
            |neg_side| neg_side
        ));
    }

    #[test]
    fn live_open_boxes_keeps_only_an_unticked_box_that_is_live_undone_and_uncovered() {
        let en = English::new();
        let tracked: BTreeSet<&str> = ["src/api/chat.rs", "docs/plan.md"].into_iter().collect();
        let files = vec![
            ("docs/plan.md".to_string(), "# Plan\n- [ ] x".to_string()),
            (
                "docs/old.md".to_string(),
                "# Old plan (superseded)\n".to_string(),
            ),
        ];
        let plan = vec!["wire the elastic cache into scheduler".to_string()];
        let ctx = LiveContext::new(&en, &tracked, &files, &plan);
        let keep = |p: &str, t: &str, k: &str| ctx.keeps(&en, p, t, k);
        assert!(keep(
            "docs/plan.md",
            "- [ ] add exporter metrics",
            "add exporter metrics"
        ));
        assert!(!keep(
            "docs/plan.md",
            "- [x] add exporter metrics",
            "add exporter metrics"
        ));
        assert!(!keep(
            "docs/plan.md",
            "The exporter must add metrics",
            "exporter add metrics"
        ));
        assert!(!keep(
            "docs/old.md",
            "- [ ] add exporter metrics",
            "add exporter metrics"
        ));
        assert!(!keep(
            "CHANGELOG.md",
            "- [ ] add exporter metrics",
            "add exporter metrics"
        ));
        assert!(!keep(
            "docs/plan.md",
            "- [ ] format `chat.rs` output",
            "format chat output"
        ));
        assert!(!keep(
            "docs/plan.md",
            "- [ ] cache elastic scheduler",
            "cache elastic scheduler"
        ));
    }
}
