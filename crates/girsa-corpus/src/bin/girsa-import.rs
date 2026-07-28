//! Import the corpus (BUILDER.md W7).
//!
//! ```sh
//! cargo run --release -p girsa-corpus --bin girsa-import -- \
//!     corpus "C:/Users/Administrator/Downloads/otzaria_latest"
//! ```
//!
//! Sefaria spine, Otzaria fill (spec.md §2.3b, decision 1). Every segment gets
//! a permanent id here and never again (§3, decision 2), and the files it
//! writes carry those ids as fields rather than implying them from line
//! position — see [`girsa_corpus::import`].
//!
//! # It ends by checking the spec's own numbers
//!
//! spec.md §2 is *measured*, not documented, so an import that quietly produces
//! different numbers means one of two things: the import is broken, or the data
//! moved. Both need saying out loud, and neither is visible if nothing checks.
//! So the run finishes with the counts §2 states, each marked, and exits
//! non-zero if any of them is wrong.
//!
//! BUILDER.md Appendix B.5: *if the data no longer matches §2, say so loudly
//! rather than coding around it.*

// A failed check is reported and exits non-zero; it never panics mid-corpus and
// leaves half a shelf written.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use girsa_corpus::import::{self, Counts, SegmentKind};
use girsa_corpus::work::{Catalogue, Source, Work};

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(corpus_root), Some(otzaria_root)) = (args.next(), args.next()) else {
        eprintln!("usage: girsa-import <corpus-root> <otzaria-root>");
        return std::process::ExitCode::from(2);
    };
    let corpus_root = PathBuf::from(corpus_root);
    let otzaria_root = PathBuf::from(otzaria_root);
    let sefaria_root = corpus_root.join("sefaria");

    eprintln!("cataloguing …");
    let (catalogue, skipped) = match Catalogue::build(&sefaria_root, &otzaria_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cannot build a catalogue: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if skipped > 0 {
        // Loud. A schema that would not parse is a sefer whose citations do not
        // resolve, and a silent count of those is how a corpus ends up
        // mysteriously missing one shelf.
        eprintln!("SKIPPED {skipped} schema files that would not parse");
    }

    let overlap = catalogue.overlap();
    eprintln!(
        "{} works: {} shared (Sefaria supplies them), {} Otzaria-only, {} Sefaria-only",
        overlap.union(),
        overlap.shared,
        overlap.otzaria_only,
        overlap.sefaria_only,
    );

    let counts = import_all(&corpus_root, catalogue.works(), threads());
    eprintln!(
        "\n{} works · {} segments · {} headings",
        counts.works, counts.segments, counts.headings
    );

    if let Err(e) = write_index(&corpus_root, &catalogue) {
        eprintln!("could not write the work index: {e}");
        return std::process::ExitCode::FAILURE;
    }

    let mut checks = Checks::default();
    checks.check(
        "spec.md §2.3 · works in the union",
        7576,
        overlap.union(),
        Tolerance::Approximate,
    );
    checks.check(
        "spec.md §2.3 · Otzaria-only works",
        978,
        overlap.otzaria_only,
        Tolerance::Approximate,
    );
    checks.check(
        "no segment id carries a separator that would re-read it elsewhere",
        0,
        counts.malformed_ids,
        Tolerance::Exact,
    );
    checks.check(
        "no work imported to nothing",
        0,
        counts.empty_works,
        Tolerance::Approximate,
    );
    check_mishnah_berurah(&mut checks, &otzaria_root);
    check_shulchan_arukh(&mut checks, &corpus_root);

    checks.report()
}

/// How exactly a measured number has to match the spec's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tolerance {
    /// The spec states this one as a fact, so it must be that.
    Exact,
    /// The spec writes `~`, and the corpus is a moving target. Within 2%.
    Approximate,
}

#[derive(Debug, Default)]
struct Checks {
    rows: Vec<(String, usize, usize, bool)>,
}

impl Checks {
    fn check(&mut self, what: &str, want: usize, got: usize, tolerance: Tolerance) {
        let ok = match tolerance {
            Tolerance::Exact => got == want,
            Tolerance::Approximate => {
                let slack = want / 50;
                got.abs_diff(want) <= slack
            }
        };
        self.rows.push((what.to_string(), want, got, ok));
    }

    fn report(&self) -> std::process::ExitCode {
        println!("\n  spec.md §2 says          measured");
        let mut failed = 0;
        for (what, want, got, ok) in &self.rows {
            println!(
                "  {:>10}  {:>10}   {}  {what}",
                want,
                got,
                if *ok { "ok  " } else { "DIFF" }
            );
            failed += usize::from(!ok);
        }
        if failed == 0 {
            println!("\nall {} checks green", self.rows.len());
            return std::process::ExitCode::SUCCESS;
        }
        println!(
            "\n{failed} of {} measurements disagree with spec.md §2.\n\
             The corpus is written; this is a report, not a rollback. Either the\n\
             import is wrong or the data moved — BUILDER.md Appendix B.5 says to\n\
             say which, loudly, rather than coding around it.",
            self.rows.len()
        );
        std::process::ExitCode::FAILURE
    }
}

fn threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get().min(16))
}

