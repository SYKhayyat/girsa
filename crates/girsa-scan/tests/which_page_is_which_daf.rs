//! The mapping, and the one property it exists for.
//!
//! spec.md §6.3: *a page → daf mapping makes a scanned sefer citable. Small
//! once-per-sefer chore; large payoff.* The chore is small because it is a
//! declaration — *page 5 is ב.* — and the count runs on from there.
//!
//! # What makes it a chore rather than a setting
//!
//! A scan is not a clean stack of dafim. A Vilna Shas PDF opens with a title
//! page and haskamos; somewhere in the middle a plate, a blank verso or a
//! photograph of the original shaar blatt is bound in, and from there the
//! arithmetic that was right for four hundred pages is one daf out for the rest
//! of the sefer.
//!
//! The easy design is one number: *the daf is the page plus 3*. It is right
//! until it isn't, and when it isn't the only repair is to change the number —
//! which moves **every citation in the sefer**, including the ones that were
//! already right, silently, exactly the way an Otzaria line index moves when a
//! line is inserted above it (BUILDER.md T1). This test is written against that
//! failure: the mapping is a list of anchors, and declaring a new one may not
//! move a page before it.
//!
//! Run it against the one-offset implementation and cases 1, 3 and 6 fail.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_ref::Address;
use girsa_scan::{Anchor, Paging, Placed, Scan, Scheme};

fn address(written: &str) -> Address {
    Address::parse(written).unwrap_or_else(|| panic!("{written} is not an address"))
}

/// A scan of Berakhot: four pages of front matter, then the sefer.
fn berakhot(pages: usize) -> Scan {
    let paging = Paging::declare(None, Scheme::Amud, vec![anchor(5, "ב.")])
        .expect("page 5 is amud alef of daf ב");
    Scan::new("user/berakhot-vilna", pages, paging)
}

fn anchor(page: usize, written: &str) -> Anchor {
    Anchor::written(page, written).unwrap_or_else(|e| panic!("{written}: {e}"))
}

/// What a page says it is, for comparing a whole scan against itself.
fn placed(scan: &Scan, page: usize) -> String {
    match scan.at(page) {
        Placed::At { from, to: None } => from.to_string(),
        Placed::At { from, to: Some(to) } => format!("{from}-{to}"),
        Placed::Unpaged => "—".to_string(),
    }
}

#[test]
fn a_page_before_the_first_anchor_is_not_given_a_daf() {
    // The title page is not daf א, and there is no daf א — `girsa-ref`'s daf
    // reader refuses one, because the first leaf of a masechta is its title
    // page. An implementation that extrapolates backwards from the anchor
    // hands the reader a citation to a place that does not exist in any
    // masechta ever printed.
    let scan = berakhot(60);
    for page in 1..=4 {
        assert_eq!(scan.at(page), Placed::Unpaged, "page {page}");
        assert_eq!(scan.reference(page), None, "page {page}");
    }
    assert_ne!(scan.at(5), Placed::Unpaged);
}

#[test]
fn the_daf_counts_on_from_the_anchor_one_amud_a_page() {
    let scan = berakhot(60);
    for (page, daf) in [(5, "2a"), (6, "2b"), (7, "3a"), (8, "3b"), (13, "6a")] {
        assert_eq!(
            scan.at(page),
            Placed::At {
                from: address(daf),
                to: None
            },
            "page {page}"
        );
    }
}

#[test]
fn a_plate_bound_into_the_middle_moves_no_page_before_it() {
    // The property the whole shape is for, and the sibling of W6's 501 links.
    //
    // The reader has read to daf כ and finds the count is out: two plates are
    // bound in after page 42, so what the mapping calls 22a is printed 21a.
    // They fix it by declaring what they can see — pages 43 and 44 are not
    // pages of the sefer, and page 45 is כא. — and **nothing they read before
    // page 43 is allowed to move.**
    let before = berakhot(80);
    let was: Vec<String> = (1..=80).map(|p| placed(&before, p)).collect();

    let after = Scan::new(
        "user/berakhot-vilna",
        80,
        Paging::declare(
            None,
            Scheme::Amud,
            vec![anchor(5, "ב."), Anchor::unpaged(43), anchor(45, "כא.")],
        )
        .expect("two plates, and the sefer picks up again at כא."),
    );

    for page in 1..=42 {
        assert_eq!(
            placed(&after, page),
            was[page - 1],
            "page {page} moved, and it is before the plates"
        );
    }
    // The plates themselves are pages of the file and not of the sefer.
    assert_eq!(after.at(43), Placed::Unpaged);
    assert_eq!(after.at(44), Placed::Unpaged);
    // And the sefer picks up where the reader says it does.
    assert_eq!(placed(&after, 45), "21a");
    assert_eq!(placed(&after, 46), "21b");
    // Which is one daf back from where the old mapping had it — the whole
    // reason the reader declared anything.
    assert_eq!(was[44], "22a");
}

