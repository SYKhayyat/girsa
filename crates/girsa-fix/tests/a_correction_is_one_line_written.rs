//! Your layer is appended to, never rewritten.
//!
//! spec.md §7.5 gives correcting a typo a three-second budget, and
//! `girsa-app/tests/three_seconds.rs` measures the whole interaction against it.
//! What that test could not see is *why* the number moved: the layer was
//! serialized in full on every mutation, so the cost of your next correction was
//! a function of how many you had already made.
//!
//! Timing catches that late and flakily. What these assert instead is the
//! property underneath it, which is exact and deterministic: **the bytes that
//! were in the file are still in the file, unchanged, in the same places.** That
//! is true of an append and false of a rewrite, and it is false of a rewrite even
//! when the rewrite happens to produce a file of the same length.
//!
//! The same property is what makes the queue in `the_queue_beats_the_editor.rs`
//! usable at its real size: 28,124 candidates, one line written per decision.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use girsa_corpus::segment::SegmentId;
use girsa_fix::suspect::{hunt, Decision, Queue, Settings, Vocabulary};
use girsa_fix::{Kind, Layer, Patch};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-one-line-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The `n`th place in a sefer.
///
/// The ordinal and not the path: a segment's identity is its work and its
/// ordinal (spec.md §3), and the address before the `#` is how it is read aloud.
fn at(n: usize) -> SegmentId {
    format!("girsa:halakhah/shulchan-arukh-orach-chayim/1:1#{n}")
        .parse()
        .expect("an id")
}

/// A correction on segment `n`, claiming characters 0..4.
fn patch(n: usize) -> Patch {
    Patch::new(at(n), 0..4, "הרבר", "הדבר", Kind::Ocr, "me")
}

#[test]
fn adding_a_correction_leaves_every_byte_before_it_where_it_was() {
    let personal = scratch("append");
    let (mut layer, trouble) = Layer::open(&personal);
    assert!(trouble.is_empty());

    for n in 1..=200 {
        layer.add(patch(n)).expect("takes it");
    }
    let before = std::fs::read(layer.path()).expect("reads");

    // Ordinal 0 sorts *first* in the by-segment map, so a store that serialized
    // the whole map would put this line at the top and move all 200 others down.
    layer.add(patch(0)).expect("takes it");
    let after = std::fs::read(layer.path()).expect("reads");

    assert_eq!(
        &after[..before.len()],
        &before[..],
        "the corrections already written moved on disk — the file was rewritten, not appended to"
    );
    assert!(after.len() > before.len(), "the new one was written down");
}

#[test]
fn a_thousand_corrections_write_a_thousand_lines() {
    // Under the old store this file would have had 1 + 2 + … + 1000 = 500,500
    // lines put through it to end up holding 1,000.
    let personal = scratch("thousand");
    let (mut layer, _) = Layer::open(&personal);
    for n in 1..=1_000 {
        layer.add(patch(n)).expect("takes it");
    }
    let body = std::fs::read_to_string(layer.path()).expect("reads");
    assert_eq!(body.lines().filter(|l| !l.is_empty()).count(), 1_000);

    // And re-opening finds all thousand, which is the only thing a reader cares
    // about.
    let (again, trouble) = Layer::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(again.count(), 1_000);
}

#[test]
fn a_correction_taken_back_stays_taken_back_across_a_restart() {
    let personal = scratch("tombstone");
    let (mut layer, _) = Layer::open(&personal);
    for n in 1..=10 {
        layer.add(patch(n)).expect("takes it");
    }
    let id = layer
        .all()
        .find(|p| p.segment == at(5))
        .expect("it is here")
        .id
        .clone();
    assert!(layer.remove(&id).expect("removes"));

    let (again, trouble) = Layer::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(again.count(), 9);
    assert!(again.on(&at(5)).is_empty(), "it is still gone");
    assert_eq!(again.on(&at(6)).len(), 1, "and its neighbour is not");
}

#[test]
fn a_layer_that_churns_is_tidied_up_when_it_is_next_opened() {
    // Sixty corrections made and taken back is 120 lines holding nothing. The
    // file is not allowed to grow without bound just because writing to it is
    // cheap now — opening compacts it once it is past twice what it holds.
    let personal = scratch("compact");
    let (mut layer, _) = Layer::open(&personal);
    for n in 1..=60 {
        layer.add(patch(n)).expect("takes it");
        let id = layer.on(&at(n)).first().expect("it is here").id.clone();
        layer.remove(&id).expect("removes");
    }
    let churned = std::fs::read_to_string(layer.path()).expect("reads");
    assert_eq!(churned.lines().filter(|l| !l.is_empty()).count(), 120);

    let (opened, trouble) = Layer::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(opened.count(), 0);
    let tidied = std::fs::read_to_string(opened.path()).expect("reads");
    assert_eq!(
        tidied.lines().filter(|l| !l.is_empty()).count(),
        0,
        "opening a churned layer left the file as long as it had ever been"
    );
}

#[test]
fn deciding_on_a_candidate_leaves_every_byte_before_it_where_it_was() {
    // The queue is where this matters most: 28,124 entries on the real corpus,
    // and the product's pitch is going down that list.
    let personal = scratch("queue");
    // ד read as ר, a hundred times over. Two leading letters that are none of
    // Hebrew's attached function words, so the grammar filter has no reason to
    // throw the pair out.
    const PLAIN: [char; 10] = ['ג', 'ז', 'ח', 'ט', 'ס', 'ע', 'פ', 'צ', 'ק', 'ת'];
    let mut vocab = Vocabulary::default();
    for a in PLAIN {
        for b in PLAIN {
            vocab.add(&format!("{a}{b}הרבר"), 1);
            vocab.add(&format!("{a}{b}הדבר"), 12_000);
        }
    }
    let found = hunt(&vocab, Settings::default());
    assert!(found.len() >= 100, "{} candidates", found.len());

    let (mut queue, _) = Queue::open(&personal);
    queue.refresh(found).expect("takes them");
    let before = std::fs::read(queue.path()).expect("reads");
    let held = queue.count();

    let id = queue.ranked(1).first().expect("one to look at").id.clone();
    assert!(queue.decide(&id, Decision::Dismissed).expect("decides"));
    let after = std::fs::read(queue.path()).expect("reads");

    assert_eq!(
        &after[..before.len()],
        &before[..],
        "the queue was rewritten to record one decision"
    );

    let (again, trouble) = Queue::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(again.count(), held, "the decision replaced its entry");
    assert_eq!(again.waiting(), held - 1);
    assert_eq!(
        again.get(&id).and_then(|s| s.decided),
        Some(Decision::Dismissed)
    );
}
