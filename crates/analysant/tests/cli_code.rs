// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant findings|counts <dir> --code <dir>`: the doc-vs-code lens in the findings producer, written
//! before the code. Each test builds throwaway git repositories, runs the real binary and compares bytes.
//! Without `--code` nothing about this lens appears (`tests/cli.rs` pins that output, unchanged).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_analysant")
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("analysant-code-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch directory");
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
    fn write(&self, rel: &str, text: &str) {
        let p = self.0.join(rel);
        fs::create_dir_all(p.parent().expect("a parent")).expect("parent directory");
        fs::write(p, text).expect("write");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ceiling(dir: &Path) -> PathBuf {
    dir.parent().expect("a parent").to_path_buf()
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", ceiling(dir))
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(bin())
        .current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", ceiling(dir))
        .args(args)
        .output()
        .expect("the binary runs")
}

fn out(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).expect("utf-8 stdout")
}

fn err(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).expect("utf-8 stderr")
}

const PLAN: &str = "# Plan\n\n1. **Cache layer**: build the cache layer for tiles.\n";
const DESIGN: &str = "# Design\n\nThe path is `build_cache` and `ghost_fn`.\n";
const LIB: &str = "pub fn build_cache() {}\n";
const MAIN: &str = "fn main() { build_cache(); }\n";

/// A docs-and-code repository, staged in the index.
fn fixture(name: &str) -> Scratch {
    let s = Scratch::new(name);
    git(s.path(), &["init", "-q"]);
    s.write("PLAN.md", PLAN);
    s.write("docs/design.md", DESIGN);
    s.write("src/lib.rs", LIB);
    s.write("src/main.rs", MAIN);
    s.write("target/junk.rs", "pub fn ghost_fn() {}\n");
    git(s.path(), &["add", "src", "docs", "PLAN.md"]);
    s
}

fn doc_rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').map(str::to_string).collect::<Vec<_>>())
        .filter(|f| f.get(3).is_some_and(|lens| lens == "doc-vs-code"))
        .collect()
}

const ARGS: [&str; 8] = [
    "findings",
    ".",
    "--plan",
    "PLAN.md",
    "--project",
    "demo",
    "--code",
    ".",
];

#[test]
fn the_missing_name_is_one_row_with_its_search_and_its_control() {
    let s = fixture("row");
    let o = run(s.path(), &ARGS);
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(err(&o), "");
    let rows = doc_rows(&out(&o));
    assert_eq!(rows.len(), 1, "{rows:?}");
    let r = &rows[0];
    assert_eq!(
        &r[..5],
        ["demo", "docs/design.md", "3", "doc-vs-code", "absent"]
    );
    assert!(
        r[5].starts_with("f-") && r[5].len() == 18,
        "a finding id: {}",
        r[5]
    );
    assert_eq!(r[6], "code=-");
    assert_eq!(r[7], "files=2;items=2;query=ghost_fn;matched=0");
    assert_eq!(r[8], "KnownPresent:found");
    assert_eq!(
        r[9],
        "\"ghost_fn\" (docs/design.md:3) names `ghost_fn`, and no definition of it is in the 2 Rust files searched; \
matching is exact and case-sensitive, uses are counted by bare name, one hop up the chain, with no call graph"
    );
}

#[test]
fn the_header_says_which_enumeration_read_the_code_and_what_the_lens_did() {
    let s = fixture("header");
    let o = run(s.path(), &ARGS);
    let text = out(&o);
    // `target/junk.rs` is on disk, defines `ghost_fn`, and is untracked: a disk walk would find it.
    assert!(
        text.contains("# code: git ls-files (4 tracked, 2 rust)\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "# lens doc-vs-code: analysed (named 2, findings 1, withheld 0, resolved 1)\n"
        ),
        "{text}"
    );
}

#[test]
fn without_code_the_lens_does_not_appear_at_all() {
    let s = fixture("off");
    let o = run(
        s.path(),
        &["findings", ".", "--plan", "PLAN.md", "--project", "demo"],
    );
    let text = out(&o);
    assert!(o.status.success(), "{}", err(&o));
    assert!(!text.contains("doc-vs-code"), "{text}");
    assert!(!text.contains("# code:"), "{text}");
    let c = run(
        s.path(),
        &["counts", ".", "--plan", "PLAN.md", "--project", "demo"],
    );
    assert!(!out(&c).contains("doc-vs-code"), "{}", out(&c));
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    let s = fixture("determinism");
    let a = run(s.path(), &ARGS);
    let b = run(s.path(), &ARGS);
    assert!(!a.stdout.is_empty());
    assert_eq!(a.stdout, b.stdout);
    // Control: a second missing name changes the bytes.
    s.write("docs/more.md", "# More\n\nAlso `phantom_fn`.\n");
    git(s.path(), &["add", "-A", "docs"]);
    assert_ne!(a.stdout, run(s.path(), &ARGS).stdout);
}

#[test]
fn counts_carry_every_kind_with_its_number_and_leak_nothing() {
    let s = fixture("counts");
    let o = run(
        s.path(),
        &[
            "counts",
            ".",
            "--plan",
            "PLAN.md",
            "--project",
            "demo",
            "--code",
            ".",
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    for want in [
        "doc-vs-code\tabsent\t1\n",
        "doc-vs-code\ttests-only\t0\n",
        "doc-vs-code\tuncalled\t0\n",
        "doc-vs-code\tdead-callers\t0\n",
        "doc-vs-code\tshared-name\t0\n",
    ] {
        assert!(text.contains(want), "missing {want:?} in\n{text}");
    }
    for leak in [
        "ghost_fn",
        "build_cache",
        "design.md",
        "src/lib.rs",
        "docs/",
    ] {
        assert!(!text.contains(leak), "{leak} leaked:\n{text}");
    }
    // Control: the leak check can fail, the findings view does carry the name.
    assert!(out(&run(s.path(), &ARGS)).contains("ghost_fn"));
}

#[test]
fn a_code_directory_that_is_not_a_git_tree_is_refused_with_no_output() {
    let s = fixture("refuse");
    let plain = Scratch::new("refuse-plain");
    plain.write("src/lib.rs", "pub fn x() {}\n");
    let o = run(
        s.path(),
        &[
            "findings",
            ".",
            "--plan",
            "PLAN.md",
            "--code",
            plain.path().to_str().expect("utf-8 path"),
        ],
    );
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(out(&o), "");
    assert!(err(&o).contains("not a git work tree"), "{}", err(&o));
}

#[test]
fn a_code_tree_with_no_rust_files_withholds_instead_of_reporting_every_name_missing() {
    let s = fixture("norust");
    let code = Scratch::new("norust-code");
    git(code.path(), &["init", "-q"]);
    code.write("README.md", "# nothing here\n");
    git(code.path(), &["add", "-A"]);
    let o = run(
        s.path(),
        &[
            "findings",
            ".",
            "--plan",
            "PLAN.md",
            "--project",
            "demo",
            "--code",
            code.path().to_str().expect("utf-8 path"),
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    assert!(doc_rows(&text).is_empty(), "{text}");
    assert!(
        text.contains(
            "# lens doc-vs-code: analysed (named 2, findings 0, withheld 2, resolved 0)\n"
        ),
        "{text}"
    );
}
