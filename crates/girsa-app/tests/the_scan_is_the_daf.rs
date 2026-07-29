//! A scan, from the file being dropped to the mekor being written down.
//!
//! spec.md §6.3, BUILDER.md W25. The crate's own tests prove the arithmetic;
//! this one proves the path a reader actually walks, on a real PDF written to a
//! real personal layer:
//!
//! 1. drop a scan on the shelf — it is a sefer, with permanent ids;
//! 2. tell it which page daf ב is on — thirty seconds, once;
//! 3. copy the mekor off page 47 and get `ברכות כג.`, carrying a ref that
//!    resolves into the library rather than into a file on one disk.
//!
//! And the property that is not obvious and is the reason any of this is shaped
//! the way it is: **the mapping does not touch the ids.** A reader who gets the
//! anchor wrong and fixes it a week later has moved every citation the scan
//! prints and **no note they have written on it**, because the two are
//! different things — which is the whole of W6 said again about pages.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::scanning::{self, mareh_makom, scan_of};
use girsa_app::shelf::Shelf;
use girsa_cite::CiteStyle;
use girsa_corpus::import;
use girsa_corpus::work::{Source, Work};
use girsa_scan::{Anchor, Paging, Placed, Scheme};
use lopdf::dictionary;

const PAGES: usize = 120;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-scan-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A corpus with Berakhot in it, so a scan has something to say it is of.
fn corpus_with_berakhot(root: &Path) -> Work {
    let work = Work {
        slug: "bavli/berakhot".to_string(),
        he_title: "ברכות".to_string(),
        en_title: "Berakhot".to_string(),
        categories: vec!["Talmud".to_string(), "Bavli".to_string()],
        source: Source::Sefaria,
        origin: PathBuf::new(),
        schema: None,
        author: None,
        era: None,
        comp_date: None,
        version: None,
        he_sections: vec!["דף".to_string(), "שורה".to_string()],
        commentary_on: Vec::new(),
    };
    std::fs::create_dir_all(root.join("works")).expect("a corpus root");
    std::fs::write(
        root.join("works/index.jsonl"),
        format!("{}\n", serde_json::to_string(&work).expect("a work")),
    )
    .expect("a catalogue");
    work
}

/// Drop a scan on the shelf and open it.
fn shelf_with_a_scan(dir: &Path) -> (Shelf, String) {
    let root = dir.join("corpus");
    let personal = dir.join("personal");
    corpus_with_berakhot(&root);

    let file = dir.join("ברכות דפוס ווילנא.pdf");
    std::fs::write(&file, blank_pdf(PAGES)).expect("a pdf");

    let mut shelf = Shelf::open(&root, &personal).expect("a shelf");
    let slug = shelf.add_mine(&file, None).expect("it goes on the shelf");
    (shelf, slug)
}

#[test]
fn a_scan_is_a_sefer_of_pages_and_says_so_rather_than_opening_empty() {
    // The defect this work order found in the window: a PDF opened into the
    // reading pane as a hundred and twenty blank lines. It is not a corrupt
    // import — it is a scan, and there is nothing to *read*; there is something
    // to *look at*. Everything downstream turns on the app being able to tell.
    let dir = scratch("a-sefer");
    let (shelf, slug) = shelf_with_a_scan(&dir);
    let sefer = shelf.read(&slug).expect("it opens");

    assert!(scanning::is_scan(&sefer.work), "{slug} is a scan");
    assert_eq!(scanning::pages_of(&sefer), PAGES);
    assert!(
        sefer.segments.iter().all(|s| s.text.is_empty()),
        "a scan has no words until it is OCR'd, and inventing some is worse"
    );
    // A text sefer of the corpus is not one, so nothing routes Berakhot into a
    // viewer that has no file to show.
    let berakhot = shelf.work("bavli/berakhot").expect("on the shelf");
    assert!(!scanning::is_scan(berakhot));

    let scan = scan_of(&shelf, &sefer).expect("a scan");
    assert_eq!(scan.pages(), PAGES);
    assert!(
        !scan.is_paged(),
        "nobody has done the chore yet, and that is not the same as having no dafim"
    );
    assert_eq!(scan.at(1), Placed::Unpaged);
}

