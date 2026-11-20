// SPDX-License-Identifier: MIT OR Apache-2.0
//! The findings producer (`analysant findings | counts | sample`), written before the code (SPEC-STEP1
//! "CLI result"). Every test builds its own small git repository in a temporary directory, runs the real
//! binary on it and compares bytes, so the output format is a contract, not a side effect.
//!
//! The fixture's expected lines were derived by running the existing lenses on the same four lines of
//! documents (the library results are already pinned by the Gate A tests) and then read against the lens
//! rules: three directives are absent from a plan that names only the cache layer, and one negation
//! ("not supported") pairs with one positive statement about the same capability in another file.

use analysant::gate_b::{FindingRef, judging_order};
use analysant_core::LensId;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_analysant")
}

/// A scratch directory that removes itself. Unique per test name and process.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("analysant-cli-{name}-{}", std::process::id()));
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

/// A directory above the scratch directory that git must not search past, so a scratch directory that is
/// not a repository is never mistaken for part of one that encloses the temp directory.
fn ceiling(dir: &Path) -> PathBuf {
    dir.parent().expect("a parent").to_path_buf()
}

const PLAN: &str = "# Plan\n\n1. **Cache layer**: build the cache layer for tiles.\n";
const ARCH: &str = "# Architecture\n\n## Caching\n\n- Build the cache layer for tiles.\n- Add a retry queue for failed uploads.\n- Support streaming ingestion.\n";
const LIMITS: &str = "# Limits\n\n- Streaming ingestion is not supported.\n";

/// The four-file fixture, staged in the index (never committed: the revision is then `none`).
fn fixture(name: &str) -> Scratch {
    let s = Scratch::new(name);
    git(s.path(), &["init", "-q"]);
    s.write("PLAN.md", PLAN);
    s.write("docs/arch.md", ARCH);
    s.write("docs/limits.md", LIMITS);
    s.write("notes.txt", "not a markdown file\n");
    git(s.path(), &["add", "-A"]);
    s
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(bin())
        .current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", ceiling(dir))
        .args(args)
        .output()
        .expect("the binary runs")
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).expect("utf-8 stdout")
}

fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).expect("utf-8 stderr")
}

/// The lines of `text` without the two header lines that name the state of the repository, which a
/// golden comparison cannot pin (they are tested on their own).
fn without_repo_state(text: &str) -> String {
    text.lines()
        .filter(|l| !l.starts_with("# revision:") && !l.starts_with("# worktree:"))
        .map(|l| format!("{l}\n"))
        .collect()
}

fn contradiction_id() -> String {
    analysant_core::FindingId::compute(
        &LensId("contradiction".into()),
        "contradiction",
        &[],
        "docs/limits.md:3|docs/arch.md:7",
    )
    .0
}

fn golden_findings() -> String {
    let id = contradiction_id();
    format!(
        "# analysant-findings v1\n\
# project: demo\n\
# enumeration: git ls-files (4 tracked, 3 markdown)\n\
# config: none (no-config baseline)\n\
# files: corpus=2 plan=1 unreadable=0\n\
# lens untasked: analysed (plan lines 3, claims 4, absent 3, set aside 0, owned 0, withheld 0, mentioned 1)\n\
# lens contradiction: analysed (pairs 1, suppressed 0)\n\
# columns: project file line lens kind id refs search control text\n\
demo\tdocs/arch.md\t6\tuntasked\tuntasked\tf-616a18a4e52f0737\tform=ListItem\tplan-lines=3;key=add retry queue failed uploads;matched=0\tSelfFind:found\tAdd a retry queue for failed uploads.\n\
demo\tdocs/arch.md\t7\tuntasked\tuntasked\tf-6b61b611c12a0c30\tform=ListItem\tplan-lines=3;key=support streaming ingestion;matched=0\tSelfFind:found\tSupport streaming ingestion.\n\
demo\tdocs/limits.md\t3\tcontradiction\tcontradiction\t{id}\tpositive=docs/arch.md:7\tshared=streaming,ingestion;negated_by=not supported\t-\tnegated: Streaming ingestion is not supported. || positive: Support streaming ingestion.\n\
demo\tdocs/limits.md\t3\tuntasked\tuntasked\tf-83fd72a5a850e73d\tform=ListItem\tplan-lines=3;key=streaming ingestion not supported;matched=0\tSelfFind:found\tStreaming ingestion is not supported.\n"
    )
}

