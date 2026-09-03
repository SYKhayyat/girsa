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
//! # It ran nowhere, and now it runs everywhere
//!
//! This needed the corpus, the link graph and `girsa-link-types` all present, and
//! `return`ed when any was missing — so it printed `4 passed` in 0.00s on every
//! machine that had not spent an hour importing 3.4 GB. It runs on
//! [`girsa_fixture`] now, which has the same three works in the same order —
//! Mishnah Berurah on Shulchan Arukh on Tur — with the same problem in it: all
//! three are stored in the direction each was *written*, and two of them share
//! an era code, so only the years say which way time runs.
//!
//! The ids below are the fixture's own, and are found by **address** rather than
//! written down as ordinals. `girsa:mishnah-berurah/58:1#1496` was the old
//! spelling, and an ordinal in a test is a hostage to the next re-import — which
//! is the very thing `redirects.jsonl` was added to survive.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use girsa_corpus::era::Timeline;
use girsa_corpus::segment::SegmentId;
use girsa_link::chain::{self, Direction, Found, Graph, Limits};
use girsa_link::repair::Repairs;

/// The shelf: works, an imported graph, and an inbound cache over it.
fn corpus() -> &'static Path {
    girsa_fixture::linked().root()
}

/// The segment at an address, by its address.
///
/// Not by its ordinal. `girsa:mishnah-berurah/58:1#1496` is how these three were
/// written down before, and an ordinal in a test is a hostage to the next
/// re-import — which is exactly the event `redirects.jsonl` exists to absorb
/// rather than to propagate into the test suite.
fn at(root: &Path, slug: &str, address: &[&str]) -> SegmentId {
    let work = girsa_corpus::import::read_back(root, slug).expect("the work is on the shelf");
    work.segments
        .iter()
        .find(|s| s.id.path() == address)
        .map(|s| s.id.clone())
        .unwrap_or_else(|| panic!("{slug} has nothing at {address:?}"))
}

/// The Mishnah Berurah on the time of krias shema of the morning.
fn mishnah_berurah(root: &Path) -> SegmentId {
    at(root, "mishnah-berurah", &["58", "1"])
}

/// The se'if it is written on.
fn shulchan_arukh(root: &Path) -> SegmentId {
    at(root, "shulchan-arukh/orach-chayim", &["58", "1"])
}

/// The first line of Berakhot, which is where all of it starts.
fn berakhot(root: &Path) -> SegmentId {
    at(root, "bavli/berakhot", &["2a", "1"])
}

#[test]
fn a_ruling_traces_back_to_the_code_and_the_tur_behind_it() {
    let root = corpus();
    let timeline = Timeline::of(root).expect("the catalogue");
    let repairs = Repairs::nowhere();
    let mut graph = Graph::new(root, &timeline, &repairs);

    let trace = chain::trace(
        &mut graph,
        &mishnah_berurah(root),
        Direction::Back,
        Limits {
            depth: 2,
            width: 4,
            ..Limits::default()
        },
        None,
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
        .position(|s| s.work() == "tur/orach-chayim" && s.parent == Some(arukh))
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
    let root = corpus();
    let timeline = Timeline::of(root).expect("the catalogue");
    let repairs = Repairs::nowhere();
    let mut graph = Graph::new(root, &timeline, &repairs);

    let trace = chain::trace(
        &mut graph,
        &shulchan_arukh(root),
        Direction::Forward,
        Limits {
            depth: 1,
            width: 40,
            ..Limits::default()
        },
        None,
    );
    assert!(
        trace.steps.iter().any(|s| s.work() == "mishnah-berurah"),
        "forward from the se'if reaches the sefer written on it"
    );
    assert!(
        !trace.steps.iter().any(|s| s.work() == "tur/orach-chayim"),
        "and not the sefer it was written on, which is the other way"
    );
}

#[test]
fn a_path_across_seven_hundred_years_is_found_and_is_honest_about_what_it_is() {
    let root = corpus();
    let timeline = Timeline::of(root).expect("the catalogue");
    let repairs = Repairs::nowhere();
    let mut graph = Graph::new(root, &timeline, &repairs);

    let found = chain::path(
        &mut graph,
        &berakhot(root),
        &mishnah_berurah(root),
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
    let root = corpus();
    let timeline = Timeline::of(root).expect("the catalogue");
    let repairs = Repairs::nowhere();
    let mut graph = Graph::new(root, &timeline, &repairs);

    let (forks, _) = chain::forks(
        &mut graph,
        &berakhot(root),
        Limits {
            depth: 1,
            width: 25,
            ..Limits::default()
        },
        None,
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
