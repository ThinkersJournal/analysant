// SPDX-License-Identifier: MIT OR Apache-2.0
//! Helpers the Gate A tests share: the reference project's files, and its run files' lines.
#![allow(dead_code)]

use analysant::Corpus;
use analysant_core::{Form, Kind};
use std::fs;
use std::path::PathBuf;

pub fn reference() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/pip-spike-2026-11-16")
}

pub fn read(p: PathBuf) -> String {
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

pub fn run_file(name: &str) -> String {
    read(reference().join("runs").join(name)).replace('\r', "")
}

pub fn section(text: &str, heading: &str) -> Vec<String> {
    let mut lines = text.lines().skip_while(|l| !l.starts_with(heading));
    assert!(lines.next().is_some(), "{heading} not in the run file");
    lines
        .take_while(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn cat(form: &Form) -> usize {
    match form {
        Form::ListItem => 0,
        Form::TableCell | Form::Bold => 1,
        Form::CodeSpan => 2,
        other => panic!("no such form in this corpus: {other:?}"),
    }
}

pub fn finding_line(corpus: &Corpus, idx: usize) -> String {
    let claims: Vec<_> = corpus.claims().collect();
    let (doc, c) = claims[idx];
    format!(
        "  {}:{}  [{}] {}",
        doc.path,
        c.unit.source.span.line_start,
        doc.section_path(c).join(" > "),
        c.text
    )
}

pub fn set_aside_line(corpus: &Corpus, idx: usize) -> String {
    let claims: Vec<_> = corpus.claims().collect();
    let (doc, c) = claims[idx];
    let what = if c.form == Form::ListItem {
        "Directive"
    } else {
        "Named"
    };
    let role = match c.kind {
        Kind::Description => "Description",
        Kind::Report => "StatusNote",
        other => panic!("not set aside: {other:?}"),
    };
    format!(
        "  {}:{}  [{what} / {role}] [{}] {}",
        doc.path,
        c.unit.source.span.line_start,
        doc.section_path(c).join(" > "),
        c.text
    )
}
