//! W13 — the relaxation ladder, and Smart mode.
//!
//! Two columns of one table (spec.md §9.6), and the difference between them is
//! the whole work order:
//!
//! | Mode | On zero results |
//! |---|---|
//! | Torat Emet (default) | **Offer** the ladder with counts. Never auto-apply. |
//! | Smart | Auto-relax in order, announce the change, one-click undo. |
//!
//! The rule underneath both: **the engine never changes your query without you
//! knowing.** Auto-applying is acceptable in Smart because widening is the
//! mode's declared purpose and it always reports itself. In the default mode it
//! is not — there the count beside `[try other forms — 7]` is computed *before*
//! the click, so the offer is informative on its own and the reader learns
//! there are seven other forms without leaving the literal mode.

// A panic in a test is a failure report.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_hebrew::VariantKind;
use girsa_search::index::{IndexError, SearchIndex};
use girsa_search::ladder::{Rule, Rung, Standing, Widened};
use girsa_search::smart::Smart;
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
        // W34's mined anchors: a fixture types its own text, so none.
        anchors: Vec::new(),
    }
}

/// Lines chosen so that each rung of the ladder has exactly one line only it
/// can reach — which is what makes a count beside an offer checkable by hand.
fn shelf() -> Vec<&'static str> {
    vec![
        // 1 · the word, plain.
        "שבת קודש",
        // 2 · the same word wearing prefixes.
        "ובשבת אין קורין",
        // 3 · `קדש` inside a longer word.
        "בית המקדש עומד",
        // 4 · `ק…ד…ש` with letters between.
        "אמירת קידוש במקומו",
        // 5 · two words, adjacent, and a third further on.
        "יתגבר כארי לעמוד בבוקר",
        // 6 · the same two with two others between them.
        "יתגבר האדם כמו כארי לעמוד",
        // 7 · the same two, reversed and adjacent.
        "כארי יתגבר",
        // 8 · the same two, far apart.
        "יתגבר אדם בכל כוחו ויעמוד בבוקר כארי",
        // 9 · an abbreviation.
        "וכן פסק שו\"ע שם",
        // 10 · what it stands for, written out.
        "שולחן ערוך סימן א",
        // 11 · a word that appears **only** under four stacked prefixes. The
        //      prefix rung is the only way to reach it from `מלך`.
        "וכשהמלך יושב על כסאו",
        // 12 · ktiv male, where the query would be written chaser.
        "כוהן גדול נכנס לפני ולפנים",
        // 13 · gershayim in the text, where a reader types without them.
        "וכן דעת רמב\"ם בפירוש המשנה",
    ]
}

fn loaded() -> SearchIndex {
    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    for (i, text) in shelf().into_iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        writer
            .add(&segment(i as u32 + 1, text), &[])
            .expect("adding a segment");
    }
    writer.commit().expect("committing");
    index.reload().expect("reloading");
    index
}

fn line_of(hit: &girsa_search::index::Hit) -> u32 {
    hit.id
        .path()
        .first()
        .and_then(|p| p.parse().ok())
        .unwrap_or(0)
}

/// Which lines of [`shelf`] a literal query returns.
fn lines(index: &SearchIndex, query: &Query) -> Vec<u32> {
    let found = index.search(query).expect("a search");
    let mut out: Vec<u32> = found.hits.iter().map(line_of).collect();
    out.sort_unstable();
    out
}

/// Which lines a widened query returns.
fn wide_lines(index: &SearchIndex, widened: &Widened) -> Vec<u32> {
    let found = index.search_widened(widened).expect("a widened search");
    let mut out: Vec<u32> = found.hits.iter().map(line_of).collect();
    out.sort_unstable();
    out
}

// ---------------------------------------------------------------------------
// The default mode: offered, never applied
// ---------------------------------------------------------------------------

