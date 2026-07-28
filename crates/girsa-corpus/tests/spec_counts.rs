//! The numbers spec.md §2 measured, checked against the corpus on disk.
//!
//! BUILDER.md W7's acceptance is that *"a count that drifts fails the import
//! loudly"*. `girsa-import` prints them at the end of a run; this makes them a
//! **test**, so a change to the schema walker that quietly loses a se'if fails
//! `cargo test` rather than waiting for somebody to re-read a log.
//!
//! # Why it skips instead of failing when the corpus is absent
//!
//! Checking these needs the fetched corpus — 3.4 GB, not committed, and not
//! present on a fresh clone or in CI. A test that failed there would be noise
//! that everybody learns to ignore, which is worse than one that says plainly
//! what it did not check.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::import;

/// The corpus root, if it has been fetched and imported.
fn corpus() -> Option<PathBuf> {
    // Tests run with the crate directory as the cwd; the corpus sits at the
    // repository root.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    root.join("works/index.jsonl").is_file().then_some(root)
}

macro_rules! corpus_or_skip {
    () => {
        match corpus() {
            Some(root) => root,
            None => {
                eprintln!("skipped: no imported corpus — run girsa-import first");
                return;
            }
        }
    };
}

#[test]
fn shulchan_arukh_orach_chayim_is_697_simanim_of_4171_seifim() {
    // spec.md §2.2. The numbers come from the schema — `"lengths": [697, 4171]`
    // — so this is a check that the walker produced what the schema promised,
    // which is the difference between using Sefaria's structure and guessing at
    // it from headings.
    let root = corpus_or_skip!();
    let work = import::read_back(&root, "shulchan-arukh/orach-chayim")
        .expect("Shulchan Arukh, Orach Chayim is on the shelf");

    let mut simanim: Vec<&str> = work
        .segments
        .iter()
        .filter_map(|s| s.id.path().first().map(String::as_str))
        .collect();
    simanim.sort_unstable();
    simanim.dedup();

    assert_eq!(simanim.len(), 697, "simanim");
    assert_eq!(work.segments.len(), 4171, "se'ifim");
}

#[test]
fn the_first_seif_reads_the_way_the_spec_quotes_it() {
    // spec.md §10.1's own example packet quotes it. If the address is off by
    // one anywhere in the walker, this is where it shows.
    let root = corpus_or_skip!();
    let work = import::read_back(&root, "shulchan-arukh/orach-chayim").expect("on the shelf");
    let first = work
        .segments
        .iter()
        .find(|s| s.id.to_string() == "girsa:shulchan-arukh/orach-chayim/1:1#1")
        .expect("siman 1, se'if 1");
    assert!(
        first.text.contains("יתגבר") && first.text.contains("כארי"),
        "{}",
        first.text.chars().take(120).collect::<String>()
    );
}

#[test]
fn berakhot_begins_at_daf_2a_and_not_at_daf_1() {
    // Sefaria stores a masechta as a flat array of amudim from `1a`, which does
    // not exist. Read as integers, every daf in Shas is off by a page and a
    // half — and every link into Shas with it.
    let root = corpus_or_skip!();
    let work = import::read_back(&root, "bavli/berakhot").expect("Berakhot is on the shelf");

    let first = work.segments.first().expect("Berakhot has segments");
    assert_eq!(
        first.id.path().first().map(String::as_str),
        Some("2a"),
        "the first segment of Berakhot is on daf 2a"
    );
    assert!(
        first.text.contains("מֵאֵימָתַי") || first.text.contains("מאימתי"),
        "{}",
        first.text.chars().take(80).collect::<String>()
    );
}

#[test]
fn every_id_on_the_shelf_survives_being_written_down_and_read_back() {
    // The property the whole scheme rests on, checked on real data rather than
    // on fixtures: an id that does not round-trip is a note, a correction and a
    // citation in a printed sefer all pointing somewhere else.
    let root = corpus_or_skip!();
    let mut checked = 0usize;
    for slug in [
        "shulchan-arukh/orach-chayim",
        "bavli/berakhot",
        "mishnah-berurah",
        "abarbanel-on-ezekiel",
    ] {
        let Ok(work) = import::read_back(&root, slug) else {
            continue;
        };
        for segment in &work.segments {
            let text = segment.id.to_string();
            let back: girsa_corpus::segment::SegmentId =
                text.parse().unwrap_or_else(|e| panic!("{text}: {e}"));
            assert_eq!(back, segment.id, "{text} did not read back as itself");
            assert!(segment.id.is_well_formed(), "{text}");
            checked += 1;
        }
    }
    assert!(checked > 20_000, "only checked {checked} ids");
}
