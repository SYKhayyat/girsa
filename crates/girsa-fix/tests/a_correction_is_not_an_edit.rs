//! What the overlay buys, as a runnable fact rather than a paragraph.
//!
//! spec.md §7.1 says corrections are patches and the shipped corpus stays
//! pristine, and lists what that buys: reverting, *show as printed / show
//! corrected*, surviving corpus updates, and handing a patch file to someone
//! else. Each of those is a test here, and each is run **twice** — once against
//! the overlay and once against the obvious alternative, which is to open the
//! file and fix the word.
//!
//! The alternative lives in this file as [`in_place`]. It is four lines and it
//! is correct as written; what it cannot do is any of the four things above,
//! and that is the whole argument.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::import::{self, ImportedWork, RawSegment, SegmentKind};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::{Source, Work};
use girsa_fix::{Kind, Layer, Patch, Showing};

/// A typo of the kind the corpus really has: ד read as ר by a scanner.
const AS_PRINTED: &str = "כל הרבר הזה טעות סופר";
const AS_IT_SHOULD_BE: &str = "כל הדבר הזה טעות סופר";

fn work(slug: &str) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: slug.to_string(),
        en_title: slug.to_string(),
        categories: Vec::new(),
        order: Vec::new(),
        source: Source::Sefaria,
        origin: PathBuf::new(),
        schema: None,
        author: None,
        era: None,
        comp_date: None,
        version: None,
        he_sections: Vec::new(),
        commentary_on: Vec::new(),
    }
}

