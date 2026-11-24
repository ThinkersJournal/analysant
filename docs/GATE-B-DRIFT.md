# Gate B board 499: the drift check, fresh sample, and the B plan

Lane analysant, 2026-11-24. Authority: CireSnave, board 499, verbatim *"499: D but with B instead of retiring it if it fails."* (via the PM TASK). Tool: analysant at d7b56a9 (branch `drift-check`), `[untasked] absent_artifact = true` only. No model used, nothing spent.

## What changed (red-first tests, `round4.rs`)
The check is re-scoped as plan-versus-tree drift: rows carry lens/kind `drift` and the ask "tick it (done), delete it (obsolete) or do it (open)", and it no longer needs a plan file. Census 497A false classes fixed, one test each: boxes in template/checklist files, optional steps, TDD red steps ("watch fail", "expected: FAIL"), paths outside the repository (`C:/`, `~`, `$`, `/tmp`), and a qualified symbol (`Tensor::scan`) searched by its final segment as a whole word. fmt, clippy `-D warnings` and `cargo test --workspace` green; Gate A unchanged (option off by default).

## Census (same method as 497A: `git archive origin/main`, then `git ls-files`)
- Repos in `tools/portfolio-repos.json` 68; scanned 61; UNMAPPED 7 (same seven as 497A).
- Findings **121** in 10 of the 61 repos: fuel 69, synapse 17, lightbulb 12, HumboldtUnified 6, coderipper 6, mlmf 5, KISS 2, auth-framework 2, OverMind 1, auto-discovery 1 (497A, before the fixes: 133 in 10; fuel 77, now 69).
- Positive control, same binary: a planted tree with one step naming a missing file, one naming an existing file, one optional step naming a missing file: exactly 1 finding (the first).

## Judging (blind; fresh sample, not the 497A rows)
- Seed **20261201** (unused before). Declared before judging: the first 30 of the seeded order of the 121, at most 6 per repo (9 repos in the 30: synapse 6, lightbulb 6, fuel 6, HumboldtUnified 5, coderipper 2, KISS 2, mlmf 1, OverMind 1, auto-discovery 1).
- TRUE = the step is open, or the document is stale against the tree (tick it, delete it or do it). Sheet and trees only for the second judge (a Sonnet subagent, directory with no verdicts); mine were written in a separate directory.

| judge | TRUE | FALSE | rate |
|---|---|---|---|
| lane (analysant) | 12 | 18 | **40%** |
| blind second (Sonnet) | 16 | 14 | **53%** |

Agreement on 22 of 30 (73%); the 8 disagreements (rows 4, 5, 8, 14, 15, 16, 19, 26): the second judge counted a done-but-unticked step as stale (TRUE) where I counted it as nothing to say; on 15 and 26 it saw a ROADMAP annotation that the target is superseded. Under the most generous reading, every disagreement going the second judge's way for the TRUE side, the union is 18 of 30 = 60%.

**Verdict: does not reach the 80% bar (40% / 53%, population 121, sample 30).** Both judges agree it fails; no reading of the disagreements reaches 80%. Per the ruling the check is not retired; the B plan follows. Fuel's list is not sent (a pass condition).

False classes still in the sample (mine): commit/push/report/"READY" ceremony steps (8 of 30: the flagged "artifact" is a branch name or `seal/open`), spurious path-like tokens in prose (`ms/token`, `alignment/bounds`, `mit/apache-2.0`, an existing directory with a trailing `/`; 8), a done step whose named artifacts exist (2). True classes: spec/roadmap items whose type or file is absent from the tree (5), steps naming a file renamed or removed (4: `0019_display_tokens.sql`, `router.rs`, `mlp_wrapper.rs`, `parallel_cache_builder.rs`), one open decision. Several fixes are cheap and deterministic (ceremony verbs, `a/b` prose tokens). **They are not applied: this sample is spent for tuning** (fixing them and re-measuring on it would be a one-armed pre/post). The remaining false rate is mostly "does the step still need doing", which is the model question.

## B plan (nothing is spent until the PM confirms the estimate)

