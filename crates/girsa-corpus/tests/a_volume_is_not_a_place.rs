//! A permanent id that names 1.2 MB of text names a volume, not a place (B12).
//!
//! Measured over the real corpus: 5,733 of 5,000,545 segments are over 10,000
//! characters, 119 over 50,000, 19 over 200,000, and the largest is **1,275,307** —
//! `girsa:bavli/chiddushei-harambam-on-rosh-hashanah/20b:7#32`. 926 works are
//! affected, including `tur` (68), `beit-yosef` (55), `akeidat-yitzchak` (183) and
//! `abarbanel-on-torah` (70). Those are not obscure works.
//!
//! The whole architecture rests on *"each record carries its own id, so every
//! anchor still names the same words"*. Three things degrade together at that size:
//! the citation is unusable as a mareh makom, a highlight cannot help, and a search
//! result is *"it is somewhere in here."*
//!
//! This file asserts both halves of the fix, and the second half is the one that
//! makes the first safe:
//!
//!  1. the importer cuts a segment too long to be a place into places;
//!  2. **everything anchored to the parent still names the same words** — which is
//!     `Ordinal::covers`, the property spec.md §3 minted the ordinal scheme for.
//!
//! `anchors_survive_editing.rs` is the same argument for a *correction* that splits
//! a segment. This is the argument for an *import* that does.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use girsa_corpus::import::{ImportedWork, RawSegment, SegmentKind};
use girsa_corpus::oversized::{self, NAMES_A_PLACE};
use girsa_corpus::work::{Source, Work};

fn work(slug: &str) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: slug.to_string(),
        en_title: slug.to_string(),
        categories: vec!["Talmud".into()],
        order: Vec::new(),
        source: Source::Otzaria,
        origin: std::path::PathBuf::new(),
        schema: None,
        author: None,
        era: None,
        comp_date: None,
        version: None,
        he_sections: Vec::new(),
        commentary_on: Vec::new(),
    }
}

fn raw(path: &[&str], text: &str) -> RawSegment {
    RawSegment {
        path: path.iter().map(|p| (*p).to_string()).collect(),
        kind: SegmentKind::Text,
        text: text.to_string(),
    }
}

/// A segment the size of the largest one in the real corpus.
fn a_volume() -> String {
    // Real Hebrew with real punctuation, so the cuts land where a sentence ends.
    let sentence = "מאימתי קורין את שמע בערבית משעה שהכהנים נכנסין לאכול בתרומתן: ";
    let mut text = String::new();
    while text.chars().count() < 1_275_307 {
        text.push_str(sentence);
    }
    text
}

#[test]
fn a_segment_too_long_to_be_a_place_is_cut_into_places() {
    let text = a_volume();
    let characters = text.chars().count();
    assert!(characters > 1_275_000, "the fixture is the real size");

    let imported = ImportedWork::assemble(
        work("bavli/chiddushei-harambam-on-rosh-hashanah"),
        vec![raw(&["20b", "7"], &text)],
    );

    // One segment in, many out — and every one of them is a place.
    assert!(
        imported.segments.len() > 100,
        "{} segments",
        imported.segments.len()
    );
    for segment in &imported.segments {
        assert!(
            segment.text.chars().count() <= NAMES_A_PLACE,
            "{} is still {} characters",
            segment.id,
            segment.text.chars().count()
        );
    }

    // Not one word is lost and not one is repeated. This is the assertion that
    // matters most: a corpus quietly a sentence short is the failure this whole
    // design is arranged against.
    let rejoined: String = imported.segments.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(rejoined, text);

    // And it is reported, rather than being a thing that silently happened.
    assert_eq!(imported.oversized.over, 1);
    assert_eq!(imported.oversized.a_volume, 1);
    assert_eq!(imported.oversized.largest, characters);
    assert_eq!(imported.oversized.split, 1);
    assert_eq!(imported.oversized.children, imported.segments.len());
    let said = imported.oversized.said();
    assert!(
        said.iter().any(|l| l.contains("split")),
        "the tally says what it cut: {said:?}"
    );
}

