# Fixtures

`v3-keys.tsv`: `file<TAB>line<TAB>key` for each of the 558 claims the reference spike extracts from the
narrow corpus (`reference/pip-spike-2026-11-16/corpus/fuel/arch-run`). Generated 2026-11-19 by running
the spike's own `parse::extract_claims` (unmodified, from a scratch copy of the `reference/` crate, plus a
10-line example that prints `claim.file`, `claim.line`, `claim.key`). The v3 run file lists absent
findings without keys, so this is what makes the `(file, line, key)` comparison of SPEC-STEP1 5.1 real.

`denylist-positive.txt`: a deliberate violation of the engine's no-hard-coded-vocabulary rule, the
positive control of the deny-list test (it must be caught, or the test proves nothing).
