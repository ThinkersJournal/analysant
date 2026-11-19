// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Markdown `FormatAdapter` (behind the `markdown` feature, SPEC-STEP1 D4). Structure only:
//! headings, list items, table cells, emphasised spans, code spans and fenced code blocks. Whether a
//! span is a name, an identifier or a directive is the language pack's call, not this file's.

use crate::model::Span;
use crate::pack::{Block, BlockKind, FormatAdapter, PackId};

/// Markdown, line oriented.
#[derive(Clone, Copy, Debug, Default)]
pub struct Markdown;

impl FormatAdapter for Markdown {
    fn id(&self) -> PackId {
        PackId {
            name: "markdown",
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    fn blocks(&self, text: &str) -> Vec<Block> {
        // (byte offset, line without its terminator), exactly the lines `str::lines` yields.
        let mut lines: Vec<(usize, &str)> = Vec::new();
        let mut offset = 0usize;
        for seg in text.split_inclusive('\n') {
            let body = seg.strip_suffix('\n').unwrap_or(seg);
            let body = body.strip_suffix('\r').unwrap_or(body);
            lines.push((offset, body));
            offset += seg.len();
        }

        // The line immediately before a separator row names columns, not components.
        let mut header_lines = vec![false; lines.len()];
        for (idx, (_, line)) in lines.iter().enumerate() {
            if idx > 0 && is_table_separator(line) {
                header_lines[idx - 1] = true;
            }
        }

        let mut blocks = Vec::new();
        let mut headings: Vec<(u8, String)> = Vec::new();
        let mut fence: Option<(String, u32, usize, String)> = None; // (info, first line, start, body)

        for (idx, &(start, raw)) in lines.iter().enumerate() {
            let line_no = (idx + 1) as u32;
            let line = raw.trim_end();
            let trimmed = line.trim_start();
            let span = Span {
                start,
                end: start + raw.len(),
                line_start: line_no,
                line_end: line_no,
            };

            if let Some(open) = trimmed.strip_prefix("```") {
                match fence.take() {
                    None => {
                        fence = Some((open.trim().to_string(), line_no, start, String::new()));
                    }
                    Some((info, first, fstart, body)) => {
                        blocks.push(Block {
                            kind: BlockKind::CodeBlock { info },
                            line: first,
                            span: Span {
                                start: fstart,
                                end: start + raw.len(),
                                line_start: first,
                                line_end: line_no,
                            },
                            section: section_of(&headings),
                            text: body,
                            list_item: None,
                            table_cell: None,
                            strong: Vec::new(),
                            code: Vec::new(),
                        });
                    }
                }
                continue;
            }
            if let Some((_, _, _, body)) = fence.as_mut() {
                body.push_str(raw);
                body.push('\n');
                continue;
            }

            if header_lines[idx] {
                blocks.push(plain(
                    BlockKind::TableHeader,
                    line,
                    span,
                    section_of(&headings),
                ));
                continue;
            }

            if let Some((level, title)) = parse_heading(line) {
                // Pop the headings this one closes first: its parents are what is left.
                while headings.last().is_some_and(|(l, _)| *l >= level) {
                    headings.pop();
                }
                let section = section_of(&headings);
                headings.push((level, title.clone()));
                blocks.push(plain(BlockKind::Heading { level }, &title, span, section));
                continue;
            }

            blocks.push(Block {
                kind: BlockKind::Content,
                line: line_no,
                span,
                section: section_of(&headings),
                text: line.to_string(),
                list_item: parse_list_item(line).map(str::to_string),
                table_cell: parse_table_first_cell(line).map(str::to_string),
                strong: extract_spans(line, "**")
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                code: extract_spans(line, "`")
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            });
        }
        blocks
    }
}

fn plain(kind: BlockKind, text: &str, span: Span, section: Vec<String>) -> Block {
    Block {
        kind,
        line: span.line_start,
        span,
        section,
        text: text.to_string(),
        list_item: None,
        table_cell: None,
        strong: Vec::new(),
        code: Vec::new(),
    }
}

fn section_of(headings: &[(u8, String)]) -> Vec<String> {
    headings.iter().map(|(_, t)| t.clone()).collect()
}

/// `#` heading: (level, title).
fn parse_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') && !rest.is_empty() {
        return None;
    }
    let title = rest.trim();
    if title.is_empty() {
        return None;
    }
    Some((hashes as u8, title.to_string()))
}

