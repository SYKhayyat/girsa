//! The acceptance test BUILDER.md W9 names, against the real shelf.
//!
//! > **Acceptance.** A sugya open with its commentary in an adjacent column;
//! > scrolling the Gemara moves the Rashi column to the matching ref.
//!
//! Berakhot 2a and Rashi on it. Every assertion here is about *where a pane
//! lands*, which is the one thing the reader notices and the one thing a
//! screenshot cannot prove.
//!
//! # Why it skips when the corpus is absent
//!
//! It reads the imported shelf, which is not committed and is not there on a
//! fresh clone. A test that failed there would be noise everybody learns to
//! ignore.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_app::{Beside, Place, Shelf};

const GEMARA: &str = "bavli/berakhot";
const RASHI: &str = "bavli/rashi-on-berakhot";
const TOSAFOT: &str = "bavli/tosafot-on-berakhot";

fn corpus() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    root.join("works/index.jsonl").is_file().then_some(root)
}

/// These read the shelf and change nothing, so the personal layer they are
/// given is one that does not exist: an empty arrangement, and none of the
/// reader's own seforim in the way of an assertion about the corpus.
fn no_personal() -> PathBuf {
    std::env::temp_dir().join("girsa-no-personal-layer")
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
    let root = corpus_or_skip!();
    let shelf = Shelf::open(&root, &no_personal()).expect("the shelf opens");

    let gemara = shelf.read(GEMARA).expect("Berakhot is on the shelf");
    let rashi = shelf
        .read(RASHI)
        .expect("Rashi on Berakhot is on the shelf");
    let beside = Beside::between(&gemara, &rashi, &root);

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
    let root = corpus_or_skip!();
    let shelf = Shelf::open(&root, &no_personal()).expect("the shelf opens");
    let gemara = shelf.read(GEMARA).expect("Berakhot");
    let rashi = shelf.read(RASHI).expect("Rashi on Berakhot");
    let beside = Beside::between(&gemara, &rashi, &root);

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
    let root = corpus_or_skip!();
    let shelf = Shelf::open(&root, &no_personal()).expect("the shelf opens");
    let Ok(gemara) = shelf.read(GEMARA) else {
        return;
    };
    let Ok(other) = shelf.read("mishnah-berakhot") else {
        return;
    };

    let beside = Beside::between(&gemara, &other, &root);
    if beside.relation().is_declared() {
        // If Sefaria ever declares this pair, the test's premise is gone and
        // saying so is more use than a green tick.
        println!("skipped: the corpus now declares mishnah-berakhot on bavli/berakhot");
        return;
    }
    for segment in gemara.segments.iter().take(50) {
        assert!(
            !matches!(beside.place(&segment.id), Place::At(_)) || beside.relation().is_linked(),
            "{} was placed in a sefer nothing relates it to",
            segment.id
        );
    }
}

#[test]
fn the_second_commentary_column_follows_the_same_gemara() {
    // Two columns beside one, which is what a daf looks like.
    let root = corpus_or_skip!();
    let shelf = Shelf::open(&root, &no_personal()).expect("the shelf opens");
    let gemara = shelf.read(GEMARA).expect("Berakhot");
    let Ok(tosafot) = shelf.read(TOSAFOT) else {
        println!("skipped: Tosafot on Berakhot is not on this shelf");
        return;
    };
    let beside = Beside::between(&gemara, &tosafot, &root);
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
    let root = corpus_or_skip!();
    let shelf = Shelf::open(&root, &no_personal()).expect("the shelf opens");
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
