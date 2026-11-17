//! `doc-spike`: run the directed-but-untasked check over real docs.
//!
//! Usage: `doc-spike <architecture-dir> <roadmap-file> [aliases-file] [code-dir]`
//! Reads every `.md` file in the architecture directory (non-recursive),
//! extracts claims, and reports which named capabilities and directives
//! have no matching ROADMAP row. Prints a summary plus every Absent
//! finding, grouped by kind (directives, named capabilities, code
//! identifiers); Mentioned findings are counted, not listed. Claims
//! from a design note owned by a ROADMAP row (slice 4: the note
//! elaborates a row that tasks the work) are reported separately under
//! OWNED BY A ROADMAP ROW, not as findings. The aliases
//! file (default `aliases.txt` next to the binary's working directory,
//! if present) holds synonym groups, one per line, phrases separated
//! by `|`; the first phrase is canonical.
//!
//! Slice 6: the optional code directory holds a Rust source tree; the
//! doc-vs-code lens then checks each absent finding against the code
//! (code is ground truth for whether a documented capability exists)
//! and prints a DOC VS CODE section plus summary counts. The lens is
//! additive — it changes no directed-but-untasked verdict and no
//! contradiction, so the v8 sections are identical with or without it.

use doc_analysis_spike::claim::{ClaimKind, PragmaticRole};
use doc_analysis_spike::contradiction::{
    find_contradictions, statements_from_claims, statements_from_headings, StatementSource,
};
use doc_analysis_spike::normalize::{parse_aliases, AliasGroup};
use doc_analysis_spike::ownership::owner_for;
use doc_analysis_spike::pragmatic::is_reportable;
use doc_analysis_spike::rust_code::{self, CodeStatus};
use doc_analysis_spike::{check, parse};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args.len() > 5 {
        eprintln!("usage: doc-spike <architecture-dir> <roadmap-file> [aliases-file] [code-dir]");
        std::process::exit(2);
    }
    let arch_dir = &args[1];
    let roadmap_path = &args[2];
    let aliases: Vec<AliasGroup> = args
        .get(3)
        .map(|p| fs::read_to_string(p).unwrap_or_else(|e| panic!("cannot read {p}: {e}")))
        .map(|text| parse_aliases(&text))
        .unwrap_or_default();

    let mut entries: Vec<_> = fs::read_dir(arch_dir)
        .unwrap_or_else(|e| panic!("cannot read {arch_dir}: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    entries.sort();

    let mut claims = Vec::new();
    // Heading statements for the contradiction lens (slice 5) are
    // built in the same pass that reads each file, from the text
    // already in hand; claim statements are built after the loop,
    // once all claims exist.
    let mut heading_statements = Vec::new();
    // The ROADMAP is read before the files: note-to-row ownership
    // (slice 4) is decided per file, from the file's title and the
    // rows, before any claim is extracted.
    let roadmap_text = fs::read_to_string(roadmap_path)
        .unwrap_or_else(|e| panic!("cannot read {roadmap_path}: {e}"));
    let roadmap_lines: Vec<String> = roadmap_text.lines().map(|l| l.to_string()).collect();
    let mut owners: HashMap<String, usize> = HashMap::new();
    for path in &entries {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if let Some(row) = owner_for(&name, &text, &roadmap_lines) {
            owners.insert(name.clone(), row);
        }
        heading_statements.extend(statements_from_headings(&name, &text));
        claims.extend(parse::extract_claims(&name, &text));
    }

    let findings = check::directed_but_untasked(&claims, &roadmap_lines, &aliases);
    // The pragmatic lens (slice 1) sets aside claims whose text does not
    // do what its kind claims: descriptions and status notes wearing a
    // directive's shape, sentences and rules wearing a capability name.
    // They are counted and listed separately, not silently dropped, so
    // the lens itself stays checkable.
    //
    // Slice 4 adds a second partition: an absent finding from a file
    // owned by a ROADMAP row is tasked via that row, so it is neither
    // a finding nor set aside — it is reported under OWNED BY A
    // ROADMAP ROW. Only claims that would otherwise be findings move
    // there; an owned file's descriptions and status notes stay set
    // aside, where their role is the more specific reason.
    let is_owned = |f: &check::Finding| owners.contains_key(&f.claim.source_file);
    let owned: Vec<_> = findings
        .iter()
        .filter(|f| f.kind == check::FindingKind::Absent && is_reportable(&f.claim) && is_owned(f))
        .collect();
    let absent: Vec<_> = findings
        .iter()
        .filter(|f| f.kind == check::FindingKind::Absent && is_reportable(&f.claim) && !is_owned(f))
        .collect();
    let set_aside: Vec<_> = findings
        .iter()
        .filter(|f| f.kind == check::FindingKind::Absent && !is_reportable(&f.claim))
        .collect();
    let mentioned = findings
        .iter()
        .filter(|f| f.kind == check::FindingKind::Mentioned)
        .count();
    let count = |kind: ClaimKind| claims.iter().filter(|c| c.kind == kind).count();
    let absent_of = |kind: ClaimKind| absent.iter().filter(|f| f.claim.kind == kind).count();
    let set_aside_of = |kind: ClaimKind, role: PragmaticRole| {
        set_aside
            .iter()
            .filter(|f| f.claim.kind == kind && f.claim.role == role)
            .count()
    };

    println!(
        "claims extracted: {} ({} named, {} directives, {} identifiers)",
        claims.len(),
        count(ClaimKind::Named),
        count(ClaimKind::Directive),
        count(ClaimKind::Identifier)
    );
    println!("roadmap lines: {}", roadmap_lines.len());
    println!(
        "findings: {} absent ({} directives, {} named, {} identifiers), {} mentioned",
        absent.len(),
        absent_of(ClaimKind::Directive),
        absent_of(ClaimKind::Named),
        absent_of(ClaimKind::Identifier),
        mentioned
    );
    println!(
        "set aside by the pragmatic lens: {} (directives as description {}, as status {}; named as description {}, as status {})",
        set_aside.len(),
        set_aside_of(ClaimKind::Directive, PragmaticRole::Description),
        set_aside_of(ClaimKind::Directive, PragmaticRole::StatusNote),
        set_aside_of(ClaimKind::Named, PragmaticRole::Description),
        set_aside_of(ClaimKind::Named, PragmaticRole::StatusNote),
    );
    let owned_of = |kind: ClaimKind| owned.iter().filter(|f| f.claim.kind == kind).count();
    println!(
        "owned by a ROADMAP row: {} ({} directives, {} named, {} identifiers)",
        owned.len(),
        owned_of(ClaimKind::Directive),
        owned_of(ClaimKind::Named),
        owned_of(ClaimKind::Identifier),
    );
    // Slice 5: the doc-vs-doc contradiction lens. Statements come from
    // headings (a document's own position) and from extracted claims;
    // the lens is additive — it reads the same claims and headings and
    // changes no directed-but-untasked verdict.
    let mut statements = statements_from_claims(&claims);
    statements.extend(heading_statements);
    let contradictions = find_contradictions(&statements);
    println!("contradictions: {}", contradictions.len());
    // Slice 6: the doc-vs-code lens, over the absent findings only
    // (those are the ones whose build status is unknown). It reads
    // the same claims and changes no verdict above.
    let code_index = args.get(4).map(|d| rust_code::scan_code_dir(Path::new(d)));
    let doc_code: Vec<(check::Finding, rust_code::DocCodeVerdict)> = match &code_index {
        Some(index) => absent
            .iter()
            .map(|f| {
                (
                    (*f).clone(),
                    rust_code::doc_code_status(&f.claim, index, &aliases),
                )
            })
            .collect(),
        None => Vec::new(),
    };
    if let Some(index) = &code_index {
        let of = |status: CodeStatus| doc_code.iter().filter(|(_, v)| v.status == status).count();
        let of_kind = |kind: &ClaimKind, status: CodeStatus| {
            doc_code
                .iter()
                .filter(|(f, v)| f.claim.kind == *kind && v.status == status)
                .count()
        };
        println!(
            "code items scanned: {} definitions, {} files",
            index.items.len(),
            index.files_scanned
        );
        println!(
            "doc vs code (absent findings): {} built, {} stub, {} test-only, {} absent",
            of(CodeStatus::Built),
            of(CodeStatus::Stub),
            of(CodeStatus::TestOnly),
            of(CodeStatus::Absent),
        );
        for (kind, label) in [
            (ClaimKind::Directive, "directives"),
            (ClaimKind::Named, "named"),
            (ClaimKind::Identifier, "identifiers"),
        ] {
            println!(
                "  {}: {} built, {} stub, {} test-only, {} absent",
                label,
                of_kind(&kind, CodeStatus::Built),
                of_kind(&kind, CodeStatus::Stub),
                of_kind(&kind, CodeStatus::TestOnly),
                of_kind(&kind, CodeStatus::Absent),
            );
        }
    }
    println!();
    for (kind, label) in [
        (ClaimKind::Directive, "ABSENT DIRECTIVES"),
        (ClaimKind::Named, "ABSENT NAMED CAPABILITIES"),
        (ClaimKind::Identifier, "ABSENT CODE IDENTIFIERS"),
    ] {
        println!("{label} — named in architecture, in no ROADMAP row:");
        for f in absent.iter().filter(|f| f.claim.kind == kind) {
            println!("  {}  [{}] {}", f.claim.id, f.claim.section, f.claim.text);
        }
        println!();
    }
    println!("SET ASIDE BY THE PRAGMATIC LENS — absent from the ROADMAP but not a directive or a capability name:");
    for f in &set_aside {
        println!(
            "  {}  [{:?} / {:?}] [{}] {}",
            f.claim.id, f.claim.kind, f.claim.role, f.claim.section, f.claim.text
        );
    }
    println!();
    println!("OWNED BY A ROADMAP ROW — the note elaborates a row that tasks the work, so these are not untasked:");
    for f in &owned {
        println!(
            "  {}  [row {}] [{}] {}",
            f.claim.id, owners[&f.claim.source_file], f.claim.section, f.claim.text
        );
    }
    println!();
    println!("CONTRADICTIONS — two documents disagree about the same capability (one denies it, one asserts it):");
    for c in &contradictions {
        let neg_rule = c.negated_by.as_deref().unwrap_or("negated");
        let pos_source = match &c.positive.source {
            StatementSource::Heading => "heading",
            StatementSource::Claim(ClaimKind::Directive) => "directive claim",
            StatementSource::Claim(ClaimKind::Named) => "named claim",
            StatementSource::Claim(ClaimKind::Identifier) => "identifier claim",
            StatementSource::Claim(ClaimKind::Assertion) => "assertion claim",
        };
        println!(
            "  NEG {}  [{}] \"{}\" — negated by \"{}\"",
            c.negated.id, c.negated.section, c.negated.text, neg_rule
        );
        println!(
            "  POS {}  [{}] \"{}\" — {} (no negation marker)",
            c.positive.id, c.positive.section, c.positive.text, pos_source
        );
        println!("  shared: {}", c.shared.join(", "));
        println!();
    }
    if !doc_code.is_empty() {
        println!("DOC VS CODE — the absent findings checked against the Rust source (code is ground truth for whether a documented capability exists):");
        for (f, v) in &doc_code {
            let status = match v.status {
                CodeStatus::Built => "BUILT",
                CodeStatus::Stub => "STUB",
                CodeStatus::TestOnly => "TEST-ONLY",
                CodeStatus::Absent => "ABSENT",
            };
            match &v.matched {
                Some(m) => println!(
                    "  {}  [{}] {} — {} via {} ({:?}) at {}:{}",
                    f.claim.id,
                    f.claim.section,
                    f.claim.text,
                    status,
                    m.name,
                    m.kind,
                    m.file,
                    m.line
                ),
                None => {
                    if v.comments_only {
                        println!(
                            "  {}  [{}] {} — ABSENT (words appear only in comments)",
                            f.claim.id, f.claim.section, f.claim.text
                        );
                    } else {
                        println!(
                            "  {}  [{}] {} — ABSENT",
                            f.claim.id, f.claim.section, f.claim.text
                        );
                    }
                }
            }
        }
        println!();
    }
}
