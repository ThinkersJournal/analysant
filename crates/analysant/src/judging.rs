// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gate B judging records (SPEC-STEP1 5.2, `docs/GATE-B-PROTOCOL.md` sections 3 to 5): the verdict table a judge keeps, and what
//! the stop rule, the report row and the second-judge agreement say about it. The judge decides every verdict; nothing here does.
//!
//! The pass line is a required argument of every function that needs it, and the agreement threshold is an `Option`: both are
//! CireSnave's (D1) and have no default. A threshold that is not given is reported as such, with the agreement figure beside it.

use crate::gate_b::{
    Reading, Stop, TARGET_JUDGED, agreement, reading, second_judge_selection, stop_state,
};
use crate::produce::{Row, escape, parse_findings};
use std::collections::{BTreeMap, BTreeSet};

pub const VERDICTS_FORMAT: &str = "analysant-verdicts v1";
pub const JUDGING_FORMAT: &str = "analysant-sample v1";
pub const REPORT_FORMAT: &str = "analysant-gate-b-report v1";

/// The class column's value where there is no class.
pub const NO_CLASS: &str = "-";

/// The separator of a list inside a line (a character, so it is not a one-word literal).
const COMMA: char = ',';

/// Why a judging record was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeError(pub String);

impl std::fmt::Display for JudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JudgeError {}

fn fail<T>(message: String) -> Result<T, JudgeError> {
    Err(JudgeError(message))
}

/// A verdict on one finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    True,
    False,
    /// Cannot be shown from the sources: counted false, as a lower bound.
    Unjudgeable,
}

impl Verdict {
    /// The word a verdict table uses.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::True => "TRUE",
            Self::False => "FALSE",
            Self::Unjudgeable => "UNJUDGEABLE",
        }
    }

    /// The verdict a word names (exactly the words [`Verdict::word`] gives).
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        [Self::True, Self::False, Self::Unjudgeable]
            .into_iter()
            .find(|v| v.word() == word)
    }

    /// Whether the verdict counts as a true positive.
    #[must_use]
    pub const fn counts_true(self) -> bool {
        matches!(self, Self::True)
    }
}

/// One judge's verdict on one finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub verdict: Verdict,
    /// The named noise class for a false verdict; [`NO_CLASS`] otherwise.
    pub class: String,
    pub reason: String,
    pub judge: String,
}

impl Entry {
    /// The rubric's shape (`docs/GATE-B-PROTOCOL.md` section 4): every verdict has a reason and a judge, a false verdict a named
    /// noise class, and nothing else a class.
    fn check(&self) -> Result<(), JudgeError> {
        if self.id.trim().is_empty() || self.judge.trim().is_empty() {
            return fail(format!(
                "a verdict needs a finding id and a judge (id `{}`, judge `{}`)",
                self.id, self.judge
            ));
        }
        if self.reason.trim().is_empty() {
            return fail(format!("{}: a verdict needs a reason", self.id));
        }
        let classed = self.class.trim() != NO_CLASS && !self.class.trim().is_empty();
        match (self.verdict, classed) {
            (Verdict::False, false) => fail(format!(
                "{}: a false verdict needs a named noise class",
                self.id
            )),
            (Verdict::True | Verdict::Unjudgeable, true) => fail(format!(
                "{}: only a false verdict carries a noise class, not `{}`",
                self.id, self.class
            )),
            _ => Ok(()),
        }
    }
}

/// Every judge's verdicts, in the order they were written. A second judge's verdict on an id is a second entry, never an overwrite.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerdictTable {
    entries: Vec<Entry>,
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('\\') | None => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    out
}

