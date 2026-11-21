# AnalysAnt: Step 1 specification (claim model, language-pack seam, base libraries, tool seam, acceptance)

Status: **DRAFT for CireSnave's review, 2026-11-17. Design only; no crate code exists.** Author: the analysant lane.
Inputs: `docs/REQUIREMENTS-2026-11-16.md`, `docs/IDEAS-AND-DECISIONS-2026-11-17.md`, and Pip's spike in
`reference/pip-spike-2026-11-16/` (cited below as `spike/<path>:<line>`; every spike claim here was read at that line in this checkout).
Anything I did not check is marked **UNVERIFIED**. Anything I recommend but CireSnave or the PM has not decided is marked **PROPOSAL**.
Section 8 lists what I did not decide.

## 0. Summary

1. A **claim** is a stable-identity statement with a form, a pragmatic kind, a place in a section tree, typed relations, provenance and an
   epistemic status. Identity is minted once and survives edits through a deterministic re-identification step plus a machine-owned
   ledger; position (file:line) is a *locator*, never the identity (section 1).
2. A **language is data plus a small adapter**. Two axes are kept apart: the *document format* (Markdown, TOML, source file) and the
   *language* (English, Rust). The engine owns the model, the matching machinery, the search records and the controls; it hard-codes no
   word list, no script rule, no heading syntax (section 2).
3. `analysant-lexical` answers "which words, and do these words match": tokens, normal forms, lexicon lookup, a term index. `analysant-semantic`
   answers "do these terms mean the same thing here": a concept store with alias and sense tables. Both are deterministic; each stage may
   return **Undecided**, which is the only point where an optional, slow, local model may be consulted (section 3).
4. Every lens is exposed to an LLM through one trait and one report shape. **Every finding carries what was searched and a positive
   control**; an absence says what it looked in (section 4).
5. Acceptance is three gates: the spike as golden oracle, a seeded precision sample across at least three other projects, and a
   kill/continue rule for CireSnave to set (section 5). The smallest first slice is `analysant-core` plus `analysant-lexical`
   reproducing the spike's narrow-corpus run v3 (section 6).

## 1. The claim model (`analysant-core`)

### 1.1 What the spike did, and what is wrong with it as a base

`spike/src/claim.rs:42-57` is a six-field struct: `id` (`file:line`), `text`, `key`, `source_file`, `line`, `section` (a string joined with
` > `, `spike/src/parse.rs:61-65`), `kind` and `role`. Three things it got right are kept: language-independence of the model
(`claim.rs:1-7`), a normalized key separate from the raw text, and the section carried on the claim. Four things it could not carry are
fixed here:

- **Identity is position** (`claim.rs:44-45`, "file:line of first occurrence"). Insert a line above a claim and its identity changes, so
  nothing can be superseded, annotated or tracked across an edit. Requirement 5 says "a claim has identity; text doesn't".
- **`kind` and `role` are two enums for one question.** `ClaimKind::Directive` is assigned from the *shape* (a list item,
  `spike/src/parse.rs:70-82`) and `PragmaticRole::Directive` from the *text* (`claim.rs:28-40`); a list item can be a Directive by kind and a
  Description by role. Here **form** (how it appears) and **kind** (what it does) are separate fields with separate owners.
- **A section is a string.** Requirement 2 needs a section to be an addressable unit, so a section is a node with its own identity.
- **No relations, provenance or status.** The spike pairs statements ad hoc (`spike/src/contradiction.rs:110-139`) and never stores a
  relation.

### 1.2 The unit tree

A corpus is a set of **documents**; a document is a tree of **units**. Units are what edits address (requirement 2) and what claims sit in.

```rust
/// Identifies a unit across revisions. Opaque: 128 bits rendered as base32. Minted once, never reused.
pub struct UnitId(/* private */);

pub enum UnitKind { Document, Section, Block, Claim }

pub struct Unit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub parent: Option<UnitId>,
    pub ordinal: u32,               // position among siblings; a locator, not identity
    pub heading: Option<String>,    // for Section
    pub span: Span,                 // byte range + line range in the source revision
    pub source: SourceRef,          // see 1.6
}
```

`Claim` is a `Unit` with a `ClaimBody`; a section keeps the claims below it as children, so "replace a subsection" (requirement 2) is
"replace the children of one `Section` unit", which is mechanical and cannot reach a sibling.

### 1.3 The claim

```rust
pub struct Claim {
    pub unit: Unit,                 // identity, parent section, span, source
    pub revision: Revision,         // 1.4: which revision of this claim's text this is
    pub text: String,               // as written
    pub form: Form,                 // how it appears (structural; set by the format adapter / language pack)
    pub kind: Kind,                 // what it does (pragmatic; Unclassified until the pragmatic lens runs)
    pub normal: NormalForm,         // from analysant-lexical (tokens, key); never part of identity
    pub status: EpistemicStatus,    // Unassessed until the epistemic lens runs
    pub provenance: Provenance,     // 1.6
}

#[non_exhaustive]
pub enum Form {
    Sentence, ListItem, TableRow, TableCell, Heading, Question,
    CodeItem(CodeItemKind),         // the Rust pack: fn, struct, enum, trait, test, const, ...
    Other(String),
}

#[non_exhaustive]
pub enum Kind {                     // speech acts; the pragmatic lens assigns, the base libraries never do
    Unclassified,
    Directive,                      // "build the multi-path DAG"
    Assertion,                      // "the plan IS the graph"
    Question,
    Decision,                       // includes the outcome: adopted / rejected / declined
    Definition,
    Proposal,
    Report,                         // as-built, dated, retired (the spike's StatusNote, spike/src/claim.rs:36-37)
    Name,                           // a naming act, not a sentence (spike's Name, claim.rs:38-39)
    Description,
}

#[non_exhaustive]
pub enum EpistemicStatus {          // a status, not a meaning; the epistemic lens assigns
    Unassessed,
    Verified { basis: Evidence },
    Asserted,
    Requested,                      // a directive nobody confirmed: the DAG failure's status
    Assumed,
    Disputed { by: Vec<ClaimRef> },
    Superseded { by: ClaimRef },
}
```

The enums are `#[non_exhaustive]` because later lenses will add values; core ships the *vocabulary*, the lens libraries own the
*assignment*. Core never classifies a claim: a claim extracted by core alone has `kind: Unclassified` and `status: Unassessed`.

### 1.4 Identity that survives edits (PROPOSAL)

