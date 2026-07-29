//! BUILDER.md W10, the second half.
//!
//! > User PDFs/DOCX/TXT droppable at any time, first-class.
//!
//! *First-class* is the whole assertion, and it is not a feeling: a sefer of
//! yours has to have **permanent segment ids** like everything else (§3), sit
//! in the catalogue like everything else, and — the one that is easy to get
//! wrong and impossible to notice — **survive a corpus update**, because
//! `girsa-import` rewrites the corpus catalogue in full on every run.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::import::{self, SegmentKind};
use girsa_corpus::work::Source;
use lopdf::dictionary;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn write(dir: &Path, name: &str, body: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("the file writes");
    path
}

#[test]
fn a_text_file_dropped_in_becomes_a_sefer_with_permanent_names() {
    let dir = scratch("girsa-mine-txt");
    let personal = dir.join("personal");
    let file = write(
        &dir,
        "חבורה על הסוגיא.txt",
        "ראשון\n\nשני\n\nשלישי\n".as_bytes(),
    );

    let added = import::mine::add(&personal, &file, None).expect("it is added");
    assert_eq!(added.work.slug, "user/חבורה-על-הסוגיא");
    assert_eq!(added.work.he_title, "חבורה על הסוגיא");
    assert_eq!(added.work.source, Source::Mine);
    assert_eq!(added.work.categories, vec!["שלי".to_string()]);
    assert_eq!(added.segments.len(), 3);
    assert_eq!(added.segments[1].text, "שני");

    // The ids are ids, not positions — the same rule the corpus is rebuilt to
    // keep (spec.md §3), and no weaker for being your own file.
    assert_eq!(
        added.segments[0].id.to_string(),
        "girsa:user/חבורה-על-הסוגיא/1#1"
    );
    assert!(added.segments.iter().all(|s| s.id.is_well_formed()));

    // And it is on the shelf: written where the reader's own layer lives, and
    // readable back by slug like anything else.
    let back = import::read_back(&personal, &added.work.slug).expect("it reads back");
    assert_eq!(back.segments, added.segments);
    let catalogue =
        std::fs::read_to_string(personal.join("works/index.jsonl")).expect("catalogued");
    assert!(catalogue.contains("user/חבורה-על-הסוגיא"));
}

#[test]
fn a_hebrew_text_file_that_is_not_utf8_is_read_rather_than_mangled() {
    let dir = scratch("girsa-mine-cp1255");
    let personal = dir.join("personal");
    // `שלום` in windows-1255, which is what a Hebrew .txt off a Windows
    // machine actually is. As UTF-8 these four bytes are not a string at all.
    let file = write(&dir, "ישן.txt", &[0xF9, 0xEC, 0xE5, 0xED]);

    let added = import::mine::add(&personal, &file, None).expect("it is added");
    assert_eq!(added.segments.len(), 1);
    assert_eq!(added.segments[0].text, "שלום");
}

#[test]
fn a_docx_keeps_the_headings_word_declared_and_does_not_invent_any() {
    let dir = scratch("girsa-mine-docx");
    let personal = dir.join("personal");
    let file = dir.join("שיעור.docx");
    docx(
        &file,
        &[("Heading1", "פרק ראשון"), ("", "גוף הענין"), ("", "")],
    );

    let added = import::mine::add(&personal, &file, None).expect("it is added");
    assert_eq!(
        added.segments.len(),
        2,
        "an empty paragraph is not a segment"
    );
    assert_eq!(added.segments[0].kind, SegmentKind::Heading);
    assert_eq!(added.segments[0].text, "פרק ראשון");
    assert_eq!(added.segments[1].kind, SegmentKind::Text);
}

#[test]
fn your_own_writing_goes_on_the_shelf_as_the_words_and_not_as_the_markup() {
    // spec.md §10.4 — *send text into the library: your writing becomes a
    // sefer on the shelf, searchable and citable.* A `.ksav` file is a Typst
    // document, and what a reader wants on the shelf is what they wrote, not
    // `#כותרת1[`.
    let dir = scratch("girsa-mine-ksav");
    let personal = dir.join("personal");
    let markup = "#כותרת1[סוגיית מאימתי]

                  #ציטוט[מאימתי קורין את שמע]#מראה_מקום(מקור: \"girsa:bavli/berakhot/2a:1\")[ברכות ב.]

                  ונראה לי דהכי פירושו.
";
    let file = write(&dir, "חבורה.ksav", markup.as_bytes());

    let added = import::mine::add(&personal, &file, None).expect("it is added");
    let text: Vec<&str> = added.segments.iter().map(|s| s.text.as_str()).collect();
    assert!(text.iter().any(|t| t.contains("סוגיית מאימתי")), "{text:?}");
    assert!(
        text.iter().any(|t| t.contains("מאימתי קורין את שמע")),
        "{text:?}"
    );
    assert!(
        text.iter().any(|t| t.contains("ונראה לי דהכי פירושו")),
        "{text:?}"
    );

    // The commands and the ref are not words anybody wrote, and would be
    // found by a search for them if they were indexed.
    assert!(!text.iter().any(|t| t.contains("כותרת1")), "{text:?}");
    assert!(!text.iter().any(|t| t.contains("girsa:bavli")), "{text:?}");

    // And it is a sefer like any other: permanent ids, on your own shelf.
    assert_eq!(added.work.source, Source::Mine);
    assert!(added.segments.iter().all(|s| s.id.is_well_formed()));
    let back = import::read_back(&personal, &added.work.slug).expect("it reads back");
    assert_eq!(back.segments, added.segments);
}

