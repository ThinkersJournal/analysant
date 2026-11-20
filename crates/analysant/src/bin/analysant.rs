// SPDX-License-Identifier: MIT OR Apache-2.0
//! `analysant`: the findings producer. A thin shell over `analysant::produce`: it parses arguments, prints, and sets the exit
//! code (0 done, 2 refused or misused). All behaviour that can be tested lives in the library.
//!
//! ```text
//! analysant findings <dir> [--plan PATH]... [--project NAME] [--code DIR] [--format tsv|jsonl]
//! analysant counts   <dir> [--plan PATH]... [--project NAME] [--code DIR]
//! analysant sample   <findings-file> --seed N [--kind LENS/KIND]
//! analysant judge report <sample-file> <verdicts-file> --judge J --pass-percent P [--second J2] [--void-below K]
//! analysant judge append <verdicts-file> --id ID --verdict V [--class C] --reason R --judge J
//! analysant judge sheet  <sample-file> <verdicts-file> --judge J --pass-percent P --kind LENS/KIND
//! ```

use analysant::gate_b::{Reading, reading};
use analysant::judging::{
    Entry, KindOrder, NO_CLASS, Params, Verdict, VerdictTable, parse_judging, render_report,
    report_all, second_judge_sheet,
};
use analysant::produce::{Options, ProduceError, judging_rows, parse_findings, produce};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage:
  analysant findings <dir> [--plan PATH]... [--project NAME] [--code DIR] [--format tsv|jsonl]
  analysant counts   <dir> [--plan PATH]... [--project NAME] [--code DIR]
  analysant sample   <findings-file> --seed N [--kind LENS/KIND]
  analysant judge report <sample-file> <verdicts-file> --judge J --pass-percent P [--second J2] [--void-below K]
  analysant judge append <verdicts-file> --id ID --verdict TRUE|FALSE|UNJUDGEABLE [--class C] --reason R --judge J
  analysant judge sheet  <sample-file> <verdicts-file> --judge J --pass-percent P --kind LENS/KIND
  analysant --help

findings  run the lenses over the files `git ls-files` lists in <dir>; one line per finding
          (--code DIR: also check the code spans against the Rust files `git ls-files` lists in DIR)
counts    the same run as counts per lens and kind only (no path, no text, no revision)
sample    put one findings file's kinds in Gate B judging order for a seed (SPEC-STEP1 5.2)
judge     keep a judge's verdict table: add a verdict, report the stop rule and second-judge agreement, print the blind sheet
          (the pass line is always given; the agreement threshold is D1's and has no default)";

/// Parsed arguments: positionals, and flags with their values.
struct Args {
    positional: Vec<String>,
    flags: Vec<(String, String)>,
}

fn parse(args: &[String], allowed: &[&str]) -> Result<Args, String> {
    let mut out = Args {
        positional: Vec::new(),
        flags: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a.starts_with("--") {
            if !allowed.contains(&a.as_str()) {
                return Err(format!("unknown flag {a}\n{USAGE}"));
            }
            let Some(v) = it.next() else {
                return Err(format!("{a} needs a value\n{USAGE}"));
            };
            out.flags.push((a.clone(), v.clone()));
        } else {
            out.positional.push(a.clone());
        }
    }
    Ok(out)
}

impl Args {
    fn all(&self, flag: &str) -> Vec<String> {
        self.flags
            .iter()
            .filter(|(f, _)| f == flag)
            .map(|(_, v)| v.clone())
            .collect()
    }
    fn one(&self, flag: &str) -> Option<String> {
        self.all(flag).pop()
    }
    fn single_positional(&self, what: &str) -> Result<&str, String> {
        match self.positional.as_slice() {
            [one] => Ok(one),
            _ => Err(format!("expected exactly one {what}\n{USAGE}")),
        }
    }
}

fn project_name(dir: &Path, given: Option<String>) -> String {
    given.unwrap_or_else(|| {
        std::fs::canonicalize(dir)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "project".to_string())
    })
}

fn run_findings(args: &[String], counts: bool) -> Result<String, String> {
    let allowed: &[&str] = if counts {
        &["--plan", "--project", "--code"]
    } else {
        &["--plan", "--project", "--code", "--format"]
    };
    let a = parse(args, allowed)?;
    let dir = Path::new(a.single_positional("directory")?);
    let opts = Options {
        project: project_name(dir, a.one("--project")),
        plan: a.all("--plan"),
        code: a.one("--code"),
    };
    let production = produce(dir, &opts).map_err(|e: ProduceError| e.to_string())?;
    if counts {
        return Ok(production.render_counts());
    }
    match a.one("--format").as_deref() {
        None | Some("tsv") => Ok(production.render_lines()),
        Some("jsonl") => Ok(production.render_jsonl()),
        Some(other) => Err(format!("unknown format {other}: use tsv or jsonl")),
    }
}

fn run_sample(args: &[String]) -> Result<String, String> {
    let a = parse(args, &["--seed", "--kind"])?;
    let path = a.single_positional("findings file")?;
    let seed: u64 = match a.one("--seed") {
        Some(s) => s
            .parse()
            .map_err(|_| format!("--seed must be a number, got {s}"))?,
        None => {
            return Err(format!(
                "--seed is required: the report states one\n{USAGE}"
            ));
        }
    };
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let rows = parse_findings(&text).map_err(|e| e.to_string())?;
    let mut kinds: BTreeMap<String, Vec<analysant::produce::Row>> = BTreeMap::new();
    for r in rows {
        kinds
            .entry(format!("{}/{}", r.lens, r.kind))
            .or_default()
            .push(r);
    }
    if let Some(want) = a.one("--kind") {
        if !kinds.contains_key(&want) {
            let present: Vec<&str> = kinds.keys().map(String::as_str).collect();
            return Err(format!(
                "the file has no kind {want} (it has: {})",
                present.join(&list_separator())
            ));
        }
        kinds.retain(|k, _| *k == want);
    }
    let mut out = vec!["# analysant-sample v1".to_string()];
    for (kind, rows) in &kinds {
        let how = match reading(rows.len()) {
            Reading::Census { .. } => "census",
            Reading::Sample => "sample",
        };
        out.push(format!(
            "# kind {kind}: findings={} reading={how} seed={seed}",
            rows.len()
        ));
        for (i, r) in judging_rows(rows, seed).iter().enumerate() {
            out.push(format!("{}\t{}", i + 1, r.line()));
        }
    }
    Ok(out.join("\n") + "\n")
}

