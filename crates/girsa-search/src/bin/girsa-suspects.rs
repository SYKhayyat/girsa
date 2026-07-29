//! The OCR queue: a batch job over the whole library (BUILDER.md W21).
//!
//! ```sh
//! cargo run --release -p girsa-search --bin girsa-suspects -- index personal
//! cargo run --release -p girsa-search --bin girsa-suspects -- index personal --common 5000
//! ```
//!
//! It reads the **index's term dictionary** rather than the corpus: tantivy has
//! already counted every word of every segment, and a second pass over five
//! million segments to arrive at the same table would be an hour for nothing.
//!
//! What it writes is `personal/suspects.jsonl` — yours, beside your corrections
//! — and it **keeps every decision already on that file** (see
//! `girsa_fix::suspect::Queue::refresh`). A batch job that forgot what you had
//! dismissed would hand you the same four thousand candidates every time the
//! corpus was updated, and you would stop running it.
//!
//! Nothing here corrects anything. Every line it writes is a question.

// A tally is printed at the end; this is a command-line tool.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use girsa_fix::suspect::{hunt, Queue, Settings, Suspect, Vocabulary};
use girsa_search::index::SearchIndex;

/// How many places to record per candidate.
///
/// Enough to go and look, and not so many that a word appearing in a thousand
/// segments — which by definition it does not — could bloat the file.
const PLACES: usize = 3;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let plain: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let (Some(index), Some(personal)) = (plain.first(), plain.get(1)) else {
        return usage();
    };
    let mut settings = Settings::default();
    if let Some(at) = flag(&args, "--common") {
        settings.common_at = at;
    }
    if let Some(at) = flag(&args, "--rare") {
        settings.rare_at = at;
    }
    if let Some(at) = flag(&args, "--shortest") {
        settings.shortest = usize::try_from(at).unwrap_or(4);
    }

    match run(Path::new(index), &PathBuf::from(personal), settings) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn flag(args: &[String], name: &str) -> Option<u64> {
    let at = args.iter().position(|a| a == name)?;
    args.get(at + 1)?.parse().ok()
}

fn run(index: &Path, personal: &Path, settings: Settings) -> Result<(), String> {
    let began = Instant::now();
    let index = SearchIndex::open(index).map_err(|e| e.to_string())?;

    let words = index.vocabulary().map_err(|e| e.to_string())?;
    let mut vocabulary = Vocabulary::default();
    for (word, count) in words {
        vocabulary.add(&word, count);
    }
    println!(
        "{} words in the index, read in {:.1}s",
        vocabulary.len(),
        began.elapsed().as_secs_f32()
    );

    let hunting = Instant::now();
    let mut found = hunt(&vocabulary, settings);
    println!(
        "{} candidates in {:.1}s — a word seen {} time(s) or fewer, one letter from one seen {} or more",
        found.len(),
        hunting.elapsed().as_secs_f32(),
        settings.rare_at,
        settings.common_at
    );

    // Where to go and look. Asked of the index one candidate at a time, which
    // is a rare word and therefore one or two segments each.
    let mut nowhere = 0usize;
    for suspect in &mut found {
        match index.words(&suspect.rare) {
            Ok(hits) => {
                if hits.is_empty() {
                    nowhere += 1;
                }
                suspect.places = hits.into_iter().take(PLACES).map(|hit| hit.id).collect();
            }
            Err(e) => return Err(format!("looking for {}: {e}", suspect.rare)),
        }
    }
    if nowhere > 0 {
        // Reported rather than swallowed: a word the term dictionary has and
        // the search cannot find is a disagreement inside the index, and it is
        // worth knowing about even though it costs only a queue row.
        println!("{nowhere} candidates the search could not place");
    }

    let confusions = found.iter().filter(|s| s.confusion.is_some()).count();
    let (mut queue, trouble) = Queue::open(personal);
    for line in trouble {
        eprintln!("{line}");
    }
    let refreshed = queue.refresh(found).map_err(|e| e.to_string())?;

    println!(
        "{} in the queue, {} of them a known confusion of shapes",
        refreshed.found, confusions
    );
    println!(
        "{} new · {} already decided · {} kept from a corpus that has changed",
        refreshed.fresh, refreshed.decided_before, refreshed.gone
    );
    println!(
        "{} waiting to be looked at → {}",
        queue.waiting(),
        queue.path().display()
    );

    for suspect in queue.ranked(10) {
        println!("  {}", one_line(suspect));
    }
    Ok(())
}

fn one_line(suspect: &Suspect) -> String {
    let pair = suspect
        .confusion
        .as_deref()
        .map_or_else(String::new, |c| format!(" [{c}]"));
    let place = suspect
        .places
        .first()
        .map_or_else(String::new, |id| format!("  {id}"));
    format!(
        "{} ({}) → {} ({}){pair}{place}",
        suspect.rare, suspect.rare_count, suspect.common, suspect.common_count
    )
}

fn usage() -> std::process::ExitCode {
    eprintln!(
        "girsa-suspects <index> <personal> [--rare 1] [--common 10000] [--shortest 4]\n\
         \n\
         Finds words that are almost certainly scanning errors and writes them\n\
         to <personal>/suspects.jsonl as a ranked queue. Corrects nothing."
    );
    std::process::ExitCode::FAILURE
}
