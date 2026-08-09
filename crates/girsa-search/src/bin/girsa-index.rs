//! Build the search index, and probe it (BUILDER.md W11).
//!
//! ```sh
//! cargo run --release -p girsa-search --bin girsa-index -- build index corpus personal
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

use girsa_plain::argv::Argv;
use girsa_corpus::import;
use girsa_corpus::work::Work;
use girsa_ref::resolve::Context;
use girsa_search::bar::{Answer, Bar, Results};
use girsa_search::chips::{Chips, Skips, Sounding};
use girsa_search::citation::{Landing, NearMiss};
use girsa_search::facets::{Catalogue, Dimension, Facets, Links};
use girsa_search::index::{BuildReport, Hit, Paging, SearchIndex, Stamp, CACHE_STAMP};
use girsa_search::ladder::{Offers, Rung, Standing, Widened};
use girsa_search::torat_emet::{Match, Query, Together};
use girsa_search::Mode;

/// How many rows of one facet the rail shows.
///
/// A rail nobody reads is a rail nobody clicks. What is cut is **counted** on
/// the next line, because a list that silently stops reads as all of them.
const FACET_ROWS: usize = 6;

/// The writer's budget. Big enough that the whole corpus goes in without
/// merging itself to death, small enough to leave the machine usable.
const HEAP_BYTES: usize = 512 * 1024 * 1024;

/// Commit every so often, so a run that dies at 90% leaves a usable index
/// rather than nothing — the same promise the fetch makes (spec.md §5).
const COMMIT_EVERY: usize = 250_000;

fn main() -> std::process::ExitCode {
    let typed: Vec<String> = std::env::args().skip(1).collect();
    if Argv::wants_help(&typed) {
        print_usage();
        return std::process::ExitCode::SUCCESS;
    }
    // Every option this binary understands, in one list: `build`'s switch, the
    // chips of `find`, and `where-from`'s `--except`. Declaring them together
    // rather than per subcommand is what makes `girsa-index build … --near 5`
    // an error naming the option instead of a root directory called `--near`.
    let args = match Argv::of(
        typed,
        &[&["--without-link-types"][..], SWITCHES].concat(),
        &[&["--except"][..], VALUES].concat(),
    ) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return usage();
        }
    };
    let Some(command) = args.word(0) else {
        return usage();
    };
    let rest = args.from(1);

    match command {
        "build" => match rest.split_first() {
            Some((index_dir, roots)) if !roots.is_empty() => {
                build(Path::new(index_dir), &args, roots)
            }
            _ => usage(),
        },
        "find" => match rest.len() {
            0 | 1 => usage(),
            _ => find(Path::new(&rest[0]), &args),
        },
        // W18: where is this phrase from — and, with `--except`, who quotes it.
        "where-from" => match rest.len() {
            0 | 1 => usage(),
            _ => where_from(Path::new(&rest[0]), &args),
        },
        "stamp" => match rest.first() {
            Some(index_dir) => stamp(Path::new(index_dir)),
            None => usage(),
        },
        _ => usage(),
    }
}

fn usage() -> std::process::ExitCode {
    print_usage();
    std::process::ExitCode::from(2)
}

/// An index built before `girsa-link-types` has no link types in it (B25).
///
/// The link facet says *"not built — run girsa-link-types and index again"*, which is
/// the right thing for it to say and the wrong time to hear it: the reader has
/// already spent four minutes. So the ordering constraint is checked before the
/// rebuild starts rather than reported after it finishes.
///
/// A root with no link graph at all is fine and is not this — a personal layer has
/// no `links/` and never will. This is about a corpus that *has* a graph whose types
/// have never been walked.
///
/// # Errors
///
/// Exit 2, before anything is written, unless `--without-link-types` is passed.
fn refuse_an_index_without_link_types(
    args: &Argv,
    roots: &[String],
) -> Result<(), std::process::ExitCode> {
    // A switch now, rather than a string looked for among the roots and left
    // there — so it was also handed to the build loop as a directory to index,
    // and only `!root.exists()` kept it out.
    if args.switch("--without-link-types") {
        eprintln!("building without link types, as asked — the link facet will say so");
        return Ok(());
    }
    let wanting: Vec<&String> = roots
        .iter()
        .filter(|root| Path::new(root).join("links").is_dir())
        .filter(|root| !girsa_link::inbound::built(Path::new(root)))
        .collect();
    if wanting.is_empty() {
        return Ok(());
    }
    for root in &wanting {
        eprintln!(
            "{root} has a link graph whose types have not been walked, so this index would \
             have no link facet."
        );
        eprintln!("  cargo run --release -p girsa-link --bin girsa-link-types -- {root}");
    }
    eprintln!(
        "Run that first, or pass --without-link-types to build anyway. Said now rather than \
         after four minutes of indexing."
    );
    Err(std::process::ExitCode::from(2))
}

