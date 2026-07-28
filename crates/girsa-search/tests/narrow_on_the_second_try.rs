//! W14 — live facets, and the scope chip they set (spec.md §9.5, §9.8).
//!
//! > *Results carry live facets — shelf section, era, author, sefer, link type
//! > — each with counts, each one click to narrow or exclude. You get it right
//! > on the second try instead of being punished for the first.*
//!
//! The acceptance is one sentence and it is the same promise the relaxation
//! ladder makes one section earlier: **the number on the row is the number
//! clicking it gives you.** A facet whose count and whose result set disagree
//! is worse than no facet, because it is a measurement of the corpus that turns
//! out to be a measurement of nothing.
//!
//! Three more things are asserted here, each of which is a way a facet column
//! could lie quietly:
//!
//! - it counts the **whole result set**, not the page you can see;
//! - a link column nobody built says *not built* rather than showing zeros;
//! - hits in seforim the catalogue does not have are counted out loud, because
//!   otherwise the column simply would not add up and nothing would say why.

// A panic in a test is a failure report.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_corpus::work::{Source, Work};
use girsa_link::EdgeType;
use girsa_ref::resolve::Context;
use girsa_search::bar::{Answer, Bar};
use girsa_search::chips::Chips;
use girsa_search::facets::{self, Catalogue, Dimension, Facets, Links, Row};
use girsa_search::index::{BuildReport, Paging, SearchIndex};
use girsa_search::scope::Scope;

/// A sefer, as the catalogue has it.
fn work(slug: &str, title: &str, categories: &[&str], era: &str, author: &str) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: title.to_string(),
        en_title: slug.to_string(),
        categories: categories.iter().map(|c| (*c).to_string()).collect(),
        source: Source::Sefaria,
        origin: std::path::PathBuf::new(),
        schema: None,
        author: (!author.is_empty()).then(|| author.to_string()),
        era: (!era.is_empty()).then(|| era.to_string()),
        comp_date: None,
        version: None,
        commentary_on: Vec::new(),
    }
}

fn segment(slug: &str, n: u32, text: &str) -> Segment {
    Segment {
        id: SegmentId::new(slug, vec![n.to_string()], Ordinal::root(n)),
        kind: SegmentKind::Text,
        text: text.to_string(),
    }
}

/// The shelf these tests search: three seforim, on two shelves, in two eras,
/// and a word that is in all of them so that one query reaches the lot.
///
/// The counts are deliberately uneven — 12, 7, 4 — so that a facet row taken
/// from the page rather than from the result set is off by a number a test can
/// see, and so that no two rows can be confused for each other.
fn shelf() -> Vec<Work> {
    vec![
        work("bavli/berakhot", "ברכות", &["Talmud", "Bavli"], "A", ""),
        work(
            "bavli/rashi-on-berakhot",
            "רש״י על ברכות",
            &["Talmud", "Bavli", "Rishonim on Talmud"],
            "RI",
            "רש״י",
        ),
        work(
            "mishnah-berurah",
            "משנה ברורה",
            &["Halakhah"],
            "AH",
            "החפץ חיים",
        ),
    ]
}

const IN_BERAKHOT: u32 = 12;
const IN_RASHI: u32 = 7;
const IN_MISHNAH_BERURAH: u32 = 4;

/// An index over that shelf, with link types on some of it.
///
/// Berakhot's segments are commented on; Rashi's are the commentary. The
/// Mishnah Berurah's are touched by nothing, which is what makes the difference
/// between *no links* and *no link column* checkable.
fn loaded(link_types: bool) -> SearchIndex {
    let mut index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    for n in 1..=IN_BERAKHOT {
        writer
            .add(
                &segment("bavli/berakhot", n, "מאימתי קורין את שמע"),
                if link_types {
                    &[EdgeType::CommentsOn, EdgeType::References]
                } else {
                    &[][..]
                },
            )
            .expect("adding a segment");
    }
    for n in 1..=IN_RASHI {
        writer
            .add(
                &segment("bavli/rashi-on-berakhot", n, "מאימתי קורין כלומר"),
                if link_types {
                    &[EdgeType::CommentsOn][..]
                } else {
                    &[][..]
                },
            )
            .expect("adding a segment");
    }
    for n in 1..=IN_MISHNAH_BERURAH {
        writer
            .add(&segment("mishnah-berurah", n, "מאימתי מברכין"), &[])
            .expect("adding a segment");
    }
    writer.commit().expect("committing");
    index.reload().expect("reloading");
    index
        .declare(BuildReport {
            works: 3,
            segments: (IN_BERAKHOT + IN_RASHI + IN_MISHNAH_BERURAH) as usize,
            link_types,
        })
        .expect("declaring what went in");
    index
}

