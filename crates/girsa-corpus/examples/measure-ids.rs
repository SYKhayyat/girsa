//! How long it takes to read a work's ids, and how long the old way took.
//!
//! ```sh
//! cargo run --release -p girsa-corpus --example measure-ids -- corpus
//! ```
//!
//! `SegmentIndex::load` deserialized every line into an `id`-only struct, which
//! does not retain the text and still lexes it — every escape of a segment that
//! reaches 1,275,307 characters, to skip a field. This runs both over whatever
//! the shelf actually holds, so the number is the corpus's rather than a guess.

#![allow(clippy::print_stdout, clippy::expect_used)]

use std::time::Instant;

/// The old reader, kept here and nowhere else: this is the thing being measured
/// against, and a measurement with nothing to compare to is a number.
fn the_old_way(root: &std::path::Path, slug: &str) -> usize {
    #[derive(serde::Deserialize)]
    struct IdOnly {
        id: String,
    }
    let path = girsa_corpus::import::work_dir(root, slug).join("segments.jsonl");
    let Ok(body) = std::fs::read_to_string(&path) else {
        return 0;
    };
    let mut n = 0;
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(record) = serde_json::from_str::<IdOnly>(line) else {
            continue;
        };
        if record
            .id
            .parse::<girsa_corpus::segment::SegmentId>()
            .is_ok()
        {
            n += 1;
        }
    }
    n
}

fn main() {
    let root = std::env::args().nth(1).map_or_else(
        || std::path::PathBuf::from("corpus"),
        std::path::PathBuf::from,
    );
    let body = std::fs::read_to_string(root.join("works/index.jsonl"))
        .expect("a catalogue — run girsa-import first");

    #[derive(serde::Deserialize)]
    struct Slug {
        slug: String,
    }
    let slugs: Vec<String> = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Slug>(l).ok())
        .map(|w| w.slug)
        .take(400)
        .collect();

    let started = Instant::now();
    let mut old = 0;
    for slug in &slugs {
        old += the_old_way(&root, slug);
    }
    let old_took = started.elapsed();

    let started = Instant::now();
    let mut new = 0;
    for slug in &slugs {
        new += girsa_corpus::import::ordered_ids(&root, slug)
            .map(|ids| ids.len())
            .unwrap_or_default();
    }
    let new_took = started.elapsed();

    println!("{} works, {old} ids", slugs.len());
    assert_eq!(
        old, new,
        "the two readers disagree about how many ids there are"
    );
    println!("  deserialize an id-only struct   {:>8.2?}", old_took);
    println!("  scan for the id field           {:>8.2?}", new_took);
    println!(
        "  {:.1}x",
        old_took.as_secs_f64() / new_took.as_secs_f64().max(0.000_001)
    );
}
