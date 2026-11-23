# Gate B round 4: design note (written before any run)

Status: 2026-11-23, lane analysant. PM TASK step 2 (CireSnave 495: B, one more round with a different design). Follows `docs/GATE-B-FAILURE-ANALYSIS.md` (committed e6856bb). Docs only: no round-4 code and no round-4 run exists. Every new option is OFF by default; Gate A (Fuel runs v4/v5) stays unchanged at every commit.

## What changes, and why it is different

Rounds 1-3 filtered by words, kinds and paths. Their survivors (analysis file) all need state outside the sentence. Round 4 gives the checks that state, deterministically, with no model (the no-ML-first requirement, doc-tooling decision 2026-11-17, stands for untasked; for contradiction see the plain statement below).

### Untasked: open work in a live plan, not yet done in the tree

New option `[untasked] live_open_boxes = true` (supersedes `require_open_work`; the two are not combined). A claim is eligible only if ALL of:

1. **Form:** it is an unticked checkbox item. Prose with `must`/`should`/`will` is no longer eligible (a design document uses them for properties: the run-3 "design constraint" row). This drops `open-work-markers` from the rule.
2. **Live document:** the file's first 20 lines carry no marker from a data file `data/en/closed-document-markers.txt` (superseded, obsolete, deprecated, archived, complete, completed, done, shipped, historical), and the path is not in `non-work-documents`. Targets run-3 class "superseded document" (2 of 7).
3. **Not shown done by the tree:** a path, file name or command word the item names that resolves in `git ls-files` at the analysed revision sets the item aside as done (the item names an artifact that exists). Targets "unticked box in a finished plan" (3 of 7, each refuted by an existing file).
4. **Not covered in other words:** an item whose key shares at least half of its tokens (and at least two) with some single plan line is set aside as covered. Targets "plan covers it under other wording" (1 of 7 in run 3, 3 of 7 in run 0). This relaxes the matcher's all-tokens rule; the cost is more claims set aside, which can only shrink the population.

Predicted effect by class (run-3 sample, my verdicts): unticked-in-finished-plan 3 of 3 removed if the judge's evidence (a file exists) is the artifact the item names, and I only know that for the cases where the item names the path; superseded 2 of 2 removed (both heading-marked); design constraint 1 removed (form); plan covers 1 removed if its overlap is at least half. Predicted population: far smaller than 337, possibly a census under 30 or under 5.

Honest risk: (a) any item that names no path and has no marker (a plain unticked box in a live plan with no artifact) is untouched, and I do not know what share of the surviving boxes are really open work; (b) a sampled "true" is a finding a human would add to the plan, and in 28 judged rows there is none, so the true rate is unknown and may be low even among survivors; (c) item 3 fails open: a path that exists but whose work is not done (a file that exists and is half-built) is set aside as done, which loses findings and adds none.

### Contradiction: a minimal pair, or nothing

Failure analysis result: I have no no-ML mechanism with evidence behind it that clears the "sides agree" class. The narrowest rule I can state is the **minimal pair**, option `[contradiction] minimal_pair = true` (replaces `strict`):

- Negated side: `<head words> <negation marker> <verb> <object words>` with a head of at least 2 non-generic words.
- Positive side: contains the same head words, the same verb stem as an exact word, and the same object words as exact words within a 6-word window after the verb, with no negation marker, in a live document (same rule 2 as above).
- Both sentences are single clauses (no `but`, `unless`, `except`, `instead`, `fallback`, `falls back`, `however`).

Predicted effect: removes every one of the 18 judged contradiction rows I have (bare label, generic word, absent denied word, family match, scoped negation, "sides agree": none has the same verb and object with a flipped marker), by construction. Predicted population: a census under 5 on three projects. **Plainly: unless a minimal pair appears in the round's projects, contradiction cannot pass round 4 by the rule (at least 5 judged), and a pass would need something no-ML cannot give (comparing meaning).** If it is a census under 5, I report that, and the question for CireSnave is the one the no-ML decision left open: allow a model for contradiction only, or retire the check. I will not widen the rule after looking.

## Round 4 plan, fixed now

- **Projects** (public at the SHAs below, none seen in rounds 1-3; verified `visibility=PUBLIC` and fetched 2026-11-23): **auth-framework** `0cec44640f3728cfd3c6b512ae3693636b179087`, **wasm-sandbox** `8f37a131383170b9e651b0574dbcc1d7acd08dd3`, **mlmf** `f47401f43c889e243a853fbc7458c0d40dbd2901`. Plan: ROADMAP.md of each. Seen so far and excluded: KISS, PersistAnt, Humboldt, Fuel, synapse, vulkane, configurant, httpant, antspawn, webhostant, sageant, fishstringer, lightbulb, baracuda, Unpopped.
- **Seed: 20261125** (never used; 20261121-24 are spent).
- **Config:** `live_open_boxes = true`, `minimal_pair = true`, plus round 1/2 options (`skip_names`, `generic_df_percent = 20`). Not combined with `require_open_work`/`strict`.
- **Procedure:** no-config populations counted first; then the tuned run; 7 per kind (or the census if under 7); the sample takes at most 3 rows per project, so one repository cannot be the whole sample (run 3 untasked was 334 of 337 lightbulb rows); blind verdicts by this lane and coderipper-c on the same rubric.
- **Success line unchanged:** at least 80% true over at least 5 judged, blind second judge. A kind under 5 judged is reported as a census that cannot pass. One round, then report; failing means Stop, back to CireSnave.
- Lane-days used: 2 of 30 (counting this run: 3).