impl VerdictTable {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The entries, in file order.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Read a verdict table. Lines starting with `#` and empty lines are comments.
    ///
    /// # Errors
    /// A malformed line, a verdict that breaks the rubric's shape, or one judge judging one finding twice.
    pub fn parse(text: &str) -> Result<Self, JudgeError> {
        let mut table = Self::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let at = n + 1;
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() != 5 {
                return fail(format!(
                    "line {at}: expected 5 tab-separated fields, found {}",
                    f.len()
                ));
            }
            let Some(verdict) = Verdict::from_word(f[1]) else {
                return fail(format!(
                    "line {at}: `{}` is not a verdict (TRUE, FALSE or UNJUDGEABLE)",
                    f[1]
                ));
            };
            let entry = Entry {
                id: unescape(f[0]),
                verdict,
                class: unescape(f[2]),
                reason: unescape(f[3]),
                judge: unescape(f[4]),
            };
            table
                .add(entry)
                .map_err(|e| JudgeError(format!("line {at}: {e}")))?;
        }
        Ok(table)
    }

    /// The table as bytes: the same entries always give the same text.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = format!("# {VERDICTS_FORMAT}\n# columns: id verdict class reason judge\n");
        for e in &self.entries {
            let fields = [
                escape(&e.id),
                e.verdict.word().to_string(),
                escape(&e.class),
                escape(&e.reason),
                escape(&e.judge),
            ];
            out += &fields.join("\t");
            out.push('\n');
        }
        out
    }

    /// Append an entry. A refused entry changes nothing.
    ///
    /// # Errors
    /// The same refusals as [`VerdictTable::parse`].
    pub fn add(&mut self, entry: Entry) -> Result<(), JudgeError> {
        entry.check()?;
        if self
            .entries
            .iter()
            .any(|e| e.judge == entry.judge && e.id == entry.id)
        {
            return fail(format!(
                "judge {} has already judged {}: a verdict is not rewritten in place",
                entry.judge, entry.id
            ));
        }
        self.entries.push(entry);
        Ok(())
    }
}

/// One kind's findings in judging order, as `analysant sample` prints them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindOrder {
    pub kind: String,
    pub findings: usize,
    pub seed: u64,
    pub rows: Vec<Row>,
}

/// Read the output of `analysant sample`.
///
/// # Errors
/// A malformed kind header or finding line, positions that are not 1, 2, 3 ..., or one finding id listed twice in a kind.
pub fn parse_judging(text: &str) -> Result<Vec<KindOrder>, JudgeError> {
    let mut kinds: Vec<KindOrder> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        let at = n + 1;
        if let Some(rest) = line.strip_prefix("# kind ") {
            kinds.push(parse_kind_header(rest, at)?);
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(current) = kinds.last_mut() else {
            return fail(format!("line {at}: a finding before any `# kind` line"));
        };
        let Some((position, rest)) = line.split_once('\t') else {
            return fail(format!("line {at}: no position before the finding"));
        };
        let expected = current.rows.len() + 1;
        if position.parse::<usize>().ok() != Some(expected) {
            return fail(format!(
                "line {at}: position `{position}` where {expected} was expected"
            ));
        }
        let mut rows = parse_findings(rest).map_err(|e| JudgeError(format!("line {at}: {e}")))?;
        let Some(mut row) = rows.pop().filter(|_| rows.is_empty()) else {
            return fail(format!("line {at}: expected exactly one finding"));
        };
        // A finding id is a content id, so one boilerplate claim in three projects has one id. The judging
        // key is therefore `<project>:<id>` (what `judge append --id` and the blind sheet use).
        row.id = format!("{}:{}", row.project, row.id);
        if current.rows.iter().any(|r| r.id == row.id) {
            return fail(format!(
                "line {at}: finding {} is listed twice in {}",
                row.id, current.kind
            ));
        }
        current.rows.push(row);
    }
    for k in &kinds {
        if k.rows.len() != k.findings {
            return fail(format!(
                "{}: the header says {} findings, the file lists {}",
                k.kind,
                k.findings,
                k.rows.len()
            ));
        }
    }
    Ok(kinds)
}

fn parse_kind_header(rest: &str, at: usize) -> Result<KindOrder, JudgeError> {
    let Some((kind, fields)) = rest.split_once(": ") else {
        return fail(format!("line {at}: a kind header needs `kind: fields`"));
    };
    let mut findings = None;
    let mut seed = None;
    for field in fields.split(' ') {
        if let Some(v) = field.strip_prefix("findings=") {
            findings = v.parse::<usize>().ok();
        } else if let Some(v) = field.strip_prefix("seed=") {
            seed = v.parse::<u64>().ok();
        }
    }
    let (Some(findings), Some(seed)) = (findings, seed) else {
        return fail(format!(
            "line {at}: the header of {kind} needs numeric findings= and seed="
        ));
    };
    Ok(KindOrder {
        kind: kind.to_string(),
        findings,
        seed,
        rows: Vec::new(),
    })
}

