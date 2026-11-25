//! #48 — a word, and every place the shelf says it.
//!
//! The acceptance for the item is a reader who selects a word and gets *all* of
//! it: every instance, in the shelf's own order, each one able to say where in
//! the printed line it sits. So these tests are about three things a concordance
//! can get wrong while still looking right:
//!
//! 1. **It lists fewer places than it counted** — a bounded list that does not
//!    say it was cut reads as *this is all of them*.
//! 2. **It ranks**, which turns *every place* into *the best places* without
//!    changing the header.
//! 3. **It widens**, which turns *this word* into *words like this word* — the
//!    one thing a concordance must never do, because the whole value of a
//!    concordance is that the count is of one form.

// A panic in a test is a failure report.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_corpus::work::{Source, Work};
use girsa_search::bar::Bar;
use girsa_search::concordance::{concordance, Asking, AT_MOST};
use girsa_search::facets::Catalogue;
use girsa_search::index::SearchIndex;
use girsa_search::scope::Scope;

/// Three seforim, and one word that appears in two of them, twice in the first.
///
/// Small enough to count by hand, which is the point: every assertion here is
/// against a number this file can derive from the fixture rather than from the
/// index.
const BAVLI: [&str; 3] = [
    "יתגבר כארי לעמוד בבוקר",
    "לעולם יהא אדם עז כנמר קל כנשר",
    "יתגבר כארי אף שהוא קשה",
];

const SHULCHAN_ARUKH: [&str; 2] = ["יתגבר כארי לעבוד את בוראו", "ויכוין לבו לשמים"];

const MISHNAH_BERURAH: [&str; 2] = ["כארי — ולא כאריה", "ויבטח בה׳"];

struct Shelf {
    bar: Bar,
}

fn work(slug: &str, title: &str) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: title.to_string(),
        en_title: slug.to_string(),
        categories: Vec::new(),
        order: Vec::new(),
        source: Source::Sefaria,
        origin: PathBuf::new(),
        schema: None,
        author: None,
        era: None,
        comp_date: None,
        version: None,
        he_sections: Vec::new(),
        commentary_on: Vec::new(),
    }
}

fn segment(slug: &str, n: u32, text: &str) -> Segment {
    Segment {
        id: SegmentId::new(slug, vec!["1".to_string(), n.to_string()], Ordinal::root(n)),
        kind: SegmentKind::Text,
        text: text.to_string(),
        anchors: Vec::new(),
    }
}

fn shelf() -> Shelf {
    let works = vec![
        work("bavli/berakhot", "ברכות"),
        work("shulchan-arukh/orach-chayim", "שולחן ערוך אורח חיים"),
        work("mishnah-berurah", "משנה ברורה"),
    ];
    let lines: [(&str, &[&str]); 3] = [
        ("bavli/berakhot", &BAVLI),
        ("shulchan-arukh/orach-chayim", &SHULCHAN_ARUKH),
        ("mishnah-berurah", &MISHNAH_BERURAH),
    ];
    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    for (slug, texts) in lines {
        for (i, text) in texts.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let n = (i + 1) as u32;
            writer
                .add(&segment(slug, n, text), &[])
                .expect("adding a segment");
        }
    }
    writer.commit().expect("committing");
    index.reload().expect("reloading");
    Shelf {
        bar: Bar::new(
            index,
            Catalogue::of(&works),
            &std::env::temp_dir().join("girsa-48-concordance"),
            None,
        ),
    }
}

/// Every place in the fixture that says `כארי`, counted from the fixture.
fn places_of_kari() -> usize {
    BAVLI
        .iter()
        .chain(SHULCHAN_ARUKH.iter())
        .chain(MISHNAH_BERURAH.iter())
        .filter(|line| {
            line.split_whitespace()
                .any(|word| word.trim_matches('־') == "כארי")
        })
        .count()
}

#[test]
fn every_place_is_listed_and_the_count_is_the_count() {
    let shelf = shelf();
    let found = concordance(&shelf.bar, "כארי", Asking::default()).expect("a concordance");
    let expected = places_of_kari();
    assert_eq!(found.total, expected, "{found:?}");
    assert_eq!(found.shown, expected, "the list is every place, not a page");
    assert!(!found.is_cut());
    // Three seforim hold it, which is what makes this a shelf-wide answer rather
    // than a list of one sefer's repetitions.
    assert_eq!(found.seforim, 3, "{:?}", found.places);
}

#[test]
fn the_places_are_in_reading_order_not_in_score_order() {
    // A concordance has no best hit: every place matched the same form. So the
    // order is the shelf's — and it is *checked*, because equal scores make both
    // orders look plausible and only one of them stays put when the index is
    // rebuilt.
    let shelf = shelf();
    let found = concordance(&shelf.bar, "כארי", Asking::default()).expect("a concordance");
    let ids: Vec<SegmentId> = found.places.iter().map(|place| place.id.clone()).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "reading order");
}

