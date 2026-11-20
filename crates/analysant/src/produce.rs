// SPDX-License-Identifier: MIT OR Apache-2.0
//! The findings producer (SPEC-STEP1 5.2, "CLI result"): run the lenses that exist over one project and say what
//! they found, as stable lines a Gate B judge can read, sort and sample.
//!
//! Three rules shape it.
//!
//! * **The population is the git index.** Files come from `git ls-files`, never from a walk of the disk, so a
//!   scratch file, a build directory or another lane's worktree cannot enter the corpus. A directory that is
//!   not a git work tree is refused, and the output says which enumeration ran.
//! * **No configuration is a baseline, not an error.** `.analysant/project.toml` is read when it is there;
//!   without it nothing is tuned (no alias, no sense table, no ownership convention). A misspelled key is an
//!   error (`ProjectConfig`).
//! * **The output is a contract.** Rows are sorted by `(project, file, line, lens, kind, id)`, fields are escaped,
//!   and the same input gives the same bytes. The aggregate view carries counts only: no path, no text and no
//!   revision, so a private project can be reported without publishing what is in it.

use crate::ownership::{Owners, partition_owned};
use crate::pragmatic::classify_corpus;
use crate::project::ProjectConfig;
use crate::untasked::{self, UntaskedRun};
use crate::{contradiction, extract_corpus};
use analysant_core::{English, FindingId, Ledger, Markdown};
use analysant_lexical::{AliasTable, MatchMode};
use analysant_logical::{Config as LogicalConfig, ContradictionLens};
use analysant_pragmatic::{Config as PragmaticConfig, PragmaticLens};
use analysant_semantic::{ConceptStore, LayerSource};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::process::Command;

/// The format name written at the head of every findings file; `parse_findings` checks nothing against it (a file
/// is any lines of ten tab-separated fields) but a reader can see which contract the file was made under.
pub const FINDINGS_FORMAT: &str = "analysant-findings v1";
pub const COUNTS_FORMAT: &str = "analysant-counts v1";
pub const PROJECT_FILE: &str = ".analysant/project.toml";
/// The ten columns of a findings row, in order. One string so the names are data, not a list of word literals.
pub const COLUMN_NAMES: &str = "project file line lens kind id refs search control text";

/// Which column holds the line number (the only column that is not text).
const NUMBER_COLUMN: usize = 2;

/// The column names, in order.
pub fn columns() -> Vec<&'static str> {
    COLUMN_NAMES.split_whitespace().collect()
}

/// The command that says whether the work tree differs from what is recorded.
const DIRTY_COMMAND: &str = "status --porcelain";

/// Why a run could not be made. The message is for a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProduceError(pub String);

impl fmt::Display for ProduceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ProduceError {}

fn fail<T>(msg: impl Into<String>) -> Result<T, ProduceError> {
    Err(ProduceError(msg.into()))
}

/// What `git ls-files` said, and the state of the repository it said it about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enumeration {
    /// Tracked paths, `/`-separated, relative to the directory asked about, sorted bytewise.
    pub files: Vec<String>,
    /// `HEAD`, or `None` before the first commit.
    pub revision: Option<String>,
    /// Whether the work tree differs from the index or `HEAD` (including untracked files).
    pub dirty: bool,
}

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output, ProduceError> {
    match Command::new("git").current_dir(root).args(args).output() {
        Ok(o) => Ok(o),
        Err(e) => fail(format!("cannot run git: {e}")),
    }
}

