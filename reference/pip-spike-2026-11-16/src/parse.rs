//! Markdown extraction (spike slice, English prose).
//!
//! Extracts three kinds of claims from a Markdown document:
//! list items (directives), bold phrases and backtick identifiers
//! (named capabilities), and table first-cells (named components).
//! Fenced code blocks are skipped: code is a different language and
//! gets its own parser (the Rust language pack), not the prose parser.
//! Headings are section context only, never claims themselves.
//! Named claims are deduplicated by normalized key within a file
//! (first occurrence wins), so a capability named fifty times is one claim.

use crate::claim::{Claim, ClaimKind, PragmaticRole};
use crate::normalize::{content_words, normalize};
use crate::pragmatic::{classify_list_item, classify_name};
use std::collections::HashSet;

/// Extract all claims from one Markdown document.
pub fn extract_claims(source_file: &str, text: &str) -> Vec<Claim> {
    let lines: Vec<&str> = text.lines().collect();
    // A table header is the line immediately before a separator row
    // (`| --- |`); it names columns, not components, so it yields no claims.
    let mut header_lines: HashSet<usize> = HashSet::new();
    for (idx, line) in lines.iter().enumerate() {
        if is_table_separator(line) && idx > 0 {
            header_lines.insert(idx); // 1-based number of the previous line
        }
    }

    let mut claims = Vec::new();
    let mut seen_named: HashSet<String> = HashSet::new();
    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut in_fence = false;

    for (idx, raw_line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim_end();
        let trimmed = line.trim_start();

        // Fenced code blocks: toggle and skip (code is another language).
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        // Table header rows yield no claims.
        if header_lines.contains(&line_no) {
            continue;
        }

        // Headings set section context.
        if let Some((level, title)) = parse_heading(line) {
            while headings.last().map(|(l, _)| *l >= level).unwrap_or(false) {
                headings.pop();
            }
            headings.push((level, title));
            continue;
        }
        let section = headings
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<_>>()
            .join(" > ");

        // List items are directives — or descriptions/status notes wearing
        // a list item's shape. The pragmatic lens (slice 1) tells them
        // apart; the kind stays Directive and the role carries the call.
        if let Some(item) = parse_list_item(line) {
            let item = item.trim();
            if item.chars().count() >= 3 {
                claims.push(Claim {
                    id: format!("{source_file}:{line_no}"),
                    key: directive_key(item),
                    text: item.to_string(),
                    source_file: source_file.to_string(),
                    line: line_no,
                    section: section.clone(),
                    kind: ClaimKind::Directive,
                    role: classify_list_item(item, &section),
                });
            }
        }

        // Table rows: the first cell names a component. Skip the header
        // separator (`| --- |`) and rows whose first cell is empty.
        if let Some(cell) = parse_table_first_cell(line) {
            let cell = cell.trim();
            if cell.chars().count() >= 3 {
                let key = normalize(cell);
                if !key.is_empty() && seen_named.insert(key.clone()) {
                    claims.push(Claim {
                        id: format!("{source_file}:{line_no}"),
                        key,
                        text: cell.to_string(),
                        source_file: source_file.to_string(),
                        line: line_no,
                        section: section.clone(),
                        kind: ClaimKind::Named,
                        role: classify_name(cell, &section),
                    });
                }
            }
        }

        // Bold phrases are named capabilities — but only if they look like
        // a name. A bolded sentence is emphasis, not a capability: reject
        // spans that contain sentence punctuation (", ", "; ", ". "),
        // start with a lowercase conjunction or preposition, run longer
        // than eight words, or are mostly uppercase (shouting, not naming).
        for span in extract_spans(line, "**") {
            if looks_like_capability_name(span) {
                push_named(
                    &mut claims,
                    &mut seen_named,
                    source_file,
                    line_no,
                    &section,
                    span,
                );
            }
        }
        // Backtick identifiers are code facts, reported separately.
        for span in extract_spans(line, "`") {
            if looks_like_identifier(span) {
                let text = span.trim();
                let key = normalize(text);
                if !key.is_empty() && seen_named.insert(key.clone()) {
                    claims.push(Claim {
                        id: format!("{source_file}:{line_no}"),
                        key,
                        text: text.to_string(),
                        source_file: source_file.to_string(),
                        line: line_no,
                        section: section.clone(),
                        kind: ClaimKind::Identifier,
                        role: PragmaticRole::Name,
                    });
                }
            }
        }
    }
    claims
}

fn push_named(
    claims: &mut Vec<Claim>,
    seen: &mut HashSet<String>,
    source_file: &str,
    line_no: usize,
    section: &str,
    span: &str,
) {
    let text = span.trim();
    if !(3..=80).contains(&text.chars().count()) {
        return;
    }
    let key = normalize(text);
    if key.is_empty() || !seen.insert(key.clone()) {
        return;
    }
    claims.push(Claim {
        id: format!("{source_file}:{line_no}"),
        key,
        text: text.to_string(),
        source_file: source_file.to_string(),
        line: line_no,
        section: section.to_string(),
        kind: ClaimKind::Named,
        role: classify_name(text, section),
    });
}

