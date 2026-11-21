# Ledger writer conformance (for EditAnt, SPEC-STEP1 1.9)

analysant reads and merges `.analysant/ledger.jsonl`; a writing tool produces it. This is what a writer
must get exactly right, and how to check it.

**The kit**
- `crates/analysant/tests/fixtures/ledger-conformance/canonical.jsonl`: the golden file (header, then one claim minted,
  reworded, moved and removed, and a second claim that stays).
- `crates/analysant/examples/ledger_writer.rs`: a reference writer (`cargo run -p analysant --example ledger_writer`);
  its output is that file byte for byte.
- `crates/analysant/tests/ledger_conformance.rs`: seven tests (reference writer = golden; parse then render is the
  identity; pinned unit and event ids; state and history; append = union; a writer's mistakes are refused).

**The rules**
1. First line of a non-empty file: `{"type":"format","name":"analysant-ledger","version":1}`. Not an event.
2. One compact JSON object per line, `\n` ends every line, field order as in the golden file, no unknown fields.
3. Events sort by `(unit, revision, kind rank minted<revised<moved<gone, event id)`; use `render`.
4. **Append only.** Never edit or delete an earlier line; old text stays in the file. Appending is `merge_text(old, new)`.
5. A unit id is `content_id(kind, document, section_path, text, occurrence, generation)`; `occurrence` indexes identical
   units minted in one pass over one document; `generation` counts earlier ledger-file units with the same content (live,
   edited or gone). The id is an identity, not an integrity check.
6. An event id is 32 hex digits: `hi` then `lo`, each an FNV-1a 64 over the parts below, each part followed by byte `0xff`
   (`lo` over the parts; `hi` over `"hi"` then the parts). Parts: kind rank byte (0..3); unit text; revision as u32 LE;
   document; then for `minted`: kind byte (document 0, section 1, block 2, claim 3), section path joined by U+001F, text hash;
   for `revised`: text hash; for `moved`: section path joined by U+001F; for `gone`: nothing more. The text hash is 16 hex
   digits of FNV-1a 64 over the text (one part).

**Independent check.** The first event id in the golden file, `2b55c072a9bce3dd261a07212533a033`, was reproduced by a
separate Python implementation of rule 6 (2026-11-21). A writer in another language can do the same.

Changing rule 5 or 6 changes every id in every ledger file and raises `LEDGER_FORMAT_VERSION`.
