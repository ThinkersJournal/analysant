# Documentation Tooling for LLMs: Requirements

Date: 2026-11-16. Status: requirements as stated by Eric (CireSnave); existing-tooling survey complete 2026-11-16; design thread (lenses, sequencing, no-ML stance, English-versus-Rust scope, scale judgment) folded in 2026-11-16.

Provenance: this is the note CireSnave and Pip wrote; the PM copied it here on 2026-11-17 from CireSnave's message. The existing-tooling survey section of the original (ProseMirror, Yjs, Notion, Aider, DITA, SummaC, FActScore, RAGAS, Argdown, Docling, GraphRAG and others; its conclusion was "the gap is integration, not invention") is Pip's and is NOT copied here; ask CireSnave for it if you need it. Nothing in the survey was verified by the PM.

## Context

The Fuel multi-path DAG surprise: a central piece of Fuel's design was never built despite repeated direction, and nobody noticed until Eric tripped over it. The direction lived in appended architecture notes while the ROADMAP, which is what the lane actually works from, never carried it as a task. The surprise is signal: tooling and documentation did not work as intended and need to be fixed.

## The problem (Eric)

Claude and other LLMs use append-only documentation as a workaround: they append rather than edit, to prevent accidentally overwriting or deleting important pieces of information from the documentation. Their hearts are in the right place; they are trying their best to do the right thing. However, an append-only log makes for poor documentation that gets worse over time: every safe edit adds a layer, the stale version keeps winning the grep, and the current truth gets buried under its own history.

## Requirements

All of the following tooling needs to already exist, so LLMs can make documentation changes and analyze documentation with confidence:

1. **Insert without worry of overwriting.** Add to a document knowing nothing already there will be clobbered.
2. **Replace a subsection without risk to the rest.** Swap out one part of a document with no risk to anything outside that part.
3. **Automatically see discrepancies.** Discrepancies in what is stated in a document surface on their own, without having to read or search for them.
4. **Side-by-side verification before accepting.** See the original document and the edited version side by side to verify the edit before accepting it.
5. **Documents as sequences of claims.** View a document as a sequence of claims, statements, questions, etc. that can be stepped through or used with a variety of tools, rather than viewing documents only as a single long string of text.

## [Pip] notes

