// SPDX-License-Identifier: MIT OR Apache-2.0
//! SPEC-STEP1 1.4 and 1.9: re-identification across edits and the ledger file. Every test names a
//! concrete before and after document and asserts the ids, revisions, events and texts that must
//! result; the controls say what must NOT change.

use analysant::extract_document;
use analysant::identity::{
    AMBIGUOUS_KIND, How, IdentityParams, Observation, Reidentified, ambiguity_findings,
    observations, reidentify,
};
use analysant::ledger_file::{
    Event, LedgerFileError, history, merge, parse, read_file, render, state, text_hash, write_file,
};
use analysant_core::{English, Ledger, Markdown, Revision, UnitId};

const PARAMS: IdentityParams = IdentityParams {
    edit_similarity_permille: 600,
};
const DOC: &str = "a.md";

const V1: &str = "# Plan\n\n## Intake\n\n\
- The intake service validates every request before queueing it.\n\
- The queue drains in arrival order.\n\n\
## Output\n\n\
- The writer flushes results to disk once per second.\n";

/// What the writing tool does for one document: extract, align to the ledger, append the proposed
/// events. Returns the proposal and the observations it was made from.
fn cycle(events: &mut Vec<Event>, md: &str) -> (Reidentified, Vec<Observation>) {
    let en = English::new();
    let mut scratch = Ledger::default();
    let doc = extract_document(&Markdown, &en, &mut scratch, DOC, md);
    let obs = observations(&en, &doc);
    let prior = state(events).unwrap();
    let r = reidentify(&en, DOC, &prior, &obs, PARAMS);
    events.extend(r.events.clone());
    (r, obs)
}

fn live_texts(events: &[Event]) -> Vec<(String, u32)> {
    let mut v: Vec<(String, u32)> = state(events)
        .unwrap()
        .into_iter()
        .filter(|s| !s.gone)
        .map(|s| (s.text, s.revision.0))
        .collect();
    v.sort();
    v
}

fn id_of(events: &[Event], text: &str) -> UnitId {
    state(events)
        .unwrap()
        .into_iter()
        .find(|s| s.text == text && !s.gone)
        .unwrap_or_else(|| panic!("no live unit with text {text:?}"))
        .unit
}

const INTAKE: &str = "The intake service validates every request before queueing it.";
const QUEUE: &str = "The queue drains in arrival order.";
const WRITER: &str = "The writer flushes results to disk once per second.";
const INTAKE_EDITED: &str =
    "The intake service validates every incoming request before queueing it.";

#[test]
fn the_first_extraction_mints_every_claim_once() {
    let mut ev = Vec::new();
    let (r, obs) = cycle(&mut ev, V1);
    assert_eq!(obs.len(), 3);
    assert_eq!(r.assignments.len(), 3);
    assert!(r.assignments.iter().all(|a| a.how == How::Minted));
    assert!(r.ambiguities.is_empty() && r.gone.is_empty());
    assert_eq!(ev.len(), 3);
    let ids: std::collections::BTreeSet<UnitId> = r.assignments.iter().map(|a| a.unit).collect();
    assert_eq!(ids.len(), 3, "three distinct ids");
    assert_eq!(
        live_texts(&ev),
        vec![
            (INTAKE.to_string(), 1),
            (QUEUE.to_string(), 1),
            (WRITER.to_string(), 1)
        ]
    );
}

#[test]
fn a_pure_line_shift_changes_no_identity_and_proposes_no_event() {
    let mut ev = Vec::new();
    cycle(&mut ev, V1);
    let before = (id_of(&ev, INTAKE), id_of(&ev, QUEUE), id_of(&ev, WRITER));
    // Control: the same claims, every one of them on a different line (a preface and blank lines).
    let shifted = format!("Preface paragraph.\n\n\n\n{V1}");
    let (r, _) = cycle(&mut ev, &shifted);
    assert!(
        r.events.is_empty(),
        "nothing changed but positions: {:?}",
        r.events
    );
    assert!(r.assignments.iter().all(|a| a.how == How::Exact));
    assert_eq!(
        before,
        (id_of(&ev, INTAKE), id_of(&ev, QUEUE), id_of(&ev, WRITER))
    );
    assert_eq!(ev.len(), 3, "the ledger did not grow");
}

