# Gate B report: precision of the untasked and contradiction findings

Status: **2026-11-22 (lane analysant).** Method: SPEC-STEP1 5.2, procedure `docs/GATE-B-PROTOCOL.md`. Pass line (D1, CireSnave 2026-11-21): 80% true over 30 judged per kind; second judge a blind re-judge; 30 lane-day cap (2 used).

Judge: the analysant lane (one reader, one rubric). Second judge: the CodeRipper lane, blind, same rubric and method, so its agreement is not independent evidence of the rubric. Tool: `analysant` on branch `gateb-round1` (runs: baseline at b504ada, round 1 at 45a3720, round 2 at 1b1c60a), `analysant judge report`.
Humboldt is private: it appears as counts only, with no file, line or text. Verdict rows from the private project show their class only (file, line and reason are withheld); every other row is from a public repository.

## Summary

| Run | Config | Kind | Findings | Reading | Judged | True | Stop | Second judge agrees |
|---|---|---|---|---|---|---|---|---|
| Run 0 | none | contradiction/contradiction | 9009 | sample | 7 | 0 | pass line unreachable (7th false positive) | 7 of 7 |
| Run 0 | none | untasked/untasked | 15479 | sample | 7 | 0 | pass line unreachable (7th false positive) | 7 of 7 |
| Run 1 | tuned | contradiction/contradiction | 553 | sample | 7 | 0 | pass line unreachable (7th false positive) | 7 of 7 |
| Run 1 | tuned | untasked/untasked | 3445 | sample | 7 | 0 | pass line unreachable (7th false positive) | 7 of 7 |
| Run 2 | tuned | contradiction/contradiction | 2 | census | 2 | 0 | census complete | 2 of 2 |
| Run 2 | tuned | untasked/untasked | 783 | sample | 7 | 0 | pass line unreachable (7th false positive) | 7 of 7 |

A kind that stopped at the 7th false positive cannot reach 30 judged; under the PM's stopped-kind rule (2026-11-21) the second judge re-judges every judged item and the figure stands at 80% agreement of at least 5 judged. Run 2 contradiction is a census of 2, below the 5-judged floor, so the tool prints that agreement as voided although the two judges agree 2 of 2.

## Reading

- No kind cleared the pass line in any run: 0 of 7 true in every sampled kind (run 0 both kinds, run 1 both kinds, run 2 untasked), and a census of 2 with 0 true (run 2 contradiction). The no-config run is the baseline.
- The rounds changed the producer through off-by-default options (Gate A, the Fuel runs v4 and v5, unchanged at every commit). They shrink the populations by 70 to 99% without raising the true rate on a fresh sample.
- Named noise: bare label or identifier as a side; one generic word carrying a pair; a denied word absent from the positive side; a family match across different words; descriptions of behaviour; standing rules and how-to steps; records of finished work (changelog, archive, checked items). The Gate C Stop condition (SPEC-STEP1 5.3: no kind clears after two tuning rounds on fresh samples) is met on this evidence; the decision is the PM's and CireSnave's.

## Run 0: no-config baseline

- Seed 20261121 (not reused). Projects: KISS 8db36f2; PersistAnt 8717dfd; Humboldt (private; aggregate counts only). Plans: ROADMAP.md, ROADMAP.md, docs/ROADMAP.md. Config: none (no-config baseline).
- contradiction/contradiction: 9009 findings, judged 7, true 0; classes: bare label as the positive side x3; one generic word carrying the pair x2; denied token absent from the positive side x2.
- untasked/untasked: 15479 findings, judged 7, true 0; classes: plan already covers it under other wording x3; bare identifier or label as the claim x3; spec clause or status note, not a work item x1.