/// The arguments a report needs. No field has a default.
#[derive(Debug, Clone, Copy)]
pub struct Params<'a> {
    pub judge: &'a str,
    pub second_judge: Option<&'a str>,
    /// The pass line in whole percent (D1).
    pub pass_percent: u32,
    /// Agreement below this many of the selected items voids the figure (D1); `None` while it is unset.
    pub void_below: Option<usize>,
    /// For a kind that stopped before 10 items could be selected (pm-ants ruling 2026-11-21): the figure holds when at least this
    /// whole percent of the selected items agree and at least this many were judged; `None` means `void_below` alone decides.
    pub stopped_rule: Option<StoppedRule>,
}

/// The agreement rule for a stopped kind: `percent` of the selected items must agree, and at least `min_judged` must exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoppedRule {
    pub percent: u32,
    pub min_judged: usize,
}

/// What the second judge's answers say about the first judge's figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgreementStatus {
    /// The threshold is not set: the number is reported and nothing is voided.
    ThresholdUnset,
    Holds,
    Voided,
    /// The second judge has not answered every selected item.
    Incomplete,
}

/// Second-judge agreement over the blind selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agreement {
    pub selected: usize,
    pub answered: usize,
    pub agree: usize,
    pub status: AgreementStatus,
}

/// One kind's row in the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindReport {
    pub kind: String,
    pub findings: usize,
    pub census: bool,
    /// Verdicts read up to the stop point (or all of them while judging continues).
    pub judged: usize,
    pub true_count: usize,
    pub false_count: usize,
    pub unjudgeable: usize,
    pub stop: Stop,
    /// Verdicts written after the stop point: kept in the table, not counted.
    pub beyond_stop: usize,
    /// 1-based position of the next finding to judge, while judging continues.
    pub next_position: Option<usize>,
    /// 30 judged out of a population large enough to sample.
    pub thirty_clause: bool,
    pub clears: bool,
    pub classes: Vec<(String, usize)>,
    pub agreement: Option<Agreement>,
}

/// The entries of one judge, by finding id.
fn by_judge<'t>(table: &'t VerdictTable, judge: &str) -> BTreeMap<&'t str, &'t Entry> {
    table
        .entries
        .iter()
        .filter(|e| e.judge == judge)
        .map(|e| (e.id.as_str(), e))
        .collect()
}

/// How many findings, from the first, the judge has judged; a verdict after a gap is an error: the order is the method.
fn judged_prefix(order: &KindOrder, own: &BTreeMap<&str, &Entry>) -> Result<usize, JudgeError> {
    let prefix = order
        .rows
        .iter()
        .take_while(|r| own.contains_key(r.id.as_str()))
        .count();
    if let Some(later) = order
        .rows
        .iter()
        .enumerate()
        .skip(prefix + 1)
        .find(|(_, r)| own.contains_key(r.id.as_str()))
    {
        return fail(format!(
            "{}: position {} is judged but position {} is not: findings are judged in the sample's order",
            order.kind,
            later.0 + 1,
            prefix + 1
        ));
    }
    Ok(prefix)
}

fn check_params(params: &Params<'_>) -> Result<(), JudgeError> {
    if params.pass_percent > 100 {
        return fail(format!(
            "the pass line is a percentage: {} is over 100",
            params.pass_percent
        ));
    }
    if params.second_judge == Some(params.judge) {
        return fail(format!(
            "the second judge must be someone other than {}",
            params.judge
        ));
    }
    Ok(())
}

/// The counted verdicts of one kind: where the stop rule ends them, and what they say.
struct Walk {
    judged: usize,
    prefix: usize,
    stop: Stop,
}

