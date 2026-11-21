// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gate B verdict capture and the stop-rule tracker (`analysant::judging`, `analysant judge ...`), written before the code.
//! Every test states a concrete verdict table and the exact figures it must produce: the stop at the 7th false positive on an
//! 80% pass line, UNJUDGEABLE counted false, a census that cannot meet the 30 clause, agreement 8 of 10 against 7 of 10 under a
//! threshold, and a second judge's verdict kept beside the first's.

use analysant::gate_b::{SECOND_JUDGE_COUNT, Stop, second_judge_selection};
use analysant::judging::{
    AgreementStatus, Entry, JudgeError, KindOrder, KindReport, Params, Verdict, VerdictTable,
    parse_judging, render_report, report_all, second_judge_sheet,
};
use analysant::produce::Row;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const KIND: &str = "contradiction/pair";
const SEED: u64 = 2026;

fn row(i: usize) -> Row {
    Row {
        project: "demo".into(),
        file: "a.md".into(),
        line: u32::try_from(i).unwrap(),
        lens: "contradiction".into(),
        kind: "pair".into(),
        id: format!("f-{i:04}"),
        refs: "-".into(),
        search: "-".into(),
        control: "-".into(),
        text: format!("finding number {i}"),
    }
}

/// The text `analysant sample` prints for one kind of `n` findings listed in id order.
fn sample_text(n: usize) -> String {
    let mut out = String::from("# analysant-sample v1\n");
    let how = if n < 30 { "census" } else { "sample" };
    out += &format!("# kind {KIND}: findings={n} reading={how} seed={SEED}\n");
    for i in 1..=n {
        out += &format!("{i}\t{}\n", row(i).line());
    }
    out
}

fn order(n: usize) -> Vec<KindOrder> {
    parse_judging(&sample_text(n)).expect("a sample file")
}

fn entry(position: usize, verdict: Verdict, judge: &str) -> Entry {
    let class = if verdict == Verdict::False {
        "scoped negation"
    } else {
        "-"
    };
    Entry {
        id: format!("demo:f-{position:04}"),
        verdict,
        class: class.into(),
        reason: format!("reason {position}"),
        judge: judge.into(),
    }
}

fn table(verdicts: &[(usize, Verdict)], judge: &str) -> VerdictTable {
    let mut t = VerdictTable::new();
    for (p, v) in verdicts {
        t.add(entry(*p, *v, judge)).expect("a valid entry");
    }
    t
}

fn params(judge: &str) -> Params<'_> {
    Params {
        judge,
        second_judge: None,
        pass_percent: 80,
        void_below: None,
    }
}

fn one(orders: &[KindOrder], t: &VerdictTable, p: &Params<'_>) -> KindReport {
    let mut r = report_all(orders, t, p).expect("a report");
    assert_eq!(r.len(), 1);
    r.remove(0)
}

use Verdict::{False as F, True as T, Unjudgeable as U};

// ---- the stop rule ----------------------------------------------------------------------------------------------------

#[test]
fn judging_stops_at_exactly_the_seventh_false_positive_on_an_80_percent_line() {
    // Positions 1..=15. False at 2, 4, 5, 7, 8, 9, 12: the 7th false is position 12.
    let falses = [2, 4, 5, 7, 8, 9, 12];
    let verdicts: Vec<(usize, Verdict)> = (1..=15)
        .map(|p| (p, if falses.contains(&p) { F } else { T }))
        .collect();
    let r = one(&order(40), &table(&verdicts, "j1"), &params("j1"));
    assert_eq!(r.stop, Stop::PassLineUnreachable);
    assert_eq!(r.judged, 12, "counted up to the stop point");
    assert_eq!((r.true_count, r.false_count, r.unjudgeable), (5, 7, 0));
    assert_eq!(r.beyond_stop, 3, "positions 13..=15 are kept, not counted");
    assert_eq!(r.next_position, None);
    assert!(!r.clears);
}

#[test]
fn the_sixth_false_positive_does_not_stop_judging() {
    let falses = [2, 4, 5, 7, 8, 9];
    let verdicts: Vec<(usize, Verdict)> = (1..=10)
        .map(|p| (p, if falses.contains(&p) { F } else { T }))
        .collect();
    let r = one(&order(40), &table(&verdicts, "j1"), &params("j1"));
    assert_eq!(r.stop, Stop::Continue);
    assert_eq!((r.judged, r.false_count, r.beyond_stop), (10, 6, 0));
    assert_eq!(r.next_position, Some(11));
}