#[test]
fn every_anchor_on_the_parent_still_names_the_same_words() {
    // The promise. A Ksav document, a link, a correction or a mark written before
    // the split named `#1`; after it, `#1` is not a record any more. It must still
    // name exactly the words it named.
    let text = a_volume();
    let imported = ImportedWork::assemble(
        work("bavli/chiddushei-harambam-on-rosh-hashanah"),
        vec![raw(&["20b", "7"], &text)],
    );

    // What the anchor said, before the split: the first segment of the work.
    let parent = imported.segments[0]
        .id
        .to_string()
        .split('#')
        .next()
        .map(|head| format!("{head}#1"))
        .expect("an id");
    let parent: girsa_corpus::segment::SegmentId = parent.parse().expect("it reads back");

    let covered: Vec<&girsa_corpus::import::Segment> = imported
        .segments
        .iter()
        .filter(|s| parent.covers(&s.id))
        .collect();
    assert_eq!(
        covered.len(),
        imported.segments.len(),
        "the parent covers every child"
    );
    let words: String = covered.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(words, text, "and covering them is naming the same words");

    // The parent is not itself on disk. It is not a segment any more, and writing
    // it as one would put the same 1.2 MB in the corpus twice.
    assert!(!imported.segments.iter().any(|s| s.id == parent));
}

#[test]
fn a_segment_that_is_already_a_place_is_untouched() {
    // The overwhelming majority: 4,994,812 of 5,000,545. A change that renamed any
    // of them would break every anchor in the corpus for no gain at all.
    let one = "מאימתי קורין את שמע בערבית";
    let imported = ImportedWork::assemble(work("bavli/berakhot"), vec![raw(&["2a", "1"], one)]);
    assert_eq!(imported.segments.len(), 1);
    assert_eq!(imported.segments[0].text, one);
    assert_eq!(
        imported.segments[0].id.to_string(),
        "girsa:bavli/berakhot/2a:1#1"
    );
    assert!(imported.oversized.is_empty());
    assert!(imported.oversized.said().is_empty());
}

#[test]
fn reading_order_survives_the_cut() {
    // Three segments, the middle one oversized. The children have to sort between
    // their neighbours, not after them — which is what an ordinal *child* buys and
    // a fresh ordinal would not.
    let long = a_volume();
    let imported = ImportedWork::assemble(
        work("bavli/berakhot"),
        vec![
            raw(&["2a", "1"], "ראשון"),
            raw(&["2a", "2"], &long),
            raw(&["2a", "3"], "אחרון"),
        ],
    );
    let texts: Vec<&str> = imported.segments.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts.first().copied(), Some("ראשון"));
    assert_eq!(texts.last().copied(), Some("אחרון"));

    // Ids ascend, which is what reading order *is* for a `SegmentId`.
    let ids: Vec<_> = imported.segments.iter().map(|s| s.id.clone()).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "the ids are already in reading order");

    // And every id survives being written down and read back — the property
    // `Counts::malformed_ids` exists to hold.
    assert_eq!(imported.counts().malformed_ids, 0);
    for segment in &imported.segments {
        let text = segment.id.to_string();
        let back: girsa_corpus::segment::SegmentId = text.parse().expect("it reads back");
        assert_eq!(back, segment.id, "{text}");
    }
}

#[test]
fn the_threshold_and_the_tally_are_the_same_number() {
    // The audit counted at 10,000 characters. If the importer cut at a different
    // number, the report and the finding would be about two different things.
    assert_eq!(NAMES_A_PLACE, 10_000);
    const _: () = assert!(
        oversized::TARGET < NAMES_A_PLACE,
        "cutting to a target at or over the threshold would loop or do nothing"
    );
    let mut tally = oversized::Tally::default();
    tally.saw("girsa:x/1#1", NAMES_A_PLACE, "x");
    assert!(tally.is_empty(), "at the threshold is not over it");
    tally.saw("girsa:x/1#2", NAMES_A_PLACE + 1, "x");
    assert_eq!(tally.over, 1);
}
