//! A scan that has not been read is missing from the search box, and the search
//! box says so.
//!
//! spec.md §9.7, BUILDER.md W26. OCR is optional and off at onboarding, so a
//! shelf with scans on it has holes in its index by design — and the one thing
//! that may not happen is for those holes to be silent:
//!
//! > **Never a silent gap:** the results header says *"4 PDFs on this shelf
//! > aren't searchable yet — [OCR now]"*.
//!
//! A reader who searches a shelf holding four unread scans and is given forty
//! hits has been told *these are the forty places this appears.* The
//! forty-first is on a page nobody has read. That is BUILDER.md rule 6 one
//! layer up from a citation: search that quietly omits a shelf is worse than
//! search that has not been run, because it looks like an answer.
//!
//! The second half of this file is the same rule applied to a reader's own
//! corrections — a fix whose ink the current reading has no word under is
//! **handed back**, not dropped.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::reading::{gap, gap_over, Gap};
use girsa_app::scanning::pages_of;
use girsa_app::shelf::Shelf;
use girsa_scan::reading::{Area, Fix, Read, Reader, Word};
use girsa_scan::words::{Job, Words};
use lopdf::dictionary;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-gap-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A shelf with `n` scans on it, of `pages` pages each.
fn shelf_with_scans(dir: &Path, n: usize, pages: usize) -> (Shelf, Vec<String>) {
    let root = dir.join("corpus");
    let personal = dir.join("personal");
    std::fs::create_dir_all(root.join("works")).expect("a corpus root");
    std::fs::write(root.join("works/index.jsonl"), "").expect("a catalogue");

    let mut shelf = Shelf::open(&root, &personal).expect("a shelf");
    let mut slugs = Vec::new();
    for at in 1..=n {
        let file = dir.join(format!("סריקה {at}.pdf"));
        std::fs::write(&file, blank_pdf(pages)).expect("a pdf");
        slugs.push(shelf.add_mine(&file, None).expect("it goes on the shelf"));
    }
    (shelf, slugs)
}

/// One page, read, with two words on it.
fn read(page: usize, by: Reader) -> Read {
    Read::new(
        page,
        by,
        ["מאימתי", "קורין"]
            .into_iter()
            .enumerate()
            .map(|(at, text)| Word {
                text: text.to_string(),
                #[allow(clippy::cast_precision_loss)]
                at: Area::new(
                    0.80 - at as f32 * 0.09,
                    0.20,
                    0.87 - at as f32 * 0.09,
                    0.222,
                ),
                confidence: 1.0,
            })
            .collect(),
    )
}

#[test]
fn four_pdfs_on_this_shelf_arent_searchable_yet() {
    let dir = scratch("four");
    let (shelf, slugs) = shelf_with_scans(&dir, 4, 12);
    let personal = shelf.personal().to_path_buf();

    let said = gap(&shelf, &personal).said();
    assert_eq!(
        said.as_deref(),
        Some("4 PDFs on this shelf aren't searchable yet — 48 pages"),
        "the sentence spec.md §9.7 asks for, with the count in it"
    );

    // Read one of them cover to cover, blanks included, and the header counts
    // three. Not "one is done" — the reader is being told what they *cannot*
    // see, and a sefer that is finished is not one of those.
    let (mut words, trouble) = Words::open(&personal, &slugs[0]);
    assert!(trouble.is_empty(), "{trouble:?}");
    for page in 1..=12 {
        words.record(read(page, Reader::Embedded)).expect("writes");
    }
    assert_eq!(
        gap(&shelf, &personal).said().as_deref(),
        Some("3 PDFs on this shelf aren't searchable yet — 36 pages")
    );

    // And when the last of them is done the header says nothing at all, which
    // is a different silence from the one this test exists to prevent.
    for slug in &slugs[1..] {
        let (mut words, _) = Words::open(&personal, slug);
        for page in 1..=12 {
            words.record(read(page, Reader::Embedded)).expect("writes");
        }
    }
    assert_eq!(gap(&shelf, &personal), Gap::None);
}

