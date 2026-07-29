//! Exporting a sefer with your corrections in it (spec.md §7.4, W22).
//!
//! *Base text + applied patches → a clean `.txt`/`.docx`. Falls out of §4.1 for
//! free.* This is the test that it does, and the one that says what "clean"
//! means: the words as they are printed, with the markup gone, with your
//! corrections in place, and with the provenance of those corrections at the
//! top rather than hidden.
//!
//! The `.docx` half is checked by **reading it back into Girsa**. The importer
//! that reads a Word file you dropped on the window (W10) is the same one, so a
//! file this writes and Girsa cannot open would be a file Word probably cannot
//! open either.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::export::{self, Format};
use girsa_app::shelf::Shelf;
use girsa_corpus::import::{self, ImportedWork, RawSegment, SegmentKind};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::{Source, Work};
use girsa_fix::{Kind, Patch};

const SLUG: &str = "mishnah-berurah";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-export-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A sefer with a heading, some markup, some nikud, and a scanning error.
fn shelf_with_a_typo(root: &Path) -> Vec<SegmentId> {
    let raw = vec![
        RawSegment {
            path: vec!["1".into()],
            kind: SegmentKind::Heading,
            text: "סימן א".into(),
        },
        RawSegment {
            path: vec!["1".into(), "1".into()],
            kind: SegmentKind::Text,
            text: "<b>יתגבר</b> כארי וּבַשַּׁבָּת כל הרבר הזה".into(),
        },
        RawSegment {
            path: vec!["1".into(), "2".into()],
            kind: SegmentKind::Text,
            text: "ולא יתבייש מפני המלעיגים".into(),
        },
    ];
    let work = Work {
        slug: SLUG.to_string(),
        he_title: "משנה ברורה".into(),
        en_title: "Mishnah Berurah".into(),
        categories: vec!["Halakhah".into()],
        source: Source::Sefaria,
        origin: PathBuf::new(),
        schema: None,
        author: Some("ישראל מאיר הכהן".into()),
        era: Some("AH".into()),
        comp_date: None,
        version: None,
        he_sections: Vec::new(),
        commentary_on: Vec::new(),
    };
    let imported = ImportedWork::assemble(work, raw);
    import::write(root, &imported).expect("writes");
    std::fs::create_dir_all(root.join("works")).expect("dir");
    std::fs::write(
        root.join("works/index.jsonl"),
        format!(
            "{}\n",
            serde_json::to_string(&imported.work).expect("writes")
        ),
    )
    .expect("writes");
    imported.segments.iter().map(|s| s.id.clone()).collect()
}

/// The shelf, with `הרבר` corrected to `הדבר`.
fn corrected(root: &Path) -> Shelf {
    let ids = shelf_with_a_typo(root);
    let mut shelf = Shelf::open(root, &root.join("personal")).expect("the shelf opens");
    let sefer = shelf.read(SLUG).expect("the sefer opens");
    let patch = girsa_app::correction(&sefer, &ids[1], 20..24, "הדבר", Kind::Ocr, "me", false)
        .expect("a correction");
    assert_eq!(patch.was, "הרבר");
    shelf.fix(patch).expect("it is taken");
    shelf
}

#[test]
fn a_text_file_comes_out_with_the_correction_in_it_and_the_markup_gone() {
    let root = scratch("txt");
    let shelf = corrected(&root);
    let sefer = shelf.read(SLUG).expect("the sefer opens");

    let to = root.join("out/mishnah-berurah.txt");
    let done = export::export(&sefer, shelf.fixes(), Format::Txt, true, &to).expect("exports");
    assert_eq!(done.segments, 3);
    assert_eq!(done.corrections, 1);

    let body = std::fs::read_to_string(&to).expect("reads");
    assert!(body.contains("כל הדבר הזה"), "{body}");
    assert!(
        !body.contains("הרבר"),
        "the printed error is not in it: {body}"
    );
    assert!(!body.contains("<b>"), "the markup is gone: {body}");
    assert!(body.contains("וּבַשַּׁבָּת"), "and the nikud is not: {body}");
    // Where it came from and what was done to it, at the top — a corrected
    // sefer handed to somebody has to say that it was corrected.
    assert!(body.starts_with("משנה ברורה"), "{body}");
    assert!(body.contains("תיקון אחד"), "{body}");
    assert!(body.contains("סימן א"), "the heading is there: {body}");
}

