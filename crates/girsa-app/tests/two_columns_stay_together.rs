//! The acceptance test BUILDER.md W9 names, against the real shelf.
//!
//! > **Acceptance.** A sugya open with its commentary in an adjacent column;
//! > scrolling the Gemara moves the Rashi column to the matching ref.
//!
//! Berakhot 2a and Rashi on it. Every assertion here is about *where a pane
//! lands*, which is the one thing the reader notices and the one thing a
//! screenshot cannot prove.
//!
//! # It used to skip, and a skip is why nobody noticed
//!
//! This gated on the fetched corpus and `return`ed when it was absent — so on
//! every fresh clone and in CI it printed `ok` in 0.00s having asserted nothing.
//! It runs on [`girsa_fixture`], a shelf the real importer builds from real
//! `merged.json` files in about a second, so the claim above is now checked
//! everywhere rather than nowhere.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_app::{Beside, Place, Shelf};

const GEMARA: &str = "bavli/berakhot";
const RASHI: &str = "bavli/rashi-on-berakhot";
const TOSAFOT: &str = "bavli/tosafot-on-berakhot";

/// The shelf, with Berakhot, its Rashi and its Tosafos on it.
fn corpus() -> &'static Path {
    girsa_fixture::linked().root()
}

/// These read the shelf and change nothing, so the personal layer they are
/// given is one that does not exist: an empty arrangement, and none of the
/// reader's own seforim in the way of an assertion about the corpus.
fn no_personal() -> PathBuf {
    std::env::temp_dir().join("girsa-no-personal-layer")
}

/// Where the follower pane goes, as text, when the leader is at `at`.
fn place_of(beside: &Beside, at: &str) -> Place {
    beside.place(&at.parse().expect("a segment id"))
}

fn first_id(place: &Place) -> Option<String> {
    match place {
        Place::At(ids) => ids.first().map(ToString::to_string),
        _ => None,
    }
}

#[test]
fn scrolling_the_gemara_moves_the_rashi_column_to_the_matching_place() {
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");

    let gemara = shelf.read(GEMARA).expect("Berakhot is on the shelf");
    let rashi = shelf
        .read(RASHI)
        .expect("Rashi on Berakhot is on the shelf");
    let beside = Beside::between(&gemara, &rashi, root);

    // The two are related because the corpus says so — `Rashi on Berakhot`
    // declares `base_text_titles: [Berakhot]` — not because one title contains
    // the other.
    assert!(
        beside.relation().is_declared(),
        "{:?}: nothing declares these two related, so the panes cannot follow \
         each other",
        beside.relation()
    );

    // Daf 2a, first line of the Gemara. Rashi's comments on it are addressed
    // with one level more: 2a:1:1, 2a:1:2, …
    let place = place_of(&beside, "girsa:bavli/berakhot/2a:1#1");
    assert_eq!(
        first_id(&place).as_deref(),
        Some("girsa:bavli/rashi-on-berakhot/2a:1:1#1"),
        "{place:?}"
    );

    // Scroll one line down and the column moves with it.
    let ids: Vec<String> = shelf
        .read(GEMARA)
        .expect("read")
        .segments
        .iter()
        .take(6)
        .map(|s| s.id.to_string())
        .collect();
    let mut moved = 0;
    let mut last = String::new();
    for id in &ids {
        if let Some(there) = first_id(&place_of(&beside, id)) {
            assert_ne!(there, last, "the column did not move for {id}");
            last = there;
            moved += 1;
        }
    }
    assert!(
        moved >= 3,
        "the Rashi column moved for only {moved} of the first {} lines",
        ids.len()
    );

    println!(
        "{} lines of Berakhot 2a, each with its Rashi: {last} last",
        ids.len()
    );
}