#[test]
fn six_false_in_thirty_judged_is_a_sample_that_clears_80_percent() {
    let falses = [3, 9, 14, 20, 25, 30];
    let verdicts: Vec<(usize, Verdict)> = (1..=30)
        .map(|p| (p, if falses.contains(&p) { F } else { T }))
        .collect();
    let r = one(&order(40), &table(&verdicts, "j1"), &params("j1"));
    assert_eq!(r.stop, Stop::ThirtyJudged);
    assert_eq!((r.judged, r.true_count, r.false_count), (30, 24, 6));
    assert!(r.thirty_clause && r.clears, "24 of 30 is 80%");
    assert_eq!(
        r.classes,
        vec![("scoped negation".to_string(), 6)],
        "the named classes of the false verdicts"
    );
    // The same 24 of 30 does not clear a 90% line (the 4th false position is the stop there).
    let strict = Params {
        pass_percent: 90,
        ..params("j1")
    };
    let r90 = one(&order(40), &table(&verdicts, "j1"), &strict);
    assert_eq!(r90.stop, Stop::PassLineUnreachable);
    assert_eq!(r90.judged, 20, "the 4th false verdict is position 20");
    assert!(!r90.clears);
}

#[test]
fn unjudgeable_counts_false_and_can_be_the_seventh() {
    // Six FALSE (positions 1..=6) and then an UNJUDGEABLE at 7: that is the 7th false-or-unjudgeable.
    let mut verdicts: Vec<(usize, Verdict)> = (1..=6).map(|p| (p, F)).collect();
    verdicts.push((7, U));
    verdicts.push((8, T));
    let t = table(&verdicts, "j1");
    let r = one(&order(40), &t, &params("j1"));
    assert_eq!(r.stop, Stop::PassLineUnreachable);
    assert_eq!(r.judged, 7);
    assert_eq!((r.true_count, r.false_count, r.unjudgeable), (0, 6, 1));
    assert_eq!(r.beyond_stop, 1);
    // An UNJUDGEABLE verdict never counts as true: with 29 true and one UNJUDGEABLE the figure is 29 of 30.
    let mut v: Vec<(usize, Verdict)> = (1..=29).map(|p| (p, T)).collect();
    v.push((30, U));
    let r = one(&order(40), &table(&v, "j1"), &params("j1"));
    assert_eq!((r.true_count, r.unjudgeable, r.judged), (29, 1, 30));
}

#[test]
fn a_census_is_read_whole_and_cannot_meet_the_thirty_clause() {
    let verdicts: Vec<(usize, Verdict)> = (1..=12).map(|p| (p, T)).collect();
    let r = one(&order(12), &table(&verdicts, "j1"), &params("j1"));
    assert!(r.census);
    assert_eq!(r.stop, Stop::CensusComplete);
    assert_eq!((r.judged, r.true_count, r.false_count), (12, 12, 0));
    assert!(!r.thirty_clause, "12 findings can never be 30 judged");
    assert!(!r.clears, "a census cannot clear the line");
}

#[test]
fn a_census_with_no_true_finding_reports_the_rule_of_three_bound() {
    let verdicts: Vec<(usize, Verdict)> = (1..=6).map(|p| (p, F)).collect();
    let r = one(&order(12), &table(&verdicts, "j1"), &params("j1"));
    assert_eq!((r.true_count, r.judged), (0, 6));
    let text = render_report(&[r], &params("j1"));
    assert!(
        text.contains("3/6=50.0%"),
        "the one-sided 95% upper bound is 3/n: {text}"
    );
}

// ---- the report's text ------------------------------------------------------------------------------------------------

#[test]
fn the_report_is_exact_text() {
    let falses = [2, 4];
    let verdicts: Vec<(usize, Verdict)> = (1..=5)
        .map(|p| (p, if falses.contains(&p) { F } else { T }))
        .collect();
    let p = params("j1");
    let r = one(&order(12), &table(&verdicts, "j1"), &p);
    let expect = "\
# analysant-gate-b-report v1
# judge: j1
# second judge: -
# pass percent: 80
# agreement threshold: not set
# columns: kind findings reading judged true false unjudgeable precision upper-bound stop beyond-stop next thirty-clause clears second-judge
contradiction/pair\t12\tcensus\t5\t3\t2\t0\t60.0%\t-\tcontinue\t0\t6\tfalse\tfalse\t-
# classes contradiction/pair: scoped negation=2
";
    assert_eq!(render_report(&[r], &p), expect);
}

// ---- second judge -----------------------------------------------------------------------------------------------------

