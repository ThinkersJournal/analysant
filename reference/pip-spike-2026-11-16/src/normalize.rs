//! Deterministic normalization: the only matching machinery the spike uses.
//!
//! Lowercase, strip punctuation, collapse whitespace, drop a leading
//! article. No ML, no embeddings, no stemming. Matching is exact
//! token-sequence containment on the normalized form, so a finding is
//! always explainable: these tokens appear (or don't) in that row.

/// Normalize one string for matching.
pub fn normalize(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    for ch in lower.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else {
            out.push(' ');
        }
    }
    let mut words: Vec<&str> = out.split_whitespace().collect();
    if matches!(words.first(), Some(&"the") | Some(&"a") | Some(&"an")) {
        words.remove(0);
    }
    words.join(" ")
}

/// Split a normalized string into tokens.
pub fn tokens(normalized: &str) -> Vec<&str> {
    normalized.split_whitespace().collect()
}

/// True when every token of `key` appears in `line` (any order).
/// Both arguments must already be normalized. This is looser than
/// contiguous-sequence matching, on purpose: word order is not part of
/// a capability's identity ("DAG multi-path" is the same claim as
/// "multi-path DAG"). The finding still explains itself: these words
/// appear in that row, or they don't.
pub fn contains_token_subset(line: &str, key: &str) -> bool {
    let line_tokens = tokens(line);
    let key_tokens = tokens(key);
    if key_tokens.is_empty() {
        return false;
    }
    key_tokens.iter().all(|tok| line_tokens.contains(tok))
}

/// Small stopword list for directive keys: function words and temporal
/// deictics that never identify a capability ("today" dates a note,
/// it does not name a thing).
pub const STOPWORDS: &[&str] = &[
    "the", "a", "an", "as", "of", "to", "and", "or", "it", "is", "are", "was", "were", "be", "by",
    "for", "with", "on", "in", "at", "from", "that", "this", "its", "today",
];

/// The content words of a normalized string (stopwords removed).
pub fn content_words(normalized: &str) -> Vec<&str> {
    tokens(normalized)
        .into_iter()
        .filter(|w| !STOPWORDS.contains(w))
        .collect()
}

/// True when `key` appears as a contiguous token sequence in `line`.
/// Both arguments must already be normalized. Kept for tests and for
/// callers that want the stricter rule.
pub fn contains_token_sequence(line: &str, key: &str) -> bool {
    let line_tokens = tokens(line);
    let key_tokens = tokens(key);
    if key_tokens.is_empty() || key_tokens.len() > line_tokens.len() {
        return false;
    }
    line_tokens
        .windows(key_tokens.len())
        .any(|window| window == key_tokens.as_slice())
}

/// One alias group: several normalized phrases that name the same thing.
/// The first phrase is the canonical form; the rest are aliases.
/// Synonymy is data, not code: groups live in `aliases.txt`, one per line,
/// phrases separated by `|`.
#[derive(Debug, Clone)]
pub struct AliasGroup {
    pub canonical: String,
    pub aliases: Vec<String>,
}

/// Parse alias groups from text: one group per non-empty, non-comment line,
/// phrases separated by `|`, each phrase normalized. Lines starting with
/// `#` are comments.
pub fn parse_aliases(text: &str) -> Vec<AliasGroup> {
    let mut groups = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let phrases: Vec<String> = line
            .split('|')
            .map(normalize)
            .filter(|p| !p.is_empty())
            .collect();
        if phrases.len() >= 2 {
            groups.push(AliasGroup {
                canonical: phrases[0].clone(),
                aliases: phrases[1..].to_vec(),
            });
        }
    }
    groups
}

/// Canonicalize a normalized string: replace every alias phrase (as a
/// token subsequence) with its group's canonical phrase. The first
/// matching group wins per position; replacement runs once per group.
pub fn canonicalize(normalized: &str, groups: &[AliasGroup]) -> String {
    let mut toks: Vec<String> = tokens(normalized).iter().map(|t| t.to_string()).collect();
    for group in groups {
        for alias in &group.aliases {
            let alias_toks = tokens(alias);
            if alias_toks.is_empty() {
                continue;
            }
            let canon_toks = tokens(&group.canonical);
            let mut i = 0;
            while i + alias_toks.len() <= toks.len() {
                if toks[i..i + alias_toks.len()]
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    == alias_toks
                {
                    toks.splice(
                        i..i + alias_toks.len(),
                        canon_toks.iter().map(|s| s.to_string()),
                    );
                    i += canon_toks.len();
                } else {
                    i += 1;
                }
            }
        }
    }
    toks.join(" ")
}
