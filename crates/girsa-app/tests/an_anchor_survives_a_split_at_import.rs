//! A citation written before a segment was cut up still opens the same words.
//!
//! B12 cuts segments too long to name a place into places, at import
//! (`girsa_corpus::oversized`). 5,733 of the corpus's 5,000,545 segments are over
//! 10,000 characters and the largest is 1,275,307, so this is not a rare path — 926
//! works are affected, `tur` and `beit-yosef` among them.
//!
//! spec.md §3 is the promise being kept: *"Splitting a segment mints a child ID
//! rather than shifting seventeen thousand others."* `girsa-corpus` asserts that
//! the ordinals cover each other. **This asserts the half that a reader touches**:
//! that a `Shelf` handed the parent id — from a Ksav document, a link, a mark, a
//! correction — finds the children rather than nothing.
//!
//! Without it, cutting the oversized segments would silently orphan every anchor
//! that named one, which is exactly the failure mode the ordinal scheme exists to
//! prevent and would be a far worse defect than the one it fixes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::shelf::Shelf;
use girsa_corpus::import::{ImportedWork, RawSegment, SegmentKind};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::{Source, Work};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-split-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn a_work(slug: &str) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: slug.to_string(),
        en_title: slug.to_string(),
        categories: vec!["Talmud".into()],
        source: Source::Otzaria,
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

/// A sefer whose second segment is too long to be a place.
fn corpus_with_one_oversized_segment(root: &Path) -> ImportedWork {
    let sentence = "מאימתי קורין את שמע בערבית משעה שהכהנים נכנסין לאכול בתרומתן: ";
    let mut long = String::new();
    while long.chars().count() < 60_000 {
        long.push_str(sentence);
    }

    let imported = ImportedWork::assemble(
        a_work("bavli/berakhot"),
        vec![
            RawSegment {
                path: vec!["2a".into(), "1".into()],
                kind: SegmentKind::Text,
                text: "ראשון".into(),
            },
            RawSegment {
                path: vec!["2a".into(), "2".into()],
                kind: SegmentKind::Text,
                text: long,
            },
            RawSegment {
                path: vec!["2a".into(), "3".into()],
                kind: SegmentKind::Text,
                text: "אחרון".into(),
            },
        ],
    );
    std::fs::create_dir_all(root.join("works")).expect("a works dir");
    let line = serde_json::to_string(&imported.work).expect("serializes");
    std::fs::write(root.join("works/index.jsonl"), format!("{line}\n")).expect("a catalogue");
    girsa_corpus::import::write(root, &imported).expect("the sefer writes");
    imported
}

#[test]
fn a_citation_that_named_the_parent_opens_the_children() {
    let root = scratch("anchor");
    let imported = corpus_with_one_oversized_segment(&root);
    assert!(imported.oversized.split > 0, "the fixture was actually cut");

    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let open = shelf.read("bavli/berakhot").expect("it opens");

    // What a document written last year carries: the second segment of the work,
    // before anything split it.
    let parent: SegmentId = "girsa:bavli/berakhot/2a:2#2".parse().expect("an id");

    // It is not a record on disk any more.
    assert!(
        !open.segments.iter().any(|s| s.id == parent),
        "the parent is not written as a segment of its own"
    );

    // And it still names a place. This is the assertion that would have gone red:
    // `position_of` was an exact map lookup, so a citation to a segment that had
    // been cut up resolved to nothing at all.
    let at = open
        .position_of(&parent)
        .expect("the anchor still finds its words");
    assert_eq!(
        at, 1,
        "and finds them where they are — after ראשון, before אחרון"
    );

    // Every child, in reading order, and nothing else.
    let covered = open.covered_by(&parent);
    assert!(covered.len() > 5, "{} children", covered.len());
    let mut ascending = covered.clone();
    ascending.sort_unstable();
    assert_eq!(covered, ascending, "in reading order");
    for at in &covered {
        let id = &open.segments[*at].id;
        assert!(parent.covers(id), "{id} is not covered by {parent}");
    }

    // The words are the words. Rejoining the children gives back what the parent
    // named, which is the whole promise.
    let rejoined: String = covered
        .iter()
        .map(|at| open.segments[*at].text.as_str())
        .collect();
    // The fixture repeats a 61-character sentence until it passes 60,000, so the
    // exact length is whatever that came to — asserted against the segments rather
    // than against arithmetic, because the arithmetic is not the point.
    let whole: String = open
        .segments
        .iter()
        .filter(|s| parent.covers(&s.id))
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(rejoined, whole);
    assert!(rejoined.chars().count() >= 60_000);
}

#[test]
fn a_segment_nothing_split_is_still_an_exact_lookup() {
    // The overwhelming majority of lookups, and the one that must not get slower or
    // looser: an id that is on disk resolves to itself and to nothing else.
    let root = scratch("exact");
    corpus_with_one_oversized_segment(&root);
    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let open = shelf.read("bavli/berakhot").expect("it opens");

    let first: SegmentId = "girsa:bavli/berakhot/2a:1#1".parse().expect("an id");
    assert_eq!(open.position_of(&first), Some(0));
    assert_eq!(open.covered_by(&first), vec![0]);

    // And an id that names nothing here names nothing — never the nearest thing.
    let nowhere: SegmentId = "girsa:bavli/berakhot/99a:9#9999".parse().expect("an id");
    assert_eq!(open.position_of(&nowhere), None);
    assert!(open.covered_by(&nowhere).is_empty());
}

