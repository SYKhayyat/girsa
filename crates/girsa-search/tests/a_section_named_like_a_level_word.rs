//! A sefer whose schema calls a section `שער`, and a citation into it.
//!
//! # The wrong landing that looks exactly like a right one
//!
//! `שער` is one of the words the resolver reads as a level label — `שער א'` is
//! gate one, and the word carries no address of its own. That is the right
//! reading of nearly every citation ever written, and it is wrong for a sefer
//! whose schema **names a section** by that word. `אברבנאל על יחזקאל שער א'`
//! then resolves to `1`, which is perek א' of the commentary: a real place, a
//! real segment, and not the one anybody asked for.
//!
//! This module's subject is the only kind of failure `citation.rs` calls the
//! worst kind there is — it resolves, it opens a page, and nothing about it
//! looks like an error.
//!
//! # Who decides, and on what
//!
//! Not this crate and not the resolver: **the schema**, and it has to say two
//! things before anything changes.
//!
//! * `שער` **is** the title of a section of this work.
//! * `שער` is **not** a level name this work uses — its levels are `פרק` and
//!   `פסוק`, so the label reading is labelling with a word this sefer never
//!   labels anything with.
//!
//! Either one alone is a guess. Both together are the schema answering, which
//! is why `girsa_ref::resolve::resolve_labels_as_names` exists at all: the
//! ordinary reading throws the word away, and a caller holding the schema
//! cannot ask about a word that is gone.
//!
//! Measured on a real shelf: 6,955 → 7,150 of 7,627 chalakim reachable by name.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use girsa_ref::resolve::Context;
use girsa_search::citation::Citations;

fn citations() -> Citations {
    let shelf = girsa_fixture::shelf();
    Citations::open(shelf.root(), Some(shelf.personal())).expect("a lexicon")
}

fn lands_on(typed: &str) -> String {
    let found = citations().look_up(typed, &Context::default());
    match found.only() {
        Some(place) => place.run.first.to_string(),
        None => format!(
            "NOT ONE PLACE: {} places, {} unrefuted",
            found.places.len(),
            found.unrefuted()
        ),
    }
}

#[test]
fn a_section_named_by_a_level_word_is_reached_by_that_name() {
    // Before: `girsa:abarbanel-on-ezekiel/1:1`, the first pasuk of perek א' of
    // the default node. Both are real segments, which is the whole difficulty.
    assert_eq!(
        lands_on("אברבנאל על יחזקאל שער א"),
        "girsa:abarbanel-on-ezekiel/gate:1#3"
    );
}

#[test]
fn the_ordinary_reading_of_a_label_is_untouched() {
    // The same sefer, addressed the way the other 7,000 are. If this ever
    // changes, the fix above has stopped being a second opinion and started
    // being a different resolver.
    assert_eq!(
        lands_on("אברבנאל על יחזקאל הקדמה ב"),
        "girsa:abarbanel-on-ezekiel/introduction:2#2"
    );
    assert_eq!(
        lands_on("אברבנאל על יחזקאל פרק א פסוק א"),
        "girsa:abarbanel-on-ezekiel/1:1#4"
    );
    // And a label word this schema *does* use stays a label: `פרק` is one of
    // this work's own level names, so no second reading is taken even though
    // the word is a name-shaped token.
    assert_eq!(
        lands_on("שולחן ערוך אורח חיים סימן א סעיף א"),
        "girsa:shulchan-arukh/orach-chayim/1:1#1"
    );
}
