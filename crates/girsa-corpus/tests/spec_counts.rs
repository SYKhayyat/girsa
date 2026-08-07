//! What the schema promised, checked against what the walker produced.
//!
//! BUILDER.md W7's acceptance is that *"a count that drifts fails the import
//! loudly"*. `girsa-import` prints the counts at the end of a run; this makes
//! them a **test**, so a change to the schema walker that quietly loses a se'if
//! fails `cargo test` rather than waiting for somebody to re-read a log.
//!
//! # Why these no longer skip, and what happened when they did
//!
//! Every test in this file used to open with `corpus_or_skip!()` and `return`
//! when the 3.4 GB corpus was absent — which it is on every fresh clone and in
//! CI. `cargo test` captures stderr on a passing test, so what CI printed was:
//!
//! ```text
//! test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s
//! ```
//!
//! Four green tests that had asserted nothing about anything. This is the file
//! that would have caught §3's permanent ids being silently renumbered by every
//! re-import — the defect `bbce9fd` fixed — and it could not, because it had not
//! run since the day it was written.
//!
//! So they run on [`girsa_fixture`] now: a shelf built in about a second by the
//! real importer from real `merged.json` files. The counts are the fixture's own
//! rather than the corpus's, and that is the point — **the assertion was never
//! that Orach Chayim has 4,171 se'ifim, it was that the walker produces exactly
//! as many segments as the schema said it would.** That is a property of this
//! code, it is true of any shelf, and it is checkable everywhere.
//!
//! # And the two facts that really are about the download
//!
//! *697 simanim of 4,171 se'ifim* is a fact about a Sefaria release. No fixture
//! can check it and none pretends to. Those assertions are still here, at the
//! bottom, marked `#[ignore]` — so a run with no corpus prints `2 ignored`
//! instead of `2 passed`, which is the difference between a test that says what
//! it did not check and one that lies about it.
//!
//! ```sh
//! cargo test -p girsa-corpus -- --ignored     # on a machine with the corpus
//! ```

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::import;

/// The corpus root, if it has been fetched and imported.
///
/// Only the `#[ignore]`d checks at the bottom use this. Nothing above it may:
/// a test whose answer depends on whether a 3.4 GB download is present is a test
/// that reports two different things and is believed about both.
fn corpus() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    assert!(
        root.join("works/index.jsonl").is_file(),
        "no imported corpus at {} — run girsa-import first. This check is \
         #[ignore]d precisely so that its absence is never read as a pass.",
        root.display()
    );
    root
}

/// What the schema for `slug` says its shape is: `lengths`, as Sefaria writes
/// it — `[697, 4171]` for Orach Chayim, meaning 697 simanim holding 4,171
/// se'ifim between them.
fn lengths(root: &Path, slug: &str) -> Vec<usize> {
    let path = root
        .join("schemas")
        .join(format!("{}.json", slug.replace('/', "_")));
    let body = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: serde_json::Value = serde_json::from_str(&body).expect("a schema parses");
    doc["schema"]["lengths"]
        .as_array()
        .expect("the schema states its lengths")
        .iter()
        .map(|n| usize::try_from(n.as_u64().expect("a length is a number")).expect("a length fits"))
        .collect()
}

#[test]
fn the_walker_produces_exactly_as_many_segments_as_the_schema_promised() {
    // spec.md §2.2 — *the schemas are the prize.* Otzaria has a line that says
    // `סימן א`; the schema knows what a siman **is** and how many se'ifim are in
    // it. This is the check that the walker used the schema rather than guessing
    // structure back out of the headings, and that it did not drop one on the
    // way — which is what W7 means by a count that drifts failing loudly.
    let root = girsa_fixture::shelf().root();
    for slug in [
        "shulchan-arukh/orach-chayim",
        "mishnah-berakhot",
        "genesis",
        "bavli/berakhot",
    ] {
        let work = import::read_back(root, slug).expect("on the fixture shelf");
        let promised = lengths(root, slug);
        assert_eq!(
            work.segments.len(),
            promised[1],
            "{slug}: the schema promised {} segments and the walker made {}",
            promised[1],
            work.segments.len()
        );
    }
}

