# Ledger writer conformance (for EditAnt, SPEC-STEP1 1.9)

analysant reads and merges `.analysant/ledger.jsonl`; a writing tool produces it. This is what a writer
must get exactly right, and how to check it. Everything a writer needs is in `analysant-core` (the
`analysant` crate re-exports it, so both paths work): `ledger_file` (format, `build`, `render`, `parse`,
`merge_text`, `state`, `history`), `corpus::extract_document`, `Ledger`, `content_id`, `Markdown`, `English`.

**The kit** (all under `crates/analysant-core`)
- `tests/fixtures/ledger-conformance/canonical.jsonl`: the golden file (header; one claim minted, reworded, moved and
  removed; a second claim that stays).
- `tests/fixtures/ledger-conformance/escaping.jsonl`: the second golden file (non-ASCII, quote, backslash, `/`, tab,
  newline, U+001F, U+007F, U+2028, an astral character; in text, section path and document path).
- `examples/ledger_writer.rs`: a reference writer; `cargo run -p analysant-core --example ledger_writer [escaping]`
  prints either file byte for byte.
- `tests/ledger_conformance.rs` (11 tests) and `tests/ledger_extraction.rs` (2 tests): see below.
- `tests/fixtures/ledger-conformance/check_ledger.py`: an independent Python check of rules 5 and 6 (and of the text hash)
  for any ledger file: `python -I check_ledger.py FILE.jsonl`. It passes on both golden files.

**Which crate.** A writer needs only `analysant-core`. Use `extract_document` to get claims (rule 7); use
`ledger_file::{build, render, merge_text}` or reimplement rules 1-6 against the golden files, as you prefer.

**The rules**
1. First line of a non-empty file: `{"type":"format","name":"analysant-ledger","version":1}`. Not an event.
2. One compact JSON object per line, field order as in the golden file, no unknown fields. Every line ends in `\n`. A reader
   also accepts `\r\n` (Windows `core.autocrlf`) and any other valid JSON escaping; a write always uses `\n` and the escaping
   of `serde_json`, which the second golden file pins: `"` and `\` escaped; backspace, form feed, newline, carriage return
   and tab as `\b \f \n \r \t`; other controls below U+0020 as `\u00xx` (lower-case hex); U+007F, `/`, U+2028 and all
   non-ASCII raw in UTF-8. Keep the file LF in a repository (`.gitattributes`: `<path> text eol=lf`; this repo does it for the
   fixtures; do the same for `.analysant/ledger.jsonl`).
3. Events are kept in one canonical order, `(unit, revision, kind rank minted<revised<moved<gone, event id)`, where `unit` is
   the unit id's number (not the string). Use `render`.
4. **No event is ever edited or deleted; old text stays. But a write is a whole-file rewrite, not a byte append.** Canonical
   order puts a new event in the middle of the file when its unit sorts there. So a writer does: read the file, add its
   events, `merge_text(old, new)` (a union on event id), write. **Two writers on one file** can lose each other's events with a
   plain temp-file-and-rename. The rule: create `.analysant/ledger.lock` exclusively (fail, do not wait forever, if it
   exists), read, merge, write a temp file, rename it over the ledger, remove the lock. A writer that cannot take the lock
   refuses. Across branches the union merge (`merge_text`, D3) is the arbiter, not the lock.
5. A unit id is `content_id(kind, document, section_path, text, occurrence, generation)` (core). `occurrence` indexes
   identical units minted in one pass over one document; `generation` counts earlier ledger-file units with the same content
   (live, edited or gone). The id is an identity, not an integrity check. Never reimplement it: call it.
6. An event id is 32 hex digits: `hi` then `lo`, each an FNV-1a 64 over the parts below, each part followed by byte `0xff`
   (`lo` over the parts; `hi` over `"hi"` then the parts). Parts: the kind rank byte (0..3); **the event's `unit` field exactly
   as written (the base32 unit id string, e.g. `G2A77DBEZX5JUGV2OOOVSG7P6S`; not the claim's text)**; the revision as u32 LE;
   the `document` string; then for `minted`: kind byte (document 0, section 1, block 2, claim 3), the section path joined by
   U+001F, the text hash; for `revised`: the text hash; for `moved`: the section path joined by U+001F; for `gone`: nothing
   more. The text hash is 16 hex digits of FNV-1a 64 over the text (one part). Hashes are over the decoded strings, so
   escaping never changes an id. (A path segment that itself contains U+001F is ambiguous with the joiner; do not rely on it.)
7. **Where claims, section paths and document paths come from.** From `corpus::extract_document(&Markdown, &English::new(),
   &mut Ledger::default(), path, text)`: `claim.text` and `doc.section_path(claim)` are exactly what an event records, and
   `occurrence` follows from the order of identical claims in that one pass. A writer never segments sentences or normalises
   headings itself. `path` is the one `git ls-files` prints: relative to the project root, `/` separators, git's case; a
   different spelling is a different document (`ledger_extraction.rs`). What the extractor does today, pinned in
   `ledger_extraction.rs`, limits included: a closing `#` of an ATX heading stays (`# Top Title #` is `Top Title #`), a setext
   heading (`Title` over `=====`) is not a heading, inline markup stays, and no Unicode normalisation is applied. A change to
   any of these changes ids and fails that test first.
8. **Revisions.** `minted` is revision 1. Only `revised` raises it, to the unit's latest revision plus one. `moved` and
   `gone` carry the revision the unit has when they are written (the golden: minted 1, revised 2, moved 2, gone 2). An edit
   plan's `expect_rev` (SPEC 1.8) is compared with the unit's latest revision in the file's state, and a mismatch is
   refused. Readers do not enforce this rule; `the_golden_file_obeys_the_revision_rules` shows the shape.
9. **A section rename or move** is one `moved` event per claim under that section, descendants included, each with the claim's
   new full section path. Sections are not units in the ledger (only claims are recorded so far), so there is no section event.

**What the tests check** (`ledger_conformance.rs`): the reference writer reproduces both golden files byte for byte; parse then
render is the identity; unit and event ids are pinned; state and history; append is a union (idempotent, either order);
mistakes are refused (no header, a hand-edited event, an unknown field, a future version); CRLF and other escapings are read
and a write is canonical; the golden obeys the revision rules.

**Independent checks.** EditAnt reproduced the five event ids and three text hashes of `canonical.jsonl` in its own Python
(reported 2026-11-21); `check_ledger.py` reproduces every event id and text hash of both golden files. A writer in another
language can do the same.

Changing rule 5 or 6 changes every id in every ledger file and raises `LEDGER_FORMAT_VERSION`.