#[test]
fn the_nikud_comes_off_when_that_is_what_you_are_reading() {
    let root = scratch("bare");
    let shelf = corrected(&root);
    let sefer = shelf.read(SLUG).expect("the sefer opens");
    let to = root.join("out/bare.txt");
    export::export(&sefer, shelf.fixes(), Format::Txt, false, &to).expect("exports");
    let body = std::fs::read_to_string(&to).expect("reads");
    assert!(body.contains("ובשבת"), "{body}");
    assert!(!body.contains("וּבַשַּׁבָּת"), "{body}");
}

#[test]
fn a_word_file_comes_out_and_girsa_can_read_it_back() {
    // The round trip is the check. `girsa-corpus`'s .docx reader is the one
    // that reads a Word file dropped on the window, so a file this writes that
    // it cannot read is a file Word would very likely refuse as well.
    let root = scratch("docx");
    let shelf = corrected(&root);
    let sefer = shelf.read(SLUG).expect("the sefer opens");

    let to = root.join("out/mishnah-berurah.docx");
    let done = export::export(&sefer, shelf.fixes(), Format::Docx, true, &to).expect("exports");
    assert_eq!(done.corrections, 1);

    let mine = root.join("mine");
    let added = import::mine::add(&mine, &to, None).expect("Girsa reads it back");
    let words: Vec<&str> = added.segments.iter().map(|s| s.text.as_str()).collect();
    assert!(
        words.iter().any(|line| line.contains("כל הדבר הזה")),
        "{words:?}"
    );
    assert!(!words.iter().any(|line| line.contains("הרבר")), "{words:?}");
    // The headings were declared as headings and come back as headings — the
    // sefer's title, which the export makes the first one, and the siman.
    let headings: Vec<&str> = added
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::Heading)
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(headings, ["משנה ברורה", "סימן א"], "{words:?}");
    // And the provenance line is in the file rather than only in the count.
    assert!(
        words.iter().any(|line| line.contains("הוחלו תיקון אחד")),
        "{words:?}"
    );
}

#[test]
fn a_sefer_with_no_corrections_exports_and_says_so() {
    let root = scratch("untouched");
    shelf_with_a_typo(&root);
    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let sefer = shelf.read(SLUG).expect("the sefer opens");
    let to = root.join("out/plain.txt");
    let done = export::export(&sefer, shelf.fixes(), Format::Txt, true, &to).expect("exports");
    assert_eq!(done.corrections, 0);
    let body = std::fs::read_to_string(&to).expect("reads");
    assert!(body.contains("כל הרבר הזה"), "as printed: {body}");
    assert!(
        !body.contains("תיקון"),
        "and nothing claiming otherwise: {body}"
    );
}

#[test]
fn a_stale_correction_is_named_in_the_file_rather_than_left_out_quietly() {
    // A patch whose words are no longer in the segment is not applied (W20).
    // Exporting is exactly when somebody would never find out — so the count
    // comes back, and the header says it.
    let root = scratch("stale");
    // Made through the shelf, so the layer on disk has a real correction in it
    // before the stale one is added beside it.
    drop(corrected(&root));

    let mut layer = girsa_fix::Layer::open(&root.join("personal")).0;
    let ids: Vec<SegmentId> = import::read_back(&root, SLUG)
        .expect("reads")
        .segments
        .iter()
        .map(|s| s.id.clone())
        .collect();
    layer
        .add(Patch::new(
            ids[2].clone(),
            0..3,
            "אבג",
            "אבד",
            Kind::Ocr,
            "me",
        ))
        .expect("takes it");

    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let sefer = shelf.read(SLUG).expect("the sefer opens");
    let to = root.join("out/stale.txt");
    let done = export::export(&sefer, shelf.fixes(), Format::Txt, true, &to).expect("exports");
    assert_eq!(done.corrections, 1);
    assert_eq!(done.stale, 1);
    let body = std::fs::read_to_string(&to).expect("reads");
    assert!(body.contains("תיקון אחד שלא חל"), "{body}");
}