/// List item: `- `, `* `, `+ `, or `1. ` / `1) `. Returns the item text.
fn parse_list_item(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return Some(rest);
        }
    }
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        let rest = &trimmed[digits..];
        // The marker is a digit run, then `.` or `)`, then one space.
        let after = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?;
        return after.strip_prefix(' ');
    }
    None
}

/// First cell of a table row, or `None`. Separator rows yield `None`.
fn parse_table_first_cell(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') || trimmed.len() < 2 {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let first = inner.split('|').next()?.trim();
    if first.is_empty() || is_table_separator(line) {
        return None;
    }
    Some(first)
}

/// `| --- |`, `| :---: |`, and the like.
fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') || trimmed.len() < 2 {
        return false;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    !inner.is_empty()
        && inner
            .split('|')
            .all(|cell| cell.trim().chars().all(|c| matches!(c, '-' | ':' | ' ')))
        && inner.split('|').any(|cell| cell.contains('-'))
}

/// Every span delimited by `delim` on one line: non-overlapping, in order.
fn extract_spans<'a>(line: &'a str, delim: &str) -> Vec<&'a str> {
    let mut spans = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find(delim) {
        let after = &rest[open + delim.len()..];
        match after.find(delim) {
            Some(close) => {
                spans.push(&after[..close]);
                rest = &after[close + delim.len()..];
            }
            None => break,
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_shapes() {
        let text = "# Top\n\n## Sub\n- **Name two**: body\n| Cell | x |\n| --- | --- |\n| Row | y |\nplain `a_b` text\n";
        let blocks = Markdown.blocks(text);
        let kinds: Vec<_> = blocks.iter().map(|b| b.kind.clone()).collect();
        assert_eq!(kinds[0], BlockKind::Heading { level: 1 });
        // The line before the separator is the header; the separator itself is plain content.
        assert!(kinds.contains(&BlockKind::TableHeader));
        let item = blocks.iter().find(|b| b.list_item.is_some()).unwrap();
        assert_eq!(item.section, ["Top", "Sub"]);
        assert_eq!(item.strong, ["Name two"]);
        assert_eq!(item.line, 4);
        let row = blocks
            .iter()
            .find(|b| b.table_cell.as_deref() == Some("Row"))
            .unwrap();
        assert_eq!(row.line, 7);
        let code = blocks.iter().find(|b| !b.code.is_empty()).unwrap();
        assert_eq!(code.code, ["a_b"]);
    }

    #[test]
    fn a_heading_section_is_its_real_parents_not_the_siblings_it_closes() {
        let text = "# A
## B
### C
## D
text
";
        let blocks = Markdown.blocks(text);
        let d = blocks.iter().find(|b| b.text == "D").unwrap();
        assert_eq!(d.section, ["A"]);
        assert_eq!(blocks.last().unwrap().section, ["A", "D"]);
    }

    #[test]
    fn fenced_code_is_its_own_block_and_carries_no_spans() {
        let text = "```rust\nfn **x**() {}\n```\nafter\n";
        let blocks = Markdown.blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[0].kind,
            BlockKind::CodeBlock {
                info: "rust".into()
            }
        );
        assert!(blocks[0].strong.is_empty());
        assert_eq!(blocks[0].span.line_end, 3);
        assert_eq!(blocks[1].line, 4);
    }

    #[test]
    fn crlf_lines_match_str_lines() {
        let text = "- one two\r\n- three\r\n";
        let blocks = Markdown.blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].list_item.as_deref(), Some("one two"));
        assert_eq!(&text[blocks[1].span.start..blocks[1].span.end], "- three");
    }
}