fn walk(
    order: &KindOrder,
    own: &BTreeMap<&str, &Entry>,
    pass_percent: u32,
) -> Result<Walk, JudgeError> {
    let prefix = judged_prefix(order, own)?;
    let mut not_true = 0;
    for (k, row) in order.rows.iter().take(prefix).enumerate() {
        if !own[row.id.as_str()].verdict.counts_true() {
            not_true += 1;
        }
        let state = stop_state(order.findings, k + 1, not_true, pass_percent);
        if state != Stop::Continue {
            return Ok(Walk {
                judged: k + 1,
                prefix,
                stop: state,
            });
        }
    }
    Ok(Walk {
        judged: prefix,
        prefix,
        stop: Stop::Continue,
    })
}

/// The findings the second judge re-judges blind: positions into the judged list, from the one seed the report states.
fn selection(order: &KindOrder, judged: usize) -> Vec<usize> {
    second_judge_selection(judged.min(TARGET_JUDGED), order.seed)
}

fn kind_report(
    order: &KindOrder,
    table: &VerdictTable,
    params: &Params<'_>,
) -> Result<KindReport, JudgeError> {
    let own = by_judge(table, params.judge);
    let w = walk(order, &own, params.pass_percent)?;
    let (mut true_count, mut false_count, mut unjudgeable) = (0, 0, 0);
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for row in order.rows.iter().take(w.judged) {
        let e = own[row.id.as_str()];
        match e.verdict {
            Verdict::True => true_count += 1,
            Verdict::False => {
                false_count += 1;
                *classes.entry(e.class.clone()).or_default() += 1;
            }
            Verdict::Unjudgeable => unjudgeable += 1,
        }
    }
    let census = matches!(reading(order.findings), Reading::Census { .. });
    let thirty_clause = !census && w.judged >= TARGET_JUDGED;
    let clears = thirty_clause
        && w.stop != Stop::PassLineUnreachable
        && true_count * 100 >= params.pass_percent as usize * w.judged;
    let agreement = params
        .second_judge
        .and_then(|second| second_judge_agreement(order, &own, table, second, &w, params));
    Ok(KindReport {
        kind: order.kind.clone(),
        findings: order.findings,
        census,
        judged: w.judged,
        true_count,
        false_count,
        unjudgeable,
        stop: w.stop,
        beyond_stop: w.prefix - w.judged,
        next_position: (w.stop == Stop::Continue).then_some(w.judged + 1),
        thirty_clause,
        clears,
        classes: classes.into_iter().collect(),
        agreement,
    })
}

fn second_judge_agreement(
    order: &KindOrder,
    own: &BTreeMap<&str, &Entry>,
    table: &VerdictTable,
    second: &str,
    w: &Walk,
    params: &Params<'_>,
) -> Option<Agreement> {
    let chosen = selection(order, w.judged);
    if chosen.is_empty() {
        return None;
    }
    let theirs = by_judge(table, second);
    let (mut first, mut other) = (Vec::new(), Vec::new());
    for idx in &chosen {
        let id = order.rows[*idx].id.as_str();
        if let Some(t) = theirs.get(id) {
            first.push(own[id].verdict.counts_true());
            other.push(t.verdict.counts_true());
        }
    }
    let (agree, answered) = agreement(&first, &other);
    let status = if answered < chosen.len() {
        AgreementStatus::Incomplete
    } else {
        match (params.stopped_rule, params.void_below) {
            (Some(r), _) if chosen.len() < crate::gate_b::SECOND_JUDGE_COUNT => {
                if chosen.len() >= r.min_judged && agree * 100 >= r.percent as usize * chosen.len()
                {
                    AgreementStatus::Holds
                } else {
                    AgreementStatus::Voided
                }
            }
            (_, None) => AgreementStatus::ThresholdUnset,
            (_, Some(t)) if agree < t => AgreementStatus::Voided,
            (_, Some(_)) => AgreementStatus::Holds,
        }
    };
    Some(Agreement {
        selected: chosen.len(),
        answered,
        agree,
        status,
    })
}

/// Report every kind.
///
/// # Errors
/// A verdict for an id no kind lists, a gap in the order judged, a pass line over 100, or the same person as both judges.
pub fn report_all(
    orders: &[KindOrder],
    table: &VerdictTable,
    params: &Params<'_>,
) -> Result<Vec<KindReport>, JudgeError> {
    check_params(params)?;
    let known: BTreeSet<&str> = orders
        .iter()
        .flat_map(|o| o.rows.iter().map(|r| r.id.as_str()))
        .collect();
    if let Some(stray) = table
        .entries
        .iter()
        .find(|e| !known.contains(e.id.as_str()))
    {
        return fail(format!(
            "{} judged {}, which no kind in the sample file lists",
            stray.judge, stray.id
        ));
    }
    orders
        .iter()
        .map(|o| kind_report(o, table, params))
        .collect()
}

