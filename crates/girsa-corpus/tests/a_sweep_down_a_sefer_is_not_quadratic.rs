//! Reading a sefer is a walk down it, and the walk was quadratic.
//!
//! `SegmentStore` is the store the reading pane draws from, and every row it
//! draws asks for *the nth segment*. That question went through
//! `self.live.keys().nth(n)`, which walks the map from its first key every
//! time: over Mishnah Berurah's 18,120 segments a single sweep came to about
//! 164 million key comparisons, and the cost of drawing one page of results
//! grew with the square of the sefer (issue #60).
//!
//! Three claims, and the third is the one a cache of this kind gets wrong:
//!
//! 1. a sweep of the whole sefer costs what one walk down a list costs, so
//!    doubling the sefer doubles the sweep and not its square;
//! 2. the order that sweep reads is `iter()`'s order, after the store has been
//!    edited — a split, a merge and a redirect — because a positional list kept
//!    beside a map is a list that can fall out of step with it, and a reading
//!    pane drawing the wrong segment *in the right place* would be worse than a
//!    slow one;
//! 3. a redirect graph with a diamond in it is walked once per id, not once per
//!    path.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::{Duration, Instant};

use girsa_corpus::segment::SegmentId;
use girsa_corpus::store::{Anchors, SegmentStore};

const WORK: &str = "bavli/berakhot";

/// A sefer of `n` segments, in reading order.
fn sefer(n: usize) -> SegmentStore {
    let segments = (1..=n)
        .map(|i| (vec![i.to_string()], format!("סימן {i} · והנה האדם הזה")))
        .collect();
    SegmentStore::import(WORK, segments)
}

/// One pass down the sefer, the way the reading pane draws it: ask for the nth
/// segment, one after another, from the first to the last.
///
/// Only the asking, and that is the measurement rather than a shortcut: reading
/// each anchor out ([`Anchors::text_at`]) is linear work of its own, and mixed
/// in here it would be a flat cost large enough to hide the quadratic one. What
/// each answer is *worth* is
/// [`nothing_has_fallen_out_of_step`]'s question, not this one's.
fn walk(store: &SegmentStore) -> usize {
    let mut read = 0_usize;
    let mut nth = 0;
    while store.anchor_at_position(nth).is_some() {
        read += 1;
        nth += 1;
    }
    read
}

/// The fastest of three walks. One walk of a whole sefer is a machine doing
/// something else away from the number being measured.
fn best_of(store: &SegmentStore) -> (Duration, usize) {
    let mut best: Option<(Duration, usize)> = None;
    for _ in 0..3 {
        let began = Instant::now();
        let read = walk(store);
        let took = began.elapsed();
        if best.as_ref().is_none_or(|(had, _)| took < *had) {
            best = Some((took, read));
        }
    }
    best.expect("three walks happened")
}

#[test]
fn a_sweep_of_the_whole_sefer_doubles_when_the_sefer_does() {
    // Quadratic would be 4×; the allowance is 3×, because a doubling is the
    // claim and the measurement runs on a machine that is not only doing this.
    const ALLOWED_DOUBLING: f64 = 3.0;

    let (small, read) = best_of(&sefer(4_000));
    assert_eq!(read, 4_000, "the whole sefer was walked");
    let (large, read) = best_of(&sefer(8_000));
    assert_eq!(read, 8_000, "the whole sefer was walked");

    let ratio = large.as_secs_f64() / small.as_secs_f64().max(f64::MIN_POSITIVE);
    assert!(
        ratio < ALLOWED_DOUBLING,
        "twice the sefer cost {ratio:.1}× the sweep ({small:?} → {large:?}); \
         quadratic is 4× and the positional walk is unindexed again"
    );
}

#[test]
fn nothing_has_fallen_out_of_step() {
    // A mixed run of the four things that change the store, then the only
    // question that matters: does the positional list still hold the map's
    // keys in the map's order?
    let mut store = sefer(6);
    let ids: Vec<SegmentId> = store.iter().map(|(id, _)| id.clone()).collect();

    store.split(&ids[1], 4);
    assert_eq!(
        walk(&store),
        7,
        "a split is two segments where there was one"
    );

    let after_split: Vec<SegmentId> = store.iter().map(|(id, _)| id.clone()).collect();
    store.merge_with_next(&after_split[0]);
    assert_eq!(
        walk(&store),
        6,
        "a merge is one segment where there were two"
    );

    // An upstream re-segmentation: an id that is no longer live at all.
    let gone = ids[4].clone();
    store.redirect(gone.clone(), store.resolve(&ids[5]));
    assert_eq!(
        store.resolve(&gone),
        store.resolve(&ids[5]),
        "the redirect points at what the sefer still has"
    );
    assert_eq!(walk(&store), 5, "a redirect takes a name, not its words");

    let by_position: Vec<SegmentId> = (0..)
        .map_while(|nth| store.anchor_at_position(nth))
        .collect();
    let by_iteration: Vec<SegmentId> = store.iter().map(|(id, _)| id.clone()).collect();
    assert_eq!(
        by_position, by_iteration,
        "the positional list and the map disagree about the order of the sefer"
    );
}

#[test]
fn a_diamond_is_walked_once() {
    // A redirect graph is a DAG and the importer does not promise it is a tree:
    // two rows can point at the same dead id, whose subtree is then walked
    // again. Twenty levels of that without a visited set is 2^20 walks of the
    // tail, so the guard is a time nobody can call a measurement of the machine
    // — the fixed version takes microseconds.
    const LEVELS: usize = 20;
    let mut store = SegmentStore::import(WORK, vec![(vec!["1".into()], "אחד".into())]);
    let live = store.iter().next().map(|(id, _)| id.clone()).unwrap();
    let mut previous = live.clone();
    for level in 0..LEVELS {
        // A distinct ordinal each: an id is its work and its ordinal, and the
        // path is metadata, so twenty ids sharing ordinal 1 are one id.
        #[allow(clippy::cast_possible_truncation)]
        let ordinal = girsa_corpus::segment::Ordinal::root(100 + level as u32);
        let dead = SegmentId::new(WORK, vec![format!("dead-{level}")], ordinal);
        store.redirect(dead.clone(), vec![previous.clone(), previous.clone()]);
        previous = dead;
    }

    let began = Instant::now();
    let resolved = store.resolve(&previous);
    let took = began.elapsed();
    assert_eq!(resolved, vec![live], "the one live segment, once");
    assert!(
        took < Duration::from_millis(100),
        "{LEVELS} levels of diamond took {took:?} to resolve, which is the tail walked more than once"
    );
}