fn bar(link_types: bool) -> Bar {
    Bar::new(
        loaded(link_types),
        Catalogue::of(&shelf()),
        std::path::Path::new("no-corpus-here"),
    )
}

/// Ask for the word every sefer on this shelf has.
fn ask(bar: &Bar, chips: &Chips, paging: Paging) -> (usize, Facets) {
    match bar.ask("מאימתי", chips, paging, &Context::default()) {
        Answer::Segments { results, .. } => (results.total, results.facets),
        other => panic!("expected segments, got {other:?}"),
    }
}

#[test]
fn a_facet_row_promises_the_number_clicking_it_gives() {
    // The acceptance of W14's facets, and the same promise the ladder makes:
    // the count is worked out before the click, from the thing the click will
    // run. Checked for **every** row of every dimension, because one row that
    // does not hold is one row a reader learns not to trust the rest by.
    let bar = bar(true);
    let chips = Chips::default();
    let (total, facets) = ask(&bar, &chips, Paging::first());
    assert_eq!(
        total,
        (IN_BERAKHOT + IN_RASHI + IN_MISHNAH_BERURAH) as usize
    );

    for dimension in Dimension::ALL {
        for row in facets.rows(dimension) {
            let narrowed = Chips {
                scope: facets::narrow(&Scope::everything(), bar.catalogue(), dimension, row),
                ..Chips::default()
            };
            let (after, _) = ask(&bar, &narrowed, Paging::first());
            assert_eq!(
                after,
                row.count,
                "clicking {} [{}] promised {} and gave {after}",
                dimension.label(),
                row.label,
                row.count
            );
        }
    }
}

#[test]
fn excluding_a_row_takes_away_exactly_what_it_said_it_would() {
    // The other click. `only the Bavli` and `anything but the Bavli` are not
    // each other's opposite in a list of fifteen shelves, and a reader chasing
    // a phrase usually wants the second.
    let bar = bar(true);
    let (total, facets) = ask(&bar, &Chips::default(), Paging::first());
    for dimension in [Dimension::Sefer, Dimension::Shelf, Dimension::Era] {
        for row in facets.rows(dimension) {
            let without = Chips {
                scope: facets::exclude(&Scope::everything(), bar.catalogue(), dimension, row),
                ..Chips::default()
            };
            let (after, _) = ask(&bar, &without, Paging::first());
            assert_eq!(
                after,
                total - row.count,
                "excluding {} [{}] should leave {} and left {after}",
                dimension.label(),
                row.label,
                total - row.count
            );
        }
    }
}

#[test]
fn the_facets_count_the_whole_result_set_and_not_the_page() {
    // A facet row that counted the page would tell a reader a shelf holds three
    // of their hits when it holds three hundred — and the number would change
    // as they scrolled, which is the tell that it was never a measurement.
    let bar = bar(true);
    let chips = Chips::default();
    let (whole, all) = ask(&bar, &chips, Paging::first());
    let (paged_total, paged) = ask(&bar, &chips, Paging::of(2));

    assert_eq!(paged_total, whole, "the total is the total either way");
    assert_eq!(
        paged.rows(Dimension::Sefer),
        all.rows(Dimension::Sefer),
        "the facets do not move when the page does"
    );
    let berakhot = paged
        .rows(Dimension::Sefer)
        .iter()
        .find(|r| r.key == "bavli/berakhot")
        .expect("a row for Berakhot");
    assert_eq!(berakhot.count, IN_BERAKHOT as usize);
}

#[test]
fn the_link_facet_counts_segments_and_not_edges() {
    // A segment touched by two kinds of link is one hit under each of them and
    // one hit altogether. The column is therefore allowed not to add up to the
    // total, and the *sefer* column is not.
    let bar = bar(true);
    let (total, facets) = ask(&bar, &Chips::default(), Paging::first());
    let Links::Counted(rows) = &facets.link else {
        panic!("the link types were built");
    };
    let comments = rows
        .iter()
        .find(|r| r.key == "comments-on")
        .expect("a comments-on row");
    assert_eq!(
        comments.count,
        (IN_BERAKHOT + IN_RASHI) as usize,
        "every segment something comments on, from both directions"
    );
    let references = rows
        .iter()
        .find(|r| r.key == "references")
        .expect("a references row");
    assert_eq!(references.count, IN_BERAKHOT as usize);

    let by_sefer: usize = facets.rows(Dimension::Sefer).iter().map(|r| r.count).sum();
    assert_eq!(by_sefer, total, "one hit is in exactly one sefer");
    assert!(
        rows.iter().map(|r| r.count).sum::<usize>() > total,
        "and can be under two kinds of link"
    );
}

