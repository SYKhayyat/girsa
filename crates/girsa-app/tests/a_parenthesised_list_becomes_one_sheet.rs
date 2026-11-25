//! The parenthesised list becomes one sheet, in the order it was written.
//!
//! spec.md §9.7 and #63: `(berachos 2a, shabbos 3b)` is one page holding both
//! passages, and `berachos 2a, shabbos 3b` is two visits. `nav` decides which of
//! those a line is; this is what the first one produces.
//!
//! # The failure this file is arranged against
//!
//! A sheet with a **hole** in it. If a reference that cannot be found simply did
//! not appear, the sheet would read as *these are the passages you asked for* and
//! be one short — and a reader comparing it against what they typed has no way to
//! know which line vanished. So every reference the reader named is on the sheet
//! or in `unread` with a sentence, and the two lists together are exactly what
//! they wrote.
//!
//! The fixture is the tree's own (`girsa_fixture::linked()`), so the references
//! here are references to seforim that are really on a shelf rather than to
//! strings chosen to make the test pass.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use girsa_app::shelf::Shelf;
use girsa_app::source_sheet::assemble;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-sheet-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn shelf(name: &str) -> Shelf {
    Shelf::open(girsa_fixture::linked().root(), &scratch(name)).expect("the fixture shelf opens")
}

/// The references this test knows really resolve on the fixture.
const BERAKHOT: &str = "girsa:bavli/berakhot/2a:1";
const ORACH_CHAYIM: &str = "girsa:shulchan-arukh/orach-chayim/1:1";

#[test]
fn two_references_become_two_passages_in_the_order_they_were_named() {
    let shelf = shelf("two");
    let sheet = assemble(&shelf, &[BERAKHOT.to_string(), ORACH_CHAYIM.to_string()]);

    assert_eq!(sheet.unread, Vec::new(), "both references resolve here");
    assert_eq!(sheet.entries.len(), 2);
    assert_eq!(sheet.entries[0].reference, BERAKHOT);
    assert_eq!(sheet.entries[0].slug, "bavli/berakhot");
    assert_eq!(sheet.entries[1].reference, ORACH_CHAYIM);

    // The words are the passage and not the reference again: a sheet of
    // references with nothing under them is what a reader can already get from
    // the sentence they typed.
    for entry in &sheet.entries {
        assert!(
            !entry.words.trim().is_empty(),
            "{}: no words",
            entry.reference
        );
        assert!(
            !entry.title.trim().is_empty(),
            "{}: no title",
            entry.reference
        );
    }

    // And the order is the reader's, not the shelf's or the alphabet's.
    let reversed = assemble(&shelf, &[ORACH_CHAYIM.to_string(), BERAKHOT.to_string()]);
    assert_eq!(reversed.entries[0].reference, ORACH_CHAYIM);
    assert_eq!(reversed.entries[1].reference, BERAKHOT);
}

#[test]
fn a_sefer_that_is_not_on_this_shelf_is_a_hole_with_a_sentence_in_it() {
    // *You can cite a sefer you have not imported* is the commonest thing that
    // goes wrong in a library, and the sheet is the surface where it would be
    // silent.
    let shelf = shelf("missing");
    let sheet = assemble(
        &shelf,
        &[BERAKHOT.to_string(), "girsa:nonesuch/1:1".to_string()],
    );

    assert_eq!(
        sheet.entries.len(),
        1,
        "the sefer that is here is on the sheet"
    );
    assert_eq!(sheet.unread.len(), 1);
    let missing = &sheet.unread[0];
    assert!(
        missing.why.contains("not on this shelf"),
        "said in the shelf's words: {}",
        missing.why
    );
    // Nothing was invented for the sefer that is here: its entry is the same as
    // it would have been on its own.
    let alone = assemble(&shelf, &[BERAKHOT.to_string()]);
    assert_eq!(sheet.entries, alone.entries);
}

#[test]
fn a_place_a_sefer_does_not_have_is_told_apart_from_a_sefer_that_is_absent() {
    // Two different sentences because they send a reader to two different
    // places to fix it: the sefer is missing, or the place inside it is.
    let shelf = shelf("place");
    let sheet = assemble(&shelf, &["girsa:bavli/berakhot/99b:1".to_string()]);

    assert!(sheet.entries.is_empty());
    assert_eq!(sheet.unread.len(), 1);
    let why = &sheet.unread[0].why;
    assert!(!why.contains("not on this shelf"), "{why}");
    assert!(why.contains("ברכות"), "the sefer is named: {why}");
}

#[test]
fn something_that_is_not_a_reference_at_all_is_refused_rather_than_dropped() {
    let shelf = shelf("garbage");
    let sheet = assemble(&shelf, &["not a reference".to_string()]);

    assert!(sheet.entries.is_empty());
    assert!(sheet.is_empty());
    assert_eq!(sheet.unread.len(), 1);
    assert!(sheet.unread[0]
        .why
        .contains("not a reference this library can read"));
}

#[test]
fn a_line_nothing_could_be_read_from_is_empty_and_says_so() {
    // `is_empty` is what the window asks before drawing a sheet, because an empty
    // sheet and a sheet that could not be built are different things to say.
    let shelf = shelf("nothing");
    let none = assemble(&shelf, &[]);
    assert!(none.is_empty());
    assert_eq!(none.entries.len(), 0);
    assert_eq!(none.unread.len(), 0);

    let nothing_readable = assemble(&shelf, &["girsa:nonesuch/1:1".to_string()]);
    assert!(nothing_readable.is_empty());
    assert_eq!(nothing_readable.unread.len(), 1);
}
