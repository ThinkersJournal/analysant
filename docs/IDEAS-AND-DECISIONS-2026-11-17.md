# AnalysAnt: ideas and decisions so far (captured 2026-11-17 by the portfolio PM so nothing is lost)

Read `REQUIREMENTS-2026-11-16.md` first: it holds CireSnave's and Pip's words. This note records what was said and decided afterwards,
in the PM session, and the PM's assessment. Anything marked PM is the PM's view, not CireSnave's.

## What CireSnave said (2026-11-17)

1. **Shape.** A set of generic libraries, each one type of analysis, over ANY language. Lexical and semantic are first and are the base
   for pragmatic, epistemic, temporal/provenance, logical, deontic/modal, discourse/rhetorical, ontological and structural.
2. **Pip's spike is not that.** Pip wrote only the portions needed to answer questions about Fuel (the multi-path DAG that was never built
   because of discrepancies between documents and between documentation and code), not the generic suite.
3. **The plan on top of the libraries.** MCP tools an LLM can run to gain a high-level understanding of a project's full documentation,
   including all manner of incompleteness and flaw finding, without spending vast portions of its tokens. Then controlled editing MCP
   tools layered on top, so an LLM specifies what it wants changed and the edit is surgical. That ends append-only documentation, which has
   repeatedly sent lanes (and the PM) to incorrect or outdated passages when a grep for a concept found the stale one.
4. **"It is a massive undertaking... but is it worth it?"** The PM's answer is below; CireSnave then said go ahead and said the go-ahead
   includes writing at least the spec so the ideas are not lost.
5. **Name.** AnalysAnt. Crates named by analysis type: `analysant-<type>`, plus `analysant-core` for pieces shared across libraries.
   (He also floated TextLens, ReaderAnt, TextAnt, AnalAnt; AnalysAnt was chosen.)
6. **Sequencing stays his:** analysis first, tool seam sketched early, rewriting tools LAST.

## PM's assessment (is it worth it?): yes, staged, with a stop condition at each gate

Why it is worth it
- The pain is real and recurring: stale or mis-scoped documents have led the PM to wrong statements (board items 455 and 448 each overstated
  a fact because of an old note), cached claims rot (the PM's own memory index opens with that warning), and the DAG gap came from the same
  cause. Searching a pile of appended notes cannot tell which statement is current.
- LLMs append because replacing a statement safely means knowing every place that fact lives. Git already makes edits recoverable, so that is
  not the obstacle. Finding every claim about X is. That is the analysis, so analysis really is the prerequisite for editing.
- It saves tokens the portfolio is short of: run the analysis slowly in batch, once; many lanes read the health report instead of
  re-reading documents.

What could make it not worth it
1. **Precision.** The spike's own numbers (Pip's, not re-run by the PM): of 764 docs-vs-ROADMAP absences, 472 stayed ABSENT; 36 of 37
   contradiction pairs were noise; BUILT is an upper bound. Measured with a Fuel-shaped tool, not a generic base. The no-ML semantic side
   may hit a ceiling.
2. **False confidence.** A tool that says "no discrepancies" will be believed. Every finding and every absence must carry what was searched
   and a positive control (CodeRipper's absence-claims-carry-the-search pattern).
3. **Data rot.** Lexicons and alias tables go stale the way the documents did; the tool needs a maintenance story.
4. **Scope.** About four lenses prevent the failure class seen so far: structural, pragmatic (a directive is not an assertion),
   temporal/provenance, and the logical cross-document and code check. The other lenses wait until those pay off.
5. **Opportunity cost** against Unpopped and Fuel, which always move first.

## PM's proposed stages with gates (not yet agreed by CireSnave)
0. Interim, cheap: a written "edit in place, supersede explicitly" rule and a CodeRipper check for append-only logs.
1. **Spec, no code:** claim model, language-pack seam, tool seam sketch, base libraries' API (see HANDOFF.md first task). CireSnave reads it.
2. **Base libraries plus structural and pragmatic lenses.** Gate: the generic code reproduces Pip's Fuel results with no Fuel-specific code,
   then CodeRipper's precision method (a shuffled seeded sample of findings judged against a rubric) across at least three other projects.
   CireSnave sets the pass line; below it, stop or re-scope.
3. **Read-only MCP "documentation health report".** Gate: a measured token saving on a real task (for example the PM's planned-vs-built audit,
   done by hand today).
4. **Editing tools** last, per his sequencing.

## What carries over from Pip's spike (reference/pip-spike-2026-11-16/)
- Golden outputs: runs v1-v9, 116 tests, the seven validation rows against the planned-vs-built audit. This is the acceptance oracle.
- Findings: which lens reduced noise (absent 212 to 105 to 73 over its first two slices), the six noise classes of the contradiction lens,
  the vocabulary clash: the lens's BUILT means production-reachable, the audit's STUB/ABSENT means not on the realize path (a call-graph
  lens closes it).
- The claim.rs shape (identity file:line, text, normalized key, section, kind) as a sketch.
- NOT carried over: the code (one crate, Fuel-shaped lexicons and alias table, hand-rolled Rust scanner that cannot see macros, cfg gates or
  re-exports).

## Related portfolio ideas
- A section-addressed read/write MCP for markdown/TOML/JSON (idea list, 2026-11-01) is the editing layer's mechanical half.
- CodeRipper has a delegated-tool framework (tools pinned by name, version and hash): a later consumer of AnalysAnt.
- CodeRipper's MOD-011 reachability asks the compiler's own dead-code analysis and is stronger than a name scan for the Rust pack's
  "is it wired in" question: reuse its approach, do not rebuild it.

## Open decisions (for CireSnave)
- The precision pass line and the kill/continue criterion for stage 2.
- Whether the MCP tools and editing tools are separate projects built on AnalysAnt (the PM suggests yes: one small Ant each).
- Sub-crate set beyond the first twelve names (reserved at 0.0.0 on crates.io: analysant and analysant-{core,lexical,semantic,pragmatic,
  epistemic,temporal,logical,deontic,discourse,ontological,structural}; the last six are being claimed at crates.io's rate limit).