#[test]
fn an_edited_sentence_keeps_its_id_and_the_old_text_stays_in_the_ledger() {
    let mut ev = Vec::new();
    cycle(&mut ev, V1);
    let id = id_of(&ev, INTAKE);
    let edited = V1.replace(
        "validates every request",
        "validates every incoming request",
    );
    let (r, _) = cycle(&mut ev, &edited);
    assert_eq!(
        r.events.len(),
        1,
        "only the edit is an event: {:?}",
        r.events
    );
    assert!(
        matches!(&r.events[0], Event::Revised { revision: 2, text, .. } if text == INTAKE_EDITED)
    );
    let edit = r.assignments.iter().find(|a| a.unit == id).unwrap();
    assert_eq!(
        edit.how,
        How::Edited {
            similarity_permille: 888
        }
    );
    assert_eq!(
        live_texts(&ev),
        vec![
            (INTAKE_EDITED.to_string(), 2),
            (QUEUE.to_string(), 1),
            (WRITER.to_string(), 1)
        ]
    );
    assert_eq!(
        history(&ev, id),
        vec![
            (Revision(1), INTAKE.to_string()),
            (Revision(2), INTAKE_EDITED.to_string())
        ]
    );
}

#[test]
fn a_rewritten_sentence_below_the_threshold_is_a_new_claim_and_the_old_one_is_gone() {
    let mut ev = Vec::new();
    cycle(&mut ev, V1);
    let id = id_of(&ev, WRITER);
    let rewritten = V1.replace(WRITER, "Results are persisted by a background thread.");
    let (r, _) = cycle(&mut ev, &rewritten);
    assert_eq!(r.gone, vec![id]);
    let s = state(&ev).unwrap();
    assert!(s.iter().find(|u| u.unit == id).unwrap().gone);
    assert!(r.assignments.iter().any(|a| a.how == How::Minted));
    assert_eq!(history(&ev, id), vec![(Revision(1), WRITER.to_string())]);
}

#[test]
fn a_moved_section_keeps_the_ids_of_its_claims() {
    let mut ev = Vec::new();
    cycle(&mut ev, V1);
    let id = id_of(&ev, WRITER);
    // The Output section now sits under a new top-level heading; its claim's text is unchanged.
    let moved = "# Plan\n\n## Intake\n\n\
- The intake service validates every request before queueing it.\n\
- The queue drains in arrival order.\n\n\
# Archive\n\n## Output\n\n\
- The writer flushes results to disk once per second.\n";
    let (r, _) = cycle(&mut ev, moved);
    assert_eq!(r.events.len(), 1);
    assert!(matches!(
        &r.events[0],
        Event::Moved { section_path, revision: 1, .. }
            if section_path == &vec!["Archive".to_string(), "Output".to_string()]
    ));
    assert_eq!(id_of(&ev, WRITER), id, "same id after the move");
    let s = state(&ev).unwrap();
    let u = s.iter().find(|u| u.unit == id).unwrap();
    assert_eq!(
        u.section_path,
        vec!["Archive".to_string(), "Output".to_string()]
    );
    assert_eq!(u.revision, Revision(1), "a move is not a revision");
}