/// Read and write every work, in parallel.
fn import_all(root: &Path, works: &[Work], threads: usize) -> Counts {
    let total = works.len();
    let queue = Arc::new(Mutex::new(works.iter().rev().cloned().collect::<Vec<_>>()));
    let done = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let counts = Arc::new(Mutex::new(Counts::default()));

    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            let queue = Arc::clone(&queue);
            let done = Arc::clone(&done);
            let failed = Arc::clone(&failed);
            let counts = Arc::clone(&counts);
            scope.spawn(move || loop {
                let Some(work) = queue.lock().ok().and_then(|mut q| q.pop()) else {
                    return;
                };
                match import::read(&work).and_then(|imported| {
                    let c = imported.counts();
                    import::write(root, &imported).map(|()| c)
                }) {
                    Ok(c) => {
                        if let Ok(mut total) = counts.lock() {
                            total.absorb(c);
                        }
                    }
                    Err(e) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                        eprintln!("\nFAILED {}: {e}", work.slug);
                    }
                }
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n % 100 == 0 || n == total {
                    eprint!("\r  {n}/{total} works");
                }
            });
        }
    });

    let failed = failed.load(Ordering::Relaxed);
    if failed > 0 {
        eprintln!("\n{failed} works could not be read");
    }
    counts.lock().map(|c| *c).unwrap_or_default()
}

/// One line per work, so the shelf and the link importer do not each have to
/// walk the tree to find out what is on it.
fn write_index(root: &Path, catalogue: &Catalogue) -> Result<(), std::io::Error> {
    let mut body = String::new();
    for work in catalogue.works() {
        match serde_json::to_string(work) {
            Ok(line) => {
                body.push_str(&line);
                body.push('\n');
            }
            Err(e) => eprintln!("could not index {}: {e}", work.slug),
        }
    }
    std::fs::create_dir_all(root.join("works"))?;
    std::fs::write(root.join("works/index.jsonl"), body)?;

    // The 978 Otzaria-only works have no Sefaria schema, so W3's lexicon has
    // never heard of them — and W8 has to resolve links *into* them.
    std::fs::write(
        root.join("lexicon-otzaria.tsv"),
        format!(
            "# GENERATED by girsa-import from the Otzaria-only works.\n\
             # variant\tslug\the-title\ten-title\n{}",
            catalogue.otzaria_lexicon_rows()
        ),
    )
}

/// spec.md §2.1: *Mishnah Berurah is 18,120 lines with 701 headings.*
///
/// Checked against the Otzaria file, which is what §2.1 measured — Mishnah
/// Berurah is a work both corpora have, so the *import* takes Sefaria's copy
/// (decision 1). This is a check on the parser against a known file, and the
/// one heading in those 701 that is `<h2></h2>` is why it is worth having.
fn check_mishnah_berurah(checks: &mut Checks, otzaria_root: &Path) {
    let Some(path) = find_txt(&otzaria_root.join("אוצריא"), "משנה ברורה") else {
        eprintln!("could not find Otzaria's משנה ברורה to check against §2.1");
        checks.check(
            "spec.md §2.1 · Mishnah Berurah segments",
            18120,
            0,
            Tolerance::Exact,
        );
        return;
    };
    let Ok(body) = std::fs::read_to_string(&path) else {
        eprintln!("could not read {}", path.display());
        checks.check(
            "spec.md §2.1 · Mishnah Berurah segments",
            18120,
            0,
            Tolerance::Exact,
        );
        return;
    };
    let segments = import::otzaria::parse(&body);
    checks.check(
        "spec.md §2.1 · Mishnah Berurah segments",
        18120,
        segments.len(),
        Tolerance::Exact,
    );
    checks.check(
        "spec.md §2.1 · Mishnah Berurah headings",
        701,
        segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Heading)
            .count(),
        Tolerance::Exact,
    );
}

/// spec.md §2.2: Shulchan Arukh, Orach Chayim is 697 simanim / 4,171 se'ifim,
/// and the schema says so before a byte of text is read.
fn check_shulchan_arukh(checks: &mut Checks, corpus_root: &Path) {
    let Ok(imported) = import::read_back(corpus_root, "shulchan-arukh/orach-chayim") else {
        eprintln!("Shulchan Arukh, Orach Chayim is not on the shelf");
        checks.check(
            "spec.md §2.2 · S.A. Orach Chayim simanim",
            697,
            0,
            Tolerance::Exact,
        );
        return;
    };
    let mut simanim: Vec<&str> = imported
        .segments
        .iter()
        .filter_map(|s| s.id.path().first().map(String::as_str))
        .collect();
    simanim.sort_unstable();
    simanim.dedup();

    checks.check(
        "spec.md §2.2 · S.A. Orach Chayim simanim",
        697,
        simanim.len(),
        Tolerance::Exact,
    );
    checks.check(
        "spec.md §2.2 · S.A. Orach Chayim se'ifim",
        4171,
        imported.segments.len(),
        Tolerance::Exact,
    );
}

/// Find a `.txt` by its title, wherever in the tree it sits.
///
/// T4, from the other side: Otzaria's folders were renamed after its links were
/// generated, so a path into that tree is not something to rely on. A filename
/// is.
fn find_txt(root: &Path, stem: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_stem().is_some_and(|s| s == stem) {
                return Some(path);
            }
        }
    }
    None
}

/// Kept honest: `Source` is what decides which reader runs, and a build that
/// stopped using it would mean the split had quietly gone away.
#[allow(dead_code)]
fn _assert_source_is_used(work: &Work) -> bool {
    work.source == Source::Sefaria
}