#[test]
fn the_first_seif_reads_the_way_the_spec_quotes_it() {
    // spec.md §10.1's own example packet quotes it. If the address is off by one
    // anywhere in the walker, this is where it shows: `1:1` has to be the first
    // se'if of the first siman and not the second of either.
    let root = girsa_fixture::shelf().root();
    let work = import::read_back(root, "shulchan-arukh/orach-chayim").expect("on the shelf");
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
    // not exist. Read as integers, every daf in Shas is off by a page and a half
    // — and every link into Shas with it. The fixture writes that array the way
    // the download writes it, two empty amudim and all, so this runs the real
    // `level_label`/`daf` pair rather than a hand-typed `"2a"`.
    let root = girsa_fixture::shelf().root();
    let work = import::read_back(root, "bavli/berakhot").expect("Berakhot is on the shelf");

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

    // And the amud after it is 2b, not 3a — the half-page half of the same bug.
    let amudim: Vec<&str> = work
        .segments
        .iter()
        .filter_map(|s| s.id.path().first().map(String::as_str))
        .collect();
    let mut seen: Vec<&str> = amudim.clone();
    seen.dedup();
    assert_eq!(
        seen,
        ["2a", "2b", "3a"],
        "the dafim of the fixture Berakhot"
    );
}

#[test]
fn a_branch_schema_makes_a_named_node_a_level_and_the_default_node_not_one() {
    // 1,101 of Sefaria's 6,595 schemas are a `SchemaNode` with `nodes` rather
    // than a jagged array, and the two halves are addressed differently:
    // `Abarbanel on Ezekiel, Introduction 3` is cited by the section's name, and
    // the commentary proper is not cited as *the default part of Abarbanel*.
    let root = girsa_fixture::shelf().root();
    let work = import::read_back(root, "abarbanel-on-ezekiel").expect("on the shelf");
    let paths: Vec<String> = work
        .segments
        .iter()
        .map(|s| s.id.path().join(":"))
        .collect();
    // `section_label` folds the title to something that survives the ref
    // grammar, which lower-cases it — so the level is `introduction`, and
    // asserting on the schema's own capitalisation would be asserting about a
    // string this test typed rather than about what the walker did with it.
    assert!(
        paths.iter().any(|p| p.starts_with("introduction:")),
        "the named node is not a level of the address: {paths:?}"
    );
    assert!(
        paths.iter().any(|p| p == "1:1"),
        "the default node put its own name in the address: {paths:?}"
    );
}

#[test]
fn every_id_on_the_shelf_survives_being_written_down_and_read_back() {
    // The property the whole scheme rests on: an id that does not round-trip is
    // a note, a correction and a citation in a printed sefer all pointing
    // somewhere else. Checked over every work the fixture has, which is every
    // schema shape and both corpora.
    let root = girsa_fixture::shelf().root();
    let body = std::fs::read_to_string(root.join("works/index.jsonl")).expect("the work index");
    let mut checked = 0usize;
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let work: girsa_corpus::work::Work = serde_json::from_str(line).expect("a work parses");
        let imported = import::read_back(root, &work.slug).expect("reads back");
        for segment in &imported.segments {
            let text = segment.id.to_string();
            let back: girsa_corpus::segment::SegmentId =
                text.parse().unwrap_or_else(|e| panic!("{text}: {e}"));
            assert_eq!(back, segment.id, "{text} did not read back as itself");
            assert!(segment.id.is_well_formed(), "{text}");
            checked += 1;
        }
    }
    assert!(checked > 40, "only checked {checked} ids");
}

// ---------------------------------------------------------------------------
// Facts about the download, not about this code
// ---------------------------------------------------------------------------

#[test]
#[ignore = "needs the fetched corpus: cargo test -p girsa-corpus -- --ignored"]
fn shulchan_arukh_orach_chayim_is_697_simanim_of_4171_seifim() {
    // spec.md §2.2. The numbers come from the schema — `"lengths": [697, 4171]`
    // — so this is a check that the walker produced what *this release of the
    // download* promised. It cannot be a fixture test and it is not pretending
    // to be one: it is ignored rather than skipped, so a run without the corpus
    // says `ignored` instead of `ok`.
    let root = corpus();
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
#[ignore = "needs the fetched corpus: cargo test -p girsa-corpus -- --ignored"]
fn every_id_in_the_real_corpus_survives_being_written_down_and_read_back() {
    // The same property as above, on the scale that finds the surprises: a
    // Hebrew section name, a title with a quote mark in it, an ordinal past a
    // cut. The fixture cannot supply 5,000,545 segments and does not claim to.
    let root = corpus();
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
