# AnalysAnt

Generic text-analysis libraries: one shared claim model, a language-pack seam (a language is data, not code in the engine), and one
library per kind of analysis. Lexical and semantic analysis are the base; pragmatic, epistemic, temporal/provenance, logical,
deontic/modal, discourse/rhetorical, ontological and structural analysis are built on them. English and Rust are the first two
languages, so text can be checked against code.

**Status: design only. Nothing here is usable yet.** The first deliverable is a written spec (the claim model, the language-pack seam
and the two base libraries), published here for review before any code is written.

Planned crates (names reserved on crates.io at 0.0.0): `analysant`, `analysant-core`, `analysant-lexical`, `analysant-semantic`,
`analysant-pragmatic`, `analysant-epistemic`, `analysant-temporal`, `analysant-logical`, `analysant-deontic`,
`analysant-discourse`, `analysant-ontological`, `analysant-structural`.

Licence: MIT OR Apache-2.0.

## Known limits: the contradiction lens (slice 4)

The seed sense table for the reference corpus reproduces the spike's triage; it is **seed-fit, not generalisation**:

- Four "budget" pairs (a resource budget against a byte budget) are removed only by a sense cued on "consumer contract" in the
  section path. Without that cue the lens keeps four pairs the spike's report calls noise.
- Three "lifecycle" pairs are removed only by an `md` cue (the positive sides are links to a document).
- Sixteen pairs remain, and four noise classes are not handled: a denied token absent from the positive side, bare labels,
  scoped negation, and scope notes.

The table is not tuned further on the pairs it already reproduces. Whether it generalises is for Gate B (non-reference projects).
