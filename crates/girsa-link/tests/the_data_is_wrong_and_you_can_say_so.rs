//! Repairing the graph without editing it (spec.md §8.3, BUILDER.md W23).
//!
//! The link data is wrong in known ways — 40% of it carries no type at all
//! (T5), and Otzaria's copies are line-indexed — and the answer is not to
//! correct the shipped files. It is the same answer as §7.1's: **your repairs
//! are an overlay in your own layer**, and the shipped graph stays as it came,
//! so a re-import cannot take your work with it and your work cannot be
//! confused with the corpus's.
//!
//! What this file checks is that the four actions of §8.3 land, that they
//! survive the corpus being re-imported, and that a repaired edge can still
//! say **what it was** — because the UI has to show its work.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_link::repair::{Repairs, Verdict};
use girsa_link::{store, Anchor, Edge, EdgeType, Method};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-repair-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn id(work: &str, n: u32) -> SegmentId {
    SegmentId::new(work, vec!["1".into(), n.to_string()], Ordinal::root(n))
}

/// An edge the corpus shipped with no type on it, which is 40% of them.
fn untyped() -> Edge {
    Edge {
        from: Anchor::point(id("mishnah/berakhot", 1)),
        to: Anchor::point(id("rambam-on-mishnah/berakhot", 5)),
        edge_type: EdgeType::References,
        method: Method::SefariaSeed,
        source_label: String::new(),
    }
}

#[test]
fn a_blank_typed_link_can_be_typed_and_the_corpus_is_untouched() {
    let root = scratch("retype");
    let mut writer = store::Writer::default();
    writer.push(&untyped());
    writer.flush(&root).expect("writes");
    let shipped = std::fs::read(store::edges_path(&root, "mishnah/berakhot")).expect("reads");

    let (mut repairs, trouble) = Repairs::open(&root.join("personal"));
    assert!(trouble.is_empty(), "{trouble:?}");
    repairs
        .retype(&untyped(), EdgeType::CommentsOn, "me")
        .expect("takes it");

    assert_eq!(
        std::fs::read(store::edges_path(&root, "mishnah/berakhot")).expect("reads"),
        shipped,
        "a repair may not write one byte into the shipped graph"
    );

    let edges = store::read_back(&root, "mishnah/berakhot").expect("reads");
    let seen = repairs.apply(edges);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].edge.edge_type, EdgeType::CommentsOn);
    // And it shows its work: what it was, and that this was you.
    assert_eq!(
        seen[0].shipped.as_ref().map(|e| e.edge_type),
        Some(EdgeType::References)
    );
    assert_eq!(seen[0].changed, ["retyped"]);
    assert_eq!(seen[0].who.as_deref(), Some("me"));
}

#[test]
fn a_repair_survives_the_link_import_running_again() {
    let root = scratch("re-import");
    let mut writer = store::Writer::default();
    writer.push(&untyped());
    writer.flush(&root).expect("writes");

    let (mut repairs, _) = Repairs::open(&root.join("personal"));
    repairs
        .retype(&untyped(), EdgeType::CommentsOn, "me")
        .expect("takes it");

    // `girsa-link-import` replaces every shard it owns on every run.
    let mut again = store::Writer::default();
    again.push(&untyped());
    again.flush(&root).expect("writes");

    let (repairs, _) = Repairs::open(&root.join("personal"));
    let seen = repairs.apply(store::read_back(&root, "mishnah/berakhot").expect("reads"));
    assert_eq!(seen[0].edge.edge_type, EdgeType::CommentsOn);
}

#[test]
fn a_link_can_be_moved_to_the_segment_it_belongs_on() {
    // Reanchoring. Otzaria's copies are line-indexed at the source, so an edge
    // arriving through them can be a line or two out — and the reader looking
    // at both texts is the one who can see it.
    let root = scratch("reanchor");
    let (mut repairs, _) = Repairs::open(&root.join("personal"));
    let right = Anchor::point(id("rambam-on-mishnah/berakhot", 6));
    repairs
        .reanchor(&untyped(), untyped().from, right.clone(), "me")
        .expect("takes it");

    let seen = repairs.apply(vec![untyped()]);
    assert_eq!(seen[0].edge.to, right);
    assert_eq!(
        seen[0].shipped.as_ref().map(|e| e.to.clone()),
        Some(untyped().to)
    );
    assert_eq!(seen[0].changed, ["reanchored"]);
}