/// 30 judged, all true by the first judge; the second judge answers the 10 selected items with `disagree` of them false.
fn with_second(disagree: usize) -> (Vec<KindOrder>, VerdictTable) {
    let mut t = table(&(1..=30).map(|p| (p, T)).collect::<Vec<_>>(), "j1");
    // Independent anchor: the fixed vector in gate_b's tests, 0-based indices into the 30 judged.
    let selected = second_judge_selection(30, SEED);
    assert_eq!(selected, [0, 5, 6, 9, 16, 18, 19, 23, 25, 27]);
    assert_eq!(selected.len(), SECOND_JUDGE_COUNT);
    for (n, idx) in selected.iter().enumerate() {
        let v = if n < disagree { F } else { T };
        t.add(entry(idx + 1, v, "j2")).unwrap();
    }
    (order(40), t)
}

#[test]
fn agreement_eight_of_ten_holds_and_seven_of_ten_voids_under_a_threshold_of_eight() {
    let p = Params {
        second_judge: Some("j2"),
        void_below: Some(8),
        ..params("j1")
    };
    let (o8, t8) = with_second(2);
    let a8 = one(&o8, &t8, &p).agreement.expect("agreement");
    assert_eq!(
        (a8.selected, a8.answered, a8.agree, a8.status),
        (10, 10, 8, AgreementStatus::Holds)
    );
    let (o7, t7) = with_second(3);
    let a7 = one(&o7, &t7, &p).agreement.expect("agreement");
    assert_eq!(
        (a7.selected, a7.answered, a7.agree, a7.status),
        (10, 10, 7, AgreementStatus::Voided)
    );
}

#[test]
fn without_a_threshold_the_number_is_reported_and_nothing_is_voided() {
    let p = Params {
        second_judge: Some("j2"),
        void_below: None,
        ..params("j1")
    };
    let (o, t) = with_second(3);
    let r = one(&o, &t, &p);
    let a = r.agreement.clone().expect("agreement");
    assert_eq!((a.agree, a.status), (7, AgreementStatus::ThresholdUnset));
    let text = render_report(&[r], &p);
    assert!(text.contains("7 of 10 (threshold not set)"), "{text}");
}

#[test]
fn an_unanswered_selected_item_makes_the_agreement_incomplete() {
    let p = Params {
        second_judge: Some("j2"),
        void_below: Some(8),
        ..params("j1")
    };
    let (o, t) = with_second(0);
    // Rebuild the table without the second judge's last answer.
    let mut short = VerdictTable::new();
    let last = t.entries().last().unwrap().clone();
    for e in t.entries().iter().filter(|e| **e != last) {
        short.add(e.clone()).unwrap();
    }
    let a = one(&o, &short, &p).agreement.expect("agreement");
    assert_eq!((a.selected, a.answered, a.agree), (10, 9, 9));
    assert_eq!(a.status, AgreementStatus::Incomplete);
}

#[test]
fn the_blind_sheet_lists_the_selected_findings_without_any_verdict() {
    let p = Params {
        second_judge: Some("j2"),
        ..params("j1")
    };
    let (o, t) = with_second(0);
    let sheet = second_judge_sheet(&o[0], &t, &p).expect("a sheet");
    let positions: Vec<usize> = sheet.iter().map(|(n, _)| *n).collect();
    assert_eq!(positions, [1, 6, 7, 10, 17, 19, 20, 24, 26, 28]);
    assert_eq!(sheet[0].1.id, "demo:f-0001");
}

// ---- the table --------------------------------------------------------------------------------------------------------

#[test]
fn a_second_judges_verdict_on_the_same_id_is_kept_beside_the_first_not_over_it() {
    let mut t = VerdictTable::new();
    t.add(entry(1, T, "j1")).unwrap();
    let mut other = entry(1, F, "j2");
    other.reason = "the second reader disagrees".into();
    t.add(other.clone()).unwrap();
    assert_eq!(t.entries().len(), 2);
    assert_eq!(t.entries()[0].verdict, T);
    assert_eq!(t.entries()[1], other);
    // The same judge cannot judge the same finding twice (a correction is a new, dated decision, not a silent rewrite).
    let again = t.add(entry(1, F, "j1")).unwrap_err();
    assert!(
        again.0.contains("j1") && again.0.contains("demo:f-0001"),
        "{again}"
    );
    assert_eq!(t.entries().len(), 2);
    // The first judge's figure ignores the second judge's contrary verdict.
    let r = one(&order(12), &t, &params("j1"));
    assert_eq!((r.judged, r.true_count, r.false_count), (1, 1, 0));
}

