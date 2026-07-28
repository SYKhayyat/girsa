//! The test that justifies the design (BUILDER.md W6).
//!
//! Import a sefer. Record where 501 links land. Fix a typo in a way that splits
//! a segment. Every one of those 501 links must still name the same words.
//!
//! The same scenario runs against both anchoring schemes. Otzaria's — file plus
//! line number — fails it, and that failure is the entire argument for
//! permanent IDs, so it is asserted here rather than described.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use girsa_corpus::segment::SegmentId;
use girsa_corpus::store::{Anchors, LineIndexStore, SegmentStore};

/// Far enough into the sefer that there is plenty below it to be damaged.
const SPLIT_AT_POSITION: usize = 50;
/// The segment being edited, plus the 500 after it.
const LINKS: usize = 501;
/// Comfortably more than we touch, so nothing is an edge case.
const SEGMENTS: usize = 600;

/// A sefer whose every segment says something different, so a link landing on
/// the wrong one is detectable rather than a coincidence.
fn segments() -> Vec<(Vec<String>, String)> {
    (1..=SEGMENTS)
        .map(|n| {
            let siman = (n / 10 + 1).to_string();
            let seif = (n % 10 + 1).to_string();
            (
                vec![siman, seif],
                format!("סימן {n} · והנה האדם הזה אף שבכמותו הוא מקטני הברואים {n}"),
            )
        })
        .collect()
}