#[test]
fn a_link_that_is_wrong_can_be_rejected_and_is_not_shown_as_a_link() {
    let root = scratch("reject");
    let (mut repairs, _) = Repairs::open(&root.join("personal"));
    repairs
        .judge(&untyped(), Verdict::Rejected, "me")
        .expect("takes it");

    let seen = repairs.apply(vec![untyped()]);
    // It is still *there* — a rejection is yours and undoable, not a deletion.
    assert_eq!(seen.len(), 1);
    assert!(seen[0].rejected);
    assert_eq!(
        seen.iter().filter(|link| !link.rejected).count(),
        0,
        "and nothing that draws links draws it"
    );

    // Taking the rejection back leaves the shipped edge exactly as it came.
    let (mut repairs, _) = Repairs::open(&root.join("personal"));
    assert!(repairs.undo(&untyped()).expect("undoes"));
    let seen = repairs.apply(vec![untyped()]);
    assert!(!seen[0].rejected);
    assert!(seen[0].changed.is_empty());
    assert_eq!(seen[0].edge, untyped());
}

#[test]
fn confirming_a_link_is_a_claim_that_a_person_made() {
    // spec.md §8.3: a blank-typed link is never presented as curated fact. The
    // difference between "the corpus connected these somehow" and "somebody
    // looked and said yes" has to be visible, or the second is worth nothing.
    let root = scratch("confirm");
    let (mut repairs, _) = Repairs::open(&root.join("personal"));

    let seen = repairs.apply(vec![untyped()]);
    assert!(!seen[0].is_curated(), "an untyped seed is not a fact");

    repairs
        .judge(&untyped(), Verdict::Confirmed, "me")
        .expect("takes it");
    let seen = repairs.apply(vec![untyped()]);
    assert!(seen[0].confirmed);
    assert!(seen[0].is_curated(), "somebody looked at it");
    assert_eq!(seen[0].confidence(), 1.0);
}

#[test]
fn a_link_can_be_drawn_by_hand_and_is_marked_as_yours() {
    let root = scratch("draw");
    let (mut repairs, _) = Repairs::open(&root.join("personal"));
    let from = Anchor::point(id("bavli/berakhot", 3));
    let to = Anchor::point(id("shulchan-arukh/orach-chayim", 9));
    repairs
        .draw(from.clone(), to.clone(), EdgeType::Codifies, "me")
        .expect("takes it");

    // Drawn edges are not in any shard, so they come from the layer — and they
    // come back beside the shipped ones for the work they start in.
    let mine = repairs.drawn_in("bavli/berakhot");
    assert_eq!(mine.len(), 1);
    assert!(mine[0].mine);
    assert_eq!(mine[0].edge.method, Method::ByHand);
    assert_eq!(mine[0].edge.to, to);
    assert!(mine[0].is_curated());
    assert!(repairs.drawn_in("bavli/shabbat").is_empty());

    // And it survives being read back, like everything else in your layer.
    let (repairs, _) = Repairs::open(&root.join("personal"));
    assert_eq!(repairs.drawn_in("bavli/berakhot").len(), 1);
    assert_eq!(repairs.count(), 1);
}

#[test]
fn two_repairs_to_one_link_both_hold() {
    // Retyping a link and then confirming it is two statements about one edge,
    // and the second must not erase the first.
    let root = scratch("both");
    let (mut repairs, _) = Repairs::open(&root.join("personal"));
    repairs
        .retype(&untyped(), EdgeType::CommentsOn, "me")
        .expect("takes it");
    repairs
        .judge(&untyped(), Verdict::Confirmed, "me")
        .expect("takes it");

    let seen = repairs.apply(vec![untyped()]);
    assert_eq!(seen[0].edge.edge_type, EdgeType::CommentsOn);
    assert!(seen[0].confirmed);
    assert_eq!(seen[0].changed, ["retyped", "confirmed"]);
}

#[test]
fn a_repair_about_an_edge_that_is_no_longer_shipped_is_kept_and_says_so() {
    // Upstream dropped the edge, or re-segmented one of its ends. The repair is
    // not silently thrown away — it is a thing you said, and the layer holds it
    // until you take it back.
    let root = scratch("orphan");
    let (mut repairs, _) = Repairs::open(&root.join("personal"));
    repairs
        .retype(&untyped(), EdgeType::CommentsOn, "me")
        .expect("takes it");

    let seen = repairs.apply(Vec::new());
    assert!(seen.is_empty(), "it repairs nothing that is there");
    assert_eq!(repairs.count(), 1);
    assert_eq!(repairs.orphans(&[]).len(), 1);
    assert_eq!(repairs.orphans(&[untyped()]).len(), 0);
}
