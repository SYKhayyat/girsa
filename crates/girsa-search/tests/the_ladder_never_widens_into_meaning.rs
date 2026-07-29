//! The relaxation ladder is lexical, all the way down (spec.md §9.6, W30).
//!
//! BUILDER.md W30 lists the ladder as a sibling of the semantic lane and states
//! the rule in one clause: **the lane is *not* a rung on it; the ladder must not
//! silently widen into embeddings.**
//!
//! That is not a small point. Every rung on the ladder is *priced before the
//! click*: the chip says `[try other forms — 7]` and the 7 is computed from the
//! very query clicking would run. An embedding lane cannot be priced that way,
//! because its answer is a ranked neighbourhood rather than a set of matches —
//! there is no count that means the same thing. A lane offered as a rung would
//! therefore be a chip with either no number on it or a made-up one, in the one
//! place in this application where a number is a promise.
//!
//! And it would be worse than a bad chip. The ladder's rungs all widen *what
//! spelling counts as this word*; the lane changes *what the question is*. A
//! reader who clicked `[try other forms]` and got adjacent-by-meaning results
//! would have had their query changed without being told, which is the one thing
//! spec.md §9 exists to prevent.
//!
//! # How this is held
//!
//! By the type system, and this test is the thing that notices if that stops
//! being true. `girsa-search` **does not depend on `girsa-lane`** — check
//! `crates/girsa-search/Cargo.toml` — so no rung can reach a vector even by
//! accident. What this file adds is an exhaustive match over [`Rung`]: adding a
//! variant makes it fail to compile, so a future work order that wants a lane
//! rung has to come here and say so out loud.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_search::ladder::{Rung, Standing};
use girsa_search::torat_emet::Query;

/// What kind of widening a rung is. Only one kind exists, and that is the point.
#[derive(Debug, PartialEq, Eq)]
enum Kind {
    /// A different way of spelling the same words, or a wider gap between them.
    /// Askable of the inverted index, and countable before the click.
    Lexical,
}

/// The exhaustive match. A new `Rung` variant does not compile until somebody
/// classifies it here, on purpose.
fn kind_of(rung: Rung) -> Kind {
    match rung {
        Rung::Nikud | Rung::Forms(_) | Rung::Root | Rung::Proximity => Kind::Lexical,
    }
}

#[test]
fn every_rung_of_the_ladder_is_a_lexical_widening_and_nothing_else() {
    assert_eq!(Rung::ALL.len(), 7, "spec.md §9.6 sets out seven");
    for rung in Rung::ALL {
        assert_eq!(
            kind_of(rung),
            Kind::Lexical,
            "{rung:?} is on the ladder and is not a lexical widening"
        );
    }
}

#[test]
fn no_rung_is_named_after_meaning() {
    // A weaker check than the match above and a different one: it catches a
    // rung that was classified `Lexical` here while being labelled to a reader
    // as something else. The chip is what the reader acts on.
    for rung in Rung::ALL {
        let label = rung.label().to_lowercase();
        for forbidden in [
            "meaning",
            "semantic",
            "similar",
            "adjacent",
            "embedding",
            "vector",
            "like this",
        ] {
            assert!(
                !label.contains(forbidden),
                "the rung labelled {label:?} promises {forbidden:?}, which the ladder cannot do"
            );
        }
    }
}

#[test]
fn every_rung_that_is_offered_can_be_priced_before_the_click() {
    // Why the lane cannot be a rung, stated as an assertion about the rungs
    // that are. Each one is either climbed already, deferred with a reason, or
    // ready — and *ready* means there is a query whose count can be taken.
    // There is no fourth standing, and an embedding neighbourhood would need
    // one.
    let query = Query::new("שבת");
    for rung in Rung::ALL {
        match rung.standing() {
            Standing::Climbed | Standing::Ready => {}
            Standing::Deferred(why) => assert!(
                !why.is_empty(),
                "{rung:?} is deferred and does not say why — a missing chip reads as \
                 *there is nothing down that road*"
            ),
        }
    }
    // And the thing being widened is the typed query, not a reinterpretation of
    // it: the typed word survives as an alternative on every rung.
    let widened = girsa_search::ladder::Widened::new(query, Rung::ALL);
    for position in widened.widening().positions {
        assert_eq!(
            position.alternatives[0].shown, position.typed,
            "a rung that dropped the typed word would be a swap, not a widening"
        );
    }
}
