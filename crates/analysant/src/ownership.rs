// SPDX-License-Identifier: MIT OR Apache-2.0
//! Note-to-plan-row ownership (the spike's genre handling; SPEC-STEP1 section 6, slice 3).
//!
//! A note that elaborates a plan row is not a source of untasked work: its details are tasked through
//! the row, even though the row's one line does not contain their words. Ownership is explicit, declared
//! in the documents themselves, in one of two ways:
//!
//! (a) the note's level-1 title names the row ("Row 157", "plan row 137 step 6.3b"), and that row exists
//!     in the plan (a note planning for a row that does not exist is still untasked work);
//! (b) a plan row cites the note's path (`<citation prefix><file>`); the owner is the lowest-numbered
//!     citing row. The citing line need not be the row's own line: the owner is the nearest preceding
//!     row-start line.
//!
//! Everything that names a project's convention is configuration (SPEC-STEP1 2.5 item 5): the word a title
//! uses for a row, what a row-start line looks like, how a plan cites a note. The engine hard-codes none
//! of them; `OwnershipConfig` has no `Default` for that reason.
//!
//! Ownership is a file-level property, separate from a claim's pragmatic role. An owned file's claims keep
//! their roles; only absences that would otherwise be findings move to the owned list. Absences already
//! set aside stay set aside, where their role is the more specific reason.

use crate::corpus::Corpus;
use crate::untasked::UntaskedRun;
use analysant_core::{BlockKind, Finding, FormatAdapter, LanguagePack, Span};
use analysant_pragmatic::is_reportable;
use std::collections::HashMap;

/// A project's ownership conventions (what `.analysant/project.toml` will carry).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnershipConfig {
    /// The word a title uses to name a plan row, compared case-folded by the pack ("row").
    pub row_word: String,
    /// What follows the row number on a row-start line, exactly (`". **"`: a period, then bold text).
    /// A bare number is not enough: numbered sub-lists inside row bodies are not rows.
    pub row_start: String,
    /// What precedes a note's file name where the plan cites it (`"notes/"`).
    pub citation_prefix: String,
}

/// The plan row that owns each note, keyed by the note's path as given to `owners`.
pub type Owners = HashMap<String, usize>;

/// Decide ownership for every `(path, text)` in `files` against `plan` (the plan's lines).
pub fn owners(
    format: &dyn FormatAdapter,
    pack: &dyn LanguagePack,
    config: &OwnershipConfig,
    plan: &[String],
    files: &[(String, String)],
) -> Owners {
    files
        .iter()
        .filter_map(|(path, text)| {
            owner_for(format, pack, config, plan, path, text).map(|row| (path.clone(), row))
        })
        .collect()
}

/// The plan row that owns the note `path`, if either signal fires. Where a title and a citation
/// disagree, the title's row wins (a self-declared plan is checked against the row first).
pub fn owner_for(
    format: &dyn FormatAdapter,
    pack: &dyn LanguagePack,
    config: &OwnershipConfig,
    plan: &[String],
    path: &str,
    text: &str,
) -> Option<usize> {
    if let Some(title) = first_title(format, text) {
        if let Some(n) = title_row(pack, config, &title) {
            if plan.iter().any(|l| row_number(config, l) == Some(n)) {
                return Some(n);
            }
        }
    }
    let needle = format!("{}{path}", config.citation_prefix);
    let mut current: Option<usize> = None;
    let mut best: Option<usize> = None;
    for line in plan {
        if let Some(n) = row_number(config, line) {
            current = Some(n);
        }
        if line.contains(&needle) {
            if let Some(n) = current {
                best = Some(best.map_or(n, |b: usize| b.min(n)));
            }
        }
    }
    best
}

/// The first level-1 heading of a document with a non-empty title.
fn first_title(format: &dyn FormatAdapter, text: &str) -> Option<String> {
    format.blocks(text).into_iter().find_map(|b| {
        (b.kind == BlockKind::Heading { level: 1 } && !b.text.trim().is_empty())
            .then(|| b.text.trim().to_string())
    })
}

/// The row a title names: the row word followed by a number, or the row word with the number attached.
/// A bare row word in prose names nothing.
fn title_row(pack: &dyn LanguagePack, config: &OwnershipConfig, title: &str) -> Option<usize> {
    let word = pack.normalizer().fold_case(&config.row_word);
    let tokens = pack.tokenize(&Span::default(), title);
    for (i, t) in tokens.iter().enumerate() {
        if t.norm == word {
            if let Some(n) = tokens.get(i + 1).and_then(|next| next.norm.parse().ok()) {
                return Some(n);
            }
        } else if let Some(rest) = t.norm.strip_prefix(&word) {
            if !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()) {
                if let Ok(n) = rest.parse() {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// A row-start line: digits, then exactly `config.row_start`.
fn row_number(config: &OwnershipConfig, line: &str) -> Option<usize> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || !line[digits..].starts_with(&config.row_start) {
        return None;
    }
    line[..digits].parse().ok()
}

/// A run split three ways: findings, set aside by the pragmatic lens, and owned by a plan row.
pub struct OwnedPartition<'a> {
    /// Absent, reportable, in a note no row owns: the findings.
    pub findings: Vec<(usize, &'a Finding)>,
    /// Absent but not doing what its form claims (owned or not: the role is the more specific reason).
    pub set_aside: Vec<(usize, &'a Finding)>,
    /// Absent and reportable, but in a note a plan row owns: tasked through that row, with the row.
    pub owned: Vec<(usize, &'a Finding, usize)>,
}

/// Split the absences of `run` (indices into `Corpus::claims()`) by the lens and by ownership.
pub fn partition_owned<'a>(
    corpus: &Corpus,
    run: &'a UntaskedRun,
    owners: &Owners,
) -> OwnedPartition<'a> {
    let claims: Vec<_> = corpus.claims().collect();
    let mut out = OwnedPartition {
        findings: Vec::new(),
        set_aside: Vec::new(),
        owned: Vec::new(),
    };
    for (i, f) in run.absent() {
        let (doc, c) = claims[i];
        if !is_reportable(&c.form, c.kind) {
            out.set_aside.push((i, f));
        } else if let Some(&row) = owners.get(&doc.path) {
            out.owned.push((i, f, row));
        } else {
            out.findings.push((i, f));
        }
    }
    out
}
