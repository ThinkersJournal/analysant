# Gate B failure analysis: why the untasked and contradiction checks failed

Status: 2026-11-23, lane analysant. Written on PM TASK step 1 (CireSnave, decision 495: *"495: B.  I'd really like to see those work.  Do we have a solid idea of how and why they failed?"*). Docs only; no design and no run follow from this file until it is committed.

Sources: every judged row of `docs/GATE-B-REPORT.md` (runs 0, 1, 2) and my round-3 verdict files (run 3, not yet in the report: scratch `gb5/v-untasked.tsv`, `gb5/v-contra.tsv`); the producer at `gateb-round1` 5abaafd (`crates/analysant/src/untasked.rs`, `crates/analysant-logical/src/lib.rs`). Run 3 second-judge verdicts have not arrived; run-3 rows below are my verdicts alone.

## What was measured

| Run | untasked judged / true | contradiction judged / true |
|---|---|---|
| 0 no config | 7 / 0 | 7 / 0 |
| 1 tuned | 7 / 0 | 7 / 0 |
| 2 tuned | 7 / 0 | 2 / 0 (census) |
| 3 tuned | 7 / 0 (stopped at 7th false) | 2 / 0 (census) |
| total | 28 / 0 | 18 / 0 |

Zero true in 46 judged rows, across 12 projects in 4 runs. For the sampled runs the 95% upper bound on the true rate is about 35% per run (0 of 7), so each run is enough to say the rate is under the 80% line; what the rows cannot say is the real rate, or whether any true finding sits in the populations. There is no recall measure.

## Untasked

The producer (`untasked.rs`): a claim is "untasked" when no single plan line contains every token of the claim's key (`index.query`, line 148). It then decides which claims are eligible by claim kind and, in round 3, by path fragment and sentence form (`states_open_work`, lines 226-257). Everything it knows is words, kinds and file-path fragments. It never reads whether the stated work is done, whether the document is live, or what the plan line means.