#[test]
fn a_zero_result_query_is_offered_the_ladder_with_counts_computed_up_front() {
    // The acceptance of W13. `מלך` is on the shelf only inside `וכשהמלך`, so
    // the literal mode finds nothing — and says so, and offers the rung that
    // would reach it, with the number already worked out.
    let index = loaded();
    let typed = Query::new("מלך");
    let found = index.search(&typed).expect("a search");
    assert_eq!(found.total, 0, "the literal mode must not find it");

    let offers = index.offers(&typed);
    let prefixes = offers
        .offers
        .iter()
        .find(|o| o.rung == Rung::Forms(VariantKind::PrefixPeeled))
        .expect("an offer to peel prefixes");
    assert_eq!(prefixes.count, 1, "one line, counted before any click");
    assert_eq!(prefixes.label, "other forms");
}

#[test]
fn the_count_beside_an_offer_is_exactly_what_clicking_it_produces() {
    // The promise and the result cannot be allowed to disagree: a chip that
    // says 7 and then shows 5 is worse than no chip, because it teaches the
    // reader that the numbers are decoration.
    let index = loaded();
    for typed in ["מלך", "כהן", "רמבם", "שו\"ע", "שבת"] {
        for offer in index.offers(&Query::new(typed)).offers {
            let found = index
                .search_widened(&offer.widened)
                .expect("the offer, applied");
            assert_eq!(
                found.total, offer.count,
                "{typed:?} · {:?} promised {} and produced {}",
                offer.rung, offer.count, found.total
            );
        }
    }
}

#[test]
fn asking_what_is_on_offer_applies_nothing() {
    // Computing the counts runs queries. None of them may leak into the
    // reader's result set: a zero stays a zero until the reader clicks.
    let index = loaded();
    let typed = Query::new("מלך");
    let _ = index.offers(&typed);
    let after = index.search(&typed).expect("a search");
    assert_eq!(after.total, 0);
    assert!(after.hits.is_empty());
    assert!(
        after.widening.is_none(),
        "the literal mode never reports a widening"
    );
}

#[test]
fn an_offer_that_would_find_nothing_is_never_shown() {
    // spec.md §9.6's interface is `[try other forms — 7]`. There is no such
    // thing as `[try other forms — 0]`: an offer that leads nowhere is noise
    // that teaches the reader to ignore the row.
    let index = loaded();
    for typed in ["מלך", "כהן", "רמבם", "שו\"ע", "שבת", "בתרומתן", ""] {
        for offer in index.offers(&Query::new(typed)).offers {
            assert!(offer.count > 0, "{typed:?} offered {:?} at 0", offer.rung);
        }
    }
}

#[test]
fn an_offer_only_ever_widens() {
    // Every rung is a relaxation. A rung that dropped a line the literal query
    // found would be a different search wearing the word "more".
    let index = loaded();
    for typed in ["שבת", "שו\"ע", "כהן", "יתגבר"] {
        let query = Query::new(typed);
        let literal: Vec<u32> = lines(&index, &query);
        for offer in index.offers(&query).offers {
            let widened = wide_lines(&index, &offer.widened);
            for line in &literal {
                assert!(
                    widened.contains(line),
                    "{typed:?} · {:?} lost line {line}",
                    offer.rung
                );
            }
        }
    }
}

#[test]
fn each_rung_reaches_the_line_that_only_it_can_reach() {
    let index = loaded();
    let cases = [
        // typed, rung, the line only that rung reaches
        ("מלך", Rung::Forms(VariantKind::PrefixPeeled), 11),
        ("כהן", Rung::Forms(VariantKind::KtivSwapped), 12),
        ("רמבם", Rung::Forms(VariantKind::GershayimDropped), 13),
        ("שו\"ע", Rung::Forms(VariantKind::AbbreviationExpanded), 10),
    ];
    for (typed, rung, line) in cases {
        let widened = Widened::new(Query::new(typed), [rung]);
        let reached = wide_lines(&index, &widened);
        assert!(
            reached.contains(&line),
            "{typed:?} · {rung:?} did not reach line {line}; it reached {reached:?}"
        );
    }
}

#[test]
fn widening_the_proximity_is_the_last_rung_and_it_is_a_rung() {
    // `יתגבר בבוקר` is not a phrase anywhere on the shelf. Widening to the
    // whole passage finds the two lines that have both words.
    let index = loaded();
    let typed = Query::new("יתגבר בבוקר").together(Together::Phrase);
    assert!(lines(&index, &typed).is_empty());

    let offers = index.offers(&typed);
    let proximity = offers
        .offers
        .iter()
        .find(|o| o.rung == Rung::Proximity)
        .expect("an offer to widen the proximity");
    assert_eq!(proximity.count, 2);
    assert_eq!(wide_lines(&index, &proximity.widened), [5, 8]);
}

