// SPDX-License-Identifier: MIT OR Apache-2.0
//! A project's configuration as data: `.analysant/project.toml` (SPEC-STEP1 2.5 item 5, section 6 slice 4).
//!
//! Everything that names one project's conventions lives here, never in an engine crate: the note-to-row
//! ownership conventions, the project's own pairing stopwords, and which alias and concept files to load
//! (paths are relative to the project root, listed lowest precedence first).
//!
//! ```toml
//! [corpus]
//! aliases  = ["aliases.txt"]          # flat alias layer: `a | b | c`, first canonical
//! concepts = ["senses.concepts"]      # concept layers with sense tables
//! plan_files = ["ROADMAP.md"]         # tracked documents that are the plan, not the corpus
//!
//! [ownership]
//! row_word        = "row"
//! row_start       = ". **"
//! citation_prefix = "notes/"
//!
//! [contradiction]
//! pairing_stopwords = ["acme"]
//!
//! [code]
//! dir = "."                           # the git work tree whose Rust files the doc-vs-code lens reads
//! ```
//!
//! An unknown table or key is an error: a misspelled key would otherwise be silently ignored, and a
//! silently ignored convention changes what the engine reports.

use crate::ownership::OwnershipConfig;
use std::fmt;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectConfig {
    /// Alias files, relative to the project root.
    pub aliases: Vec<String>,
    /// Plan documents (tracked files, relative to the project root): their lines are what the untasked check searches, and they are
    /// not themselves part of the corpus.
    pub plan: Vec<String>,
    /// Concept-layer files, relative to the project root, lowest precedence first.
    pub concepts: Vec<String>,
    /// `None` when the project declares no ownership convention (ownership is then not applied).
    pub ownership: Option<OwnershipConfig>,
    pub pairing_stopwords: Vec<String>,
    /// The code tree for the doc-vs-code lens, relative to the project root; `None` when the project names none
    /// (the lens then runs only if `--code` is given).
    pub code_dir: Option<String>,
    /// `[untasked] skip_names = true`: the untasked check sets aside Name-kind claims (bare identifiers, labels, crate names), which state no
    /// work. Off by default: the Gate A oracle runs (v4, v5) include Name-claim findings; this is a Gate B tuning option.
    pub untasked_skip_names: bool,
    /// `[untasked] only_kinds = ["directive", ...]`: only claims of these kinds are checked against the plan; empty means every kind (the
    /// default, which the Gate A oracle runs rely on). Kind names: directive, assertion, question, decision, definition, proposal,
    /// report, name, description, unclassified.
    pub untasked_only_kinds: Vec<analysant_core::Kind>,
    /// `[contradiction] skip_names = true`: Name-kind claims are not statements the contradiction lens pairs. Off by default (Gate A).
    pub contradiction_skip_names: bool,
    /// `[contradiction] require_denied_token = true`: a denial pairs only with a positive statement that carries the denied word. Off by default.
    pub contradiction_require_denied_token: bool,
    /// `[contradiction] generic_df_percent = N`: a shared word in more than N% of the corpus's files does not count toward a pair's strength.
    pub contradiction_generic_df_percent: Option<u32>,
    /// `[contradiction] min_shared_tokens = N`: a pair needs at least N shared words that are not generic. Off by default.
    pub contradiction_min_shared_tokens: Option<usize>,
}

/// A project file that could not be read as configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectError(pub String);

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "project.toml: {}", self.0)
    }
}

impl std::error::Error for ProjectError {}

fn err<T>(msg: impl Into<String>) -> Result<T, ProjectError> {
    Err(ProjectError(msg.into()))
}

fn only_keys(table: &toml::Table, what: &str, allowed: &[&str]) -> Result<(), ProjectError> {
    match table.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(k) => err(format!("unknown key `{k}` in {what}")),
        None => Ok(()),
    }
}

fn strings(table: &toml::Table, key: &str, what: &str) -> Result<Vec<String>, ProjectError> {
    let Some(v) = table.get(key) else {
        return Ok(Vec::new());
    };
    let Some(items) = v.as_array() else {
        return err(format!("`{key}` in {what} must be an array of strings"));
    };
    items
        .iter()
        .map(|i| match i.as_str() {
            Some(s) => Ok(s.to_string()),
            None => err(format!("`{key}` in {what} must be an array of strings")),
        })
        .collect()
}

fn subtable<'a>(
    root: &'a toml::Table,
    name: &str,
) -> Result<Option<&'a toml::Table>, ProjectError> {
    match root.get(name) {
        None => Ok(None),
        Some(v) => match v.as_table() {
            Some(t) => Ok(Some(t)),
            None => err(format!("`{name}` must be a table")),
        },
    }
}

/// The claim kind a config name stands for: the kind's own variant name, lower-cased (no word list lives in the engine).
fn kind_named(name: &str) -> Result<analysant_core::Kind, ProjectError> {
    use analysant_core::Kind;
    [
        Kind::Unclassified,
        Kind::Directive,
        Kind::Assertion,
        Kind::Question,
        Kind::Decision,
        Kind::Definition,
        Kind::Proposal,
        Kind::Report,
        Kind::Name,
        Kind::Description,
    ]
    .into_iter()
    .find(|k| format!("{k:?}").eq_ignore_ascii_case(name))
    .map_or_else(
        || {
            err(format!(
                "unknown claim kind `{name}` in [untasked] only_kinds"
            ))
        },
        Ok,
    )
}