/// A sefer of `lines` on disk, written the way `girsa-import` writes one.
fn sefer(root: &Path, slug: &str, lines: &[&str]) -> Vec<SegmentId> {
    let raw = lines
        .iter()
        .enumerate()
        .map(|(i, text)| RawSegment {
            path: vec!["1".into(), (i + 1).to_string()],
            kind: SegmentKind::Text,
            text: (*text).to_string(),
        })
        .collect();
    let imported = ImportedWork::assemble(work(slug), raw);
    import::write(root, &imported).expect("writes");
    imported.segments.iter().map(|s| s.id.clone()).collect()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-fix-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The obvious alternative: open the file and fix the word.
///
/// Kept here for the same reason `girsa_corpus::store::LineIndexStore` is kept
/// — so that "why not just edit it" is answered by a run rather than by a
/// paragraph somebody stops believing in.
fn in_place(root: &Path, slug: &str, id: &SegmentId, now: &str) {
    let mut read = import::read_back(root, slug).expect("reads");
    for segment in &mut read.segments {
        if segment.id == *id {
            segment.text = now.to_string();
        }
    }
    import::write(root, &read).expect("writes");
}

fn text_of(root: &Path, slug: &str, id: &SegmentId) -> String {
    import::read_back(root, slug)
        .expect("reads")
        .segments
        .iter()
        .find(|s| s.id == *id)
        .map(|s| s.text.clone())
        .unwrap_or_default()
}

/// The patch a reader makes by highlighting `הרבר` and typing `הדבר`.
fn a_typo_fixed(id: &SegmentId) -> Patch {
    Patch::new(id.clone(), 3..7, "הרבר", "הדבר", Kind::Ocr, "me")
}

#[test]
fn the_shipped_text_is_the_same_bytes_after_a_correction_and_is_not_after_an_edit() {
    let root = scratch("pristine");
    let ids = sefer(&root, "mishnah-berurah", &[AS_PRINTED, "שורה שניה"]);
    let file = import::work_dir(&root, "mishnah-berurah").join("segments.jsonl");
    let before = std::fs::read(&file).expect("reads");

    let (mut layer, trouble) = Layer::open(&root.join("personal"));
    assert!(trouble.is_empty(), "{trouble:?}");
    layer.add(a_typo_fixed(&ids[0])).expect("takes the patch");

    assert_eq!(
        std::fs::read(&file).expect("reads"),
        before,
        "a correction may not write one byte into the corpus"
    );
    // And it is a correction: the words a reader gets are the corrected ones.
    let shown = layer.apply(&ids[0], AS_PRINTED, Showing::Fixed);
    assert_eq!(shown.text, AS_IT_SHOULD_BE);
    assert_eq!(shown.applied.len(), 1);

    // The alternative, for comparison.
    in_place(&root, "mishnah-berurah", &ids[0], AS_IT_SHOULD_BE);
    assert_ne!(
        std::fs::read(&file).expect("reads"),
        before,
        "an in-place edit rewrites the sefer, which is the thing §7.1 forbids"
    );
}

#[test]
fn show_as_printed_still_has_the_printed_words_and_after_an_edit_they_are_gone() {
    let root = scratch("as-printed");
    let ids = sefer(&root, "mishnah-berurah", &[AS_PRINTED]);
    let (mut layer, _) = Layer::open(&root.join("personal"));
    layer.add(a_typo_fixed(&ids[0])).expect("takes the patch");

    let base = text_of(&root, "mishnah-berurah", &ids[0]);
    assert_eq!(
        layer.apply(&ids[0], &base, Showing::AsPrinted).text,
        AS_PRINTED
    );
    assert_eq!(
        layer.apply(&ids[0], &base, Showing::Fixed).text,
        AS_IT_SHOULD_BE
    );

    in_place(&root, "mishnah-berurah", &ids[0], AS_IT_SHOULD_BE);
    let base = text_of(&root, "mishnah-berurah", &ids[0]);
    assert_eq!(
        base, AS_IT_SHOULD_BE,
        "the edit worked — and there is now no way to ask what was printed"
    );
}

#[test]
fn a_correction_survives_the_corpus_being_re_imported_and_an_edit_does_not() {
    // The one that decides it. `girsa-import` rewrites every work it owns on
    // every run, so a fix living in the corpus is a fix that lasts until the
    // next update — and nothing says a word when it goes.
    let root = scratch("re-import");
    let ids = sefer(&root, "mishnah-berurah", &[AS_PRINTED]);
    let (mut layer, _) = Layer::open(&root.join("personal"));
    layer.add(a_typo_fixed(&ids[0])).expect("takes the patch");
    in_place(&root, "mishnah-berurah", &ids[0], AS_IT_SHOULD_BE);

    // Upstream ships the same sefer again.
    sefer(&root, "mishnah-berurah", &[AS_PRINTED]);

    let base = text_of(&root, "mishnah-berurah", &ids[0]);
    assert_eq!(base, AS_PRINTED, "the in-place fix is gone, silently");
    let (layer, _) = Layer::open(&root.join("personal"));
    assert_eq!(
        layer.apply(&ids[0], &base, Showing::Fixed).text,
        AS_IT_SHOULD_BE,
        "the patch is in your layer and the update could not reach it"
    );
}

#[test]
fn a_patch_can_be_taken_back_and_the_sefer_reads_as_printed_again() {
    let root = scratch("revert");
    let ids = sefer(&root, "mishnah-berurah", &[AS_PRINTED]);
    let (mut layer, _) = Layer::open(&root.join("personal"));
    let id = layer
        .add(a_typo_fixed(&ids[0]))
        .expect("takes it")
        .id
        .clone();

    assert!(layer.remove(&id).expect("removes"));
    assert_eq!(
        layer.apply(&ids[0], AS_PRINTED, Showing::Fixed).text,
        AS_PRINTED
    );
    // And it stayed removed.
    let (layer, _) = Layer::open(&root.join("personal"));
    assert_eq!(layer.count(), 0);
}

#[test]
fn a_patch_file_can_be_handed_to_someone_else() {
    // spec.md §7.1. It is a file of lines, so this is a copy — but the merge
    // has to be idempotent, or the person who takes a patch file twice ends up
    // with every correction applied twice on top of itself.
    let mine = scratch("mine");
    let yours = scratch("yours");
    let ids = sefer(&mine, "mishnah-berurah", &[AS_PRINTED]);

    let (mut layer, _) = Layer::open(&mine.join("personal"));
    layer.add(a_typo_fixed(&ids[0])).expect("takes it");

    let (mut theirs, _) = Layer::open(&yours.join("personal"));
    let took = theirs.merge(layer.path()).expect("merges");
    assert_eq!((took.taken, took.already_had), (1, 0));
    assert_eq!(
        theirs.apply(&ids[0], AS_PRINTED, Showing::Fixed).text,
        AS_IT_SHOULD_BE
    );

    let again = theirs.merge(layer.path()).expect("merges");
    assert_eq!(
        (again.taken, again.already_had),
        (0, 1),
        "taking it twice is taking it once"
    );
    assert_eq!(theirs.count(), 1);
}

#[test]
fn a_patch_whose_words_are_no_longer_there_is_reported_rather_than_applied() {
    // BUILDER.md rule 6, in the place it is most dangerous: upstream re-typed
    // the line, the span now covers different letters, and applying the patch
    // by its offsets would rewrite words nobody asked about.
    let root = scratch("stale");
    let ids = sefer(&root, "mishnah-berurah", &[AS_PRINTED]);
    let (mut layer, _) = Layer::open(&root.join("personal"));
    layer.add(a_typo_fixed(&ids[0])).expect("takes it");

    let rewritten = "טעות סופר אחרת לגמרי כאן";
    let shown = layer.apply(&ids[0], rewritten, Showing::Fixed);
    assert_eq!(shown.text, rewritten, "nothing was applied");
    assert_eq!(shown.stale.len(), 1);
    assert!(shown.applied.is_empty());
}

#[test]
fn a_patch_whose_words_moved_along_the_line_still_lands_on_them() {
    // The common case of a corpus update: a word is inserted earlier in the
    // segment and every offset after it is out by four characters. The patch
    // carries the words it was made against, so it can be re-found — and this
    // is not a guess, because it is taken **only when they are there exactly
    // once**.
    let root = scratch("moved");
    let ids = sefer(&root, "mishnah-berurah", &[AS_PRINTED]);
    let (mut layer, _) = Layer::open(&root.join("personal"));
    layer.add(a_typo_fixed(&ids[0])).expect("takes it");

    let with_a_word_added = format!("והנה {AS_PRINTED}");
    let shown = layer.apply(&ids[0], &with_a_word_added, Showing::Fixed);
    assert_eq!(shown.text, format!("והנה {AS_IT_SHOULD_BE}"));
    assert_eq!(shown.moved.len(), 1, "and it says that it moved");
    assert!(shown.stale.is_empty());
}

#[test]
fn a_patch_whose_words_are_there_twice_is_refused() {
    let root = scratch("twice");
    let ids = sefer(&root, "mishnah-berurah", &[AS_PRINTED]);
    let (mut layer, _) = Layer::open(&root.join("personal"));
    layer.add(a_typo_fixed(&ids[0])).expect("takes it");

    let twice = format!("הרבר {AS_PRINTED}");
    let shown = layer.apply(&ids[0], &twice, Showing::Fixed);
    assert_eq!(shown.text, twice);
    assert_eq!(shown.stale.len(), 1);
}

#[test]
fn a_girsa_variant_is_the_same_machinery_and_a_different_claim() {
    // spec.md §7.2. An OCR error is *the scanner misread this*; a hagahah is
    // *the Gra reads it differently*. One mechanism, and the reader can see
    // which of the two they are looking at — a variant is not applied by
    // default, because silently replacing the text you are learning with
    // somebody's emendation is a claim made on your behalf.
    let root = scratch("girsa");
    let ids = sefer(&root, "bavli/berakhot", &[AS_PRINTED]);
    let (mut layer, _) = Layer::open(&root.join("personal"));
    layer.add(a_typo_fixed(&ids[0])).expect("takes it");
    layer
        .add(
            Patch::new(ids[0].clone(), 8..11, "הזה", "ההוא", Kind::Girsa, "הגר\"א")
                .from_source("girsa:hagahot-hagra/1:1"),
        )
        .expect("takes it");

    let fixed = layer.apply(&ids[0], AS_PRINTED, Showing::Fixed);
    assert_eq!(
        fixed.text, AS_IT_SHOULD_BE,
        "the scanning error, and only it"
    );
    assert_eq!(
        fixed.noted.len(),
        1,
        "the variant is noted rather than applied"
    );

    let with = layer.apply(&ids[0], AS_PRINTED, Showing::FixedWithVariants);
    assert_eq!(with.text, "כל הדבר ההוא טעות סופר");
    assert_eq!(with.applied.len(), 2);
    assert_eq!(
        with.applied[1].source.as_deref(),
        Some("girsa:hagahot-hagra/1:1"),
        "and it says who says so"
    );
}

#[test]
fn two_patches_on_one_line_both_land_and_an_overlapping_one_is_refused() {
    let root = scratch("two");
    let ids = sefer(&root, "mishnah-berurah", &["אבגד ההוו זחטי"]);
    let (mut layer, _) = Layer::open(&root.join("personal"));
    layer
        .add(Patch::new(
            ids[0].clone(),
            0..4,
            "אבגד",
            "אבגר",
            Kind::Ocr,
            "me",
        ))
        .expect("takes it");
    layer
        .add(Patch::new(
            ids[0].clone(),
            10..14,
            "זחטי",
            "זחטו",
            Kind::Ocr,
            "me",
        ))
        .expect("takes it");
    assert_eq!(
        layer.apply(&ids[0], "אבגד ההוו זחטי", Showing::Fixed).text,
        "אבגר ההוו זחטו"
    );

    // Two corrections claiming the same letters is a conflict, and there is no
    // right answer to pick — so it is refused at the moment it is made, which
    // is the only moment the reader is there to be told.
    let clash = layer.add(Patch::new(
        ids[0].clone(),
        2..6,
        "גד ה",
        "גד ו",
        Kind::Ocr,
        "me",
    ));
    assert!(clash.is_err(), "{clash:?}");
    assert_eq!(layer.count(), 2);
}

#[test]
fn a_correction_names_a_segment_and_not_a_line_number() {
    // T1, at the one place it could come back in: the patch file. A patch that
    // said "line 4 of this file" would move every time a line above it was
    // split — which is precisely the failure the corrections feature would be
    // causing, rather than the one it is fixing.
    let root = scratch("anchors");
    let ids = sefer(&root, "mishnah-berurah", &["ראשון", AS_PRINTED, AS_PRINTED]);
    let (mut layer, _) = Layer::open(&root.join("personal"));
    layer.add(a_typo_fixed(&ids[1])).expect("takes it");

    let body = std::fs::read_to_string(layer.path()).expect("reads");
    assert!(
        body.contains("girsa:mishnah-berurah/1:2#2"),
        "the patch names the segment: {body}"
    );
    assert!(
        !body.contains("\"line\"") && !body.contains("\"nth\""),
        "and nothing in it is a position: {body}"
    );

    // The third segment holds the same words, at the same offsets, and is a
    // different place. A patch keyed by anything but the id — a line number, a
    // hash of the text — would land on both.
    assert_eq!(
        layer.apply(&ids[2], AS_PRINTED, Showing::Fixed).text,
        AS_PRINTED
    );
    assert_eq!(
        layer.apply(&ids[1], AS_PRINTED, Showing::Fixed).text,
        AS_IT_SHOULD_BE
    );
}
