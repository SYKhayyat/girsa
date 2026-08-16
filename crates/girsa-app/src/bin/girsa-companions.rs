//! Work out which seforim are worth opening beside which, from the link graph.
//!
//! ```sh
//! cargo run --release -p girsa-app --bin girsa-companions -- corpus
//! ```
//!
//! A commentary declares itself — Sefaria's schema for `Rashi on Berakhot`
//! names Berakhot as its base text, and the shelf reads that straight out of
//! `works/index.jsonl`. Nothing declares that the **Beit Yosef** cites Berakhot
//! 815 times, and that is exactly the sefer a person wants in the next column.
//! Only the graph knows, and the graph is 4.1 million edges across 3,700 files
//! — too much to walk every time a menu opens.
//!
//! So it is walked once, here, and the answer is written to
//! `corpus/links/companions.jsonl`:
//!
//! ```jsonl
//! {"work":"bavli/berakhot","with":[{"slug":"beit-yosef","n":815},…]}
//! ```
//!
//! # And which of them keep the same order
//!
//! *Parallel seforim* — the Tur beside the Shulchan Arukh — is the second thing
//! only the graph knows, and `girsa_corpus::taxonomy::Keeping` says what the
//! evidence for it is: **siman 3 of one is joined to siman 3 of the other**.
//! Counting that needs every edge between two works at once, which is this tool
//! and nowhere else, so two more numbers go into each pair:
//!
//! ```jsonl
//! {"slug":"tur","n":410,"simanim":402,"same":402}
//! ```
//!
//! *how many of this work's simanim are joined to that one at all*, and *how
//! many of those to a siman of the same number*. Read back by
//! `Shelf::keeping` and handed to `taxonomy::settled`, which is what stops the
//! Mishneh Torah being offered as a parallel of Yoreh De'ah on the strength of
//! both being הלכה.
//!
//! It is counted **only for the pairs where the shelf permits the question** —
//! `Stands::AskTheAddresses`, which is both works on one top shelf and neither
//! of them a commentary. Everything else already has its answer, and holding a
//! siman table for four million edges' worth of pairs would not fit.
//!
//! # It is a cache and it is allowed to be missing
//!
//! spec.md §4.1: the text files are the truth and anything faster is
//! rebuildable. The shelf opens without this file and offers a shorter list —
//! the declared commentaries only — rather than refusing to open. Delete it and
//! run this again.

// A tool that prints a report. The library it calls does not print.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use girsa_corpus::taxonomy::{self, Keeping, Stands};
use girsa_corpus::work::Work;
use girsa_plain::argv::{self, Argv};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// How many seforim are listed per work.
///
/// Berakhot is joined to 1,600 other works and no menu is 1,600 rows long. The
/// cut is by edge count, and what it dropped is **reported** rather than
/// quietly disappearing — a list that silently stops at 200 reads as "these are
/// all of them".
const PER_WORK: usize = 200;

/// One work's companion list, sorted and cut, ready to be written.
///
/// A named type because the shape is read three times — to pick the pairs whose
/// simanim are worth counting, to write the file, and to report what the cut
/// dropped — and a `(String, usize, Vec<(String, usize)>)` read at any of them
/// is three anonymous numbers.
struct Listing {
    work: String,
    /// How many works were joined to it before the cut at [`PER_WORK`].
    joined: usize,
    /// Who, and how many edges, thickest first.
    with: Vec<(String, usize)>,
}

const USAGE: &str = "usage: girsa-companions <corpus>

  Works out which seforim are worth opening beside which, and WRITES
  <corpus>/links/companions.jsonl. A commentary declares itself; nothing
  declares that the Beit Yosef cites Berakhot 815 times, and only the graph
  knows. Walks all 4.1 million edges, so it takes a while.

  It is a cache and it is allowed to be missing: without it the shelf offers
  the declared commentaries only, and says which kind of empty it is.";

