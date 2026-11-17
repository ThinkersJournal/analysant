//! Slice 4: note-to-row ownership (genre handling).
//!
//! A design note that elaborates a ROADMAP row is not a source of
//! untasked work: its details are tasked via the row, even though the
//! row's one line does not contain their words, so token-subset
//! matching can never see them as tasked. Ownership is deterministic
//! and explicit — it is declared in the documents themselves, in one
//! of two ways:
//!
//! (a) The note's title names the row: "ROADMAP row 157", "Row 137
//!     step 6.3b". The row must exist in the ROADMAP (a note planning
//!     for a row that does not exist is still an untasked finding).
//! (b) A ROADMAP row cites the note's path: `design-notes/<file>`.
//!     The owner is the lowest-numbered citing row. The citing line
//!     need not be the row's own line — rows can span several lines —
//!     so the owner is the nearest preceding row-start line.
//!
//! A row-start line is `N. **` — a number, a period, and bold text.
//! The bold marker matters: the ROADMAP also contains numbered
//! sub-lists inside row bodies ("1. Add `StorageMap` to `Graph`."),
//! and a bare `N. ` would misattribute a citation to "row 1".
//!
//! Ownership is a file-level property, separate from a claim's
//! pragmatic role (what its text is doing). An owned file's claims
//! keep their roles; the report partitions absent findings by file
//! ownership instead. Owned claims that are already set aside stay
//! set aside (their role is the more specific reason); only claims
//! that would otherwise be absent findings move to the OWNED section.

/// The level-1 heading of a document, if it has one.
pub fn first_heading(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let title = line.strip_prefix("# ")?;
        let title = title.trim();
        if title.is_empty() {
            None
        } else {
            Some(title.to_string())
        }
    })
}

/// The ROADMAP row a note is owned by, if either ownership signal
/// fires. Returns the row number; the caller formats "row N".
pub fn owner_for(filename: &str, text: &str, roadmap_lines: &[String]) -> Option<usize> {
    // Signal (a): the title names a row that exists. A self-declared
    // plan for a row is checked against the row first, so a title and
    // a citation that disagree resolve to the title's row — in this
    // corpus they never disagree (verified against the real ROADMAP).
    if let Some(title) = first_heading(text) {
        if let Some(n) = title_row(&title) {
            if roadmap_lines.iter().any(|l| row_number(l) == Some(n)) {
                return Some(n);
            }
        }
    }
    // Signal (b): the lowest-numbered row whose text cites the note.
    let needle = format!("design-notes/{filename}");
    let mut current: Option<usize> = None;
    let mut best: Option<usize> = None;
    for line in roadmap_lines {
        if let Some(n) = row_number(line) {
            current = Some(n);
        }
        if line.contains(&needle) {
            if let Some(n) = current {
                best = Some(best.map_or(n, |b: usize| b.min(n)));
            }
        }
    }
    best
}

/// The row a title names ("ROADMAP row 157", "Row 137 step 6.3b"),
/// if it names one. The word "row" must be followed by digits — a
/// bare "row" in prose ("the row below") names nothing.
fn title_row(title: &str) -> Option<usize> {
    let words: Vec<&str> = title
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    for (i, word) in words.iter().enumerate() {
        let lower = word.to_lowercase();
        if lower == "row" {
            if let Some(next) = words.get(i + 1) {
                if let Ok(n) = next.parse::<usize>() {
                    return Some(n);
                }
            }
        } else if let Some(rest) = lower.strip_prefix("row") {
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(n) = rest.parse::<usize>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// A ROADMAP row-start line: `N. **` — a number, a period, a space,
/// and bold text. The bold marker is load-bearing (see the module
/// docs): numbered sub-lists inside row bodies are not rows.
fn row_number(line: &str) -> Option<usize> {
    let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = &line[digits.len()..];
    if rest.starts_with(". **") {
        digits.parse().ok()
    } else {
        None
    }
}