| Class (rows, run) | Producer path and root cause | Evidence |
|---|---|---|
| Bare identifier or label as the claim (3 run 0) | Name-kind claims with a key were eligible: an identifier has a key, so "absent from plan" was true. | KNOW, by intervention: `skip_names` (run 1) removed the class; 0 of 14 rows in runs 1-3 are one. (Different samples, so a reading with a clean absence, not a same-population pre/post.) |
| Spec clause or status note (1 run 0); description of behaviour (4 run 1, 1 run 2) | Claim kind said nothing about whether work remains: a description is a "claim named in the documents". | KNOW (same weak form): `only_kinds` (run 2) and `require_open_work` (run 3) were followed by 1 then 0 description rows. |
| Record of finished work: changelog, archive, checked box (3 run 1, 2 run 2) | Nothing told the lens the file records past work. | GUESS-leaning-KNOW: `non-work-documents` (changelog, archive) in run 3 followed by 0 such rows. |
| Standing rule (2 run 2); how-to in a guide (2 run 2) | Rule and instruction sentences have a key like any other; the lens has no "this is a convention" notion except opener lists. | KNOW (same form): rule-openers + guide paths in run 3, 0 rows. |
| Plan covers it under other wording (3 run 0, 1 run 3) | The match needs ALL key tokens in ONE plan line. Any paraphrase, split across lines or a different word for the same thing is an "absence". | KNOW the mechanism from the code; it was never intervened on (no synonym or partial-match option was tried). Predicted large (it is the matcher's definition), and it appeared in 4 of 28 rows. |
| Design constraint, not a work item (1 run 3) | Path fragment `spec` is in `work-documents`, and a spec sentence contains `must`/`should`, so it passes both gates (`any_in(&text, "open-work-markers")`, line 256). | KNOW (read from lines 231 and 256 and the marker list). |
| Step of a finished plan, box left unticked (3 run 3) | An open checkbox returns true (lines 243-248) whatever the document's state. The work shipped; the box was never ticked. | KNOW the mechanism; whether it shipped I KNOW only from the judge reading the code (file exists, plan header says done). |
| Superseded document (2 run 3) | `V1_ROADMAP.md` contains the fragment `roadmap` and an open box; its own heading says SUPERSEDED, and `states_open_work` never reads headings. | KNOW. |

Pattern: rounds 1-3 removed classes by words, kinds and paths, and each round the next class appeared. Round 3's survivors are the classes that need state outside the sentence: whether the work is done, whether the document is live, whether another plan line says the same thing in other words.

What the check would need to know, that no word, place or form filter can supply:

1. **Whether the work in an unchecked box was already done.** This is a fact about the repository, not the sentence. Part of it is checkable with no ML: a plan step that names a file, symbol or command either exists in the tree at the checked-out revision or not (3 of 7 run-3 rows were refuted by a file the judge found in the tree).
2. **Whether the document is live.** A SUPERSEDED/OBSOLETE/COMPLETE marker in the heading or first lines is lexical, but it is a document-level fact applied to every claim in the file (2 of 7).
3. **Whether the plan covers the claim in other words.** Needs meaning, or at least partial/ranked overlap plus a human-curated alias list; the all-tokens-in-one-line rule has no tolerance.
4. **Whether the claim ever belonged on a plan** (a design constraint, a standing rule). The current signal is wording (`must`, `should`), which design documents use for properties, not tasks.

## Contradiction

The producer (`analysant-logical`): a sentence is "negated" if any marker from the negation lists occurs in it; the pair is a contradiction when a negated sentence and a positive sentence in two files have subjects (bags of normalized content words) where the smaller is family-contained in the larger, and the shared tokens are "strong enough". There is no predicate, no scope of the negation, and no check that the two sentences describe the same entity.

| Class (rows, run) | Producer path and root cause | Evidence |
|---|---|---|
| Bare label as the positive side (3 run 0, 1 run 1, 1 run 2) | Headings, bare IDs and table cells have subject tokens and "positive" polarity by default (no negation marker is the positive polarity). | KNOW, by intervention: `strict` full-claim form (run 3) left 0 of 2 rows of this class; same weak form. |
| One generic word carrying the pair (2 run 0, 4 run 1) | `strong_enough`: one shared token of the minimum length is enough; a project's own name or a word like "compile" in every file qualifies. | KNOW: `generic_df_percent` (run 1) and `min_shared_tokens` (run 2) were followed by 0 rows of this class in runs 2-3. |
| Denied token absent from the positive side (2 run 0, 1 run 1) | The pair shares unrelated words; the negated word itself never occurs in the positive side. | KNOW (same form): `require_denied_token`, then `strict` exact match. 0 rows in runs 2-3. |
| Family match across different words (1 run 1) | `same_family` merges words on a shared prefix of `family_prefix_min = 6` letters ("persistant" vs "persistent"). | KNOW (same form): `strict` exact match; 0 rows after. |
| Scoped negation (1 run 2) | The negation governs a requirement ("run N+1 does not find run N's sentinel"), the positive side describes a test of it. Marker-in-sentence cannot see scope. | KNOW the mechanism; never intervened. |
| Sides agree (2 run 3) | Both sides really are about one fact (complex masks), one phrased with a negation ("needs no complex masks"), the other as a positive ("falls back to manual attention"); the pair mismatches negation of a need with the positive statement of the same fact. Also a SUPERSEDED document. | KNOW: the two rows. These survive every round-3 filter: same subject, exact denied word, full claims, shared heading. |

Pattern: the round-1 to 3 filters made the pair rule stricter about overlap and form and left it blind to what the sentences assert. After `strict` the population is 2, and both survivors are the class the pairing rule cannot exclude: two sentences about one subject that do not conflict.

What the check would need to know:

1. **Whether two sentences are about the same subject and make incompatible statements about it** (the predicate and the scope of the negation). Neither a bag of words nor a prefix rule supplies this. A rule-based parse is possible in principle for narrow templates ("X is/does Y" vs "X is not/does not Y" with the same head noun and verb); it is narrow, and I have no evidence it would find anything.
2. **Which sentences are live** (superseded documents again).
3. **Whether the negation is about the same thing the positive side asserts** (no-need vs fallback).

## Judge limits

- One reader and one rubric for the analysant lane; coderipper-c judged the same rubric blind, so agreement narrows the shared-method caveat and does not remove it (all 21 run 1-2 rows agreed). The rubric asks "is this a real, actionable, correct finding"; a row that is true but trivial is judged false, which matches CireSnave's goal but is a rubric choice.
- 7 per run, stopped at the 7th false positive: the rule can fail a kind quickly and cannot measure its true rate. Stopping at 7 also leaves rows of a kind unseen (e.g. the 330 other run-3 untasked rows).
- The samples are small and clustered by project (3 projects per run, one plan each). Run 3 untasked was 334 of 337 lightbulb rows, so it measured one repository's roadmap habits.
- Interventions were never run on a fixed population, so every "class disappeared" above is a one-armed pre/post on different projects: it supports "this filter is not the reason a class is still there", not "this filter was the only cause".
- Run 3 rows carry my verdicts only until the second judge's file arrives.

## What this says

No-word-filter ceiling: with words, kinds and paths alone the checks have shown a pattern of residual classes that all need state outside the sentence (repo state, document status, meaning). It does not show that nothing no-ML can help: items 1 and 2 under untasked (a named artifact exists in the tree; a SUPERSEDED/complete marker) are deterministic and measurable. For contradiction, I have no no-ML mechanism with evidence behind it; a design must say so.