#[test]
fn a_pdf_is_on_the_shelf_with_its_pages_and_says_it_has_no_words_yet() {
    let dir = scratch("girsa-mine-pdf");
    let personal = dir.join("personal");
    let file = write(&dir, "צילום.pdf", &pdf_of_two_blank_pages());

    let added = import::mine::add(&personal, &file, None).expect("it is added");
    assert_eq!(added.segments.len(), 2, "one segment per page");
    assert!(added.segments.iter().all(|s| s.kind == SegmentKind::Page));
    assert!(
        added.segments.iter().all(|s| s.text.is_empty()),
        "a scan has no text until it is OCR'd, and inventing some is worse \
         than saying so"
    );
    assert_eq!(added.segments[1].id.to_string(), "girsa:user/צילום/2#2");
}

#[test]
fn a_file_of_a_kind_nobody_can_read_is_refused_by_name() {
    let dir = scratch("girsa-mine-refused");
    let personal = dir.join("personal");
    let file = write(&dir, "משהו.epub", b"not a sefer");
    let refused = import::mine::add(&personal, &file, None).expect_err("refused");
    let said = refused.to_string();
    for kind in import::mine::ACCEPTS {
        assert!(
            said.contains(kind),
            "the refusal says what it does read: {said}"
        );
    }

    // And an empty file is refused too, rather than becoming a sefer with
    // nothing in it.
    let empty = write(&dir, "ריק.txt", b"   \n\n  \n");
    assert!(import::mine::add(&personal, &empty, None).is_err());
}

#[test]
fn two_files_with_one_name_are_two_seforim() {
    let one = scratch("girsa-mine-clash-1");
    let two = scratch("girsa-mine-clash-2");
    let personal = one.join("personal");
    let a = write(&one, "חבורה.txt", "ראשון".as_bytes());
    let b = write(&two, "חבורה.txt", "אחר לגמרי".as_bytes());

    let first = import::mine::add(&personal, &a, None).expect("added");
    let second = import::mine::add(&personal, &b, None).expect("added");
    assert_ne!(first.work.slug, second.work.slug);
    // The first one is still readable, and still says what it said.
    let back = import::read_back(&personal, &first.work.slug).expect("still here");
    assert_eq!(back.segments[0].text, "ראשון");
    assert_eq!(
        import::read_back(&personal, &second.work.slug)
            .unwrap()
            .segments[0]
            .text,
        "אחר לגמרי"
    );
}

/// The one that is easy to get wrong: `girsa-import` truncates and rewrites
/// `corpus/works/index.jsonl` on every run. A sefer of yours filed in that file
/// would be gone at the next corpus update with nothing to say so.
#[test]
fn a_corpus_reimport_does_not_touch_your_own_seforim() {
    let dir = scratch("girsa-mine-reimport");
    let corpus = dir.join("corpus");
    let personal = dir.join("personal");
    std::fs::create_dir_all(corpus.join("works")).unwrap();
    std::fs::write(corpus.join("works/index.jsonl"), "").unwrap();

    let file = write(&dir, "שלי.txt", "הענין".as_bytes());
    let added = import::mine::add(&personal, &file, None).expect("added");

    // What a re-import does to the file it owns.
    std::fs::write(corpus.join("works/index.jsonl"), "{\"nonsense\":true}\n").unwrap();

    let back = import::read_back(&personal, &added.work.slug).expect("your sefer is still here");
    assert_eq!(back.segments[0].text, "הענין");
    let catalogue = std::fs::read_to_string(personal.join("works/index.jsonl")).unwrap();
    assert!(catalogue.contains(&added.work.slug));
}

// ---------------------------------------------------------------------------
// Fixtures: the smallest real .docx and .pdf that a reader could drop in.
// ---------------------------------------------------------------------------

/// A .docx is a zip with one interesting file in it.
fn docx(path: &Path, paragraphs: &[(&str, &str)]) {
    let mut body = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
    );
    for (style, text) in paragraphs {
        body.push_str("<w:p>");
        if !style.is_empty() {
            body.push_str(&format!("<w:pPr><w:pStyle w:val=\"{style}\"/></w:pPr>"));
        }
        body.push_str(&format!(
            "<w:r><w:t xml:space=\"preserve\">{text}</w:t></w:r>"
        ));
        body.push_str("</w:p>");
    }
    body.push_str("</w:body></w:document>");

    let file = std::fs::File::create(path).expect("the docx writes");
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file(
        "word/document.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("a document part");
    std::io::Write::write_all(&mut zip, body.as_bytes()).expect("written");
    zip.finish().expect("finished");
}

/// Two blank pages, hand-written: enough of a PDF to have a page tree.
fn pdf_of_two_blank_pages() -> Vec<u8> {
    let mut doc = lopdf::Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let page = |doc: &mut lopdf::Document| {
        doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        })
    };
    let first = page(&mut doc);
    let second = page(&mut doc);
    doc.objects.insert(
        pages_id,
        lopdf::Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => 2,
            "Kids" => vec![first.into(), second.into()],
        }),
    );
    let catalog = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog);

    let mut out = Vec::new();
    doc.save_to(&mut out).expect("the pdf writes");
    out
}