#[test]
fn dropping_nikud_is_never_an_offer_because_it_has_already_happened() {
    // The first rung of spec.md §9.6's ladder is climbed at index time, in
    // every mode, with no toggle (§9.1). Offering it would be offering to do
    // something that is already done.
    assert_eq!(Rung::Nikud.standing(), Standing::Climbed);
    let index = loaded();
    for typed in ["מלך", "שבת", "מֵאֵימָתַי"] {
        let offers = index.offers(&Query::new(typed));
        assert!(
            offers.offers.iter().all(|o| o.rung != Rung::Nikud),
            "{typed:?} was offered a rung it is already standing on"
        );
    }
}

#[test]
fn the_root_rung_is_named_by_the_spec_and_is_not_built_and_says_so() {
    // spec.md §9.6's ladder has a root rung; §9.4 defers morphology on purpose,
    // because there is no rabbinic-Hebrew analyser to build it on. A gap that
    // is named is a gap a reader can act on. A gap that is silent is the thing
    // §9.7 forbids in the neighbouring feature.
    assert!(matches!(Rung::Root.standing(), Standing::Deferred(_)));
    let index = loaded();
    let offers = index.offers(&Query::new("מלך"));
    assert!(offers.deferred.contains(&Rung::Root));
    assert!(offers.offers.iter().all(|o| o.rung != Rung::Root));
}

#[test]
fn the_rungs_are_in_the_order_the_spec_sets_out() {
    // "drop nikud → other forms → root → expand abbreviations → widen
    // proximity" (spec.md §9.6). Smart climbs them in this order and the offers
    // are shown in it, so the order is data rather than four call sites that
    // happen to agree today.
    assert_eq!(
        Rung::ALL,
        [
            Rung::Nikud,
            Rung::Forms(VariantKind::PrefixPeeled),
            Rung::Forms(VariantKind::KtivSwapped),
            Rung::Forms(VariantKind::GershayimDropped),
            Rung::Root,
            Rung::Forms(VariantKind::AbbreviationExpanded),
            Rung::Proximity,
        ]
    );
}

#[test]
fn the_offers_come_back_in_ladder_order() {
    let index = loaded();
    let offers = index.offers(&Query::new("שו\"ע"));
    let mut previous = 0usize;
    for offer in &offers.offers {
        let at = Rung::ALL
            .iter()
            .position(|r| *r == offer.rung)
            .expect("a rung on the ladder");
        assert!(at >= previous, "{:?} came out of order", offer.rung);
        previous = at;
    }
}

// ---------------------------------------------------------------------------
// Smart mode: applied, and announced
// ---------------------------------------------------------------------------

#[test]
fn smart_mode_finds_the_other_forms_without_being_asked_twice() {
    // spec.md §9.3: "type words, and prefixes, male/chaser and abbreviations
    // are handled for you". That is the mode's declared purpose, so it happens
    // on the first search rather than after a zero.
    let index = loaded();
    let answered = Smart::new(Query::new("מלך"))
        .run(&index)
        .expect("a smart search");
    let mut found: Vec<u32> = answered.found.hits.iter().map(line_of).collect();
    found.sort_unstable();
    assert_eq!(found, [11]);
}

#[test]
fn smart_mode_says_how_many_hits_are_only_there_because_it_widened() {
    // spec.md §9.4: "43 results — 12 match other forms of כתב". The number is
    // the difference between the widened set and the literal one, so the reader
    // can tell what the mode did for them.
    let index = loaded();
    let answered = Smart::new(Query::new("שבת"))
        .run(&index)
        .expect("a smart search");
    // Line 1 has `שבת`; line 2 has it under two prefixes.
    assert_eq!(answered.exact_total, 1);
    assert_eq!(answered.found.total, 2);
    assert_eq!(answered.from_other_forms(), 1);
    let said = answered.announcement();
    assert!(said.contains('2'), "{said}");
    assert!(said.contains("other forms"), "{said}");
}

