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

— lane: analysant
