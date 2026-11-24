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
    /// Extensions of the tracked files: a token ending in one of these names a file.
    extensions: HashSet<String>,
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
            extensions: tracked
                .iter()
                .filter_map(|p| p.rsplit_once('.'))
                .map(|(_, e)| e.to_lowercase())
                .collect(),
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

    /// `absent_artifact` (the round 5 narrow check): an unticked box in a live document outside the non-work documents that names an
    /// artifact absent from the tree. A path artifact is absent when neither the path nor its file name is tracked; a symbol artifact
    /// (a backticked identifier) is absent when `symbol_absent` says so. A step naming no artifact is not a finding.
    pub fn keeps_absent_artifact(
        &self,
        pack: &dyn LanguagePack,
        path: &str,
        text: &str,
        symbol_absent: &dyn Fn(&str) -> bool,
    ) -> bool {
        let lex = pack.lexicons();
        let lower = text.trim_start_matches(['-', '*', '+', ' ']).to_lowercase();
        if !lex
            .entries("checkbox-openers")
            .first()
            .is_some_and(|b| lower.starts_with(b.as_str()))
        {
            return false;
        }
        let path_l = path.replace('\\', "/").to_lowercase();
        if ["non-work-documents", "template-documents"]
            .iter()
            .any(|l| lex.entries(l).iter().any(|e| path_l.contains(e.as_str())))
            || self.closed.get(path).copied().unwrap_or(false)
            || lex
                .entries("optional-step-markers")
                .iter()
                .any(|m| lower.contains(m.as_str()))
        {
            return false;
        }
        path_artifacts(&lower, &self.extensions).iter().any(|t| {
            !self.paths.contains(t) && !self.names.contains(t.rsplit('/').next().unwrap_or(t))
        }) || symbol_artifacts(text)
            .iter()
            .any(|s| symbol_absent(s.rsplit_once(':').map_or(s.as_str(), |(_, last)| last)))
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

/// Tokens of a step that name a file: they hold `/`, or end in a known file extension.
fn path_artifacts(lower: &str, extensions: &HashSet<String>) -> Vec<String> {
    lower
        .split(|c: char| {
            c.is_whitespace() || matches!(c, '`' | '(' | ')' | ',' | ';' | '"' | '\'' | '[' | ']')
        })
        .map(|t| t.trim_matches(['.', ':', '*']).trim_start_matches("./"))
        .filter(|t| {
            !t.contains("://")
                && !t.starts_with('/')
                && !t.starts_with('~')
                && !t.starts_with('$')
                && !t.starts_with('%')
                && !t.contains(":/")
                && ((t.contains('/') && t.chars().any(|c| c.is_alphabetic()))
                    || t.rsplit_once('.')
                        .is_some_and(|(stem, ext)| !stem.is_empty() && extensions.contains(ext)))
        })
        .map(str::to_string)
        .collect()
}

/// Backticked identifiers that look like code (`snake_case`, `a::b`, `camelCase`), not paths, words or commands.
fn symbol_artifacts(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .filter(|s| {
            s.len() >= 4
                && !s.contains('/')
                && s.chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
                && (s.contains('_')
                    || s.matches(':').count() >= 2
                    || s.chars()
                        .zip(s.chars().skip(1))
                        .any(|(a, b)| a.is_lowercase() && b.is_uppercase()))
        })
        .map(str::to_string)
        .collect()
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
    fn absent_artifact_fires_only_for_a_live_unticked_step_naming_a_missing_artifact() {
        let en = English::new();
        let tracked: BTreeSet<&str> = ["src/api/chat.rs", "docs/plan.md"].into_iter().collect();
        let files = vec![
            ("docs/plan.md".to_string(), "# Plan\n- [ ] x".to_string()),
            (
                "docs/old.md".to_string(),
                "# Old plan (superseded)\n".to_string(),
            ),
        ];
        let ctx = LiveContext::new(&en, &tracked, &files, &[]);
        let none_missing = |_: &str| false;
        let all_missing = |_: &str| true;
        let fires = |p: &str, t: &str, sym_absent: &dyn Fn(&str) -> bool| {
            ctx.keeps_absent_artifact(&en, p, t, sym_absent)
        };
        // A step naming a missing file in a live plan fires.
        assert!(fires(
            "docs/plan.md",
            "- [ ] add `src/api/export.rs`",
            &none_missing
        ));
        // A step naming an existing file (by path or by bare name) does not.
        assert!(!fires(
            "docs/plan.md",
            "- [ ] extend `src/api/chat.rs`",
            &none_missing
        ));
        assert!(!fires(
            "docs/plan.md",
            "- [ ] extend chat.rs",
            &none_missing
        ));
        // The same missing-file step in a superseded document does not.
        assert!(!fires(
            "docs/old.md",
            "- [ ] add `src/api/export.rs`",
            &none_missing
        ));
        // A ticked box, or a step naming no artifact, does not.
        assert!(!fires(
            "docs/plan.md",
            "- [x] add `src/api/export.rs`",
            &none_missing
        ));
        assert!(!fires("docs/plan.md", "- [ ] improve docs", &all_missing));
        // A missing symbol fires; a present one does not; a plain word or version is no artifact.
        assert!(fires(
            "docs/plan.md",
            "- [ ] implement `export_metrics`",
            &all_missing
        ));
        assert!(!fires(
            "docs/plan.md",
            "- [ ] implement `export_metrics`",
            &none_missing
        ));
        assert!(!fires(
            "docs/plan.md",
            "- [ ] bump to v1.2 e.g. `cache`",
            &all_missing
        ));
    }

    /// Census 497A false classes (docs/GATE-B-CENSUS-497A.md), one red-first test each; every census false positive is quoted.
    #[test]
    fn drift_skips_the_census_false_classes() {
        let en = English::new();
        let tracked: BTreeSet<&str> = ["docs/plan.md"].into_iter().collect();
        let files = vec![(
            "docs/plan.md".to_string(),
            "# Plan
- [ ] x"
                .to_string(),
        )];
        let ctx = LiveContext::new(&en, &tracked, &files, &[]);
        let none_missing = |_: &str| false;
        let fires = |p: &str, t: &str, sym_absent: &dyn Fn(&str) -> bool| {
            ctx.keeps_absent_artifact(&en, p, t, sym_absent)
        };
        // Control: the same shape of step fires in a plain plan.
        assert!(fires(
            "docs/plan.md",
            "- [ ] Add `src/export.rs` for the exporter",
            &none_missing
        ));
        // A PR template or other checklist file is not a plan (auth-framework PULL_REQUEST_TEMPLATE.md).
        for p in [
            ".github/PULL_REQUEST_TEMPLATE.md",
            ".github/ISSUE_TEMPLATE/bug.md",
            "docs/release-checklist.md",
        ] {
            assert!(
                !fires(
                    p,
                    "- [ ] Add `src/export.rs` for the exporter",
                    &none_missing
                ),
                "{p}"
            );
        }
        // An optional step (fuel "(Optional, local GPU only) Mamba decode bench").
        assert!(!fires(
            "docs/plan.md",
            "- [ ] **Step 3: (Optional, local GPU only) bench.** run `benches/decode.rs`. Skip if no device",
            &none_missing
        ));
        // A TDD red step expects the artifact to be missing (fuel "Run, watch fail ... Expected: FAIL").
        assert!(!fires(
            "docs/plan.md",
            "- [ ] **Step 3: Run, watch fail.** Expected: FAIL (`Tensor::scan_builder` undefined).",
            &|_: &str| true
        ));
        // A path outside the repository is scratch, not a tracked artifact (KISS `C:/Projects/KISS/.superpowers/sdd/progress.md`).
        for t in [
            "- [ ] Update the ledger at `C:/Projects/KISS/.superpowers/sdd/progress.md` with outcomes",
            "- [ ] Copy to `~/notes/progress.md`",
            "- [ ] Write `/tmp/out/progress.md`",
        ] {
            assert!(!fires("docs/plan.md", t, &none_missing), "{t}");
        }
    }

    /// fuel `Tensor::scan`: a qualified Rust path never appears literally in source, so it is searched by its final segment.
    #[test]
    fn a_qualified_symbol_is_searched_by_its_final_segment() {
        let en = English::new();
        let tracked: BTreeSet<&str> = ["docs/plan.md"].into_iter().collect();
        let files = vec![(
            "docs/plan.md".to_string(),
            "# Plan
- [ ] x"
                .to_string(),
        )];
        let ctx = LiveContext::new(&en, &tracked, &files, &[]);
        // The source defines `scan` but never the text `Tensor::scan`.
        let only_scan_exists = |s: &str| s != "scan";
        assert!(!ctx.keeps_absent_artifact(
            &en,
            "docs/plan.md",
            "- [ ] Add `Tensor::scan` to the graph crate",
            &only_scan_exists
        ));
        // A qualified symbol whose final segment is also absent still fires.
        assert!(ctx.keeps_absent_artifact(
            &en,
            "docs/plan.md",
            "- [ ] Add `Tensor::unscan` to the graph crate",
            &only_scan_exists
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