impl ProjectConfig {
    pub fn parse(text: &str) -> Result<Self, ProjectError> {
        let root: toml::Table = text
            .parse()
            .map_err(|e: toml::de::Error| ProjectError(e.to_string()))?;
        only_keys(
            &root,
            "the file",
            &["corpus", "ownership", "contradiction", "code", "untasked"],
        )?;
        let mut cfg = Self::default();
        if let Some(t) = subtable(&root, "corpus")? {
            only_keys(t, "[corpus]", &["aliases", "concepts", "plan_files"])?;
            cfg.aliases = strings(t, "aliases", "[corpus]")?;
            cfg.concepts = strings(t, "concepts", "[corpus]")?;
            cfg.plan = strings(t, "plan_files", "[corpus]")?;
        }
        if let Some(t) = subtable(&root, "ownership")? {
            only_keys(
                t,
                "[ownership]",
                &["row_word", "row_start", "citation_prefix"],
            )?;
            let get = |k: &str| match t.get(k).and_then(toml::Value::as_str) {
                Some(s) => Ok(s.to_string()),
                None => err(format!("[ownership] needs a string `{k}`")),
            };
            cfg.ownership = Some(OwnershipConfig {
                row_word: get("row_word")?,
                row_start: get("row_start")?,
                citation_prefix: get("citation_prefix")?,
            });
        }
        if let Some(t) = subtable(&root, "contradiction")? {
            only_keys(
                t,
                "[contradiction]",
                &[
                    "pairing_stopwords",
                    "skip_names",
                    "require_denied_token",
                    "generic_df_percent",
                    "min_shared_tokens",
                ],
            )?;
            cfg.pairing_stopwords = strings(t, "pairing_stopwords", "[contradiction]")?;
            cfg.contradiction_skip_names = match t.get("skip_names") {
                Some(v) => match v.as_bool() {
                    Some(b) => b,
                    None => return err("`skip_names` in [contradiction] must be true or false"),
                },
                None => false,
            };
            cfg.contradiction_require_denied_token = match t.get("require_denied_token") {
                Some(v) => match v.as_bool() {
                    Some(b) => b,
                    None => {
                        return err(
                            "`require_denied_token` in [contradiction] must be true or false",
                        );
                    }
                },
                None => false,
            };
            cfg.contradiction_min_shared_tokens = match t.get("min_shared_tokens") {
                None => None,
                Some(v) => match v.as_integer().and_then(|n| usize::try_from(n).ok()) {
                    Some(n) => Some(n),
                    None => {
                        return err(
                            "`min_shared_tokens` in [contradiction] must be a whole number",
                        );
                    }
                },
            };
            cfg.contradiction_generic_df_percent = match t.get("generic_df_percent") {
                None => None,
                Some(v) => match v.as_integer().and_then(|n| u32::try_from(n).ok()) {
                    Some(n) if n <= 100 => Some(n),
                    _ => {
                        return err(
                            "`generic_df_percent` in [contradiction] must be a whole number from 0 to 100",
                        );
                    }
                },
            };
        }
        if let Some(t) = subtable(&root, "untasked")? {
            only_keys(t, "[untasked]", &["skip_names", "only_kinds"])?;
            cfg.untasked_only_kinds = strings(t, "only_kinds", "[untasked]")?
                .iter()
                .map(|n| kind_named(n))
                .collect::<Result<_, _>>()?;
            cfg.untasked_skip_names = match t.get("skip_names") {
                Some(v) => match v.as_bool() {
                    Some(b) => b,
                    None => return err("`skip_names` in [untasked] must be true or false"),
                },
                None => false,
            };
        }
        if let Some(t) = subtable(&root, "code")? {
            only_keys(t, "[code]", &["dir"])?;
            cfg.code_dir = match t.get("dir") {
                None => None,
                Some(v) => match v.as_str() {
                    Some(s) => Some(s.to_string()),
                    None => return err("`dir` in [code] must be a string"),
                },
            };
        }
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn contradiction_skip_names_is_off_by_default_and_must_be_a_bool() {
        assert!(!ProjectConfig::parse("").unwrap().contradiction_skip_names);
        assert!(
            ProjectConfig::parse(
                "[contradiction]
skip_names = true
"
            )
            .unwrap()
            .contradiction_skip_names
        );
        assert!(
            ProjectConfig::parse(
                "[contradiction]
skip_names = 1
"
            )
            .is_err()
        );
    }

    #[test]
    fn contradiction_require_denied_token_is_off_by_default_and_must_be_a_bool() {
        assert!(
            !ProjectConfig::parse("")
                .unwrap()
                .contradiction_require_denied_token
        );
        assert!(
            ProjectConfig::parse(
                "[contradiction]
require_denied_token = true
"
            )
            .unwrap()
            .contradiction_require_denied_token
        );
        assert!(
            ProjectConfig::parse(
                "[contradiction]
require_denied_token = 2
"
            )
            .is_err()
        );
    }

    #[test]
    fn contradiction_generic_df_percent_is_off_by_default_and_must_be_0_to_100() {
        assert_eq!(
            ProjectConfig::parse("")
                .unwrap()
                .contradiction_generic_df_percent,
            None
        );
        assert_eq!(
            ProjectConfig::parse(
                "[contradiction]
generic_df_percent = 25
"
            )
            .unwrap()
            .contradiction_generic_df_percent,
            Some(25)
        );
        for bad in ["101", "-1", "\"x\"", "2.5"] {
            let t = format!(
                "[contradiction]
generic_df_percent = {bad}
"
            );
            assert!(ProjectConfig::parse(&t).is_err(), "{bad}");
        }
    }

    #[test]
    fn untasked_only_kinds_is_empty_by_default_and_names_real_kinds() {
        use analysant_core::Kind;
        assert!(
            ProjectConfig::parse("")
                .unwrap()
                .untasked_only_kinds
                .is_empty()
        );
        let c = ProjectConfig::parse(
            "[untasked]
only_kinds = [\"directive\", \"proposal\"]
",
        )
        .unwrap();
        assert_eq!(c.untasked_only_kinds, [Kind::Directive, Kind::Proposal]);
        assert!(
            ProjectConfig::parse(
                "[untasked]
only_kinds = [\"order\"]
"
            )
            .is_err()
        );
        assert!(
            ProjectConfig::parse(
                "[untasked]
only_kinds = \"directive\"
"
            )
            .is_err()
        );
    }

    #[test]
    fn contradiction_min_shared_tokens_is_off_by_default_and_must_be_a_number() {
        assert_eq!(
            ProjectConfig::parse("")
                .unwrap()
                .contradiction_min_shared_tokens,
            None
        );
        assert_eq!(
            ProjectConfig::parse(
                "[contradiction]
min_shared_tokens = 2
"
            )
            .unwrap()
            .contradiction_min_shared_tokens,
            Some(2)
        );
        assert!(
            ProjectConfig::parse(
                "[contradiction]
min_shared_tokens = -1
"
            )
            .is_err()
        );
        assert!(
            ProjectConfig::parse(
                "[contradiction]
min_shared_tokens = \"2\"
"
            )
            .is_err()
        );
    }

    #[test]
    fn untasked_skip_names_is_off_by_default_and_must_be_a_bool() {
        assert!(!ProjectConfig::parse("").unwrap().untasked_skip_names);
        assert!(
            ProjectConfig::parse(
                "[untasked]
skip_names = true
"
            )
            .unwrap()
            .untasked_skip_names
        );
        assert!(
            ProjectConfig::parse(
                "[untasked]
skip_names = \"yes\"
"
            )
            .is_err()
        );
        assert!(
            ProjectConfig::parse(
                "[untasked]
skip = true
"
            )
            .is_err()
        );
    }

    use super::*;

    #[test]
    fn parses_every_table() {
        let c = ProjectConfig::parse(
            "[corpus]\naliases = [\"a.txt\"]\nconcepts = [\"c1\", \"c2\"]\nplan_files = [\"P.md\"]\n\
             [ownership]\nrow_word = \"row\"\nrow_start = \". **\"\ncitation_prefix = \"n/\"\n\
             [contradiction]\npairing_stopwords = [\"acme\"]\n[code]\ndir = \"src\"\n",
        )
        .expect("valid");
        assert_eq!(c.aliases, ["a.txt"]);
        assert_eq!(c.concepts, ["c1", "c2"]);
        assert_eq!(c.plan, ["P.md"]);
        assert_eq!(c.pairing_stopwords, ["acme"]);
        assert_eq!(c.code_dir.as_deref(), Some("src"));
        let o = c.ownership.expect("ownership");
        assert_eq!(
            (
                o.row_word.as_str(),
                o.row_start.as_str(),
                o.citation_prefix.as_str()
            ),
            ("row", ". **", "n/")
        );
        // An empty file is a valid, empty project.
        assert_eq!(
            ProjectConfig::parse("").expect("empty"),
            ProjectConfig::default()
        );
    }

    #[test]
    fn a_typo_or_a_wrong_type_is_an_error_not_a_silent_default() {
        for bad in [
            "[ownerhsip]\nrow_word = \"row\"\n",
            "[ownership]\nrow_word = \"row\"\nrow_start = \".\"\n",
            "[ownership]\nrow_word = \"row\"\nrow_start = \".\"\ncitation_prefix = \"n\"\nextra = 1\n",
            "[corpus]\naliases = \"a.txt\"\n",
            "[corpus]\nconcepts = [1]\n",
            "ownership = 3\n",
            "[code]\ndirs = [\"src\"]\n",
            "[code]\ndir = 3\n",
            "[codes]\ndir = \"src\"\n",
            "not toml at all [",
        ] {
            assert!(ProjectConfig::parse(bad).is_err(), "{bad}");
        }
    }
}