#[test]
fn a_table_round_trips_and_escapes_what_would_break_a_line() {
    let mut t = VerdictTable::new();
    let mut e = entry(1, F, "j1");
    e.reason = "tab\there\nnewline and back\\slash".into();
    t.add(e).unwrap();
    t.add(entry(2, T, "j1")).unwrap();
    t.add(entry(3, U, "j2")).unwrap();
    let text = t.render();
    assert!(text.starts_with("# analysant-verdicts v1\n"));
    assert_eq!(text.lines().filter(|l| !l.starts_with('#')).count(), 3);
    assert_eq!(VerdictTable::parse(&text).unwrap(), t);
    assert_eq!(
        VerdictTable::new().render(),
        "# analysant-verdicts v1\n# columns: id verdict class reason judge\n"
    );
}

#[test]
fn a_verdict_that_breaks_the_rubric_is_refused() {
    let mut t = VerdictTable::new();
    let mut no_class = entry(1, F, "j1");
    no_class.class = "-".into();
    assert!(t.add(no_class).unwrap_err().0.contains("noise class"));
    let mut class_on_true = entry(1, T, "j1");
    class_on_true.class = "scoped negation".into();
    assert!(t.add(class_on_true).is_err());
    let mut no_reason = entry(1, T, "j1");
    no_reason.reason = "  ".into();
    assert!(t.add(no_reason).unwrap_err().0.contains("reason"));
    let mut no_judge = entry(1, T, "j1");
    no_judge.judge = String::new();
    assert!(t.add(no_judge).is_err());
    assert!(t.entries().is_empty(), "a refused entry is not kept");
    // A malformed file line is refused with its line number.
    let bad =
        VerdictTable::parse("# analysant-verdicts v1\nf-0001\tMAYBE\t-\twhy\tj1\n").unwrap_err();
    assert!(bad.0.contains("line 2") && bad.0.contains("MAYBE"), "{bad}");
    let short = VerdictTable::parse("f-0001\tTRUE\n").unwrap_err();
    assert!(short.0.contains("line 1"), "{short}");
}

#[test]
fn judging_out_of_order_or_naming_an_unknown_finding_is_refused() {
    // Position 3 judged while position 2 is not: the sample's order is the method, so a gap is an error.
    let gap = table(&[(1, T), (3, T)], "j1");
    let e: JudgeError = report_all(&order(40), &gap, &params("j1")).unwrap_err();
    assert!(e.0.contains("position 2"), "{e}");
    let mut unknown = table(&[(1, T)], "j1");
    let mut stray = entry(1, T, "j1");
    stray.id = "demo:f-9999".into();
    unknown.add(stray).unwrap();
    let e = report_all(&order(40), &unknown, &params("j1")).unwrap_err();
    assert!(e.0.contains("demo:f-9999"), "{e}");
}

#[test]
fn a_sample_file_that_lists_one_finding_twice_is_refused() {
    let mut text = sample_text(3);
    text += &format!("4\t{}\n", row(2).line());
    let e = parse_judging(&text).unwrap_err();
    assert!(e.0.contains("demo:f-0002"), "{e}");
    assert!(
        parse_judging("# analysant-sample v1\n# kind x: findings=banana reading=census seed=1\n")
            .is_err()
    );
}

#[test]
fn nothing_judged_yet_is_continue_with_position_one_next() {
    let r = one(&order(40), &VerdictTable::new(), &params("j1"));
    assert_eq!(
        (r.judged, r.stop, r.next_position),
        (0, Stop::Continue, Some(1))
    );
    assert!(!r.clears);
}

// ---- the command ------------------------------------------------------------------------------------------------------

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("analysant-judge-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_analysant"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("the binary runs")
}