| Pos | Kind | Project | File:line | Verdict | Class | Reason | Second judge |
|---|---|---|---|---|---|---|---|
| 1 | contradiction/contradiction | KISS | docs/superpowers/plans/2026-07-19-oracle-vector-corpus-pipeline.md:13 | FALSE | one generic word carrying the pair | Shared tokens are the project name and 'conform'; stdlib-only dependency rule and traceability-matrix derivation are unrelated statements. | FALSE |
| 2 | contradiction/contradiction | KISS | CHANGELOG.md:823 | FALSE | denied token absent from the positive side | Negated side says no spec or wire change; positive side is an op-reference clause that says nothing about spec or wire change. | FALSE |
| 3 | contradiction/contradiction | KISS | spec/classify.md:1558 | FALSE | bare label as the positive side | Positive side is only the cross-reference (KISS-CLASSIFY section 4) and; asserts nothing the escape-denial could contradict. | FALSE |
| 4 | contradiction/contradiction | KISS | docs/superpowers/plans/2026-07-19-oracle-vector-corpus-pipeline.md:13 | FALSE | one generic word carrying the pair | stdlib-only dependency rule vs a freeze-gate test requirement; shared tokens are project name and 'conform' only. | FALSE |
| 5 | contradiction/contradiction | KISS | docs/options-r1-15-scalar-source-leaf-representation.md:7 | FALSE | bare label as the positive side | Positive side is a bare clause ID (KISS-OPS-6.16-0006); the denial that leaves are not ops is not contradicted by an ID. | FALSE |
| 6 | contradiction/contradiction | KISS | docs/options-op-identity-spelling.md:17 | FALSE | bare label as the positive side | Positive side is a bare clause ID; scan not being a KISS-Ops op is unrelated to it. | FALSE |
| 7 | contradiction/contradiction | (private) | withheld | FALSE | denied token absent from the positive side | withheld | FALSE |
| 1 | untasked/untasked | (private) | withheld | FALSE | plan already covers it under other wording | withheld | FALSE |
| 2 | untasked/untasked | (private) | withheld | FALSE | plan already covers it under other wording | withheld | FALSE |
| 3 | untasked/untasked | KISS | docs/conformance-results-2026-10-09.md:18 | FALSE | bare identifier or label as the claim | A crate name in a table cell; states no work. | FALSE |
| 4 | untasked/untasked | KISS | docs/decision-263-op-family-mapping.md:22 | FALSE | plan already covers it under other wording | Decision record phrase; issue 263 is in the roadmap (R0.2, close the issues the merged work answered). | FALSE |
| 5 | untasked/untasked | (private) | withheld | FALSE | bare identifier or label as the claim | withheld | FALSE |
| 6 | untasked/untasked | KISS | spec/announce.md:396 | FALSE | spec clause or status note, not a work item | A normative spec clause about announce tags; a roadmap does not list each clause, no work stated. | FALSE |
| 7 | untasked/untasked | KISS | docs/superpowers/specs/2026-07-21-planb-slice0-workflow-output.md:722 | FALSE | bare identifier or label as the claim | A code identifier (eval_add) in a code span; states no work. | FALSE |

## Run 1: tuned round 1

- Seed 20261122 (not reused). Projects: synapse d092421; vulkane 2523f36; configurant 583b30a. Plans: ROADMAP.md each. Config: `[untasked] skip_names`; `[contradiction] skip_names, require_denied_token, generic_df_percent = 20` (no-config populations on the same projects: contradiction 1892, untasked 11686).
- contradiction/contradiction: 553 findings, judged 7, true 0; classes: one generic word carrying the pair x4; denied token absent from the positive side x1; family match across different words x1; bare label as the positive side x1.
- untasked/untasked: 3445 findings, judged 7, true 0; classes: description of behaviour, not a work item x4; record of finished work (changelog or archive) x3.

| Pos | Kind | Project | File:line | Verdict | Class | Reason | Second judge |
|---|---|---|---|---|---|---|---|
| 1 | contradiction/contradiction | synapse | docs/CUTOVER_READINESS_2026-10-15.md:27 | FALSE | denied token absent from the positive side | Not needed for cutover is a scope note; the changelog line is a docs entry that merely mentions cutover readiness. | FALSE |
| 2 | contradiction/contradiction | synapse | docs/superpowers/specs/2026-11-09-state-redb-via-persistant.md:111 | FALSE | family match across different words | persistant (the library name) paired with persistent keystore via the family-prefix rule; the negated side is a bare parenthetical fragment. | FALSE |
| 3 | contradiction/contradiction | synapse | docs/ACCESS_CONTROL_DESIGN.md:83 | FALSE | one generic word carrying the pair | Shared words are the project name auth-framework; an offers-today section title and a version-wording note state nothing in conflict. | FALSE |
| 4 | contradiction/contradiction | synapse | docs/PILOT_T0_RUNBOOK.md:14 | FALSE | bare label as the positive side | Positive side is the heading MCP adapters; it asserts nothing the adapter-naming correction contradicts. | FALSE |
| 5 | contradiction/contradiction | synapse | docs/superpowers/specs/2026-10-12-receiver-only-ack.md:37 | FALSE | one generic word carrying the pair | Shared word is changed: a spec section What is NOT changed vs a changelog feature line about change notices. | FALSE |
| 6 | contradiction/contradiction | synapse | CAPABILITY_INVENTORY.md:1453 | FALSE | one generic word carrying the pair | Shared stem compile: an example that does not compile vs a plan step about updating compiled construction sites. | FALSE |
| 7 | contradiction/contradiction | synapse | CAPABILITY_INVENTORY.md:1453 | FALSE | one generic word carrying the pair | Negated side is about an example file not compiling (the NAT test); positive side says the QUIC transport compiles; different subjects sharing the word compile. | FALSE |
| 1 | untasked/untasked | synapse | docs/superpowers/specs/2026-09-22-quic-transport-design.md:72 | FALSE | description of behaviour, not a work item | A design spec sentence stating validate_config refuses bad values; states how the code behaves, no task. | FALSE |
| 2 | untasked/untasked | synapse | docs/superpowers/specs/2026-09-30-m0-workspace-split-design.md:85 | FALSE | description of behaviour, not a work item | A design detail about what a job prints; no work stated. | FALSE |
| 3 | untasked/untasked | configurant | CLAUDE.md:6 | FALSE | description of behaviour, not a work item | A project one-line description in CLAUDE.md; no work. | FALSE |
| 4 | untasked/untasked | configurant | CHANGELOG.md:347 | FALSE | record of finished work (changelog or archive) | A CHANGELOG entry for a CI repin already done. | FALSE |
| 5 | untasked/untasked | vulkane | CHANGELOG.md:1936 | FALSE | record of finished work (changelog or archive) | A CHANGELOG API note for a shipped function. | FALSE |
| 6 | untasked/untasked | configurant | docs/TRANSFER.md:68 | FALSE | description of behaviour, not a work item | States what the transfer verify test shows; documented behaviour, no task. | FALSE |
| 7 | untasked/untasked | synapse | docs/archive/ENTERPRISE_IMPLEMENTATION_COMPLETE.md:22 | FALSE | record of finished work (changelog or archive) | An archived completion report line marking enterprise profiles complete. | FALSE |