**What the model judges.** For each candidate step (an unticked box in a live document that names an artifact), one call with: the step text and its two preceding lines, the document title and status lines, and deterministic evidence the tool already computes (which named artifacts are absent, and `git grep` hits for the step's content words, capped). Output: `OPEN | DONE | OBSOLETE | NOT_A_STEP` plus one sentence. Rendered finding: OPEN -> "do it", DONE -> "tick it", OBSOLETE -> "delete it"; NOT_A_STEP (commit/push/report ceremony, optional) is dropped. Candidate set = today's drift rows plus unticked boxes in live documents naming no artifact only if the PM wants recall measured (default: no).

**Provider.** A non-Claude model (CireSnave's cost rule); the PM names the provider, any OpenAI-compatible endpoint works. No provider is configured in this lane, so none is assumed. The call is behind a trait in `analysant-core`'s seam (no hard-coded provider), off by default, so Gate A and the deterministic path are unchanged.

**Cost estimate (before any spend).** Candidate steps: 121 now; the upper bound for the full set of unticked boxes in the 61 repos is 2,970 (497A raw count, includes closed documents). Per call about 1,500 input tokens and 100 output tokens. Rows at 121: ~0.2M input + 12k output tokens. Rows at 2,970: ~4.5M input + 0.3M output. At an assumed price of $0.30 per million input and $1.20 per million output (a mid-priced non-Claude model; the PM's provider sets the real figure) that is **about $0.07 for 121 rows and about $1.7 for 2,970**, plus a re-run for a second configuration at most doubling it. Claude cost: the judging of the fresh sample below by this lane and one blind subagent, roughly 60 rows each at the sample-30 cost (~70k tokens per judge pass of 30).

**Judging plan.** Fresh seed 20261202, **60 rows** of the model's OPEN/DONE/OBSOLETE output (double this round's 30), at most 10 per repo, declared before judging; the same TRUE definition and the 80% bar over at least 5 judged; blind second judge (sheet and trees only, no verdicts, no model verdicts shown); census population and the planted positive control beside every number. The model's own NOT_A_STEP drops are sampled too (20 rows) so a model that drops everything cannot pass by emptiness. Recall is still not measured and the report says so. One round, then report; failing it is a CireSnave question, not another filter round.

**Needs from the PM:** the provider and price, and confirmation of the estimate above.

## B run on a local model (2026-11-24, PM decision: local only, no hosted call, no secret read)

Model `overmind-qwen35-9b-ctx16k` (Qwen3.5 9B Q4_K_M) on this laptop's Ollama; the RTX 4070 (8 GB) was idle at start (0 MiB used), the model took about 6.6 GB, calls ran one at a time at about 4-5 s each (about 14 minutes of GPU in all: 30 + 30 + 30 + 121 rows, plus 3 control rows). Nothing left the machine. The driver is a scratch script (not in the repository); it feeds the step, the document start and deterministic evidence (does each named file or identifier exist in the tree) and asks for OPEN / DONE / OBSOLETE / NOT_A_STEP as JSON at temperature 0. TRUE for the model = OPEN, DONE ("tick it") or OBSOLETE ("delete it").

**Validation on the 30 rows already judged (both judges are Claude family).** Planted control: 3 of 3 correct (missing file: OPEN; existing file: DONE; optional step: NOT_A_STEP).

| prompt / model | rows kept | lane-true among kept | blind-true among kept | agreement with lane | agreement with blind |
|---|---|---|---|---|---|
| v1, qwen3.5 | 24 | 12 | 16 | 18/30 | 22/30 |
| v1, qwen3.8 | 23 | 11 | 15 | 17/30 | 21/30 |
| v2 (sharper rules), qwen3.5 | 17 | 11 | 13 | 23/30 | 23/30 |

v2 was written after reading v1's errors on these 30, so **those 30 rows are spent and v2's validation numbers are not a clean test**; the fresh sample below is. v2 and qwen3.5 were used for everything after.

**Run over the 121 findings (census population 121, 61 repos read):** OPEN 61, DONE 7, OBSOLETE 7, NOT_A_STEP 46; kept 75.

**Fresh blind sample (seed 20261202).** Pool: the 75 kept rows minus the 17 already judged = 58 (all taken; fewer than 60, so no per-repo cap was applied: fuel 40, synapse 7, lightbulb 5, coderipper 2, auth-framework 2, mlmf 1, HumboldtUnified 1) plus 20 of the model's 46 NOT_A_STEP drops (fresh), shuffled together so the judges cannot tell which is which. Two independent blind Sonnet judges (sheet and trees only, separate directories, no model output shown). The judges, not this lane, judged these 78 rows; the lane audited 12 rows afterwards.

| | judge A | judge B |
|---|---|---|
| TRUE among the 58 kept | **47 (81%)** | **46 (79%)** |
| fuel (40) / not fuel (18) | 30 / 17 | 30 / 16 |
| TRUE among the 20 dropped | 3 | 3 |

A and B agree on 75 of 78 rows; both TRUE on 45 of the 58 kept. **Against the 80% bar this is on the line: one judge clears it by one row, the other misses by one, so it is not a clean pass.** Read it with these cautions, which I am reporting rather than resolving:
- Both judges are Claude and lenient: B said it checked the trees for about half its rows, and both gave one of two stock reasons for every row.
- Lane audit of 12 randomly chosen both-TRUE rows against the tree: 5 clearly genuine (CONTRIBUTING.md absent; a named function, test or contract file already present, so tick it), 7 weak (a TDD "write the failing test" step that names no artifact, or a path-with-line-range token that is spurious). By the letter of the definition (an unticked step in a finished plan is stale), the weak ones are TRUE; by a stricter reading (would the owner want to be told) they are not. That reading gap is the whole difference between 80% and about 50% here.
- 40 of the 58 are fuel (69%); outside fuel the rate is 17/18 and 16/18 on small numbers.
- The model dropped 3 true rows of 20 (15%): a recall cost, not measured over the population.

**What the PM needs to decide:** whether TRUE includes "unticked box in a finished plan" (the definition in the board 499 brief says yes, and then even the deterministic check is near-trivially true on old plans), and whether Claude judges are acceptable for a cost-rule-bound check. The local model is sane (control right, drops are 85% correct) and cost nothing; with the brief's definition its output sits at 79-81%.

— lane: analysant
