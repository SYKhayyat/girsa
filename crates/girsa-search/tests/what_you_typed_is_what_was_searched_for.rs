//! W12 — Torat Emet, the default mode.
//!
//! One promise, and everything here is a way of checking it: **what you typed
//! is what was searched for.** Nothing is stemmed, expanded or guessed
//! (spec.md §9.3). The only thing that happened to your words is that their
//! nikud came off, which removes marks nobody types and can never cause a match
//! you would not want (§9.1).
//!
//! The operators are the ones that actually get used in learning:
//!
//! - the word **is** this word;
//! - the word **contains** these letters — `קדש` finding `המקדש`;
//! - **these letters in this order** with others between;
//! - these words **within X words** of each other.
//!
//! Every query also carries a plan saying exactly what was asked of the index,
//! because a mode whose promise is *no surprises* has to be able to show its
//! work. §9.6's ladder — the widening, offered with counts — is W13's, and a
//! zero here stays a zero.

// A panic in a test is a failure report.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_search::index::{IndexError, SearchIndex};
use girsa_search::torat_emet::{Match, Query, Together};

fn segment(n: u32, text: &str) -> Segment {
    Segment {
        id: SegmentId::new(
            "shas",
            vec![n.to_string(), "1".to_string()],
            Ordinal::root(n),
        ),
        kind: SegmentKind::Text,
        text: text.to_string(),
    }
}

/// Lines chosen because each one is a way the literal mode could quietly stop
/// being literal.
fn shelf() -> Vec<&'static str> {
    vec![
        // 1 · the word, plain.
        "שבת קודש",
        // 2 · the same word wearing a prefix. Peeling it is a widening.
        "ובשבת אין קורין",
        // 3 · `קדש` inside longer words.
        "בית המקדש עומד",
        // 4 · `ק…ד…ש` with letters between: `Letters` finds it, `Contains` must not.
        "אמירת קידוש במקומו",
        // 5 · two words, adjacent.
        "יתגבר כארי לעמוד בבוקר",
        // 6 · the same two words with two others between them.
        "יתגבר האדם כמו כארי לעמוד",
        // 7 · the same two, reversed and adjacent.
        "כארי יתגבר",
        // 8 · the same two, far apart.
        "יתגבר אדם בכל כוחו ויעמוד בבוקר כארי",
        // 9 · an abbreviation, and the words it stands for, in different lines.
        "וכן פסק שו\"ע שם",
        // 10 · the expansion. `שו"ע` must not reach this line in this mode.
        "שולחן ערוך סימן א",
    ]
}

fn loaded() -> SearchIndex {
    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    for (i, text) in shelf().into_iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        writer
            .add(&segment(i as u32 + 1, text))
            .expect("adding a segment");
    }
    writer.commit().expect("committing");
    index.reload().expect("reloading");
    index
}

/// Which lines of [`shelf`] came back, by their number in it.
fn lines(index: &SearchIndex, query: &Query) -> Vec<u32> {
    let found = index.search(query).expect("a search");
    assert_eq!(
        found.total,
        found.hits.len(),
        "the shelf is smaller than the page"
    );
    let mut out: Vec<u32> = found
        .hits
        .iter()
        .map(|hit| {
            hit.id
                .path()
                .first()
                .and_then(|p| p.parse().ok())
                .unwrap_or(0)
        })
        .collect();
    out.sort_unstable();
    out
}

// ---------------------------------------------------------------------------
// The promise
// ---------------------------------------------------------------------------

#[test]
fn the_plan_says_exactly_what_was_searched_for() {
    // The acceptance of W12. For every input, the words asked of the index are
    // the words that were typed with their marks off — no peeled prefix, no
    // expansion, no root, nothing added and nothing dropped.
    for typed in [
        "מֵאֵימָתַי קוֹרִין",
        "ובשבת",
        "שו\"ע",
        "רמב\"ם הלכות תפילה",
        "כהן",
    ] {
        let plan = Query::new(typed).plan();
        let expected: Vec<String> = girsa_hebrew::normalize(typed)
            .split_whitespace()
            .map(str::to_string)
            .collect();
        assert_eq!(plan.words, expected, "for {typed:?}");
        // In the literal mode the pattern *is* the word: no `.*`, no
        // alternation, nothing that could match something else.
        assert_eq!(plan.patterns, expected, "for {typed:?}");
        assert_eq!(plan.slop, 0);
        assert_eq!(plan.orderings, 1);
    }
}

#[test]
fn a_word_is_matched_as_written_and_not_as_a_stem() {
    // `ובשבת` is on the shelf and `שבת` is on it too, as its own word. A
    // literal search for `שבת` finds the second and not the first. Getting to
    // the first is W13's offer, and it is one the reader makes.
    let index = loaded();
    assert_eq!(lines(&index, &Query::new("שבת")), [1]);
    assert_eq!(lines(&index, &Query::new("ובשבת")), [2]);
}

#[test]
fn an_abbreviation_is_not_expanded_and_neither_is_the_expansion() {
    let index = loaded();
    assert_eq!(lines(&index, &Query::new("שו\"ע")), [9]);
    assert_eq!(lines(&index, &Query::new("שולחן ערוך")), [10]);
}