#[test]
fn thirty_seconds_of_declaring_makes_every_page_of_it_citable() {
    let dir = scratch("citable");
    let (mut shelf, slug) = shelf_with_a_scan(&dir);

    // The chore: four pages of front matter, so page 5 is ב. — and this scan
    // is a scan *of* Berakhot, which is what makes its mekoros the same
    // mekoros everybody else writes.
    shelf
        .declare_paging(
            &slug,
            Paging::declare(
                Some("bavli/berakhot".to_string()),
                Scheme::Amud,
                vec![Anchor::written(5, "ב.").expect("an anchor")],
            )
            .expect("a mapping"),
        )
        .expect("it saves");

    let sefer = shelf.read(&slug).expect("it opens");
    let scan = scan_of(&shelf, &sefer).expect("a scan");
    let naming = scanning::naming(&shelf, &scan).expect("Berakhot is on the shelf");
    let scanned = &sefer.work;

    // Page 47: four pages of front matter, then 42 amudim on from ב. — daf
    // כג, amud alef. Printed with no gershayim, because the full stop is what
    // says these letters are a daf and a gershayim would be saying it twice.
    let sent = mareh_makom(&scan, 47, &naming, scanned, CiteStyle::HebrewShort)
        .expect("page 47 is inside the mapping");
    assert_eq!(sent.plain, "ברכות כג.");
    assert_eq!(sent.packet.reference, "girsa:bavli/berakhot/23a");
    assert!(
        sent.packet.text.is_empty(),
        "there is nothing to quote off a scan nobody has OCR'd"
    );
    // Which scan it was read in — a mekor off the Vilna and one off the Lemberg
    // are the same place and not the same page.
    assert!(
        sent.packet.version.provenance.contains("ווילנא"),
        "{:?}",
        sent.packet.version
    );

    // And what Ksav writes for it is a mareh makom, not an empty quote block.
    let markup = girsa_ksav::to_ksav(&sent.packet, girsa_ksav::CitationPlacement::Mekor);
    assert!(!markup.contains("#ציטוט"), "{markup}");
    assert!(markup.contains("girsa:bavli/berakhot/23a"), "{markup}");

    // The page after it is the other side of the same leaf.
    let over = mareh_makom(&scan, 48, &naming, scanned, CiteStyle::HebrewShort)
        .expect("page 48 is inside the mapping");
    assert_eq!(over.plain, "ברכות כג:");

    // The front matter is described and not cited. There is no daf א in any
    // masechta, and offering one would be a mekor to a place never printed.
    assert!(mareh_makom(&scan, 2, &naming, scanned, CiteStyle::HebrewShort).is_none());

    // And the other direction, which is what makes a scan a reading mode: a ref
    // — from a search hit, a link, or a mekor clicked in a Ksav document —
    // lands on the page it is printed on.
    let line: girsa_ref::Ref = "girsa:bavli/berakhot/23a:4".parse().expect("a ref");
    assert_eq!(scan.page_of_ref(&line), Some(47));
}

#[test]
fn a_ref_into_a_paged_scan_means_one_page_and_not_two() {
    // Found by running `girsa-daf` against a real 302-page PDF rather than by
    // a test, which is why it is here.
    //
    // A scan's segments are addressed by the **file's** page — page 47 is `47`.
    // A sefer numbered by page has its own numbers, and once six pages of front
    // matter are taken off, printed 41 *is* file page 47. Both are plain
    // numbers. So `girsa:user/x/41` meant printed 41 to the viewer and file 41
    // to everything that resolves a ref, seven pages apart, with nothing
    // anywhere saying which was meant — two answers for one ref.
    //
    // Once the reader says what the pages are called, that is what an address
    // of this sefer means, everywhere.
    let dir = scratch("one-page");
    let (mut shelf, slug) = shelf_with_a_scan(&dir);
    shelf
        .declare_paging(
            &slug,
            Paging::declare(
                None,
                Scheme::Numbered,
                vec![Anchor::written(7, "1").expect("an anchor")],
            )
            .expect("a mapping"),
        )
        .expect("it saves");

    let sefer = shelf.read(&slug).expect("it opens");
    let scan = scan_of(&shelf, &sefer).expect("a scan");
    let printed_41 = girsa_ref::Address::parse("41").expect("an address");

    assert_eq!(scan.page_of(&printed_41), Some(47));
    assert_eq!(
        sefer.at(&printed_41),
        scanning::page_id(&sefer, 47)
            .into_iter()
            .collect::<Vec<_>>(),
        "the ref and the viewer have to land on the same page"
    );

    // And the shaar blatt is not a place in the sefer. It keeps its permanent
    // id — a note written on it stays on it — and it is not reachable by an
    // address, because the reader has said the sefer starts on page 7.
    assert!(sefer
        .at(&girsa_ref::Address::parse("300").expect("an address"))
        .is_empty());
    assert!(scanning::page_id(&sefer, 1).is_some());
}