fn out(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn err(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}

fn add(dir: &Path, id: &str, verdict: &str, class: &str, judge: &str) -> Output {
    run(
        dir,
        &[
            "judge",
            "append",
            "verdicts.tsv",
            "--id",
            id,
            "--verdict",
            verdict,
            "--class",
            class,
            "--reason",
            "read against the source",
            "--judge",
            judge,
        ],
    )
}

#[test]
fn the_command_appends_verdicts_and_reports_the_stop() {
    let s = Scratch::new("flow");
    fs::write(s.path().join("sample.tsv"), sample_text(40)).unwrap();
    // Seven false verdicts in a row, then a true one that lies beyond the stop.
    for n in 1..=7 {
        let o = add(
            s.path(),
            &format!("demo:f-{n:04}"),
            "FALSE",
            "bare label",
            "j1",
        );
        assert!(o.status.success(), "{}", err(&o));
        assert_eq!(out(&o), "");
    }
    assert!(
        add(s.path(), "demo:f-0008", "TRUE", "-", "j1")
            .status
            .success()
    );
    let table = fs::read_to_string(s.path().join("verdicts.tsv")).unwrap();
    assert!(table.starts_with("# analysant-verdicts v1\n"));
    assert_eq!(table.lines().filter(|l| !l.starts_with('#')).count(), 8);
    let report = |extra: &[&str]| {
        let mut args = vec![
            "judge",
            "report",
            "sample.tsv",
            "verdicts.tsv",
            "--judge",
            "j1",
            "--pass-percent",
            "80",
        ];
        args.extend(extra);
        run(s.path(), &args)
    };
    let o = report(&[]);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    assert!(
        text.contains("contradiction/pair\t40\tsample\t7\t0\t7\t0\t0.0%\t3/7=42.9%\tpass-line-unreachable\t1\t-\tfalse\tfalse\t-\n"),
        "{text}"
    );
    assert_eq!(text, out(&report(&[])), "deterministic");
    // The same judge twice on one finding is refused, and the table is unchanged.
    let dup = add(s.path(), "demo:f-0001", "TRUE", "-", "j1");
    assert_eq!(dup.status.code(), Some(2));
    assert!(err(&dup).contains("demo:f-0001"));
    assert_eq!(
        fs::read_to_string(s.path().join("verdicts.tsv")).unwrap(),
        table
    );
}

#[test]
fn the_pass_line_has_no_default() {
    let s = Scratch::new("noline");
    fs::write(s.path().join("sample.tsv"), sample_text(5)).unwrap();
    fs::write(s.path().join("verdicts.tsv"), VerdictTable::new().render()).unwrap();
    let o = run(
        s.path(),
        &[
            "judge",
            "report",
            "sample.tsv",
            "verdicts.tsv",
            "--judge",
            "j1",
        ],
    );
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("--pass-percent"), "{}", err(&o));
    assert_eq!(out(&o), "");
    let o = run(
        s.path(),
        &[
            "judge",
            "report",
            "sample.tsv",
            "verdicts.tsv",
            "--pass-percent",
            "80",
        ],
    );
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("--judge"));
    let o = run(
        s.path(),
        &[
            "judge",
            "report",
            "sample.tsv",
            "verdicts.tsv",
            "--judge",
            "j1",
            "--pass-percent",
            "101",
        ],
    );
    assert_eq!(o.status.code(), Some(2), "a percentage is at most 100");
}

#[test]
fn the_sheet_command_prints_the_blind_rows_and_no_verdicts() {
    let s = Scratch::new("sheet");
    fs::write(s.path().join("sample.tsv"), sample_text(40)).unwrap();
    for n in 1..=30 {
        assert!(
            add(s.path(), &format!("demo:f-{n:04}"), "TRUE", "-", "j1")
                .status
                .success()
        );
    }
    let o = run(
        s.path(),
        &[
            "judge",
            "sheet",
            "sample.tsv",
            "verdicts.tsv",
            "--judge",
            "j1",
            "--pass-percent",
            "80",
            "--kind",
            KIND,
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    let positions: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').next().unwrap())
        .collect();
    assert_eq!(
        positions,
        ["1", "6", "7", "10", "17", "19", "20", "24", "26", "28"]
    );
    assert!(
        !text.contains("TRUE") && !text.contains("reason") && !text.contains("read against"),
        "blind: {text}"
    );
    assert!(
        text.contains("finding number 6"),
        "the finding text is shown: {text}"
    );
}

/// One boilerplate claim in two projects has one content id; the judging key is `<project>:<id>`, so both are listed and judged apart.
#[test]
fn the_same_content_id_in_two_projects_is_two_findings() {
    let mut a = row(1);
    let mut b = row(1);
    a.project = "alpha".into();
    b.project = "beta".into();
    let text = format!(
        "# analysant-sample v1\n# kind {KIND}: findings=2 reading=census seed={SEED}\n1\t{}\n2\t{}\n",
        a.line(),
        b.line()
    );
    let kinds = parse_judging(&text).expect("two projects, one id, is not a duplicate");
    assert_eq!(kinds[0].rows[0].id, "alpha:f-0001");
    assert_eq!(kinds[0].rows[1].id, "beta:f-0001");
}
