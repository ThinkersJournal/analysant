# Gate B census 497A: the narrow `untasked` check over every repository

Lane analysant, 2026-11-24. Authority: CireSnave, board 497, verbatim *"497: A/"* (read as A), via the PM TASK. No model. Tool: analysant at ebed416 (`[untasked] absent_artifact = true`, nothing else on).

## Method
- Repos from `tools/portfolio-repos.json` (68). Each read from `git archive origin/main` after `git fetch` (never a working tree), exported to scratch, re-initialised, checked. A repo with no checkout is UNMAPPED, not guessed.
- Lens run with a one-line placeholder plan (the lens needs a plan to run; `absent_artifact` does not use it), so every markdown doc is corpus. The contradiction lens also ran (default); its output is ignored here.
- Positive control, same run: a planted tree with a live plan (two unticked steps naming a missing file and a missing identifier) and a superseded plan with one such step. Result: 2 findings, both in the live plan, 0 from the superseded one.

## Population
- Repos in the list 68; scanned 61 (origin/main at each SHA in the scratch summary); UNMAPPED 7: community, ThinkersFoundation-site, authframework-python, authframework-js, system-analysis, hardware-query, pypilib.
- Markdown documents scanned 2462 (tool corpus count; 2464 by a separate `git ls-files *.md` count including the planted-free trees, difference is the 2 placeholder/plan files).
- Steps scanned: 2970 unticked boxes by an independent regex over all markdown (an upper bound for live documents: it includes closed ones), in 16 of the 61 repos. The tool's own "claims" count (236186) is every directive claim, not steps, and is not a step count.
- Repos where it fired 10 of 61. Findings 133: fuel 77, synapse 16, lightbulb 14, HumboldtUnified 6, coderipper 6, mlmf 6, KISS 3, auth-framework 3, OverMind 1, auto-discovery 1.

## Judging (blind, seed 20261127, first 10 of the seeded order of 133)
My verdicts and a blind second judge (a Sonnet subagent given only the sheet and anonymised repo trees, in a directory with no verdicts) agree on all 10: **3 true, 7 false = 30%.** True: deferred follow-up naming a missing test; CONTRIBUTING.md missing from a pre-publish checklist; VirtualMemoryPool missing from a spec's acceptance list. False classes: done-but-unticked in a finished plan (4: shape oracle verification, migration renumbered, workspace-split step obsolete, scan TDD step), PR-template checklist (1), optional step (1), untracked scratch path (1). One false positive is the checker's own: a qualified Rust path (`Tensor::scan`) never appears literally in source, so it reads absent.

## Verdict
**Does not pass the 80% line (30% of 10 judged, both judges agree); it fires in 10 of 61 repos, and 77 of the 133 findings are in one repo (fuel).** Not "too narrow": it fires, and its output is mostly finished plans nobody ticked. Removing the false classes needs a judgement of whether a step is still open, which is the model option (board item B) for CireSnave. Recall is not measured. Not tuned: this sample is spent for any later tuning.

— lane: analysant