fn percent_tenths(numerator: usize, denominator: usize) -> String {
    // Tenths of a percent, rounded half up, in integers: 3/7 is 42.9%, not 42.8%.
    let tenths = (numerator * 2000 / denominator).div_ceil(2);
    format!("{}.{}%", tenths / 10, tenths % 10)
}

fn stop_word(stop: Stop) -> &'static str {
    match stop {
        Stop::Continue => "continue",
        Stop::ThirtyJudged => "thirty-judged",
        Stop::PassLineUnreachable => "pass-line-unreachable",
        Stop::CensusComplete => "census-complete",
    }
}

fn agreement_cell(a: &Option<Agreement>) -> String {
    let Some(a) = a else {
        return "-".to_string();
    };
    let state = match a.status {
        AgreementStatus::ThresholdUnset => "threshold not set".to_string(),
        AgreementStatus::Holds => "stands".to_string(),
        AgreementStatus::Voided => "voided".to_string(),
        AgreementStatus::Incomplete => {
            format!("incomplete: {} of {} answered", a.answered, a.selected)
        }
    };
    format!("{} of {} ({state})", a.agree, a.selected)
}

/// The report as text: one row per kind, so the same verdicts always give the same bytes.
#[must_use]
pub fn render_report(reports: &[KindReport], params: &Params<'_>) -> String {
    let threshold = params
        .void_below
        .map_or_else(|| "not set".to_string(), |t| t.to_string());
    let mut out = format!(
        "# {REPORT_FORMAT}\n# judge: {}\n# second judge: {}\n# pass percent: {}\n# agreement threshold: {threshold}\n",
        params.judge,
        params.second_judge.unwrap_or("-"),
        params.pass_percent
    );
    out += "# columns: kind findings reading judged true false unjudgeable precision upper-bound stop beyond-stop next thirty-clause clears second-judge\n";
    for r in reports {
        let how = if r.census { "census" } else { "sample" };
        let precision = if r.judged == 0 {
            "-".to_string()
        } else {
            percent_tenths(r.true_count, r.judged)
        };
        let upper = if r.true_count == 0 && r.judged > 0 {
            format!("3/{}={}", r.judged, percent_tenths(3, r.judged))
        } else {
            "-".to_string()
        };
        let next = r
            .next_position
            .map_or_else(|| "-".to_string(), |n| n.to_string());
        let fields = [
            r.kind.clone(),
            r.findings.to_string(),
            how.to_string(),
            r.judged.to_string(),
            r.true_count.to_string(),
            r.false_count.to_string(),
            r.unjudgeable.to_string(),
            precision,
            upper,
            stop_word(r.stop).to_string(),
            r.beyond_stop.to_string(),
            next,
            r.thirty_clause.to_string(),
            r.clears.to_string(),
            agreement_cell(&r.agreement),
        ];
        out += &fields.join("\t");
        out.push('\n');
        if !r.classes.is_empty() {
            let named: Vec<String> = r.classes.iter().map(|(c, n)| format!("{c}={n}")).collect();
            out += &format!(
                "# classes {}: {}\n",
                r.kind,
                named.join(&format!("{COMMA} "))
            );
        }
    }
    out
}

/// The findings the second judge re-judges blind, with their 1-based positions: no verdict, class or reason.
///
/// # Errors
/// The same refusals as [`report_all`]'s for this kind.
pub fn second_judge_sheet<'a>(
    order: &'a KindOrder,
    table: &VerdictTable,
    params: &Params<'_>,
) -> Result<Vec<(usize, &'a Row)>, JudgeError> {
    check_params(params)?;
    let own = by_judge(table, params.judge);
    let w = walk(order, &own, params.pass_percent)?;
    Ok(selection(order, w.judged)
        .into_iter()
        .map(|idx| (idx + 1, &order.rows[idx]))
        .collect())
}
