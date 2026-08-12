//! Three things say *which seforim relate to this one*, and they used to
//! disagree without anything noticing.
//!
//! # The three
//!
//! | | reads | asked |
//! |---|---|---|
//! | `Shelf::companions` — the picker | `companions.jsonl`, every edge type | nothing: the `commentary_on` field, in either direction |
//! | `mefarshim::Marks::of` — the tick-list | `inbound.jsonl`, `comments-on` only | `taxonomy::stands`, then a private threshold |
//! | `Beside::between` — the column | both works' shards, every edge type | nothing: the `commentary_on` field, in either direction |
//!
//! Three data sources, three rules, three separate on-disk caches with three
//! generators, and **no test that they agree**. `taxonomy::stands` says in its
//! own doc comment that it is *"the question W43's tick-list, and anything else
//! that says these are the mefarshim on this sefer, has to ask"* — and two of
//! the three were not asking it.
//!
//! # What a reader saw
//!
//! The Beit Yosef declares no base and is a mefaresh on the Tur by its shelf.
//! So it was a full mefaresh in the tick-list, an **undeclared** counted link in
//! the picker — and `app/src/mefarshim.ts` filters that field to count the
//! button, so the button read *5* over a list of forty. In the column,
//! `Relation::Linked` rather than `Declared` meant the pane never fell back to
//! lining up by address: it followed edges or it sat still.
//!
//! # What this test does and does not hold
//!
//! It does **not** collapse the three into one. They are three questions —
//! *what could I open beside this*, *who is a mefaresh on this*, *given these
//! two open, how do they line up* — and a generous offer, a strict list and a
//! placement rule are all correct answers to different things.
//!
//! What it holds is that they use **one predicate** where they mean one thing:
//! `girsa_corpus::taxonomy::settled`. A sefer the corpus places on another is
//! placed the same way by all three.

// A panic in a test is a failure report.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_app::shelf::Related;
use girsa_app::{Beside, Relation, Shelf};

const GEMARA: &str = "bavli/berakhot";
const RASHI: &str = "bavli/rashi-on-berakhot";
const TOSAFOT: &str = "bavli/tosafot-on-berakhot";

fn corpus() -> &'static Path {
    girsa_fixture::linked().root()
}

fn no_personal() -> PathBuf {
    std::env::temp_dir().join("girsa-no-personal-layer")
}

#[test]
fn the_picker_and_the_column_place_a_sefer_the_same_way() {
    // The disagreement a reader meets: offered a companion the pane then
    // refuses to follow. `Shelf::companions` marked a row `declared` from the
    // `commentary_on` field and `Joined::between` decided `Relation::Declared`
    // from the same field — which agreed by coincidence, and stopped agreeing
    // the moment either learned about the shelf. Now both ask
    // `taxonomy::settled`, so this holds by construction and says so.
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");
    let gemara = shelf.read(GEMARA).expect("Berakhot is on the shelf");

    let mut checked = 0;
    for companion in shelf.companions(GEMARA) {
        let Ok(other) = shelf.read(&companion.slug) else {
            continue;
        };
        checked += 1;
        let relation = Beside::between(&gemara, &other, root).relation();
        assert_eq!(
            companion.related(),
            matches!(relation, Relation::Declared { .. }),
            "{}: the picker says stands={:?} and the column says {relation:?}",
            companion.slug,
            companion.stands,
        );
    }
    assert!(
        checked >= 2,
        "only {checked} companions were readable — the fixture is not exercising this"
    );
}

#[test]
fn a_mefaresh_in_the_tick_list_is_a_companion_in_the_picker() {
    // The other direction, and the one the window papered over: `mefarshim.ts`
    // has a `rest` list whose own comment says it exists *because* the two
    // disagree — "mefarshim the graph knows and the metadata does not (the Ben
    // Yehoyada on Berakhot, most of Otzaria's shelf) follow, by slug, rather
    // than being dropped." A reconciliation at render time, in TypeScript, with
    // no Rust test behind it.
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");
    let marks = girsa_app::mefarshim::Marks::of(&shelf, GEMARA).expect("the inbound cache reads");

    let companions: Vec<String> = shelf
        .companions(GEMARA)
        .into_iter()
        .map(|c| c.slug)
        .collect();
    let mefarshim = marks.commentators();
    let missing: Vec<&String> = mefarshim
        .iter()
        .filter(|slug| !companions.contains(slug))
        // A mefaresh that is not on this shelf at all cannot be a companion,
        // and that is not a disagreement — it is a sefer the reader has not
        // got. The picker offers what can be opened.
        .filter(|slug| shelf.work(slug).is_some())
        .collect();
    assert!(
        missing.is_empty(),
        "a mefaresh in the tick-list that the picker never offers: {missing:?}"
    );
}

#[test]
fn a_declared_commentary_is_declared_by_all_three() {
    // Rashi on Berakhot, which states its base outright. The easy case, and the
    // one that has to keep working while the hard ones are being fixed.
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");
    let gemara = shelf.read(GEMARA).expect("Berakhot");
    let rashi = shelf.read(RASHI).expect("Rashi on Berakhot");

    let offered = shelf
        .companions(GEMARA)
        .into_iter()
        .find(|c| c.slug == RASHI)
        .expect("Rashi is offered beside Berakhot");
    assert!(
        offered.related(),
        "the picker does not relate Rashi to Berakhot"
    );
    assert_eq!(offered.stands, Some(Related::On), "and it is a mefaresh");

    assert_eq!(
        Beside::between(&gemara, &rashi, root).relation(),
        Relation::Declared {
            follower_is_commentary: true
        },
    );

    let marks = girsa_app::mefarshim::Marks::of(&shelf, GEMARA).expect("the inbound cache reads");
    assert!(
        marks.commentators().iter().any(|w| w == RASHI),
        "the tick-list does not hold Rashi: {:?}",
        marks.commentators()
    );
}

#[test]
fn the_two_commentaries_on_one_gemara_are_offered_and_placed_alike() {
    // Tosafos as well as Rashi, so this is about the rule and not about one
    // sefer that happens to work.
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");
    let gemara = shelf.read(GEMARA).expect("Berakhot");
    for slug in [RASHI, TOSAFOT] {
        let other = shelf.read(slug).expect("on the shelf");
        let offered = shelf
            .companions(GEMARA)
            .into_iter()
            .find(|c| c.slug == slug)
            .unwrap_or_else(|| panic!("{slug} is not offered beside Berakhot"));
        let relation = Beside::between(&gemara, &other, root).relation();
        assert!(
            offered.related(),
            "{slug}: the picker relates it to nothing"
        );
        assert!(
            matches!(relation, Relation::Declared { .. }),
            "{slug}: the column says {relation:?}"
        );
    }
}

#[test]
fn a_truncated_companions_list_says_it_was_truncated() {
    // `girsa-companions` keeps the 200 thickest joins per work — Berakhot is
    // joined to about 1,600 — and the number it dropped went to stdout at the
    // end of the run and into the file nowhere. So `Shelf::companions` could
    // not tell a cut list from a complete one, which is a list that silently
    // stops, which is the shape this repository refuses by name.
    let root = corpus();
    let shelf = Shelf::open(root, &no_personal()).expect("the shelf opens");
    let (kept, joined) = shelf.joins(GEMARA);
    assert!(
        joined >= kept,
        "more kept ({kept}) than were joined ({joined}) — the field is not being read"
    );
    // On the fixture nothing is truncated, so this asserts the shape rather
    // than the cut: the two numbers exist and agree, and on the real corpus
    // they part company at 200.
    assert_eq!(kept, joined, "the fixture shelf is small enough not to cut");
}
