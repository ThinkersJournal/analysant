# Gate B protocol: the precision gate

Status: **prep, 2026-11-20 (lane analysant).** The method is SPEC-STEP1 section 5.2; this file is the part a judge needs open while judging.
The pass line is set (D1, CireSnave 2026-11-21, SPEC-STEP1 5.2): 80% true over 30 judged per kind, the second judge a blind 10 of 30 with `--void-below 8`, a 30 lane-day cap on Gate B.
The code is `analysant::gate_b` (`crates/analysant/src/gate_b.rs`); nothing in it judges a finding. The findings producer is the `analysant` binary (section 7).

## 1. Population (D2, PM ruling 2026-11-17)

| Project | Kind of project | Visibility | Rule |
|---|---|---|---|
| KISS | docs-heavy | see its repo | findings may be quoted |
| PersistAnt | Rust library | public | findings may be quoted |
| HumboldtUnifiedKidTracker | Rust application | **PRIVATE** | local clone only; publish **aggregate counts only** (no file names, lines or text) |

Fuel is the oracle and the tuning set: **never scored here.** Do not reuse CodeRipper's sample repos (D2).
Every project is read at one recorded revision (section 5); judging uses the sources at *that* revision.

## 2. The shuffle, written down so it can be rebuilt in any language

1. List one kind's findings as `(project, file, line)`; sort ascending, field by field (strings bytewise, line numerically).
2. SplitMix64 with 64-bit state `s = seed`. Next output: `s += 0x9E3779B97F4A7C15 (mod 2^64)`; `z = s`;
   `z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9`; `z = (z ^ (z >> 27)) * 0x94D049BB133111EB` (both mod 2^64); output `z ^ (z >> 31)`.
3. Bounded draw in `0..b`: `t = (2^64 - b) mod b`; draw until the output `r >= t`; the result is `r mod b`.
4. Fisher-Yates from the back: for `i = n-1` down to 1, `j = bounded(i + 1)`, swap positions `i` and `j`.
5. Judge in the resulting order. The seed is stated in the report header and **is not reused** after a tuning round (section 6).

Fixed vectors (in the tests, and checked against an independent implementation): seed 0 gives `0xE220A8397B1DCDAF` first; shuffling `0..10`
with seed 2026 gives `[5, 0, 9, 2, 4, 7, 3, 6, 8, 1]`.

## 3. When to stop

- **Fewer than 30 findings in a kind: census.** Read all of them. A census cannot satisfy the "30 sampled" clause and is reported as a census.
- Otherwise judge until **30 are judged, or the pass line is unreachable**, whichever is first. At pass line `p` percent the stop is
  `floor(30 * (100 - p) / 100) + 1` false positives, in integers (at 80: the 7th). UNJUDGEABLE counts as false for this.
- **Second judge:** another lane or CireSnave re-judges **10 of the 30 blind** (verdict and reason hidden). The 10 are the first ten of a
  shuffle of positions `0..judged` with seed `report seed XOR 0x5EC0D0DE0A11CE55`, returned in ascending order. The report states the agreement
  as `k of 10`. The threshold below which the figure is void is not set (D1 territory); the report gives the number and says so.
- **Model-judged findings** (3.3) are excluded from this figure and measured on their own.

## 4. Rubric

Judge against the sources at the finding's own revision, and write a **reason** for every verdict in a table anyone can dispute.

- **TRUE:** the discrepancy or absence the finding states is real at that revision **and** a reader acting on it fixes or learns something real
  (a document, plan or piece of code is wrong, stale, or missing a thing it should have).
- **FALSE, with a named noise class.** Start from the spike's six (`spike-report-2026-11-16.md:330-364`):
  1. one generic word carrying the pair across unrelated senses;
  2. the denied token is absent from the positive side (the shared tokens are the topic);
  3. a bare label as the positive side;
  4. scoped negation (the denial is about one layer, the assertion about another);
  5. a scope note read as a refusal;
  6. a copula classification read as a capability denial.
  A new class gets a name and a one-line definition in the report before it is used twice.
- **UNJUDGEABLE** (cannot be shown from the sources): **counted FALSE, as a lower bound.**

Tuning discipline: a rule tuned on a sample invalidates that sample. The re-measure uses a fresh seed and projects not seen while tuning.

