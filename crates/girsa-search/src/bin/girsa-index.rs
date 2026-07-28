//! Build the search index, and probe it (BUILDER.md W11).
//!
//! ```sh
//! cargo run --release -p girsa-search --bin girsa-index -- build corpus index personal
//! cargo run --release -p girsa-search --bin girsa-index -- words  index יתגבר כארי
//! cargo run --release -p girsa-search --bin girsa-index -- phrase index יתגבר כארי
//! cargo run --release -p girsa-search --bin girsa-index -- stamp  index
//! ```
//!
//! `build` reads every root's `works/index.jsonl` and each work's
//! `segments.jsonl` — the files `girsa-import` wrote — and indexes them under
//! the normalizer. The index is a **rebuildable cache** (spec.md §4.1), so
//! `build` throws away whatever was there rather than patching it.
//!
//! # It reports what did not land
//!
//! Same rule as the link importer: a work that would not read is named, and the
//! segment count is checked against what was on the shelf. An index quietly one
//! sefer short is indistinguishable, from the search box, from a corpus that
//! does not contain the passage.
//!
//! `words` and `phrase` are the index's own probes, not the search bar — no
//! widening, no operators, no paging. Those are W12–W14.

// A tally is printed at the end; this is a command-line tool.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use girsa_corpus::import;
use girsa_corpus::work::Work;
use girsa_hebrew::VariantKind;
use girsa_search::index::{Found, Hit, SearchIndex, Stamp, CACHE_STAMP, PROBE_LIMIT};
use girsa_search::ladder::{Rung, Standing, Widened};
use girsa_search::smart::Smart;
use girsa_search::torat_emet::{Match, Query, Together};

/// The writer's budget. Big enough that the whole corpus goes in without
/// merging itself to death, small enough to leave the machine usable.
const HEAP_BYTES: usize = 512 * 1024 * 1024;

/// Commit every so often, so a run that dies at 90% leaves a usable index
/// rather than nothing — the same promise the fetch makes (spec.md §5).
const COMMIT_EVERY: usize = 250_000;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        return usage();
    };

    match command.as_str() {
        "build" => match rest.split_first() {
            Some((index_dir, roots)) if !roots.is_empty() => build(Path::new(index_dir), roots),
            _ => usage(),
        },
        "find" => match rest.split_first() {
            Some((index_dir, rest)) if !rest.is_empty() => find(Path::new(index_dir), rest),
            _ => usage(),
        },
        "stamp" => match rest.first() {
            Some(index_dir) => stamp(Path::new(index_dir)),
            None => usage(),
        },
        _ => usage(),
    }
}

fn usage() -> std::process::ExitCode {
    eprintln!(
        "usage:\n  \
         girsa-index build <index-dir> <corpus-root> [personal-root …]\n  \
         girsa-index find  <index-dir> [how …] <query …>\n  \
         girsa-index stamp <index-dir>\n\
         \n\
         how — the chips of spec.md §9.5, as flags. Nothing else is applied:\n  \
         --contains   the word contains these letters      קדש → המקדש\n  \
         --letters    these letters, in this order         קדש → קידוש\n  \
         --phrase     the words one after the other\n  \
         --near N     within N words of each other, in any order\n\
         \n\
         the relaxation ladder (spec.md §9.6). In the literal mode a zero is\n\
         offered the rungs with their counts and nothing is applied; --rung is\n\
         the click:\n  \
         --rung NAME  prefixes · spellings · gershayim · abbreviations · proximity\n  \
         --smart      Smart mode: apply the form rungs, and say what that did"
    );
    std::process::ExitCode::from(2)
}

/// What a build found, and what it could not.
#[derive(Default)]
struct Tally {
    works: usize,
    segments: usize,
    headings: usize,
    /// Segments with no words at all once normalized — an empty `<h2></h2>`
    /// (BUILDER.md T8), or a `page` of a scan that has not been OCR'd
    /// (spec.md §9.7). Counted rather than dropped silently: the second kind is
    /// *"not searchable yet"* and the reader has to be able to be told so.
    wordless: usize,
    /// Works listed on the shelf whose segments would not read.
    unreadable: Vec<String>,
}

