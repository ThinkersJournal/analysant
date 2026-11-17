# HANDOFF: analysant (soft-slept lane created 2026-11-17)

You own `C:\Projects\analysant` (github.com/ThinkersJournal/analysant, public). Read `C:\Projects\CLAUDE.md` first, then
`docs/REQUIREMENTS-2026-11-16.md` (the whole design, CireSnave's words and Pip's notes), then `reference/pip-spike-2026-11-16/`
(HANDOFF.md and spike-report-2026-11-16.md there). Sonnet only; never Opus.

## What this project is
AnalysAnt: generic text-analysis libraries: one shared claim model, a language-pack seam (a language is data; English and Rust from the
start), a library per analysis type. Lexical and semantic are the base; pragmatic, epistemic, temporal, logical, deontic, discourse,
ontological and structural are built on them. Crates are named `analysant-<type>` with `analysant-core` for what the libraries share
(all 12 names are reserved at 0.0.0 on crates.io). Later layers (NOT yours yet): MCP tools that give an LLM a project-wide documentation
health report, then controlled editing tools. Sequencing is CireSnave's: analysis first, tool seam sketched early, rewriting LAST.
Deterministic first (no ML in the base); ML only as an escalation for the fuzzy tail; it must run slowly on limited resources.

## CireSnave's decisions so far (2026-11-17)
- Name AnalysAnt; type suffixes plus `-core`; "Go ahead" to start. Not yet decided: a pass line for precision, hosting of MCP tools.
- The spike is NOT the base (Pip wrote only what Fuel needed). It is the oracle: a generic base plus the pragmatic lens must reproduce its
  Fuel results with no Fuel-specific code.

## Your FIRST task (docs only; no crate code yet): write `docs/SPEC-STEP1.md` and open ONE PR with it
1. The **claim model**: identity (stable, survives edits), text, kind (directive, assertion, question, decision, definition...), the
   section it sits in, relations, provenance, epistemic status; what "supersedes" means; how a claim is addressed.
2. The **language-pack seam**: what a language supplies as data (tokenizer, sentence rules, lexicons, normalizer, for Rust a parser giving
   signatures, tests and types as claims); what the engine owns; how English and Rust both fit; what is NOT allowed to be hard-coded.
3. The **two base libraries**, `analysant-lexical` and `analysant-semantic`: public API sketch (types and function signatures, no
   bodies), what is deterministic and what is the ML escalation point, how an alias table and a sense table are stored and kept from rotting
   (the maintenance story).
4. The **tool seam** (sketch): how a lens is exposed to an LLM as a tool; every finding carries what was searched and a positive control
   (an absence claim must say what it looked in).
5. **Acceptance**: how the Fuel spike is used as the golden oracle; a precision gate (a shuffled seeded sample of findings judged against
   a rubric across at least three projects, as CodeRipper does); a kill/continue criterion for CireSnave to set.
6. A **build order** with the smallest first slice, and an explicit list of what you did NOT decide (go to the PM as [ASK]).
Read the spike's code and report before writing; say what you reuse (ideas, claim.rs shape, test data) and what you do not (Fuel-shaped
lexicons). Cite file and line for every claim about the spike; a claim you did not check is marked UNVERIFIED.

## Repo hygiene you must do in the same PR or the next (PM sets branch protection after the first CI run)
CI (build, test, clippy -D warnings, fmt, cargo deny), the portfolio CodeRipper workflow, `changelog.d/` fragments (one per PR), SPDX
header on every source file, CHANGELOG, workspace Cargo.toml with the 12 crates as empty members only when code exists. One PR per day.

## Holds / WAITs
none.