/// A positional root that is not a directory is a mistake, and it is said.
///
/// This is the same family as the guardrail the README says was bought
/// expensively: `build corpus index` transposed, and the corpus deleted. That fix
/// landed on `rebuild` and not on `find` — so `find index corpus personal יתגבר
/// כארי` silently absorbed `personal` as a query word, answered `0 in 5000847
/// segments`, and exited **0**. A word that is not Hebrew, not in the corpus and
/// not typed by the reader was added to the query and the answer looked like an
/// answer.
///
/// A root that cannot be *read* was already refused loudly. The gap was the
/// argument that reads fine and is not a root.
///
/// # Errors
///
/// Exit 2 — a usage error, not a failure — when the path is not a directory.
fn refuse_a_query_word_in_root_position(root: &Path) -> Result<(), std::process::ExitCode> {
    if root.is_dir() {
        return Ok(());
    }
    let shown = root.display();
    if root.exists() {
        eprintln!("{shown} is a file, not a root. The root is the corpus or personal directory.");
    } else {
        eprintln!(
            "{shown} is not a directory, so it is not a root — and a word in root position \
             would otherwise be searched for as part of the query. If it is a query word, \
             put the root first: girsa-index find <index-dir> <root> {shown} …"
        );
    }
    Err(std::process::ExitCode::from(2))
}

/// A second root in the *query* is the transposition that was actually reported.
///
/// `build` takes `<index-dir> <corpus-root> [personal-root …]` — several roots. So
/// somebody who has run `build index corpus personal` once types
/// `find index corpus personal יתגבר כארי`, and `find` takes one root and reads
/// `personal` as a Hebrew word to look for. It answered
/// `0 in 5000847 segments · showing 0` and exited 0.
///
/// A query word that names a directory on this machine is a transposition, not a
/// search term: no word in the corpus is a path, and a reader looking for a
/// directory name has the regex mode. Both readings are named in the refusal,
/// because guessing which one was meant is how this happened in the first place.
///
/// # Errors
///
/// Exit 2, before any search runs.
fn refuse_a_root_among_the_query_words(words: &[&str]) -> Result<(), std::process::ExitCode> {
    let looks_like_a_root: Vec<&&str> = words
        .iter()
        .filter(|word| Path::new(word).is_dir())
        .collect();
    if looks_like_a_root.is_empty() {
        return Ok(());
    }
    for word in &looks_like_a_root {
        eprintln!(
            "{word} is a directory on this machine, and it is in the query rather than in \
             root position."
        );
    }
    eprintln!(
        "`find` and `where-from` take ONE root, unlike `build` which takes several — so a \
         second root here would be searched for as a word. Either drop it, or make it the root."
    );
    Err(std::process::ExitCode::from(2))
}

/// The usage. `--help` asks for it; `usage` prints it on the way to exit 2.
/// What this reads.
///
/// # Why the old one is worth remembering
///
/// It had two formatting defects a reader saw and nobody did — a literal
/// newline inside the string put a stray blank line and nine spaces before the
/// `stamp` line, and a missing continuation indented `--rung` four spaces under
/// `--tag`, as though it were a sub-option of it.
///
/// It also listed **four** of the eighteen options. `--regex`, `--citation`,
/// `--instrument`, `--skips`, `--in`, `--shelf`, `--era`, `--by`, `--linked`,
/// `--not`, `--not-shelf`, `--page`, `--size` and `--without-link-types` all
/// worked and none of them were named. The way to find out that `--in` existed
/// was to read the parser.
///
/// And `<root>` — the argument whose absence used to make `find index corpus
/// personal יתגבר כארי` search for the words `personal יתגבר כארי`, answer
/// zero and exit 0 — is now both named here and refused by
/// `refuse_a_query_word_in_root_position` when it is not a directory.
const USAGE: &str = "\
usage:
  girsa-index build <index> <corpus> [personal \u{2026}] [--without-link-types]
  girsa-index find <index> <root> [how \u{2026}] <query \u{2026}>
  girsa-index where-from <index> <root> [--except SLUG] <phrase>
  girsa-index stamp <index>