fn main() -> std::process::ExitCode {
    let typed: Vec<String> = std::env::args().skip(1).collect();
    if Argv::wants_help(&typed) {
        return argv::asked(USAGE);
    }
    let args = match Argv::of(typed, &[], &[]) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return argv::refuse(USAGE);
        }
    };
    let Some(root) = args.word(0).map(PathBuf::from) else {
        return argv::refuse(USAGE);
    };
    let links = root.join("links");
    if !links.is_dir() {
        eprintln!(
            "no link graph at {} — run girsa-link-import first",
            links.display()
        );
        return std::process::ExitCode::FAILURE;
    }

    eprintln!("walking {} …", links.display());
    let mut pairs: HashMap<(String, String), usize> = HashMap::new();
    let mut edges = 0usize;
    let mut shards = 0usize;
    let mut unreadable = 0usize;

    for path in shard_paths(&links) {
        let Ok(body) = std::fs::read_to_string(&path) else {
            unreadable += 1;
            continue;
        };
        shards += 1;
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(row) = serde_json::from_str::<girsa_link::store::Row>(line) else {
                continue;
            };
            let (Some(from), Some(to)) = (work_of(&row.from), work_of(&row.to)) else {
                continue;
            };
            edges += 1;
            if from == to {
                // A sefer joined to itself is not a second column.
                continue;
            }
            // Counted both ways: an edge is stored once, in the direction it
            // was written (spec.md §8.2), and both ends want the other offered.
            *pairs.entry((from.to_string(), to.to_string())).or_default() += 1;
            *pairs.entry((to.to_string(), from.to_string())).or_default() += 1;
        }
        if shards % 500 == 0 {
            eprint!("\r  {shards} shards, {edges} edges");
        }
    }
    eprintln!("\r  {shards} shards, {edges} edges          ");
    if unreadable > 0 {
        eprintln!("{unreadable} shards would not read");
    }

    let mut by_work: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    for ((work, other), n) in pairs {
        by_work.entry(work).or_default().push((other, n));
    }
    // Sorted and cut **here**, before anything reads the lists, rather than in
    // the write loop below where it used to be. The second walk asks *which
    // pairs survive the cut* and got its answer from an unsorted vector, so it
    // counted an arbitrary two hundred: the Tur came back parallel to Orach
    // Chayim and not to Yoreh De'ah, which is a shape no rule about seforim
    // could produce and took a while to stop believing.
    let mut listings: Vec<Listing> = by_work
        .into_iter()
        .map(|(work, mut with)| {
            with.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            // How many there were before the cut. **Written into the row**,
            // because a list that silently stops reads as all of them —
            // Berakhot is joined to about 1,600 works and this keeps 200, and
            // the other 1,400 were absent from the picker with nothing anywhere
            // saying so. The number was printed to stdout at the end of the run,
            // where a reader who never ran it cannot see it.
            let joined = with.len();
            with.truncate(PER_WORK);
            Listing { work, joined, with }
        })
        .collect();
    listings.sort_by(|a, b| a.work.cmp(&b.work));

    // Which pairs are worth counting simanim for. The shelf answers every other
    // pair on its own — a commentary is a commentary — and this is the one
    // question it can only narrow down, so it is the only one that costs a
    // second walk. See `girsa_corpus::taxonomy::Keeping`.
    let works = read_works(&root);
    let mut order: HashMap<(String, String), HashMap<u32, bool>> = HashMap::new();
    for listing in &listings {
        let work = &listing.work;
        let Some(mine) = works.get(work.as_str()) else {
            continue;
        };
        for (other, _) in &listing.with {
            let Some(theirs) = works.get(other.as_str()) else {
                continue;
            };
            if taxonomy::stands(mine, theirs) == Stands::AskTheAddresses {
                order.insert((work.clone(), other.clone()), HashMap::new());
            }
        }
    }
    eprintln!(
        "{} pairs the shelf cannot settle — counting simanim",
        order.len()
    );
    if !order.is_empty() {
        let mut walked = 0usize;
        for path in shard_paths(&links) {
            let Ok(body) = std::fs::read_to_string(&path) else {
                continue;
            };
            walked += 1;
            for line in body.lines().filter(|l| !l.trim().is_empty()) {
                let Ok(row) = serde_json::from_str::<girsa_link::store::Row>(line) else {
                    continue;
                };
                let (Some(from), Some(to)) = (work_of(&row.from), work_of(&row.to)) else {
                    continue;
                };
                if from == to {
                    continue;
                }
                let (Some(here), Some(there)) = (
                    taxonomy::kept_number(address_of(&row.from)),
                    taxonomy::kept_number(address_of(&row.to)),
                ) else {
                    continue;
                };
                let same = here == there;
                // Both ends, because the row is written once and both works'
                // lists are being built.
                if let Some(seen) = order.get_mut(&(from.to_string(), to.to_string())) {
                    *seen.entry(here).or_default() |= same;
                }
                if let Some(seen) = order.get_mut(&(to.to_string(), from.to_string())) {
                    *seen.entry(there).or_default() |= same;
                }
            }
            if walked % 500 == 0 {
                eprint!("\r  {walked} shards");
            }
        }
        eprintln!("\r  {walked} shards          ");
    }

    let mut body = String::from(
        "# GENERATED by girsa-companions from corpus/links/*/edges.jsonl.\n\
         # A cache: delete it and run the tool again. The shelf works without it.\n",
    );
    let mut truncated = 0usize;
    let mut dropped = 0usize;
    let mut alongside = 0usize;
    for listing in &listings {
        let (work, joined, with) = (&listing.work, listing.joined, &listing.with);
        if joined > with.len() {
            truncated += 1;
            dropped += joined - with.len();
        }
        let row = serde_json::json!({
            "work": work,
            "joined": joined,
            "with": with.iter().map(|(slug, n)| {
                let keeping = order
                    .get(&(work.clone(), slug.clone()))
                    .map_or_else(Keeping::unknown, |seen| Keeping {
                        joined: seen.len(),
                        same: seen.values().filter(|same| **same).count(),
                    });
                if keeping.keeps_the_same_order() {
                    alongside += 1;
                }
                // Written only where they were counted, so a row says either
                // *this many* or nothing at all, and never *zero* where the
                // question was never asked.
                if keeping == Keeping::unknown() {
                    serde_json::json!({"slug": slug, "n": n})
                } else {
                    serde_json::json!({
                        "slug": slug, "n": n,
                        "simanim": keeping.joined, "same": keeping.same,
                    })
                }
            }).collect::<Vec<_>>(),
        });
        body.push_str(&row.to_string());
        body.push('\n');
    }

    let out = links.join("companions.jsonl");
    if let Err(e) = std::fs::write(&out, body) {
        eprintln!("could not write {}: {e}", out.display());
        return std::process::ExitCode::FAILURE;
    }
    println!("{}", out.display());
    println!(
        "{alongside} pairs keep the same order — {} or more simanim joined to \
         the siman of the same number",
        Keeping::ENOUGH
    );
    if truncated > 0 {
        println!(
            "{truncated} works are joined to more than {PER_WORK} others; \
             {dropped} of the thinnest joins are not listed"
        );
    }
    std::process::ExitCode::SUCCESS
}