#[test]
fn a_duplicated_sentence_is_ambiguous_and_nothing_is_decided() {
    let mut ev = Vec::new();
    cycle(&mut ev, V1);
    let id = id_of(&ev, QUEUE);
    let duplicated = V1.replace(&format!("- {QUEUE}\n"), &format!("- {QUEUE}\n- {QUEUE}\n"));
    let (r, obs) = cycle(&mut ev, &duplicated);
    assert_eq!(r.ambiguities.len(), 1);
    let a = &r.ambiguities[0];
    assert_eq!(a.old, vec![id]);
    assert_eq!(a.new.len(), 2, "both candidates are named");
    assert!(a.new.iter().all(|&j| obs[j].text == QUEUE));
    // Neither candidate got an id; the old unit is untouched, not gone; no event was proposed.
    assert!(
        r.assignments
            .iter()
            .all(|x| !a.new.contains(&x.observation))
    );
    assert!(r.gone.is_empty());
    assert!(r.events.is_empty(), "{:?}", r.events);
    assert_eq!(id_of(&ev, QUEUE), id);

    let en = English::new();
    let findings = ambiguity_findings(&en, &r, &state(&ev).unwrap(), &obs);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind.label(), AMBIGUOUS_KIND);
    assert_eq!(findings[0].claims.len(), 1);
    assert_eq!(findings[0].claims[0].id, id);
    assert_eq!(findings[0].search.matched, 1);
}

#[test]
fn deleting_one_of_two_identical_sentences_is_ambiguous_too() {
    let mut ev = Vec::new();
    let twice = V1.replace(&format!("- {QUEUE}\n"), &format!("- {QUEUE}\n- {QUEUE}\n"));
    let (first, _) = cycle(&mut ev, &twice);
    assert!(first.ambiguities.is_empty(), "first sight mints both");
    let (again, _) = cycle(&mut ev, &twice);
    assert!(again.ambiguities.is_empty(), "two and two pair in order");
    assert!(again.events.is_empty());
    let (r, _) = cycle(&mut ev, V1);
    assert_eq!(r.ambiguities.len(), 1);
    assert_eq!(r.ambiguities[0].old.len(), 2);
    assert_eq!(r.ambiguities[0].new.len(), 1);
    assert!(r.gone.is_empty(), "neither is marked gone on a guess");
}

#[test]
fn a_duplicate_in_another_section_is_told_apart_by_the_section() {
    let mut ev = Vec::new();
    cycle(&mut ev, V1);
    let id = id_of(&ev, QUEUE);
    let copied = V1.replace(
        &format!("- {WRITER}\n"),
        &format!("- {WRITER}\n- {QUEUE}\n"),
    );
    let (r, _) = cycle(&mut ev, &copied);
    assert!(r.ambiguities.is_empty());
    let kept = r.assignments.iter().find(|a| a.unit == id).unwrap();
    assert_eq!(
        kept.how,
        How::Exact,
        "the one still under Intake keeps the id"
    );
    assert_eq!(
        r.assignments
            .iter()
            .filter(|a| a.how == How::Minted)
            .count(),
        1
    );
}

#[test]
fn two_equally_similar_candidates_are_ambiguous_not_picked() {
    let two = "# Plan\n\n- Retries back off exponentially after failure.\n- Retries back off linearly after failure.\n";
    let one = "# Plan\n\n- Retries back off quickly after failure.\n";
    let mut ev = Vec::new();
    cycle(&mut ev, two);
    let (r, _) = cycle(&mut ev, one);
    assert_eq!(r.ambiguities.len(), 1);
    assert_eq!(r.ambiguities[0].old.len(), 2);
    assert_eq!(r.ambiguities[0].new.len(), 1);
    assert!(
        r.ambiguities[0].reason.contains("714/1000"),
        "{}",
        r.ambiguities[0].reason
    );
    assert!(r.gone.is_empty() && r.events.is_empty());
}

