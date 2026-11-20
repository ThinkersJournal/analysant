# Gate B protocol: the precision gate

Status: **prep, 2026-11-20 (lane analysant).** The method is SPEC-STEP1 section 5.2; this file is the part a judge needs open while judging.
The pass line is CireSnave's (D1) and is **not** set here; the strawman is 80% true on 30 judged per finding kind.
The code is `analysant::gate_b` (`crates/analysant/src/gate_b.rs`); nothing in it judges a finding.

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