/// A heading line: file, line, title, and the parent heading path
/// (the section the heading sits under, not including itself).
#[derive(Debug, Clone)]
pub struct Heading {
    pub file: String,
    pub line: usize,
    pub title: String,
    pub section: String,
}

/// All headings in one document, in order. Headings are section
/// context for the directed-but-untasked check, but the contradiction
/// lens (slice 5) reads them as statements: a heading states the
/// document's own position ("Cancellation is not supported"). Fenced
/// code blocks are skipped, as in `extract_claims`.
pub fn extract_headings(source_file: &str, text: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut in_fence = false;
    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim_end();
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((level, title)) = parse_heading(line) {
            let section = stack
                .iter()
                .map(|(_, t)| t.as_str())
                .collect::<Vec<_>>()
                .join(" > ");
            headings.push(Heading {
                file: source_file.to_string(),
                line: line_no,
                title: title.clone(),
                section,
            });
            while stack.last().map(|(l, _)| *l >= level).unwrap_or(false) {
                stack.pop();
            }
            stack.push((level, title));
        }
    }
    headings
}

/// `#` heading: returns (level, title).
fn parse_heading(line: &str) -> Option<(usize, String)> {
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
    Some((hashes, title.to_string()))
}

/// List item: `- `, `* `, `+ `, or `1. ` / `1) `. Returns the item text.
fn parse_list_item(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return Some(rest);
        }
    }
    // Numbered: digits, then '.' or ')', then a space.
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let rest = &trimmed[digits.len()..];
        if let Some(rest) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return Some(rest);
        }
    }
    None
}

/// Table row first cell, or None. Separator rows (`| --- |`) return None.
fn parse_table_first_cell(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let first = inner.split('|').next()?;
    let first = first.trim();
    if first.is_empty() || is_table_separator(line) {
        return None;
    }
    Some(first)
}

/// True when the line is a table separator row (`| --- |`, `| :---: |`, ...).
fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return false;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    !inner.is_empty()
        && inner.split('|').all(|cell| {
            cell.trim()
                .chars()
                .all(|c| c == '-' || c == ':' || c == ' ')
        })
        && inner.split('|').any(|cell| cell.contains('-'))
}

/// All spans delimited by `delim` on one line (non-overlapping, in order).
/// For `**` this finds bold phrases; for backtick, code spans.
/// A span is the text between two occurrences of the delimiter.
fn extract_spans<'a>(line: &'a str, delim: &'a str) -> Vec<&'a str> {
    let mut spans = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find(delim) {
        let after_open = &rest[open + delim.len()..];
        match after_open.find(delim) {
            Some(close) => {
                spans.push(&after_open[..close]);
                rest = &after_open[close + delim.len()..];
            }
            None => break,
        }
    }
    spans
}

/// The key of a list item: its head, not the whole item. A leading bold
/// span of two or more words names it; otherwise the first eight content
/// words do. A single-word bold lead ("Today") is a temporal marker, not
/// a name, so it falls through to the head.
fn directive_key(item: &str) -> String {
    if let Some(span) = extract_spans(item, "**").into_iter().next() {
        let span = span.trim();
        if span.split_whitespace().count() >= 2 {
            let key = normalize(span);
            if !key.is_empty() {
                return key;
            }
        }
    }
    content_words(&normalize(item))
        .into_iter()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ")
}
fn looks_like_capability_name(span: &str) -> bool {
    let s = span.trim();
    if s.is_empty() {
        return false;
    }
    if s.contains(", ") || s.contains("; ") || s.contains(". ") {
        return false;
    }
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.is_empty() || words.len() > 8 {
        return false;
    }
    let first = words[0];
    if first
        .chars()
        .next()
        .map(|c| c.is_lowercase())
        .unwrap_or(false)
        && matches!(
            first.to_lowercase().as_str(),
            "and" | "or" | "but" | "with" | "for" | "after" | "at" | "by" | "of" | "to"
        )
    {
        return false;
    }
    let letters: Vec<char> = s.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return false;
    }
    let upper = letters.iter().filter(|c| c.is_uppercase()).count();
    if upper * 5 > letters.len() * 4 {
        return false; // more than 80% uppercase: shouting, not naming
    }
    true
}

/// A backtick span that looks like a code identifier rather than prose:
/// no spaces, and either a path separator, underscore, dot, or CamelCase.
fn looks_like_identifier(span: &str) -> bool {
    let s = span.trim();
    if s.is_empty() || s.contains(' ') || s.chars().count() > 80 {
        return false;
    }
    s.contains("::")
        || s.contains('_')
        || s.contains('.')
        || (s.chars().any(|c| c.is_uppercase()) && s.chars().any(|c| c.is_lowercase()))
}