#[test]
fn a_deleted_sentence_goes_gone_and_a_rewritten_one_does_not_reuse_its_id() {
    let mut ev = Vec::new();
    cycle(&mut ev, V1);
    let id = id_of(&ev, WRITER);
    let without = V1.replace(&format!("- {WRITER}\n"), "");
    let (r, _) = cycle(&mut ev, &without);
    assert_eq!(r.gone, vec![id]);
    assert!(matches!(&r.events[0], Event::Gone { revision: 1, .. }));
    // The same words come back later: a new claim, a new id.
    let (r2, _) = cycle(&mut ev, V1);
    let back = r2
        .assignments
        .iter()
        .find(|a| a.how == How::Minted)
        .unwrap();
    assert_ne!(back.unit, id, "an id is never reused");
    assert_eq!(history(&ev, id), vec![(Revision(1), WRITER.to_string())]);
}

// ---- the ledger file ----

fn base() -> Vec<Event> {
    let mut ev = Vec::new();
    cycle(&mut ev, V1);
    ev
}

#[test]
fn the_file_round_trips_byte_for_byte_and_is_one_record_per_line() {
    let mut ev = base();
    cycle(
        &mut ev,
        &V1.replace(
            "validates every request",
            "validates every incoming request",
        ),
    );
    let text = render(&ev);
    assert_eq!(text.lines().count(), 4, "3 minted + 1 revised");
    assert!(text.ends_with('\n'));
    let back = parse(&text).unwrap();
    assert_eq!(render(&back), text);
    let s = |t: &str| text.lines().filter(|l| l.contains(t)).count();
    assert_eq!(
        (s("\"type\":\"minted\""), s("\"type\":\"revised\"")),
        (3, 1)
    );
}

#[test]
fn a_hand_edited_or_malformed_line_is_refused_with_its_line_number() {
    let text = render(&base());
    let tampered = text.replacen("flushes results", "flushes all results", 1);
    assert!(
        matches!(parse(&tampered), Err(LedgerFileError::BadTextHash { .. })),
        "text edited, hash not"
    );
    let rehashed = tampered.replacen(
        &text_hash(WRITER),
        &text_hash("The writer flushes all results to disk once per second."),
        1,
    );
    assert!(
        matches!(parse(&rehashed), Err(LedgerFileError::BadId { .. })),
        "text and hash edited, id not"
    );
    let broken = format!("{text}{{not json}}\n");
    assert!(matches!(
        parse(&broken),
        Err(LedgerFileError::Malformed(4, _))
    ));
    let extra = text.replacen("\"revision\":1", "\"revision\":1,\"colour\":\"red\"", 1);
    assert!(
        matches!(parse(&extra), Err(LedgerFileError::Malformed(1, _))),
        "unknown field"
    );
    // Control: the untouched text parses.
    assert_eq!(parse(&text).unwrap().len(), 3);
}

#[test]
fn an_event_for_a_unit_never_minted_is_an_orphan() {
    let mut ev = base();
    cycle(
        &mut ev,
        &V1.replace(WRITER, "Results are persisted by a background thread."),
    );
    let gone: Vec<Event> = ev
        .iter()
        .filter(|e| matches!(e, Event::Gone { .. }))
        .cloned()
        .collect();
    assert_eq!(gone.len(), 1);
    assert!(matches!(
        state(&gone),
        Err(LedgerFileError::OrphanEvent { .. })
    ));
    // Control: with its minting events the same ledger folds.
    assert!(state(&ev).is_ok());
}

#[test]
fn a_missing_file_is_an_empty_ledger_and_a_written_one_is_read_back() {
    let dir = std::env::temp_dir().join(format!("analysant-ledger-{}", std::process::id()));
    let path = dir.join(".analysant").join("ledger.jsonl");
    assert_eq!(read_file(&path).unwrap(), Vec::new());
    let ev = base();
    write_file(&path, &ev).unwrap();
    assert_eq!(read_file(&path).unwrap(), parse(&render(&ev)).unwrap());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), render(&ev));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn unit_ids_survive_the_text_form() {
    let id = id_of(&base(), WRITER);
    assert_eq!(UnitId::parse(&id.to_string()), Some(id));
    assert_eq!(UnitId::parse("short"), None);
    assert_eq!(
        UnitId::parse(&"Z".repeat(26)),
        None,
        "top digit out of range"
    );
    assert_eq!(
        UnitId::parse(&"1".repeat(26)),
        None,
        "1 is not in the alphabet"
    );
}

