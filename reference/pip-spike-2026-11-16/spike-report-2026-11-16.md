# Documentation Tooling — Spike Report

Date: 2026-11-16. Builder: Pip, outside the Claude agents' microcosms (Eric's call: the agents are what failed and who the tool is for, so the auditor does not live inside the audited system). Crate: `~/workspace/doc-analysis-spike` (Rust, std-only, no ML, no external crates).

## What the spike is

The first slice of the design in `llm-documentation-tooling-requirements-2026-11-16.md`: one lens (cross-document discrepancy, directed-but-untasked) over a claim model, run against real docs. Scope, deliberately narrow:

- **Claim model** (`src/claim.rs`): a claim has stable identity (file:line), text, a normalized key, the section it sits under, and a kind (Directive / Named / Identifier). The model is language-agnostic; language work lives in the parsers.
- **Extraction** (`src/parse.rs`): list items → directives (key = the item's head: a leading bold span of 2+ words, else the first 8 content words); bold phrases → named capabilities (only if they look like a name: short, no sentence punctuation, not mostly uppercase); backtick spans → code identifiers (reported separately — code facts, not directives); table first-cells → named components. Fenced code blocks are skipped (code is another language). Named claims deduped by key per file.
- **Matching** (`src/normalize.rs`, `src/check.rs`): lowercase, strip punctuation, drop a leading article and stopwords; a claim is "mentioned" when every word of its key appears in one ROADMAP row (token-subset, any order — word order is not part of a capability's identity). Alias groups are data (`aliases.txt`, one group per line, `|`-separated, first phrase canonical); both sides are canonicalized before matching. Every finding is explainable: these words appear in that row, or in none of them.
- **Tests**: 11 tests, written against the spec (claim model, normalization, extraction, the check, and a miniature of the DAG failure). `cargo test` green, `cargo clippy` clean, `cargo fmt` clean.

## Corpus

Fuel at `origin/main` `b0d74efa` (2026-11-16T09:38:42Z), fetched 2026-11-16:

- Architecture side: `docs/architecture/02-layers.md` (47,894 B), `03-ir.md` (41,542 B), `06-runtime.md` (34,833 B). Excluded by scope: `10-decisions-log.md` (241,115 B, a log not directives) and the design notes.
- Plan side: `ROADMAP.md` (556,262 B, 3,351 lines).
- Ground truth for validation (not an input — the spike never reads it): `docs/design-notes/2026-11-15-planned-vs-built-audit.md`, the PM's planned-vs-built audit from yesterday, with explicit BUILT / PARTIAL / STUB / ABSENT statuses per component.

## Results

Three runs, one iteration each (v1: naive extraction + contiguous matching; v2: bold filters + token-subset + alias table + kind split; v3: directive head-keys):

| run | claims | absent (directives / named / identifiers) | mentioned |
| --- | --- | --- | --- |
| v1 | 624 | 366 | 258 |
| v2 | 558 | 241 (104 / 103 / 34) | 317 |
| v3 | 558 | 212 (75 / 103 / 34) | 346 |

Full v3 output: `runs/fuel-2026-11-16-v3.txt` in the crate. Runtime for the full run: ~2.2 s, release build. No ML, no network at run time, no GPU — the limited-resources constraint is satisfied trivially.

### What it got right

- **The DAG concept is correctly NOT flagged.** "Multi-path DAG" and "multi-path graph" do not appear in the absent list: the alias group bridges the two names, and ROADMAP row 157 ("Multi-node placement pathfinder: build the multi-path DAG the architecture describes", GAP-385, opened 2026-11-15) matches. The remedy landed in the ROADMAP, and the spike sees it. Had the spike run on Sunday's ROADMAP, row 157 would not have existed and the DAG would have been the headline finding — the miniature test (`dag_in_miniature_absent_when_roadmap_has_no_row`) proves that exact case.
- **It surfaces real audit-ABSENT items.** In the v3 absent list: tolerance budget, pattern harvest, PlanStore, persisted graph, lookahead, cancellation, Judge baseline. These match the audit's ABSENT/STUB rows — capabilities the architecture directs and nothing has built.
- **Every finding is checkable.** Each names its key, source file:line, and section. The audit's own words apply to the current state of the art, from `06-runtime.md` v1.6: "a claim repeated across N documents is protected in none of them" — the doc-vs-code guards read only rust fences or work only within a single file. The spike is the cross-document guard those guards are not.

### What it did NOT prove — precision is still poor

212 absent findings is mostly noise, and the report says so plainly. The bottleneck is extraction, not matching:

- **Descriptive vs directive confusion.** Most absent directives are crate descriptions ("fuel-core-types: dtypes, shapes, layouts…") and most absent named capabilities are status notes and rules ("Rule 1: dependencies flow downward only", "As-built status (2026-07-29)", "That is no longer true and was misleading"). List items and bold phrases in a descriptive document are not directives. Telling them apart needs the pragmatic lens (what the text is doing: directive vs description vs status note) — the next slice, not this one.
- **Capability names vs emphasized prose.** The bold filters cut the worst of it (v1→v2: 366→241), but bold is still used for emphasis, rules, and asides. Real capability-name extraction is its own lens.
- **Misses against the audit.** Coupling, pick_route, and rule registry do not appear in the absent list — coupling because `04-optimization.md` was out of the corpus (scope), the other two because the ROADMAP mentions them (mentioned ≠ tasked — the check cannot tell a passing mention from a task; that triage is a human's job, stated in the check's own docs).
- **Synonymy beyond the seed.** The alias table works, but it holds exactly one group (DAG/graph). Every other synonym pair in the corpus is unbridged. Growing the table is data work, and it rots like any data — the maintenance story the design note already flags.

## Verdict

The spike proves the machinery and disproves nothing about the design — but it does not yet produce a usable directed-but-untasked list. What is proven: the claim model, deterministic matching, and alias canonicalization work end-to-end on real 500 KB docs, fast, explainably, with no ML. What is not proven: that extraction precision is good enough to hand a human a list without drowning them. The next slices, in order:

1. **Pragmatic classification** of list items and bold phrases (directive vs description vs status note) — the precision bottleneck.
2. **Capability-name extraction** — names, not emphasized prose.
3. **Wider corpus** — `04-optimization.md`, the decisions log, the design notes.
4. **The doc-vs-doc contradiction lens** — the audit's section 6 item 7 (`06` "cancellation not supported" vs `15` C-2 "can cancel") is the first case.
5. **The Rust language pack** — code as ground truth (signatures/tests/types as the code's claims).

The spike earns the next slice, not the full build. The build stays outside the lanes.

## Slice 1 — pragmatic classification (2026-11-16)

The next slice from the verdict above, built the same day. `src/pragmatic.rs` classifies every extracted claim by what its text is *doing*: Directive / Description / StatusNote / Name, using deterministic lexicons plus structure — imperative-verb and modal lists, status markers (as-built, dated, retired, no-longer), definition-head patterns (bold term + `:` / `.` / parenthetical gloss), descriptive sections (crate boundaries, see-also, where-this-lives), and copula openings (`The X is Y`). The claim model gained a `role` field; the report counts only role-matching claims as findings (directives that direct, names that name) and lists the rest under SET ASIDE, so the lens itself stays checkable. 16 tests in `tests/pragmatic.rs`, written against the spec first; `cargo test` (27 total), clippy, and fmt all clean; full run ~2.3 s.

Results, same corpus and ROADMAP (run `runs/fuel-2026-11-16-v4.txt`):

| run | absent (directives / named / identifiers) | mentioned | set aside |
| --- | --- | --- | --- |
| v3 | 212 (75 / 103 / 34) | 346 | — |
| v4 | 105 (7 / 64 / 34) | 346 | 107 (59 directives as description, 9 as status; 32 named as description, 7 as status) |

What survived, checked against the audit's ABSENT rows: pattern harvest, persisted graph, lookahead window, Judge baseline (directive), cache management — all still absent findings. The DAG is still correctly not flagged as such; "The loaded multi-path graph is the working state" remains absent, but it was already absent in v3 — a long-key matching issue (key `loaded multi path graph`, "loaded" in no ROADMAP row), not a pragmatic regression. The 7 remaining directives read like a usable list: 5 genuinely directed, 2 residual descriptive prose ("Across backends, multiple runs are simultaneously in flight", "The optimizer walks decision points").

Known losses, stated plainly:

- **Cancellation** drops out entirely. The architecture's only cancellation text is negated assertions ("Cancellation is not supported", "dispatched runs are committed") — literally descriptions, so the lens sets them aside. The gap is real (audit ABSENT), but the doc-vs-doc contradiction lens (slice 4: `06` "cancellation not supported" vs `15` C-2 "can cancel") is the one that catches it, not this check.
- **Tolerance budgets** lose their vehicle: the `03-ir:20` "Side-table metadata" item is a definition, so it is set aside — but tolerance still surfaces via other claims ("Per-call overrides don't mutate the IR", "Tolerance configuration changed"). Sub-claims inside descriptive prose need slice 2.
- **PlanStore** is set aside as a status note ("is likewise gone", dated) — which is correct as far as this check goes: the architecture no longer directs it, it retracts it. The audit's ABSENT row is a doc-vs-code finding (plan-fragment memoization not built), not a doc-vs-roadmap one.

Two bugs found and fixed while building the slice, both worth remembering because they are silent:

- A bold term ending in a period (`**Side-table metadata.** Information ...`) carries the '.' *inside* the span; the first version of the definition rule only looked *after* the span and missed it, reading the gloss's first word as the item's. Fixed by treating a period-terminated bold span as a definition head.
- Lexicon lookups were substring matches: "shall" fired inside "shallow" (a `**Shallow (just-in-time)**` item was misread as a modal directive) and "gone" would fire inside "undergone". All lexicon lookups are now word-boundary phrase matches (`contains_phrase`); tests cover both lookalikes.

Known lexicon weakness, left in: "superseded" fires on domain vocabulary ("superseded nodes are left orphaned"), so a few set-aside items carry the StatusNote label where Description would be more accurate. It changes only the sub-count label, never the set-aside decision.

Verdict: the directed-but-untasked list is now handable for directives (7 items, ~5 real). The named list (64) is still roughly half fragment noise — "Immutable nodes after creation.", "populated at startup", "v1.3 refines landed-vs-intent ..." — which is slice 2's job (capability-name extraction), not this one's. The next slices, in order, stand as before: 2. capability-name extraction, 3. wider corpus, 4. doc-vs-doc contradiction, 5. the Rust pack.

## Slice 2 — capability-name extraction (2026-11-16)

Built on Eric's go-ahead ("Keep going — build slice 2"). `src/names.rs` classifies every named claim as a Name or a Fragment, returning the rule that fired so each verdict explains itself; `pragmatic::classify_name` calls it after the slice-1 checks, and fragments become Description (set aside, counted, listed — the same treatment as slice 1). 12 tests in `tests/names.rs`, written against a spec derived from the v4 run's 64 absent named claims before the implementation; `cargo test` (39 total), clippy, and fmt all clean; full run ~2.3 s.

Rules, first firing wins: version-opening, arrow, quantifier-opening, verb-opening, finite-verb, not-yet, per-fragment, predicative-participle, temporal-tail, sentence-period. Two design points: parentheticals are stripped before any verb scan (a verb in a tag is not the phrase's grammar — "crowding-distance cap (keep-per-device)" is a name), and hyphenated compounds are judged on structure, not substrings.

Results, same corpus and ROADMAP (run `runs/fuel-2026-11-16-v5.txt`):

| run | absent (directives / named / identifiers) | mentioned | set aside |
| --- | --- | --- | --- |
| v3 | 212 (75 / 103 / 34) | 346 | — |
| v4 | 105 (7 / 64 / 34) | 346 | 107 |
| v5 | 73 (7 / 32 / 34) | 346 | 139 (named as description 32 → 64) |

The 32 kills, by firing rule: finite-verb 9, quantifier-opening 6, verb-opening 5, per-fragment 3, arrow 2, predicative-participle 2, temporal-tail 2, not-yet 1, sentence-period 1, version-opening 1. All 32 were predicted from the v4 list before the run; the diff of the absent-named sections matches the prediction exactly, and no real name was lost: the directives are byte-identical v4 → v5 (7 items), identifiers unchanged (34), mentioned unchanged (346).

What survived, checked against the audit's ABSENT rows: pattern harvest, persisted graph, lookahead window, cache management — all still absent findings. The DAG is still correctly not flagged.

Known losses, stated plainly:

- **Judge baseline drops out.** Its only surfacing claim is the sentence "ships a bundled baseline Judge dataset in-package", so the finite-verb rule sets it aside. The audit-ABSENT substance is real — the sentence rule cannot tell a sentence stating a capability from a sentence stating a property. Correction to the slice-1 report, which called it "Judge baseline (directive)": in v4 it surfaced as a named claim, not a directive — the directives are identical v4 → v5 and contain no Judge item. Recovering it needs either a narrow carve-out ("ships/provides a bundled X" — corpus-tuned, not built) or the wider-corpus / contradiction slices.
- **Side-table correction.** The slice-1 report said the "Side-table metadata" item was set aside as a definition. Imprecise: the LIST-ITEM bullet carrying the tolerance-budget gloss is set aside, but the bold-span NAME still surfaces as a finding in v4 and v5. What was lost was the tolerance content, not the name.
- **"The loaded multi-path graph is the working state"** stays set aside as a copula description, unchanged from v4 — a long-key matching issue, not a regression.

Residual misses, pinned in tests as current behavior so a later slice can see them change: "hardware-independent and portable" and "recipe in two inverse directions — both mandatory" (bare noun-phrase property statements, indistinguishable from names without semantics) and "orchestrate eager collectives above" (imperative + adjective: a clause the rules cannot tell from a compound name). Borderline keeps (7): the Default/Fallback mode glosses, "Re-optimization from canonical state.", "Path resolution at decision points.", "Synchronization at join points.", "Per-decision-point atomic swap.", "The cached resolved route".

Two traps found while building, both worth remembering because they are silent:

- **Verb-words as nouns.** "Cache management" and "Profile data" open with words in the imperative list ("cache", "profile") — but they are compound names, not clauses. The verb-opening rule now fires for an imperative only on an article object ("keep the per-device ...") or a hyphenated verb phrase ("write-new-file-and-swap"). The imperative lexicon is also missing "orchestrate" (found while testing); left out deliberately — the article guard means it would not fire for this item anyway, and lexicon growth is data work for a later pass.
- **Hyphenated first tokens.** "build-time-closed" opens with "build", an imperative — but it is a compound adjective, not a command. A hyphenated first token now fires only when no later segment is a participle. The same guard is what keeps "Fuel-orchestrated", "mmap-backed", and "length-prefixed" as names.

Verdict: the named list is now handable (32 items: ~22 real names, 7 borderline, 3 known misses). The next slices, in order, stand as before: 3. wider corpus, 4. doc-vs-doc contradiction, 5. the Rust pack. The Judge-baseline loss is the open question for the next slice: the sentence rule buys precision at a real recall cost, and only the wider corpus (more phrasings of the same capability) or the contradiction lens can tell whether that cost is acceptable.

## Slice 3 — wider corpus (2026-11-16)

Corpus change only — no code changes. New directory `corpus/fuel/arch-wide/` (24 files, 623,529 B): the three original files plus exactly the slice-3 additions:

- `04-optimization.md` (62,325 B), fetched from the pinned commit `b0d74efa` via raw.githubusercontent.com.
- `10-decisions-log.md` (241,115 B) — the on-disk copy verified byte-identical to the pinned commit (sha256 `9a2e703fe86ab931d1974b1e482d4759871a2c80b0c2e614c25f2d9a63ebf092`); the re-fetch matched, so the on-disk copy was used.
- 19 design notes from `docs/design-notes/` at the pinned commit (195,820 B total), listed via the GitHub contents API — every file in the directory **except** `2026-11-15-planned-vs-built-audit.md`, which is ground truth for validation only and was never fed to the run. The only file in `arch-wide/` with "audit" in its name is `2026-10-30-qwen35-27b-audit.md`, a model audit note, not the planned-vs-built audit.
- `corpus/fuel/arch-run/` untouched; the v5 run reproduces byte-identically against it.

Scope observation (one line): `docs/architecture/` at the pinned commit holds 16 files (`00`–`15`); slice 3 uses 5 of them. The 11 not added — `00`, `01`, `05`, `07`, `08`, `09`, `11`, `12`, `13`, `14`, `15` — are out of scope. The audit's ABSENT rows for pattern harvest (`08`) and persistence (`11`) therefore still have no dedicated doc in the corpus.

Results (run `runs/fuel-2026-11-16-v6.txt`):

| run | claims | absent (directives / named / identifiers) | mentioned | set aside |
| --- | --- | --- | --- | --- |
| v3 | 558 | 212 (75 / 103 / 34) | 346 | — |
| v4 | 558 | 105 (7 / 64 / 34) | 346 | 107 |
| v5 | 558 | 73 (7 / 32 / 34) | 346 | 139 |
| v6 | 3657 | 1164 (290 / 324 / 550) | 2044 | 449 |

Full run ~14.7 s, release build. `cargo test` 39 green, clippy 0 warnings, fmt clean; the v6 run re-run once and diffed byte-identical, and the v5 run re-run against `arch-run/` and diffed byte-identical.

### v5 → v6 diff: strictly additive

All 73 v5 absent findings and all 139 v5 set-aside items survive unchanged (same claim ids, same verdicts); the original three files contribute exactly the same 73 absent + 139 set-aside in v6. New: 283 directives, 292 named, 516 identifiers, 310 set-aside. Nothing dropped, nothing reclassified.

Where the 1,164 come from: decisions log 496 (43% — 70 directives / 139 named / 287 identifiers); `04-optimization.md` 81 (22 / 40 / 19); the 19 design notes 587 combined — largest: nvidia licence read 63, qwen35-27b plan 54, mlmf move survey 51, qwen35-27b audit 43, fireline seam plan 36, pathfinder note 28.

### The four audit checks

- **Coupling surfaces: yes.** `04-optimization.md:107` "conditional cost adjustments" is an absent directive under the section "Coupling between decisions" — the audit rates coupling STUB (`.coupling` read in 1 place, a test asserting it is empty). Note the claim is the mechanism, not the word "coupling" itself; the section header carries the name.
- **Judge baseline resurfaces: yes — the slice-2 loss is recovered by the wider corpus.** `10-decisions-log.md:115` "Local Judge baseline initialization." is an absent named claim; the audit rates "Bundled Judge baseline; online Judge accumulation" ABSENT. (The directive-shaped twin of the same entry is set aside as a description — the name survives, the sentence doesn't.) `04:65` and `04:307` also surface Judge-data claims, but those are the cost model's empirical layer, not the baseline. This answers the slice-2 open question: the sentence rule's recall cost is partly repaid by more phrasings of the same capability.
- **pick_route / rule registry stay "mentioned ≠ tasked": yes.** `pick_route` appears 0 times in the v6 output — no absent or set-aside claim carries it; it matches ROADMAP rows (mentioned) while the audit rates it STUB. The rule-registry substance likewise: `04:353` surfaces only the rule name `SoftmaxLastDimFuseRule` (identifier); the registry claims at `04:357–361` ("PR 3's `Rule` trait extends…", "Decomposition rules are PR 3's lowering family", "Per-decision-point alternatives are now landed", "Concurrent optimize-and-execute is not yet landed") are all set aside as descriptions. The audit's "RuleRegistry / FusionRule / find-best-cover on realize | ABSENT on realize" is not surfaced as a finding by this lens.
- **DAG stays correctly unflagged: yes.** No absent finding is keyed "multi-path dag" or "multi-path graph". "The loaded multi-path graph is the working state" remains set aside as a copula description, unchanged from v4/v5. The closest item is a fragment, "in place into the multi-path graph" (`04:3`, named) — a piece of a status paragraph, not the DAG concept.

### What the new `04` findings add (real, per the audit)

- Rule properties: `04:65` "Cost contribution", `04:67` `Concurrent`, `04:68` `WholeGraph` → audit §2 "Rules declare cost/error/frontier-compat; Concurrent/WholeGraph | ABSENT".
- Sliding window: `04:151`/`153`/`158`/`159`, `04:239` → audit §3 "Concurrent optimize-and-execute, frontier, commit, regional swap | ABSENT".
- Plan-fragment memoization: `04:262` "Persisted-plan cache", `04:266` "Record-once-replay execution", `04:271` "Activation-pool sizing" → audit §3 "Plan-fragment memoization | ABSENT" (and §4 persisted graph ABSENT).
- Tolerance budgets: `04:332–335` → audit §2 "Per-op/per-region tolerance budgets… | ABSENT".
- Borderline-real: `04:100`/`101` alternative storage and `04:129`/`130` Pareto-frontier survival → audit §1 "Frontier over whole paths | PARTIAL: per node".
- `04` noise, stated plainly: `04:41` "A panic! in decompose is always a bug" (an assertion about code, not a capability), `04:103` "meaningful diversity not enforced" (a known-gap note), `04:168` "Implementation complexity is high" (commentary, counted as a directive), and 9 named claims from the line-3 status paragraph ("lower-then-differentiate pre-pass", "cross-device placement honest", "in place into the multi-path graph", "single central metric" — fragments of a version paragraph, not names).

### Does the decisions log drown the list? Yes — and the pragmatic lens only partly holds it back

The log contributes 496 absent (43% of the total) while the lens sets aside only 133 of its 629 flagged items (21%), against a 66% set-aside rate in v5 (139 of 212). The lens has no genre concept: a log records decisions made, rejected, and superseded — it does not direct. Concretely: 31 of its 70 absent directives contain "Rejected"/"rejected" ("Continue piecemeal phase work without consolidation…; rejected", "Op-shape D…; rejected because…") — rejected options surfacing as directives is pure noise, and "Rejected" is not in the lens's status lexicon. The "What counts as material" list (lines 39–48, versioning-convention criteria) surfaces as 7 directives. The named list is table first-cells and emphasis ("Sections affected", "Five competitive edges.", "The bet stated as compounding."). The 287 identifiers are file paths of old docs, type names from historical entries, and a Windows path (`C:/Projects/frontier-prototype`). The log needs a decision-outcome filter (rejected/superseded/withdrawn) or genre-aware handling before it belongs in this check — that is slice-4-adjacent work, not a code change made here.

The design notes have the same genre problem in miniature: 191 absent directives across 19 notes, in two patterns. The pathfinder note (28 absent) is a plan *for* ROADMAP row 157 — its title names the row — but the lens has no row-ownership concept, so its implementation details ("Compute dominators and post-dominators over the realize-reachable DAG", "Keep a region only if the runner-up stretch is within the frontier's competitiveness bound") surface as untasked. They are tasked, via the row; the row's one-line text just doesn't contain their words. And the nvidia licence note (63 absent, 30 directives) is a licence survey: its "directives" are legal-analysis prose ("GitHub files are pinned to a commit, so the text can be…"), not Fuel capabilities. A note about something else entirely still feeds the lens.

### Cancellation: still not a finding

The only cancellation mentions are set aside as descriptions (`06:73` "dispatched runs are committed"), plus a decisions-log summary item (`10:948`, "Seven clauses: C-1…, C-2 bounded quantum + cancellation…") which lists section-15 clause names, not a cancellation directive. Slice 4 (doc-vs-doc contradiction, `06` "not supported" vs `15` C-2 "can cancel") remains the lens that catches it.

### Identifiers: not handable at this corpus width

550 absent (34 → 550), 287 from the decisions log alone. The identifier kind was already the weakest in v5 and the wider corpus multiplies it 16×. The report's counts keep it separate, as before — but the honest statement is that nobody should read the identifier list as findings.

### Verdict

Slice 3 answers its questions and the list is worse. The wider corpus recovers the two things it was asked to recover (coupling, Judge baseline) and keeps the DAG unflagged, but the headline count (73 → 1,164) is a step backward in usability: the log and the notes are the wrong genres for a directed-but-untasked lens as it stands, and the pragmatic lens's set-aside rate falls from 66% to 28% on them. The `04` file alone is a genuine improvement (81 findings, most of them audit-real). The next slice should not be more corpus — it should be genre handling (rejected/superseded outcomes, note-to-row ownership) or the contradiction lens (slice 4), which is the one the audit's §6.7 cancellation case is waiting on. The corpus directory stays as built; the v6 run is the record of what the wider corpus does without that handling.

## Slice 4 — genre handling (2026-11-16)

Code only — corpus unchanged (same `arch-wide/` dir, ROADMAP, aliases; the decisions log's sha256 re-verified identical to the slice-3 record). Two mechanisms, in the order the slice-3 verdict named them.

### Mechanism 1: the decision-outcome filter (`src/pragmatic.rs`)

A claim recording a rejected / withdrawn / declined outcome is a status note about a decision, not a directive. `OUTCOME_MARKERS` (`rejected`, `withdrawn`, `declined`) are checked in `is_status_note`, status-first as in slice 1. "superseded" is already a status marker and stays where it is; the slice-1 weakness (it fires on domain vocabulary, "superseded nodes are left orphaned") is preserved, not fixed here.

The marker must be a standalone whitespace-delimited word in the prose, outside backtick spans. Three real cases from v6 force that shape, and all three are pinned in tests:

- "hard-rejected when nonzero" (`10:746`) is what `validate()` does to bad input — validation behaviour, not a decision. A token-based match would fire on it; the standalone-word rule does not.
- `` `Rejected` `` (`10:768`) is an enum variant the ingestion service returns, not a decision recorded in prose. Backtick spans are stripped before the check (code is another language, the parse module's own principle).
- "verify→adopt/reject" (`10:769`) is a process with a reject step; the bare verb "reject" is not the outcome participle "rejected".

**The section header is deliberately NOT checked — a deviation from the brief, on evidence.** The brief said the marker sits in the item text or the section header, and to use the section. Both outcome-sections in the log contain live design, so a header is not a verdict on everything under it:

- Under "save-all-activations rejected": the derived-weights variant (`10:245`) and the L1/L2/L3 sequencing (`10:246`) are live design; the header's "rejected" modifies the save-all-activations *option*, not the section's subject (the L3 snapshot artifact, which was not itself rejected — `10:241` rejects dropping it).
- Under "memoization is DECLINED": the live replacement (`10:1141`, invariance declaration + optimizer hoist, "remains open") and a real missing capability (`10:1171`, constant folding — "the missing capability... an ordinary and defensible optimizer feature") sit under the declined header. A section check would hide both.

So the two section-only items (`10:245`, `10:246`) stay findings — defensible on their merits (a design decision to implement someday; conditional future work). The section is not ignored out of caution; it was tested against the evidence and it over-fires in both places it could fire.

No clean deterministic signal exists for the "What counts as material" criteria list (`10:40–48`, 6 directives) — left as a stated residual, not chased through the lexicon.

### Mechanism 2: note-to-row ownership (`src/ownership.rs`, `src/main.rs`)

A design note is owned by a ROADMAP row when (a) its title names the row ("ROADMAP row 157", "Row 137 step 6.3b") and the row exists, or (b) a row's text cites the note's path (`design-notes/<file>`); the owner is the lowest-numbered citing row. Worked out from the real cases:

- The row-start pattern is `N. **` — number, period, bold text. The bold marker is load-bearing: the ROADMAP contains numbered sub-lists inside row bodies ("1. Add `StorageMap` to `Graph`."), and a bare `N. ` pattern would misattribute a citation to "row 1".
- Rows can span several lines, so (b) takes the nearest preceding row-start (the kiss note's citation at ROADMAP line 990 belongs to row 7, which starts at line 933).
- (a) and (b) never disagree in this corpus (all five overlap cases agree on the row). GAP ids corroborate but are not needed as a third signal — both GAP-titled notes also name their row.
- Ownership is a file-level property, separate from a claim's role. `main.rs` partitions absent findings: reportable + owned file → a new OWNED BY A ROADMAP ROW section (counted, listed with the row number); an owned file's descriptions and status notes stay in SET ASIDE, where the role is the more specific reason. The tradeoff, stated plainly: an owned note's details count as tasked even though the row's one line does not cover them, so a genuinely untasked detail smuggled into an owned note would be hidden — that is the recall cost of this mechanism.

17 of the 19 design notes are owned. The 2 unowned ones name no row and are cited by none: `2026-11-14-arc004-cure.md` (29 absent) and `2026-11-07-fireline-fuel-baracuda-seam-plan.md` (36 absent).

**The nvidia note is owned — against the assumption going in.** The brief said it "is not owned by any row". Row 141 says otherwise: "Finish the UNREAD parts of the Nvidia terms read... `docs/design-notes/2026-11-02-nvidia-licence-read.md` (sections 3 to 5b) marks what was not read... Method as in the note... Done when each item is read or marked UNREAD." The note is row 141's working paper, and the row continues its work — signal (b) owns it by row 141. That is the general rule working, not a stretch for this file; but it is flagged here because the assumption going in was the opposite, and the parent may want to revisit whether citation-as-evidence should count as ownership. The other (b)-only cases are the same shape (the survey as the basis of row 42's decision, the audit as evidence for row 40, seam-s3 as the analysis row 137 executes) — no passing mentions were found among the 14 cited files.

Results (run `runs/fuel-2026-11-16-v7.txt`):

| run | claims | absent (directives / named / identifiers) | mentioned | set aside | owned |
| --- | --- | --- | --- | --- | --- |
| v3 | 558 | 212 (75 / 103 / 34) | 346 | — | — |
| v4 | 558 | 105 (7 / 64 / 34) | 346 | 107 | — |
| v5 | 558 | 73 (7 / 32 / 34) | 346 | 139 | — |
| v6 | 3657 | 1164 (290 / 324 / 550) | 2044 | 449 | — |
| v7 | 3657 | 688 (89 / 224 / 375) | 2044 | 476 | 449 (175 / 99 / 175) |

Full run ~18 s, release build (a re-run took ~15 s; the spread is run-to-run variance, not the mechanism — the ownership scan is 24 files × 3,351 ROADMAP lines). `cargo test` 55 green (16 new genre tests + 12 names + 16 pragmatic + 11 spike), clippy 0 warnings, fmt clean; the v7 run re-run once and diffed byte-identical; the corpus verified untouched (decisions-log sha256 matches the slice-3 record).

### v6 → v7 diff: nothing new, nothing lost

Absent 1,164 → 688 (−476). Mentioned unchanged at 2,044. Claims unchanged at 3,657 (no extraction change). Verified by id-set diff: **zero** new absent findings, **zero** findings vanished — every dropped id is owned or set-aside.

By mechanism: ownership moves 449 (175 directives / 99 named / 175 identifiers); the outcome filter moves 27 to set-aside (26 directives + 1 named), all in the decisions log. By file: all 17 owned notes go to 0 absent; the log 496 → 469 (directives 70 → 44, named 139 → 138, identifiers unchanged at 287); arc004 29, fireline 36, and the four architecture files (16 / 32 / 81 / 25) unchanged.

Owned, by row: 7: 14 (kiss-corpus-verdict), 40: 43 (qwen35-27b audit), 42: 51 (mlmf move survey), 67: 11 (larger-than-vram), 95: 7 (seam-crate-versioning), 115: 31 (composed-forward), 117: 19 (hybrid-decode), 118: 20 (gguf-header), 124: 6 (kquant), 136: 51 (seams-and-cleanup 21 + structure-measurement 30), 137: 51 (seam-s3 18 + row137-step6-plan 20 + row137-step63b 13), 141: 63 (nvidia licence read), 155: 54 (qwen35-27b plan), 157: 28 (pathfinder).

The outcome filter's 27, exactly: the 24 "X. Rejected — reason" directives (`10:127`, `:130`, `:132–134`, `:159–161`, `:181–183`, `:208–213`, `:239–241`, `:354–357`) plus the two uppercase "REJECTED" items (`10:1483`, `:1485`), plus one named claim (`10:940`, "withdrawn as wrong" — the section header there says "withdrawal of the 'inference center of gravity' claim"). The traps all survive as findings: `10:746` (hard-rejected), `10:768` (`` `Rejected` `` variant), `10:769` (ingestion service), `10:245`/`246` (live design under the rejected section). The bold "declined" spans at `10:1066`/`1497` are Mentioned, not findings (the ROADMAP contains "declined" 8 times), so the filter changes no verdict there; `10:1497`'s directive was already StatusNote in v6 via "gone" ("the old hardcoded errors are gone").

One re-label, no decision change: `04:150`'s directive (the `Concurrent`-rule definition, "the match is rejected") moves Description → StatusNote and stays set aside. The sub-label is slightly wrong — it is a definition, not a status note — the same class of label-only imprecision as slice 1's "superseded".

### Audit guards — all pass

- Coupling (`04:107`, "conditional cost adjustments") still an absent directive.
- Judge baseline (`10:115`, "Local Judge baseline initialization.") still an absent named claim — the guard that mattered most, since the outcome filter operates on the log it lives in.
- DAG still correctly unflagged; "The loaded multi-path graph is the working state" still set aside.
- Cancellation still NOT a finding (0 absent findings mention it).
- pick_route still 0 occurrences in the output; the rule-registry claims at `04:357–361` still set aside as descriptions.
- Plus two guards the section decision rests on: constant folding (`10:1171`) and the invariance-declaration replacement (`10:1141`) are still absent named findings — the live content a section-level check would have hidden.

### Correction to the slice-3 report

The slice-3 section said "the 19 design notes 587 combined". The true design-note total in v6 is **514**; the other 73 are the three original architecture files (`02`: 16, `03`: 32, `06`: 25), which the slice-3 breakdown lumped into the notes bucket. The headline 1,164 was correct; only the breakdown was wrong (496 log + 81 `04` + 73 arch + 514 notes = 1,164). The v6→v7 per-file diff is what proves it.

### Known residuals, stated plainly

- **Log identifiers: 287 of the 375 absent identifiers.** The outcome filter does not touch identifiers (their role is hardcoded Name; `is_reportable` is always true for them) and the log is owned by no row. Old doc paths, historical type names, a Windows path — still the largest single bucket, still not handable as findings.
- **Unowned notes: 65 absent** (arc004 29, fireline seam plan 36).
- **"What counts as material": 6 directives** (`10:40–48`) — criteria, no clean signal.
- **DECLINED-section census data** (`10:1124` "21 `Const` + 20 `BroadcastTo`", `10:1133` "mutates them in place", `10:1139` "What replaces it", `10:1149` "Consequences") — descriptive fragments surviving as named findings under the declined header; the text-only rule cannot tell them from the live items beside them.
- **The owned-section recall cost** (above): a genuinely untasked detail inside an owned note is now hidden by design.

### Verdict

The list is handable again for directives (89, down from 290 — the log contributes 44, the architecture files 29, the unowned notes 16), and the owned section is itself checkable (449 items, each with its row). Named (224) is still roughly half noise — the log's 138 are table first-cells and emphasis, and the architecture files' 72 include the known borderline keeps. Identifiers (375) are still not handable, and the log's 287 are the reason.

**Renumbering:** genre handling was inserted ahead of the contradiction lens, so the contradiction lens is now **slice 5** and the Rust language pack is **slice 6**. The contradiction lens is the next slice — it is the one the audit's §6.7 cancellation case (`06` "cancellation not supported" vs `15` C-2 "can cancel") is waiting on, and nothing in this slice touches it.

## Slice 5 — doc-vs-doc contradiction lens (2026-11-16)

The directed-but-untasked check cannot see a conflict: it only asks whether the ROADMAP mentions a capability. Two architecture documents can disagree with each other — one says a capability is not supported, another requires it — and no ROADMAP row is involved. The first case is the audit's §6 item 7: `06` "cancellation not supported" vs `15` C-2 "can cancel" (audit row 64: "Memory-pressure serialization; cancellation; serial-reduction ordering for determinism | ABSENT | `CancellationToken|cancel_token` 0 hits (06 and 15 C-2 contradict each other)").

### Corpus change — the only one this slice

Doc 15, the consumer-contract document, was not in the corpus. Found via the GitHub contents API for `docs/architecture/` at the pinned commit (the first call returned a 301 to `repositories/1207364956`; `-L` follows it), fetched via raw.githubusercontent.com at the same commit, added to `arch-wide/`:

- `15-consumer-contract.md`, 41,756 B, sha256 `09a103dd53052ca670d54088bad1d8a069eeb6e2989a62afb98a679bebfa973a` (matches the API's size exactly).
- The local `corpus/fuel/architecture/00-index.md` independently confirms the filename ("v1.2 adds **[15 consumer-contract](15-consumer-contract.md)**").
- No other architecture file was added (`00`, `01`, `05`, `07`, `08`, `09`, `11`, `12`, `13`, `14` stay out). `arch-wide/` now holds 25 files. The decisions log's sha256 re-verified identical to the slice-3 record (`9a2e703fe86ab931d1974b1e482d4759871a2c80b0c2e614c25f2d9a63ebf092`).

Side B confirmed in the fetched file: `15:91` heading "C-2 — Bounded quantum and cancellation"; `15:93` "The consumer can say 'advance this set, then return to me' and get control back in bounded time, and can cancel in flight. ..."; `15:53` "a quantum and cancellation token". The C-2 paragraph yields no claim under the prose parser (no list item, no bold span) — which is why the lens must read headings. Doc 15 is NOT owned by any ROADMAP row (its title names no row; rows cite `docs/architecture/15-consumer-contract.md` — rows 78, 94, 136, 150, 842 — but the slice-4 ownership signal looks for `design-notes/<file>`), so its findings stay in absent / mentioned / set-aside.

### The lens (`src/contradiction.rs`, new; `src/parse.rs` gains headings)

A contradiction is a PAIR of statements about the same capability where one side negates it and the other asserts it. Statements come from headings (a document's own position) and from extracted claims (everything except code identifiers — a code observation, not an assertion — and status notes — a retraction is temporal, not a position). Deterministic, lexicons + structure, no ML, spec-first from the real cases, tests before implementation (28 contradiction tests).

Polarity, in order: temporal first ("not yet", dates, retraction words), then negation markers ("not supported", "cannot", "no <capability>", "never", "is not", "does not", … — the exact lexicon worked out from the real corpus cases, word-boundary matching via the existing `contains_phrase`), else positive. Morphology: `same_family` — identical, differing only by a trailing "s", or sharing a common prefix of 6+ letters, so "cancel" / "cancellation" / "cancelled" / "cancelling" are one family while "state" / "statement" (5 shared) are not. Pairing: different files (a document contradicting itself is a different finding, not reported here); opposite polarity (Temporal never pairs — "must support X" vs "X not yet built" is directed-but-untasked, not a contradiction); the smaller subject family-contained in the larger one; shared tokens strong enough (≥ 1 token of 6+ letters, or ≥ 2 tokens).

**The first v8 run found 722 pairs — and 721 of them were noise.** The redesign that followed, in the order the triage named the mechanisms (all five are now in the code, the module docs, and tests):

1. **No contrast split for claims.** The split (", not " → asserted + denied halves) applied to claims produced 399 split-half sides whose subjects came from long gloss text — the largest single noise source. Headings keep the split (they are short; "Dispatch: runs, not single ops" is a real position); claims do not.
2. **The head window.** Claim polarity is read from the first 16 tokens of the text — the head plus the first clause. A negation deep in a 200-word gloss ("never silent" in `10:442`, "no materialized attention matrix" in `10:444`, "the graph itself is not rebuilt" 45 words into `03:17`) does not govern the head's capability. Pinned by a test on the real `03:17` text.
3. **Negation of a bad thing is positive.** "does not break" (arc004 K3), "never panics", "never silent" assert the absence of something bad, not the absence of a capability. When the word after a negation marker is in `BAD_WORDS` (break / fail / panic / corrupt / lose / silent / …), the statement is Positive.
4. **The positive side must have at least two subject tokens.** A bare name ("accuracy", "storage", "contract") asserts almost nothing; single-token positive sides were a major false-pair source.
5. **"contract", "interface", "backend" are structural words** (removed from subjects — "backend" alone carried 81 pairs in the first run), and **tokens containing digits are dropped** ("qwen35", "conv2d" are versions and identifiers, not capabilities).

After the redesign the second v8 run finds **37 pairs**. By marker: "no <capability>" 30, "never" 2, "is not" 2, "not supported" 1, "isn't" 1, "are not" 1. By positive side: 19 directive claims, 14 named claims, 4 headings.

Results (run `runs/fuel-2026-11-16-v8.txt`):

| run | claims | absent (directives / named / identifiers) | mentioned | set aside | owned | contradictions |
| --- | --- | --- | --- | --- | --- | --- |
| v3 | 558 | 212 (75 / 103 / 34) | 346 | — | — | — |
| v4 | 558 | 105 (7 / 64 / 34) | 346 | 107 | — | — |
| v5 | 558 | 73 (7 / 32 / 34) | 346 | 139 | — | — |
| v6 | 3657 | 1164 (290 / 324 / 550) | 2044 | 449 | — | — |
| v7 | 3657 | 688 (89 / 224 / 375) | 2044 | 476 | 449 (175 / 99 / 175) | — |
| v8 | 3846 | 764 (107 / 271 / 386) | 2115 | 518 | 449 (175 / 99 / 175) | 37 |

Full run ~14 s, release build (the first v8 run's ~21 s included ~6 s of compile time). `cargo test` 83 green (28 contradiction + 16 genre + 12 names + 16 pragmatic + 11 spike), clippy 0 warnings, fmt clean; the v8 run re-run once and diffed byte-identical.

### The cancellation pair — real

```
NEG 06-runtime.md:71  [Runtime > Dispatch: runs, not single ops]
    "Cancellation is not supported" — negated by "not supported"
POS 15-consumer-contract.md:91  [Consumer contract > What fuel provides
    to consumers > C-1 — Capacity advertisement > C-1 holds where fuel
    controls a finite budget — and memory-mapped host storage is not that]
    "C-2 — Bounded quantum and cancellation" — heading (no negation marker)
shared: cancellation
```

The headline marker "not supported" fired exactly once in the whole run — on the headline case. Side A is the heading; side B is the C-2 heading (the "can cancel in flight" paragraph at `15:93` yields no claim, so without headings this pair would not exist). Cancellation is still NOT an absent finding — `06:73` ("dispatched runs are committed") stays set aside as a description, exactly as in v7; the conflict is visible only through this lens.

**Code reality, stated so the finding is not overstated:** the audit says "no cancel" *in code* — 0 hits for `CancellationToken|cancel_token`. The lens flags a **doc conflict** (06 vs 15), not a code state. What the pair proves is that the two documents disagree; whether either side matches the code is the audit's job, not this lens's.

### Triage of the other 36 pairs — 1 real, 36 noise

Every pair below was read against its source text. The noise falls into six classes; the counts sum to 36.

**One generic word carrying the pair — 21.** The shared token is a word with several meanings in this codebase, and the two sides mean different ones:

- **"budget", 11 pairs** — all from the single negated side `2026-10-25-larger-than-vram.md:25` ("**No budget**: `PlanOptions` has no device byte budget, and no policy turns the probed free VRAM into one"). The positives are tolerance budgets (`03:118`, `04:129`, `04:320`, `04:328`, `04:382`), a parallelism budget (`04:311`), and doc 15's C-5 resource budget (`15:3`, `15:116`, `15:120`, `15:134`) plus a memory-budget condition (`multi-node:65`). Different budgets entirely. The closest four (doc 15's resource budget) are still a different artifact: a contract clause requiring a per-device resource budget (admissibility + quantity, an entitlement axis) vs a struct lacking a byte-budget field — a gap, not a contradiction.
- **"harvest", 4 pairs** — `10:1122` ("No harvest, measured. A token-invariance census of a decode step…") vs pattern-harvest telemetry (`02:197`, `03:132`, `04:383`, `06:199`). The log's "harvest" is a runtime value census (declined: the census found zero invariant nodes materialize); the positives are pattern harvesting for rule discovery. Same word, two harvests.
- **"lifecycle", 3 pairs** — `15:265` ("**Work lifecycle.** No streaming protocol, no retry, no request identity…", under "What fuel does NOT decide") vs `03:208` and `06:201` ("See also" links to `14-lifecycle.md`) and `10:314` (a terminology citation of the same doc). The positive sides are references, not assertions that fuel decides the lifecycle.
- **"written", 2 pairs** — `15:189` ("attention probabilities are … **never written** on the fused flash arm" — not materialized) vs "hand-written" (`10:590`, qwen35-audit `:113` — kernels authored by hand). A shared verb form, unrelated meanings.
- **"network", 1 pair** — `06:149` ("This requires **no network** — important for offline deployments", the bundled Judge baseline) vs a paper citation ("Quantization and Training of Neural Networks…", nvidia-licence `:154`). Computer network vs neural network, and the positive side is a citation.

**The denied token is absent from the positive side; the shared tokens are the topic — 6.**

- `10:83` ("No separate `NodeKind` discriminator type") vs `03:65` (`Op::Fused` as the registry-extended arm) and `03:182` ("Op enum (primitives)") — both sides discuss Op-shape A; neither positive side mentions NodeKind.
- `04:369` ("No e-graph saturation on the per-realize hot path") vs `15:167` ("hot-path" — a C-6 regime name) and row137-step63b `:29` ("Realize a graph…") — the shared words are the location, not e-graph saturation.
- `03:77` (Op::Scan: "No `LoweringRule` matches `Op::Scan` … no native kernel in Phase 1") vs `10:347` ("base map") — the negation governs lowering rules and native kernels; "base map" is incidental gloss vocabulary.
- `06:176` ("No runtime kernel selection across paths the optimizer didn't keep") vs `03:195` ("Per-path KernelRef") — the positive side names a data structure, not the denied activity; the pair also rests on the family rule overmerging "kernel" with "KernelRef" (6-letter shared prefix, different things).

**A bare label as the positive side — 2.** `04:371` ("No *untrusted* user-installable rules at runtime") vs `10:338` ("untrusted-user") and `10:339` ("untrusted user-installable rules") — the log's named claims are labels for the category being ruled out. Both sides agree untrusted-user rules are out; the label just names the category.

**Scoped negation — the denial is about one layer, the assertion about another — 4.**

- `10:402` ("built with **no data-carrying `Const` and no device handle** (a `decompose` fn has neither)") vs seam-s3 `:26` and `:94` (finding the device handle a kernel needs, at the executor/backend seam). Recipes don't carry handles; the executor resolves them at runtime. Different layers.
- qwen35-audit `:21` ("The multimodal wrapper's `model.visual.*` and `mtp.*` tensors are not read. Text-only is the stated scope.") vs qwen35-plan `:33` ("Tensors (read).") — the negation is scoped to a subset (visual/mtp tensors); the positive side is generic and never asserts that subset.
- structure-measurement `:176` ("`fuel-backend-contract` … is not on FireLine's [list]") vs arc004 `:143` ("Seam 3 (executor and backends). Steps 3-5 are the cleanup row 137 prescribes…") — the denial is about FireLine membership; the assertion is about row-137 prescription.

**A scope note misread as a refusal — 2.**

- row137-step6-plan `:1` ("ROADMAP row 137 step 6: move backend-gated code out of `fuel-dispatch` (read and plan, no code)") vs seam-s3 `:119` ("**Move backend-gated code out of `fuel-dispatch`** (the JIT ingest path first)…"). "(read and plan, no code)" means this note wrote no code; both sides agree the move should happen. The "no" + "code" rule cannot tell a document-scope note from a capability refusal.
- kquant `:54` ("No timing or memory measurement of a real GGUF; resident bytes are asserted on a synthetic") vs qwen35-gguf-header `:1` ("Qwen3.5 GGUF (`qwen35`): what a real file contains") — a "Not covered" gap list (no measurement exists) vs a note describing what a real file contains (a header parse, not a measurement). Denied activity vs asserted artifact.

**A copula classification misread as a capability denial — 1.** mlmf-move-survey `:60` ("`WeightStorage` and everything in `fuel-tensor/src/lazy.rs` that is not F9…") vs kquant `:10` ("`WeightStorage` … has `F32`, `BF16`, `Q4_0`, `Awq`, `WithLoRA`.") — "is not F9" is a survey classification, not a denial that WeightStorage exists; both sides agree it exists. (The shared tokens also include path fragments — "src", "lazy", "rs" — which are not capabilities.)

The six classes: generic word 21 (11 budget + 4 harvest + 3 lifecycle + 2 written + 1 network), denied-token-absent 6, bare label 2, scoped negation 4, scope note 2, copula classification 1 — 36 in all.

### v7 → v8 diff: the lens is additive

Verified by id-set diff over the run files: **no v7 verdict changes** — every v7 absent / set-aside / owned id is present in v8 with the same verdict, and every new id comes from `15-consumer-contract.md` only (absent +76: 18 directives, 47 named, 11 identifiers; set aside +42; mentioned +71; owned unchanged at 449). Doc 15 contributes 189 claims (76 absent + 71 mentioned + 42 set aside = 189). The contradiction count (37) is separate from the directed-but-untasked numbers and changes none of them.

One near-miss the diff caught, recorded here because the invocation matters: the first v8 re-run in this session omitted `aliases.txt` (the fourth argument — the Reproduce command below includes it, as v6/v7 did). Without it, "multi-path graph" never canonicalizes to "multi-path DAG", and the named claim `04:3` "in place into the multi-path graph" flipped from absent to mentioned (a ROADMAP row contains "in", "place", "into", "multi", "path", "graph"). The id-set diff showed exactly one v7 id missing; the final v8 run uses `aliases.txt` like v6/v7, and the diff is clean.

### Audit guards — all pass

- Coupling (`04:107`, "conditional cost adjustments") still an absent named finding.
- Judge baseline (`10:115`, "Local Judge baseline initialization.") still an absent named finding (its directive twin stays set aside as a description).
- DAG still correctly unflagged — the word appears in the CONTRADICTIONS section only inside quoted context (a section title, "fully-decomposed primitive DAG", and a link anchor); no pair is about the DAG.
- "The loaded multi-path graph is the working state" still set aside.
- pick_route still 0 occurrences in the output (3 ROADMAP hits — mentioned ≠ tasked, as in v6/v7); the rule-registry claims at `04:357–361` still set aside as descriptions.
- Cancellation still NOT an absent finding — the only "Cancellation is not supported" in the output is the NEG side of the contradiction pair above.

### Correction to the slice-4 report

The slice-4 guards line calls coupling (`04:107`, "conditional cost adjustments") "an absent directive". It is an absent **named** finding, not a directive: the text is a bold span in prose ("The optimizer encodes these couplings as **conditional cost adjustments**…"), so the extractor makes it a Named claim, and it sits in ABSENT NAMED CAPABILITIES in v6, v7, and v8 alike. The verdict — absent, in no ROADMAP row — is unchanged in all three runs; only the kind label was wrong.

### Known limits — what the lens does NOT catch

- **Scope is statement-level, not token-level.** A gloss that negates Z while the head names X is read as a negation of X whenever the marker falls inside the head window (16 tokens). The window cut the false negations from 722 pairs to 37; it cannot cut the ones whose marker sits next to the head but governs something else (`10:402`'s device handle, `15:189`'s "never written").
- **One word, several meanings.** "budget", "harvest", "network", "written" each carry pairs across unrelated senses. The lens has no sense disambiguation; a shared token is a shared token. The budget cluster (11 pairs from one negated side) is the shape this failure takes at scale.
- **The denied token need not be named by the positive side.** When the positive side shares only the topic vocabulary (Op-shape A, hot path, base map), the pair is noise the subset rule cannot block, because the subset rule checks containment, not whether the denied thing is named.
- **References are not positions.** "See also" links and terminology citations (`14-lifecycle`) read as positive statements. A reference to a doc is not an assertion of its contents, but the lens cannot tell them apart.
- **Scope notes read as refusals.** "(read and plan, no code)" and "Not covered" gap lists fire the "no" rule. A scope vocabulary ("no code", "not covered", "no measurement") would need its own rule, and it was not added — the two cases are named here instead.
- **Synonyms the alias table does not hold do not meet.** "cancel" meets "cancellation" by morphology; truly different words for one capability do not meet at all. Roots shorter than 6 letters with a consonant mutation ("admit" / "admission") do not meet either.
- **Same-file pairs are excluded by design** — a document contradicting itself is a different finding, and none was checked for here.
- **It does not check the code.** The cancellation pair is a doc conflict; the audit's 0-hit code finding stands on its own. A doc pair and a code state can agree, disagree, or be unrelated, and this lens sees only the docs.

### Verdict

The lens works — on exactly one pair. The cancellation case the audit named is found, with both sides quoted and the rule that fired, and the directed-but-untasked numbers are provably unchanged. The other 36 pairs are noise, and the noise is now named by mechanism rather than hidden behind the one good case: the dominant failure is a single generic word ("budget", "harvest") carrying pairs across unrelated senses, followed by shared topic vocabulary where the denied thing is never named. The next step, if this lens is worth a slice 7, is not more lexicon — it is scope: the denied capability has to be the thing the positive side asserts, not a word the two sides happen to share. That needs the denied token to appear in the positive side (not just shared tokens), or a sense table for the overloaded words ("budget", "harvest") — both are data work, and the 37-pair triage above is the spec for it.

**Renumbering (from slice 4):** the contradiction lens is slice 5; the Rust language pack is slice 6.

## Slice 6 — Rust language pack (2026-11-16)

The design this slice proves is Eric's, from the requirements note: "languages" means as distant as English versus Rust, so claims in text documents can be transparently checked against the code — code is the ground truth for whether a documented capability exists. The DAG failure was a three-way check: architecture notes vs ROADMAP vs code. The language-pack seam is a Rust tokenizer: signatures, tests, and types as the code's claims.

### Corpus change — the only one this slice

The Fuel Rust source at the pinned commit `b0d74efafed63d9700ec62f652468b197dd46a88`, fetched via the GitHub codeload tarball into `corpus/fuel/code/`:

- Tarball 15,275,146 bytes, 2,322 entries; tree 51M, **951 `.rs` files, no `target/` directory** (verified: 0 directories named `target`).
- The decisions log's sha256 re-verified identical to the slice-3 record (`9a2e703fe86ab931d1974b1e482d4759871a2c80b0c2e614c25f2d9a63ebf092`). No other corpus file was touched.
- The planned-vs-built audit (`corpus/fuel/design-notes/2026-11-15-planned-vs-built-audit.md`) stays ground truth for validation only and is never an input.

### The lens (`src/rust_code.rs`, new; `src/lib.rs` + `src/main.rs` wiring)

Deterministic and std-only — **Cargo.toml has zero dependencies: no syn, no external crates. The Rust reader is a hand-rolled line-pattern scanner**, and the gaps that buys are named below. Spec-first from the real cases, tests before implementation (33 rust_code tests).

**Code claims.** One pass per `.rs` file, brace-tracked scopes, comments / string / char literals stripped first (doc-test code inside `///` blocks never counts). Recorded, each with name, file:line, kind, and a test-only flag: `pub fn` / `pub struct` / `pub enum` / `pub trait` / `pub type` / `pub const` / `pub static`, `impl ... for ...` (qualified by the trait), `#[test]` fns, enum variants, struct `pub` fields, and methods inside impl and trait blocks (qualified `Type::method`, including private `async fn` methods). Every *mention* (any occurrence of the name outside comments, definition lines, and `pub use` re-export lines excluded) is recorded separately, because the audit's distinctions turn on it. 25,301 definitions across 951 files in the v9 run.

**Provenance — production vs test/fixture.** This is load-bearing, not a nicety: a naive "name exists in code" check calls things built that the audit rates ABSENT/STUB. A code claim is TEST-ONLY when its file lives under `tests/` or `benches/`, inside a `#[cfg(test)]` module, in a crate whose first path component contains `fixture` (covers `-fixture` and the plural `-fixtures`) or `test-support`, or its name ends in `Fixture` — and methods inherit fixture-ness from their impl type (`impl Model for FixtureModel { fn name }` is scaffolding even in a plain file). The extension beyond the brief's "`-fixture`" is deliberate, on evidence: `fuel-test-support/src/lib.rs` self-describes as "Shared test-gate helpers", and `fuel-correctness-fixtures` is fixture tooling. The audit's own example pins the rule: `fuel-model-core::Model` has exactly 3 impls — `LocalModel` (in a `tests/` directory), `FixtureModel` and `UmbrellaModel` (in `-fixture` crates) — all TEST-ONLY.

**Matching.** Code identifiers are split into words (snake_case + CamelCase + SCREAMING_SNAKE), lowercased, then token-subset / `same_family` matching (reused from slice 5: identical, trailing `s`, or 6+ letter shared prefix) against the doc claim's normalized, alias-canonicalized key. A doc claim matches when every key content token is family-matched by the code name's words, OR the code name has ≥2 words all family-matched by the doc key. A one-word code name can never satisfy a multi-word doc key — that is what keeps the `Judge` type from "building" the bundled Judge baseline. Identifier claims try the exact last segment first (`Op::Branch` → `Branch`, preferring the item qualified by the path prefix, `Op`).

**Statuses — the lens's own words, weaker than the audit's.** BUILT = production definition + ≥1 production use; STUB = production definition, zero production uses; TEST-ONLY = no production definition; ABSENT = no definition and no code mention (with a separate "comments only" flag). A fourth status (STUB) was added because the real cases demanded it: `RouteCache` and `LOOKAHEAD_K` are defined in production and never used, which is neither built nor test-only. Use-counting is by kind: fns count calls (`name(` — a `use` import line is not a call); fields count reads (`.name` — struct-literal `name: Vec::new()` initializations are not reads); types count any occurrence. Definition lines, `pub use` re-export lines, and comments never count. Items are keyed by bare name — the conflation is real and documented: every `new` / `resolve` / `len` in the tree is one item (`pub fn new` ×177, `load_from_mmapped` ×161, `forward` ×158 — declarations, counted by grep over the tree, not occurrences).

**Scope.** The doc-vs-code check runs over the v8 absent findings only (764) — those are the ones whose build status is unknown. Mentioned / set-aside / owned claims are not checked; the report says so. The lens is additive: the doc-vs-roadmap numbers and the contradiction count must be identical to v8, proved by diff below.

Results (run `runs/fuel-2026-11-16-v9.txt`, ~32–34 s):

| run | claims | absent (directives / named / identifiers) | mentioned | set aside | owned | contradictions | doc vs code (of the absent) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| v3 | 558 | 212 (75 / 103 / 34) | 346 | — | — | — | — |
| v4 | 558 | 105 (7 / 64 / 34) | 346 | 107 | — | — | — |
| v5 | 558 | 73 (7 / 32 / 34) | 346 | 139 | — | — | — |
| v6 | 3657 | 1164 (290 / 324 / 550) | 2044 | 449 | — | — | — |
| v7 | 3657 | 688 (89 / 224 / 375) | 2044 | 476 | 449 (175 / 99 / 175) | — | — |
| v8 | 3846 | 764 (107 / 271 / 386) | 2115 | 518 | 449 (175 / 99 / 175) | 37 | — |
| v9 | 3846 | 764 (107 / 271 / 386) | 2115 | 518 | 449 (175 / 99 / 175) | 37 | **236 built, 34 stub, 22 test-only, 472 absent** |

Doc-vs-code breakdown by kind (of the 764 absent findings):

| kind | built | stub | test-only | absent |
| --- | --- | --- | --- | --- |
| directives (107) | 18 | 4 | 0 | 85 |
| named (271) | 30 | 8 | 4 | 229 |
| identifiers (386) | 188 | 22 | 18 | 158 |
| **total (764)** | **236** | **34** | **22** | **472** |

Of the 472 ABSENT, 378 are "comments only" (the claim's words appear in comments but the capability has no definition or mention) and 94 have no words anywhere in the tree. The bulk of the BUILT verdicts are identifiers (188 of 236) — named code things the ROADMAP never mentions but the code has; the prose claims are mostly still absent in code (229 of 271 named, 85 of 107 directives), which is the expected shape: the docs describe more than the code has built.

`cargo test` 116 green (33 rust_code + 28 contradiction + 16 genre + 12 names + 16 pragmatic + 11 spike), clippy 0 warnings, fmt clean; the v9 run re-run once and diffed byte-identical.

### v8 → v9 diff: the lens is additive

Verified by stripping the new summary lines and the DOC VS CODE section from v9 and diffing against v8: **byte-identical**. Every v8 absent / mentioned / set-aside / owned id and every one of the 37 contradictions is unchanged; the only new content is the `code items scanned` / `doc vs code` summary lines and the 764-line DOC VS CODE section. The guards all still pass: coupling (`04:107`) still an absent named finding; Judge baseline (`10:115`) still absent named; DAG still correctly unflagged (no contradiction pair about it); "The loaded multi-path graph is the working state" still set aside; pick_route still 0 occurrences in the output; registry claims `04:357–361` still set aside as descriptions.

Two wrong expectations in the slice's own tests were caught during verification and corrected, both with the lens's behavior documented rather than the test bent to fit: `capability_gated_rules` was first expected STUB (see the validation row below), and a fixture-suffix test used `FixtureModel` — which ends in `Model`, not `Fixture`, so the name-suffix rule correctly does not apply to it (the real `FixtureModel` is caught by the crate rule).

### Validation vs the audit — per row

The audit is ground truth for validation only. Where the lens disagrees, the row says which side is believed and why. The lens's BUILT means **production-reachable** (a production definition with a production call site); the audit's STUB/ABSENT on these rows means **not on the realize path**. Those are different claims, and three of the seven rows turn on exactly that difference.

| row | audit | lens | agree? |
| --- | --- | --- | --- |
| Cancellation (`CancellationToken\|cancel_token` 0 hits) | ABSENT | ABSENT — 0 code mentions (pinned by test); probes of "can cancel" and "bounded quantum and cancellation" against the real index both return ABSENT | **Yes.** (The C-2 claims are not absent findings in doc-vs-roadmap — they are mentioned/set-aside — so they do not appear in the DOC VS CODE section, which covers absent findings only. The code verdict is proved by the probe, not the section.) |
| `pick_route` / `RouteCache` / `LOOKAHEAD_K` | STUB | `pick_route` **BUILT** — called from production code inside `RouteCache::resolve` (`fuel-dispatch/src/ranker/route_picker.rs:182`); `RouteCache` **STUB** (defined, `resolve` never called outside tests); `LOOKAHEAD_K` **STUB** (defined and re-exported, unused) | **No, on `pick_route`; yes on the other two.** Side with the **audit** on `pick_route`: reachability beats provenance. `resolve` is itself never called outside tests (only `cache.resolve(` at `fuel-dispatch/src/ranker/route_picker.rs:692/698/711`, all inside `mod tests`), so the production call site is dead code. The lens cannot see that; the audit can. |
| `RuleRegistry` / `FusionRule` (`optimize_to_fixpoint`, `default_rules(`, `capability_gated_rules(` non-test callers: 0) | ABSENT on realize | `default_rules` **STUB** — both callers (`opt.rs:3832/5210`) are in test modules. `capability_gated_rules` **BUILT** — one production caller, `opt.rs:168`, inside `default_rules` itself. `optimize_to_fixpoint` **BUILT** — called from production via `lower_to_base_map` (`opt.rs:378`), whose callers include `jit_ingest.rs:1184/1215`, `jit_ingest_probe.rs:418`, `runtime_fused.rs:138` (production) | **Yes on `default_rules`; no on the literal predicate for the other two, yes on the substance.** The audit's "non-test callers: 0" is literally false for `capability_gated_rules` (one production caller) and for `optimize_to_fixpoint` (production JIT callers) — but true on the realize path in both cases: `capability_gated_rules`' only production caller is `default_rules`, itself only called from tests (a dead chain); the fixpoint's production callers are JIT paths, not realize. Believed: the audit on the realize-path question, the lens on the literal "is there a production caller" question. |
| Plan-fragment memoization (`PlanStore` deleted, 4 hits, comments) | ABSENT | ABSENT, comments-only — exactly 4 hits in the tree, all in `//!`/`//` comments (`optimize.rs:43/61`, `planner.rs:13/40`); the doc findings at `04:262/266/271` all come out ABSENT (comments only) | **Yes.** Comments-only does not count as BUILT, as designed. |
| Coupling (`.coupling` read in 1 place, a test asserting it is empty) | STUB | The doc finding "conditional cost adjustments" (`04:107`) stays **ABSENT** in code — no code name matches it; its words appear only in comments. The `.coupling` **field** itself is **STUB**: `pub coupling: Vec<CouplingAdjustment>` (`fuel-dispatch/src/ranker/candidate.rs:82`, production); the only `.coupling` read is the test assert at `fuel-dispatch/src/ranker/candidate.rs:166`; the `coupling: Vec::new()` initializations are not reads | **Yes, with a scope note.** The audit's STUB is about the field; the doc finding is about the named concept, which has no code name. Both verdicts are in the run and they are about different things. |
| Judge baseline ("Bundled Judge baseline") | ABSENT | ABSENT — no `Baseline` type or fn exists; the doc claim "Local Judge baseline initialization" (`10:115`) has no name match, and comes out ABSENT (comments only). `populate_dispatch_table` is a production fn with zero callers (`fuel-tensor/src/judge/cache.rs:44`) — STUB as a fn — but it does not match the doc claim | **Yes.** |
| C-6 observe/intervene (`Op::Observe\|Op::Tap` 0 hits) | PARTIAL (C-1..C-7 overall) | ABSENT for the observe/intervene sub-item — 0 hits for `Op::Observe\|Op::Tap` (pinned by test); the C-6 findings come out ABSENT | **Yes.** |

### What the scanner misses — the real gaps, with sizes from this tree

- **Macro-generated items: invisible.** 257 `macro_rules!` invocations and 1,888 `#[derive(...)]` lines in the tree; anything a macro or derive generates (trait impls, helper fns, builder methods) is not a code claim. A doc claim matching a derive-generated method comes out ABSENT.
- **cfg-gated items: counted as present regardless of feature.** 1,389 `#[cfg(...)]` lines. The scanner reads the file as written; an item behind `#[cfg(feature = "cuda")]` counts the same as an unconditional one. Only `#[cfg(test)]` is treated specially.
- **Re-exports are not resolved.** 268 `pub use` lines: a re-export is neither a definition nor a use, so a capability that exists only as a re-exported name from another crate is ABSENT unless the defining crate is also in the tree (it is, here — the whole workspace — but the link is by name, not by path).
- **Bare-name conflation.** Items are keyed by bare name: every `new` / `resolve` / `len` in the tree is one item, and a use of one is a use of all. The use-count for common names is an over-count; the definition lookup is a first-match.
- **Single-line impl + method is missed** (the line is processed as one declaration); **doc-test code is invisible** (inside comments by construction); a **`#[cfg(test)] mod foo;`** declaration does not propagate test-only-ness into `foo.rs` (the file is scanned on its own path); **trait methods without `pub`** are recorded only when the trait block itself is recognized.
- **The word rule fires on generic words.** Directive keys are whole sentences, so a directive whose key contains "rule" and "matches" matches the generic `Rule::matches` trait method (`04:67` `Concurrent` → BUILT via `Rule::matches` at `opt.rs:119` — the `Concurrent` family itself does not exist in code; the Identifier claim `concurrent` is mentioned in the ROADMAP, not absent, so it was never checked against code). Single-word keys match any same-named item, however unrelated the claim's subject is (`15:211` "A heavy-hitter observer pays ~4–5% of decode" → BUILT via a `decode` fn in a whisper example). TEST-ONLY matches have the same shape in reverse (`04:3` "differentiate-the-decomposition" → TEST-ONLY via a test fn whose name contains the words). These are the lens's over-eager matches, and they sit inside the 236 BUILT / 22 TEST-ONLY counts — the counts are an upper bound on "the docs' absent claims that the code really has", not a verified one.

### What the lens does NOT do

- **It never checks whether code is correct, tested, or wired in. Existence is not integration.** BUILT means a production definition with a production call site — nothing more. The audit's "non-test callers: 0" rows are the proof of that limit: three of the seven validation rows disagree with the audit precisely because a production call site can be dead code (`pick_route` via `resolve`), a dead chain (`capability_gated_rules` via `default_rules`), or a different path than the one the docs describe (`optimize_to_fixpoint` via JIT, not realize). The lens measures production-reachability; the audit measures reachability from the realize path; only the audit's question is the one the docs' claims are actually about.
- **It does not check the code against the docs' semantics.** A name match is not a behavior match: `harvest` in code is `harvest_grads` (gradient harvesting, unrelated to pattern harvesting) and ImageNet labels; the word rule does not know that, and a doc claim about pattern harvesting that matched `harvest_grads` would be a false BUILT. (In this run the pattern-harvest claims came out ABSENT or matched genuinely; the point stands as a limit, not a found error.)
- **It does not read the ROADMAP.** The doc-vs-code check is doc-vs-code only; a claim can be BUILT in code and still untasked in the ROADMAP (that is the common case — 236 of them), and a claim can be tasked in the ROADMAP and ABSENT in code (the directed-but-untasked lens already says the first; the second is the intersection a reader has to compute from the two sections).

### Verdict

The slice works — as an existence + provenance check, and no further than that. The corpus change is exactly one directory; the v8 numbers are provably unchanged; the lens finds 236 of the 764 absent findings built in code (188 of them identifiers), 34 stubs, 22 test-only, and 472 absent, with the validation rows agreeing with the audit on four of seven and disagreeing on three in the same instructive way: the lens's BUILT is production-reachability, the audit's is realize-path reachability, and dead production code is the gap between them. The disagreement is the finding, not a bug to patch over — it names the next lens (a call graph from the realize entry points) more precisely than any of the earlier slices named theirs.

**Handoff assessment (the last planned slice; no decision — Eric decides whether this goes to a lane).** What the spike as a whole has now proven, per lens: (1) pragmatic — descriptions and status notes wearing a directive's or capability's shape can be set aside deterministically (narrow corpus: absent 212 → 105 → 73 across slices 1–2); (2) names — capability names can be extracted by 10 deterministic rules, with the Judge baseline as the known loss; (3) wider corpus — the decisions log drowns the finding list without genre handling (absent 73 → 1,164); (4) genre — a decision-outcome filter plus note-to-row ownership restores it (1,164 → 688, owned 449); (5) contradiction — two documents disagreeing is findable deterministically, but 36 of 37 pairs are noise in six named classes, and the real one is the cancellation case the audit already knew; (6) Rust pack — code as ground truth works for existence and provenance (236/764 absent findings built), but its BUILT is weaker than the audit's STUB/ABSENT, and the three disagreements pin exactly where (dead production code, dead chains, wrong-path reachability). What is still weak: the word rule's over-eager matches (generic words, single-word keys — inside the 236 BUILT count); the contradiction lens's noise classes (dominant: one generic word carrying pairs across senses); no sense disambiguation anywhere; the scanner's gaps (macros, cfg, re-exports, bare-name conflation); no integration check (wiring); and the owned-by-row signal, whose citation-as-ownership case (the nvidia note, row 141) was flagged for revisit in slice 4 and never revisited. What a lane receiving this would still need to do: settle the verdict vocabulary (BUILT vs realize-path reachability) and build the call-graph lens that closes the three disagreements; scope the contradiction lens so the denied capability must be the thing the positive side asserts (or build the sense table for "budget"/"harvest"); replace or bound the hand-rolled scanner (a real parser, or explicit acceptance of its gaps); and build the rewriting tools — every slice so far is analysis only, and the requirements note's insert / replace-a-subsection / side-by-side-verify tools are unbuilt. The spike is a proven set of lenses with named limits, not a tool; the handoff, if Eric wants it, is the lenses plus this report plus the triage tables, with the limits carried as the work list.

## Reproduce

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cd ~/workspace/doc-analysis-spike
cargo test && cargo clippy --all-targets && cargo fmt --check
# v5 (narrow corpus — the three architecture files)
cargo run --release -q -- corpus/fuel/arch-run corpus/fuel/ROADMAP.md aliases.txt
# v6 and v7 run the same command on the same wide corpus — they differ
# by the binary: v6 is the pre-slice-4 binary's output (saved in
# runs/fuel-2026-11-16-v6.txt), v7 the slice-4 binary's
# (runs/fuel-2026-11-16-v7.txt).
cargo run --release -q -- corpus/fuel/arch-wide corpus/fuel/ROADMAP.md aliases.txt
# v8 is the same command with the slice-5 binary, on the wide corpus
# plus doc 15 (runs/fuel-2026-11-16-v8.txt). The aliases.txt argument
# is load-bearing: without it, "multi-path graph" never canonicalizes
# to "multi-path DAG" and one named claim (04:3 "in place into the
# multi-path graph") flips from absent to mentioned.
# v9 is the same command with the slice-6 binary plus the code
# directory as a fifth argument (runs/fuel-2026-11-16-v9.txt). The
# doc-vs-roadmap numbers and the contradiction count are identical
# to v8 — the new lens is additive; the DOC VS CODE section and the
# "doc vs code" summary lines are the only new output.
cargo run --release -q -- corpus/fuel/arch-wide corpus/fuel/ROADMAP.md aliases.txt corpus/fuel/code
```
