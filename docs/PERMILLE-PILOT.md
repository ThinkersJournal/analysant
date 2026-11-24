# Permille-calibration pilot: what `edit_similarity_permille` should be

Status: **pilot, 2026-11-22 (lane analysant), one judge.** Task: pm-ants 2026-11-21. Aggregate counts only; the labelled set (claim text) stays out of git.
Tool: `crates/analysant/examples/permille_pilot.rs` (consecutive git revisions of one file; exact-key matches dropped; each remaining NEW claim gets its best Jaccard similarity, in permille, to a remaining OLD claim). `identity::reidentify` links a reworded claim to its predecessor at or above the threshold the writer passes; the crate has no default.

## Corpus and counts

Three files from our own public repos, read from fixed clones at origin/main (baracuda `README.md` at ce29119, fuel `README.md` at 8a54003, KISS `ROADMAP.md` at 8db36f2); consecutive revisions by `git log --follow`; seed 20261124 (SplitMix64 Fisher-Yates, `analysant::gate_b::shuffle`); 40 revision pairs per file.

| Measure | Value |
|---|---|
| File revisions (baracuda / fuel / KISS) | 203 / 157 / 77 |
| Revision pairs sampled | 120 |
| Claims in old / new revisions | 33527 / 33832 |
| New claims with an exact normal-form key match | 33439 |
| New claims with no exact match | 393 |
| of those, with an unmatched old claim to compare | 243 |
| Best-candidate similarity, 0-99 | 160 |
| Best-candidate similarity, 100 and above | 83 (all labelled) |

Labelling: one judge ("same claim, revised" or "different claim"). All 83 candidates at 100 or above were labelled; 15 of the 160 below 100, drawn at random, were all "different" (added claims paired with an unrelated leftover), so the 160 are counted as different.

## Result

Of the 83: **73 same claim revised, 10 different.**

| Similarity (permille) | 100-199 | 200-299 | 300-399 | 400-499 | 500-599 | 600-699 | 700-799 | 800-899 | 900-1000 |
|---|---|---|---|---|---|---|---|---|---|
| Same | 0 | 2 | 0 | 2 | 3 | 42 | 4 | 15 | 5 |
| Different | 6 | 3 | 1 | 0 | 0 | 0 | 0 | 0 | 0 |

Threshold sweep (lost-link = a revised claim the threshold would not link; wrong-link = a different claim it would link):

| Threshold | 250 | 300 | 350 | 400 | 500 | 600 | 700 |
|---|---|---|---|---|---|---|---|
| Lost-link of 73 | 0 | 2 | 2 | 2 | 4 | 7 | 49 |
| Wrong-link of 10 | 2 | 1 | 0 | 0 | 0 | 0 | 0 |

The curves cross between 250 and 350: the highest "different" pair is 300 and the lowest "same" pair is 250.

## Stated default and failure direction

**Default `edit_similarity_permille` = 400 for this pack and token-set Jaccard**: on this corpus it gives 2 lost-links of 73 and 0 wrong-links of 10.

- **Set too low (about 300 and below):** the failure is a **wrong-link**: short, table-cell-like claims (file paths, identifiers, row labels) sharing one or two tokens are merged into one claim's history.
- **Set too high (600 and above):** the failure is a **lost-link**: a revised claim is minted as a new claim. At 700 it loses 49 of 73, because the counter and version bumps sit at 600 to 700.
- A wrong-link merges two claims' histories silently; a lost-link shows up as a removal plus an addition. So when unsure, err high within 400-600, not low.

## Limits (why this is a pilot)

- **One judge**; no second judge yet (board 469).
- **The "same" set is dominated by two mechanical edit types**: counter or version bumps (38 of 73, similarity 500-700, from one file) and a project-wide rename (17 of 73, 750-916). The other 18 are small edits such as a typo, a path prefix or an added word, so the 250-500 region, where the default matters, rests on a handful of pairs: **not enough data to claim 400 over 350 or 500.**
- The "different" set is 10 pairs, mostly short table cells and paths.
- Three files from three repos; Markdown only.