#[test]
fn re_declaring_the_mapping_moves_the_citations_and_not_one_permanent_id() {
    // The property W6 exists for, said about pages. The reader counts the front
    // matter wrong, writes notes on forty pages, then fixes the anchor. What
    // must move: what each page is *called*. What must not: which page each of
    // their notes is *on*.
    let dir = scratch("ids-hold");
    let (mut shelf, slug) = shelf_with_a_scan(&dir);

    let paging = |first: usize| {
        Paging::declare(
            None,
            Scheme::Amud,
            vec![Anchor::written(first, "ב.").expect("an anchor")],
        )
        .expect("a mapping")
    };

    shelf.declare_paging(&slug, paging(5)).expect("it saves");
    let sefer = shelf.read(&slug).expect("it opens");
    let ids_before: Vec<String> = (1..=PAGES)
        .filter_map(|p| scanning::page_id(&sefer, p).map(|id| id.to_string()))
        .collect();
    let scan = scan_of(&shelf, &sefer).expect("a scan");
    let said_before = scan.at(47);

    // Off by two: the scan has six pages of front matter, not four.
    shelf.declare_paging(&slug, paging(7)).expect("it saves");
    let sefer = shelf.read(&slug).expect("it opens");
    let ids_after: Vec<String> = (1..=PAGES)
        .filter_map(|p| scanning::page_id(&sefer, p).map(|id| id.to_string()))
        .collect();
    let scan = scan_of(&shelf, &sefer).expect("a scan");

    assert_eq!(ids_before.len(), PAGES);
    assert_eq!(
        ids_before, ids_after,
        "a mapping said what the pages are called; it may not rename them"
    );
    assert_ne!(
        said_before,
        scan.at(47),
        "and the citation is what the correction was for"
    );
    assert_eq!(
        scan.page_of(&girsa_ref::Address::parse("2a").expect("an address")),
        Some(7)
    );
}

#[test]
fn the_scan_beside_the_gemara_turns_to_the_daf_the_gemara_is_on() {
    // W9's acceptance, in the second reading mode: *scrolling the Gemara moves
    // the column beside it to the matching ref* — and here the column is a
    // photograph, so what moves is which page is on the screen.
    //
    // It follows only because the reader **said** this is a scan of Berakhot.
    // A scan and a text that merely share an address shape line up beautifully
    // and mean nothing, and W9 exists because a column that moved on a
    // resemblance shows a reader one place while the header names another.
    let dir = scratch("beside");
    let (mut shelf, slug) = shelf_with_a_scan(&dir);
    shelf
        .declare_paging(
            &slug,
            Paging::declare(
                Some("bavli/berakhot".to_string()),
                Scheme::Amud,
                vec![Anchor::written(5, "ב.").expect("an anchor")],
            )
            .expect("a mapping"),
        )
        .expect("it saves");
    let sefer = shelf.read(&slug).expect("it opens");
    let scan = scan_of(&shelf, &sefer).expect("a scan");

    // A line of the Gemara, addressed the way the corpus addresses one.
    let line = girsa_corpus::segment::SegmentId::new(
        "bavli/berakhot",
        vec!["23a".to_string(), "4".to_string()],
        girsa_corpus::segment::Ordinal::root(9),
    );
    assert_eq!(scanning::beside(&scan, &line), Some(47));

    // A daf this scan does not carry: the pane is told nothing rather than
    // turned to the nearest page it has.
    let far = girsa_corpus::segment::SegmentId::new(
        "bavli/berakhot",
        vec!["90a".to_string()],
        girsa_corpus::segment::Ordinal::root(1),
    );
    assert_eq!(scanning::beside(&scan, &far), None);

    // And another masechta entirely.
    let elsewhere = girsa_corpus::segment::SegmentId::new(
        "bavli/shabbat",
        vec!["23a".to_string()],
        girsa_corpus::segment::Ordinal::root(1),
    );
    assert_eq!(scanning::beside(&scan, &elsewhere), None);

    // A scan that has not been told what it is a scan of follows nothing, even
    // though its dafim would line up.
    shelf
        .declare_paging(
            &slug,
            Paging::declare(
                None,
                Scheme::Amud,
                vec![Anchor::written(5, "ב.").expect("an anchor")],
            )
            .expect("a mapping"),
        )
        .expect("it saves");
    let sefer = shelf.read(&slug).expect("it opens");
    let alone = scan_of(&shelf, &sefer).expect("a scan");
    assert_eq!(scanning::beside(&alone, &line), None);
}