/// Every `edges.jsonl` under the links tree.
fn shard_paths(links: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![links.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "edges.jsonl") {
                out.push(path);
            }
        }
    }
    out
}

/// The work slug out of a written anchor.
///
/// `girsa:bavli/berakhot/2a:1#1` → `bavli/berakhot`. A run endpoint is written
/// `<id>-girsa:<id>` and the two ends are always in the same work, so the first
/// half answers for both. Done by hand rather than by parsing a `SegmentId`
/// because this runs eight million times and the answer is one `rfind`.
fn work_of(anchor: &str) -> Option<&str> {
    let one = anchor.split("-girsa:").next().unwrap_or(anchor);
    let body = one.strip_prefix("girsa:")?;
    let cut = body.rfind('/')?;
    Some(&body[..cut])
}

/// The address out of a written anchor — the other half of [`work_of`].
///
/// `girsa:shulchan-arukh/yoreh-deah/1:1#1` → `1:1`. Everything after the last
/// slash and before the ordinal, which is where the slug stops by the same rule
/// [`work_of`] uses. Empty for anything that is not an anchor, which
/// `taxonomy::kept_number` then declines rather than being handed a `None` to
/// carry around.
fn address_of(anchor: &str) -> &str {
    let one = anchor.split("-girsa:").next().unwrap_or(anchor);
    let body = one.split('#').next().unwrap_or(one);
    match body.rfind('/') {
        Some(cut) => &body[cut + 1..],
        None => "",
    }
}

/// The catalogue, as `girsa-import` wrote it.
///
/// Read straight rather than through `Shelf::open`, because the only thing
/// wanted here is `taxonomy::stands`, which asks a work's categories and
/// nothing else. A work whose line will not parse is skipped: this is a cache
/// builder, and a pair it cannot judge simply does not get its simanim counted.
fn read_works(root: &Path) -> HashMap<String, Work> {
    let Ok(body) = std::fs::read_to_string(root.join("works/index.jsonl")) else {
        return HashMap::new();
    };
    body.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|line| serde_json::from_str::<Work>(line).ok())
        .map(|work| (work.slug.clone(), work))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slug_comes_off_an_anchor_whichever_shape_it_is_in() {
        assert_eq!(
            work_of("girsa:bavli/berakhot/2a:1#1"),
            Some("bavli/berakhot")
        );
        assert_eq!(
            work_of(
                "girsa:shulchan-arukh/orach-chayim/1:1#1-girsa:shulchan-arukh/orach-chayim/1:3#3"
            ),
            Some("shulchan-arukh/orach-chayim"),
            "a run endpoint answers with the work, not with half a slug"
        );
        assert_eq!(
            work_of("girsa:קרן-אורה-על-נדרים/2#2"),
            Some("קרן-אורה-על-נדרים")
        );
        assert_eq!(work_of("not a ref"), None);
    }

    #[test]
    fn an_address_comes_off_the_same_anchor_the_slug_does() {
        // The two have to agree about where the slug stops, or a siman is
        // counted against the wrong sefer's numbering.
        assert_eq!(address_of("girsa:shulchan-arukh/yoreh-deah/1:1#1"), "1:1");
        assert_eq!(
            address_of("girsa:tur/yoreh_deah:1:5#715"),
            "yoreh_deah:1:5",
            "the Tur is one work holding four chalakim"
        );
        assert_eq!(
            address_of(
                "girsa:shulchan-arukh/orach-chayim/1:1#1-girsa:shulchan-arukh/orach-chayim/1:3#3"
            ),
            "1:1",
            "a run is counted at the siman it starts in"
        );
        assert_eq!(address_of("not a ref"), "");
    }
}
