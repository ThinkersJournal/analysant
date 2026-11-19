// SPDX-License-Identifier: MIT OR Apache-2.0
//! Comment text of Rust source, by line. `syn` drops plain comments, so this is a small lexer: it skips
//! string, raw-string, byte-string and character literals (a `//` inside a literal is not a comment) and
//! reports every comment's text with the line it starts on.

/// `(line, text)` for each comment, line numbers starting at 1. A block comment is reported per line.
pub(crate) fn comment_lines(src: &str) -> Vec<(u32, String)> {
    let b: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1u32;
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied();
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c == '/' && next == Some('/') {
            let start = i;
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
            out.push((line, b[start..i].iter().collect()));
        } else if c == '/' && next == Some('*') {
            let mut depth = 1;
            i += 2;
            let mut cur = String::new();
            let mut cur_line = line;
            while i < b.len() && depth > 0 {
                if b[i] == '/' && b.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if b[i] == '*' && b.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                } else if b[i] == '\n' {
                    out.push((cur_line, std::mem::take(&mut cur)));
                    line += 1;
                    cur_line = line;
                    i += 1;
                } else {
                    cur.push(b[i]);
                    i += 1;
                }
            }
            out.push((cur_line, cur));
        } else if c == '"' {
            i = skip_string(&b, i + 1, &mut line);
        } else if (c == 'r' || (c == 'b' && next == Some('r'))) && is_raw_start(&b, i) {
            i = skip_raw(&b, i, &mut line);
        } else if c == 'b' && next == Some('"') {
            i = skip_string(&b, i + 2, &mut line);
        } else if c == '\'' {
            i = skip_char_or_lifetime(&b, i);
        } else {
            i += 1;
        }
    }
    out
}

fn skip_string(b: &[char], mut i: usize, line: &mut u32) -> usize {
    while i < b.len() {
        match b[i] {
            '\\' => i += 2,
            '"' => return i + 1,
            '\n' => {
                *line += 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    i
}

/// `r"`, `r#"`, `br"`, `br#"` at `i`, and the `r` is not the tail of a longer identifier.
fn is_raw_start(b: &[char], i: usize) -> bool {
    if i > 0 && (b[i - 1].is_alphanumeric() || b[i - 1] == '_') {
        return false;
    }
    let mut j = i;
    if b[j] == 'b' {
        j += 1;
    }
    if b.get(j) != Some(&'r') {
        return false;
    }
    j += 1;
    while b.get(j) == Some(&'#') {
        j += 1;
    }
    b.get(j) == Some(&'"')
}

fn skip_raw(b: &[char], i: usize, line: &mut u32) -> usize {
    let mut j = i;
    if b[j] == 'b' {
        j += 1;
    }
    j += 1;
    let mut hashes = 0;
    while b.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    j += 1;
    while j < b.len() {
        if b[j] == '\n' {
            *line += 1;
        }
        if b[j] == '"' && (0..hashes).all(|k| b.get(j + 1 + k) == Some(&'#')) {
            return j + 1 + hashes;
        }
        j += 1;
    }
    j
}

/// A `'` starts a character literal (`'x'`, `'\n'`) or a lifetime (`'a`). Only the first is skipped.
fn skip_char_or_lifetime(b: &[char], i: usize) -> usize {
    if b.get(i + 1) == Some(&'\\') {
        let mut j = i + 2;
        while j < b.len() && b[j] != '\'' {
            j += 1;
        }
        return j + 1;
    }
    if b.get(i + 2) == Some(&'\'') {
        return i + 3;
    }
    i + 1
}

/// Words of a comment: maximal runs of alphanumerics and `_`, two characters or longer.
pub(crate) fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| w.chars().count() >= 2)
}
