//! Why a link did not become an edge — the rows, not the count.
//!
//! ```sh
//! cargo run --release -p girsa-link --example why-dropped -- corpus [rows]
//! ```
//!
//! `girsa-link-import` reports how many rows were dropped under each reason.
//! That is the number BUILDER.md W8 asks for, and it is not enough to act on:
//! *"1.4 million citations name an address the work does not have"* is a
//! finding only once you can see forty of them side by side with what the work
//! actually contains.
//!
//! So this prints the citation, the ref it resolved to, and the addresses that
//! **are** in that work near where it pointed. Kept rather than thrown away
//! after one use, because W23's repair UI is the same question asked one link
//! at a time.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use girsa_corpus::csv::{fields, link_columns};
use girsa_corpus::index::SegmentIndex;
use girsa_link::sefaria::{Resolved, Resolver};
use girsa_ref::Lexicon;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next() else {
        eprintln!("usage: why-dropped <corpus-root> [rows-to-read]");
        std::process::exit(2);
    };
    let root = PathBuf::from(root);
    let limit: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(400_000);

    let mut tsv = std::fs::read_to_string(root.join("lexicon.tsv"))?;
    if let Ok(extra) = std::fs::read_to_string(root.join("lexicon-otzaria.tsv")) {
        tsv.push_str(&extra);
    }
    let lexicon = Lexicon::from_tsv(&tsv);

    eprintln!("indexing the shelf …");
    let (index, _) = SegmentIndex::load(&root)?;
    let mut resolver = Resolver::new(&lexicon);

    // Grouped by work, because the interesting answer is almost never one bad
    // citation — it is one work whose whole shelf of commentaries misses.
    let mut by_work: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    let mut examined = 0usize;

    let body = std::fs::read_to_string(root.join("sefaria/links/links0.csv"))?;
    for line in body.lines().skip(1) {
        if examined >= limit {
            break;
        }
        examined += 1;
        let row = fields(line);
        let (Some(c1), Some(c2)) = (
            row.get(link_columns::CITATION_1),
            row.get(link_columns::CITATION_2),
        ) else {
            continue;
        };
        let t1 = row
            .get(link_columns::TEXT_1)
            .map(String::as_str)
            .unwrap_or("");
        let t2 = row
            .get(link_columns::TEXT_2)
            .map(String::as_str)
            .unwrap_or("");

        for (citation, work_column) in [(c1, t1), (c2, t2)] {
            let Resolved::Exact(reference) =
                resolver.resolve_citation(citation, work_column, &index)
            else {
                continue;
            };
            let slug = reference.work_slug();
            if !index.has_work(&slug) || index.resolve(&reference).is_some() {
                continue;
            }
            let entry = by_work.entry(slug).or_insert((0, Vec::new()));
            entry.0 += 1;
            if entry.1.len() < 3 {
                entry.1.push(format!("{citation}  ->  {reference}"));
            }
        }
    }

    let mut ranked: Vec<(&String, &(usize, Vec<String>))> = by_work.iter().collect();
    ranked.sort_by_key(|(_, (count, _))| std::cmp::Reverse(*count));

    println!("read {examined} rows of links0.csv\n");
    println!("works whose citations name addresses the work does not have:\n");
    for (slug, (count, examples)) in ranked.iter().take(25) {
        println!("{count:>7}  {slug}");
        for example in examples.iter() {
            println!("         {example}");
        }
        if let Some(shape) = shape_of(&root, slug) {
            println!("         the work actually holds: {shape}");
        }
        println!();
    }
    Ok(())
}

/// The first few addresses a work really has, so a miss can be read against
/// them rather than guessed at.
fn shape_of(root: &Path, slug: &str) -> Option<String> {
    let work = girsa_corpus::import::read_back(root, slug).ok()?;
    let mut addresses: Vec<String> = work
        .segments
        .iter()
        .take(3)
        .map(|s| s.id.address())
        .collect();
    addresses.push(format!("… {} segments", work.segments.len()));
    Some(addresses.join(", "))
}
