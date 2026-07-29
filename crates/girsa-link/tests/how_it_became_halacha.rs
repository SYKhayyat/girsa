//! The chain spec.md §8.6 asks for, over the corpus that is actually on disk.
//!
//! > *Trace forward from a Gemara to how it became halacha; trace backward from
//! > a ruling to where the posek got it; find the path between two texts, or
//! > report honestly that none exists.*
//!
//! The one to check is **backward**, because it is the one the graph is stored
//! against: the Mishnah Berurah's shard holds 18,806 edges pointing at the
//! Shulchan Arukh, so *what does this se'if answer to* is a question about
//! edges that are not in the Shulchan Arukh's file and are not in its
//! direction. Both halves of W28 have to be right for it to come out —
//! `inbound.jsonl` to find the edge at all, and the time axis to know which way
//! it runs.
//!
//! # Why it skips when the corpus is absent
//!
//! It needs the fetched corpus, imported, with links imported over it and
//! `girsa-link-types` run — none of which is committed. A test that failed on a
//! fresh clone would be noise everybody learns to ignore.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::era::Timeline;
use girsa_corpus::segment::SegmentId;
use girsa_link::chain::{self, Direction, Found, Graph, Limits};
use girsa_link::repair::Repairs;

fn corpus() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    (root.join("links").is_dir() && girsa_link::inbound::built(&root)).then_some(root)
}

macro_rules! corpus_or_skip {
    () => {
        match corpus() {
            Some(root) => root,
            None => {
                eprintln!(
                    "skipped: no link graph with an inbound cache — run girsa-link-import \
                     then girsa-link-types"
                );
                return;
            }
        }
    };
}

fn id(text: &str) -> SegmentId {
    text.parse().expect("a segment id")
}

/// `girsa:mishnah-berurah/58:1#1496` — the Mishnah Berurah on the time of
/// krias shema of the morning.
const MISHNAH_BERURAH: &str = "girsa:mishnah-berurah/58:1#1496";
/// The se'if it is written on.
const SHULCHAN_ARUKH: &str = "girsa:shulchan-arukh/orach-chayim/58:1#404";
/// The first mishnah of Berakhot, which is where all of it starts.
const BERAKHOT: &str = "girsa:bavli/berakhot/2a:1#1";

#[test]
fn a_ruling_traces_back_to_the_code_and_the_tur_behind_it() {
    let root = corpus_or_skip!();
    let timeline = Timeline::of(&root).expect("the catalogue");
    let repairs = Repairs::nowhere();
    let mut graph = Graph::new(&root, &timeline, &repairs);

    let trace = chain::trace(
        &mut graph,
        &id(MISHNAH_BERURAH),
        Direction::Back,
        Limits {
            depth: 2,
            width: 4,
            ..Limits::default()
        },
    );

    let arukh = trace
        .steps
        .iter()
        .position(|s| s.work() == "shulchan-arukh/orach-chayim")
        .expect("the Mishnah Berurah is written on the Shulchan Arukh");
    assert_eq!(trace.steps[arukh].depth, 1, "one hop back");
    assert!(
        trace.is_transmission(arukh),
        "the corpus calls this one a commentary, so the chain is a chain"
    );

    // And the hop behind that one: the Tur, which the Shulchan Arukh is written
    // on and which is stored in the other direction again.
    let tur = trace
        .steps
        .iter()
        .position(|s| s.work() == "tur" && s.parent == Some(arukh))
        .expect("and the Shulchan Arukh on the Tur");
    assert_eq!(trace.steps[tur].depth, 2);

    // The dates, which are what made the direction knowable at all. Both of the
    // first two works are era `AH`; on era codes alone this chain is three
    // contemporaries and does not exist.
    assert_eq!(timeline.when("mishnah-berurah").years, Some((1875, 1905)));
    assert_eq!(
        timeline.when("shulchan-arukh/orach-chayim").years,
        Some((1563, 1563))
    );
    assert_eq!(
        timeline.when("shulchan-arukh/orach-chayim").era,
        timeline.when("mishnah-berurah").era,
        "the same era code, and the years are what tell them apart"
    );
}

#[test]
fn the_same_chain_forwards_reaches_the_later_sefer_from_the_earlier() {
    let root = corpus_or_skip!();
    let timeline = Timeline::of(&root).expect("the catalogue");
    let repairs = Repairs::nowhere();
    let mut graph = Graph::new(&root, &timeline, &repairs);

    let trace = chain::trace(
        &mut graph,
        &id(SHULCHAN_ARUKH),
        Direction::Forward,
        Limits {
            depth: 1,
            width: 40,
            ..Limits::default()
        },
    );
    assert!(
        trace.steps.iter().any(|s| s.work() == "mishnah-berurah"),
        "forward from the se'if reaches the sefer written on it"
    );
    assert!(
        !trace.steps.iter().any(|s| s.work() == "tur"),
        "and not the sefer it was written on, which is the other way"
    );
}

#[test]
fn a_path_across_seven_hundred_years_is_found_and_is_honest_about_what_it_is() {
    let root = corpus_or_skip!();
    let timeline = Timeline::of(&root).expect("the catalogue");
    let repairs = Repairs::nowhere();
    let mut graph = Graph::new(&root, &timeline, &repairs);

    let found = chain::path(
        &mut graph,
        &id(BERAKHOT),
        &id(MISHNAH_BERURAH),
        Limits::default(),
    );
    let Found::Path(links) = found else {
        panic!("the first mishnah of Berakhot and the Mishnah Berurah on it are connected");
    };
    assert!(
        !links.is_empty() && links.len() <= 4,
        "and not by a long way"
    );
    assert_eq!(
        links.last().expect("a last link").at.from.work(),
        "mishnah-berurah",
        "a path ends where it was asked to end"
    );
    // What the path is worth is not what it is. Most of this graph says only
    // that two places are connected, and a reader is owed that on the answer.
    let asserted = links.iter().filter(|l| l.edge_type.is_asserted()).count();
    assert!(
        asserted < links.len(),
        "this particular path runs through unasserted links, and the caller can see it"
    );
}

#[test]
fn two_readings_of_the_first_mishnah_and_the_sefer_that_cites_both() {
    let root = corpus_or_skip!();
    let timeline = Timeline::of(&root).expect("the catalogue");
    let repairs = Repairs::nowhere();
    let mut graph = Graph::new(&root, &timeline, &repairs);

    let (forks, _) = chain::forks(
        &mut graph,
        &id(BERAKHOT),
        Limits {
            depth: 1,
            width: 25,
            ..Limits::default()
        },
    );
    let rashi_and_tosafot = forks
        .iter()
        .find(|f| {
            let works = [f.a_work(), f.b_work()];
            works.contains(&"bavli/rashi-on-berakhot")
                && works.contains(&"bavli/tosafot-on-berakhot")
        })
        .expect("Rashi and Tosafot both read the first mishnah");
    assert!(
        !rashi_and_tosafot.witnesses.is_empty(),
        "and a later sefer had to deal with both, which is what makes it a fork"
    );
    assert!(
        !rashi_and_tosafot.joined,
        "nothing in the corpus joins the two directly"
    );
}