The mechanism, in the order it is tried. Everything is deterministic and explains itself (the same standard as the spike's "the rule that
fired is the reason", `spike/src/pragmatic.rs:9-12`).

1. **Mint.** The first time a claim is seen it gets a fresh `UnitId`, recorded in the **ledger** (1.5) with its document, section path,
   normalized text hash and revision 1.
2. **Re-identify on every re-extraction.** The new extraction is aligned to the ledger's last state for that document:
   a. *Exact*: same normalized text under the same section lineage gives the same id (a moved paragraph keeps its identity);
   b. *Edited in place*: same section lineage, same ordinal neighbourhood and token-set similarity at or above a pack-configured threshold
      gives the same id and `revision + 1`, with the old text kept in the ledger;
   c. *Otherwise* a new id; an old id with no match is marked `Gone`, never silently dropped.
   Ties are not resolved by guessing: an ambiguous alignment is reported as a finding (`identity:ambiguous`) and both candidates keep
   their old ids until a human or an explicit edit operation decides.
3. **Edits by the tools are exact.** A tool edit (1.8) names the claim id and the revision it expects; the ledger records old and new text
   at the moment of the edit, so tool-made edits never need step 2.

**Revision versus supersession.** A *revision* is the same proposition, reworded: same id, `revision + 1`. A *supersession* is a different
proposition that replaces one: new id, and a `Supersedes` relation (1.5). A deterministic rule cannot tell these apart in general, so the
line is drawn by who asks: an edit operation says which it is (1.8); a lens that infers supersession only *proposes* it (a finding), and the
relation becomes fact when accepted.

### 1.5 Relations and "supersedes"

```rust
pub struct Relation {
    pub from: ClaimRef, pub to: ClaimRef,
    pub kind: RelationKind,
    pub by: Derivation,             // which rule or edit created it (never empty)
    pub state: RelationState,       // Asserted (by the document or an edit) | Proposed (by a lens) | Accepted | Rejected
}

#[non_exhaustive]
pub enum RelationKind {
    Supersedes,                     // from replaces to
    Contradicts,                    // logical lens
    Implements, Tests, Missing,     // doc claim -> code claim; Missing is the DAG case
    Elaborates, Supports, Attacks, Justifies, Concedes,   // discourse lens
    Owns,                           // a plan row owns a note (spike/src/ownership.rs:1-28)
    Cites, Duplicates, Refines,
    Other(String),                  // namespaced "lens:name" for lens-private relations
}
```

**`Supersedes(B -> A)` means:** A is retained, readable and addressable, but is **not current**. Concretely: (a) default queries and
discrepancy checks run over the *current view* (claims with no accepted `Supersedes` pointing at them), so a stale version stops winning
the search, which is the failure in the problem statement; (b) A's `status` becomes `Superseded { by: B }`; (c) supersession is transitive
through chains and a cycle is an error; (d) the old text is never deleted from the ledger, so "recoverable" holds without the document
having to carry its own history, which is what ends the append-only habit. A document that says "superseded by X" in prose
yields an `Asserted` relation only when the pack's lexicon and the reference resolve to a claim; otherwise it is a `Proposed` relation.

### 1.6 Provenance

```rust
pub struct SourceRef { pub repo: Option<String>, pub path: String, pub rev: Option<String> /* commit sha */, pub span: Span }

pub struct Provenance {
    pub source: SourceRef,
    pub extracted_by: ExtractorId,  // format adapter + language pack, each with name and version
    pub extracted_at: Timestamp,
    pub asserted_at: Option<Timestamp>,   // when the document says it, if derivable (a date in the text; git blame)
    pub author: Option<String>,           // only when derivable; never guessed
}

pub struct Derivation { pub lens: LensId, pub rule: String /* the rule that fired */, pub table_entries: Vec<TableEntryRef> }
```

`asserted_at` is the temporal lens's raw material. Core only carries it. The spike detects dates with a hand-written `YYYY-MM-DD`
window (`spike/src/pragmatic.rs:182-196`); here the date grammar is language-pack data.

### 1.7 Addressing

Two forms, never confused:

- **Stable reference** `ClaimRef { id: UnitId, rev: Option<Revision> }`, textual `an:<id>[@<rev>]`. This is what relations, findings and edit
  operations hold. With `rev` it names an exact text; without it, "this claim, whichever revision is current".
- **Locator** `Locator { doc, section_path: Vec<SectionRef>, ordinal, span }`, textual `<doc>#<slug>/<slug>/¶<n>`. For people and for
  diffs. It breaks when the document is restructured, by design; a stable reference does not.

A **section is addressable as a unit** (`an:<section-id>`), so the edit "replace the content of section S" carries S's id and a
precondition (1.8), not a line range.

### 1.8 Edit operations (defined now, built last)

Requirement note: "Edit operations themselves can be defined early as part of the model; the rewriting tools come last." Only the types:

```rust
pub enum EditOp {
    InsertAfter   { anchor: ClaimRef, text: String },                    // cannot clobber: adds a unit, touches nothing else
    ReplaceClaim  { target: ClaimRef, expect_rev: Revision, text: String, mode: ReviseOrSupersede },
    ReplaceSection{ target: UnitId,   expect_digest: Digest, children: Vec<NewClaim> },
    Retire        { target: ClaimRef, expect_rev: Revision, reason: String },   // marks Gone/Superseded; text stays in the ledger
}
pub struct EditPlan { pub ops: Vec<EditOp>, pub base: SourceRef /* the revision the plan was made against */ }
```

Every mutating op carries an **expectation** (`expect_rev`, `expect_digest`): if the target changed since the plan was made, the op is
refused, not merged. Requirement 4 (side by side) is `render_diff(EditPlan) -> SideBySide` built from the plan and the ledger; whether a
human or a model accepts it is a policy outside the library. No op is ever applied by an analysis crate.

### 1.9 The ledger

Identity needs memory. The **ledger** is a file in the analysed project (PROPOSAL: `.analysant/ledger.jsonl`), owned and written only by
AnalysAnt tools, one JSON record per unit event (minted, revised, moved, gone, relation changed). It is the one place where append-only is
correct, because a machine writes it and nothing reads it by grep. Whether it is committed to the project, and how two branches merge it,
is open (section 8, D3).

**Ledger writer: EditAnt (pm-ants 2026-11-20, from D5; board 465).** The writing tool is the editing tool, so the ledger *writer* is not built in AnalysAnt: AnalysAnt reads the file and union-merges (`ledger_file`), and re-identifies (`identity`). Required interface of the writer: it writes `.analysant/ledger.jsonl` events in the `ledger_file` format; unit ids come from `analysant_core::content_id`, the one id scheme for every unit kind (a function of unit kind, document, section path, text, occurrence and generation, and nothing else). Exactly: `text` is the unit text as extracted, with no further normalisation; `occurrence` is the index of the unit among units of identical (kind, document, section path, text) minted in the same pass over that one document (scope: one document, never the project); `generation` is the number of units minted with that same content (the text at minting) in the ledger file's state for that document, live, edited since or gone, and is 0 with no ledger file. So a writer that mints must read the ledger first for `generation`, and must call `content_id`, never reimplement it; the first line of a non-empty file is the format header `{"type":"format","name":"analysant-ledger","version":1}` (`ledger_file::LEDGER_FORMAT_VERSION`), written first and never among the events, and a reader refuses a file without it and a version it does not read, naming both (`UnsupportedVersion`), and `merge_text` refuses to merge files of different versions (`VersionMismatch`); the write is atomic (temp file, then rename); it never deletes old text and appends `minted`, `revised`, `moved` and `gone` events; it refuses an edit whose expected revision does not match the ledger's. Nothing is needed from EditAnt until its D7 path (`.analysant/`) settles. **Where the code lives (2026-11-21, kit gaps G2/G5/G6):** `corpus` (extract_document), `identity` (reidentify) and `ledger_file` are in `analysant-core`, re-exported by `analysant`, so a core-only publish is enough for a writer; the full rules, the two golden files, the reference writer and the independent checker are `docs/LEDGER-WRITER-CONFORMANCE.md`. A write is a whole-file union rewrite under `.analysant/ledger.lock`, not a byte append.

## 2. The language-pack seam

### 2.1 Two axes, not one

The spike's `parse.rs` is a Markdown parser with English assumptions inside it: a list of English conjunctions and prepositions
(`spike/src/parse.rs:351-353`), `", "`/`"; "`/`". "` as sentence punctuation (`parse.rs:338`), a stopword list (`spike/src/normalize.rs:49-52`).
Markdown is a *document format*; English is a *language*; neither belongs to the engine. So:

| axis | trait | supplies | examples |
|---|---|---|---|
| document format | `FormatAdapter` | the unit tree (documents, sections, blocks) and the text/code spans inside it, with the language each span is in | Markdown, plain text, TOML comments; a `.rs` file |
| language | `LanguagePack` | everything about reading the text of a span | English; Rust |

A fenced code block in a Markdown document is handed to the pack for its language (the spike skips fences and says why:
`spike/src/parse.rs:9`, "code is a different language and gets its own parser"). A Rust source file is a "document" whose format adapter is
the Rust parser and whose spans are Rust.

### 2.2 What a pack supplies

```rust
pub trait LanguagePack: Send + Sync {
    fn id(&self) -> PackId;                         // "en", "rust"; carries a semantic version
    fn script_kind(&self) -> LanguageKind;          // Natural | Formal
    fn tokenize(&self, span: &Span, text: &str) -> Vec<Token>;
    fn sentences(&self, text: &str) -> Vec<Range<usize>>;          // natural: rules are DATA; formal: items, not sentences
    fn normalizer(&self) -> &dyn Normalizer;        // case folding, lemma/stem table, identifier splitting, stopword set
    fn lexicons(&self) -> &LexiconSet;              // data (2.3)
    fn extract(&self, unit: &Unit, text: &str) -> Vec<RawClaim>;   // form + text + span; NEVER sets Kind or status
}

/// Formal languages additionally expose artifacts and references, so doc claims can be checked against them.
pub trait FormalPack: LanguagePack {
    fn artifacts(&self, src: &SourceFile) -> Vec<Artifact>;        // fn, type, trait, test, const; qualified name, signature, visibility, cfg
    fn references(&self, src: &SourceFile) -> Vec<Reference>;      // calls, uses, re-exports: the edges a call-graph lens needs
}
```

The `en` pack's `extract` implements the *structural* shapes only (a list item, a table row's first cell, a sentence); deciding that a
list item is a directive is the pragmatic lens (later). The spike did both in one function (`spike/src/parse.rs:70-82`).

### 2.3 What is data

All of these live in versioned data files with the schema in 3.5, never in Rust source:

- tokenizer parameters (word-boundary classes, script rules, abbreviation list for sentence splitting);
- stopwords, imperative verbs (`spike/src/pragmatic.rs:202-`), status markers (`pragmatic.rs:74-90`), outcome markers (`pragmatic.rs:109`),
  negation phrases (`spike/src/contradiction.rs:374-`), temporal phrases (`contradiction.rs:346-`), "bad words" that make a negation
  positive (`contradiction.rs:262-`), modal verbs, hedges, discourse markers, date grammar;
- normalizer tables (lemma/stem, case-folding exceptions);
- alias tables and sense tables (3.4).

Why data: Eric's constraint is that a language is a pack and the engine is generic (requirements, "Scope: English versus Rust"); and data can
carry provenance, examples and a maintenance record, which a `const &[&str]` in source cannot (the spike's own comment at
`spike/src/pragmatic.rs:11-13` says it "rots like any data" and offers no remedy).

### 2.4 What the engine owns (and packs may not)

The unit tree, claim, relation, ledger and finding types; the matching *modes* (3.2); the term index; orchestration across documents;
search records and controls (4.2); scoring, sampling and reporting. A pack plugs in *behaviour that varies by language*; it never owns
the shape of a result.

### 2.5 What may NOT be hard-coded in an engine crate (mechanically checked)

1. A word list of any language (including `"the" | "a" | "an"`, `spike/src/normalize.rs:20`).
2. A character-class assumption about words (`is_alphanumeric` as the token rule, `normalize.rs:13`, is English-shaped; CJK has no spaces).
3. Case folding by `to_lowercase()` (`normalize.rs:10`) without a pack hook (Turkish dotless i).
4. A heading, list or table syntax: those belong to a `FormatAdapter`.
5. A project's vocabulary: `"fuel"` is in the spike's pairing stopwords (`spike/src/contradiction.rs:636`), a ROADMAP row is `N. **`
   (`spike/src/ownership.rs:18-19`, `101-115`), and a note cites a row as `design-notes/<file>` (`ownership.rs:58`). All are *configuration*
   of a project (`.analysant/project.toml`: plan-row pattern, note-citation pattern, project stopwords), not engine code.
6. A verdict vocabulary: BUILT/STUB/ABSENT is Fuel's audit's vocabulary mapped onto the spike's lens (`spike/src/rust_code.rs:39-49`); the
   engine's vocabulary for doc-vs-code is `Relation { Implements, Tests, Missing }` with evidence, and reports map it (see 5.1, "verdict
   vocabulary").

**Mechanical check (acceptance, 5.1):** a test greps every `src/` file of every engine crate for a deny-list (`fuel`, project names, the words in
2.3's lexicon files) and fails on any hit. Its positive control is the same grep over the test fixtures, which must hit.

### 2.6 How English and Rust both fit

| seam point | English pack | Rust pack |
|---|---|---|
| format adapter | Markdown | the `.rs` file; Markdown doc comments are handed back to English |
| tokenizer | word/number/identifier-in-prose tokens from pack data | lexer tokens |
| sentence rules | abbreviation-aware, from data | none: an item is the unit |
| normalizer | lowercase, lemma table, drop leading article | `split_ident_words`: `pick_route` -> [pick, route], `RuleRegistry` -> [rule, registry] (the spike's rule, `spike/src/rust_code.rs:145-149`, reused as an idea) |
| claims | sentences, list items, table rows, headings, questions | items: signature, visibility, type, `#[test]` fns, cfg conditions, doc comment |
| lexicons | hedges, modals, negation, status, discourse | `test`, `deprecated`, `unsafe`, visibility words, `cfg` predicates as qualifiers |
| ground-truth role | the claims being checked | the artifacts checked against (code cannot lie about existence) |

**Rust pack implementation.** The spike's hand-rolled scanner is not reused (it names its own gaps: no parser, no call graph, no macro
expansion, no cfg evaluation beyond `cfg(test)`, bare-name conflation, `spike/src/rust_code.rs:8-37`; sizes in
`spike-report-2026-11-16.md:475-480`). PROPOSAL: `syn` for items and signatures, as CodeRipper's detector does
(`C:\Projects\coderipper\docs\superpowers\plans\2026-10-24-performance-rules-precision.md:10`, a throwaway `syn` visitor); reachability
("is it wired in") asks the compiler instead of a name scan, reusing CodeRipper's MOD-011 approach (named in
`docs/IDEAS-AND-DECISIONS-2026-11-17.md:66-67`; **I have not read MOD-011**, UNVERIFIED). The Rust pack is **not** in the first slice
(section 6); the seam is specified now so the first slice does not paint it out.

## 3. The two base libraries

### 3.1 Division of labour

