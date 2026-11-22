# Gate B round 3: design note (written before any run)

Status: 2026-11-22, lane analysant. Authority: board 491 ruled B by CireSnave ("491: B."), relayed by the PM: a third tuning round with a different design. Docs only; no code and no run exists yet for round 3.

## Why a different design

Rounds 1 and 2 only subtracted: each option dropped one named noise class from a huge population and the next sample showed the next class (0 of 7 true in every sampled kind, runs 0 to 2). Round 3 stops subtracting by class and selects by what a true finding must be. All new options are OFF by default; Gate A (the Fuel runs v4 and v5) must stay unchanged at every commit, and the v4/v5 oracle is not touched.

## Untasked: select open work, not "a claim the plan lacks"

A true untasked finding is a statement that work remains, in a document that is allowed to state work, that the plan does not cover. Measured false classes: standing rules, how-to steps, records of finished work, descriptions of behaviour. New option `[untasked] require_open_work = true` sets aside a claim unless all of:

1. **Where:** its file is a work-bearing document (plan, roadmap, TODO, spec, decision, handoff, issue-like), not a changelog, archive, CONTRIBUTING, CLAUDE.md, instructions file, README, runbook or guide. Decided by path and top heading from a data file (`data/en/work-documents.txt`, `non-work-documents.txt`), not by project.
2. **Form:** the sentence is an unchecked checkbox, or carries an open-work marker (need, must, will, should, TODO, not yet, planned, to do, missing) with an action verb; a checked box, a past or present-tense description, an `always`/`never`/`must not` rule opener and a bare imperative with `run`, `add` or `use` in a guide are set aside. Reuses `rule-openers`, `not-yet-phrases`, `status-markers`, `outcome-markers`, `descriptive-sections`.

Predicted fix: standing rules (rule-openers, form), how-to (where, form), finished-work records (where, outcome-markers, checked boxes), behaviour descriptions (form: no open-work marker).

## Contradiction: require both sides to be claims about one thing

Measured false classes: generic words, bare labels, a denied token absent from the positive side, a cross-word family match. New option `[contradiction] strict = true` requires all of:

1. both sides are full claims (at least 6 words and a finite verb), which removes bare labels and headings;
2. the pair shares at least 2 non-generic words (generic as in round 1) and they match as exact words, so the family-prefix match is off;
3. the denied word appears in the positive side as the same exact word (round 1's `require_denied_token`, tightened to exact);
4. the two sides are in the same section or share a heading word.

Predicted fix: generic words (2, round-1 df), bare labels (1), absent denied token (3), family match (2).

## Round 3 plan, and the risk I can see now

Three public projects not seen in rounds 1 or 2 (not KISS, PersistAnt, Humboldt, Fuel, synapse, vulkane, configurant, httpant, antspawn, webhostant, baracuda, Unpopped), named to pm-ants before any look; one seed fixed in advance and never reused; no-config populations counted first; 7 findings per kind; blind verdicts by this lane and coderipper-c; pass line 80% true of at least 5 judged (CireSnave's ruling). Config fixed in advance: round 1 and 2 options plus the two above.

Risk stated now: `strict` may shrink contradiction to a census below 5 judged (run 2 left 2). A census under 5 cannot pass the rule; I will report it as that, not as a pass, and will not loosen the rule after looking. If round 3 fails, the Stop condition returns to CireSnave.
