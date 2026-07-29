//! Being handed four thousand ranked candidates, rather than fixing what you
//! trip over.
//!
//! spec.md §7.3, BUILDER.md W21: *a word appearing exactly once in the corpus,
//! one edit-distance from a word appearing ten thousand times, is almost
//! certainly an OCR error.* The whole of that sentence is load-bearing, and the
//! part that decides whether the queue is usable is **one edit-distance from**:
//! Hebrew is a language where one letter at the front is a word meaning *and*,
//! so the naive version of this hands you a queue that is mostly grammar.
//!
//! What this file checks is therefore mostly what the queue **refuses**.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use girsa_corpus::segment::SegmentId;
use girsa_fix::suspect::{hunt, Decision, Queue, Settings, Vocabulary};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-suspects-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A corpus's worth of words, as counts.
fn vocabulary(words: &[(&str, u64)]) -> Vocabulary {
    let mut vocab = Vocabulary::default();
    for (word, count) in words {
        vocab.add(word, *count);
    }
    vocab
}

#[test]
fn a_word_seen_once_beside_a_word_seen_ten_thousand_times_is_a_suspect() {
    // ד read as ר. The pair spec.md §7.2 names first, and the one that is
    // everywhere in scanned Hebrew print.
    let vocab = vocabulary(&[("הרבר", 1), ("הדבר", 12_000), ("ואמר", 40_000)]);
    let found = hunt(&vocab, Settings::default());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].rare, "הרבר");
    assert_eq!(found[0].common, "הדבר");
    assert_eq!(found[0].common_count, 12_000);
    assert_eq!(found[0].confusion.as_deref(), Some("ד/ר"));
}

#[test]
fn a_word_with_a_prefix_on_it_is_grammar_and_not_a_scanner() {
    // `ובשבת` is `בשבת` with a vav. On edit distance it is one insertion at the
    // front and it looks exactly like a dropped letter; it is how the language
    // works, and a queue that offers it is a queue nobody finishes reading.
    let vocab = vocabulary(&[
        ("ובשבת", 1),
        ("בשבת", 30_000),
        ("שהמלך", 1),
        ("המלך", 25_000),
    ]);
    assert!(hunt(&vocab, Settings::default()).is_empty());
}

#[test]
fn a_word_with_a_pronoun_on_the_end_is_grammar_too() {
    // `דברו` is `דבר` with a suffix, and `דברי` likewise. Same shape as a
    // scanner adding a letter, same answer.
    let vocab = vocabulary(&[("דברו", 1), ("דברי", 1), ("דבר", 80_000)]);
    assert!(hunt(&vocab, Settings::default()).is_empty());
}

#[test]
fn a_short_word_is_not_offered_however_rare_it_is() {
    // Two- and three-letter Hebrew words are one edit from dozens of others.
    // Every one of those pairings is a coincidence and the queue would be made
    // of them.
    let vocab = vocabulary(&[("רבר", 1), ("דבר", 90_000), ("אם", 1), ("עם", 60_000)]);
    assert!(hunt(&vocab, Settings::default()).is_empty());
}

#[test]
fn a_rare_word_with_no_common_neighbour_is_left_alone() {
    let vocab = vocabulary(&[("אנפילאות", 1), ("הדבר", 12_000)]);
    assert!(hunt(&vocab, Settings::default()).is_empty());
}

#[test]
fn a_known_confusion_outranks_a_coincidence_of_the_same_size() {
    // Both are one edit from a common word. One of them is a pair of letters
    // that look alike in print, and that is the whole of what makes a ranked
    // queue worth reading from the top.
    let vocab = vocabulary(&[
        ("הרבר", 1), // ד/ר — shapes
        ("הדבל", 1), // ר/ל — not a pair anybody's scanner confuses
        ("הדבר", 12_000),
    ]);
    let found = hunt(&vocab, Settings::default());
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0].rare, "הרבר", "the confusion pair comes first");
    assert!(found[0].score > found[1].score);
}