// ---- merge by union (D3) ----

/// Two branches from the same base: A rewords the intake sentence; B replaces the writer sentence
/// with a new one.
fn branches() -> (Vec<Event>, Vec<Event>) {
    let mut a = base();
    cycle(
        &mut a,
        &V1.replace(
            "validates every request",
            "validates every incoming request",
        ),
    );
    let mut b = base();
    let b_doc = V1.replace(
        &format!("- {WRITER}\n"),
        "- Failed jobs are retried three times.\n",
    );
    cycle(&mut b, &b_doc);
    (a, b)
}

#[test]
fn two_diverged_branches_merge_to_the_same_ledger_in_either_order() {
    let (a, b) = branches();
    let ab = merge(&a, &b).unwrap();
    let ba = merge(&b, &a).unwrap();
    assert_eq!(ab, ba);
    assert_eq!(render(&ab.events), render(&ba.events), "byte-identical");
    assert!(ab.conflicts.is_empty());
    // 3 shared minted + A's revised + B's gone + B's minted.
    assert_eq!(ab.events.len(), 6);
    assert_eq!(
        live_texts(&ab.events),
        vec![
            ("Failed jobs are retried three times.".to_string(), 1),
            (INTAKE_EDITED.to_string(), 2),
            (QUEUE.to_string(), 1),
        ]
    );
}

#[test]
fn merging_is_idempotent_and_a_repeated_event_id_is_kept_once() {
    let (a, b) = branches();
    let m = merge(&a, &b).unwrap();
    assert_eq!(merge(&m.events, &a).unwrap().events, m.events);
    assert_eq!(merge(&m.events, &m.events).unwrap().events, m.events);
    // A file with one line twice reads as the same ledger.
    let text = render(&a);
    let first = text.lines().next().unwrap();
    let doubled = format!("{text}{first}\n");
    assert_eq!(parse(&doubled).unwrap(), parse(&text).unwrap());
}

#[test]
fn the_same_new_sentence_minted_on_two_branches_is_one_event() {
    let added = format!("{V1}- Failed jobs are retried three times.\n");
    let mut a = base();
    cycle(&mut a, &added);
    let mut b = base();
    cycle(&mut b, &added);
    let m = merge(&a, &b).unwrap();
    assert_eq!(m.events.len(), 4, "3 shared + 1 minted once, not twice");
    assert_eq!(m.events, parse(&render(&a)).unwrap());
}

#[test]
fn two_branches_that_reword_one_claim_differently_are_a_conflict_and_nothing_is_lost() {
    let mut a = base();
    cycle(
        &mut a,
        &V1.replace(
            "validates every request",
            "validates every incoming request",
        ),
    );
    let mut b = base();
    cycle(
        &mut b,
        &V1.replace("validates every request", "validates every single request"),
    );
    let id = id_of(&base(), INTAKE);
    let ab = merge(&a, &b).unwrap();
    let ba = merge(&b, &a).unwrap();
    assert_eq!(ab, ba, "the same result in either order");
    assert_eq!(ab.conflicts.len(), 1);
    assert_eq!((ab.conflicts[0].unit, ab.conflicts[0].revision), (id, 2));
    assert_eq!(ab.conflicts[0].event_ids.len(), 2);
    let texts: Vec<String> = history(&ab.events, id)
        .into_iter()
        .map(|(_, t)| t)
        .collect();
    assert_eq!(texts.len(), 3, "the original and both rewordings are kept");
    assert!(texts.iter().any(|t| t.contains("incoming")));
    assert!(texts.iter().any(|t| t.contains("single")));
}