## Run 2: tuned round 2

- Seed 20261123 (not reused). Projects: httpant fa54a4d; antspawn 5af5223; webhostant 54d8019. Plans: ROADMAP.md each. Config: round 1 + `[untasked] only_kinds = [directive, proposal, decision]`; `[contradiction] min_shared_tokens = 2` (no-config populations on the same projects: contradiction 252, untasked 2939).
- contradiction/contradiction: 2 findings, judged 2, true 0; classes: bare label as the positive side x1; scoped negation x1.
- untasked/untasked: 783 findings, judged 7, true 0; classes: standing rule or convention, not a work item x2; record of finished work (changelog or archive) x2; how-to instruction in a guide, not project work x2; description of behaviour, not a work item x1.

| Pos | Kind | Project | File:line | Verdict | Class | Reason | Second judge |
|---|---|---|---|---|---|---|---|
| 1 | contradiction/contradiction | antspawn | CHANGELOG.md:251 | FALSE | bare label as the positive side | Positive side is the section heading Sessions (slice S1; plan in docs/SESSION-PLAN.md); the negated side is a launcher default. Nothing conflicts. | FALSE |
| 2 | contradiction/contradiction | antspawn | docs/ACCOUNT-DISPATCH.md:62 | FALSE | scoped negation | Negated side is a clean-slate requirement (run N+1 does not find run N's sentinel); positive is a changelog line saying negative tests exist for clean slate. Same topic, no conflict. | FALSE |
| 1 | untasked/untasked | httpant | .github/copilot-instructions.md:74 | FALSE | standing rule or convention, not a work item | Contributor style rule (tests co-located with code), not a task. | FALSE |
| 2 | untasked/untasked | webhostant | docs/CONSUMER-CONTRACT.md:66 | FALSE | standing rule or convention, not a work item | A standing security rule (no secret value on a command line or in logs); a roadmap does not task a rule. | FALSE |
| 3 | untasked/untasked | httpant | CHANGELOG.md:325 | FALSE | record of finished work (changelog or archive) | A CHANGELOG entry describing a shipped conformance case. | FALSE |
| 4 | untasked/untasked | httpant | .github/copilot-instructions.md:94 | FALSE | record of finished work (changelog or archive) | A checked-off item, Middleware system complete. | FALSE |
| 5 | untasked/untasked | httpant | CONTRIBUTING.md:57 | FALSE | how-to instruction in a guide, not project work | A CONTRIBUTING step for adding an adapter; a how-to, not project work. | FALSE |
| 6 | untasked/untasked | httpant | CONTRIBUTING.md:34 | FALSE | how-to instruction in a guide, not project work | A CONTRIBUTING step, run clippy; a how-to. | FALSE |
| 7 | untasked/untasked | antspawn | docs/DESIGN.md:6 | FALSE | description of behaviour, not a work item | A design line defining Needs; describes the design, no task. | FALSE |