#[test]
fn a_zero_stays_a_zero() {
    // spec.md §9.6: in the default mode the engine offers the ladder; it never
    // climbs it. Nothing here may quietly return the near miss.
    let index = loaded();
    let found = index.search(&Query::new("טרף")).expect("a search");
    assert_eq!(found.total, 0);
    assert!(found.hits.is_empty());
    assert_eq!(
        found.asked.words,
        ["טרפ"],
        "the word, unchanged, with its final folded"
    );
}

// ---------------------------------------------------------------------------
// The operators
// ---------------------------------------------------------------------------

#[test]
fn the_word_contains_these_letters() {
    // BUILDER.md W12's own example: `קדש` → `המקדש`. This is not stemming —
    // the reader asked for *contains*, and gets exactly the words that do.
    let index = loaded();
    let query = Query::new("קדש").matching(Match::Contains);
    assert_eq!(lines(&index, &query), [3]);
    // And it stops there. `קודש` and `קידוש` have a letter in between, so the
    // letters are not *contained* — that is the next operator, and the reader
    // chooses it.
}

#[test]
fn these_letters_in_this_order_with_others_between() {
    let index = loaded();
    let query = Query::new("קדש").matching(Match::Letters);
    // קודש · המקדש · קידוש — ק then ד then ש, whatever is between.
    assert_eq!(lines(&index, &query), [1, 3, 4]);
}

#[test]
fn a_phrase_is_adjacent_and_in_that_order() {
    let index = loaded();
    let query = Query::new("יתגבר כארי").together(Together::Phrase);
    assert_eq!(lines(&index, &query), [5]);
}

#[test]
fn words_within_x_words_of_each_other() {
    let index = loaded();
    // Adjacent, and two words apart, and reversed — all within two.
    let near_two = Query::new("יתגבר כארי").together(Together::Near { words: 2 });
    assert_eq!(lines(&index, &near_two), [5, 6, 7]);

    // Line 8 has five words between them. Asking for two must not reach it,
    // and asking for five must.
    let near_five = Query::new("יתגבר כארי").together(Together::Near { words: 5 });
    assert_eq!(lines(&index, &near_five), [5, 6, 7, 8]);
}

#[test]
fn near_does_not_care_which_word_came_first() {
    // "within X words of each other" says nothing about order, so neither does
    // this. Line 7 is the pair reversed.
    let index = loaded();
    let query = Query::new("יתגבר כארי").together(Together::Near { words: 0 });
    assert_eq!(lines(&index, &query), [5, 7]);
}

#[test]
fn contains_can_be_asked_for_as_a_phrase() {
    // The two kinds of operator compose: letters inside a word, and words
    // beside each other. Nothing about this is a widening — both halves were
    // asked for.
    let index = loaded();
    let query = Query::new("גבר ארי")
        .matching(Match::Contains)
        .together(Together::Phrase);
    assert_eq!(lines(&index, &query), [5]);
}

#[test]
fn every_word_has_to_be_there() {
    // The default shape is *all of these words, anywhere in the segment* — an
    // AND, not an OR. An OR would return pages that do not have what you asked
    // for, which is the other way a search box becomes untrustworthy.
    let index = loaded();
    assert_eq!(lines(&index, &Query::new("יתגבר כארי")), [5, 6, 7, 8]);
    assert!(lines(&index, &Query::new("יתגבר בתרומתן")).is_empty());
}

#[test]
fn an_empty_query_finds_nothing_rather_than_everything() {
    let index = loaded();
    for typed in ["", "   ", "־ ׃"] {
        let found = index.search(&Query::new(typed)).expect("a search");
        assert_eq!(found.total, 0, "for {typed:?}");
    }
}

// ---------------------------------------------------------------------------
// Refusals — a partial answer is worse than a refused one
// ---------------------------------------------------------------------------

#[test]
fn a_pattern_that_matches_too_much_is_refused_rather_than_quietly_cut() {
    // A `contains` pattern expands to every term that matches it, and there is
    // a ceiling. Past it the index must say so: a phrase search silently run
    // over the first N of the matching words would return a subset of the
    // truth and look like the whole of it.
    let index = loaded();
    let query = Query::new("א ב")
        .matching(Match::Letters)
        .together(Together::Phrase)
        .with_max_expansions(1);
    match index.search(&query) {
        Err(IndexError::TooBroad { .. }) => {}
        other => panic!("expected a refusal, got {:?}", other.map(|f| f.total)),
    }
}

#[test]
fn an_unordered_near_of_too_many_words_is_refused_with_its_reason() {
    // Order-free proximity is checked one ordering at a time, and the number of
    // orderings grows factorially. Rather than run some of them and call it an
    // answer, the limit is stated.
    let index = loaded();
    let query = Query::new("א ב ג ד ה ו").together(Together::Near { words: 3 });
    match index.search(&query) {
        Err(IndexError::TooManyWords { words, limit }) => {
            assert_eq!(words, 6);
            assert!(limit < 6);
        }
        other => panic!("expected a refusal, got {:?}", other.map(|f| f.total)),
    }
}

#[test]
fn one_word_is_a_legal_phrase_and_a_legal_proximity() {
    // A reader with the proximity chip up who types one word gets that word,
    // not a panic. Tantivy's phrase queries assert on fewer than two terms.
    let index = loaded();
    for together in [
        Together::Phrase,
        Together::Near { words: 3 },
        Together::Anywhere,
    ] {
        let query = Query::new("קידוש").together(together);
        assert_eq!(lines(&index, &query), [4], "for {together:?}");
    }
}