| | `analysant-lexical` | `analysant-semantic` |
|---|---|---|
| question | what are the words, and do these words match | do these terms mean the same here |
| owns | tokens, normal forms, lexicon lookup, matching modes, the term index | the concept store, alias table, sense table, term relations |
| unit of data | `Token`, `NormalForm`, `LexiconHit` | `Concept`, `Sense`, `Resolution` |
| knows documents? | no: text in, hits out | the corpus's scope layering (project, document) |
| ML | none; one declared escalation hook for unsegmented scripts (3.3) | the escalation point for paraphrase (3.3) |

Both depend only on `analysant-core` and each other in that direction (`semantic` depends on `lexical`).

**Field libraries (PM notice, CireSnave 2026-11-18, recorded 2026-11-19).** EpistemologicAnt, OntologicAnt and SemanticAnt are separate projects: comprehensive libraries for their fields, which this project's lenses *use* (they are not the lenses). CireSnave's standing rule, quoted by the PM: *"If something needs a tool that falls in one of those fields, we should build that tool in those projects and use it as a dependency for the project that needs it."* The seam: when `analysant-epistemic`, `analysant-ontological`, `analysant-semantic` or the semantic half of `analysant-lexical` needs a tool in epistemology, ontology or semantics, it is **not grown here**. The lane writes the requirement (inputs, outputs, and the spike evidence that shows why) and sends it to the PM for the matching lane; the result arrives as a crates.io dependency. Slice 1 (core and the lexical half that is only words) is unaffected. Until a field library exists, `analysant-semantic`'s concept store is specified here as the *consumer's* shape and may be replaced by a thin adapter over SemanticAnt.

### 3.2 `analysant-lexical`: public API sketch (types and signatures, no bodies)

```rust
pub struct Token { pub text: String, pub span: Range<usize>, pub norm: String, pub class: TokenClass }
pub enum TokenClass { Word, Number, Identifier, Punct, Date, Other }

pub struct NormalForm { pub tokens: Vec<Token>, pub key: String, pub content: Vec<String> /* stopwords removed */ }

pub fn tokenize(pack: &dyn LanguagePack, text: &str) -> Vec<Token>;
pub fn normalize(pack: &dyn LanguagePack, text: &str) -> NormalForm;

/// Matching modes the spike used, named so a finding can say which one fired.
pub enum MatchMode {
    Sequence,       // contiguous token run         (spike/src/normalize.rs:62-74)
    TokenSubset,    // every key token present, any order (normalize.rs:31-44; the spike's directed-but-untasked check, check.rs:48-51)
    Family,         // morphological family: shared long prefix (spike/src/contradiction.rs:422)
}
pub struct MatchEvidence { pub mode: MatchMode, pub matched: Vec<Range<usize>>, pub rule: &'static str }
pub fn matches(hay: &NormalForm, needle: &NormalForm, mode: MatchMode) -> Option<MatchEvidence>;

/// Word-boundary phrase lookup against a lexicon (never substring: "shall" must not hit "shallow", spike/src/pragmatic.rs:158-162).
pub struct LexiconHit { pub lexicon: LexiconId, pub entry: EntryId, pub span: Range<usize> }
pub fn lookup(set: &LexiconSet, lexicon: &str, text: &NormalForm) -> Vec<LexiconHit>;

/// Cross-document index: normalized token -> claim ids. Replaces the spike's O(claims x lines) scan (spike/src/check.rs:48-51).
pub struct TermIndex { /* private */ }
impl TermIndex {
    pub fn build(claims: &[Claim]) -> Self;
    pub fn query(&self, key: &NormalForm, mode: MatchMode) -> QueryResult;   // carries the SearchRecord (4.2)
}
```

`Family` carries the spike's own stated limits (roots under 6 letters with a consonant mutation do not meet,
`spike/src/contradiction.rs:73-79`); the limit is part of the mode's documentation, not hidden.

### 3.3 Deterministic parts and the ML escalation point

Every decision in both crates returns a three-valued result:

```rust
pub enum Decision<T> { Yes(T, Evidence), No(Evidence), Undecided(Reason) }
```

- **Deterministic by contract:** tokenizing (for spaced scripts), normalizing, lexicon lookup, the three match modes, the term index, alias
  canonicalization, sense selection from cues, term relations from tables.
