# Gate B round 5: design note (written before any run)

Status: 2026-11-23, lane analysant. Authority: CireSnave, board 496, verbatim *"496: C"* (via the PM's TASK): keep only the deterministic part of the untasked check, retire the contradiction check. The PM decided the rubric as the check's design: an open item in a superseded or completed document is NOT a finding.

## The check

`[untasked] absent_artifact = true` (commit d8515f6). A finding is an unticked box in a live document (no superseded/obsolete/deprecated/archived/complete/done marker in a heading, quote or status line in the first 20 lines; not a changelog/guide/README-type non-work document) whose step names an artifact ABSENT from the tree at the checked-out revision: a file path (neither the path nor its file name is tracked), or a backticked identifier (no tracked non-markdown file contains it). Commands are not checked in this version (stated, not hidden). Everything else is not a finding. Tests written first (red, then green): existing file does not fire; missing file in a live plan fires; the same step in a superseded document does not fire.

The contradiction lens is not run, not judged, and makes no claim.

## Fixed before any run

- Seed **20261126** (never used). Pass line: 80 percent true of at least 5 judged (`--void-below 8 --stopped-agree-percent 80 --stopped-min-judged 5`).
- Projects (public, none seen in runs 0 to 4; not coderipper, which is the second judge's own project), clean clones, no `.analysant/` directory, at: **json-mcp-server** 2a866e3e9efa1d2eb1ed99c6542e51945fec8e4b; **commy** bd5e51b0e8b78e003e3bd8f09e44a6de48bafeb2; **agentlife** dce80aa76216eb5b0db7f4b537c95d309920716d (all ciresnave/<name> at origin default branch, 2026-11-23). Plan: ROADMAP.md each (all three have one).
- Config: `[untasked] absent_artifact = true` only; every other option off.
- Population: the whole kind is read if under 30; otherwise 7 per the seeded order, capped at 3 rows per project as in run 4. The report says plainly how many findings there were and that recall is not measured.
- Second judge: coderipper-c, BLIND: the sheet and the clones only, in a directory holding no verdicts file; I will not send verdicts first.
- If this fails, the next step is the model option (board item B), not another filter round.
