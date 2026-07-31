//! Import the link graph onto permanent segment ids (BUILDER.md W8).
//!
//! ```sh
//! cargo run --release -p girsa-link --bin girsa-link-import -- \
//!     corpus "C:/Users/Administrator/Downloads/otzaria_latest"
//! ```
//!
//! Sefaria's `links*.csv` for everything, resolved through `girsa-ref` onto the
//! ids `girsa-import` minted; Otzaria's `*_links.json` only for the 978 works
//! Sefaria has no text for (spec.md §8.1).
//!
//! # It reports what it dropped
//!
//! BUILDER.md W8: *report resolution rate and the count of links dropped as
//! unresolvable — a silent drop is a defect.* Every row that did not become an
//! edge is counted under the reason it did not, because "97% imported" without
//! the other 3% named is a number nobody can act on.

// A tally is printed at the end; nothing here panics mid-corpus.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use girsa_corpus::index::SegmentIndex;
use girsa_corpus::work::{Source, Work};
use girsa_link::otzaria::{LineMap, OtzariaTally, TitleIndex};
use girsa_link::sefaria::{Resolver, Tally};
use girsa_link::store;
use girsa_ref::Lexicon;

/// Flush the edge buffer at roughly this size, so a run of the whole corpus is
/// bounded by the index rather than by how many edges it has found.
const FLUSH_AT_BYTES: usize = 64 * 1024 * 1024;

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(corpus_root), Some(otzaria_root)) = (args.next(), args.next()) else {
        eprintln!("usage: girsa-link-import <corpus-root> <otzaria-root>");
        return std::process::ExitCode::from(2);
    };
    let corpus_root = PathBuf::from(corpus_root);
    let otzaria_root = PathBuf::from(otzaria_root);

    let lexicon = match load_lexicon(&corpus_root) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot load the lexicon: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    eprintln!(
        "lexicon: {} works, {} spellings",
        lexicon.len(),
        lexicon.variant_count()
    );

    let works = match load_works(&corpus_root) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("cannot read the work index — has girsa-import run? {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    eprintln!("indexing {} works' addresses …", works.len());
    let (index, unreadable) = match SegmentIndex::load(&corpus_root) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("cannot index the shelf: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if !unreadable.is_empty() {
        eprintln!(
            "{} works would not load and their links will all be dropped: {}",
            unreadable.len(),
            unreadable
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    eprintln!(
        "  {} works · {} addressable segments",
        index.works(),
        index.segments()
    );

    let mut resolver = Resolver::new(&lexicon);
    let mut writer = store::Writer::default();

    // Which of a `commentary` row's two ends is the commentary. Read off the
    // work index, because the row itself does not say and Sefaria writes it
    // both ways round — see `girsa_link::orient`.
    let bases = girsa_link::orient::Bases::of(&works);
    let mut oriented = girsa_link::orient::Orienting::new(&bases);
    eprintln!("  {} works declare a base text", bases.declaring());

    let sefaria = import_sefaria(
        &corpus_root,
        &mut resolver,
        &index,
        &mut writer,
        &mut oriented,
    );
    let otzaria = import_otzaria(
        &corpus_root,
        &otzaria_root,
        &works,
        &mut resolver,
        &index,
        &mut writer,
        &mut oriented,
    );
    eprintln!("  {}", oriented.tally().said());

    if let Err(e) = writer.flush(&corpus_root) {
        eprintln!("could not write the last of the edges: {e}");
        return std::process::ExitCode::FAILURE;
    }

    let unsettled = match write_unsettled(&corpus_root, &resolver) {
        Ok(n) => n,
        Err(e) => {
            // Loud, and fatal. The whole point of the file is that an ambiguity
            // is not thrown away; a run that dropped 5,000 of them and could not
            // write the list has thrown them away and must not look successful.
            eprintln!("could not write the unsettled citations: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    report(sefaria, otzaria, resolver.settled(), unsettled)
}

/// Write every ambiguity nothing settled, worst first.
///
/// BUILDER.md rule 6: *ambiguity is surfaced to the user as a choice.* An
/// import has no user to ask, so it does the only other honest thing — it keeps
/// the question. `corpus/links/unsettled.jsonl` is the queue W23's repair UI
/// reads, and until then it is a file anyone can open and count.
fn write_unsettled(root: &Path, resolver: &Resolver<'_>) -> Result<usize, std::io::Error> {
    let unsettled = resolver.unsettled();
    let dir = root.join("links");
    std::fs::create_dir_all(&dir)?;
    let mut out = std::io::BufWriter::new(std::fs::File::create(dir.join("unsettled.jsonl"))?);
    for row in &unsettled {
        let Ok(line) = serde_json::to_string(row) else {
            continue;
        };
        writeln!(out, "{line}")?;
    }
    out.flush()?;
    Ok(unsettled.len())
}

fn report(
    sefaria: Tally,
    otzaria: OtzariaTally,
    settled: (usize, usize),
    unsettled: usize,
) -> std::process::ExitCode {
    println!("\n== Sefaria links*.csv — citation-addressed, the whole corpus");
    print_tally(&sefaria);
    println!("\n== Otzaria *_links.json — the 978 works Sefaria has no text for");
    print_tally(&otzaria.common);
    println!(
        "   target file unknown  {:>9}\n   line is not a segment{:>9}",
        otzaria.unknown_target_file, otzaria.line_not_a_segment
    );

    let mut total = sefaria;
    total.absorb(otzaria.common);
    println!(
        "\n{} of {} rows became an edge — {:.1}%",
        total.imported,
        total.rows,
        total.rate() * 100.0
    );
    println!(
        "{} rows ({:.0}%) arrived with a blank Conection Type, which spec.md §2.1 \
         measured at 74% and says originates upstream.",
        total.untyped,
        (total.untyped as f64) * 100.0 / (total.rows.max(1) as f64)
    );

    // Reported, not folded into the rate. These are the endpoints where a
    // citation named several seforim and something other than a guess said
    // which — so the size of each kind of evidence stays visible, and a change
    // that starts leaning on the weaker one shows up as a number rather than as
    // a slightly better import.
    let (by_column, by_shelf) = settled;
    println!(
        "\n{} ambiguous endpoints were settled without a guess: \
         {by_column} by the row's own Text N column, {by_shelf} because every \
         other candidate names no place on the shelf.",
        by_column + by_shelf
    );
    println!(
        "{unsettled} citations nothing settled are written to \
         corpus/links/unsettled.jsonl, with their candidates — dropped from the \
         graph, kept as a question."
    );

    if total.imported == 0 {
        println!("\nnothing was imported — that is a failure, not an empty corpus");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn print_tally(tally: &Tally) {
    println!("   rows                 {:>9}", tally.rows);
    println!(
        "   imported             {:>9}  {:>5.1}%",
        tally.imported,
        tally.rate() * 100.0
    );
    println!("   citation unresolved  {:>9}", tally.unresolved_citation);
    println!("   still ambiguous      {:>9}", tally.ambiguous);
    println!("   work not on shelf    {:>9}", tally.work_not_on_shelf);
    println!("   address not in work  {:>9}", tally.address_not_found);
    println!("   blank type (T5)      {:>9}", tally.untyped);
}

/// The resolver's vocabulary: Sefaria's schemas, plus the 978 works that have
/// no schema and would otherwise be unciteable.
fn load_lexicon(root: &Path) -> Result<Lexicon, std::io::Error> {
    let mut tsv = std::fs::read_to_string(root.join("lexicon.tsv"))?;
    // Written by girsa-import. Missing is survivable — it costs the Otzaria
    // half of the graph, and the run says so rather than quietly halving.
    match std::fs::read_to_string(root.join("lexicon-otzaria.tsv")) {
        Ok(extra) => tsv.push_str(&extra),
        Err(e) => eprintln!("no Otzaria lexicon ({e}) — links into those 978 will not resolve"),
    }
    Ok(Lexicon::from_tsv(&tsv))
}

fn load_works(root: &Path) -> Result<Vec<Work>, std::io::Error> {
    let body = std::fs::read_to_string(root.join("works/index.jsonl"))?;
    Ok(body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Work>(l).ok())
        .collect())
}

fn import_sefaria(
    corpus_root: &Path,
    resolver: &mut Resolver<'_>,
    index: &SegmentIndex,
    writer: &mut store::Writer,
    oriented: &mut girsa_link::orient::Orienting<'_>,
) -> Tally {
    let dir = corpus_root.join("sefaria/links");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            // `links_by_book.csv` is a per-book summary of the same graph, not
            // more of it. Importing both would double every edge.
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.starts_with("links") && n.ends_with(".csv") && !n.contains("by_book")
            })
        })
        .collect();
    files.sort();

    let mut tally = Tally::default();
    for (i, path) in files.iter().enumerate() {
        eprint!("\r  sefaria links {}/{}", i + 1, files.len());
        match girsa_link::sefaria::read_file(path, resolver, index, |edge| {
            let mut edge = edge;
            oriented.apply(&mut edge);
            writer.push(&edge);
        }) {
            Ok(t) => tally.absorb(t),
            Err(e) => eprintln!("\n{}: {e}", path.display()),
        }
        if writer.buffered_bytes() > FLUSH_AT_BYTES {
            if let Err(e) = writer.flush(corpus_root) {
                eprintln!("\ncould not write edges: {e}");
            }
        }
    }
    eprintln!();
    tally
}

fn import_otzaria(
    corpus_root: &Path,
    otzaria_root: &Path,
    works: &[Work],
    resolver: &mut Resolver<'_>,
    index: &SegmentIndex,
    writer: &mut store::Writer,
    oriented: &mut girsa_link::orient::Orienting<'_>,
) -> OtzariaTally {
    let titles = TitleIndex::build(works);
    let links_dir = otzaria_root.join("links");
    let mut target_lines: HashMap<String, Option<LineMap>> = HashMap::new();
    let mut tally = OtzariaTally::default();

    let otzaria_works: Vec<&Work> = works
        .iter()
        .filter(|w| w.source == Source::Otzaria)
        .collect();
    let mut with_links = 0usize;

    for (i, work) in otzaria_works.iter().enumerate() {
        if i % 100 == 0 {
            eprint!("\r  otzaria links {}/{}", i, otzaria_works.len());
        }
        let Some(stem) = work.origin.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let path = links_dir.join(format!("{stem}_links.json"));
        if !path.is_file() {
            continue;
        }
        with_links += 1;

        let source_lines = match LineMap::build(corpus_root, work) {
            Ok(map) => map,
            Err(e) => {
                eprintln!("\n{}: {e}", work.slug);
                continue;
            }
        };
        match girsa_link::otzaria::read_file(
            &path,
            &source_lines,
            &titles,
            corpus_root,
            &mut target_lines,
            resolver,
            index,
            |edge| {
                // Otzaria's rows are already base-first-as-index-1 by
                // convention, so this mostly confirms rather than corrects —
                // which is exactly what makes it worth running over them.
                let mut edge = edge;
                oriented.apply(&mut edge);
                writer.push(&edge);
            },
        ) {
            Ok(t) => tally.absorb(t),
            Err(e) => eprintln!("\n{}: {e}", path.display()),
        }
        if writer.buffered_bytes() > FLUSH_AT_BYTES {
            if let Err(e) = writer.flush(corpus_root) {
                eprintln!("\ncould not write edges: {e}");
            }
        }
    }
    eprintln!(
        "\r  otzaria links: {with_links} of {} Otzaria-only works have a link file",
        otzaria_works.len()
    );
    tally
}
