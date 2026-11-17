# HANDOFF: analysant (updated 2026-11-17)

You own `C:\Projects\analysant` (github.com/ThinkersJournal/analysant, public). Read `C:\Projects\CLAUDE.md`, then `docs/SPEC-STEP1.md`
(your step-1 design; section 8 = decisions, section 9 = unverified), then `docs/REQUIREMENTS-2026-11-16.md`. Sonnet only; never Opus.
Primary checkout stays on `main`; branches live in worktrees (`C:\Projects\analysant-wt-<topic>`). This HANDOFF is uncommitted in the
primary: copy it into the CI PR's worktree and commit it there.

## State
- Task 1 MERGED (PR #1, cd092cd). Branch `ci-skeleton` (worktree `analysant-wt-ci`, PUSHED, NO PR YET) holds 3 commits: CI + 12 empty crates
  0.0.1 + deny.toml + SPDX gate (excludes reference/); PM rulings D2-D10 in spec section 8; slice 0 results in spec 5.1/9 (116 tests pass, v9 byte-identical,
  2668 lines; the grep 119 is 3 string literals). Local fmt/clippy/test/SPDX pass; GitHub CI never run on it. Portfolio CodeRipper pinned 03d8ac31 (0.47.12).
- PM: today's PR slot used (#1). OPEN the PR from `ci-skeleton` on 2026-11-18, then [READY]. PM sets branch protection after first green.

## Next (unblocked; start without being told; one PR per day)
1. 2026-11-18: open the `ci-skeleton` PR (title: CI, skeleton, D-rulings, slice 0), [READY]. Sign the body `— lane: analysant`.
2. Slice 1 (core + lexical reproducing run v3), on a new branch off `ci-skeleton` or main after merge. Spec section 6. Fixtures point at
   reference/pip-spike-2026-11-16/corpus (D9). Scratch copy of the Fuel tree is NOT kept; refetch if needed (command in the spike HANDOFF).

## Holds / WAITs
None.
