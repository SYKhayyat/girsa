//! Where the half-second before the links panel actually goes.
//!
//! `measure-standing` found that clicking a line in Orach Chayim costs ~524 ms
//! and that 70% of it is inside the two `read_back` calls. That is an
//! attribution, not a diagnosis — "reading a file" covers a 27 MB `read_to_string`,
//! 159,273 JSON parses, 318,546 segment-id parses and a `Repaired` built for
//! every row, of which about forty survive the filter.
//!
//! This splits that into the stages a reader is actually waiting on, so the fix
//! is aimed at the one that costs rather than the one that is easiest to see.
//!
//! ```text
//! cargo run --release --example why-the-panel-waits -p girsa-link
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use girsa_link::repair::Repairs;
use girsa_link::store::Row;
use girsa_link::{EdgeType, Method};

const SEFARIM: [&str; 3] = [
    "shulchan-arukh/orach-chayim",
    "shulchan-arukh/choshen-mishpat",
    "bavli/berakhot",
];

// This file carried its own `parse_anchor` under a doc comment that said
// *"mirrors `store::parse_anchor`, which is crate-private"* — an accurate
// description of a copy, which is not the same as a reason for one. The
// original is `pub` now.
use girsa_link::store::parse_anchor;

fn main() {
    let root = std::path::Path::new("corpus");
    if !root.join("works/index.jsonl").is_file() {
        println!("no corpus/ here — nothing to measure against");
        return;
    }
    // An empty repair layer, which is what every reader has until they judge a
    // link — and the case the panel should therefore be fastest in.
    let (repairs, _) = Repairs::open(&std::env::temp_dir().join("girsa-no-repairs"));

    for slug in SEFARIM {
        let path = girsa_link::inbound::inbound_path(root, slug);
        if !path.is_file() {
            println!("\n{slug}: no inbound cache");
            continue;
        }

        // 1. Off the disk.
        let began = Instant::now();
        let body = std::fs::read_to_string(&path).expect("reads");
        let read = began.elapsed();
        let rows = body.lines().filter(|l| !l.trim().is_empty()).count();

        // 2. JSON → `Row`: five `String`s a line, and nothing understood yet.
        let began = Instant::now();
        let parsed: Vec<Row> = body
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<Row>(line).ok())
            .collect();
        let json = began.elapsed();

        // 3. `Row` → `Edge`: two segment ids a row, each one a string parse
        //    that allocates a work, a path and an ordinal.
        let began = Instant::now();
        let edges: Vec<girsa_link::Edge> = parsed
            .iter()
            .filter_map(|row| {
                let (from, to) = (parse_anchor(&row.from)?, parse_anchor(&row.to)?);
                Some(girsa_link::Edge {
                    from,
                    to,
                    edge_type: EdgeType::from_sefaria(&row.label),
                    method: if row.method == Method::OtzariaSeed.as_str() {
                        Method::OtzariaSeed
                    } else {
                        Method::SefariaSeed
                    },
                    direction: girsa_link::Direction::parse(row.direction.as_deref()),
                    source_label: row.label.clone(),
                })
            })
            .collect();
        let ids = began.elapsed();

        // 4. The repair layer over every edge — for a reader who has repaired
        //    nothing, and whose `by_edge` map is therefore empty.
        // Cloned *outside* the timing: `apply` consumes its input, and a clone
        // the real caller does not do would be charged to the thing being
        // measured. `over` does its own clone per edge internally; that one is
        // real and stays counted.
        let handed = edges.clone();
        let began = Instant::now();
        let repaired = repairs.apply(handed);
        let applying = began.elapsed();

        // 5. …and what all of that was for.
        let at = edges
            .first()
            .map(|edge| girsa_corpus::standing::Standing::just(edge.to.from.clone()))
            .expect("a first edge");
        let began = Instant::now();
        let kept = repaired.iter().filter(|r| r.edge.to.names(&at)).count();
        let filtering = began.elapsed();

        // And the cost of the key on its own, which is what step 4 spends its
        // time on: one `format!` per edge, to look up a map with nothing in it.
        let began = Instant::now();
        let mut bytes = 0usize;
        for edge in &edges {
            bytes += girsa_link::repair::name_of(edge).len();
        }
        let naming = began.elapsed();

        let total = read + json + ids + applying + filtering;
        let share = |d: std::time::Duration| d.as_secs_f64() / total.as_secs_f64() * 100.0;
        println!("\n{slug} — {rows} inbound rows, {} MB", body.len() >> 20);
        println!("  1 read off disk     {read:>8.1?}  {:>4.0}%", share(read));
        println!("  2 json → Row        {json:>8.1?}  {:>4.0}%", share(json));
        println!("  3 Row → Edge        {ids:>8.1?}  {:>4.0}%", share(ids));
        println!(
            "  4 repairs.apply     {applying:>8.1?}  {:>4.0}%   (layer is empty)",
            share(applying)
        );
        println!(
            "  5 the filter        {filtering:>8.1?}  {:>4.0}%   → {kept} kept of {}",
            share(filtering),
            edges.len()
        );
        println!("  total               {total:>8.1?}");
        println!("    of step 4, name_of alone: {naming:.1?} building {bytes} bytes of key");
    }
}