#[test]
fn an_index_built_before_the_link_cache_says_so_rather_than_showing_zeros() {
    // spec.md §9.7's rule, one facet over: never a silent gap. *Nothing here is
    // commented on* and *nobody worked out what is commented on* are different
    // statements, and a column of zeros says the first while meaning the
    // second.
    let (_, facets) = ask(&bar(false), &Chips::default(), Paging::first());
    assert_eq!(facets.link, Links::NotBuilt);

    let (_, built) = ask(&bar(true), &Chips::default(), Paging::first());
    assert!(matches!(built.link, Links::Counted(_)));
}

#[test]
fn a_hit_in_a_sefer_the_catalogue_does_not_have_is_counted_out_loud() {
    // The index and the catalogue are two files and one can be ahead of the
    // other. A hit in a sefer the catalogue has never heard of is in the total
    // and in no shelf row, so the column does not add up — and the reader is
    // told by how much rather than left to notice.
    let mut index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    writer
        .add(&segment("who-is-this", 1, "מאימתי מברכין"), &[])
        .expect("adding a segment");
    writer.commit().expect("committing");
    index.reload().expect("reloading");
    index
        .declare(BuildReport {
            works: 1,
            segments: 1,
            link_types: true,
        })
        .expect("declaring");

    let bar = Bar::new(
        index,
        Catalogue::of(&shelf()),
        std::path::Path::new("no-corpus-here"),
    );
    let (total, facets) = ask(&bar, &Chips::default(), Paging::first());
    assert_eq!(total, 1);
    assert_eq!(facets.uncatalogued, 1);
    assert!(facets.rows(Dimension::Shelf).is_empty());
    assert_eq!(facets.rows(Dimension::Sefer).len(), 1);
}

#[test]
fn narrowing_twice_narrows_twice() {
    // Two clicks are an *and*, not a replacement. A second facet click that
    // quietly dropped the first would give a reader a number they could not
    // account for from anything on screen.
    let bar = bar(true);
    let shelf_row = Row {
        key: "תלמוד".to_string(),
        label: "תלמוד".to_string(),
        count: 0,
        depth: 0,
    };
    let era_row = Row {
        key: "RI".to_string(),
        label: "ראשונים".to_string(),
        count: 0,
        depth: 0,
    };
    let scope = facets::narrow(
        &Scope::everything(),
        bar.catalogue(),
        Dimension::Shelf,
        &shelf_row,
    );
    let scope = facets::narrow(&scope, bar.catalogue(), Dimension::Era, &era_row);
    let (after, _) = ask(
        &bar,
        &Chips {
            scope: scope.clone(),
            ..Chips::default()
        },
        Paging::first(),
    );
    assert_eq!(
        after, IN_RASHI as usize,
        "the Bavli **and** the rishonim is Rashi alone"
    );
    assert_eq!(
        scope.describe(),
        "תלמוד · ראשונים",
        "and the chip says both"
    );
}

#[test]
fn the_scope_can_only_ever_take_hits_away() {
    // Every clause a scope adds is a Must or a MustNot over the same result
    // set. This is the property that lets the chip change the number in the
    // header without changing what was searched for — and it is worth a test,
    // because a scope that could widen would be a silent widening with a
    // visible label on it.
    let bar = bar(true);
    let (whole, facets) = ask(&bar, &Chips::default(), Paging::first());
    for dimension in Dimension::ALL {
        for row in facets.rows(dimension) {
            for scope in [
                facets::narrow(&Scope::everything(), bar.catalogue(), dimension, row),
                facets::exclude(&Scope::everything(), bar.catalogue(), dimension, row),
            ] {
                let (after, _) = ask(
                    &bar,
                    &Chips {
                        scope,
                        ..Chips::default()
                    },
                    Paging::first(),
                );
                assert!(
                    after <= whole,
                    "{} [{}] widened",
                    dimension.label(),
                    row.label
                );
            }
        }
    }
}