- The fifth requirement is the key move: a claim has identity; text doesn't. It is the same shape as the cloth's knowns: statements that can be weighed, contradicted, and replaced, instead of a blob that can only be appended to.
- Open questions (not settled): what counts as a "subsection" (the document needs structural addressing: a section, a claim, a paragraph as an addressable unit, for #2 to be mechanical rather than textual); what counts as a "discrepancy" for #3 (direct contradiction, outdated-by-later-statement, superseded claim, or all three); whether #4's side-by-side is a human verification step, a model verification step, or both.
- Scope: this is about the documentation LLMs themselves read and write (ROADMAPs, architecture docs, design notes), not only end-user docs.

## Lenses, beyond lexical and semantic

[Design discussion, 2026-11-16. Eric's question; the lens list is Pip's answer, recorded as design notes, not Eric's words unless marked.]

Lexical and semantic are the base. Requirement #5 (documents as claims) already reaches into the structural lens, and #3 (discrepancies) into the logical lens. The other lenses the analysis should look through:

- **Pragmatic**: what the text is *doing*, not just what it says. A ROADMAP item is a directive ("build this"); a design note is a proposal; a test report is an assertion. Confusing a directive with an assertion is exactly how "we said to build the DAG" becomes "the DAG exists."
- **Epistemic**: how sure the claim is, and on what basis: verified, assumed, flagged-not-assumed, disputed, superseded. A status, not a meaning.
- **Temporal / provenance**: when a claim was made, by whom, on what evidence, and what has superseded it. This is the lens the append-only pile destroys: grep finds the claim but not its timestamp or its successor.
- **Logical**: entailment and contradiction between claims. Requirement #3 lives here.
- **Deontic / modal**: must, should, may, must-not. In specs and ROADMAPs the modal verb is load-bearing: "must support multi-path" is a different claim from "should support multi-path."
- **Discourse / rhetorical**: how claims relate: supports, attacks, elaborates, justifies, concedes (Rhetorical Structure Theory, Toulmin's claim-evidence-warrant). This is what turns a pile of claims into an argument you can walk.
- **Ontological**: what entities the text is about and how they relate: the DAG, the ROADMAP, the lane, the hook. The knowledge-graph view.
- **Structural**: the document as a tree of addressable units, which is part of #5: sections, claims, questions, decisions.

Stylistic, affective, and social lenses exist too (tone, sentiment, audience) but they matter less for documentation than for conversation.

[Pip] The lenses that would have caught the DAG failure are pragmatic, epistemic, temporal, and logical. The direction was a directive whose epistemic status was "requested, never confirmed," made at a time, and logically inconsistent with the ROADMAP's actual task list, and no lens in the current tooling sees any of that. Lexical and semantic see the words; they can't see that a directive was never executed.

Each lens needs tooling an LLM can use to analyze text not just within a single document but across as many documents as it needs to. The DAG failure was cross-document: each document was fine on its own; the contradiction lived between the architecture notes and the ROADMAP.

## Sequencing

[Eric's design, 2026-11-16, confirmed with two refinements.]

1. **Analysis first**: a comprehensive multi-document text analysis library (or set of libraries). One shared claim model, a library per lens, not a monolith. The claim model is part of step one: analysis needs something to operate on besides raw text, and the tools and the rewriting both need to agree on what a claim is: its identity, its relations, its provenance. Without that shared model, the analysis library just re-implements grep with fancier math.
2. **Tool pluggability second**: the lenses exposed as tools an LLM can call. The tool interface should be sketched early, even though it is built second: define the seam first (the FireLine instinct), then build to it, so the analysis is built to be called from the start rather than wrapped later as a monolith.
3. **Rewriting tools third**: the only step that mutates state; it needs the analysis to know what it's changing and the accept/reject loop to verify it. Edit operations themselves can be defined early as part of the model; the rewriting tools come last.

First lens to build: the one that catches the known failure, cross-document discrepancy between the notes and the ROADMAP, not all lenses at once.

Constraints: it doesn't have to be fast, and it must run in environments with limited resources. That rules out "call a frontier model per claim" and points at small local models plus deterministic structure, run as slow batch analysis.

## No-ML stance

[Eric's assumption, confirmed 2026-11-16.]

Most of the analysis can be built without an ML model. Deterministic mechanisms cover the bulk of the value:

- Structure parsing, lexicons (hedges, modals, discourse markers, negation), alias tables for entity identity in a bounded domain, metadata for temporal/provenance, table lookups for the structured logical cases (same item, different status or value).
- ML is the escalation layer for the fuzzy tail, not the foundation: paraphrase-level contradiction, atomic-fact decomposition, implicit discourse relations: the cases the cheap modules can't decide. Same shape as the input heads: deterministic first, ML last.

The DAG case needed no model at all: it was an absence, not a contradiction: directed items minus tasked items is set difference.

[Pip] A second reason for no-ML as the foundation: explainability. When the tool says two claims disagree, a deterministic rule can show exactly why: same item, different status. An NLI score can't. For a tool whose job is to make documentation trustworthy, the findings need to be checkable by the human reading them.

## Scope: English versus Rust

[Eric, 2026-11-16.]

"Languages" here means languages as distant as English versus Rust, natural language versus programming language, so claims in text documents can be transparently checked against the code. The code is the ground truth for whether a documented capability exists. The language-pack seam must therefore cover programming languages too: a Rust parser as the "tokenizer," and signatures, tests, and types as the code's claims.

- The DAG failure is a three-way check: architecture notes vs ROADMAP vs code. Code can't lie about existence.
- Doc-drift tools (living-doc, Moxie Docs) become narrow instances of the general framework rather than separate tools.
- If the seam handles English versus Rust (maximally distant, one ambiguous and one formal) then natural languages are the easy case by comparison. Rust parsing is solved (tree-sitter, syn, rust-analyzer), so the Rust pack is easier to build than a CJK one.
- Matching means: a text claim maps to a code artifact (a function, a test, a type) with a relation: implements, tests, contradicts, or missing. Missing is the DAG case.

[Pip] The natural-language multilingual argument still holds (generic engines, per-language data packs: tokenizer, sentence rules, lexicons, normalizer; language as data, never hardcoded in the engine), but English-versus-Rust is the stronger genericity proof and the more valuable check. The genericity is only proven if the seam is exercised by two languages from the start, one of them structurally distant from English.

## Scale and scope

[Eric's judgment, 2026-11-16.]

The project is massive, and its scale and scope are necessary. The pieces are interdependent: a dependency chain, not feature creep: no discrepancy detection without the claim model, no safe rewriting without discrepancy detection and the verify loop, no docs-versus-code check without the language-pack seam. The failure is a class (claims drift everywhere), not an instance; a narrow tool catches the instance and misses the class. The genericity amortizes across the whole ecosystem (~20 agents, dozens of repos): build once, reuse everywhere. The cost of not building it is already being paid silently in every project.

[Pip] Cautions recorded with the judgment: build in thin slices, each delivering value on its own: first slice: cross-document discrepancy between ROADMAP and architecture notes for one project, deterministic, English first (that alone would have caught the DAG failure). Architecture built for the full scope from day one so scope can grow without rewrites. Lexicons and alias tables are data, and data rots: the tool needs its own maintenance story, or its lexicons go stale the same way the docs did.

## Clarification by CireSnave, 2026-11-17 (the project's actual shape)

The concept was building a set of generic libraries each covering a different type of analysis over any language. The first two libraries were to be lexical and semantic analysis, to serve as a base for building pragmatic, epistemic, temporal/provenance, logical, deontic/modal, discourse/rhetorical, ontological, and structural analysis libraries. Pip took the fact that the need for those forms of analysis came from Fuel's lack of a multipath DAG being caused by discrepancies between documents and between documentation and code, and wrote just the portions needed to answer questions about Fuel rather than the full suite of generic text analysis libraries. (Pip's spike is in `reference/pip-spike-2026-11-16/`.)

Once the analysis libraries exist, they are used to build MCP tools an LLM can run to gain a high-level understanding of a project's full documentation, including all manner of incompleteness and flaw finding, without the LLM spending vast portions of its tokens. Once that works, controlled editing MCP tools are layered on top so an LLM can specify what it wants changed and the tool edits surgically, which ends the append-only documentation habit that sends lanes to incorrect or outdated parts of a project's documentation when they grep.

Naming (CireSnave 2026-11-17): the project is AnalysAnt; crates are named by analysis type with a `-core` crate for pieces shared across libraries.
