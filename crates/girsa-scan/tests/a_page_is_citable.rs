//! What the mapping is *for*.
//!
//! spec.md §6.3 does not ask for a page counter. It asks for a scan to be
//! **citable** — for a reader who has just read something on page 47 of a PDF
//! to be able to write down where it is, in the words a sefer is cited in, and
//! for that citation to come back to page 47 a year later.
//!
//! So the assertion here is the one `girsa-cite` makes about every other
//! citation in this system, extended to a scan: **what a page cites as reads
//! back as the page it came from.** A mareh makom that cannot be followed is a
//! string, and this project has an app for strings.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_cite::{CiteStyle, Sefer};
use girsa_ref::{resolve, Lexicon, Resolution};
use girsa_scan::{Anchor, Paging, Scan, Scheme};

const SLUG: &str = "user/berakhot-vilna";
const TITLE: &str = "ברכות דפוס ווילנא";

fn anchor(page: usize, written: &str) -> Anchor {
    Anchor::written(page, written).unwrap_or_else(|e| panic!("{written}: {e}"))
}

/// A scan of Berakhot with four pages of front matter, standing on its own —
/// nothing on the shelf says what it is a scan *of*.
fn its_own() -> Scan {
    let paging = Paging::declare(None, Scheme::Amud, vec![anchor(5, "ב.")]).expect("page 5 is ב.");
    Scan::new(SLUG, 400, paging)
}

fn sefer() -> Sefer {
    Sefer::new(TITLE, "Berakhot (Vilna)")
}

#[test]
fn a_page_cites_as_the_daf_printed_on_it() {
    let scan = its_own();
    assert_eq!(
        scan.cite(5, &sefer(), CiteStyle::HebrewShort).as_deref(),
        Some("ברכות דפוס ווילנא ב.")
    );
    assert_eq!(
        scan.cite(6, &sefer(), CiteStyle::HebrewShort).as_deref(),
        Some("ברכות דפוס ווילנא ב:")
    );
    assert_eq!(
        scan.reference(7).map(|r| r.to_string()),
        Some("girsa:user/berakhot-vilna/3a".to_string())
    );
}

#[test]
fn a_page_with_no_daf_on_it_is_not_given_one() {
    // The haskamos. There is nothing printed on them that a mekor could name,
    // and inventing `א.` would be a citation to a daf that does not exist.
    // The window says *page 2 of the file*; this crate says **nothing**, which
    // is the difference between describing a page and citing one.
    let scan = its_own();
    for page in 1..=4 {
        assert_eq!(scan.cite(page, &sefer(), CiteStyle::HebrewShort), None);
        assert_eq!(scan.reference(page), None);
    }
    // Nor is a page past the end of the file a place.
    assert_eq!(scan.cite(401, &sefer(), CiteStyle::HebrewShort), None);
}

#[test]
fn what_a_page_cites_as_reads_back_as_the_page_it_came_from() {
    // The property. Printed one way, read back the other, over every page of
    // the scan and in all three styles — the same claim `girsa-cite` makes
    // about the corpus, now about a PDF somebody scanned themselves.
    let scan = its_own();
    let mut lexicon = Lexicon::default();
    lexicon.add(
        girsa_ref::Work {
            slug: SLUG.to_string(),
            he_title: TITLE.to_string(),
            en_title: "Berakhot (Vilna)".to_string(),
        },
        &[TITLE, "Berakhot (Vilna)"],
    );

    let mut checked = 0usize;
    for page in (5..=400).step_by(7) {
        for style in [
            CiteStyle::HebrewShort,
            CiteStyle::HebrewFull,
            CiteStyle::English,
        ] {
            let printed = scan
                .cite(page, &sefer(), style)
                .unwrap_or_else(|| panic!("page {page} is inside the mapping"));
            match resolve(&lexicon, &printed) {
                Resolution::Exact(back) => assert_eq!(
                    scan.page_of_ref(&back),
                    Some(page),
                    "{printed:?} ({style:?}) came back as a different page"
                ),
                other => panic!("{printed:?} ({style:?}) did not read back: {other:?}"),
            }
            checked += 1;
        }
    }
    assert!(checked > 150, "only {checked} citations checked");
}

#[test]
fn a_scan_of_a_sefer_on_the_shelf_cites_that_sefer() {
    // The payoff of saying what the scan is *of*. A reader who owns a scan of
    // Berakhot and writes down where they read it should be writing
    // `ברכות ב.` — the same mekor everyone else writes, resolving to the same
    // place in the library — and not a mareh makom into a file on their disk
    // that nobody else has.
    let paging = Paging::declare(
        Some("bavli/berakhot".to_string()),
        Scheme::Amud,
        vec![anchor(5, "ב.")],
    )
    .expect("a scan of Berakhot");
    let scan = Scan::new(SLUG, 400, paging);

    assert_eq!(
        scan.reference(5).map(|r| r.to_string()),
        Some("girsa:bavli/berakhot/2a".to_string())
    );
    let berakhot = Sefer::new("ברכות", "Berakhot").with_sections(["דף", "שורה"]);
    assert_eq!(
        scan.cite(5, &berakhot, CiteStyle::HebrewShort).as_deref(),
        Some("ברכות ב.")
    );
}

#[test]
fn a_ref_into_the_sefer_opens_the_page_it_is_printed_on() {
    // The other direction, and what makes a scan a second reading mode rather
    // than a folder of images: a search hit, a link, a mekor clicked in a Ksav
    // document — all of them are refs, and a ref lands on a page.
    let paging = Paging::declare(
        Some("bavli/berakhot".to_string()),
        Scheme::Amud,
        vec![anchor(5, "ב.")],
    )
    .expect("a scan of Berakhot");
    let scan = Scan::new(SLUG, 400, paging);

    let line: girsa_ref::Ref = "girsa:bavli/berakhot/2a:5".parse().expect("a ref");
    assert_eq!(
        scan.page_of_ref(&line),
        Some(5),
        "the scan knows the daf a line is on, which is the page it is printed on"
    );

    // A ref into a different sefer is not this scan's business — and answering
    // with a page anyway is how a reader ends up looking at Berakhot with the
    // header saying Shabbos.
    let elsewhere: girsa_ref::Ref = "girsa:bavli/shabbat/2a:5".parse().expect("a ref");
    assert_eq!(scan.page_of_ref(&elsewhere), None);
}