/// The tracked files under `root`, by `git ls-files -z` and by nothing else.
pub fn enumerate(root: &Path) -> Result<Enumeration, ProduceError> {
    let out = git(root, &["ls-files", "-z"])?;
    if !out.status.success() {
        return fail(format!(
            "not a git work tree: `git ls-files` failed in {}: {}",
            root.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let mut files: Vec<String> = out
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect();
    files.sort();
    files.dedup();
    let head = git(root, &["rev-parse", "--verify", "-q", "HEAD"])?;
    let revision = head
        .status
        .success()
        .then(|| String::from_utf8_lossy(&head.stdout).trim().to_string())
        .filter(|r| !r.is_empty());
    let dirty_args: Vec<&str> = DIRTY_COMMAND.split_whitespace().collect();
    let status = git(root, &dirty_args)?;
    Ok(Enumeration {
        files,
        revision,
        dirty: !status.stdout.is_empty(),
    })
}

/// One finding, as a row. Field order is the sort order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Row {
    pub project: String,
    pub file: String,
    pub line: u32,
    pub lens: String,
    pub kind: String,
    pub id: String,
    /// Other places the finding points at.
    pub refs: String,
    /// What was searched, as `key=value` pairs; `-` where a lens records none.
    pub search: String,
    /// The control that vouches for the search, or `-`.
    pub control: String,
    pub text: String,
}

/// A lens that ran, or did not and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LensStatus {
    Analysed(String),
    NotAnalysed(String),
}

impl fmt::Display for LensStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Analysed(d) => write!(f, "analysed ({d})"),
            Self::NotAnalysed(r) => write!(f, "not analysed ({r})"),
        }
    }
}

/// Everything one run says.
#[derive(Clone, Debug)]
pub struct Production {
    pub project: String,
    pub tracked: usize,
    pub markdown: usize,
    pub revision: Option<String>,
    pub dirty: bool,
    /// The project file that was read, or `None` for the no-config baseline.
    pub config_file: Option<String>,
    pub corpus_files: usize,
    pub plan_files: usize,
    pub unreadable: usize,
    pub untasked: LensStatus,
    pub contradiction: LensStatus,
    /// Findings per `(lens, kind)`; a kind a lens declares but did not run has no entry.
    pub counts: BTreeMap<(String, String), usize>,
    /// Sorted.
    pub rows: Vec<Row>,
}

/// What a caller chooses; everything else comes from the project.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub project: String,
    /// Plan files named on the command line, added to the project file's.
    pub plan: Vec<String>,
}

