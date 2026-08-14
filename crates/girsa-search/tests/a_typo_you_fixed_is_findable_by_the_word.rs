//! Your corrections, in the search index — spec.md §7, BUILDER.md W20 and W11.
//!
//! Everything a reader looks at went through the corrections overlay except the
//! one thing that finds anything. The pane drew the fix, a quote copied to Ksav
//! carried it, an export wrote it and said in its header that it had — and a
//! search answered out of the corpus files, which are the sefer as it was
//! scanned. So a typo fixed this morning was findable **by the typo and not by
//! the word**, which is the single surface where a correction looked like it
//! had never been made.
//!
//! The four claims here are the ones that would go quietly wrong:
//!
//! 1. the corrected word finds the line, and the typo no longer does;
//! 2. a **girsa variant** is not applied — it is a claim about what the text
//!    should say, and an index is about what it does say;
//! 3. a correction whose words the corpus no longer has changes nothing and is
//!    **counted**, so a build can say so rather than dropping it;
//! 4. a sefer nobody has corrected is not asked about twice — the cheap
//!    question that keeps a five-million-segment build the length it was.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_corpus::standing::Standing;
use girsa_fix::{Kind, Layer, Patch};
use girsa_search::corrected::Corrections;
use girsa_search::index::SearchIndex;

/// What the scan says: a resh where the sefer has a dalet.
const AS_SCANNED: &str = "כל הרבר אשר אנכי מצוה אתכם";
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch personal root");
    dir
}

fn at() -> SegmentId {
    SegmentId::new(
        "devarim/rambam",
        vec!["1".into(), "1".into()],
        Ordinal::root(1),
    )
}

fn segment(id: &SegmentId, text: &str) -> Segment {
    Segment {
        id: id.clone(),
        kind: SegmentKind::Text,
        text: text.to_string(),
        anchors: Vec::new(),
    }
}

/// A place with no history: nothing has been cut or redirected here, which is
/// every work on the shelf today and is not what makes this interesting.
fn standing(id: &SegmentId) -> Standing {
    Standing::just(id.clone())
}

/// The overlay this build would read, with one correction in it.
fn layer(dir: &Path, was: &str, now: &str, kind: Kind) -> Corrections {
    let (mut layer, trouble) = Layer::open(dir);
    assert!(
        trouble.is_empty(),
        "a fresh layer has nothing to complain of"
    );
    let from = AS_SCANNED.chars().position(|c| c == 'ה').expect("a word");
    layer
        .add(Patch::new(
            at(),
            from + 3..from + 7,
            was,
            now,
            kind,
            "a reader",
        ))
        .expect("the correction is written down");
    let (corrections, trouble) = Corrections::of(&[dir]);
    assert!(trouble.is_empty());
    corrections
}

/// One segment, indexed the way `girsa-index build` indexes it.
fn indexed(corrections: &Corrections, text: &str) -> SearchIndex {
    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    let id = at();
    let segment = segment(&id, text);
    match corrections
        .touch(id.work())
        .then(|| corrections.text(&standing(&id), text))
        .flatten()
    {
        Some(reading) => writer.add_saying(&segment, &[], &reading.text),
        None => writer.add(&segment, &[]),
    }
    .expect("the segment is indexed");
    writer.commit().expect("committing");
    index.reload().expect("reloading");
    index
}

fn finds(index: &SearchIndex, word: &str) -> usize {
    index.words(word).expect("a search").len()
}

#[test]
fn the_word_you_corrected_to_finds_the_line_and_the_typo_does_not() {
    let dir = scratch("girsa-index-corrections");
    let corrections = layer(&dir, "הרבר", "הדבר", Kind::Ocr);
    let index = indexed(&corrections, AS_SCANNED);

    assert_eq!(finds(&index, "הדבר"), 1, "the word the reader put there");
    assert_eq!(
        finds(&index, "הרבר"),
        0,
        "and not the letters the scanner got wrong — the sefer does not say that"
    );
    // The rest of the line is untouched, which is the difference between an
    // overlay and a rewrite.
    assert_eq!(finds(&index, "מצוה"), 1);
}

#[test]
fn a_girsa_variant_is_noted_and_not_indexed() {
    // `Showing::Fixed` — the reading pane's default. A variant says what the
    // text *should* read; the index says what it does. A search that found
    // words the pane does not draw would be a result a reader cannot see when
    // they arrive at it.
    let dir = scratch("girsa-index-variant");
    let corrections = layer(&dir, "הרבר", "הדבר", Kind::Girsa);
    let index = indexed(&corrections, AS_SCANNED);

    assert_eq!(
        finds(&index, "הרבר"),
        1,
        "the printed word is what is findable"
    );
    assert_eq!(finds(&index, "הדבר"), 0, "the emendation is not applied");

    // And the stronger form of the same claim: a segment carrying nothing but
    // variants does not reach the corrected path at all, so this index is
    // bit-identical to one built with no layer. Turning a variant on changes
    // what the reader is shown and never what a query can reach.
    let id = at();
    assert!(corrections.text(&standing(&id), AS_SCANNED).is_none());
}

#[test]
fn a_correction_whose_words_are_gone_changes_nothing_and_is_counted() {
    // The corpus was updated and the line no longer reads what the correction
    // was made from. Never applied — re-anchoring onto different letters is the
    // failure the `was` field exists to prevent — and never silently dropped
    // either, because the reader is the only one who can say what happened to
    // it. An export's header says this; so must a build's report.
    let dir = scratch("girsa-index-stale");
    let corrections = layer(&dir, "הרבר", "הדבר", Kind::Ocr);
    let moved = "פסוק אחר לגמרי שאין בו אותן המילים";
    let id = at();

    let reading = corrections
        .text(&standing(&id), moved)
        .expect("a segment with a correction on it reports one");
    assert_eq!(reading.applied, 0);
    assert_eq!(reading.stale, 1);
    assert_eq!(reading.text, moved, "the corpus text, exactly as it stands");

    let index = indexed(&corrections, moved);
    assert_eq!(finds(&index, "פסוק"), 1);
    assert_eq!(finds(&index, "הדבר"), 0);
}

#[test]
fn a_sefer_nobody_corrected_is_not_asked_about_twice() {
    // The question that keeps the build's cost where it was: a work the layer
    // does not touch skips the standing derivation and the apply entirely, and
    // on a real shelf that is very nearly all 7,189 of them.
    let dir = scratch("girsa-index-untouched");
    let corrections = layer(&dir, "הרבר", "הדבר", Kind::Ocr);

    assert!(corrections.touch("devarim/rambam"));
    assert!(!corrections.touch("bavli/berakhot"));
    assert_eq!(corrections.count(), 1);

    // And with no layer at all there is nothing to ask, which is the state of
    // a fresh install and is not an error.
    assert_eq!(Corrections::none().count(), 0);
    assert!(!Corrections::none().touch("devarim/rambam"));
}