#[test]
fn a_line_with_no_rashi_on_it_leaves_the_column_where_it_was() {
    // Rashi does not comment on every line, and the honest answer is *there is
    // nothing here* — not the nearest comment. A pane that slid to the nearest
    // one would be showing the reader Rashi on a different line with nothing
    // to say it had moved (BUILDER.md rule 6, in the place a reader would
    // never check).
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");
    let gemara = shelf.read(GEMARA).expect("Berakhot");
    let rashi = shelf.read(RASHI).expect("Rashi on Berakhot");
    let beside = Beside::between(&gemara, &rashi, root);

    let bare = gemara
        .segments
        .iter()
        .find(|s| matches!(beside.place(&s.id), Place::NoPlace))
        .map(|s| s.id.to_string());

    assert!(
        bare.is_some(),
        "every single line of Berakhot has a Rashi on it, which cannot be right"
    );
    println!("no Rashi on {}", bare.unwrap_or_default());
}

#[test]
fn two_seforim_nothing_relates_do_not_drag_each_other_around() {
    // Berakhot and Rashi *on Berakhot* line up because Sefaria declares it.
    // Two works with no declaration and no edge between them line up by
    // accident — both are addressed `1:1` — and a pane that followed that
    // would show a reader one sefer while claiming to show another.
    //
    // The pair has to be genuinely unrelated for that to be a test at all. It
    // used to be Berakhot and Mishnah Berakhot, and the assertion had `|| …
    // is_linked()` on the end, so the day anything linked those two — the corpus
    // links them constantly, the mishnah is printed on the daf — the whole check
    // became `assert!(true)` and went on printing `ok`. Bereishis and Even
    // HaEzer share the address `1:1`, and nothing anywhere joins them.
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");
    let chumash = shelf.read("genesis").expect("Bereishis is on the shelf");
    let halacha = shelf
        .read("shulchan-arukh/even-haezer")
        .expect("Even HaEzer is on the shelf");

    let beside = Beside::between(&chumash, &halacha, root);
    assert!(
        !beside.relation().is_declared(),
        "the premise is gone: something now declares these two related — {:?}",
        beside.relation()
    );
    assert!(
        !beside.relation().is_linked(),
        "the premise is gone: something now links these two — {:?}",
        beside.relation()
    );
    for segment in chumash.segments.iter().take(50) {
        assert!(
            !matches!(beside.place(&segment.id), Place::At(_)),
            "{} was placed in a sefer nothing relates it to",
            segment.id
        );
    }
}

#[test]
fn the_second_commentary_column_follows_the_same_gemara() {
    // Two columns beside one, which is what a daf looks like.
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");
    let gemara = shelf.read(GEMARA).expect("Berakhot");
    // Asserted rather than skipped: Tosafos is on the fixture shelf by
    // construction, and a check that passes because it could not find what it
    // checks is the failure this whole file stopped committing.
    let tosafot = shelf
        .read(TOSAFOT)
        .expect("Tosafot on Berakhot is on the shelf");
    let beside = Beside::between(&gemara, &tosafot, root);
    assert!(beside.relation().is_declared(), "{:?}", beside.relation());

    let placed = gemara
        .segments
        .iter()
        .filter(|s| matches!(beside.place(&s.id), Place::At(_)))
        .count();
    assert!(placed > 0, "Tosafot never lands anywhere in Berakhot");
    println!("{placed} lines of Berakhot have a Tosafot");
}

#[test]
fn the_shelf_offers_the_commentaries_on_what_you_are_reading() {
    // The pane beside you has to be chosen from something. Offering all 7,189
    // works is not a choice, and offering the ones whose title looks similar is
    // a guess — so the offer is what the corpus declares plus what the link
    // graph recorded.
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");
    let beside_it = shelf.companions(GEMARA);
    assert!(
        beside_it.iter().any(|c| c.slug == RASHI),
        "Rashi is not offered beside Berakhot; {} companions offered",
        beside_it.len()
    );
    println!(
        "{} seforim are offered beside Berakhot; the first five are {:?}",
        beside_it.len(),
        beside_it
            .iter()
            .take(5)
            .map(|c| c.slug.as_str())
            .collect::<Vec<_>>()
    );
}