<root> is the corpus or personal root that `find` reads its catalogue,
corrections and shelf from. It is required.

how \u{2014} the chips of spec.md \u{a7}9.5, as options. Nothing else is applied.
An option that takes a value takes it either way round: --near 5 and --near=5.

  --contains         the word contains these letters      \u{5e7}\u{5d3}\u{5e9} \u{2192} \u{5d4}\u{5de}\u{5e7}\u{5d3}\u{5e9}
  --letters          these letters, in this order         \u{5e7}\u{5d3}\u{5e9} \u{2192} \u{5e7}\u{5d9}\u{5d3}\u{5d5}\u{5e9}
  --phrase           the words one after the other
  --near N           within N words of each other, in any order
  --regex            a regular expression
  --citation         read the query as a mareh makom
  --instrument NAME  gematria \u{b7} rashei \u{b7} sofei \u{b7} atbash \u{b7} dilug
  --skips FROM-TO    how far apart the letters may be, for dilug

narrowing. Each may be given more than once:

  --in SLUG          one sefer
  --shelf NAME       one shelf
  --era NAME         one era
  --by AUTHOR        one author
  --linked KIND      places with an edge of this kind
  --tag NAME         your own tag (spec.md \u{a7}11)
  --not SLUG         everything but this sefer
  --not-shelf NAME   everything but this shelf

the relaxation ladder (spec.md \u{a7}9.6). A literal search that finds nothing is
offered the rungs with their counts and applies none of them; --rung is the
click:

  --rung NAME        prefixes \u{b7} spellings \u{b7} gershayim \u{b7} abbreviations \u{b7} proximity
  --smart            apply the form rungs, and say what that did

  --page N           which page of the results
  --size N           how many on a page";

fn print_usage() {
    eprintln!("{USAGE}");
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
    /// Pages of a scan with words on them, and pages nobody has read.
    ///
    /// Both, because spec.md §9.7 forbids the second from being silent: a scan
    /// in the index as three hundred blank pages is *absent from the results
    /// and present in the count*, and the count is what the results header is
    /// allowed to say.
    pages_read: usize,
    pages_unread: usize,
    scanned_words: usize,
    /// Works listed on the shelf whose segments would not read.
    unreadable: Vec<String>,
    /// Works the link-type cache had something to say about. Zero over the
    /// whole run means the cache was never built, and the link facet then
    /// reports *not built* rather than a column of zeros (spec.md §9.8).
    with_links: usize,
    /// Works whose masks were built against a different segmentation, and were
    /// therefore refused.
    ///
    /// The one failure a positional cache has that the anchor-keyed file it
    /// replaced did not: a stale anchor file is short, a stale mask file is
    /// **wrong**, and wrong in a way that produces a facet column indistinguish-
    /// able from a correct one. Named per work, because the fix is per work.
    links_stale: Vec<String>,
}