/// A required flag's value, or the error naming the flag.
fn required(a: &Args, flag: &str) -> Result<String, String> {
    a.one(flag)
        .ok_or_else(|| format!("{flag} is required\n{USAGE}"))
}

fn pass_percent(a: &Args) -> Result<u32, String> {
    let given = required(a, "--pass-percent")?;
    given
        .parse::<u32>()
        .ok()
        .filter(|p| *p <= 100)
        .ok_or_else(|| format!("--pass-percent is a whole percentage up to 100, got {given}"))
}

fn read_text(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
}

fn run_judge(args: &[String]) -> Result<String, String> {
    let (action, rest) = args
        .split_first()
        .ok_or_else(|| format!("judge needs an action\n{USAGE}"))?;
    match action.as_str() {
        "report" => judge_report(rest),
        "append" => judge_append(rest),
        "sheet" => judge_sheet(rest),
        other => Err(format!("unknown judge action {other}\n{USAGE}")),
    }
}

fn judge_inputs(a: &Args) -> Result<(Vec<KindOrder>, VerdictTable), String> {
    let [sample, verdicts] = a.positional.as_slice() else {
        return Err(format!(
            "expected a sample file and a verdicts file\n{USAGE}"
        ));
    };
    let orders = parse_judging(&read_text(sample)?).map_err(|e| e.to_string())?;
    let table = if std::path::Path::new(verdicts).exists() {
        VerdictTable::parse(&read_text(verdicts)?).map_err(|e| e.to_string())?
    } else {
        VerdictTable::new()
    };
    Ok((orders, table))
}

fn judge_report(args: &[String]) -> Result<String, String> {
    let a = parse(
        args,
        &["--judge", "--second", "--pass-percent", "--void-below"],
    )?;
    let judge = required(&a, "--judge")?;
    let second = a.one("--second");
    let void_below = match a.one("--void-below") {
        Some(v) => Some(
            v.parse::<usize>()
                .map_err(|_| format!("--void-below must be a number, got {v}"))?,
        ),
        None => None,
    };
    let params = Params {
        judge: &judge,
        second_judge: second.as_deref(),
        pass_percent: pass_percent(&a)?,
        void_below,
    };
    let (orders, table) = judge_inputs(&a)?;
    let reports = report_all(&orders, &table, &params).map_err(|e| e.to_string())?;
    Ok(render_report(&reports, &params))
}

fn judge_append(args: &[String]) -> Result<String, String> {
    let a = parse(
        args,
        &["--id", "--verdict", "--class", "--reason", "--judge"],
    )?;
    let path = a.single_positional("verdicts file")?;
    let word = required(&a, "--verdict")?;
    let Some(verdict) = Verdict::from_word(&word) else {
        return Err(format!(
            "--verdict is TRUE, FALSE or UNJUDGEABLE, got {word}"
        ));
    };
    let entry = Entry {
        id: required(&a, "--id")?,
        verdict,
        class: a.one("--class").unwrap_or_else(|| NO_CLASS.to_string()),
        reason: required(&a, "--reason")?,
        judge: required(&a, "--judge")?,
    };
    let mut table = if std::path::Path::new(path).exists() {
        VerdictTable::parse(&read_text(path)?).map_err(|e| e.to_string())?
    } else {
        VerdictTable::new()
    };
    table.add(entry).map_err(|e| e.to_string())?;
    std::fs::write(path, table.render()).map_err(|e| format!("cannot write {path}: {e}"))?;
    Ok(String::new())
}

fn judge_sheet(args: &[String]) -> Result<String, String> {
    let a = parse(args, &["--judge", "--pass-percent", "--kind"])?;
    let judge = required(&a, "--judge")?;
    let kind = required(&a, "--kind")?;
    let params = Params {
        judge: &judge,
        second_judge: None,
        pass_percent: pass_percent(&a)?,
        void_below: None,
    };
    let (orders, table) = judge_inputs(&a)?;
    let Some(order) = orders.iter().find(|o| o.kind == kind) else {
        return Err(format!("the sample file has no kind {kind}"));
    };
    let sheet = second_judge_sheet(order, &table, &params).map_err(|e| e.to_string())?;
    let mut out = format!(
        "# analysant-second-judge-sheet v1\n# kind {kind}: seed={} items={}\n",
        order.seed,
        sheet.len()
    );
    for (position, row) in sheet {
        out += &format!("{position}\t{}\n", row.line());
    }
    Ok(out)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("--help" | "-h") => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some("findings") => run_findings(&args[1..], false),
        Some("counts") => run_findings(&args[1..], true),
        Some("sample") => run_sample(&args[1..]),
        Some("judge") => run_judge(&args[1..]),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(text) => {
            // A reader that stops early (`| head`) is not an error of ours: no panic, no message.
            let _ = std::io::stdout().lock().write_all(text.as_bytes());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("analysant: {message}");
            ExitCode::from(2)
        }
    }
}

/// How a list of names is shown in a message.
fn list_separator() -> String {
    format!("{COMMA} ")
}

const COMMA: char = ',';