#[test]
fn smart_mode_hands_back_the_exact_query_for_one_click_undo() {
    // "one-click undo" is a query, not a flag: the literal query is carried
    // alongside the widened result so the interface can re-run it.
    let index = loaded();
    let answered = Smart::new(Query::new("שבת"))
        .run(&index)
        .expect("a smart search");
    let undone = index.search(&answered.literal).expect("the undo");
    assert_eq!(undone.total, answered.exact_total);
    assert!(undone.widening.is_none());
}

#[test]
fn smart_mode_never_loses_a_literal_hit() {
    let index = loaded();
    for typed in ["שבת", "שו\"ע", "קידוש", "יתגבר כארי"] {
        let query = Query::new(typed);
        let literal = lines(&index, &query);
        let answered = Smart::new(query).run(&index).expect("a smart search");
        let widened: Vec<u32> = answered.found.hits.iter().map(line_of).collect();
        for line in literal {
            assert!(widened.contains(&line), "{typed:?} lost line {line}");
        }
    }
}

#[test]
fn smart_mode_climbs_to_the_proximity_rung_only_after_the_rest_found_nothing() {
    let index = loaded();

    // Nothing on the shelf has these two adjacent, and no form rung changes
    // that — so Smart goes on to the last rung and says it did.
    let answered = Smart::new(Query::new("יתגבר בבוקר").together(Together::Phrase))
        .run(&index)
        .expect("a smart search");
    assert!(answered.applied.contains(&Rung::Proximity));
    assert_eq!(answered.found.total, 2);
    assert!(
        answered.announcement().contains("same passage"),
        "{}",
        answered.announcement()
    );

    // Where the form rungs were enough, the proximity is left alone: widening
    // further than the reader needed is a change they did not ask for.
    let enough = Smart::new(Query::new("מלך"))
        .run(&index)
        .expect("a smart search");
    assert!(!enough.applied.contains(&Rung::Proximity));
}

#[test]
fn smart_mode_reports_an_honest_zero_and_what_it_tried() {
    // Widening is not a promise of results. When the whole ladder is climbed
    // and nothing is there, the mode says so and names what it tried, rather
    // than widening on past what the reader asked.
    let index = loaded();
    let answered = Smart::new(Query::new("בתרומתן"))
        .run(&index)
        .expect("a smart search");
    assert_eq!(answered.found.total, 0);
    let said = answered.announcement();
    assert!(said.contains("no results"), "{said}");
}

#[test]
fn smart_mode_reports_the_widening_it_ran_and_not_a_description_of_one() {
    // The header says what was searched for, read out of the thing that was
    // run — the same discipline as W12's plan.
    let index = loaded();
    let answered = Smart::new(Query::new("שבת"))
        .run(&index)
        .expect("a smart search");
    let widening = answered
        .found
        .widening
        .as_ref()
        .expect("smart mode always reports its widening");
    assert_eq!(widening.applied, answered.applied);
    let described = widening.describe();
    assert!(described.contains("שבת"), "{described}");
}

// ---------------------------------------------------------------------------
// Where the two columns meet
// ---------------------------------------------------------------------------

#[test]
fn a_widened_hit_highlights_the_word_that_actually_matched() {
    // The reader is looking at `וכשהמלך`. A mark on `מלך` alone would point at
    // letters in the middle of a word they did not type; the word that answered
    // the question is the whole of it.
    let index = loaded();
    let widened = Widened::new(Query::new("מלך"), [Rung::Forms(VariantKind::PrefixPeeled)]);
    let found = index.search_widened(&widened).expect("a widened search");
    let hit = found.hits.first().expect("a hit");
    let marks = found.marks(hit);
    assert_eq!(marks.len(), 1);
    let (start, end) = marks[0];
    assert_eq!(&hit.text[start..end], "וכשהמלך");
}