#[test]
fn the_page_a_daf_is_on_is_the_page_that_daf_is_on() {
    // Both directions, over the whole scan: the mapping is a bijection between
    // the pages it covers and the dafim they carry, or `page_of` is a lookup
    // that lands somewhere else.
    let scan = berakhot(200);
    for page in 1..=200 {
        match scan.at(page) {
            Placed::At { from, .. } => {
                assert_eq!(scan.page_of(&from), Some(page), "{from} is on page {page}");
            }
            Placed::Unpaged => {}
        }
    }
    assert_eq!(scan.page_of(&address("2a")), Some(5));
    // However the reader writes the daf — `girsa-ref` reads six notations and
    // an address goes through all of them. `ב:` is **not** among them here and
    // that is not an oversight: a colon separates the levels of an address, so
    // amud beis is written `ב ע"ב` when it is being read as one.
    assert_eq!(scan.page_of(&address("ב.")), Some(5));
    assert_eq!(scan.page_of(&address("ב ע\"ב")), Some(6));
}

#[test]
fn a_daf_the_scan_does_not_reach_is_not_the_nearest_page_it_has() {
    // 200 pages from daf ב is up to daf קא, and the reader asking for קכא is
    // asking for a page this scan does not have. The answer is *not here*
    // rather than the last page, for the reason every lookup in this codebase
    // refuses to round: a scan opened one daf away, with the header naming the
    // daf that was asked for, is wrong in the one way nobody checks.
    let scan = berakhot(200);
    assert_eq!(scan.page_of(&address("121a")), None);
    // And below it: daf ב is the first thing in the scan.
    let short = berakhot(10);
    assert_eq!(short.page_of(&address("40a")), None);
}

#[test]
fn a_mapping_that_puts_two_pages_on_one_daf_is_refused() {
    // A duplicated page — the same daf photographed twice, which happens — or
    // an anchor typed one out. Either way `page_of` stops being a function and
    // one of the two pages becomes unreachable, silently. Refused with both
    // pages named, so the reader can look at them.
    let refused = Paging::declare(
        None,
        Scheme::Amud,
        vec![anchor(5, "ב."), anchor(9, "ג ע\"ב")],
    )
    .expect_err("page 9 is ד. by the first anchor, and ג ע\"ב is behind it");
    let said = refused.to_string();
    assert!(said.contains('9'), "{said}");
    assert!(said.contains("3b") || said.contains('8'), "{said}");

    // The same page twice is the same defect, said sooner.
    assert!(Paging::declare(None, Scheme::Amud, vec![anchor(5, "ב."), anchor(5, "ג.")]).is_err());
}

#[test]
fn an_anchor_that_is_not_the_scheme_it_was_declared_under_is_refused() {
    // `page 5 is 17` under a scheme that counts amudim is a siman where a daf
    // was asked for, and the arithmetic would silently treat it as daf 17.
    assert!(Paging::declare(None, Scheme::Amud, vec![anchor(5, "17")]).is_err());
    // And the other way: a daf where the sefer is numbered by page.
    assert!(Paging::declare(None, Scheme::Numbered, vec![anchor(5, "ב.")]).is_err());
    // Page 0 is not a page.
    assert!(Anchor::written(0, "ב.").is_err());
}

#[test]
fn a_scan_where_each_page_is_a_whole_daf_carries_both_amudim() {
    // Some scans are photographs of the open sefer rather than of one side of
    // a leaf, so a page is a daf. A page is then a **span**, which is what a
    // ref has been since W3 — a quote is a range, and so is a daf.
    let scan = Scan::new(
        "user/shas-open",
        50,
        Paging::declare(None, Scheme::Daf, vec![anchor(3, "ב")]).expect("page 3 is daf ב"),
    );
    assert_eq!(
        scan.at(3),
        Placed::At {
            from: address("2a"),
            to: Some(address("2b"))
        }
    );
    assert_eq!(placed(&scan, 4), "3a-3b");
    // Either amud finds the page it is printed on.
    assert_eq!(scan.page_of(&address("2b")), Some(3));
    assert_eq!(scan.page_of(&address("3a")), Some(4));
}

#[test]
fn a_sefer_numbered_by_page_counts_by_one() {
    // Not everything is a masechta. A sefer whose pages carry printed numbers —
    // or whose divisions run one to a page — is declared the same way, and the
    // front matter is the reason the printed number is not the file's page.
    let scan = Scan::new(
        "user/shut",
        400,
        Paging::declare(None, Scheme::Numbered, vec![anchor(9, "1")]).expect("page 9 is printed 1"),
    );
    assert_eq!(placed(&scan, 9), "1");
    assert_eq!(placed(&scan, 10), "2");
    assert_eq!(scan.page_of(&address("100")), Some(108));
    assert_eq!(placed(&scan, 8), "—");
}

#[test]
fn a_scan_nobody_has_paged_says_so_rather_than_guessing() {
    // The state every scan is in the moment it is dropped on the window. It is
    // on the shelf, its pages have permanent ids, and it has no mareh makom
    // until somebody spends the thirty seconds. Not a defect — an undone chore,
    // and the difference has to be visible.
    let scan = Scan::new("user/something", 20, Paging::default());
    assert!(!scan.is_paged());
    for page in 1..=20 {
        assert_eq!(scan.at(page), Placed::Unpaged);
    }
    assert_eq!(scan.page_of(&address("2a")), None);
}