#[test]
fn a_letter_added_beside_the_commonest_word_in_the_corpus_does_not_open_the_queue() {
    // What the real corpus said the first time this ran. `הוא` is in 1,305,264
    // segments and every four-letter misspelling of it is one edit away, so a
    // queue ranked by how common the neighbour is opens with ten of them and a
    // reader never reaches the ד/ר findings at all.
    //
    // The claims are not equally strong: one letter read as another is what
    // scanners do, and a letter appearing beside a three-letter word is the
    // weakest evidence there is.
    let vocab = vocabulary(&[
        ("הועא", 1),
        ("הוא", 1_305_264),
        ("הרבר", 1),
        ("הדבר", 12_000),
    ]);
    let found = hunt(&vocab, Settings::default());
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        found[0].rare, "הרבר",
        "the ד/ר in a four-letter word comes first, not the letter beside הוא"
    );
    assert_eq!(found[1].rare, "הועא");
    // And it is still offered, further down. It is a real finding.
    assert_eq!(found[1].common, "הוא");
}

#[test]
fn the_queue_never_corrects_anything_by_itself() {
    // spec.md §7.3 asks for a *reviewable* queue and BUILDER.md rule 6 says
    // ambiguity is a choice, never a guess. A suspect is a question: it carries
    // what it thinks and it makes no patch.
    let dir = scratch("reviewable");
    let vocab = vocabulary(&[("הרבר", 1), ("הדבר", 12_000)]);
    let (mut queue, trouble) = Queue::open(&dir);
    assert!(trouble.is_empty(), "{trouble:?}");
    queue
        .refresh(hunt(&vocab, Settings::default()))
        .expect("writes");

    let (layer, _) = girsa_fix::Layer::open(&dir);
    assert_eq!(layer.count(), 0, "the queue made no corrections");
    assert_eq!(queue.ranked(10).len(), 1);
    assert!(queue.ranked(10)[0].decided.is_none());
}

#[test]
fn a_decision_survives_the_batch_job_running_again() {
    // The queue is rebuilt from the whole corpus whenever the corpus changes.
    // If that forgets what you have already looked at, the second run hands you
    // the four thousand you have already dismissed.
    let dir = scratch("decisions");
    let vocab = vocabulary(&[("הרבר", 1), ("הדבר", 12_000), ("ואמד", 1), ("ואמר", 40_000)]);
    let found = hunt(&vocab, Settings::default());
    assert_eq!(found.len(), 2);

    let (mut queue, _) = Queue::open(&dir);
    queue.refresh(found.clone()).expect("writes");
    let first = queue.ranked(10)[0].id.clone();
    queue.decide(&first, Decision::Dismissed).expect("decides");

    // A second machine reads the file, and the batch job runs again.
    let (mut again, _) = Queue::open(&dir);
    let refreshed = again.refresh(found).expect("writes");
    assert_eq!(refreshed.found, 2);
    assert_eq!(refreshed.fresh, 0);
    assert_eq!(refreshed.decided_before, 1);
    assert_eq!(
        again.ranked(10).len(),
        1,
        "what is left to review is what has not been reviewed"
    );
    assert_eq!(again.count(), 2, "and the decision is still on the file");
}

#[test]
fn a_suspect_says_where_to_look() {
    // A queue of words nobody can find is a list, not a queue.
    let dir = scratch("places");
    let vocab = vocabulary(&[("הרבר", 1), ("הדבר", 12_000)]);
    let mut found = hunt(&vocab, Settings::default());
    let at: SegmentId = "girsa:mishnah-berurah/1:1#7".parse().expect("an id");
    found[0].places.push(at.clone());

    let (mut queue, _) = Queue::open(&dir);
    queue.refresh(found).expect("writes");
    let (queue, _) = Queue::open(&dir);
    assert_eq!(queue.ranked(10)[0].places, vec![at]);
}