fn build(index_dir: &Path, args: &Argv, roots: &[String]) -> std::process::ExitCode {
    let started = Instant::now();

    // Two caches with an ordering constraint between them, and nothing was enforcing
    // it (B25). The link column of a result's facets is filled from what
    // `girsa-link-types` wrote; an index built before that pass has no link types at
    // all, and the facet says *not built* — honestly, but a reader who has just
    // waited four minutes for a rebuild is entitled to have been told first.
    //
    // Refused rather than run: `girsa-link-types` walks the whole graph and is its
    // own job with its own report, and a build that silently ran a second long job
    // inside itself would be a four-minute command that sometimes takes twelve.
    // `--without-link-types` is the way to say you meant it.
    if let Err(code) = refuse_an_index_without_link_types(args, roots) {
        return code;
    }

    let mut index = match SearchIndex::rebuild(index_dir) {
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
            // Which kinds of link touch each segment, from both directions —
            // spec.md §9.8's fifth facet. A cache (`girsa-link-types`), and
            // allowed to be missing: what is not allowed is reading its
            // absence as *nothing is commented on*, which is why the build
            // records whether it was there.
            //
            // Since the cache became one 16-bit mask per segment there is a
            // third thing it can be, and it is the one that would be silent:
            // masks written against a segmentation this work no longer has.
            // The ids are passed in so the file can be refused rather than
            // believed — every mask after an inserted se'if is about the line
            // above it, and the facet column would look exactly like a good one.
            let ids: Vec<girsa_corpus::segment::SegmentId> =
                imported.segments.iter().map(|s| s.id.clone()).collect();
            let by_segment = match girsa_link::touching::read(&root, &work.slug, &ids) {
                girsa_link::touching::Touching::Known(masks) => {
                    tally.with_links += 1;
                    masks
                }
                girsa_link::touching::Touching::Unbuilt => vec![Default::default(); ids.len()],
                girsa_link::touching::Touching::NotThisSegmentation { held, wanted } => {
                    eprintln!(
                        "  {}: link-type masks are for {held} segments and this work has \
                         {wanted} — not read. Run girsa-link-types.",
                        work.slug
                    );
                    tally.links_stale.push(work.slug.clone());
                    vec![Default::default(); ids.len()]
                }
            };

            // What somebody has read off the pages of this sefer, if it is a
            // scan (W26). Read once per work rather than once per page, and
            // **corrections applied**, because a reader who fixed a misread
            // word and then cannot find it has been given a correction that
            // only corrects the display.
            let (words, trouble) = girsa_scan::Words::open(&root, &work.slug);
            for line in trouble {
                eprintln!("  {}: {line}", work.slug);
            }
            let mut page = 0;

            for (at, segment) in imported.segments.iter().enumerate() {
                let kinds: Vec<girsa_link::EdgeType> =
                    by_segment.get(at).copied().unwrap_or_default().kinds();
                // A page of a scan is counted through the pages, never read
                // off the segment's ordinal — splitting one mints `#47.1` and
                // the arithmetic would quietly slip by one from there
                // (`girsa_app::scanning::page_of_id`, and W6 underneath it).
                let read = if segment.kind == girsa_corpus::import::SegmentKind::Page {
                    page += 1;
                    words.page(page)
                } else {
                    None
                };
                let outcome = match &read {
                    Some(read) => writer.add_page(segment, &kinds, read),
                    None => writer.add(segment, &kinds),
                };
                if let Err(e) = outcome {
                    eprintln!("cannot index {}: {e}", segment.id);
                    return std::process::ExitCode::FAILURE;
                }
                if let Some(read) = &read {
                    tally.pages_read += 1;
                    tally.scanned_words += read.words.len();
                }
                if read.is_none() && girsa_hebrew::normalize(&segment.text).is_empty() {
                    tally.wordless += 1;
                }
                if segment.kind == girsa_corpus::import::SegmentKind::Page && read.is_none() {
                    tally.pages_unread += 1;
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

    // What went in, written beside the index — so the facets can tell an empty
    // link column from a link column nobody filled (spec.md §9.8).
    if let Err(e) = index.declare(BuildReport {
        works: tally.works,
        segments: tally.segments,
        link_types: tally.with_links > 0,
    }) {
        eprintln!("cannot write the build report: {e}");
        return std::process::ExitCode::FAILURE;
    }

    let elapsed = started.elapsed();
    println!("\nindexed:");
    println!("  works              {}", tally.works);
    println!("  segments           {}", tally.segments);
    if tally.pages_read + tally.pages_unread > 0 {
        println!(
            "  scanned pages      {} read ({} words), {} not searchable yet",
            tally.pages_read, tally.scanned_words, tally.pages_unread
        );
    }
    println!("  of which headings  {}", tally.headings);
    println!(
        "  wordless           {}   (empty headings, and scans not yet OCR'd)",
        tally.wordless
    );
    println!("  in the index       {}", index.count());
    println!(
        "  link types from    {} works{}",
        tally.with_links,
        if tally.with_links == 0 {
            "   (girsa-link-types has not run — the link facet will say so)"
        } else {
            ""
        }
    );
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

    // Said out loud and not merely counted: these works are in the index with
    // an empty link column, and the reason is a rebuildable cache that has gone
    // out of date rather than a graph that says nothing about them.
    if !tally.links_stale.is_empty() {
        println!(
            "\n{} works have link-type masks for a segmentation they no longer have. \
             They were NOT read, and those works have no link facet:",
            tally.links_stale.len()
        );
        for slug in &tally.links_stale {
            println!("  {slug}");
        }
        println!("\n  Fix: cargo run --release -p girsa-link --bin girsa-link-types -- <corpus>");
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

/// Search, with the chips given as flags (spec.md §9.5, W14).
///
/// The flags are not a query language — §9.5 makes every control an object you
/// can see, and these are those objects on a command line. Anything that is not
/// a flag is part of what you are looking for, and the **sigils** work here too:
/// `"…"`, `*…*`, `~…`, `~5`, `/…/`, `@…`, `=613` set the same chips they set in
/// the window, because they are read by the same code.
/// *Where is this phrase from?* — and *who quotes this Gemara?* (W18).
///
/// One call for both, which is the claim: `--except` is the only difference,
/// and it is the sefer you are standing in.
fn where_from(index_dir: &Path, args: &Argv) -> std::process::ExitCode {
    let Some((root, rest)) = args.from(1).split_first() else {
        return usage();
    };
    let root = PathBuf::from(root);
    // `where-from` has the same shape as `find` — a positional root and then a
    // free-form phrase — so it has the same hole and gets the same two guards.
    if let Err(code) = refuse_a_query_word_in_root_position(&root) {
        return code;
    }

    // `--except` used to be pulled out by a loop with no unknown-option arm, so
    // a mistyped `--excpet` was pushed into the phrase and searched for — and
    // the answer was *this phrase is from nowhere*.
    let except = args.value("--except").map(ToString::to_string);
    let words: Vec<&str> = rest.iter().map(String::as_str).collect();
    if let Err(code) = refuse_a_root_among_the_query_words(&words) {
        return code;
    }
    let phrase = words.join(" ");

    let index = match SearchIndex::open(index_dir) {
        Ok(index) => index,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let works = match load_works(&root) {
        Ok(works) => works,
        Err(e) => {
            eprintln!("cannot read {}'s work index: {e}", root.display());
            return std::process::ExitCode::FAILURE;
        }
    };
    let (notes, _) = girsa_note::note::Notes::open(&root);
    let bar = Bar::new(index, Catalogue::of(&works).tagged(&notes), &root);
    let found = match girsa_search::mekoros::where_from(&bar, &phrase, except.as_deref(), 8) {
        Ok(found) => found,
        Err(why) => {
            eprintln!("{why}");
            return std::process::ExitCode::FAILURE;
        }
    };

    println!("{}  —  {}", found.phrase, found.describe());
    if !found.is_a_quotation() && found.total > 0 {
        println!("(not offered as a source: {} places)", found.total);
    }
    if let Some(why) = found.only_literally() {
        println!("(only literally: {why})");
    }
    for candidate in &found.candidates {
        println!("  {:<28} {}", candidate.he_title, candidate.shown);
        println!("  {:<28} {}", "", candidate.id);
    }
    std::process::ExitCode::SUCCESS
}

/// spec.md §9.7's badge, on a terminal.
///
/// Nothing for a line of the corpus, which was not read off anything. Two
/// different words for the two readers, because *the file said so* and *a
/// machine guessed at a photograph* are forty points of precision apart
/// (`girsa_scan::engine`) and a reader is entitled to know which is in front of
/// them. **Badge them, don't demote them**: the row is where the score put it.
fn badge(hit: &girsa_search::index::Hit) -> String {
    let read = match &hit.by {
        None => String::new(),
        Some(by) if by.is_ocr() => format!("  [OCR — {}]", by.name()),
        Some(_) => "  [read off the file]".to_string(),
    };
    // A permanent id naming this much text names a volume, not a place (B12): the
    // words are in there and the citation is not a mareh makom, and both halves of
    // that are said rather than only the first.
    if hit.is_a_volume() {
        return format!(
            "{read}  [{} chars — this id names a volume, not a place]",
            hit.characters()
        );
    }
    read
}

/// The chips of spec.md §9.5 that stand alone.
const SWITCHES: &[&str] = &[
    "--contains",
    "--letters",
    "--phrase",
    "--smart",
    "--regex",
    "--citation",
];

/// The chips that take a value — either way round, `--near 5` and `--near=5`.
///
/// `--near=5` used to fall through to the `no such chip` arm and exit 2, while
/// the usage line said `--near N`. And a value option at the end of the line
/// with nothing after it became the empty string and was searched for.
const VALUES: &[&str] = &[
    "--near",
    "--rung",
    "--instrument",
    "--skips",
    "--in",
    "--shelf",
    "--era",
    "--by",
    "--linked",
    "--tag",
    "--not",
    "--not-shelf",
    "--page",
    "--size",
];

fn find(index_dir: &Path, args: &Argv) -> std::process::ExitCode {
    // `find <index-dir> <root> <query…>`. The index directory is word 0 and was
    // taken by the caller; this reads from word 1.
    let Some((root, rest)) = args.from(1).split_first() else {
        return usage();
    };
    let root = PathBuf::from(root);
    if let Err(code) = refuse_a_query_word_in_root_position(&root) {
        return code;
    }

    let mut chips = Chips::default();
    let mut rungs: Vec<Rung> = Vec::new();
    let mut paging = Paging::first();
    let words: Vec<&str> = rest.iter().map(String::as_str).collect();
    let mut narrow: Vec<(Dimension, String)> = Vec::new();
    let mut exclude: Vec<(Dimension, String)> = Vec::new();

    if args.switch("--contains") {
        chips.matching = Match::Contains;
    }
    if args.switch("--letters") {
        chips.matching = Match::Letters;
    }
    if args.switch("--phrase") {
        chips.together = Together::Phrase;
    }
    if args.switch("--smart") {
        chips.mode = Mode::Smart;
    }
    if args.switch("--regex") {
        chips.mode = Mode::Regex;
    }
    if args.switch("--citation") {
        chips.mode = Mode::Citation;
    }
    if let Some(near) = args.value("--near") {
        match near.parse() {
            Ok(gap) => chips.together = Together::Near { words: gap },
            Err(_) => {
                eprintln!("--near wants a number of words");
                return std::process::ExitCode::from(2);
            }
        }
    }
    for named in args.every("--rung") {
        match Rung::named(named) {
            Some(rung) => rungs.push(rung),
            None => {
                eprintln!(
                    "--rung wants one of: prefixes spellings gershayim abbreviations proximity"
                );
                return std::process::ExitCode::from(2);
            }
        }
    }
    if let Some(named) = args.value("--instrument") {
        match sounding_named(named) {
            Some(sounding) => {
                chips.mode = Mode::Instruments;
                chips.sounding = sounding;
            }
            None => {
                eprintln!("--instrument wants one of: gematria rashei sofei atbash dilug");
                return std::process::ExitCode::from(2);
            }
        }
    }
    if let Some(range) = args.value("--skips") {
        match range.split_once('-').map(|(a, b)| (a.parse(), b.parse())) {
            Some((Ok(from), Ok(to))) => chips.skips = Skips { from, to },
            _ => {
                eprintln!("--skips wants a range, like 1-50");
                return std::process::ExitCode::from(2);
            }
        }
    }
    for (flag, dimension) in [
        ("--in", Dimension::Sefer),
        ("--shelf", Dimension::Shelf),
        ("--era", Dimension::Era),
        ("--by", Dimension::Author),
        ("--linked", Dimension::Link),
        // Your own tags (B18). The same one naming as the chip and the facet
        // row, because `Dimension` is the one place a dimension is named.
        ("--tag", Dimension::Tag),
    ] {
        narrow.extend(
            args.every(flag)
                .into_iter()
                .map(|value| (dimension, value.to_string())),
        );
    }
    for (flag, dimension) in [
        ("--not", Dimension::Sefer),
        ("--not-shelf", Dimension::Shelf),
    ] {
        exclude.extend(
            args.every(flag)
                .into_iter()
                .map(|value| (dimension, value.to_string())),
        );
    }
    if let Some(page) = args.value("--page") {
        paging = pages(paging, page);
    }
    if let Some(size) = args.value("--size") {
        match size.parse() {
            Ok(size) => paging = Paging { size, ..paging },
            Err(_) => {
                eprintln!("--size wants a number of results");
                return std::process::ExitCode::from(2);
            }
        }
    }

    let index = match SearchIndex::open(index_dir) {
        Ok(index) => index,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let works = match load_works(&root) {
        Ok(works) => works,
        Err(e) => {
            eprintln!("cannot read {}'s work index: {e}", root.display());
            return std::process::ExitCode::FAILURE;
        }
    };
    let (notes, _) = girsa_note::note::Notes::open(&root);
    let bar = Bar::new(index, Catalogue::of(&works).tagged(&notes), &root);

    // The scope chip, set the way a facet click sets it — through the same
    // functions, so the command line cannot narrow by a rule the window does
    // not have.
    for (dimension, key) in narrow {
        chips.scope =
            girsa_search::facets::narrow(&chips.scope, bar.catalogue(), dimension, &row(&key));
    }
    for (dimension, key) in exclude {
        chips.scope =
            girsa_search::facets::exclude(&chips.scope, bar.catalogue(), dimension, &row(&key));
    }

    // Before anything is searched for, and before the index is even opened: a
    // refusal that arrives after `0 in 5000847 segments` is a refusal nobody reads.
    if let Err(code) = refuse_a_root_among_the_query_words(&words) {
        return code;
    }

    let typed = words.join(" ");
    // `--rung` is the click on an offer, and it is the literal mode's only way
    // of widening. It is applied here rather than inside the bar because the
    // bar never widens on its own.
    if !rungs.is_empty() {
        let code = clicked(&bar, &typed, &chips, &rungs, paging);
        say_what_is_missing(index_dir, &root);
        return code;
    }

    match bar.ask(&typed, &chips, paging, &Context::default()) {
        Answer::Segments {
            results,
            offers,
            note,
        } => {
            println!("searched for: {}", results.header);
            if let Some(note) = note {
                println!("{note}");
            }
            show(&bar, &results, paging);
            if !offers.is_empty() {
                ladder(&offers);
            }
            say_what_is_missing(index_dir, &root);
            std::process::ExitCode::SUCCESS
        }
        Answer::Cited(landing) => {
            cited(&bar, &landing);
            std::process::ExitCode::SUCCESS
        }
        Answer::Refused(why) => {
            eprintln!("{why}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// A rung clicked: the widened search, run and reported as widened.
fn clicked(
    bar: &Bar,
    typed: &str,
    chips: &Chips,
    rungs: &[Rung],
    paging: Paging,
) -> std::process::ExitCode {
    let query = Query::new(typed)
        .matching(chips.matching)
        .together(chips.together);
    let widened = Widened::new(query, rungs.to_vec());
    let found = match bar
        .index()
        .search_widened_in(&widened, &chips.scope, paging)
    {
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
    println!(
        "{} in {} segments · showing {}",
        found.total,
        bar.index().count(),
        found.hits.len()
    );
    for hit in &found.hits {
        println!("{}  [{}]{}", hit.id, hit.kind.as_str(), badge(hit));
        println!("  {}", excerpt(hit, &found.marks(hit)));
    }
    std::process::ExitCode::SUCCESS
}

/// Never a silent gap, on the command line too (B7).
///
/// A reader who writes a chaburah and then searches for it gets
/// `0 in 5000847 segments · showing 0` and, until this, no hint at all that the
/// index simply has not seen it yet. The sentence comes from `girsa_note::since`,
/// which is also where the window's results header gets it, so the two cannot
/// disagree about a count.
fn say_what_is_missing(index_dir: &Path, personal: &Path) {
    let at = girsa_note::since::Unindexed::of(Some(index_dir), personal);
    if let Some(said) = at.said() {
        println!(
            "
note: {said}"
        );
    }
}

/// One page of results, and the facets under them.
fn show(bar: &Bar, results: &Results, paging: Paging) {
    // A scan hands back everything it found — it read the scope, not a page of
    // it — so it is not divided into pages, and saying it was would invite a
    // reader to look for results that are already on the screen.
    let pages = if results.hits.len() >= results.total {
        1
    } else {
        results.total.div_ceil(paging.size.max(1))
    };
    println!(
        "{} in {} segments · showing {}{}\n",
        results.total,
        bar.index().count(),
        results.hits.len(),
        if pages > 1 {
            format!(
                " · page {} of {pages}",
                paging.from / paging.size.max(1) + 1
            )
        } else {
            String::new()
        }
    );
    for hit in &results.hits {
        println!("{}  [{}]{}", hit.id, hit.kind.as_str(), badge(hit));
        println!("  {}", excerpt(hit, &results.marker.marks(hit)));
    }
    if results.total > 0 {
        rail(&results.facets);
    }
}

/// The facet rail (spec.md §9.8), counted over the whole result set.
fn rail(facets: &Facets) {
    println!("\nnarrow by:");
    for dimension in Dimension::ALL {
        if dimension == Dimension::Link {
            if let Links::NotBuilt = facets.link {
                println!(
                    "  {:<10} not built — run girsa-link-types and index again",
                    dimension.label()
                );
                continue;
            }
        }
        let rows = facets.rows(dimension);
        if rows.is_empty() {
            continue;
        }
        let shown: Vec<String> = rows
            .iter()
            .take(FACET_ROWS)
            .map(|row| format!("{}{} {}", "  ".repeat(row.depth), row.label, row.count))
            .collect();
        println!("  {:<10} {}", dimension.label(), shown.join(" · "));
        if rows.len() > FACET_ROWS {
            println!("  {:<10} … and {} more", "", rows.len() - FACET_ROWS);
        }
    }
    if facets.uncatalogued > 0 {
        println!(
            "  {:<10} {} hits are in seforim this catalogue does not have — the three above are \
             short by that many",
            "note:", facets.uncatalogued
        );
    }
}

/// A citation: a jump, a choice, or neither — and never a guess.
fn cited(bar: &Bar, landing: &Landing) {
    println!("{}", landing.describe());
    for place in &landing.places {
        println!("  {}  →  {}", place.reference, place.run.first);
        if let Ok(Some(hit)) = bar.index().segment(&place.run.first) {
            println!("     {}", excerpt(&hit, &[]));
        }
    }
    for near in &landing.near {
        match near {
            NearMiss::AddressNotThere { reference, work } => println!(
                "  [{work}] is on the shelf and has no {} — open the sefer?",
                reference.from()
            ),
            NearMiss::NotOnTheShelf { reference, work } => println!(
                "  [{work}] would answer {reference}, and this shelf has no Hebrew text for it"
            ),
            NearMiss::OtherTitle { spelling, slug } => {
                println!("  did you mean [{spelling}] ({slug})?");
            }
        }
    }
    if landing.more_spellings > 0 {
        println!("  … and {} more spellings", landing.more_spellings);
    }
}

/// The rungs on offer, priced before anything is applied.
fn ladder(offers: &Offers) {
    for offer in &offers.offers {
        println!(
            "  [{} — {}]   --rung {}",
            offer.label,
            offer.count,
            offer.rung.name()
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
    if offers.is_empty() {
        println!("\nnothing on the ladder would find it either.");
    }
}

/// A facet row named on the command line rather than clicked.
fn row(key: &str) -> girsa_search::facets::Row {
    girsa_search::facets::Row {
        key: key.to_string(),
        label: key.to_string(),
        count: 0,
        depth: 0,
    }
}

fn pages(paging: Paging, n: &str) -> Paging {
    let page: usize = n.parse().unwrap_or(1);
    Paging {
        from: paging.size * page.saturating_sub(1),
        size: paging.size,
    }
}

fn sounding_named(name: &str) -> Option<Sounding> {
    Some(match name {
        "gematria" => Sounding::Gematria,
        "rashei" => Sounding::Rashei,
        "sofei" => Sounding::Sofei,
        "atbash" => Sounding::Atbash,
        "dilug" => Sounding::Dilug,
        _ => return None,
    })
}

/// A line of the hit with the matched words bracketed.
///
/// The marks come from the answer's own [`girsa_search::bar::Marker`], so what
/// is bracketed is what the search matched — the widened word rather than the
/// three letters of it that were typed, and the word a gematria added up rather
/// than the number.
fn excerpt(hit: &Hit, marks: &[(usize, usize)]) -> String {
    // One renderer, windowed on the match. This used to bracket the marks and then
    // take the first 220 characters *from the start*, so a match half a megabyte
    // into a 495,726-character segment was nowhere near what got displayed.
    girsa_search::snippet::of(&hit.text, marks).text
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