#[test]
fn a_scan_half_read_is_neither_searchable_nor_absent() {
    // The state a reader is actually in most of the time: the job ran for a
    // while and was stopped. Saying *this sefer is searchable* would be a lie
    // about the pages left, and leaving it out of the header would be a lie
    // about the pages done.
    let dir = scratch("half");
    let (shelf, slugs) = shelf_with_scans(&dir, 1, 302);
    let personal = shelf.personal().to_path_buf();

    let (mut words, _) = Words::open(&personal, &slugs[0]);
    for page in 1..=40 {
        words.record(read(page, Reader::Embedded)).expect("writes");
    }

    let Gap::Some { scans, pages } = gap(&shelf, &personal) else {
        panic!("a sefer with 262 unread pages is a gap");
    };
    assert_eq!(pages, 262);
    assert_eq!(scans.len(), 1);
    assert_eq!(scans[0].read, 40);
    assert_eq!(scans[0].pages, 302);
    assert!(!scans[0].is_read());

    // And the queue picks up where it stopped, off the file and nothing else.
    let sefer = shelf.read(&slugs[0]).expect("it opens");
    let (again, _) = Words::open(&personal, &slugs[0]);
    let job = Job::of(&slugs[0], pages_of(&sefer), &again);
    assert_eq!(job.next(), Some(41));
    assert_eq!(job.remaining(), 262);
}

#[test]
fn the_header_is_about_the_shelf_the_reader_is_looking_at() {
    // §9.7 says *on this shelf*, and a search scoped to one sefer must not be
    // told about scans it was never going to search.
    let dir = scratch("scope");
    let (shelf, slugs) = shelf_with_scans(&dir, 3, 10);
    let personal = shelf.personal().to_path_buf();

    assert_eq!(
        gap_over(&shelf, &personal, &slugs[..1]).said().as_deref(),
        Some("1 PDF on this shelf isn't searchable yet — 10 pages")
    );
    // A scope with no scans in it says nothing, and a slug that is not a scan
    // is skipped rather than refused — the caller hands over the whole scope.
    assert_eq!(
        gap_over(&shelf, &personal, &["bavli/berakhot".to_string()]),
        Gap::None
    );
}

#[test]
fn a_correction_whose_ink_the_new_engine_missed_is_reported_not_dropped() {
    // The other half of the same rule. A reader corrected a word; the page was
    // read again by something that found no word there at all. Silently losing
    // the correction means the reader makes it again next year and never knows
    // why it went.
    let dir = scratch("stranded");
    let (shelf, slugs) = shelf_with_scans(&dir, 1, 4);
    let personal = shelf.personal().to_path_buf();

    let (mut words, _) = Words::open(&personal, &slugs[0]);
    words
        .record(read(
            2,
            Reader::Ocr {
                engine: "tesseract v5.4.0".into(),
            },
        ))
        .expect("writes");

    let ink = words.as_read(2).expect("a reading").words[1].at;
    words
        .fix(
            2,
            Fix {
                at: ink,
                was: "קורין".into(),
                says: "קוראין".into(),
            },
        )
        .expect("writes");
    assert_eq!(
        words.page(2).map(|p| p.text()).as_deref(),
        Some("מאימתי קוראין")
    );
    assert!(words.stranded().is_empty());

    // Something better reads the page and finds only the first word.
    words
        .record(Read::new(
            2,
            Reader::Ocr {
                engine: "something better".into(),
            },
            vec![Word {
                text: "מאימתי".into(),
                at: Area::new(0.80, 0.20, 0.87, 0.222),
                confidence: 0.9,
            }],
        ))
        .expect("writes");

    let stranded = words.stranded();
    assert_eq!(stranded.len(), 1, "the correction went missing");
    assert_eq!(stranded[0].0, 2);
    assert_eq!(stranded[0].1.says, "קוראין");
    // The correction is not applied to a word it was not about.
    assert_eq!(words.page(2).map(|p| p.text()).as_deref(), Some("מאימתי"));

    // It survives a restart, because it is on disk and not in the reading.
    let (again, trouble) = Words::open(&personal, &slugs[0]);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(again.stranded().len(), 1);
    assert_eq!(again.read_by(), ["something better"]);
}

/// A PDF of blank pages — the same fixture `the_scan_is_the_daf.rs` uses, and
/// written rather than checked in for the same reason: a binary nobody can diff
/// is a fixture nobody can correct.
fn blank_pdf(pages: usize) -> Vec<u8> {
    let mut doc = lopdf::Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let kids: Vec<lopdf::Object> = (0..pages)
        .map(|_| {
            doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            })
            .into()
        })
        .collect();
    let count = i64::try_from(pages).expect("a page count");
    doc.objects.insert(
        pages_id,
        lopdf::Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => count,
            "Kids" => kids,
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