/// What a set of links pointed at before the edit.
fn record<S: Anchors>(store: &S, from: usize, count: usize) -> Vec<(S::Anchor, String)> {
    (from..from + count)
        .filter_map(|nth| {
            let anchor = store.anchor_at_position(nth)?;
            let text = store.text_at(&anchor)?;
            Some((anchor, text))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Permanent IDs
// ---------------------------------------------------------------------------

#[test]
fn splitting_a_segment_leaves_all_501_links_naming_the_same_words() {
    let mut store = SegmentStore::import("mishnah-berurah", segments());
    let before = record(&store, SPLIT_AT_POSITION, LINKS);
    assert_eq!(before.len(), LINKS);

    // The correction: one segment turns out to be two.
    let target = store
        .anchor_at_position(SPLIT_AT_POSITION)
        .expect("the segment being split exists");
    let text_len = store.text_at(&target).map(|t| t.len()).unwrap_or_default();
    store.split(&target, text_len / 2);

    let mut moved = Vec::new();
    for (anchor, was) in &before {
        match store.text_at(anchor) {
            Some(now) if now == *was => {}
            Some(now) => moved.push(format!("{anchor}\n  was: {was}\n  now: {now}")),
            None => moved.push(format!("{anchor}\n  was: {was}\n  now: <gone>")),
        }
    }

    assert!(
        moved.is_empty(),
        "{} of {LINKS} links moved:\n{}",
        moved.len(),
        moved.iter().take(3).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn merging_two_segments_leaves_all_501_links_still_naming_their_words() {
    // A merge cannot leave an anchor naming *exactly* what it did — the two
    // segments are one now. What must hold is that it still names text
    // containing what it used to say, and never someone else's words.
    let mut store = SegmentStore::import("mishnah-berurah", segments());
    let before = record(&store, SPLIT_AT_POSITION, LINKS);

    let target = store
        .anchor_at_position(SPLIT_AT_POSITION)
        .expect("the segment being merged exists");
    store.merge_with_next(&target);

    let mut lost = Vec::new();
    for (anchor, was) in &before {
        match store.text_at(anchor) {
            Some(now) if now.contains(was.as_str()) => {}
            Some(now) => lost.push(format!("{anchor}\n  was: {was}\n  now: {now}")),
            None => lost.push(format!("{anchor} resolves to nothing")),
        }
    }

    assert!(
        lost.is_empty(),
        "{} of {LINKS} links lost their text:\n{}",
        lost.len(),
        lost.iter().take(3).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn an_upstream_resegmentation_is_absorbed_by_the_redirect_table() {
    // Sefaria re-releases a work with different boundaries. Every ref in every
    // Ksav document written against the old release has to keep resolving —
    // that is the promise spec.md §4.2 says the two-app system rests on.
    let mut store = SegmentStore::import("mishnah-berurah", segments());
    let before = record(&store, SPLIT_AT_POSITION, LINKS);

    // Upstream split one segment into three and renumbered its own way. We do
    // not control any of it; all we get to do is record where the text went.
    let old = store
        .anchor_at_position(SPLIT_AT_POSITION)
        .expect("the re-segmented segment exists");
    let old_text = store.text_at(&old).unwrap_or_default();
    let thirds = old_text.len() / 3;
    let first = store.split(&old, boundary(&old_text, thirds));
    let second_half = first[1].clone();
    store.split(&second_half, boundary(&old_text, thirds));

    let mut broken = Vec::new();
    for (anchor, was) in &before {
        match store.text_at(anchor) {
            Some(now) if now == *was => {}
            Some(now) => broken.push(format!("{anchor}\n  was: {was}\n  now: {now}")),
            None => broken.push(format!("{anchor} resolves to nothing")),
        }
    }

    assert!(
        broken.is_empty(),
        "{} of {LINKS} refs broke across a re-segmentation:\n{}",
        broken.len(),
        broken
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn a_redirect_recorded_by_hand_is_followed() {
    let mut store = SegmentStore::import("mishnah-berurah", segments());
    let a = store.anchor_at_position(0).expect("first segment");
    let b = store.anchor_at_position(1).expect("second segment");
    let b_text = store.text_at(&b).expect("second segment has text");

    store.redirect(a.clone(), vec![b.clone()]);
    assert_eq!(store.text_at(&a), Some(b_text));
}

#[test]
fn a_redirect_cycle_stops_rather_than_hanging() {
    // Nothing should build one. If a hand-edited overlay does, the reader gets
    // an empty result, not a frozen window.
    let mut store = SegmentStore::import("mishnah-berurah", segments());
    let a = store.anchor_at_position(0).expect("first segment");
    let b = store.anchor_at_position(1).expect("second segment");
    store.redirect(a.clone(), vec![b.clone()]);
    store.redirect(b.clone(), vec![a.clone()]);
    assert_eq!(store.text_at(&a), None);
}

#[test]
fn no_line_number_is_persisted_as_a_durable_reference() {
    // BUILDER.md W6 acceptance. A SegmentId carries a work, a section path and
    // an ordinal, and the ordinal is not a position — it is a name that was
    // once derived from one and is never derived again. After a split, the
    // ordinals present are no longer 1..n, which is the observable difference.
    let mut store = SegmentStore::import("mishnah-berurah", segments());
    let target = store.anchor_at_position(SPLIT_AT_POSITION).expect("exists");
    store.split(&target, 10);

    let ordinals: Vec<String> = store
        .iter()
        .map(|(id, _)| id.ordinal().to_string())
        .collect();
    assert!(
        ordinals.iter().any(|o| o.contains('.')),
        "a split must produce an ordinal a line number could not express"
    );
    assert!(
        ordinals.iter().position(|o| o == "52").unwrap_or(0) > SPLIT_AT_POSITION,
        "segment #52 must still be called #52 after the edit above it"
    );
}

// ---------------------------------------------------------------------------
// Otzaria's scheme, kept as the counter-example
// ---------------------------------------------------------------------------

#[test]
fn line_numbers_silently_repoint_500_links_and_this_is_why_they_are_not_used() {
    // Not a bug in LineIndexStore. Every method is correct as written; the
    // defect is that a line number names a *position*, and positions move.
    //
    // This is asserted rather than described so that nobody can quietly
    // "simplify" the design back to it and have the suite stay green.
    let mut store = LineIndexStore::new(segments().into_iter().map(|(_, t)| t).collect());
    let before = record(&store, SPLIT_AT_POSITION, LINKS);

    store.split_at_position(SPLIT_AT_POSITION, 20);

    let moved = before
        .iter()
        .filter(|(anchor, was)| store.text_at(anchor).as_ref() != Some(was))
        .count();

    assert!(
        moved >= 500,
        "the counter-example stopped being broken ({moved} moved) — \
         if line indices became safe, the whole of spec.md §3 needs rewriting"
    );

    // And this is the part that makes it dangerous rather than merely wrong:
    // the links did not break. They resolve, cleanly, to somebody else's words.
    let (anchor, was) = &before[300];
    let now = store.text_at(anchor);
    assert!(
        now.is_some(),
        "a moved link still resolves — no error is raised"
    );
    assert_ne!(now.as_ref(), Some(was), "…to different text");
}

#[test]
fn the_same_edit_costs_permanent_ids_nothing() {
    // Side by side, on the same sefer and the same edit.
    let mut naive = LineIndexStore::new(segments().into_iter().map(|(_, t)| t).collect());
    let mut real = SegmentStore::import("mishnah-berurah", segments());

    let naive_before = record(&naive, SPLIT_AT_POSITION, LINKS);
    let real_before = record(&real, SPLIT_AT_POSITION, LINKS);

    naive.split_at_position(SPLIT_AT_POSITION, 20);
    let target = real.anchor_at_position(SPLIT_AT_POSITION).expect("exists");
    real.split(&target, 20);

    let naive_moved = naive_before
        .iter()
        .filter(|(a, was)| naive.text_at(a).as_ref() != Some(was))
        .count();
    let real_moved = real_before
        .iter()
        .filter(|(a, was)| real.text_at(a).as_ref() != Some(was))
        .count();

    assert_eq!(real_moved, 0);
    assert!(naive_moved > 0);
    println!("one typo fix, {LINKS} links: line numbers moved {naive_moved}, permanent ids moved {real_moved}");
}

// ---------------------------------------------------------------------------

/// Nudge a byte offset onto a character boundary. Hebrew is two bytes a letter,
/// so a third of the way through a string is usually not a boundary.
fn boundary(s: &str, at: usize) -> usize {
    let mut at = at.min(s.len());
    while at > 0 && !s.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// The type is used through the trait everywhere above; this keeps the import
/// honest rather than silently unused.
#[allow(dead_code)]
fn _assert_id_type(id: SegmentId) -> String {
    id.to_string()
}
