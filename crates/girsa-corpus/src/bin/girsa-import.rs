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
use girsa_plain::argv::{self, Argv};

const USAGE: &str = "\
usage: girsa-import [--metadata-only] <corpus> <otzaria>

  Reads an Otzaria tree and writes it into the corpus as seforim with
  permanent segment ids.

  --metadata-only        rebuild the catalogue and leave the text alone";

fn main() -> std::process::ExitCode {
    let typed: Vec<String> = std::env::args().skip(1).collect();
    if Argv::wants_help(&typed) {
        return argv::asked(USAGE);
    }
    // The old parser stripped **every** `--`-prefixed token with a `retain`, so
    // a mistyped `--metadata-onyl` was swallowed without a word and the full
    // hour-long import ran instead of the minute-long one.
    let args = match Argv::of(typed, &["--metadata-only"], &[]) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return argv::refuse(USAGE);
        }
    };
    // Rebuild every work's metadata from the catalogue and leave the text
    // alone. The segments are five million records and take an hour; the
    // metadata is a schema field per work and takes a minute, and a shelf that
    // has to be re-imported to learn one new fact about a sefer is a shelf
    // nobody will ever add a field to again.
    let metadata_only = args.switch("--metadata-only");
    let (Some(corpus_root), Some(otzaria_root)) = (args.word(0), args.word(1)) else {
        return argv::refuse(USAGE);
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

    if metadata_only {
        let (rewritten, missing) = rewrite_metadata(&corpus_root, &catalogue);
        eprintln!("\n{rewritten} works re-catalogued, {missing} not on the shelf");
        let commentaries = catalogue
            .works()
            .iter()
            .filter(|w| !w.commentary_on.is_empty())
            .count();
        eprintln!("{commentaries} of them say which sefer they are a commentary on");
        return std::process::ExitCode::SUCCESS;
    }

    let imported = import_all(&corpus_root, catalogue.works(), threads());
    let counts = imported.counts;
    eprintln!(
        "\n{} works · {} segments · {} headings",
        counts.works, counts.segments, counts.headings
    );
    // Counted out loud, whether or not any were cut (B12). A number nobody prints
    // is a number nobody knows, and this one was 5,733 segments over 10,000
    // characters, in 926 works, with the largest at 1,275,307 — reported nowhere.
    // `cargo run -p girsa-corpus --example measure-oversized -- corpus` is the same
    // measurement over a corpus already on disk.
    if imported.oversized.is_empty() {
        eprintln!(
            "no segment is over {} characters — every permanent id names a place",
            girsa_corpus::oversized::NAMES_A_PLACE
        );
    } else {
        eprintln!("segments too long to name a place:");
        for line in imported.oversized.said() {
            eprintln!("{line}");
        }
    }
    // What the second and every later run of this tool costs the shelf. Silent
    // on a first import, because there was no name to keep; loud after that,
    // because this is the number spec.md §3 is about.
    if imported.continuity.is_empty() {
        eprintln!("first import — every permanent id is new");
    } else {
        eprintln!("permanent ids across the re-import:");
        for line in imported.continuity.said() {
            eprintln!("{line}");
        }
    }

    // From `imported.works`, never from `catalogue.works()`. The catalogue is
    // built from the schemas and has not opened a text file, so it does not
    // know the printed edition — see `Imported`.
    if let Err(e) = write_index(&corpus_root, &imported.works, &catalogue) {
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
        // Padded to the same widths as the rows below, so the two labels sit
        // over the two numeric columns. This used to be one hand-spaced string
        // reading `spec.md §2 says          measured`, and the spacing lined up
        // with nothing: at fifteen characters the first label overflows the
        // ten-wide field it labels, which pushed `measured` five columns past
        // the numbers underneath it.
        println!("\n  {:>10}  {:>10}", "spec.md §2", "measured");
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

/// What one import pass produced: the measurements, and the works exactly as
/// they were written to disk.
///
/// The works come back rather than being dropped on the floor because a `Work`
/// learns something during this pass that the catalogue can never know. The
/// printed edition is read out of the text file by `import::read`, and spec.md
/// §13 says that field is *"the only thing preserving the option to distribute
/// publicly later"*. Returning it here is what lets `works/index.jsonl` have a
/// single writer: this pass, holding the same value it just wrote into each
/// work's own `work.json`. It used to be written from `catalogue.works()`
/// instead, which had never opened a `merged.json` — so 6,211 of 7,189 works
/// were catalogued with no edition at all while the edition sat on disk one
/// directory away.
struct Imported {
    counts: Counts,
    /// Segments too long to name a place, and what was done about them (B12).
    ///
    /// Reported the way the link table's six lines are reported, which is the
    /// standard this project already holds itself to — 5,733 segments over 10,000
    /// characters, the largest 1,275,307, were counted nowhere at all.
    oversized: girsa_corpus::oversized::Tally,
    /// How many permanent ids this pass kept, and how many it had to mint.
    ///
    /// The number that says whether spec.md §3 held. Every link, correction,
    /// mark and Ksav citation on the shelf names an id this pass either kept or
    /// did not, and a re-import that renamed 4,170 segments is not a slow import
    /// — it is a silently wrong one, so it gets a line in the report rather than
    /// being something you find out about from a reader.
    continuity: girsa_corpus::import::continuity::Continuity,
    /// In catalogue order, and only the works that were actually written. A
    /// work that could not be read has no text on disk, so a line for it in
    /// the index would be a sefer the shelf offers and then fails to open.
    works: Vec<Work>,
}

/// Read and write every work, in parallel.
fn import_all(root: &Path, works: &[Work], threads: usize) -> Imported {
    let total = works.len();
    let queue = Arc::new(Mutex::new(
        works.iter().cloned().enumerate().rev().collect::<Vec<_>>(),
    ));
    let done = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let oversized = Arc::new(Mutex::new(girsa_corpus::oversized::Tally::default()));
    let continuity = Arc::new(Mutex::new(
        girsa_corpus::import::continuity::Continuity::default(),
    ));
    let counts = Arc::new(Mutex::new(Counts::default()));
    let written = Arc::new(Mutex::new(Vec::<(usize, Work)>::with_capacity(total)));

    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            let queue = Arc::clone(&queue);
            let done = Arc::clone(&done);
            let failed = Arc::clone(&failed);
            let counts = Arc::clone(&counts);
            let oversized = Arc::clone(&oversized);
            let continuity = Arc::clone(&continuity);
            let written = Arc::clone(&written);
            scope.spawn(move || loop {
                let Some((at, work)) = queue.lock().ok().and_then(|mut q| q.pop()) else {
                    return;
                };
                // `read_over`, not `read`: the work may already be on the shelf,
                // and a name it was given then is inside links, corrections and
                // Ksav documents that this pass does not get to see. spec.md §3.
                match import::read_over(root, &work).and_then(|imported| {
                    let c = imported.counts();
                    let big = imported.oversized.clone();
                    let kept = imported.continuity.clone();
                    import::write(root, &imported)?;
                    // `imported.work`, not `work`: this is the one that has
                    // been told which edition it was read out of.
                    Ok((c, big, kept, imported.work))
                }) {
                    Ok((c, big, kept, as_written)) => {
                        if let Ok(mut total) = counts.lock() {
                            total.absorb(c);
                        }
                        if let Ok(mut total) = oversized.lock() {
                            total.absorb(&big);
                        }
                        if let Ok(mut total) = continuity.lock() {
                            total.absorb(&kept);
                        }
                        if let Ok(mut written) = written.lock() {
                            written.push((at, as_written));
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
        // Named as a consequence rather than a statistic: these are the works
        // that will not be in the index, and so not on the shelf.
        eprintln!("\n{failed} works could not be read, and are not in the index");
    }
    // Threads finish out of order; the index does not get to be arbitrary. A
    // byte-stable `index.jsonl` is what lets a rebuild be diffed against the
    // one before it.
    let mut works = written.lock().map(|w| w.clone()).unwrap_or_default();
    works.sort_by_key(|(at, _)| *at);
    Imported {
        oversized: oversized.lock().map(|t| t.clone()).unwrap_or_default(),
        continuity: continuity.lock().map(|c| c.clone()).unwrap_or_default(),
        counts: counts.lock().map(|c| *c).unwrap_or_default(),
        works: works.into_iter().map(|(_, w)| w).collect(),
    }
}

/// Rewrite every work's `work.json` from the catalogue, keeping what only the
/// text file knows.
///
/// The catalogue is built from the schemas, so it knows a work's title,
/// categories, author, era and — since W9 — the sefer it is a commentary on.
/// It does **not** know the printed edition: that is read out of the text when
/// the text is read, so it is carried over from what is already on disk rather
/// than being blanked by a pass that never opened a merged.json.
///
/// Returns how many were rewritten and how many the catalogue knows but the
/// shelf does not.
fn rewrite_metadata(root: &Path, catalogue: &Catalogue) -> (usize, usize) {
    let mut merged = Vec::new();
    let mut missing = 0usize;
    for work in catalogue.works() {
        let path = import::work_dir(root, &work.slug).join("work.json");
        let Ok(existing) = std::fs::read_to_string(&path) else {
            missing += 1;
            continue;
        };
        let mut work = work.clone();
        if let Ok(on_disk) = serde_json::from_str::<Work>(&existing) {
            work.version = on_disk.version;
        }
        match serde_json::to_vec_pretty(&work) {
            Ok(body) => {
                if let Err(e) = std::fs::write(&path, body) {
                    eprintln!("could not rewrite {}: {e}", work.slug);
                    continue;
                }
                merged.push(work);
            }
            Err(e) => eprintln!("could not encode {}: {e}", work.slug),
        }
    }
    if let Err(e) = write_index(root, &merged, catalogue) {
        eprintln!("could not write the work index: {e}");
    }
    (merged.len(), missing)
}

/// One line per work, so the shelf and the link importer do not each have to
/// walk the tree to find out what is on it.
fn write_index(root: &Path, works: &[Work], catalogue: &Catalogue) -> Result<(), std::io::Error> {
    let mut body = String::new();
    for work in works {
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

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// The smallest `merged.json` that carries a printed edition — the two
    /// fields `sefaria::version_of` reads, and a `text` array to walk.
    fn a_sefer_on_disk(at: &Path, edition: &str) {
        std::fs::write(
            at,
            format!(
                r#"{{ "text": ["ראוי לכל ירא שמים", "בשעה שמכניסין"],
                      "sectionNames": ["Paragraph"],
                      "versionTitle": "{edition}",
                      "versionSource": "https://www.sefaria.org/Shulchan_Arukh,_Orach_Chayim" }}"#
            ),
        )
        .unwrap();
    }

    /// A work exactly as the catalogue builds one: **no version**, because the
    /// catalogue is built from the schemas and has never opened a text file.
    fn as_the_catalogue_builds_it(slug: &str, origin: PathBuf) -> Work {
        Work {
            slug: slug.to_string(),
            he_title: "אורח חיים".into(),
            en_title: "Orach Chayim".into(),
            categories: vec!["Halakhah".into()],
            order: Vec::new(),
            source: Source::Sefaria,
            origin,
            schema: None,
            author: None,
            era: None,
            comp_date: None,
            version: None,
            he_sections: Vec::new(),
            commentary_on: Vec::new(),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-import-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The pass that reads the text has to hand back what it read.
    ///
    /// This is finding N-1 at its source. `write_index` was never wrong — it
    /// wrote whatever it was given, and it was given `catalogue.works()`, which
    /// cannot know a printed edition. So the property to pin is that the
    /// importer returns the enriched `Work` rather than dropping it, and that
    /// what reaches the index is the same value that reached `work.json`.
    #[test]
    fn what_the_import_hands_back_carries_the_edition_it_read() {
        let dir = scratch("hands-back");
        let origin = dir.join("merged.json");
        a_sefer_on_disk(&origin, "Maginei Eretz, Lemberg, 1893");
        let catalogued = as_the_catalogue_builds_it("test/orach-chayim", origin);
        assert!(
            catalogued.version.is_none(),
            "the catalogue never knows the edition — that is the whole premise"
        );

        let corpus = dir.join("corpus");
        let imported = import_all(&corpus, std::slice::from_ref(&catalogued), 1);

        assert_eq!(imported.works.len(), 1);
        let version = imported.works[0]
            .version
            .as_ref()
            .expect("the edition was read out of the text and has to come back");
        assert!(version.edition.contains("Lemberg"), "{:?}", version.edition);

        // And it is the *same* value the work's own file got. Two writers that
        // disagree about this field is exactly what N-1 was.
        let on_disk = import::read_back(&corpus, &catalogued.slug).unwrap();
        assert_eq!(
            on_disk.work.version, imported.works[0].version,
            "work.json and the index are written from one value, or they drift"
        );
    }

    /// A work that could not be read is not handed back, so it is not indexed.
    ///
    /// There is no text on disk for it, so a line in `index.jsonl` would be a
    /// sefer the shelf offers and then fails to open.
    #[test]
    fn a_work_that_would_not_read_is_not_in_the_index() {
        let dir = scratch("unreadable");
        let good = dir.join("good.json");
        a_sefer_on_disk(&good, "Lemberg, 1893");

        let works = vec![
            as_the_catalogue_builds_it("test/good", good),
            // Never written, so `import::read` fails on it.
            as_the_catalogue_builds_it("test/missing", dir.join("not-here.json")),
        ];

        let imported = import_all(&dir.join("corpus"), &works, 1);

        assert_eq!(imported.works.len(), 1, "only the one that read");
        assert_eq!(imported.works[0].slug, "test/good");
    }

    /// The index is in catalogue order however the threads finished.
    ///
    /// A byte-stable `index.jsonl` is what lets one rebuild be diffed against
    /// the one before it; workers popping a shared queue do not deliver that on
    /// their own.
    #[test]
    fn the_index_is_in_catalogue_order_whatever_the_threads_did() {
        let dir = scratch("ordering");
        let mut works = Vec::new();
        for n in 0..60 {
            let origin = dir.join(format!("{n}.json"));
            a_sefer_on_disk(&origin, "Lemberg, 1893");
            works.push(as_the_catalogue_builds_it(&format!("test/{n:03}"), origin));
        }

        let imported = import_all(&dir.join("corpus"), &works, 8);

        let got: Vec<&str> = imported.works.iter().map(|w| w.slug.as_str()).collect();
        let want: Vec<&str> = works.iter().map(|w| w.slug.as_str()).collect();
        assert_eq!(got, want);
    }
}
