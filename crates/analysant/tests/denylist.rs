// SPDX-License-Identifier: MIT OR Apache-2.0
//! SPEC-STEP1 2.5, mechanically: no engine crate hard-codes a project's vocabulary or a language's
//! word list. The check reads every `src/` file of every engine crate and fails on
//!   (a) the reference project's name, anywhere in the file;
//!   (b) a string literal that is exactly an entry of a shipped lexicon (`"the"`, `"and"`, ...).
//! Its positive control is the same check over a fixture that violates both, which must be caught.
//! The deny-list is built from the data files at run time, so a new lexicon entry is covered at once.

use std::fs;
use std::path::{Path, PathBuf};

const PROJECT_NAMES: &[&str] = &["fuel"];

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every entry of every lexicon file of every pack, folded.
fn lexicon_entries() -> Vec<String> {
    let dir = manifest().join("../analysant-core/data/en");
    let mut out = Vec::new();
    for e in fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display())) {
        let path = e.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name == "params.txt" {
            continue;
        }
        for line in fs::read_to_string(&path).unwrap().lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            out.push(t.trim_matches('"').to_lowercase());
        }
    }
    out
}

/// The string literals of Rust source: `"..."` with `\\` escapes, outside `//` comments.
fn string_literals(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let code = line.split("//").next().unwrap_or("");
        let mut chars = code.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '"' {
                continue;
            }
            let mut lit = String::new();
            while let Some(d) = chars.next() {
                match d {
                    '\\' => {
                        chars.next();
                    }
                    '"' => break,
                    _ => lit.push(d),
                }
            }
            out.push(lit);
        }
    }
    out
}

/// Engine code only: a test module may name the words it tests. By convention every module in these
/// crates puts its tests last, behind the first `#[cfg(test)]`; the fixture has none, so it is read whole.
fn engine_part(text: &str) -> &str {
    text.split("#[cfg(test)]").next().unwrap_or(text)
}

/// Violations in one file's text.
fn violations(text: &str, lexicon: &[String]) -> Vec<String> {
    let text = engine_part(text);
    let mut found = Vec::new();
    let lower = text.to_lowercase();
    for name in PROJECT_NAMES {
        if lower.contains(name) {
            found.push(format!("project name `{name}`"));
        }
    }
    for lit in string_literals(text) {
        let folded = lit.to_lowercase();
        if lexicon.contains(&folded) {
            found.push(format!("lexicon entry as a literal: \"{lit}\""));
        }
    }
    found
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn the_check_can_fail() {
    let lexicon = lexicon_entries();
    assert!(
        lexicon.len() > 20,
        "the lexicon is loaded ({} entries)",
        lexicon.len()
    );
    let fixture =
        fs::read_to_string(manifest().join("tests/fixtures/denylist-positive.txt")).unwrap();
    let got = violations(&fixture, &lexicon);
    assert!(got.iter().any(|v| v.contains("project name")), "{got:?}");
    assert!(got.iter().any(|v| v.contains("\"the\"")), "{got:?}");
    assert!(got.iter().any(|v| v.contains("\"an\"")), "{got:?}");
}

#[test]
fn no_engine_source_hard_codes_a_project_or_a_word_list() {
    let lexicon = lexicon_entries();
    let mut files = Vec::new();
    for krate in [
        "analysant-core",
        "analysant-lexical",
        "analysant-logical",
        "analysant",
    ] {
        rust_files(&manifest().join("..").join(krate).join("src"), &mut files);
    }
    // Population stated beside the null: the check read these files.
    assert!(
        files.len() >= 12,
        "only {} engine source files found",
        files.len()
    );
    let mut bad = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).unwrap();
        for v in violations(&text, &lexicon) {
            bad.push(format!("{}: {v}", f.display()));
        }
    }
    assert!(
        bad.is_empty(),
        "{} files read; violations:\n{}",
        files.len(),
        bad.join("\n")
    );
}