#[test]
fn a_place_says_where_in_the_printed_line_the_form_sits() {
    // The acceptance's other half: a hit a reader lands on must be a position,
    // not a bare verse. `marks` are offsets into the printed text, so they are
    // checked against the fixture's own line rather than against `shown`, which
    // has brackets added and characters dropped.
    let shelf = shelf();
    let found = concordance(&shelf.bar, "כארי", Asking::default()).expect("a concordance");
    let place = found
        .places
        .iter()
        .find(|place| place.work == "bavli/berakhot")
        .expect("a place in the Bavli");
    assert_eq!(place.marks.len(), 1, "{place:?}");
    let (from, to) = place.marks[0];
    let line = BAVLI[0];
    // Byte spans, which is what `Marker::marks` speaks — slicing chars here would
    // read the mark as pointing one word further on, which is how this test was
    // first written and how it first failed.
    let marked = &line[from..to];
    assert_eq!(
        girsa_hebrew::normalize(marked),
        found.indexed,
        "the mark covers the word that answered, and nothing else"
    );
    // And the snippet shows it, bracketed — a list of forty places with no
    // visible match is a list a reader has to land in all forty to read.
    assert!(place.shown.contains(&format!("[{marked}]")), "{place:?}");
}

#[test]
fn a_pointed_selection_is_answered_about_the_bare_word_and_says_so() {
    // The index holds normal forms (W11), so `וַיֹּאמֶר` and `ויאמר` are one
    // question. What must not happen is the *reader* being told the pointed word
    // is what was counted.
    let shelf = shelf();
    let found = concordance(&shelf.bar, "כָּאֲרִי", Asking::default()).expect("a concordance");
    assert_eq!(found.indexed, "כארי");
    assert!(!found.as_typed(), "{found:?}");
    assert_eq!(found.total, places_of_kari());
    assert!(found.describe().contains("כארי"), "{}", found.describe());
}

#[test]
fn a_phrase_is_refused_by_name_and_told_where_to_go() {
    // *Every place this word is said* and *every place these two words are said
    // together* are different questions with different counts. Answering the
    // first when the second was asked is the failure, so it is refused and the
    // two commands that do answer sentences are named.
    let shelf = shelf();
    let refused = concordance(&shelf.bar, "יתגבר כארי", Asking::default())
        .expect_err("two words are not a concordance");
    assert!(refused.contains("find"), "{refused}");
    assert!(refused.contains("where-from"), "{refused}");
}

#[test]
fn a_bound_that_cuts_the_list_says_that_it_cut_the_list() {
    let shelf = shelf();
    let found = concordance(
        &shelf.bar,
        "כארי",
        Asking {
            at_most: 2,
            ..Asking::default()
        },
    )
    .expect("a concordance");
    assert_eq!(
        found.total,
        places_of_kari(),
        "the count is still the count"
    );
    assert_eq!(found.shown, 2);
    assert!(found.is_cut());
    // The count is not the only thing that survives the bound: the seforim are
    // counted over the whole result set too. Deriving them from the two places
    // that fit said *975 places in 5 seforim* on the real shelf, because the
    // bound was recomputing a fact about a set it had already cut.
    assert_eq!(found.seforim, 3, "{:?}", found.places);
    let said = found.describe();
    assert!(said.contains("מוצגים 2"), "{said}");
    assert!(said.contains("ב־3 ספרים"), "{said}");
}

#[test]
fn a_form_the_shelf_does_not_hold_is_zero_and_is_not_widened_into_a_guess() {
    // `כאריי` is one letter away from a word the shelf has three times, and the
    // ladder's spelling rung reaches `כארי` from it — the search bar finds two
    // places for this, offered and counted as an offer. A concordance may not: a
    // count of near matches presented as *the places this word is said* is a
    // wrong count that looks like a right one.
    //
    // (The pair is measured, not assumed: a first draft used `כאריות` and stayed
    // green when the mode was switched to Smart, because nothing in the ladder
    // reaches it. A guard that cannot fail is the shape this repository refuses
    // by name.)
    let shelf = shelf();
    let found = concordance(&shelf.bar, "כאריי", Asking::default()).expect("a concordance");
    assert_eq!(found.total, 0, "{found:?}");
    assert!(found.places.is_empty());
    assert_eq!(found.describe(), "אין בשום ספר");
}

#[test]
fn a_scope_narrows_the_places_and_the_count_together() {
    // The pane's concordance and the shelf-wide one are one question with one
    // chip different (spec.md §9.8). If the count did not move with the list,
    // the header would be describing a search that did not happen.
    let shelf = shelf();
    let only = Scope::everything().only(["bavli/berakhot".to_string()], "בבלי");
    let found = concordance(
        &shelf.bar,
        "כארי",
        Asking {
            scope: Some(only),
            ..Asking::default()
        },
    )
    .expect("a concordance");
    assert_eq!(found.seforim, 1);
    assert!(
        found
            .places
            .iter()
            .all(|place| place.work == "bavli/berakhot"),
        "{:?}",
        found.places
    );
    assert_eq!(
        found.total,
        BAVLI
            .iter()
            .filter(|line| line
                .split_whitespace()
                .any(|word| word.trim_matches('־') == "כארי"))
            .count()
    );
}

#[test]
fn the_default_bound_is_the_one_this_module_states() {
    // A default written in one place and read in another is how a caller ends up
    // asking for a different number than the document says. `Asking::bound` is
    // the one reader.
    assert_eq!(Asking::default().bound(), AT_MOST);
    assert_eq!(
        Asking {
            at_most: 7,
            ..Asking::default()
        }
        .bound(),
        7
    );
}