- **The escalation point is exactly `Undecided`.** A deterministic stage that cannot decide says why (no sense matched; two senses tie;
  no alias group; token overlap below threshold). An optional `Escalator` may then be consulted:

  ```rust
  pub trait Escalator { fn decide(&self, q: &Question) -> Decision<Verdict>; fn identity(&self) -> ModelId; }
  ```

  Rules for it: **off by default**; slow and batch (the requirements' constraint); local and small; the call, its prompt/input hash and the
  model id are recorded in the finding's `Derivation`; a finding whose truth rests on an escalated verdict is **marked as model-judged**
  and is never counted in the deterministic precision figure (5.2). Candidates the spike's own report names for the fuzzy tail:
  paraphrase-level contradiction, atomic-fact decomposition, implicit discourse relations (`docs/REQUIREMENTS-2026-11-16.md:71`).
  Unsegmented scripts (CJK) need a data-driven segmenter; that is the one lexical escalation point and a Natural-pack concern.
- Which model, and its resource ceiling, is **not decided** (section 8, D6).

### 3.4 `analysant-semantic`: public API sketch

```rust
pub struct Concept { pub id: ConceptId, pub canonical: String, pub forms: Vec<Form>, pub senses: Vec<Sense>, pub scope: Scope, pub meta: EntryMeta }
pub struct Sense  { pub id: SenseId, pub gloss: String, pub cues: Vec<Cue>, pub excludes: Vec<Cue> }   // cue: co-occurring term, section pattern, document glob

pub struct ConceptStore { /* layered: language pack < portfolio < project < document */ }
impl ConceptStore {
    pub fn load(layers: &[LayerSource]) -> Result<Self, TableError>;
    pub fn resolve(&self, term: &NormalForm, ctx: &Context) -> Resolution;     // Concept(+sense) | Ambiguous(Vec<Sense>) | Unknown
    pub fn canonicalize(&self, text: &NormalForm, ctx: &Context) -> NormalForm; // generalizes the spike's canonicalize (spike/src/normalize.rs:111-143)
}

pub enum TermRelation { Same, Broader, Narrower, Related, Unrelated }
pub fn relate(store: &ConceptStore, a: &NormalForm, b: &NormalForm, ctx: &Context) -> Decision<TermRelation>;

/// What a claim is about: the head term(s). Generalizes the spike's subject_tokens (spike/src/contradiction.rs:700).
pub fn subject_of(store: &ConceptStore, claim: &Claim, ctx: &Context) -> Decision<Subject>;
```

`semantic` is what stops the spike's dominant failure. Of the 37 contradiction pairs, 36 were noise, 21 of them from one generic word
carrying pairs across unrelated senses ("budget" 11, "harvest" 4, "lifecycle" 3, "written" 2, "network" 1;
`spike-report-2026-11-16.md:334-340`). The spike's report calls the fix "a sense table for the overloaded words" (`:398`); that is
`Concept.senses` here, and the 21 cases are its first seed and its first negative tests.

### 3.5 How an alias table and a sense table are stored, and kept from rotting

**Today's form** is one line per group, `a | b | c`, first canonical (`spike/aliases.txt:1-6`; parser `spike/src/normalize.rs:76-109`). It has one
entry, and that entry is load-bearing: omitting the file flips claim `04:3` from absent to mentioned
(`spike/HANDOFF.md:49-51`). It has no scope, no sense, no provenance and no test, so nothing can tell an entry that is right from one that
rotted.

**Proposed form** (PROPOSAL): TOML, diffable and commentable, layered by scope, one file per layer (`*.concepts.toml`):

```toml
[[concept]]
id        = "multi-path-dag"
canonical = "multi-path DAG"
forms     = ["multi-path graph"]
scope     = { project = "fuel" }          # applies in this project's corpus only
status    = "active"                       # active | proposed | retired
added     = "2026-11-16"
by        = "pip"                          # who: a role or a person, never a guess
evidence  = ["fuel:docs/architecture/03-ir.md#...", "fuel:ROADMAP.md#157"]   # where the two names were seen co-referring
last_confirmed = "2026-11-17"

[[concept.tests]]                          # executable: CI runs these
same = ["multi-path graph", "multi-path DAG"]
not_same = ["path graph"]

[[concept]]
id = "budget"
[[concept.sense]]
id = "budget-tolerance"; gloss = "numerical error tolerance budget"; cues = ["tolerance", "epsilon", "rounding"]
[[concept.sense]]
id = "budget-resource";  gloss = "per-device resource budget";         cues = ["vram", "byte", "device", "memory"]
```

**The maintenance story** (the requirements ask for one; this is the design, and each point is a check that can fail a build):

1. **Every entry carries evidence and a `last_confirmed` date**; `analysant doctor tables` lists entries whose evidence claims are `Gone` or
   `Superseded` (stale), entries none of whose forms occur anywhere in the current corpus (unused, candidates to retire), and entries older
   than a configured age without confirmation.
2. **Tables are tested like code.** Each entry's `tests` run in CI. A change to any table runs the golden corpus (5.1) and prints the diff of
   finding-id sets; a table edit that flips a golden verdict must say so in its change fragment (this is the spike's method: its v8-to-v9
   check diffed the run files and found them byte-identical, `spike-report-2026-11-16.md:453-455`, and its own near-miss was an alias file
   left out of one run, `:370`).
3. **Tools propose, humans promote.** A miner may emit `status = "proposed"` entries (two forms in the same slot of aligned claims; a
   cluster of dismissed findings sharing a token). A proposed entry affects no finding until a human or the PM sets it `active`. Nothing is
   auto-applied, because a wrong alias silently *removes* findings (it merges two things), which is the dangerous direction.
4. **Every finding records the table entries it used** (`Derivation.table_entries`), so retiring or editing an entry yields the list of
   findings to re-run, and a report can say "this absence relied on alias X".
5. **Ownership follows scope.** The `en` pack's tables live in the AnalysAnt repo under CODEOWNERS; a project's table lives in that
   project's repo and is maintained by that project's lane, next to the documents it describes; the table's content hash is printed in
   every report header so a report is reproducible.
6. **Rot is measured, not hoped about.** Per table: unused ratio, median age since confirmation, and the dismissal rate of findings that
   used it. A table whose dismissal rate is above the precision pass line (5.2) is flagged.

## 4. The tool seam (sketch)

The tool interface is defined now and built second (requirements, "Sequencing"): the analysis is built to be called, not wrapped later.

### 4.1 A lens is a function with a declared shape

```rust
pub trait Lens {
    fn spec(&self) -> LensSpec;                    // name, description, input_schema, output_schema, cost class, side_effects = None
    fn run(&self, corpus: &Corpus, params: &Params, ctx: &mut RunContext) -> Report;
}
pub struct LensSpec { pub id: LensId, pub requires: Vec<LensId>, pub cost: Cost /* Cheap | Batch */, pub reads_code: bool /* ... */ }
```

`LensSpec` is plain data (JSON Schema for input and output, derived with `schemars`, PROPOSAL), so a later `analysant-mcp` can publish one MCP
tool per lens without a line of lens-specific glue. `analysant-core` takes **no MCP dependency**; the PM suggests the MCP and editing
tools are separate projects built on AnalysAnt (`docs/IDEAS-AND-DECISIONS-2026-11-17.md:70-71`); that is CireSnave's call (D5).

Two tool shapes serve the stated goal of "high-level understanding without spending vast portions of the tokens":

- `health_report(project) -> Report` : counts per lens and finding kind, the top-k findings by severity, **coverage** (what was and was not
  analysed), and finding ids. Short by construction (a size budget is a parameter).
- `explain(finding_id) -> Explanation` : the claims involved (with text and locator), the derivation (rules, table entries) and the search
  record. Findings are expanded on demand, not dumped.

### 4.2 A finding carries what was searched and a control

This is the portfolio's absence-claim rule (`C:\Projects\CLAUDE.md` section 6: state the control beside the null) made a type, so it cannot be omitted:

```rust
pub struct Finding {
    pub id: FindingId,                    // stable: hash of (lens, kind, sorted claim ids, key); survives re-runs
    pub lens: LensId, pub kind: FindingKind,
    pub claims: Vec<ClaimRef>,
    pub statement: String,                // one sentence a reader can check
    pub derivation: Vec<Derivation>,      // rules fired, table entries used, in order
    pub basis: Basis,                     // Deterministic | ModelJudged { model, input_hash }
    pub search: SearchRecord,             // REQUIRED for any absence or "no discrepancy" claim
    pub control: ControlResult,           // REQUIRED whenever `search` is
    pub disposition: Disposition,         // Open | Dismissed{reason} | Accepted | Fixed{ref}
}

pub struct SearchRecord {
    pub looked_in: ScopeDescription,      // documents (count, names or glob), code roots, at which revisions
    pub population: u64,                  // how many units were in scope: the denominator
    pub query: QueryDescription,          // normalized key, aliases applied, match mode
    pub matched: u64,                     // the null is matched == 0, stated as a number
    pub excluded: Vec<(Reason, u64)>,     // what was set aside, and how many, so exclusions are visible
}

pub struct ControlResult {
    pub kind: ControlKind,                // SelfFind | KnownPresent | DeclaredPresent
    pub item: ClaimRef,                   // the control's own subject
    pub outcome: ControlOutcome,          // Found | NotFound(=> the finding is INVALID and is withheld)
}
```

**Controls the engine runs on its own:**
- *SelfFind:* take a sample claim from the same population, search its own key against its own line with the same matcher and mode. If this
  fails the matcher is broken on this input (wrong tokenizer for the script, a normalizer that eats the key) and **every absence in that run
  is withheld**, not reported as clean. It has the same query and population as the null, varying only the subject, held at the same clock
  (the portfolio's "vary the subject, hold the clock").
- *KnownPresent:* an item the project **declares** present in each searched document class (`controls` in `project.toml`).
- A control that points at the same object as the null, or at an empty string, is rejected by construction (`item != null subject`,
  `key` non-empty): the portfolio's warning that a sound control "can't tell you that you pointed it at the same object twice"
  (`C:\Projects\CLAUDE.md` section 6).

`health_report` never says "no discrepancies" without the `SearchRecord` and `ControlResult` of the checks that produced the silence.

## 5. Acceptance

### 5.1 Gate A: the spike as golden oracle

The spike is the **oracle, not the base** (`docs/IDEAS-AND-DECISIONS-2026-11-17.md:54-62`). Its inputs and outputs are in the repo:
`spike/corpus/fuel/` (25 files in `arch-wide/`, the 3-file narrow corpus in `arch-run/`, `ROADMAP.md`), `spike/aliases.txt`, and the runs
`spike/runs/fuel-2026-11-16-v2..v9.txt`. Pinned figures, read from the run files (v8 and v9 headers, lines 1-8):

| run | claims | absent | mentioned | set aside | owned | contradictions | doc-vs-code |
|---|---|---|---|---|---|---|---|
| v3 (narrow corpus) | 558 | 212 (75 dir / 103 named / 34 ident) | 346 | - | - | - | - |
| v8 (wide corpus + doc 15) | 3846 | 764 (107 / 271 / 386) | 2115 | 518 | 449 | 37 | - |
| v9 (v8 + code at `b0d74efa`) | 3846 | 764 | 2115 | 518 | 449 | 37 | 236 built, 34 stub, 22 test-only, 472 absent |

**The gate:** the generic crates, given Fuel *only as data* (corpus, `aliases.txt` as the first concept layer, a `project.toml`
expressing the ROADMAP row pattern and the note-citation pattern), reproduce, per slice as it lands:

1. **Finding-id-set equality** against the matching run for the lexical, pragmatic and doc-vs-doc slices (v3, then v6/v7, then v8), where
   the algorithm is the same. Differences are allowed only when each one is listed and explained (for example, the spike's `file:line` ids
   map to ledger ids; the comparison keys on `(file, line, key)`).
2. **No Fuel-specific code**, enforced mechanically (2.5): a deny-list grep over engine `src/` with a positive control over the fixtures.
3. **The seven validation rows** against the planned-vs-built audit (`spike-report-2026-11-16.md:463-471`) for the Rust slice, **not** the
   v9 counts: a real parser will legitimately count differently from a line scanner. Four rows agreed with the audit; the generic Rust
   pack must at least match those four, and on the three where the spike disagreed (dead production code, dead chain, wrong-path
   reachability) must report the disagreement *explicitly* (reachable-from-where is part of the verdict) rather than a bare BUILT.
   Closing those three needs the call-graph lens the spike itself names (`spike/HANDOFF.md:67-70`); it is not required for step 1.
4. **Verdict vocabulary.** The spike's BUILT means "a production definition with a production call site", the audit's STUB/ABSENT means "not on
   the realize path" (`spike/HANDOFF.md:67-70`). The generic engine reports `Implements`/`Missing` with the *scope of the reachability
   claim* attached, and the Fuel mapping to BUILT/STUB/ABSENT is a report template, not engine behaviour.
5. **Recall replay (PROPOSAL, UNVERIFIED feasible).** The spike observed that run on the ROADMAP *before* row 157 existed would have
   put the multi-path DAG at the head of the finding list (`spike-report-2026-11-16.md:36`, with a miniature test of the same case). The gate
   adds the real replay: run the generic tools on Fuel at the commit before row 157 was added, and require the DAG to be a finding. I have not
   checked that Fuel's history supports this replay.

**Declared caveats about the oracle (so it is not over-trusted):** its precision is poor (472 of 764 absences remained ABSENT in code; 36
of 37 contradiction pairs are noise, `spike/HANDOFF.md:28-29`), so Gate A proves **fidelity to a known, weak baseline, not quality**. The spike's figures were checked by the lane on 2026-11-17 (slice 0, at the repo's `reference/pip-spike-2026-11-16` copy plus Fuel at `b0d74efa`,
fetched as a tarball into a scratch dir, 951 `.rs` files, no `target/`): `cargo test` passes **116** tests (28+16+12+16+33+11 across the test
binaries), 0 failed, matching the HANDOFF; the `grep -c '#[test]'` figure is 119 because 3 occurrences are string literals in `src/rust_code.rs:513`
and `tests/rust_code.rs:246,289`, not tests. Run v9 (`cargo run --release`, the HANDOFF command) produces 2668 lines identical to the committed
`runs/fuel-2026-11-16-v9.txt` (diff of 0 lines after CR stripping).

### 5.2 Gate B: the precision gate (modelled on CodeRipper's)

Method source: `C:\Projects\coderipper\docs\superpowers\plans\2026-10-24-performance-rules-precision.md:8-47` (read in the working tree; not pinned to
a ref). Adopted as written, with the changes noted:

- **Population:** every finding of a given `(lens, finding kind)`, over **at least three projects other than Fuel** (Fuel is the oracle and
  the tuning set; scoring on it would overfit). CodeRipper used three repositories (fuel, baracuda, Unpopped). Which three is D2.
- **Order and sampling:** sort by `(project, file, line)`, then shuffle with a **specified, language-independent seeded PRNG** (SplitMix64
  driving Fisher-Yates, seed fixed in the report). CodeRipper's shuffle is Python's `random.Random(2026)` (`:34`), which a Rust tool cannot
  reproduce bit-for-bit; the algorithm is therefore written into this spec and tested with fixed vectors.
- **Stop rule:** judge until **30 judged, or until the pass line is unreachable**, whichever is first. With pass line *p* the stop is
  `floor(30 * (1 - p)) + 1` false positives (at 80% that is the 7th false positive; arithmetic, `:36-37` of the plan above applies it to its own 80% gate, which was set for that plan's candidate performance rules, not a standing CodeRipper bar). A kind with fewer than 30 findings is a
  **census**, read whole, and cannot satisfy the "30 sampled" clause (`:36`).
- **Rubric (draft; the finding must be judged against the sources at the finding's own revisions):**
  - TRUE: the discrepancy or absence it states is real at the stated revisions **and** a reader acting on it fixes or learns something
    real (the document, the plan or the code is wrong, stale, or missing a thing it should have).
  - FALSE, with a **named noise class**: the spike's six are the starting list (one generic word carrying the pair; the denied token is
    absent from the positive side; a bare label as the positive side; scoped negation; a scope note read as a refusal; a copula
    classification read as a denial; `spike-report-2026-11-16.md:330-364`), plus new ones as found. Named classes become the next sense-table
    and rule work, which is the point of naming them.
  - UNJUDGEABLE (cannot be shown from the sources): **counted FALSE, as a lower bound**, the same convention as CodeRipper (`:42-43`).
- **Who judges:** one reader writes every verdict and a reason in a table anyone can dispute (CodeRipper's own caveat: "not an independent
  labelling", `:46-47`). **Addition:** a second judge (another lane or CireSnave) re-judges 10 of the 30 blind, and the report states the
  agreement; agreement below a threshold voids the figure.
- **Model-judged findings are excluded** from the deterministic figure and measured on their own (3.3).
- **Report:** per kind: findings, judged, true, precision, the one-sided **95% upper bound when true is 0** (CodeRipper's "about 7% (3/42)",
  `:65-66`, the rule of three), stop reason, disposition.
- **Tuning discipline:** a rule tuned on a sample invalidates that sample; the re-measure uses a **fresh** sample (different seed, and
  projects not seen during tuning). Without this the gate measures the tuner, not the tool.

**The pass line is CireSnave's** (D1). **PROPOSAL (strawman only):** 80% true on at least 30 judged per reported finding kind, the gate the CodeRipper performance-rules plan set for its own candidate rules (`2026-10-24-performance-rules-precision.md:4`, from `2026-10-23-performance-rules.md` section 7); CodeRipper has no standing precision bar (its lane, 2026-11-20). The 80% is this proposal's own, on the stop-at-7th-false-positive arithmetic; a kind below the line
is **not shown in `health_report`** (it can still be run and inspected) rather than shown with a
warning, because the false-confidence risk is that a presented finding is believed.

### 5.3 Gate C: kill / continue (for CireSnave to set)

The decision is his; this section gives him the evidence the gate will produce and a strawman. It follows the PM's stage 2 gate
(`docs/IDEAS-AND-DECISIONS-2026-11-17.md:45-49`: "CireSnave sets the pass line; below it, stop or re-scope").

**Evidence produced at the end of stage 2:** Gate A result (fidelity, with every difference explained); Gate B table per finding kind;
the named-noise-class list with which are fixed by data and which need a model; lane-days spent; the list of finding kinds that clear the
line.

**Strawman (PROPOSAL):**
- **Continue** if Gate A holds and at least **one** finding kind clears the pass line on three projects and answers a question the PM says
  a lane would otherwise ask by hand (the planned-vs-built audit is the named example).
- **Re-scope** (keep the base crates and ship only the kinds that clear) if some clear and some do not.
- **Stop** if **no** kind clears after **two tuning rounds**, each remeasured on a fresh sample (5.2), or if Gate A cannot be reached without
  Fuel-specific code (the genericity claim failed, which is the project's central claim).
- The **cost** line is a number of lane-days CireSnave names (the PM's note lists opportunity cost against Unpopped and Fuel as a risk,
  `docs/IDEAS-AND-DECISIONS-2026-11-17.md:41`); I do not invent one.

## 6. Build order

Each slice delivers something checkable on its own and is gated by the oracle (5.1) before the next starts. One PR per day per project
(portfolio rule); a slice may take several days and use a working branch.

- **Slice 0: confirm the oracle (no product code).** Run the spike's tests; fetch the Fuel tree at `b0d74efa`; reproduce v9; record the real
  test count. Output: a short note in `docs/`. Why first: Gate A is only as good as the baseline it compares to (5.1, caveats).
- **Slice 1, the smallest first slice: `analysant-core` + `analysant-lexical`, reproduce run v3.** Core: unit tree, `Claim`, `Form`, `Relation`
  types, `ClaimRef`, `Finding`, `SearchRecord`, `ControlResult`, the in-memory ledger (file format later), the Markdown `FormatAdapter`
  (PROPOSAL: in core behind a `markdown` feature, so a structural library is not a prerequisite), the `en` pack's tokenizer/normalizer/
  extract with its tables as data files. Lexical: normalize, `matches` (`TokenSubset`), `TermIndex`, alias canonicalization (a flat alias
  layer first). Gate: the 558 claims and 212 absent / 346 mentioned of v3 on the narrow corpus with `aliases.txt`, finding ids compared on
  `(file, line, key)`; the deny-list check; the SelfFind control wired in. **No pragmatic classification, no code, no contradiction.** v3 is
  chosen because it is the last run before the pragmatic lens: it needs only extraction, normalization and matching.
- **Slice 1 result (2026-11-19, lane analysant; branch `slice1-core-lexical`).** Built: `analysant-core` (model, seams, findings with search record and control, in-memory ledger, `en` pack with its word lists as data files, Markdown adapter behind the `markdown` feature), `analysant-lexical` (normalization, `Sequence` and `TokenSubset`, flat alias layer, word-bounded lexicon lookup, term index), and in `analysant` the extraction pipeline and the directed-but-untasked check. Gate, in `crates/analysant/tests/gate_v3.rs`: the narrow corpus with `aliases.txt` gives 558 claims (104 list items, 257 named, 197 code spans), 212 absent (75/103/34) and 346 mentioned; all 212 absent lines equal run v3's lists verbatim and in order; the 558 `(file, line, key)` tuples equal what the spike's own `extract_claims` produces (golden `tests/fixtures/v3-keys.tsv`, generated by running the spike unmodified); 0 findings withheld. The deny-list test (`tests/denylist.rs`) reads 15 engine source files and passes, with a positive control that must fail on a fixture; a deliberately unstable matcher has all its absences withheld (SelfFind), with the stable pack as the control. **Deviations from this spec, each deliberate:** (1) `Token`, `NormalForm` and `normalize` live in core, re-exported by lexical, because `LanguagePack::extract` must build keys and core cannot depend on lexical; (2) `Form` gains `Bold` and `CodeSpan` (the spike's two span shapes are not sentences); (3) `RawClaim.head` names the phrase a list item is keyed by (a leading emphasised name of two or more words, else its first eight content words), a pack rule with its numbers in `params.txt`; (4) lexicons are plain line files (`# analysant-lexicon v1`), not the 3.5 TOML, until the semantic layer needs scope and evidence fields; (5) `MatchMode::Family` is not built (first user is the contradiction lens); (6) the check and its gate live in the umbrella crate, since neither core nor lexical may know documents or plans; (7) a heading block's `section` is its real parents, where the spike's `extract_headings` listed the headings it was about to close (the spike's quirk is not reproduced; no slice-1 output reads it). **Not in this slice:** re-identification across edits (1.4), the ledger file (D3, D7), relations beyond the type, the pragmatic lens, code, contradiction.
- **Slice 2: `analysant-semantic` (concept store, sense table) and the pragmatic lens (status first, then directive, then description,
  the spike's order, `spike/src/pragmatic.rs:15-20`).** Gate: runs v4 to v5 (absent 212 to 105 to 73 on the narrow corpus, `spike/HANDOFF.md:23-24`).
- **Slice 2 result (2026-11-19, lane analysant; branch `slice2-semantic`).** Built: `analysant-pragmatic` (`PragmaticLens`, `Config { fragment_rules }`, `is_reportable`; the spike's status, directive, description and name rules, with its word lists moved to 20 `en` lexicons under `analysant-core/data/en/`) and `analysant-semantic` (`ConceptStore` with layers, `resolve`, `canonicalize`, `relate`, `run_tests`; plain-line file form, not the 3.5 TOML). The gate, in `crates/analysant/tests/gate_v4_v5.rs`, reproduces run v4 (105 absent: 7 directives, 64 named, 34 identifiers; 107 set aside) with `fragment_rules: false` and run v5 (73 absent: 7/32/34; 139 set aside) with it on; the finding lists and the set-aside lists equal the run files line for line and in order. **Deviations, each deliberate:** (1) the lens needs no semantic crate for its gate (the spike's v4/v5 used none), so the two crates are independent in this slice; (2) `analysant-semantic` is the consumer's shape only (PM notice 2026-11-18): no taxonomy, `Broader`/`Narrower` never produced, an unrecorded relation is `Undecided`; `subject_of` is not built (first user: slice 4); (3) the three-valued `Decision<T>` of 3.3 is not built: per the PM's ruling (2026-11-19, substrate: CireSnave) results use `semanticant-core`'s `Verdict<T>` and `Reason` (crates.io 0.1.0), so `relate` returns `Verdict<Option<TermRelation>>` where `None` is *undecided* (never *unrelated*) and the `Reason`'s rule says which table path fired; `relate_in` reads each term in its own context. 3.3 and 3.4 above still show the earlier sketch; (4) status/outcome/imperative lists are pack data, but the *shapes* the rules test (leading bold span, `Name:` head, YYYY-MM-DD date, parenthetical, `->`) are Markdown-and-English conventions in the lens; (5) the spike's `is_owned` partition is slice 3. **Not in this slice:** genre handling, ownership, contradiction, the seed sense table for the 21 noise cases (slice 4), a Kind for `Assertion`/`Question`/`Decision`/`Definition`/`Proposal` (the lens assigns Directive, Report, Description, Name only; D10 stands).
- **Slice 3: genre handling and ownership as project configuration** (outcome filter; note-to-row ownership via the plan-row pattern),
  wide corpus. Gate: v6 to v7 (absent 688, owned 449).
- **Slice 3 result (2026-11-19, lane analysant; branch `slice2-semantic`).** Built: `analysant::ownership` (`OwnershipConfig { row_word, row_start, citation_prefix }`, `owners`, `owner_for`, `partition_owned`) and `Config::decision_outcomes` in the pragmatic lens. The outcome filter itself was already in the slice 2 lens (its lexicon `outcome-markers` and its "decision outcome" rule), so this slice only made it switchable to reproduce v6. The gate, `crates/analysant/tests/gate_v6_v7.rs`, over the wide corpus without doc 15 (24 files, 3657 claims; the spike's v6/v7 input, doc 15 being v8's), reproduces v6 (1164 absent: 290/324/550; 449 set aside; filter off, no ownership) and v7 (688 absent: 89/224/375; 476 set aside; 449 owned: 175/99/175), with the finding, set-aside and owned lists equal to the run files line for line and in order; the filter moves exactly 27 claims. **Deviations:** (1) a title's first level-1 heading comes from the format adapter's blocks, not a `# ` prefix scan; (2) the row word is tokenized and case-folded by the pack, not split on `is_alphanumeric`; (3) `OwnershipConfig` has no `Default` (2.5 item 5), the gate supplies the reference project's three values as data; `.analysant/project.toml` is still slice 4.
- **Slice 4: the contradiction lens on `semantic`, including the sense table seeded from the 21 generic-word cases.** Gate: the 37 pairs
  reproduced, and then the 21 sense-noise pairs *removed with the cancellation pair still found* (the first place the generic code should
  beat the oracle, and the first Gate B sample).
- **Slice 4 result (2026-11-19, lane analysant; branch `slice2-semantic`).** Built: `analysant-logical` (`ContradictionLens`: statements from claims and headings, polarity, the spike's pairing rules, sense-aware suppression; `Findings { pairs, suppressed }`), `analysant::contradiction::statements`, and `analysant::ProjectConfig` (`.analysant/project.toml`: `[corpus]` aliases and concept files, `[ownership]`, `[contradiction] pairing_stopwords`; unknown keys are errors). The lens's word lists are nine `en` lexicons (negation phrases, bad words, negation and refusal words, temporal phrases and words, contrast separators, pairing stopwords) and six parameters in `params.txt`; the spike's `fuel` pairing stopword is not pack data but the reference project's `project.toml`. The gate, `crates/analysant/tests/gate_v8.rs`, over the wide corpus including doc 15 (3846 claims): part 1 reproduces run v8's 37 contradictions line for line and in order (negated and positive ids, sections, texts, rule, shared tokens), with no sense table; part 2 loads the reference project's seed sense table (`tests/fixtures/fuel-project/senses.concepts`: budget, harvest, lifecycle, written, network, 13 senses) and suppresses exactly the 21 pairs (budget 11, harvest 4, lifecycle 3, written 2, network 1), each with the store's `different-senses` reason; the other 16 stand in order; the cancellation pair (`06-runtime.md:71` against `15-consumer-contract.md:91`, "not supported") is still found; a table whose cues no context contains suppresses nothing (all 37 stand). **Deviations, each deliberate:** (1) the lens lives in `analysant-logical` (the stub crate), depending on `semantic` and `pragmatic`; (2) polarity splits text on non-alphanumerics as the spike does, not through the pack tokenizer, so `can't` matches as `can`, `t`; (3) a heading statement's section is its real parents; the spike printed the heading stack before closing the sibling it replaced (slice 1, deviation 7), so the gate rebuilds the spike's string from the adapter's blocks only to compare printed lines; (4) a pair is suppressed only when the sense-split tokens leave the remaining shared tokens too weak to carry it (`strong_enough`), and a candidate that is suppressed no longer competes for its line pair; (5) `subject_of` stays unbuilt: the lens needs only the subject tokens of a claim's key, which are lens logic; (6) **the seed table is overfitted by construction and the 21 are not all one kind.** Seventeen pairs are separated by senses of the word as the spike's triage read them. Four (budget against doc 15's resource budget) are the *same* thing a PlanOptions byte budget names, which the report called "a gap, not a contradiction"; they are removed only by a sense `budget-consumer-entitlement` cued on the words "consumer contract" in their section path. That is a judgement for the PM or CireSnave, not a measurement: without it the lens keeps 4 pairs the report calls noise. The three lifecycle pairs rest on a `md` cue (the positive sides are links to a document). Also, cues are matched against a statement's text and its section path; (7) `.analysant/project.toml` carries paths relative to the directory holding `.analysant/`; the fixture's alias file stays in the reference copy. **Not in this slice:** `MatchMode::Family` as a lexical matching mode (the family rule is in the lens), `KnownPresent` control, CLI, Escalator, `subject_of`, re-identification across edits, the ledger file, relations written to the ledger for a pair (a `Contradicts` relation is not yet emitted), the other five noise classes of the report's triage (denied token absent from the positive side 6, bare labels 2, scoped negation 4, scope notes 2, copula classification 1: 15 pairs; the report's "four" in this paragraph's first draft omitted the copula class, corrected 2026-11-20), which remain in the 16.
- **Slice 5: the Rust language pack** (`syn`), doc-vs-code relations, the seven validation rows.
- **Slice 5 result (2026-11-19, lane analysant; branch `slice2-semantic`).** Built: `analysant-rust` (a new crate; section 1's list of planned crates did not name it), `syn` 3.0.10 (D8: `syn` for items). `RustPack` implements `LanguagePack` (identifier tokens, `split_ident_words`, no sentences, no stopwords, `extract` returns nothing for a Markdown block); `CodeIndex::add_source` reads a `.rs` file into items (fn, struct, enum, variant, trait, const, static, type, union, field, macro), uses (every identifier of an item's tokens except its own definition name and `use` lines, each with its production or test context, its enclosing item, and whether it is called or dotted) and comment mentions (a small lexer, because `syn` drops comments). `CodeIndex::relate(name)` returns `Relation { Implements, Tests, Missing }` with the definitions, the mentions by kind, and for `Implements` a `Reach` whose `Scope` is `NameBasedOneHop`: `Called`, `CalledIfSharedNameResolves`, `OnlyFromUncalled` or `Uncalled`. Test context is a `#[test]` attribute, a `cfg` that does not hold in a production build (evaluated: `all(test, x)` is test, `any(x, test)` and `not(test)` are production, other predicates assumed true), or a test path (data: `tests`, `benches`, crate names containing `fixture` or `test-support`). All word lists are `data/rust/*.txt`; the qualified-name separator is data too, because the deny-list test (2.5) forbids a lexicon entry as a literal. **Gate (the seven rows, `crates/analysant-rust/tests/gate_rows.rs`, against the reference tree at `b0d74efa`, 951 files, 33,214 items, 0 parse failures, run with `ANALYSANT_REFERENCE_CODE`):** cancellation `Missing` with no code mention (agrees with the audit); `PlanStore` `Missing` with exactly 4 comment mentions and none in code (agrees); the Judge baseline concept `Missing` (agrees; a bare `Baseline` exists only as a struct in a benchmark under `tests/`, so it is `Tests`); observe/tap `Missing` (agrees); `coupling`: `Implements`, defined at `candidate.rs:82`, `Uncalled { test_uses: 1 }`, no production read (agrees with the audit's STUB exactly); `default_rules` `Uncalled` with two test uses (agrees); `capability_gated_rules` `OnlyFromUncalled`, its one caller `RuleRegistry::default_rules` (the dead chain, reported as such, where the spike said BUILT); `optimize_to_fixpoint` `Called` with `lower_to_base_map` named (the literal "has a production caller" is true; the audit's claim was about the realize path, which this engine does not see); `pick_route` `CalledIfSharedNameResolves`, its caller `RouteCache::resolve`, a name 40 other production definitions share (the spike said BUILT; the engine cannot say the caller is dead, and says so instead of answering built); `LOOKAHEAD_K` `Uncalled`. The four rows the spike agreed on (cancellation, `PlanStore`, the Judge baseline, observe/tap) agree. On the three the spike disagreed on (`pick_route`, the dead chain, the fixpoint) the engine reports the scope of its claim explicitly and does not close them; closing them needs the call graph (5.1 item 3). `tests/shapes.rs` holds the same shapes on synthetic sources and runs everywhere. **Deviations, each deliberate:** (1) the gate does not run in CI: the reference tree is not in the repository, and without `ANALYSANT_REFERENCE_CODE` the test prints SKIPPED and passes. It needs the self-hosted runner arrangement (CLAUDE.md, tests that cannot run on GitHub); until then the synthetic shapes are the only CI coverage of these rows, which is a gap and not a pass; (2) `relate` takes a name or `Type::member`, and `relate_words` a word list; no claim-to-code matching is built, so the spike's v9 DOC VS CODE section (236 built, 34 stub, 22 test-only, 472 absent) is **not** reproduced and was not an oracle (5.1 item 3); (3) a bare identifier used but never defined in the scanned tree (an external crate's item) is `Missing` with its production mentions counted, not `Implements`; (4) a function counts any occurrence except a dotted read as a use, where the spike counted only `name(` calls, so a function passed as a value is a use; (5) `Reach` has the fourth state `CalledIfSharedNameResolves`, which the spec's three words did not foresee: bare-name use counting cannot tell `RouteCache::resolve` from the other `resolve` methods, and a verdict that hid that would be the spike's bare BUILT again; (6) a trait-impl method and a function named by the pack's `entry-points` lexicon (`main`) count as called, since their callers cannot be seen. **Not in this slice:** reachability by the compiler (D8's later step), macro expansion, `cfg(feature)` evaluation against real features, doc comments handed back to the English pack, a Rust format adapter, a `Tests` relation by test-name matching, the report template that maps `Implements`/`Missing` to BUILT/STUB/ABSENT, and the umbrella crate's claim wiring.
- **Slice 6 part 1 result (2026-11-20, lane analysant; branch `gateb-prep`; scope per pm-ants [FIX] 2026-11-20: the library part, the MCP server is a separate, unbuilt project).** Built: `analysant_core::{LensSpec, Cost}` and `FindingKind::label` (plain data, std only); `analysant::tool` with `health_report(outputs, params) -> Report` (counts per `(lens, kind)` with a `KindStatus` of `Shown{measured}` or `Unmeasured`, the top `top_k` by severity then finding id, every shown finding id, per-lens coverage with population, withheld and set-aside counts and what was not analysed, and `kinds_below_pass_line`), `explain(id, outputs, corpus) -> Explanation` (the claims with text and locator, derivation, basis, search record, control, disposition; a claim missing from the corpus is reported as missing); `untasked::spec`, `contradiction::spec`, `UntaskedRun::output`. A lens with nothing shown is `Silence::Verified` (its `SearchRecord`s and `ControlResult`s, controls all `Found`, all `matched == 0`) or `Silence::Unverified` with the reason (it has findings that are hidden; findings were withheld; no search or control was recorded; a control failed; a search matched): the report has no bare "clean" state. The Gate B pass line is a parameter (`HealthParams`), never a default, because D1 is CireSnave's; a kind below it is left out of counts, top and ids, counted in `kinds_below_pass_line`, and can still be `explain`ed. Tests (`tests/tool_report.rs`, 8) assert exact values over the slice 1 gate's real untasked run (558 claims, 212 findings) and one-variable copies of its findings; flipping the pass-line comparison to `<=` fails one of them. **Deviations, each deliberate:** (1) `schemars` and any JSON are not built: the spec marks them a PROPOSAL, and a derive-based schema is the tool project's concern; `LensSpec` carries no input or output schema yet; (2) `Lens` the trait and `Corpus`, `Params`, `RunContext` are not built: the two lenses are called as functions, and a trait would be shaped by the tool project's needs; (3) the contradiction lens declares a spec but its pairs are not `Finding`s: a pair is not an absence, so `Finding`'s required `SearchRecord` and `ControlResult` do not fit it, and forcing them would invent a control; a `Contradicts` relation or a pair-shaped finding is the next step; (4) `UntaskedRun::output` records no null checks because the run keeps no search for a claim that was mentioned, so an untasked run with zero absences reports `Unverified`, not clean; (5) severity is a caller-supplied map by kind label, not a property of a finding, because no lens has a severity rule yet; (6) `explain` finds a claim by scanning the corpus (linear); an index is an optimisation nobody has asked for. **Not in this slice:** the MCP server, JSON, a schema, a `Lens` trait, `health_report` size budget in bytes (it is a count of findings), re-identification across edits.
- **Ledger and re-identification result (2026-11-20, lane analysant; branch `gateb-prep`; pm-ants accepted the row 2026-11-20).** Built: `analysant::ledger_file` and `analysant::identity` (tests: `crates/analysant/tests/identity.rs`, 19). The file is `.analysant/ledger.jsonl` (D3, D7), one JSON record per event with a `type` of `minted`, `revised`, `moved` or `gone`; a `revised` event carries the new text and the earlier events keep the old, so no text is ever deleted. An event's id is 32 hex digits of a hash of its content and is checked on every read, together with the stored `text_hash`, so a hand-edited line is refused (the tests edit a text with its hash left alone, and a text with its hash re-computed but its id left alone). `merge` is a union on event id in the canonical order `(unit, revision, kind, id)`: it is commutative and idempotent, either order writes byte-identical text, a sentence minted on two branches is one event, and two branches that reword one claim differently are a `Conflict` that keeps both rewordings (the later event id wins `state`, deterministically; a person decides). `reidentify` follows 1.4 step 2 in its order: exact text (same section path, or moved to another under a `moved` event), then edited in place (same path, token-set similarity at or above the caller's threshold, each the other's best), then mint or `gone`. **Deviations, each deliberate:** (1) *Ties.* Where the old side and the new side hold different numbers of identical sentences in one section (a sentence duplicated, one of two deleted), or two candidates are equally similar, the result is an `identity:ambiguous` finding and nothing is assigned: the old ids stay, no event is proposed, and a later extraction reports the same ambiguity until a person or an edit operation decides. Equal numbers in one section pair in reading order (so an unchanged document with a repeated sentence is not ambiguous every run); that pairing is a stated convention, not evidence. A duplicate placed in another section is told apart by the section. (2) *No ordinal.* 1.4 names "the same ordinal neighbourhood"; mutual best similarity within one section path stands in for it, because an ordinal moves when a sibling is inserted. (3) *The threshold has no default* (`IdentityParams.edit_similarity_permille`, a share of 1000 so no float comparison decides identity): 1.4 says the pack configures it and no pack carries one yet. The tests use 600. (4) *Ids.* A minted id is a hash of the document, section path, text, the occurrence among identical new sentences, and how many units of that content the ledger has already seen, so a sentence deleted and written again gets a new id (an id is never reused) and two branches that mint the same new sentence mint the same id (the union dedups it). The in-memory `Ledger` of slice 1 is unchanged and still counts. (5) *The finding's shape.* `Finding` requires a search record and a control (4.2); an ambiguity records the old candidates as `matched`, the live claims of the document as `population`, and the first candidate found by its own text as the control: a shape filled in, not an absence claim. (6) *Scope.* Claims only: sections and documents are not re-identified, and `relation changed` events are not written. A claim whose section is renamed with its text edited in the same commit reads as a new claim and a `gone` one. `UnitId::from_raw` is public (a writer mints, a reader restores) and `UnitId::parse` inverts `Display`; new dependencies `serde` 1.0.229 and `serde_json` 1.0.154. The module proposes events and writes nothing; `ledger_file::write_file` is for the writing tool alone (1.9). **Not in this slice:** the writing tool itself (`EditOp` application), a CLI that runs the cycle, ledger compaction, and wiring `reidentify` into `extract_corpus` so a `Claim`'s `unit.id` comes from the ledger file.
- **CLI result (2026-11-20, lane analysant; branch `gateb-prep`; pm-ants 2026-11-20: "build the findings producer CLI in analysant now, red-first").** Built: the `analysant` binary (`src/bin/analysant.rs`, a shell with no logic) and `analysant::produce`. `findings <dir> [--plan P]... [--project N] [--format tsv|jsonl]` runs the untasked check, the pragmatic partition, optional ownership, and the contradiction lens (with the project's sense table when it names one); `counts <dir>` gives counts per `(lens, kind)` and nothing else; `sample <file> --seed N [--kind lens/kind]` puts each kind in `analysant::gate_b` judging order. **Enumeration is `git ls-files -z` only** (the header says `# enumeration: git ls-files (T tracked, M markdown)`); a directory that is not a git work tree is refused with exit 2 and no output. Output is a header of `#` lines (project, enumeration, revision or `none (no commits)`, `clean`/`dirty`, config or `none (no-config baseline)`, file counts, one status per lens) then rows `project file line lens kind id refs search control text`, sorted by `(project, file, line, lens, kind, id)`, fields escaped; the same input gives the same bytes. The aggregate view withholds the revision, tree state, paths, text and ids, and renders a kind of a lens that did not run as `n/a`, never 0. Red first: `crates/analysant/tests/cli.rs` was written against a stub that exits 64 (18 of 19 failed; the one that passed compared two empty outputs and was strengthened), then the code; 19 tests run the real binary on throwaway git repositories (golden rows, determinism, jsonl equal to the lines, non-git refused with a positive control, untracked files unread with a positive control, aggregate leak test with a positive control that the findings view does contain the needles, no-plan, config read, a misspelled key an error, revision and dirty state, six `sample` tests and one usage test). Two mutations were caught (a header that leaks the revision into counts; the row sort removed). Run on this repository itself with the new files staged (210 tracked files, 57 markdown, `--plan CHANGELOG.md` as an arbitrary plan): two runs identical byte for byte, 349 contradiction pairs, 8317 untasked rows; that run is a smoke test, not a result. **Deviations, each deliberate:** (1) it is a binary in `analysant`, not a separate project (D5 separates MCP and editing tools; the producer is analysis output and has no MCP code); (2) `.analysant/project.toml` gains `[corpus] plan_files`, since the untasked check needs a plan and a baseline run must name it in the report, and a flag `--plan` adds to it; the plan files must be tracked and are not part of the corpus; (3) the corpus is the tracked `.md` files (the only format adapter is Markdown) outside `.analysant/`; (4) contradiction pairs are not `Finding`s (slice 6 deviation 3), so a pair's row has `-` for control, its `search` holds the shared tokens and the negation marker, its `refs` the positive side, its id `FindingId::compute` over `file:line|file:line` with no claim refs, and its line is the negated side's; (5) `sample` orders ties on `(project, file, line)` by finding id, because several pairs share one negated line (11 did for "budget") and the spec's sort key does not separate them; (6) arguments are parsed by hand (no `clap`: three subcommands, no new dependency); (7) the deny-list test (SPEC 2.5) reads the new files, which is why the column names are one string, the JSON keys are those names, the `git status` command is one string, and the project key is `plan_files` (`"plan"` is a shipped lexicon entry): a literal that equals a lexicon entry fails the test, and the test was not changed; (8) the control column is `<Kind>:<outcome>` in `Debug` form, lower-cased for the outcome (`SelfFind:found`, `SelfFind:notfound`). **Not in this slice:** verdict capture and a stop-rule tracker (the judge keeps the table; `gate_b::stop_state` is a library call), a JSON reader for `sample`, a pack choice (English and Markdown only), ownership without a plan, and any run on KISS, PersistAnt or HumboldtUnifiedKidTracker (D1 is unanswered; `docs/GATE-B-PROTOCOL.md` section 7 writes the baseline procedure).
- **Unit ids result (2026-11-20, lane analysant; branch `gateb-prep`; pm-ants ruling "ledger-writer-and-id-scheme", 2026-11-20).** One content-based id scheme for every unit kind: `analysant_core::content_id(kind, document, section_path, text, occurrence, generation)`. `Ledger::mint` (documents, sections, claims in `extract_corpus`) and `identity::mint_id` (claims minted by re-identification) both call it, so the in-memory ledger no longer has a salt or a counter: nothing else was needed, since occurrence (how many units of identical kind, document, path and text the ledger has already minted) separates duplicates and mint order across documents no longer matters. The unit kind is part of the hash, so a section and a claim with the same text differ. **Every unit id changed, and so did every claim-derived finding id** (an untasked finding's id is computed from its claim ids; a contradiction pair's id is computed from `file:line` pairs and did not change). Golden finding lines in `crates/analysant/tests/cli.rs` were regenerated in the same commit (three untasked ids; the contradiction id is the same); gates v3 to v8 compare by `(file, line, key)` and text, not ids, and passed unchanged. Tests, red first (`analysant-core/tests/unit_ids.rs`): the same content gets the same id in two ledgers and in either mint order of two documents; identical sentences in one section differ by occurrence; the kind is part of the id; `Ledger::mint` equals `content_id` at occurrence 0, generation 0. A new test in `tests/identity.rs` shows a fresh `reidentify` on an empty ledger mints exactly the ids extraction gives every claim, so `reidentify` can be wired into `extract_corpus` without changing a first extraction's ids: extract per document, read the prior `UnitState` from the ledger file, call `reidentify`, use its assignments for claim ids. It is **not wired**, because the only reason to wire it is the writer, which is EditAnt's (1.9), and wiring now changes no output. **Deviations:** the engine's tag bytes for unit kinds are numbers, not words (the deny-list test flags a word-list literal in engine source).
- **Judging result (2026-11-20, lane analysant; branch `gateb-prep`; the row named to pm-ants: verdict capture and a stop-rule tracker).** Built: `analysant::judging` and the `analysant judge append | report | sheet` subcommands. A verdict table (`# analysant-verdicts v1`, one line per verdict: finding id, `TRUE`/`FALSE`/`UNJUDGEABLE`, noise class, reason, judge; fields escaped; the same entries give the same bytes) is read and written by the tool, but every verdict is the judge's: a `FALSE` verdict must name a class, `TRUE` and `UNJUDGEABLE` have none, every verdict has a reason, one judge cannot judge one finding twice (a rewrite in place is refused), and a second judge's verdict on an id is a second line beside the first. `report` reads `analysant sample` output and the table and, per kind, walks the first judge's verdicts **in the sample's order** (a verdict after a gap is an error: the order is the method), applies `gate_b::stop_state` after each one, and prints findings, census or sample, judged (counted to the stop point), true, false, unjudgeable (counted false), precision, the rule-of-three bound `3/n` when true is 0, the stop reason, the verdicts kept beyond the stop point, the next position, the 30 clause, whether the kind clears, the noise-class counts, and second-judge agreement `k of n` over the 10 blind items `gate_b::second_judge_selection` picks. `sheet` prints exactly those items with positions and no verdict. **The pass line is a required argument with no default (D1), and the agreement threshold is `Option`**: unset, the report prints `threshold not set` beside the number and voids nothing; set (`--void-below K`), agreement below K is `voided`; an unanswered selected item is `incomplete`. Red first: `crates/analysant/tests/judging.rs` was run against stubs (20 of 20 failed), then the code (20 pass): the stop at exactly the 7th false positive on an 80% line (and not the 6th), 24 of 30 clearing 80% but stopping at the 4th false on a 90% line, `UNJUDGEABLE` as the 7th false, a census that cannot meet the 30 clause, agreement 8 of 10 standing and 7 of 10 voided under threshold 8 (the 10 positions are the fixed vector of `gate_b`'s own tests), a second judge's contrary verdict leaving the first judge's figure unchanged, an exact report text, a round trip with escapes, the rubric refusals, gap and unknown-id refusals, and the command end to end. Two mutations were caught (`>=` to `>` on the clears test; `UNJUDGEABLE` no longer counted false). **Deviations, each deliberate:** (1) figures are counted to the stop point and later verdicts are kept but not counted (`beyond-stop`), where CodeRipper's table counted every verdict it had read; the spec's stop rule is the definition and this keeps the figure a function of the rule; (2) the subcommand is `append`, because the deny-list test (2.5) forbids `"add"`, `"holds"` and `", "` as literals (they are shipped lexicon entries) and was not changed; (3) percentages are computed in integers and rounded half up to a tenth (so `3/7` prints 42.9%), not through `gate_b::rule_of_three_upper_bound`'s float; (4) `clears` needs the 30 clause, no unreachable stop, and `true * 100 >= p * judged`, so a census never clears (it is reported, as the spec says); (5) the report names the judge and the second judge it was run for but cannot check who they are; (6) **`reidentify` is not wired into `extract_corpus`, deliberately.** The in-memory ledger mints ids from a salt and a counter, and `identity` mints them from content and an occurrence count, so a fresh-ledger run through `reidentify` would give every claim a different id, and finding ids derive from claim ids: the findings producer's golden lines and the CLI's recorded ids would change. A parallel entry point would mix two id schemes in one `Corpus` (sections and documents are not re-identified, deviation 6 of the ledger paragraph) and nothing writes a ledger yet. Wiring it needs the writing tool and one id scheme for all three unit kinds, which is a change to what gates v3 to v8 and the CLI goldens pin, so it was not forced. **Not in this slice:** a stop-rule line count across kinds, a JSON form of the table, the writing tool, any run on KISS, PersistAnt or HumboldtUnifiedKidTracker (D1 is unanswered).
- **Doc-vs-code result (2026-11-20, lane analysant; branch `gateb-prep`; the "claim-to-code wiring" row of the slice 5 omissions).** Built: `analysant::doccode` (`doc_vs_code(&Corpus, &CodeIndex) -> DocCodeRun`, `spec()`, `DocCodeRun::output()` for `health_report`), `RustPack::code_reference` / `CodeIndex::code_reference` (which claim text names a code item) with two new `rust` pack data files (`external-roots`: `std core alloc`; `path-prefix-words`: `crate self super`), and `analysant findings|counts <dir> --code <dir>` (the code files come from `git ls-files` in the code directory, and the header says so: `# code: git ls-files (T tracked, R rust)`, `# code revision:`, `# code worktree:`, `# lens doc-vs-code: ...`; the counts view withholds revision and tree state and carries the five kinds with their numbers). A claim of form code span, bold or table cell whose text is identifier-shaped (a trailing call, a path of two segments or more, an `_`, or a lower-to-upper change of case) is looked up with `CodeIndex::relate`; defined-and-called says nothing (`Resolved`); the other five outcomes are the kinds `absent`, `tests-only`, `uncalled`, `dead-callers`, `shared-name`, each sentence ending in the scope of the claim (exact and case-sensitive, uses by bare name, one hop, no call graph). Every finding has a `SearchRecord` (Rust files and items searched, the query, definitions matched) and a `KnownPresent` control that is a **different claim** of the corpus that resolves to a production definition that is called; with no such claim, or with no Rust item indexed, a name that would have been reported is **withheld** (counted, never printed as clean, never as absent). Without `--code` the output of every command is byte-identical to before (the existing `cli.rs` goldens pass unchanged). Tests, written first: `crates/analysant/tests/doccode.rs` (10 value-asserting cases on a synthetic crate: each reach has its own kind, a called name has none, the control is a different claim, no control or an empty index withholds, std paths and `crate::` prefixes, stable ids, the spec, the report input), `tests/cli_code.rs` (7: exact row bytes, header, absent without the flag, determinism with a control, aggregate counts with a leak check and its control, a non-git code directory refused, a tree with no Rust files withheld; an untracked `target/junk.rs` that defines the missing name is not read), and `tests/doccode_real.rs` (env-gated exactly like `gate_rows`: `ANALYSANT_REFERENCE_CODE`, and with `ANALYSANT_REQUIRE_REFERENCE` a missing tree fails; CI's `reference-gate` job runs it). Two mutations were caught (the uncalled kind dropped; the std-path exclusion pointed at the wrong lexicon). **Run on the reference tree** (951 files, 33,214 items; the three narrow corpus documents): 133 names, 56 findings (absent 41, tests-only 4, uncalled 5, dead-callers 3, shared-name 3; pinned as a regression count in the test, not an oracle), 77 resolved, 0 withheld; it agrees with the audit rows the pack already reproduces (`PlanStore` absent; `pick_route` shared-name with 40 other definitions, not "built"). **Deviations and known noise, each a Gate B input, not a fix here:** (1) a path that starts in a *crate* (`fuel_core::lazy_nn_varbuilder::VarBuilder`) is looked up as `Type::member` on its last owner segment, which a module is not, so such names come out `absent`; (2) types of the standard library named without a path (`HashMap`) are not excluded; (3) a name a document gives in prose form with a call (`qwen2_7b()`) is reduced to the name, so the claim's own text and the query differ by the brackets; (4) the five kinds are the pack's `Relation` and `Reach` states mapped one to one, a project's own words (built, stub, absent) being a report template; (5) `Implements` with reach `Uncalled` is reported as `uncalled` even for a public library item that is meant to be called from outside the tree. **Not in this row:** a run on any Gate B project (D1 unanswered); the project file does not yet name a code directory (`--code` only). **Code directory in the project file (2026-11-20):** `[code] dir = "..."` in `.analysant/project.toml` (relative to the project root) names the code tree, so `--code` is optional; the flag overrides it; a missing directory or a non-git one is an error naming `[code]` and the path. One directory, not a list (deviation from the first sketch): the lens reads one tree with one revision, and a list would need merged enumerations and revisions that nothing asks for yet.
- **Slice 6: the tool seam made real** (`LensSpec`, `health_report`, `explain`) in a separate project if D5 says so.
- **Stage 4 (not mine yet): rewriting tools**, last by CireSnave's sequencing.

**Repo hygiene** (HANDOFF; same PR or the next): CI (build, test, `clippy -D warnings`, `fmt`, `cargo deny`), the portfolio CodeRipper
workflow, `changelog.d/` fragments (one per PR), SPDX headers on every source file, CHANGELOG, a workspace `Cargo.toml` whose members are
added only when their code exists. This PR adds the changelog fragment and CHANGELOG only; the CI workflows and the first workspace land in
the next PR (PM sets branch protection after the first CI run).

## 7. What carries over from the spike, and what does not

**Reused as ideas, with credit:** the claim-shape split (identity, text, key, section, kind), `spike/src/claim.rs`; the *order* of the pragmatic
tests (status, then directive, then description, then default, `pragmatic.rs:15-20`); word-boundary phrase matching instead of substring
(`pragmatic.rs:158-177`); stripping code spans before reading prose polarity (`pragmatic.rs:136-156`); the alias-then-subset matching
(`normalize.rs`, `check.rs`); identifier splitting (`rust_code.rs:145-149`); the contradiction lens's scoping rules (head window, negation
of a bad thing is positive, contrast split for headings only, strict pairing; `contradiction.rs:23-79`); the genre handling (outcome
filter, note-to-row ownership); and the **validation method** (additive diffs of run files, audit rows as ground truth for validation only,
never an input, `spike/HANDOFF.md:97-100`).

**Reused as data:** the corpus (`spike/corpus/fuel/`), `aliases.txt`, the run files, the 37-pair triage and its six noise classes, the
seven validation rows, as acceptance fixtures.

**Not reused:** the code. One crate with Fuel-shaped lexicons (the `fuel` pairing stopword, `contradiction.rs:636`; the imperative list;
`BAD_WORDS`, `contradiction.rs:262`) and a Fuel ROADMAP row format (`ownership.rs:101-115`); position-based identity; two overlapping enums
for kind and role; an English-shaped tokenizer; the hand-rolled Rust scanner and its named gaps (`rust_code.rs:8-37`). Where any spike
*code* is later ported, it keeps its SPDX/licence header and a credit to Pip and CireSnave: the spike declares `MIT OR Apache-2.0`
(`spike/PROVENANCE.md`), the same licence as this repository.

## 8. What I did not decide

Each is an `[ASK]` to the PM (recommended answer first).

| # | Question | Recommend | Blocks |
|---|---|---|---|
| D1 | The precision pass line and the kill/continue criterion (CireSnave's) | 80% on 30 judged per finding kind, three non-Fuel projects, two tuning rounds (5.2, 5.3) | Gate B, Gate C |
| D2 | Which three other projects form the Gate B population | one repo each from a Rust library, a Rust application and a docs-heavy repo; baracuda and Unpopped (used by CodeRipper's sample) are candidates | Gate B |
| D3 | Is the ledger committed to the analysed project, and how do branches merge it (1.9) | committed; merge by union of event records, since events carry unique ids | slice 1's file format |
| D4 | Markdown adapter inside `analysant-core` behind a feature (recommended), or only in `analysant-structural` | core + feature, so slice 1 needs no unbuilt crate | slice 1 |
| D5 | MCP and editing tools as separate projects on AnalysAnt (the PM's suggestion) or crates in this repo | separate, per the PM | slice 6 |
| D6 | The escalation model and its resource ceiling (3.3) | none until a Gate B shows a named noise class that tables cannot fix | any ML work |
| D7 | The ledger's and tables' location convention (`.analysant/`) across the portfolio | `.analysant/` at repo root | slice 1 |
| D8 | Rust pack parsing route (`syn` vs tree-sitter vs rust-analyzer) and whether to reuse CodeRipper MOD-011 reachability (I have not read it) | `syn` for items, compiler-backed reachability later | slice 5 |
| D9 | Whether Gate A's "Fuel as data only" may include a Fuel `project.toml` kept in this repo, or must live in the Fuel repo | in this repo under `tests/fixtures/`, as it is test data | slice 1 |
| D10 | Whether claim `kind` stays a single enum or becomes a set (a sentence can be a decision *and* a directive) | single, primary kind plus lens-private tags, until a Gate B case needs more | slice 2 |

**PM rulings, decided 2026-11-17, PM** (D1 stays CireSnave's; the strawman is the recommendation):
- D2: Gate B projects are KISS (docs-heavy), PersistAnt (Rust library) and HumboldtUnifiedKidTracker (Rust application, PRIVATE: local clone only, publish aggregate counts only). Do not reuse CodeRipper's sample repos.
- D3: ledger committed, merged by union on unique event ids. D4: markdown adapter in core behind a feature. D5: MCP and editing tools are separate projects.
- D6: no model until a Gate B names a noise class tables cannot fix. D7: `.analysant/` at repo root.
- D8: `syn` for items; read CodeRipper MOD-011 reachability code (origin/main, pinned SHA) before slice 5 and say what is reused.
- D9: fixtures live in this repo but the 4 MB reference/ corpus is not duplicated: fixtures point at `reference/pip-spike-2026-11-16/corpus`. D10: single primary kind plus lens-private tags.

## 9. Unverified list (everything I did not check)

- The spike's test count and that its tests pass (5.1): not run.
- That `corpus/fuel/code/` can be fetched at `b0d74efa` and v9 reproduced: **done 2026-11-17** (section 5.1, caveats): identical output.
- That Fuel's git history supports the pre-row-157 replay (5.1 item 5).
- CodeRipper MOD-011 (2.6): not read; cited only from `docs/IDEAS-AND-DECISIONS-2026-11-17.md`.
- The CodeRipper precision plan was read in the working tree, not at a pinned ref (5.2).
- The requirements' existing-tooling survey was not available to me (`docs/REQUIREMENTS-2026-11-16.md:5`), so nothing here is compared
  against ProseMirror, DITA, SummaC, GraphRAG or the others; "reuse an existing crate" options for tokenization or segmentation are not surveyed.
- Slice 2: the lens's recall on text outside the Fuel corpus (Gate B), and whether the lexicons' coverage of imperatives and participles transfers; the sense table has no seed yet (slice 4).
- `syn`, `schemars` and every other dependency named here as a PROPOSAL: version and licence not checked; they must be the current
  crates.io versions at the time of use (portfolio dependency rule).

— lane: analysant