#[test]
fn a_widening_that_would_take_too_many_exact_searches_is_refused() {
    // Widening a proximity query multiplies: every ordering of the words times
    // every combination of their forms, each asked exactly. Past the ceiling it
    // is refused and the reason is stated — running some of the combinations
    // and calling that an answer is the failure this whole mode avoids.
    //
    // The form rungs only. Adding the proximity rung would widen the shape to
    // *anywhere in the segment*, where every position is required separately
    // and there is no cross product to blow up — which is worth knowing: the
    // ceiling is a property of asking for a distance exactly, not of widening.
    let index = loaded();
    let widened = Widened::new(
        Query::new("כהן גדול נכנס").together(Together::Near { words: 3 }),
        Smart::baseline(),
    );
    match index.search_widened(&widened) {
        Err(IndexError::TooManyForms { queries, limit }) => {
            assert!(queries > limit, "{queries} vs {limit}");
        }
        other => panic!("expected a refusal, got {:?}", other.map(|f| f.total)),
    }

    // And the same query with the proximity rung on top is answerable, because
    // the shape it widens to has no cross product in it.
    let loosened = Widened::new(
        Query::new("כהן גדול נכנס").together(Together::Near { words: 3 }),
        Rung::ALL,
    );
    assert!(index.search_widened(&loosened).is_ok());
}

#[test]
fn a_rung_whose_count_cannot_be_worked_out_is_named_rather_than_dropped() {
    // An offer that cannot be computed must not simply be absent: the reader
    // would read the missing chip as "there is nothing down that road".
    let index = loaded();
    let offers =
        index.offers(&Query::new("כהן גדול נכנס לפני").together(Together::Near { words: 3 }));
    assert!(
        !offers.refused.is_empty(),
        "expected at least one rung to be refused with a reason"
    );
    for refusal in &offers.refused {
        assert!(!refusal.why.is_empty());
        assert!(offers.offers.iter().all(|o| o.rung != refusal.rung));
    }
}

#[test]
fn the_literal_mode_is_untouched_by_all_of_this() {
    // W12's promise, re-checked from the outside after the ladder exists: for
    // every input the words asked of the index are the typed words with their
    // marks off, and nothing else.
    let index = loaded();
    for typed in ["מלך", "שו\"ע", "כהן", "ובשבת"] {
        let query = Query::new(typed);
        let _ = index.offers(&query);
        let found = index.search(&query).expect("a search");
        let expected: Vec<String> = girsa_hebrew::normalize(typed)
            .split_whitespace()
            .map(str::to_string)
            .collect();
        assert_eq!(found.asked.words, expected, "for {typed:?}");
        assert_eq!(found.asked.patterns, expected, "for {typed:?}");
        assert!(found.widening.is_none(), "for {typed:?}");
    }
}

#[test]
fn contains_and_letters_are_not_widened_in_a_direction_they_already_go() {
    // `contains` already reaches `וכשהמלך` from `מלך` — the reader asked for
    // words holding those letters, and a prefix is letters in front. So the
    // prefix rung's *prefixed* half is not added under those operators: it
    // would be a chip that changes nothing, and an inert chip is a lie about
    // what the engine can do.
    //
    // Its *peeling* half still is, because `מלך` → `לך` reaches words that
    // `contains מלך` does not. Two halves, two rulings — the rung as a whole is
    // neither dropped nor applied wholesale.
    let index = loaded();
    for matching in [Match::Contains, Match::Letters] {
        let query = Query::new("מלך").matching(matching);
        assert!(!lines(&index, &query).is_empty(), "{matching:?}");
        let widening = Widened::new(query, [Rung::Forms(VariantKind::PrefixPeeled)]).widening();
        for position in &widening.positions {
            for alternative in &position.alternatives {
                for form in &alternative.forms {
                    assert!(
                        !matches!(form.rule, Rule::Prefixed(_)),
                        "{matching:?} was widened towards prefixes it already reaches"
                    );
                }
            }
        }
    }

    // Under the literal operator it is added, because there it is the only way
    // to get from the word to the page.
    let widening =
        Widened::new(Query::new("מלך"), [Rung::Forms(VariantKind::PrefixPeeled)]).widening();
    assert!(widening.positions.iter().any(|position| position
        .alternatives
        .iter()
        .any(|a| a.forms.iter().any(|f| matches!(f.rule, Rule::Prefixed(_))))));
}
