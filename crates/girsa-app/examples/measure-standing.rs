//! What asking the shelf instead of the name costs, over the real graph.
//!
//! Coverage used to be `starts_with` on a `Vec<u32>` — about as cheap as a
//! question gets. It is now a lookup in the set of names a place answers to
//! ([`girsa_corpus::standing`]), because the cheap version said yes to a se'if
//! upstream inserted beside the line and no to one it folded away.
//!
//! The links panel asks that question once per edge in a shard, and the Shulchan
//! Arukh's shards are the biggest in the corpus. So the number that matters is
//! not the predicate in isolation but *what a reader waits for after clicking a
//! line*, and both are printed here — over `corpus/`, not a fixture.
//!
//! ```text
//! cargo run --release --example measure-standing -p girsa-app
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use girsa_app::shelf::Shelf;

/// The four Shulchan Arukh volumes, which hold the corpus's largest shards.
const SEFARIM: [&str; 4] = [
    "shulchan-arukh/orach-chayim",
    "shulchan-arukh/yoreh-deah",
    "shulchan-arukh/even-haezer",
    "shulchan-arukh/choshen-mishpat",
];

/// How many lines to stand on. Enough that one slow read does not decide the
/// answer, few enough that this finishes while you are looking at it.
const LINES: usize = 200;

/// How many of those to actually open the panel on. Each one re-reads both link
/// files off disk, which is what this is here to expose.
const PANEL_LINES: usize = 20;

fn main() {
    let root = std::path::Path::new("corpus");
    if !root.join("works/index.jsonl").is_file() {
        println!("no corpus/ here — nothing to measure against");
        return;
    }
    let personal = std::env::temp_dir().join("girsa-measure-standing");
    let _ = std::fs::remove_dir_all(&personal);
    let shelf = Shelf::open(root, &personal).expect("the shelf opens");

    let mut edges_seen = 0usize;
    let mut redirect_rows = 0usize;

    for slug in SEFARIM {
        let Ok(sefer) = shelf.read(slug) else {
            println!("{slug}: not on this shelf");
            continue;
        };
        let shard = girsa_link::store::read_back(root, slug).unwrap_or_default();
        let inbound = girsa_link::inbound::read_back(root, slug).unwrap_or_default();
        edges_seen += shard.len() + inbound.len();

        // Every name each line answers to, built the way the window builds it.
        let step = (sefer.segments.len() / LINES).max(1);
        let places: Vec<_> = sefer
            .segments
            .iter()
            .step_by(step)
            .take(LINES)
            .map(|segment| segment.id.clone())
            .collect();

        let began = Instant::now();
        let standings: Vec<_> = places.iter().map(|id| sefer.standing(id)).collect();
        let building = began.elapsed();
        let names: usize = standings
            .iter()
            .map(girsa_corpus::standing::Standing::len)
            .sum();
        redirect_rows += names - standings.len();

        // The predicate alone, over a shard already in memory: the old test
        // against the new one, same edges, same lines.
        let began = Instant::now();
        let by_name: usize = standings
            .iter()
            .map(|at| shard.iter().filter(|e| e.from.covers(at.at())).count())
            .sum();
        let by_prefix = began.elapsed();

        let began = Instant::now();
        let by_shelf: usize = standings
            .iter()
            .map(|at| shard.iter().filter(|e| e.from.names(at)).count())
            .sum();
        let by_standing = began.elapsed();

        // And what a reader actually waits for: one click, one line. Far fewer
        // lines, because this one reads both files off disk every time and the
        // whole point is to find out what that costs.
        let clicks = standings.iter().take(PANEL_LINES);
        let began = Instant::now();
        let mut links = 0usize;
        for at in clicks.clone() {
            links += girsa_app::touching(&shelf, shelf.repairs(), at).links.len();
        }
        let panel = began.elapsed();

        // The same clicks, reading the two files and doing nothing else — so a
        // slow panel is attributed to the thing actually spending the time
        // rather than to whichever code changed most recently.
        let began = Instant::now();
        for _ in clicks {
            let _ = girsa_link::store::read_back(root, slug).unwrap_or_default();
            let _ = girsa_link::inbound::read_back(root, slug).unwrap_or_default();
        }
        let reading = began.elapsed();

        println!("\n{slug}");
        println!(
            "  {} segments, {} outgoing edges, {} inbound",
            sefer.segments.len(),
            shard.len(),
            inbound.len()
        );
        println!(
            "  standing           {:>7.1?} for {} lines ({} names in total)",
            building,
            standings.len(),
            names
        );
        println!(
            "  the old predicate  {:>7.1?} over {} tests → {by_name} hits",
            by_prefix,
            standings.len() * shard.len()
        );
        println!(
            "  the new one        {:>7.1?} over {} tests → {by_shelf} hits",
            by_standing,
            standings.len() * shard.len()
        );
        let clicked = PANEL_LINES.min(standings.len());
        println!(
            "  the panel          {:>7.1?} for {clicked} lines — {:.0} ms a line, {links} links",
            panel,
            panel.as_secs_f64() * 1000.0 / clicked as f64
        );
        println!(
            "    of which reading {:>7.1?} — {:.0}% of it, and none of it this question",
            reading,
            reading.as_secs_f64() / panel.as_secs_f64() * 100.0
        );
    }

    println!("\n{edges_seen} edges read; {redirect_rows} inherited names across the lines sampled");
    let _ = std::fs::remove_dir_all(&personal);
}