fn is_markdown(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

fn norm_path(p: &str) -> String {
    p.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn read_data(root: &Path, rel: &str, what: &str) -> Result<String, ProduceError> {
    match std::fs::read_to_string(root.join(rel)) {
        Ok(t) => Ok(t),
        Err(e) => fail(format!("cannot read {what} `{rel}`: {e}")),
    }
}

/// Run every lens on the project under `root`.
pub fn produce(root: &Path, opts: &Options) -> Result<Production, ProduceError> {
    let enumeration = enumerate(root)?;
    let tracked: std::collections::BTreeSet<&str> =
        enumeration.files.iter().map(String::as_str).collect();

    let config_present = root.join(PROJECT_FILE).is_file();
    let config = if config_present {
        let text = read_data(root, PROJECT_FILE, "the project file")?;
        ProjectConfig::parse(&text).map_err(|e| ProduceError(e.to_string()))?
    } else {
        ProjectConfig::default()
    };

    let mut plan_paths: Vec<String> = config
        .plan
        .iter()
        .chain(&opts.plan)
        .map(|p| norm_path(p))
        .collect();
    let mut seen_plans = std::collections::BTreeSet::new();
    plan_paths.retain(|p| seen_plans.insert(p.clone()));
    for p in &plan_paths {
        if !tracked.contains(p.as_str()) {
            return fail(format!(
                "the plan `{p}` is not a tracked file (the corpus is what `git ls-files` lists)"
            ));
        }
    }

    let mut unreadable = 0usize;
    let mut read = |path: &str| -> Option<String> {
        let text = std::fs::read_to_string(root.join(path)).ok();
        if text.is_none() {
            unreadable += 1;
        }
        text
    };
    let mut plan_lines: Vec<String> = Vec::new();
    for p in &plan_paths {
        if let Some(t) = read(p) {
            plan_lines.extend(t.lines().map(str::to_string));
        }
    }
    let project_dir = format!("{}/", PROJECT_FILE.split('/').next().unwrap_or_default());
    let corpus_paths: Vec<&String> = enumeration
        .files
        .iter()
        .filter(|f| is_markdown(f) && !plan_paths.contains(f) && !f.starts_with(&project_dir))
        .collect();
    let files: Vec<(String, String)> = corpus_paths
        .iter()
        .filter_map(|p| read(p).map(|t| ((*p).clone(), t)))
        .collect();

    let en = English::new();
    let mut corpus = extract_corpus(&Markdown, &en, &mut Ledger::default(), &files);
    let pragmatic = PragmaticLens::new(&en, PragmaticConfig::default())
        .map_err(|e| ProduceError(e.to_string()))?;
    classify_corpus(&mut corpus, &pragmatic);

    let mut alias_text = String::new();
    for a in &config.aliases {
        alias_text += &read_data(root, a, "the alias file")?;
        alias_text.push('\n');
    }
    let aliases = AliasTable::parse(&en, &alias_text);
    let layers: Vec<LayerSource> = config
        .concepts
        .iter()
        .map(|p| {
            read_data(root, p, "the concept file").map(|text| LayerSource {
                name: p.clone(),
                text,
            })
        })
        .collect::<Result<_, _>>()?;
    let store = if layers.is_empty() {
        None
    } else {
        let s = ConceptStore::load(&en, &layers).map_err(|e| ProduceError(e.to_string()))?;
        let failed = s.run_tests();
        if let Some(f) = failed.first() {
            return fail(format!(
                "the concept table fails its own tests ({} failed; first: {} in {}: {})",
                failed.len(),
                f.concept.0,
                f.layer,
                f.reason
            ));
        }
        Some(s)
    };

    let mut rows: Vec<Row> = Vec::new();
    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();

    let untasked_status = if plan_lines.is_empty() {
        LensStatus::NotAnalysed("no plan given: use --plan or [corpus] plan_files".into())
    } else {
        let run: UntaskedRun = untasked::directed_but_untasked(
            &en,
            &corpus,
            &plan_lines,
            &aliases,
            MatchMode::TokenSubset,
        );
        let owners: Owners = match &config.ownership {
            Some(cfg) => crate::ownership::owners(&Markdown, &en, cfg, &plan_lines, &files),
            None => Owners::new(),
        };
        let part = partition_owned(&corpus, &run, &owners);
        let claims: Vec<_> = corpus.claims().collect();
        for (i, f) in &part.findings {
            let (doc, claim) = claims[*i];
            let mut search = format!(
                "plan-lines={};key={};matched={}",
                f.search.population, f.search.query.key, f.search.matched
            );
            if !f.search.query.aliases_applied.is_empty() {
                search += &format!(";aliases={}", f.search.query.aliases_applied.join(","));
            }
            rows.push(Row {
                project: opts.project.clone(),
                file: doc.path.clone(),
                line: claim.unit.source.span.line_start,
                lens: f.lens.0.clone(),
                kind: f.kind.label().to_string(),
                id: f.id.0.clone(),
                refs: format!("form={:?}", claim.form),
                search,
                control: format!(
                    "{:?}:{}",
                    f.control.kind,
                    format!("{:?}", f.control.outcome).to_lowercase()
                ),
                text: claim.text.clone(),
            });
        }
        let spec = untasked::spec();
        for kind in &spec.kinds {
            let n = part
                .findings
                .iter()
                .filter(|(_, f)| f.kind.label() == kind.as_str())
                .count();
            counts.insert((spec.id.0.clone(), kind.clone()), n);
        }
        LensStatus::Analysed(format!(
            "plan lines {}, claims {}, absent {}, set aside {}, owned {}, withheld {}, mentioned {}",
            run.plan_lines,
            run.outcomes.len(),
            run.absent().count(),
            part.set_aside.len(),
            part.owned.len(),
            run.withheld(),
            run.mentioned()
        ))
    };

    let lens = ContradictionLens::new(
        &en,
        LogicalConfig {
            extra_pairing_stopwords: config.pairing_stopwords.clone(),
        },
    )
    .map_err(|e| ProduceError(e.to_string()))?;
    let statements = contradiction::statements(&corpus, &lens);
    let found = lens.find_contradictions(&statements, store.as_ref());
    let contradiction_lens = contradiction::spec().id;
    for c in &found.pairs {
        let key = format!("{}|{}", c.negated.id, c.positive.id);
        rows.push(Row {
            project: opts.project.clone(),
            file: c.negated.file.clone(),
            line: u32::try_from(c.negated.line).unwrap_or(u32::MAX),
            lens: contradiction_lens.0.clone(),
            kind: contradiction::spec().kinds[0].clone(),
            id: FindingId::compute(
                &contradiction_lens,
                &contradiction::spec().kinds[0],
                &[],
                &key,
            )
            .0,
            refs: format!("positive={}", c.positive.id),
            search: format!(
                "shared={};negated_by={}",
                c.shared.join(","),
                c.negated_by.as_deref().unwrap_or("negated")
            ),
            control: "-".into(),
            text: format!(
                "negated: {} || positive: {}",
                c.negated.text, c.positive.text
            ),
        });
    }
    let cspec = contradiction::spec();
    counts.insert(
        (cspec.id.0.clone(), cspec.kinds[0].clone()),
        found.pairs.len(),
    );
    let contradiction_status = LensStatus::Analysed(format!(
        "pairs {}, suppressed {}",
        found.pairs.len(),
        found.suppressed.len()
    ));

    rows.sort();
    Ok(Production {
        project: opts.project.clone(),
        tracked: enumeration.files.len(),
        markdown: enumeration.files.iter().filter(|f| is_markdown(f)).count(),
        revision: enumeration.revision,
        dirty: enumeration.dirty,
        config_file: config_present.then(|| PROJECT_FILE.to_string()),
        corpus_files: files.len(),
        plan_files: plan_paths.len(),
        unreadable,
        untasked: untasked_status,
        contradiction: contradiction_status,
        counts,
        rows,
    })
}

/// A field with the characters that would break a line or a column written as escapes.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

impl Row {
    /// The row as one tab-separated line, without the newline.
    pub fn line(&self) -> String {
        [
            &self.project,
            &self.file,
            &self.line.to_string(),
            &self.lens,
            &self.kind,
            &self.id,
            &self.refs,
            &self.search,
            &self.control,
            &self.text,
        ]
        .iter()
        .map(|f| escape(f))
        .collect::<Vec<_>>()
        .join("\t")
    }
}

impl Production {
    fn header(&self, format: &str, aggregate: bool) -> Vec<String> {
        let revision = if aggregate {
            "withheld (aggregate)".to_string()
        } else {
            self.revision
                .clone()
                .unwrap_or_else(|| "none (no commits)".to_string())
        };
        let worktree = if aggregate {
            "withheld (aggregate)"
        } else if self.dirty {
            "dirty"
        } else {
            "clean"
        };
        vec![
            format!("# {format}"),
            format!("# project: {}", self.project),
            format!(
                "# enumeration: git ls-files ({} tracked, {} markdown)",
                self.tracked, self.markdown
            ),
            format!("# revision: {revision}"),
            format!("# worktree: {worktree}"),
            match &self.config_file {
                Some(p) => format!("# config: {p}"),
                None => "# config: none (no-config baseline)".to_string(),
            },
            format!(
                "# files: corpus={} plan={} unreadable={}",
                self.corpus_files, self.plan_files, self.unreadable
            ),
            format!("# lens {}: {}", untasked::LENS, self.untasked),
            format!("# lens contradiction: {}", self.contradiction),
        ]
    }

    /// The findings, one line each, after the header. Ends with a newline.
    pub fn render_lines(&self) -> String {
        // The revision and the tree state are header lines too: a report names the revision it read.
        let mut out: Vec<String> = self.header(FINDINGS_FORMAT, false);
        out.push(format!("# columns: {}", COLUMN_NAMES));
        out.extend(self.rows.iter().map(Row::line));
        out.join("\n") + "\n"
    }

    /// JSON lines: a header object, then one object per finding.
    pub fn render_jsonl(&self) -> String {
        let status = |s: &LensStatus| match s {
            LensStatus::Analysed(d) => serde_json::json!({"analysed": true, "detail": d}),
            LensStatus::NotAnalysed(r) => serde_json::json!({"analysed": false, "detail": r}),
        };
        let header = serde_json::json!({
            "type": "header",
            "format": FINDINGS_FORMAT,
            "project": self.project,
            "enumeration": {
                "method": "git ls-files",
                "tracked_files": self.tracked,
                "markdown_files": self.markdown,
            },
            "revision": self.revision,
            "worktree": if self.dirty { "dirty" } else { "clean" },
            "config": self.config_file,
            "inputs": {
                "corpus_files": self.corpus_files,
                "plan_files": self.plan_files,
                "unreadable_files": self.unreadable,
            },
            "lenses": {
                "untasked": status(&self.untasked),
                "contradiction": status(&self.contradiction),
            },
        });
        let mut out = vec![header.to_string()];
        for r in &self.rows {
            let mut object = serde_json::Map::new();
            object.insert("type".into(), "finding".into());
            // The column names are the keys; the line number is the one number.
            let text_fields = [
                &r.project, &r.file, &r.lens, &r.kind, &r.id, &r.refs, &r.search, &r.control,
                &r.text,
            ];
            let mut names = columns().into_iter();
            let mut texts = text_fields.into_iter();
            for (i, name) in names.by_ref().enumerate() {
                let value = if i == NUMBER_COLUMN {
                    serde_json::Value::from(r.line)
                } else {
                    serde_json::Value::from(texts.next().map(String::as_str))
                };
                object.insert(name.into(), value);
            }
            out.push(serde_json::Value::Object(object).to_string());
        }
        out.join("\n") + "\n"
    }

    /// Counts per `(lens, kind)` and nothing else: no path, no text, no id, no revision. A kind of a lens
    /// that did not run reads `n/a`, never 0.
    pub fn render_counts(&self) -> String {
        let mut out: Vec<String> = self.header(COUNTS_FORMAT, true);
        out.push("# columns: lens kind count".to_string());
        let mut kinds: BTreeMap<(String, String), String> = BTreeMap::new();
        for spec in [untasked::spec(), contradiction::spec()] {
            for kind in spec.kinds {
                let key = (spec.id.0.clone(), kind);
                let shown = self
                    .counts
                    .get(&key)
                    .map_or_else(|| "n/a".to_string(), ToString::to_string);
                kinds.insert(key, shown);
            }
        }
        out.extend(kinds.iter().map(|((l, k), n)| format!("{l}\t{k}\t{n}")));
        out.join("\n") + "\n"
    }
}

/// A findings file's rows, in the order the file has them.
pub fn parse_findings(text: &str) -> Result<Vec<Row>, ProduceError> {
    let mut rows = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != columns().len() {
            return fail(format!(
                "line {}: expected {} tab-separated fields, found {}",
                n + 1,
                columns().len(),
                f.len()
            ));
        }
        let Ok(number) = f[2].parse::<u32>() else {
            return fail(format!("line {}: `{}` is not a line number", n + 1, f[2]));
        };
        rows.push(Row {
            project: f[0].to_string(),
            file: f[1].to_string(),
            line: number,
            lens: f[3].to_string(),
            kind: f[4].to_string(),
            id: f[5].to_string(),
            refs: f[6].to_string(),
            search: f[7].to_string(),
            control: f[8].to_string(),
            text: f[9].to_string(),
        });
    }
    Ok(rows)
}

/// One kind's findings in judging order (SPEC-STEP1 5.2): sorted by `(project, file, line)`, shuffled with `seed`.
/// Findings that share a position are told apart by id, so the order does not depend on how the file listed them.
pub fn judging_rows(rows: &[Row], seed: u64) -> Vec<Row> {
    use crate::gate_b::{FindingRef, judging_order};
    let mut by_position: BTreeMap<FindingRef, Vec<&Row>> = BTreeMap::new();
    for r in rows {
        by_position
            .entry(FindingRef {
                project: r.project.clone(),
                file: r.file.clone(),
                line: r.line,
            })
            .or_default()
            .push(r);
    }
    for group in by_position.values_mut() {
        group.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.cmp(b)));
    }
    let refs: Vec<FindingRef> = by_position
        .iter()
        .flat_map(|(k, g)| std::iter::repeat_n(k.clone(), g.len()))
        .collect();
    let mut taken: BTreeMap<FindingRef, usize> = BTreeMap::new();
    judging_order(refs, seed)
        .into_iter()
        .map(|k| {
            let i = taken.entry(k.clone()).or_insert(0);
            let row = by_position[&k][*i];
            *i += 1;
            row.clone()
        })
        .collect()
}
