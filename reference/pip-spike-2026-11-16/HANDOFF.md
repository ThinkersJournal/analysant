# Handoff — Documentation-analysis spike (2026-11-16)

**To:** the PM, for a lane. **From:** Pip, via Eric. **Date:** 2026-11-16.
**Status:** as-is, after the last planned slice (slice 6). Eric chose this handoff over further slices.

## What this is

A spike for the documentation tooling Eric specified in
`llm-documentation-tooling-requirements-2026-11-16.md`: tooling so LLMs can
change and analyze documentation with confidence (insert without overwriting,
replace a subsection without risk to the rest, surface discrepancies
automatically, side-by-side verify, view a document as a sequence of claims).
Built **outside the Claude agents' microcosms**, per Eric's call — the agents
are what failed and who the tool is for, so the auditor does not live inside
the audited system.

It is a Rust crate (std-only, zero dependencies, no ML) that extracts claims
from architecture documents and checks them against a plan (the ROADMAP) and,
since slice 6, against the code. Six lenses, one per slice:

| slice | lens | result |
|---|---|---|
| 1 | pragmatic classification (directive / description / status note) | narrow-corpus absent 212 → 105 |
| 2 | capability-name extraction (10 deterministic rules) | absent 105 → 73; Judge baseline is the known loss |
| 3 | wider corpus (25 files) | absent 73 → 1,164 — the decisions log drowns the list |
| 4 | genre handling (outcome filter + note-to-row ownership) | absent 1,164 → 688; 449 findings owned by a ROADMAP row |
| 5 | contradiction lens (doc vs doc) | 37 pairs; 1 real (cancellation), 36 noise in 6 named classes |
| 6 | Rust language pack (code as ground truth) | of 764 absent findings: 236 BUILT, 34 STUB, 22 TEST-ONLY, 472 ABSENT |

Full detail, per-slice write-ups, and the handoff assessment are in
`spike-report-2026-11-16.md`. The triage tables are in `runs/` (v1–v9).

## How to reproduce

```sh
cargo test && cargo clippy --all-targets && cargo fmt --check   # 116 tests green
# v9 (the current run) — needs the Fuel code tree at the pinned commit:
#   fetch https://codeload.github.com/CireSnave/Fuel/tar.gz/b0d74efafed63d9700ec62f652468b197dd46a88
#   into corpus/fuel/code/ (expect 51M, 951 .rs files, no target/ dir)
cargo run --release -q -- corpus/fuel/arch-wide corpus/fuel/ROADMAP.md aliases.txt corpus/fuel/code
```

Until the code tree is fetched, `cargo test` shows 12 failures — all the
`real_*` tests in `tests/rust_code.rs`, which pin the slice-6 validation rows
against the real Fuel tree (they panic with "corpus/fuel/code must exist —
run the fetch first"). The other 104 tests pass without the tree. Once the
tree is in place, all 116 pass.

`aliases.txt` is load-bearing: omitting it flips one named claim
(04:3, "multi-path graph") from absent to mentioned. The decisions log's
sha256 is `9a2e703fe86ab931d1974b1e482d4759871a2c80b0c2e614c25f2d9a63ebf092`.

## What is proven, per lens

- Descriptions and status notes wearing a directive's or capability's shape
  can be set aside deterministically (slices 1–2).
- Capability names extract by 10 deterministic rules (slice 2).
- A decisions log without genre handling drowns the finding list; outcome
  filtering plus note-to-row ownership restores it (slices 3–4).
- Two documents disagreeing is findable deterministically (slice 5) — but
  36 of 37 pairs are noise, and the real one is the cancellation case the
  audit already knew.
- Code as ground truth works for existence and provenance (slice 6): 236 of
  764 absent findings are built in code, 34 are stubs, 22 test-only.

## What is still weak — the work list (limits carried as tasks)

1. **Settle the verdict vocabulary first.** The lens's BUILT means
   production-reachable; the audit's STUB/ABSENT means not on the realize
   path. Three of seven validation rows disagree in exactly this way (dead
   production code, dead chains, wrong-path reachability). Build the
   **call-graph lens** from the realize entry points to close them.
2. **Scope the contradiction lens** so the denied capability must be the
   thing the positive side asserts — or build the sense table for
   "budget"/"harvest". Do not add more lexicon.
3. **Bound the word rule's over-eager matches.** Generic words and
   single-word keys fire on unrelated items (04:67 `Concurrent` → BUILT via
   generic `Rule::matches`; 15:211 → BUILT via a `decode` fn in a whisper
   example). The 236 BUILT count is an upper bound, not a verified one.
4. **Replace or bound the hand-rolled scanner** (a real parser, or explicit
   acceptance of its gaps): macro-generated items invisible (257
   `macro_rules!`, 1,888 `#[derive]` lines); cfg-gated items counted as
   present regardless of feature (1,389 `#[cfg]` lines); re-exports not
   resolved (268 `pub use` lines); bare-name conflation (`pub fn new` ×177).
5. **No sense disambiguation anywhere.** A name match is not a behavior
   match (`harvest` in code is `harvest_grads`, unrelated to pattern
   harvesting).
6. **No integration check.** BUILT is existence, not wiring. The lens never
   checks whether code is correct, tested, or wired in.
7. **Revisit the owned-by-row citation case** flagged in slice 4: the nvidia
   licence note is "owned" by ROADMAP row 141 only because the row cites it
   as a working paper.
8. **Build the rewriting tools.** Every slice so far is analysis only.
   The requirements note's insert / replace-a-subsection / side-by-side
   verify tools are unbuilt.

## Provenance rules (do not break these)

- The planned-vs-built audit (`corpus/fuel/design-notes/`) is ground truth
  **for validation only, never an input** — the spike never reads it.
- The corpus is Fuel at pinned commit `b0d74efa` (2026-11-16T09:38:42Z).
- The v8 doc-vs-roadmap numbers (764 absent / 2,115 mentioned / 518 set
  aside / 449 owned) and the 37 contradictions are identical in v9; the
  Rust lens is additive, proved by diffing the run files.

## Package contents

- `HANDOFF.md` (this note), `spike-report-2026-11-16.md`, `Cargo.toml`,
  `Cargo.lock`, `aliases.txt`
- `src/` (11 modules), `tests/` (6 suites, 116 tests)
- `runs/` — run outputs v1–v9 (triage tables; v9 is current)
- `corpus/fuel/ROADMAP.md`, `corpus/fuel/arch-wide/` (25 files),
  `corpus/fuel/arch-run/` (narrow 3-file corpus),
  `corpus/fuel/architecture/` (original fetch), `corpus/fuel/design-notes/`
  (the audit, validation only)

**Not included:** `corpus/fuel/code/` (the 51M Fuel source tree — re-fetch at
the pinned commit per the command above) and `target/` (build artifacts).
