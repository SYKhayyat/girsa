//! Your writing, in the index, without four minutes — spec.md §11, W11.
//!
//! *"Local, exportable as plain files, no account"* and *"a note has the same
//! typed edges as anything else"* — and a note was searchable **as of the last
//! build**. So the honest sentence in the results header was *1 note since the
//! index was built*, and the only way to make it stop saying that was to
//! re-read 5,000,545 segments. Four minutes for one paragraph.
//!
//! Nothing about tantivy required it. A work has been the unit of replacement
//! since W11 — the first segment of a work deletes every segment of that work
//! already in the index, because `girsa-import` rewrites `segments.jsonl`
//! wholesale and an append would double every hit. Read from the other side,
//! that rule *is* an incremental update. What was missing was a caller, and
//! before a caller could exist, the body of the build loop had to become a
//! function.
//!
//! What is asserted here is that the shortcut is not a second indexer:
//!
//! 1. a work taken in is findable, and nothing else moved;
//! 2. taking the same work in twice leaves one copy, not two — the one failure
//!    mode an append-only index has, and the one W8 shipped once already;
//! 3. an edit replaces rather than accumulates, so the old words stop being
//!    findable;
//! 4. a work thrown away is taken **out**, which `absorb` cannot do for it: a
//!    deleted work has no segments to read back, so the delete-then-add rule
//!    never fires and a hit would open a sefer that is not there.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_search::building::{absorb, forget};
use girsa_search::corrected::Corrections;
use girsa_search::index::SearchIndex;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch root");
    dir
}

/// Write a note, through the door the window writes one through.
///
/// `Notes::write` is what gives a note its `work.json`, its `segments.jsonl`
/// and its catalogue line — a note is a sefer (spec.md §11) — so what is
/// absorbed below is a work `read_back` really returns rather than a fixture
/// shaped like one. Called twice with the same name, it is an edit.
fn note(root: &Path, name: &str, words: &str) -> String {
    let (mut notes, trouble) = girsa_note::Notes::open(root);
    assert!(trouble.is_empty(), "a fresh layer reads");
    let mut held = match notes.get(name).cloned() {
        Some(mut held) => {
            let first = held.paras().first().expect("a paragraph").id.clone();
            assert!(held.set(&first, words.to_string()));
            held
        }
        None => {
            let mut fresh = notes.start(name, "a reader");
            fresh.append(words.to_string());
            fresh
        }
    };
    held.title = name.to_string();
    let written = notes.write(held).expect("the note is written");
    written.slug.clone()
}

fn finds(index: &SearchIndex, word: &str) -> usize {
    index.words(word).expect("a search").len()
}

/// An index with one sefer of the corpus already in it, so *nothing else moved*
/// is a claim with something to move.
fn started(root: &Path) -> SearchIndex {
    use girsa_corpus::import::{Segment, SegmentKind};
    use girsa_corpus::segment::{Ordinal, SegmentId};

    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    writer
        .add(
            &Segment {
                id: SegmentId::new("bavli/berakhot", vec!["2a".into()], Ordinal::root(1)),
                kind: SegmentKind::Text,
                text: "מאימתי קורין את שמע בערבין".to_string(),
                anchors: Vec::new(),
            },
            &[],
        )
        .expect("a line of the corpus");
    writer.commit().expect("committing");
    index.reload().expect("reloading");
    let _ = root;
    index
}

#[test]
fn a_note_written_now_is_findable_now() {
    let root = scratch("girsa-absorb-note");
    let index = started(&root);
    let before = index.count();

    let slug = note(&root, "חבורה", "דבר שכתבתי היום");
    let done = absorb(&index, &root, &slug, &Corrections::none()).expect("it is absorbed");

    // Two, not one: a note is its title as a heading and then its paragraphs,
    // which is what makes it a sefer with a shape rather than a blob of text.
    assert_eq!(done.segments, 2);
    assert_eq!(finds(&index, "שכתבתי"), 1, "the words of the note");
    assert_eq!(index.count(), before + 2);
    assert_eq!(
        finds(&index, "מאימתי"),
        1,
        "and the corpus is exactly where it was"
    );
}

#[test]
fn taking_the_same_note_twice_leaves_one_copy() {
    // The failure an append-only index has, and this repository has shipped it
    // once already: W8's importer opened its shards in append mode and doubled
    // the graph on a second run. Here it would mean every hit twice and a
    // segment count that drifted upward on every keystroke.
    let root = scratch("girsa-absorb-twice");
    let index = started(&root);

    let slug = note(&root, "חבורה", "דבר שכתבתי היום");
    absorb(&index, &root, &slug, &Corrections::none()).expect("once");
    let after_one = index.count();
    absorb(&index, &root, &slug, &Corrections::none()).expect("twice");

    assert_eq!(index.count(), after_one, "no second copy");
    assert_eq!(finds(&index, "שכתבתי"), 1, "and one hit, not two");
}

#[test]
fn an_edit_replaces_the_note_rather_than_adding_to_it() {
    // A note is rewritten whole on every edit, so *changed* and *new* are the
    // same operation — and the old words have to stop being findable, or a
    // search would answer with a paragraph the reader deleted.
    let root = scratch("girsa-absorb-edit");
    let index = started(&root);

    let slug = note(&root, "חבורה", "הנוסח הראשון");
    absorb(&index, &root, &slug, &Corrections::none()).expect("written");
    assert_eq!(finds(&index, "הראשון"), 1);

    let slug = note(&root, "חבורה", "הנוסח המתוקן");
    absorb(&index, &root, &slug, &Corrections::none()).expect("edited");

    assert_eq!(finds(&index, "המתוקן"), 1, "what it says now");
    assert_eq!(finds(&index, "הראשון"), 0, "and not what it used to say");
}

#[test]
fn a_note_you_threw_away_stops_being_findable() {
    // The asymmetry `absorb` cannot cover: a deleted work has no
    // `segments.jsonl` to read back, so nothing is ever added under its name
    // and the delete half of the rule never fires. Left alone it stays findable
    // until the next full build, and a hit on it opens a sefer that is not on
    // the shelf — which is worse than the gap this whole module closes.
    let root = scratch("girsa-absorb-forget");
    let index = started(&root);

    let slug = note(&root, "חבורה", "דבר שכתבתי היום");
    absorb(&index, &root, &slug, &Corrections::none()).expect("written");
    assert_eq!(finds(&index, "שכתבתי"), 1);

    forget(&index, &slug).expect("it is taken out");
    assert_eq!(finds(&index, "שכתבתי"), 0);
    assert_eq!(
        finds(&index, "מאימתי"),
        1,
        "and the sefer beside it is untouched"
    );
}
