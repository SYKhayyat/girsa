//! BUILDER.md W10, against the real shelf.
//!
//! > Browse by the real taxonomy (Tanach / Shas / Halacha / Machshava /
//! > Chassidus / Responsa / yours), **with the arrangement editable**.
//!
//! The corpus does not have one taxonomy. It has two — Sefaria writes
//! `Talmud/Bavli/Acharonim on Talmud` in English and Otzaria writes
//! `תלמוד בבלי/אחרונים` in Hebrew — and a shelf that shows both is not a shelf,
//! it is the seam between two downloads. So the assertions here are about what
//! a reader would notice: **one top level, in Hebrew, and every sefer on
//! exactly one shelf.**
//!
//! # Why it skips when the corpus is absent
//!
//! It reads the imported shelf, which is not committed and is not there on a
//! fresh clone.

// A panic in a test is a failure report.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use girsa_corpus::taxonomy::{self, TOP};
use girsa_corpus::work::Work;

fn works() -> Option<Vec<Work>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let index: PathBuf = root.join("works/index.jsonl");
    let body = std::fs::read_to_string(index).ok()?;
    Some(
        body.lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Work>(l).ok())
            .collect(),
    )
}

macro_rules! works_or_skip {
    () => {
        match works() {
            Some(works) if !works.is_empty() => works,
            _ => {
                eprintln!("skipped: no imported corpus — run girsa-import first");
                return;
            }
        }
    };
}

#[test]
fn every_sefer_lands_on_exactly_one_shipped_shelf() {
    let works = works_or_skip!();

    let mut counted = BTreeMap::<String, usize>::new();
    let mut homeless = Vec::new();
    for work in &works {
        let shelf = taxonomy::shelf_of(work);
        match shelf.first() {
            Some(top) if TOP.contains(&top.as_str()) => {
                *counted.entry(top.clone()).or_default() += 1;
            }
            _ => homeless.push((work.slug.clone(), shelf.join("/"))),
        }
    }

    for (slug, shelf) in homeless.iter().take(20) {
        eprintln!("not on any shipped shelf: {slug} — {shelf}");
    }
    assert!(
        homeless.is_empty(),
        "{} of {} seforim are on no shipped shelf",
        homeless.len(),
        works.len()
    );

    // And the shelves account for every sefer exactly once: a work counted
    // twice is a work a reader meets twice, and one counted nowhere is a work
    // that is on the shelf and cannot be found by browsing.
    let total: usize = counted.values().sum();
    assert_eq!(total, works.len(), "{counted:#?}");
    eprintln!("{counted:#?}");
}

#[test]
fn the_two_corpora_land_on_the_same_shelf_rather_than_beside_each_other() {
    let works = works_or_skip!();

    // Sefaria says `Talmud/Bavli/Acharonim on Talmud`; Otzaria says
    // `תלמוד בבלי/אחרונים`. They are the same shelf and a reader browsing for
    // an acharon on the Gemara has to find both in one place.
    let shelf_of_slug = |slug: &str| {
        works
            .iter()
            .find(|w| w.slug == slug)
            .map(|w| taxonomy::shelf_of(w).join("/"))
    };

    let mut acharonim = BTreeSet::new();
    for work in &works {
        let first = work.categories.first().map(String::as_str);
        let is_bavli_acharon = match first {
            Some("Talmud") => {
                work.categories.get(1).map(String::as_str) == Some("Bavli")
                    && work.categories.get(2).map(String::as_str) == Some("Acharonim on Talmud")
            }
            Some("תלמוד בבלי") => {
                work.categories.get(1).map(String::as_str) == Some("אחרונים")
            }
            _ => false,
        };
        if is_bavli_acharon {
            // The shelf, not the sub-shelf: both corpora subdivide further, by
            // author and by collection, and that is theirs to do. What matters
            // is that a reader opening `תלמוד/בבלי/אחרונים` is standing in
            // front of both of them.
            let shelf = taxonomy::shelf_of(work);
            acharonim.insert(shelf.iter().take(3).cloned().collect::<Vec<_>>().join("/"));
        }
    }

    eprintln!("bavli acharonim are shelved at: {acharonim:#?}");
    assert_eq!(
        acharonim.len(),
        1,
        "the acharonim on the Bavli are on {} different shelves",
        acharonim.len()
    );

    // Spot checks a reader would notice, in the vocabulary of the shelf rather
    // than of the download. Berakhot keeps its seder, because Sefaria knows
    // which one it is in and a shelf that threw that away would be worse than
    // the download it came from.
    assert_eq!(
        shelf_of_slug("bavli/berakhot").as_deref(),
        Some("תלמוד/בבלי/סדר זרעים")
    );
    assert_eq!(
        shelf_of_slug("shulchan-arukh/orach-chayim").as_deref(),
        Some("הלכה/שולחן ערוך")
    );
}