fn build(index_dir: &Path, roots: &[String]) -> std::process::ExitCode {
    let started = Instant::now();

    let index = match SearchIndex::rebuild(index_dir) {
        Ok(index) => index,
        Err(e) => {
            eprintln!("cannot create the index at {}: {e}", index_dir.display());
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut writer = match index.writer_with_heap(HEAP_BYTES) {
        Ok(writer) => writer,
        Err(e) => {
            eprintln!("cannot open a writer: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut tally = Tally::default();
    let mut since_commit = 0usize;

    for root in roots {
        let root = PathBuf::from(root);
        // A personal layer that does not exist yet is the ordinary state of a
        // fresh install, not an error — but it is said out loud, because "your
        // seforim are not in the index" is exactly the kind of thing that must
        // never be found out from an empty result.
        if !root.exists() {
            eprintln!("{}: not there, nothing to index", root.display());
            continue;
        }
        let works = match load_works(&root) {
            Ok(works) => works,
            Err(e) => {
                eprintln!(
                    "cannot read {}'s work index — has girsa-import run? {e}",
                    root.display()
                );
                return std::process::ExitCode::FAILURE;
            }
        };
        eprintln!("{}: {} works", root.display(), works.len());

        for work in works {
            let imported = match import::read_back(&root, &work.slug) {
                Ok(imported) => imported,
                Err(e) => {
                    eprintln!("  {}: {e}", work.slug);
                    tally.unreadable.push(work.slug.clone());
                    continue;
                }
            };
            for segment in &imported.segments {
                if let Err(e) = writer.add(segment) {
                    eprintln!("cannot index {}: {e}", segment.id);
                    return std::process::ExitCode::FAILURE;
                }
                if girsa_hebrew::normalize(&segment.text).is_empty() {
                    tally.wordless += 1;
                }
                if segment.kind == girsa_corpus::import::SegmentKind::Heading {
                    tally.headings += 1;
                }
            }
            tally.works += 1;
            tally.segments += imported.segments.len();
            since_commit += imported.segments.len();

            if since_commit >= COMMIT_EVERY {
                if let Err(e) = writer.commit() {
                    eprintln!("cannot commit: {e}");
                    return std::process::ExitCode::FAILURE;
                }
                since_commit = 0;
                eprintln!(
                    "  … {} works, {} segments, {:.0}s",
                    tally.works,
                    tally.segments,
                    started.elapsed().as_secs_f64()
                );
            }
        }
    }

    if let Err(e) = writer.commit() {
        eprintln!("cannot commit: {e}");
        return std::process::ExitCode::FAILURE;
    }
    if let Err(e) = index.reload() {
        eprintln!("cannot reload: {e}");
        return std::process::ExitCode::FAILURE;
    }

    let elapsed = started.elapsed();
    println!("\nindexed:");
    println!("  works              {}", tally.works);
    println!("  segments           {}", tally.segments);
    println!("  of which headings  {}", tally.headings);
    println!(
        "  wordless           {}   (empty headings, and scans not yet OCR'd)",
        tally.wordless
    );
    println!("  in the index       {}", index.count());
    println!(
        "  took               {:.0}s  ({:.0} segments/s)",
        elapsed.as_secs_f64(),
        tally.segments as f64 / elapsed.as_secs_f64().max(0.001)
    );
    println!("  on disk            {}", size_on_disk(index_dir));

    if !tally.unreadable.is_empty() {
        println!(
            "\n{} works on the shelf would not read and are NOT in the index:",
            tally.unreadable.len()
        );
        for slug in &tally.unreadable {
            println!("  {slug}");
        }
    }

    // The count that matters: everything the shelf held is findable. A
    // shortfall here looks, from the search box, exactly like a corpus that
    // does not contain the passage.
    if index.count() != tally.segments {
        eprintln!(
            "\nMISMATCH: {} segments were read and {} are in the index",
            tally.segments,
            index.count()
        );
        return std::process::ExitCode::FAILURE;
    }
    if !tally.unreadable.is_empty() {
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

/// Search, in the literal mode, with the chips given as flags.
///
/// The flags are not a query language — spec.md §9.5 makes every control an
/// object you can see, and these are those objects on a command line. Anything
/// that is not a flag is part of what you are looking for.
fn find(index_dir: &Path, args: &[String]) -> std::process::ExitCode {
    let mut matching = Match::default();
    let mut together = Together::default();
    let mut rungs: Vec<Rung> = Vec::new();
    let mut smart = false;
    let mut words: Vec<&str> = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--contains" => matching = Match::Contains,
            "--letters" => matching = Match::Letters,
            "--phrase" => together = Together::Phrase,
            "--smart" => smart = true,
            "--near" => match args.next().and_then(|n| n.parse().ok()) {
                Some(gap) => together = Together::Near { words: gap },
                None => {
                    eprintln!("--near wants a number of words");
                    return std::process::ExitCode::from(2);
                }
            },
            "--rung" => match args.next().map(String::as_str).and_then(rung_named) {
                Some(rung) => rungs.push(rung),
                None => {
                    eprintln!(
                        "--rung wants one of: prefixes spellings gershayim abbreviations proximity"
                    );
                    return std::process::ExitCode::from(2);
                }
            },
            other if other.starts_with("--") => {
                eprintln!("no such chip: {other}");
                return usage();
            }
            word => words.push(word),
        }
    }
    let query = Query::new(words.join(" "))
        .matching(matching)
        .together(together);

    let index = match SearchIndex::open(index_dir) {
        Ok(index) => index,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    if smart {
        if !rungs.is_empty() {
            eprintln!("--smart chooses its own rungs; --rung is the literal mode's click");
            return std::process::ExitCode::from(2);
        }
        return smart_find(&index, &query);
    }

    // A refusal is an answer here, and it says why. What it never is, in either
    // branch, is a shorter list of results with no note attached.
    let found = if rungs.is_empty() {
        index.search(&query)
    } else {
        index.search_widened(&Widened::new(query.clone(), rungs))
    };
    let found = match found {
        Ok(found) => found,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    match &found.widening {
        Some(widening) => println!("searched for: {}", widening.describe()),
        None => println!("searched for: {}", found.asked.describe()),
    }
    page(&index, &found);

    // spec.md §9.6: zero results is a bug in the interface, not an answer — but
    // the default mode offers the next step rather than taking it, with the
    // counts worked out first.
    if found.total == 0 && found.widening.is_none() {
        ladder(&index, &query);
    }
    std::process::ExitCode::SUCCESS
}

/// Smart mode: widen, and say what widening did.
fn smart_find(index: &SearchIndex, query: &Query) -> std::process::ExitCode {
    let answered = match Smart::new(query.clone()).run(index) {
        Ok(answered) => answered,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Some(widening) = &answered.found.widening {
        println!("searched for: {}", widening.describe());
    }
    println!("{}", answered.announcement());
    if answered.from_other_forms() > 0 {
        println!(
            "[exact form only] would show {} — girsa-index find … without --smart",
            answered.exact_total
        );
    }
    page(index, &answered.found);
    std::process::ExitCode::SUCCESS
}

/// The rungs on offer, priced before anything is applied.
fn ladder(index: &SearchIndex, query: &Query) {
    let offers = index.offers(query);
    if offers.offers.is_empty() && offers.refused.is_empty() {
        println!("\nnothing on the ladder would find it either.");
    }
    for offer in &offers.offers {
        println!(
            "  [{} — {}]   --rung {}",
            offer.label,
            offer.count,
            cli_name(offer.rung)
        );
    }
    for refusal in &offers.refused {
        println!(
            "  [{}] could not be counted: {}",
            refusal.rung.label(),
            refusal.why
        );
    }
    for rung in &offers.deferred {
        if let Standing::Deferred(why) = rung.standing() {
            println!("  [{}] is not built: {why}", rung.label());
        }
    }
}

/// How much of the result set is being shown, and the hits themselves.
fn page(index: &SearchIndex, found: &Found) {
    // A page whose total is unstated reads as the whole of it.
    println!(
        "{} in {} segments · showing {}{}\n",
        found.total,
        index.count(),
        found.hits.len(),
        if found.hits.len() == PROBE_LIMIT {
            " (the probe stops here; paging is W14)"
        } else {
            ""
        }
    );
    for hit in &found.hits {
        println!("{}  [{}]", hit.id, hit.kind.as_str());
        println!("  {}", excerpt(hit, found));
    }
}

/// The ladder's rungs, as they are typed on a command line.
fn rung_named(name: &str) -> Option<Rung> {
    Some(match name {
        "prefixes" => Rung::Forms(VariantKind::PrefixPeeled),
        "spellings" => Rung::Forms(VariantKind::KtivSwapped),
        "gershayim" => Rung::Forms(VariantKind::GershayimDropped),
        "abbreviations" => Rung::Forms(VariantKind::AbbreviationExpanded),
        "proximity" => Rung::Proximity,
        _ => return None,
    })
}

fn cli_name(rung: Rung) -> &'static str {
    match rung {
        Rung::Forms(VariantKind::PrefixPeeled) => "prefixes",
        Rung::Forms(VariantKind::KtivSwapped) => "spellings",
        Rung::Forms(VariantKind::GershayimDropped) => "gershayim",
        Rung::Forms(VariantKind::AbbreviationExpanded) => "abbreviations",
        Rung::Proximity => "proximity",
        Rung::Nikud | Rung::Root => "",
    }
}

/// A line of the hit with the matched words bracketed.
///
/// Through [`Found::marks`], so what is bracketed is what the index matched —
/// pointed at the text as printed, which is the property W11 has to hold, and
/// following the widening when there is one, so a hit found by peeling a prefix
/// brackets `[וכשהמלך]` rather than the three letters of it that were typed.
fn excerpt(hit: &Hit, found: &Found) -> String {
    let marks = found.marks(hit);
    let mut out = String::new();
    let mut at = 0usize;
    for (start, end) in marks {
        if start < at || end > hit.text.len() {
            continue;
        }
        out.push_str(hit.text.get(at..start).unwrap_or_default());
        out.push('[');
        out.push_str(hit.text.get(start..end).unwrap_or_default());
        out.push(']');
        at = end;
    }
    out.push_str(hit.text.get(at..).unwrap_or_default());
    out.chars().take(220).collect()
}

fn stamp(index_dir: &Path) -> std::process::ExitCode {
    let path = index_dir.join(CACHE_STAMP);
    match std::fs::read_to_string(&path) {
        Ok(body) => {
            println!("{}: {}", path.display(), body.trim());
            println!(
                "this build wants: {}",
                serde_json::to_string(&Stamp::current()).unwrap_or_default()
            );
            match SearchIndex::open(index_dir) {
                Ok(index) => {
                    println!("usable, {} segments", index.count());
                    std::process::ExitCode::SUCCESS
                }
                Err(e) => {
                    println!("NOT usable: {e}");
                    std::process::ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            std::process::ExitCode::FAILURE
        }
    }
}

fn load_works(root: &Path) -> Result<Vec<Work>, std::io::Error> {
    let body = std::fs::read_to_string(root.join("works/index.jsonl"))?;
    Ok(body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Work>(l).ok())
        .collect())
}

fn size_on_disk(dir: &Path) -> String {
    fn walk(dir: &Path) -> u64 {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| match entry.file_type() {
                Ok(kind) if kind.is_dir() => walk(&entry.path()),
                _ => entry.metadata().map(|m| m.len()).unwrap_or_default(),
            })
            .sum()
    }
    let bytes = walk(dir) as f64;
    format!("{:.1} GB", bytes / 1_073_741_824.0)
}