#[test]
fn findings_are_exactly_the_golden_lines() {
    let s = fixture("golden");
    let o = run(
        s.path(),
        &["findings", ".", "--plan", "PLAN.md", "--project", "demo"],
    );
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(stderr(&o), "");
    assert_eq!(without_repo_state(&stdout(&o)), golden_findings());
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    let s = fixture("determinism");
    let args = ["findings", ".", "--plan", "PLAN.md", "--project", "demo"];
    let a = run(s.path(), &args);
    let b = run(s.path(), &args);
    assert!(a.status.success() && b.status.success());
    assert!(!a.stdout.is_empty());
    assert_eq!(a.stdout, b.stdout, "two runs, byte for byte");
    // Control: the check can fail. A different input (one more tracked document) changes the bytes.
    s.write("docs/more.md", "# More\n\n- Add a dashboard for tiles.\n");
    git(s.path(), &["add", "-A"]);
    let c = run(s.path(), &args);
    assert_ne!(a.stdout, c.stdout);
}

#[test]
fn the_order_of_findings_does_not_depend_on_the_order_files_were_added() {
    let a = fixture("order-a");
    let b = Scratch::new("order-b");
    git(b.path(), &["init", "-q"]);
    // Same files, staged in the opposite order.
    b.write("notes.txt", "not a markdown file\n");
    git(b.path(), &["add", "notes.txt"]);
    b.write("docs/limits.md", LIMITS);
    git(b.path(), &["add", "docs/limits.md"]);
    b.write("docs/arch.md", ARCH);
    git(b.path(), &["add", "docs/arch.md"]);
    b.write("PLAN.md", PLAN);
    git(b.path(), &["add", "PLAN.md"]);
    let args = ["findings", ".", "--plan", "PLAN.md", "--project", "demo"];
    assert_eq!(run(a.path(), &args).stdout, run(b.path(), &args).stdout);
    assert!(!run(a.path(), &args).stdout.is_empty());
}

#[test]
fn jsonl_carries_the_same_findings_as_the_lines() {
    let s = fixture("jsonl");
    let lines = run(
        s.path(),
        &["findings", ".", "--plan", "PLAN.md", "--project", "demo"],
    );
    let json = run(
        s.path(),
        &[
            "findings",
            ".",
            "--plan",
            "PLAN.md",
            "--project",
            "demo",
            "--format",
            "jsonl",
        ],
    );
    assert!(json.status.success(), "{}", stderr(&json));
    let rows: Vec<serde_json::Value> = stdout(&json)
        .lines()
        .map(|l| serde_json::from_str(l).expect("every line is JSON"))
        .collect();
    assert_eq!(rows[0]["type"], "header");
    assert_eq!(rows[0]["project"], "demo");
    assert_eq!(rows[0]["format"], "analysant-findings v1");
    let findings: Vec<&serde_json::Value> = rows[1..].iter().collect();
    assert_eq!(findings.len(), 4);
    let table: Vec<Vec<String>> = stdout(&lines)
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect();
    assert_eq!(table.len(), 4);
    for (f, t) in findings.iter().zip(&table) {
        assert_eq!(f["type"], "finding");
        assert_eq!(f["project"], t[0]);
        assert_eq!(f["file"], t[1]);
        assert_eq!(f["line"].to_string(), t[2]);
        assert_eq!(f["lens"], t[3]);
        assert_eq!(f["kind"], t[4]);
        assert_eq!(f["id"], t[5]);
        assert_eq!(f["text"], t[9]);
    }
}