## 5. Report skeleton (copy per run)

```
Gate B report  | date | judge | second judge
seed (u64)     | pass line p (percent, D1) | lens version (analysant git SHA, crate version)
sense tables / alias tables used (path @ SHA): ...
projects (D2): <name> @ <SHA>   x3     # Humboldt: aggregate counts only below
per finding kind:
  kind | findings | reading (census|sample) | judged | true | false (named class counts) | unjudgeable | precision
  one-sided 95% upper bound when true = 0: 3/n          (rule of three)
  stop reason (30 judged | pass line unreachable | census complete)
  second judge: agreement k of 10, threshold ...
  disposition (clears | does not clear | not shown in health_report)
verdict table (kinds with a quotable project only): position | project | file:line | verdict | noise class | reason
```

## 6. Seed-fit list: recorded inputs to Gate B

The slice 4 seed sense table (`crates/analysant/tests/fixtures/fuel-project/senses.concepts`) was built from the 21 generic-word pairs it then
suppressed, so its result on Fuel is **fidelity, not generalisation** (pm-ants, 2026-11-19: no further tuning on those pairs). Gate B on the
three projects above is the independent check. Record these when reading, and do not tune on what is seen there without re-measuring fresh.

**Seed-fit, depends on a hand-chosen cue:**
- **4 "budget" pairs** (doc 15's C-5 resource budget, `15:3`, `15:116`, `15:120`, `15:134`, against `2026-10-25-larger-than-vram.md:25`). The
  report calls them "a gap, not a contradiction" of the same artifact; the table removes them only through the sense `budget-consumer-entitlement`,
  cued on the word `consumer` in the section path. Without that cue they stay.
- **3 "lifecycle" pairs** (`15:265` against `03:208`, `06:201`, `10:31`; the positive sides are links to `14-lifecycle.md`). Removed only by an `md`
  cue. Without it they stay.

**Not handled by any rule: five of the six noise classes, 15 pairs.** The 16 pairs still found on Fuel are these 15 plus the 1 real one (the
cancellation pair). The README and the slice 4 spec paragraph name four classes and omit the copula class; this list is the complete one.
- the denied token is absent from the positive side (6 pairs);
- a bare label as the positive side (2);
- scoped negation (4);
- a scope note read as a refusal (2);
- a copula classification read as a capability denial (1).

Expect those classes on the new projects. If one dominates a kind there, it becomes the next rule or table work; if tables cannot fix it, that
is the evidence D6 waits on (no model until Gate B names such a class).

## 7. Baseline runs: the procedure (written 2026-11-20; run 1 made 2026-11-21, seed 20261121)

D1 was answered 2026-11-21. Nothing is tuned on these projects without spending the sample (step 11). **Stopped kind (PM-accepted 2026-11-21):** if a kind stops before 10 can be selected, the second judge re-judges all judged items blind; the figure holds at 80% agreement of the judged and at least 5 judged: `report ... --void-below 8 --stopped-agree-percent 80 --stopped-min-judged 5`. Run 1: 7 of 7 per kind, both held.
**Judging key:** a finding id is a content id, so
one boilerplate claim in three projects has one id; judging rows are keyed `<project>:<id>`, and `judge append --id` takes that form. The producer is the
`analysant` binary (`crates/analysant/src/bin/analysant.rs`, library `analysant::produce`); it has only been run on this repository and on
its own test fixtures.

A **no-config baseline** means: no `.analysant/project.toml`, so no alias table, no sense table, no ownership convention, no extra
pairing stopwords. Whatever the lenses find with the pack's own data is the figure; a convention added afterwards is tuning.

1. **Fix the tool.** Build `analysant` from a recorded revision (`git rev-parse HEAD` of this repo, crate version) and write both in the
   report header. Do not rebuild between projects.
2. **Fix each project's revision.** Use a **clean** clone at one recorded SHA (`git status` clean). The producer enumerates with
   `git ls-files` only, so untracked files are never read, and it refuses a directory that is not a git work tree.
3. **Check there is no project file.** `.analysant/project.toml` must not exist in the clone; if the project has one, the run is not a
   baseline (note it, and run from an export without it). The header must say `# config: none (no-config baseline)`.
4. **Choose the plan before looking.** The untasked check needs a plan (a tracked document whose lines say what is tasked). Name the
   project's own roadmap or TODO file once, write the choice in the report, and pass it with `--plan <path>`. A project with none gets the
   untasked lens "not analysed" (the header says so; that is a result, not a failure). The contradiction lens needs no plan and always runs.
5. **Run it.** One line per project, with the names fixed in advance (`--project` is the name in the project column):
   `analysant findings <clone> --project <name> --plan <plan path> > findings-<name>.tsv`
   Check the header: `# revision:` equals the recorded SHA, `# worktree: clean`, `unreadable=0`, `# enumeration: git ls-files`.
6. **Check it is deterministic.** Run it a second time and compare the two files byte for byte. A difference stops the procedure.
7. **Aggregate counts.** `analysant counts <clone> --project <name> --plan <plan path>` prints counts per lens and kind and nothing else: no
   path, no text, no id, no revision (those header lines read `withheld`). **For Humboldt this is the only output that may leave the
   machine or appear in any report, issue, PR or message.** Its `findings-humboldt.tsv` stays local.
8. **Sample per kind over the three projects.** Concatenate the data lines (not the `#` lines) of the three files into one file
   (`grep -hv '^#' findings-*.tsv > all.tsv`), then `analysant sample all.tsv --seed <S> --kind <lens>/<kind>`. The seed is chosen once, written
   in the report header, and **not reused after a tuning round**. The output is that kind in judging order, numbered from 1, each row being
   the whole finding line (so the judge has the text); `reading=census` means fewer than 30 findings and the whole kind is read. Findings at
   the same `(project, file, line)` are ordered by id, so the order does not depend on how the file listed them.
9. **Judge** per sections 3 and 4 of this file: read in order, write a verdict and a reason per row, stop at 30 judged or at the false-positive
   count for the pass line (`analysant::gate_b::stop_state`). The judge decides every verdict; the tool keeps the table and the stop rule:
   `analysant judge append verdicts.tsv --id <finding id> --verdict TRUE|FALSE|UNJUDGEABLE [--class <noise class>] --reason <why> --judge <you>`
   writes one line (a FALSE verdict needs a named class, a TRUE or UNJUDGEABLE one has none, every verdict has a reason; one judge cannot
   judge one finding twice, and a second judge's verdict on the same id is a second line, never an overwrite). Findings are judged in the
   sample's order: a verdict after a gap is refused by `report`. `analysant judge report sample.tsv verdicts.tsv --judge <you>
   --pass-percent <P>` prints, per kind, findings, census or sample, judged, true, false, unjudgeable, precision, the rule-of-three upper
   bound when true is 0, the stop reason, how many verdicts lie beyond the stop point (kept, not counted), the next position, whether the 30
   clause is met and whether the kind clears the line, with the named noise classes below. **UNJUDGEABLE counts as false** for the stop and
   the figure. The pass line is a required argument with no default. Humboldt's rows are judged on the local machine and appear in the
   report as counts only (`verdicts.tsv` for Humboldt stays local, as its findings do).
10. **Second judge: the CodeRipper lane** (PM decision 2026-11-20, reversible). It re-judges the 10 of 30 chosen by
    `analysant::gate_b::second_judge_selection(judged, seed)` blind (verdict and reason hidden): `analysant judge sheet sample.tsv
    verdicts.tsv --judge <first judge> --pass-percent <P> --kind <lens/kind>` prints exactly those findings with their positions and no
    verdict. The second judge's lines go in the same table under their own name, and `report ... --second <their name>` gives the agreement
    as `k of n`. It reads Humboldt rows only on the machine that holds the clone. The threshold that voids a figure is D1's and unset: give
    it as `--void-below K` once he sets it; until then the report says `threshold not set` beside the number and voids nothing.
11. **After any tuning** (a new alias, sense or rule written because of what was seen), the sample is spent: choose a fresh seed and
    projects not seen while tuning (section 4), and say in the report which round this is.

What the tool does **not** do: judge anything, choose the plan, or tune anything. A finding's `search` and `control`
columns are what the lens recorded (the untasked check's search record and SelfFind control); contradiction pairs record neither (a pair is
not an absence), so those columns read `-` for them.