#[test]
fn an_address_that_named_the_parent_names_every_child() {
    // The other route in: not a segment id but a mareh makom. `ברכות ב:ב` used to
    // name one segment; it now names the group, which is the same words.
    let root = scratch("address");
    corpus_with_one_oversized_segment(&root);
    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let open = shelf.read("bavli/berakhot").expect("it opens");

    let address = girsa_ref::Address::parse("2a:2").expect("an address");
    let named = open.at(&address);
    assert!(
        named.len() > 5,
        "the address names all {} children, not one of them",
        named.len()
    );
    let parent: SegmentId = "girsa:bavli/berakhot/2a:2#2".parse().expect("an id");
    for id in &named {
        assert!(parent.covers(id), "{id}");
    }
}

// ---------------------------------------------------------------------------
// The other half of the same promise: upstream re-segmentation
// ---------------------------------------------------------------------------

/// A shelf holding one work, imported over whatever is already there.
fn shelve(root: &Path, lines: &[(&str, &str)]) -> ImportedWork {
    let raw = lines
        .iter()
        .map(|(address, text)| RawSegment {
            path: vec!["2a".into(), (*address).to_string()],
            kind: SegmentKind::Text,
            text: (*text).to_string(),
        })
        .collect();
    let previous = girsa_corpus::import::Previous::on_the_shelf(root, "bavli/berakhot");
    let imported = ImportedWork::assemble_after(a_work("bavli/berakhot"), raw, &previous);
    std::fs::create_dir_all(root.join("works")).expect("a works dir");
    let line = serde_json::to_string(&imported.work).expect("serializes");
    std::fs::write(root.join("works/index.jsonl"), format!("{line}\n")).expect("a catalogue");
    girsa_corpus::import::write(root, &imported).expect("the sefer writes");
    imported
}

#[test]
fn an_anchor_on_a_seif_upstream_merged_away_opens_the_words_that_absorbed_it() {
    // A cut is Girsa's own doing and `Ordinal::covers` handles it. This is the
    // case the corpus does *to* Girsa: Sefaria folds one se'if into the one
    // before it. Nothing about the ordinal can express that, which is what
    // `redirects.jsonl` is for — and until it existed, the anchor in a Ksav
    // document written last year resolved to nothing at all.
    let root = scratch("redirected");
    shelve(&root, &[("1", "ראשון"), ("2", "שני"), ("3", "שלישי")]);

    // What a document written before the corpus update carries.
    let anchor: SegmentId = "girsa:bavli/berakhot/2a:2#2".parse().expect("an id");

    let imported = shelve(&root, &[("1", "ראשון שני"), ("2", "שלישי")]);
    assert!(
        imported
            .redirects
            .iter()
            .any(|r| r.from == anchor && !r.to.is_empty()),
        "the merged se'if is redirected: {:?}",
        imported.redirects
    );

    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let open = shelf.read("bavli/berakhot").expect("it opens");
    assert!(
        !open.segments.iter().any(|s| s.id == anchor),
        "the id is not a record any more — that is the premise"
    );

    let at = open
        .position_of(&anchor)
        .expect("the anchor still finds its words");
    assert_eq!(
        open.segments[at].text, "ראשון שני",
        "and they are the words that absorbed it"
    );
}

#[test]
fn an_anchor_on_a_seif_upstream_deleted_finds_nothing_rather_than_the_nearest_thing() {
    // The failure this is all arranged against is not a broken link. It is a
    // link that resolves cleanly to somebody else's words. A place upstream no
    // longer has must come back empty.
    let root = scratch("deleted");
    shelve(&root, &[("1", "ראשון"), ("2", "שני"), ("3", "שלישי")]);
    let anchor: SegmentId = "girsa:bavli/berakhot/2a:2#2".parse().expect("an id");

    shelve(&root, &[("1", "ראשון"), ("2", "שלישי")]);

    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let open = shelf.read("bavli/berakhot").expect("it opens");
    assert_eq!(open.position_of(&anchor), None);
    assert!(open.covered_by(&anchor).is_empty());

    // And nothing on the shelf has taken over its name, which is the part that
    // would have been silent.
    assert!(
        open.segments
            .iter()
            .all(|s| s.id.ordinal() != anchor.ordinal()),
        "the name of a deleted se'if was handed to different words"
    );
}

#[test]
fn a_redirect_that_was_redirected_again_is_followed_the_whole_way() {
    // Two corpus updates in a row. The first sends `#2` at `#1`; the second
    // merges `#1` into what is left. An anchor from before either of them has
    // to follow both hops.
    let root = scratch("chained");
    shelve(&root, &[("1", "ראשון"), ("2", "שני"), ("3", "שלישי")]);
    let anchor: SegmentId = "girsa:bavli/berakhot/2a:2#2".parse().expect("an id");

    shelve(&root, &[("1", "ראשון שני"), ("2", "שלישי")]);
    shelve(&root, &[("1", "ראשון שני שלישי")]);

    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let open = shelf.read("bavli/berakhot").expect("it opens");
    let at = open
        .position_of(&anchor)
        .expect("two hops is still an answer");
    assert_eq!(open.segments[at].text, "ראשון שני שלישי");
}