#[test]
fn a_scan_that_names_a_sefer_this_shelf_does_not_have_is_refused_by_name() {
    // The reader said this is a scan of Eruvin, and Eruvin is not on this
    // shelf. Printing it under the filename instead would answer a different
    // question without saying so — the reader would get `ברכות דפוס ווילנא ב.`
    // for a sefer they told it was something else.
    let dir = scratch("no-such-sefer");
    let (mut shelf, slug) = shelf_with_a_scan(&dir);
    shelf
        .declare_paging(
            &slug,
            Paging::declare(
                Some("bavli/eruvin".to_string()),
                Scheme::Amud,
                vec![Anchor::written(5, "ב.").expect("an anchor")],
            )
            .expect("a mapping"),
        )
        .expect("it saves");

    let sefer = shelf.read(&slug).expect("it opens");
    let scan = scan_of(&shelf, &sefer).expect("a scan");
    let refused = scanning::naming(&shelf, &scan).expect_err("Eruvin is not here");
    assert!(refused.to_string().contains("bavli/eruvin"), "{refused}");
}

#[test]
fn a_mapping_is_kept_in_your_own_layer_and_survives_the_window_closing() {
    let dir = scratch("survives");
    let (mut shelf, slug) = shelf_with_a_scan(&dir);
    shelf
        .declare_paging(
            &slug,
            Paging::declare(
                None,
                Scheme::Amud,
                vec![Anchor::written(5, "ב.").expect("an anchor")],
            )
            .expect("a mapping"),
        )
        .expect("it saves");

    // Written under `personal/`, never under the corpus root — which
    // `girsa-import` rewrites in full on every run.
    let personal = dir.join("personal");
    assert!(girsa_scan::Scans::path_in(&personal).is_file());
    assert!(!dir.join("corpus/scans.json").exists());

    let again = Shelf::open(&dir.join("corpus"), &personal).expect("a shelf");
    assert!(again.trouble().is_none(), "{:?}", again.trouble());
    let sefer = again.read(&slug).expect("it opens");
    let scan = scan_of(&again, &sefer).expect("a scan");
    assert!(scan.is_paged());
    assert_eq!(
        scan.at(5),
        Placed::At {
            from: girsa_ref::Address::parse("2a").expect("an address"),
            to: None
        }
    );
}

/// A PDF of `pages` blank pages: enough of one to have a page tree, which is
/// what the importer counts through.
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

/// Keeps `import` used: a scan read back off the shelf is the sefer that was
/// written, page for page.
#[test]
fn a_scan_reads_back_off_the_shelf_as_the_sefer_that_was_written() {
    let dir = scratch("read-back");
    let (shelf, slug) = shelf_with_a_scan(&dir);
    let back = import::read_back(shelf.personal(), &slug).expect("it reads back");
    assert_eq!(back.segments.len(), PAGES);
    assert!(back.segments.iter().all(|s| s.id.is_well_formed()));
}