#[test]
fn a_directory_that_is_not_a_git_work_tree_is_refused_not_walked() {
    let s = Scratch::new("nongit");
    s.write("docs/arch.md", ARCH);
    s.write("PLAN.md", PLAN);
    let o = run(s.path(), &["findings", ".", "--plan", "PLAN.md"]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert_eq!(stdout(&o), "", "nothing is analysed from a disk walk");
    let err = stderr(&o);
    assert!(err.contains("git ls-files"), "names the enumeration: {err}");
    assert!(err.contains("not a git work tree"), "{err}");
    // Positive control: the same files in a repository are read.
    git(s.path(), &["init", "-q"]);
    git(s.path(), &["add", "-A"]);
    let o = run(s.path(), &["findings", ".", "--plan", "PLAN.md"]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert!(stdout(&o).contains("# enumeration: git ls-files (2 tracked, 2 markdown)"));
}

#[test]
fn only_tracked_files_are_read() {
    let s = fixture("tracked");
    s.write(
        "docs/untracked.md",
        "# Untracked\n\n- Rotate the quokka telemetry keys.\n",
    );
    let o = run(s.path(), &["findings", ".", "--plan", "PLAN.md"]);
    assert!(o.status.success(), "{}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("(4 tracked, 3 markdown)"), "{out}");
    assert!(!out.contains("quokka"), "an untracked file is not read");
    // Control: tracked, the same document is read and its claim is reported.
    git(s.path(), &["add", "docs/untracked.md"]);
    let o = run(s.path(), &["findings", ".", "--plan", "PLAN.md"]);
    let out = stdout(&o);
    assert!(out.contains("(5 tracked, 4 markdown)"), "{out}");
    assert!(out.contains("quokka telemetry keys"), "{out}");
}

#[test]
fn aggregate_counts_leak_no_path_and_no_text() {
    let s = Scratch::new("aggregate");
    git(s.path(), &["init", "-q"]);
    s.write("private-area/zebra-plan.md", PLAN);
    s.write(
        "private-area/zebra-arch.md",
        "# Zebra\n\n- Add a quokka telemetry exporter.\n- Support walrus ingestion.\n",
    );
    s.write(
        "private-area/zebra-limits.md",
        "# Limits\n\n- Walrus ingestion is not supported.\n",
    );
    git(s.path(), &["add", "-A"]);
    let args = ["--plan", "private-area/zebra-plan.md", "--project", "demo"];
    let mut full = vec!["findings", "."];
    full.extend(args);
    let o = run(s.path(), &full);
    let findings = stdout(&o);
    // Positive control: the findings view does contain what the aggregate view must not.
    for needle in ["private-area", "zebra", "quokka", "walrus"] {
        assert!(findings.contains(needle), "control: {needle} in {findings}");
    }
    let mut agg = vec!["counts", "."];
    agg.extend(args);
    let o = run(s.path(), &agg);
    assert!(o.status.success(), "{}", stderr(&o));
    let out = stdout(&o);
    for needle in [
        "private-area",
        "zebra",
        "quokka",
        "walrus",
        "PLAN",
        ".md",
        "f-",
    ] {
        assert!(
            !out.contains(needle),
            "aggregate output leaked {needle:?}:\n{out}"
        );
    }
    assert_eq!(
        out,
        "# analysant-counts v1\n\
# project: demo\n\
# enumeration: git ls-files (3 tracked, 3 markdown)\n\
# revision: withheld (aggregate)\n\
# worktree: withheld (aggregate)\n\
# config: none (no-config baseline)\n\
# files: corpus=2 plan=1 unreadable=0\n\
# lens untasked: analysed (plan lines 3, claims 3, absent 3, set aside 0, owned 0, withheld 0, mentioned 0)\n\
# lens contradiction: analysed (pairs 1, suppressed 0)\n\
# columns: lens kind count\n\
contradiction\tcontradiction\t1\n\
untasked\tuntasked\t3\n"
    );
}

#[test]
fn without_a_plan_the_untasked_lens_says_it_did_not_run() {
    let s = fixture("noplan");
    let o = run(s.path(), &["findings", ".", "--project", "demo"]);
    assert!(o.status.success(), "{}", stderr(&o));
    let out = stdout(&o);
    assert!(
        out.contains(
            "# lens untasked: not analysed (no plan given: use --plan or [corpus] plan_files)"
        ),
        "{out}"
    );
    assert!(
        out.contains("# files: corpus=3 plan=0 unreadable=0"),
        "{out}"
    );
    assert!(
        !out.lines().any(|l| l.contains("\tuntasked\tuntasked\t")),
        "no untasked row without a plan"
    );
    // The silence is not "clean": the contradiction lens still ran on all three documents.
    assert!(out.contains("# lens contradiction: analysed (pairs 1, suppressed 0)"));
    let o = run(s.path(), &["counts", ".", "--project", "demo"]);
    assert!(stdout(&o).contains("untasked\tuntasked\tn/a\n"));
}

#[test]
fn the_project_file_is_read_when_present_and_the_baseline_runs_without_it() {
    let s = fixture("config");
    s.write(
        "PLAN.md",
        "# Plan\n\n1. **Cache layer**: build the cache layer for tiles.\n2. **Resend**: add resend list failed uploads.\n",
    );
    git(s.path(), &["add", "-A"]);
    let base = run(
        s.path(),
        &["findings", ".", "--plan", "PLAN.md", "--project", "demo"],
    );
    let base_out = stdout(&base);
    assert!(
        base_out.contains("# config: none (no-config baseline)"),
        "{base_out}"
    );
    assert!(base_out.contains("add retry queue failed uploads"));
    // The project's own alias file says a retry queue and a resend list are one thing.
    s.write("aliases.txt", "retry queue | resend list\n");
    s.write(
        ".analysant/project.toml",
        "[corpus]\naliases = [\"aliases.txt\"]\n",
    );
    git(s.path(), &["add", "-A"]);
    let o = run(
        s.path(),
        &["findings", ".", "--plan", "PLAN.md", "--project", "demo"],
    );
    assert!(o.status.success(), "{}", stderr(&o));
    let out = stdout(&o);
    assert!(
        out.contains("# config: .analysant/project.toml"),
        "names the file it read: {out}"
    );
    assert!(
        !out.contains("add retry queue failed uploads"),
        "the alias makes the plan line mention it: {out}"
    );
    assert!(out.contains("absent 2,"), "{out}");
    // `.analysant/` is the project's own data, never analysed as a document.
    assert!(out.contains("(6 tracked, 3 markdown)"), "{out}");
}

#[test]
fn a_misspelled_project_key_is_an_error_not_a_silent_baseline() {
    let s = fixture("badconfig");
    s.write(".analysant/project.toml", "[corpus]\nplann_files = []\n");
    git(s.path(), &["add", "-A"]);
    let o = run(s.path(), &["findings", ".", "--plan", "PLAN.md"]);
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(stdout(&o), "");
    assert!(
        stderr(&o).contains("unknown key `plann_files`"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn the_project_file_can_name_the_plan() {
    let s = fixture("planinconfig");
    s.write(
        ".analysant/project.toml",
        "[corpus]\nplan_files = [\"PLAN.md\"]\n",
    );
    git(s.path(), &["add", "-A"]);
    let o = run(s.path(), &["findings", ".", "--project", "demo"]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(
        without_repo_state(&stdout(&o))
            .replace(
                "# config: .analysant/project.toml\n",
                "# config: none (no-config baseline)\n"
            )
            .replace("(5 tracked, 3 markdown)", "(4 tracked, 3 markdown)"),
        golden_findings()
    );
}

#[test]
fn the_revision_is_recorded_after_a_commit_and_the_tree_state_is_stated() {
    let s = fixture("revision");
    let o = run(s.path(), &["findings", ".", "--plan", "PLAN.md"]);
    let before = stdout(&o);
    assert!(before.contains("# revision: none (no commits)"), "{before}");
    assert!(before.contains("# worktree: dirty"), "{before}");
    git(
        s.path(),
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "commit",
            "-q",
            "-m",
            "fixture",
        ],
    );
    let after = stdout(&run(s.path(), &["findings", ".", "--plan", "PLAN.md"]));
    let rev = after
        .lines()
        .find_map(|l| l.strip_prefix("# revision: "))
        .expect("a revision line");
    assert_eq!(rev.len(), 40, "{rev}");
    assert!(rev.bytes().all(|b| b.is_ascii_hexdigit()), "{rev}");
    assert!(after.contains("# worktree: clean"), "{after}");
    // A change after the commit is stated, not hidden: the report would otherwise name a revision it did not read.
    s.write(
        "docs/arch.md",
        &format!("{ARCH}- Add a gamma ray shield.\n"),
    );
    let dirty = stdout(&run(s.path(), &["findings", ".", "--plan", "PLAN.md"]));
    assert!(dirty.contains("# worktree: dirty"), "{dirty}");
}

fn finding_line(file: &str, line: u32, id: &str) -> String {
    format!(
        "demo\t{file}\t{line}\tcontradiction\tcontradiction\t{id}\tpositive=x.md:1\tshared=a\t-\ttext {id}\n"
    )
}

fn sample(dir: &Path, findings: &str, extra: &[&str]) -> Output {
    let p = dir.join("findings.tsv");
    fs::write(&p, findings).expect("write findings");
    let mut args = vec!["sample", "findings.tsv"];
    args.extend(extra);
    run(dir, &args)
}

#[test]
fn sample_orders_a_kind_as_the_gate_b_library_does() {
    let s = Scratch::new("sample");
    let mut text = String::from("# analysant-findings v1\n");
    let mut refs = Vec::new();
    for n in 1..=12u32 {
        text += &finding_line("a.md", n, &format!("f-{n:016x}"));
        refs.push(FindingRef {
            project: "demo".into(),
            file: "a.md".into(),
            line: n,
        });
    }
    let o = sample(s.path(), &text, &["--seed", "2026"]);
    assert!(o.status.success(), "{}", stderr(&o));
    let out = stdout(&o);
    assert!(
        out.contains("# kind contradiction/contradiction: findings=12 reading=census seed=2026"),
        "{out}"
    );
    let order: Vec<u32> = out
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let mut f = l.split('\t');
            let _position: usize = f.next().expect("position").parse().expect("number");
            let row: Vec<&str> = f.collect();
            row[2].parse().expect("line")
        })
        .collect();
    let want: Vec<u32> = judging_order(refs, 2026).iter().map(|r| r.line).collect();
    assert_eq!(order, want);
    // A different seed: the same members, a different order. The control that the seed is read.
    let o2 = sample(s.path(), &text, &["--seed", "7"]);
    let mut other: Vec<u32> = stdout(&o2)
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').nth(3).expect("line").parse().expect("number"))
        .collect();
    assert_ne!(other, order);
    other.sort_unstable();
    assert_eq!(other, (1..=12).collect::<Vec<u32>>());
}

#[test]
fn sample_numbers_positions_from_one_and_keeps_the_whole_finding_line() {
    let s = Scratch::new("samplefields");
    let line = finding_line("a.md", 5, "f-00000000000000aa");
    let o = sample(s.path(), &line, &["--seed", "1"]);
    let out = stdout(&o);
    assert!(
        out.contains(&format!("1\t{}", line.trim_end())),
        "position then the original line: {out}"
    );
}

#[test]
fn findings_that_share_a_position_are_ordered_by_id_whatever_the_input_order() {
    let s = Scratch::new("sampleties");
    let ids = [
        "f-0000000000000003",
        "f-0000000000000001",
        "f-0000000000000002",
    ];
    let forward: String = ids.iter().map(|i| finding_line("a.md", 9, i)).collect();
    let backward: String = ids
        .iter()
        .rev()
        .map(|i| finding_line("a.md", 9, i))
        .collect();
    let a = stdout(&sample(s.path(), &forward, &["--seed", "5"]));
    let b = stdout(&sample(s.path(), &backward, &["--seed", "5"]));
    assert_eq!(a, b);
    let seen: Vec<&str> = a
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').nth(6).expect("id"))
        .collect();
    assert_eq!(seen.len(), 3);
    let mut sorted = seen.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        [
            "f-0000000000000001",
            "f-0000000000000002",
            "f-0000000000000003"
        ]
    );
}

#[test]
fn sample_reads_each_kind_on_its_own_and_marks_a_sample_from_thirty() {
    let s = Scratch::new("samplekinds");
    let mut text = String::new();
    for n in 1..=30u32 {
        text += &finding_line("a.md", n, &format!("f-{n:016x}"));
    }
    text += "demo\tb.md\t1\tuntasked\tuntasked\tf-ffffffffffffffff\tform=ListItem\tq\tSelfFind:found\tone\n";
    let out = stdout(&sample(s.path(), &text, &["--seed", "3"]));
    assert!(
        out.contains("# kind contradiction/contradiction: findings=30 reading=sample seed=3"),
        "{out}"
    );
    assert!(
        out.contains("# kind untasked/untasked: findings=1 reading=census seed=3"),
        "{out}"
    );
    // `--kind` selects one.
    let only = stdout(&sample(
        s.path(),
        &text,
        &["--seed", "3", "--kind", "untasked/untasked"],
    ));
    assert!(only.contains("# kind untasked/untasked"));
    assert!(!only.contains("# kind contradiction/contradiction"));
}

#[test]
fn sample_refuses_a_malformed_line_and_says_which() {
    let s = Scratch::new("samplebad");
    let text = format!(
        "{}demo\tonly\tthree\n",
        finding_line("a.md", 1, "f-0000000000000001")
    );
    let o = sample(s.path(), &text, &["--seed", "1"]);
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(stdout(&o), "");
    assert!(stderr(&o).contains("line 2"), "{}", stderr(&o));
}

#[test]
fn sample_needs_a_seed_because_the_report_must_state_one() {
    let s = Scratch::new("sampleseed");
    let o = sample(
        s.path(),
        &finding_line("a.md", 1, "f-0000000000000001"),
        &[],
    );
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("--seed"), "{}", stderr(&o));
}

#[test]
fn an_unknown_subcommand_or_flag_is_a_usage_error_and_help_succeeds() {
    let s = Scratch::new("usage");
    let o = run(s.path(), &["frobnicate"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("usage"), "{}", stderr(&o));
    let o = run(s.path(), &["findings", ".", "--no-such-flag"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("--no-such-flag"), "{}", stderr(&o));
    let o = run(s.path(), &["--help"]);
    assert!(o.status.success());
    assert!(stdout(&o).contains("findings") && stdout(&o).contains("sample"));
}
