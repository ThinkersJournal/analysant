// SPDX-License-Identifier: MIT OR Apache-2.0
//! What a writer gets from `extract_document` (kit gaps G5 and G6): the claim texts, section paths
//! and document path a ledger event records are exactly these strings. A writer calls this function
//! and never re-derives them. The pinned values below are what the extractor does today, including
//! its limits (a closing `#` of an ATX heading stays, a setext heading is not a heading, inline markup
//! stays, no Unicode normalisation); a change to any of them changes ids, so it fails here first.

use analysant_core::{English, Ledger, Markdown, extract_document};

const DOC: &str = "Docs/Sub Dir/A.md";

fn paths_and_texts(md: &str) -> Vec<(Vec<String>, String)> {
    let mut scratch = Ledger::default();
    let doc = extract_document(&Markdown, &English::new(), &mut scratch, DOC, md);
    doc.claims
        .iter()
        .map(|c| {
            (
                doc.section_path(c)
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
                c.text.clone(),
            )
        })
        .collect()
}

#[test]
fn awkward_headings_come_out_as_these_exact_strings() {
    let md = "# Top Title #\n\n- The intake service validates every request.\n\nSetext Head\n===========\n\n- The queue drains in arrival order.\n\n## *Emph* and `code` head\n\n- The writer flushes results to disk.\n\n### Caf\u{e9} e\u{301} 100% [q]\n\n- The reader tails the file.\n";
    let got = paths_and_texts(md);
    let s = |p: &[&str]| p.iter().map(|x| (*x).to_string()).collect::<Vec<_>>();
    assert_eq!(
        got,
        vec![
            (
                s(&["Top Title #"]),
                "The intake service validates every request.".to_string()
            ),
            (
                s(&["Top Title #"]),
                "The queue drains in arrival order.".to_string()
            ),
            (
                s(&["Top Title #", "*Emph* and `code` head"]),
                "The writer flushes results to disk.".to_string()
            ),
            (
                s(&[
                    "Top Title #",
                    "*Emph* and `code` head",
                    "Caf\u{e9} e\u{301} 100% [q]"
                ]),
                "The reader tails the file.".to_string()
            ),
        ]
    );
}

#[test]
fn the_document_path_is_used_as_given_with_no_case_or_separator_change() {
    // The writer passes the path `git ls-files` prints: relative to the project root, `/` separators,
    // the case git has. The ledger records it unchanged; a different spelling is a different document.
    let mut a = Ledger::default();
    let mut b = Ledger::default();
    let one = extract_document(
        &Markdown,
        &English::new(),
        &mut a,
        "docs/A.md",
        "- Same words here.\n",
    );
    let two = extract_document(
        &Markdown,
        &English::new(),
        &mut b,
        "Docs/A.md",
        "- Same words here.\n",
    );
    assert_eq!(one.path, "docs/A.md");
    assert_ne!(one.claims[0].unit.id, two.claims[0].unit.id);
}
