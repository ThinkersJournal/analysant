// SPDX-License-Identifier: MIT OR Apache-2.0
//! The permille-calibration pilot's pair generator (pm-ants TASK 2026-11-21; `docs/PERMILLE-PILOT.md`).
//!
//! `identity::reidentify` links a reworded claim to its predecessor when their token-set similarity (a share of
//! 1000) reaches `edit_similarity_permille`, and the crate has no default. This tool produces the labelled set that
//! default needs: for consecutive git revisions of one tracked file it extracts the claims of both revisions, drops
//! every claim whose normal-form key is unchanged, and for each remaining NEW claim reports its best similarity to a
//! remaining OLD claim. A judge then labels each pair "same claim, revised" or "different claim"; the labels stay out
//! of git, and only aggregate counts leave the machine.
//!
//! Run: `cargo run -p analysant --example permille_pilot -- <repo-dir> <path> --seed N --pairs M > pairs.tsv`
//! stdout: `#` lines of aggregate counts (safe to quote), then one tab-separated row per candidate pair
//! (`pair  rev_old  rev_new  sim_permille  new_text  old_text`; the texts are private corpus text: keep the file local).

use analysant::gate_b::shuffle;
use analysant_core::identity::{Observation, observations};
use analysant_core::{English, Ledger, Markdown, extract_document};
use std::collections::BTreeMap;
use std::process::Command;

fn git(repo: &str, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn jaccard_permille(a: &Observation, b: &Observation) -> u32 {
    let union = a.tokens.union(&b.tokens).count();
    if union == 0 {
        return 0;
    }
    u32::try_from(a.tokens.intersection(&b.tokens).count() * 1000 / union).unwrap_or(0)
}

fn flat(s: &str) -> String {
    s.replace(['\t', '\r', '\n'], " ")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(repo), Some(path)) = (args.first(), args.get(1)) else {
        eprintln!("usage: permille_pilot <repo-dir> <path> --seed N --pairs M");
        std::process::exit(2);
    };
    let flag = |name: &str| -> Option<u64> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
    };
    let (Some(seed), Some(want)) = (flag("--seed"), flag("--pairs")) else {
        eprintln!("--seed N and --pairs M are required (no defaults)");
        std::process::exit(2);
    };

    // Oldest to newest, following renames of the path.
    let log = git(
        repo,
        &["log", "--follow", "--no-merges", "--format=%H", "--", path],
    )
    .unwrap_or_default();
    let mut revs: Vec<&str> = log.lines().collect();
    revs.reverse();
    let mut pair_ids: Vec<usize> = (0..revs.len().saturating_sub(1)).collect();
    shuffle(&mut pair_ids, seed);
    pair_ids.truncate(usize::try_from(want).unwrap_or(usize::MAX));
    pair_ids.sort_unstable();

    let pack = English::new();
    let observe = |rev: &str| -> Option<Vec<Observation>> {
        let text = git(repo, &["show", &format!("{rev}:{path}")])?;
        let doc = extract_document(&Markdown, &pack, &mut Ledger::default(), path, &text);
        Some(observations(&pack, &doc))
    };

    let (mut pairs, mut old_n, mut new_n, mut exact, mut unmatched_new, mut with_candidate) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut buckets = [0usize; 10];
    let mut rows = Vec::new();
    for &i in &pair_ids {
        let (Some(old), Some(new)) = (observe(revs[i]), observe(revs[i + 1])) else {
            continue;
        };
        pairs += 1;
        old_n += old.len();
        new_n += new.len();
        // Exact matches by normal-form key, counted as a multiset (a repeated sentence matches once per copy).
        let mut left: BTreeMap<&str, usize> = BTreeMap::new();
        for o in &old {
            *left.entry(o.key.as_str()).or_default() += 1;
        }
        let mut new_rest = Vec::new();
        for n in &new {
            match left.get_mut(n.key.as_str()) {
                Some(c) if *c > 0 => {
                    *c -= 1;
                    exact += 1;
                }
                _ => new_rest.push(n),
            }
        }
        let mut old_rest: Vec<&Observation> = Vec::new();
        for o in &old {
            if let Some(c) = left.get_mut(o.key.as_str())
                && *c > 0
            {
                *c -= 1;
                old_rest.push(o);
            }
        }
        unmatched_new += new_rest.len();
        for n in new_rest {
            let best = old_rest
                .iter()
                .map(|o| (jaccard_permille(n, o), *o))
                .max_by_key(|(s, _)| *s);
            if let Some((s, o)) = best {
                with_candidate += 1;
                buckets[usize::try_from((s / 100).min(9)).unwrap_or(9)] += 1;
                rows.push(format!(
                    "{pairs}\t{}\t{}\t{s}\t{}\t{}",
                    &revs[i][..10],
                    &revs[i + 1][..10],
                    flat(&n.text),
                    flat(&o.text)
                ));
            }
        }
    }
    println!("# permille-pilot v1");
    println!(
        "# file-revisions={} pairs-sampled={} seed={seed}",
        revs.len(),
        pairs
    );
    println!("# claims-old={old_n} claims-new={new_n} exact-key-match={exact}");
    println!("# no-exact-match-new={unmatched_new} with-an-old-candidate={with_candidate}");
    let hist: Vec<String> = buckets
        .iter()
        .enumerate()
        .map(|(i, n)| format!("{}-{}:{n}", i * 100, i * 100 + 99))
        .collect();
    println!(
        "# similarity-histogram(best candidate, permille) {}",
        hist.join(" ")
    );
    println!("# columns: pair rev_old rev_new sim_permille new_text old_text");
    for r in rows {
        println!("{r}");
    }
}
