//! What the links panel does when the corpus moves under it.
//!
//! The last commit gave the reading pane a redirect table and said out loud that
//! the link graph did not use it. This is that half, and it is two failures
//! rather than one — they point opposite ways and only one of them was named:
//!
//! | upstream did | stored anchor | what a prefix test said | the truth |
//! |---|---|---|---|
//! | folded se'if 3 into se'if 2 | `#3` | nothing here | it is se'if 2's words now |
//! | inserted a se'if after 1 | `#1` | **this is your line** | it has never seen those words |
//!
//! The second is the dangerous one, and it is the one nobody wrote down. A
//! missing link is a reader noticing the panel looks thin. An invented link is
//! Girsa asserting a connection that nobody made, on a se'if that did not exist
//! when the comment was written — BUILDER.md rule 6 with the sign flipped.
//!
//! Both come from `Ordinal::child` having two callers that mean opposite things:
//! the oversized cutter carving `#1` up, and `mint_between` naming a se'if
//! inserted after `#1`. Both mint `#1.1`. What tells them apart is that **a cut
//! deletes its parent** — so this file re-imports a real work over itself and
//! asks the panel, rather than asserting the rule against a fixture that agrees
//! with it by construction.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::shelf::Shelf;
use girsa_corpus::import::{ImportedWork, Previous, RawSegment, SegmentKind, Why};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::{Source, Work};
use girsa_link::{Anchor, Edge, EdgeType, Method};

const SEFER: &str = "shulchan-arukh/orach-chayim";
const MEFARESH: &str = "mishnah-berurah";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-resegment-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn a_work(slug: &str) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: slug.to_string(),
        en_title: slug.to_string(),
        categories: vec!["Halakhah".into()],
        source: Source::Sefaria,
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

fn seif(n: usize, text: &str) -> RawSegment {
    RawSegment {
        path: vec!["1".into(), n.to_string()],
        kind: SegmentKind::Text,
        text: text.to_string(),
    }
}

/// Write the catalogue both works appear in, so the panel can name the far end.
fn catalogue(root: &Path) {
    std::fs::create_dir_all(root.join("works")).expect("a works dir");
    let body: String = [SEFER, MEFARESH]
        .iter()
        .map(|slug| {
            let line = serde_json::to_string(&a_work(slug)).expect("serializes");
            format!("{line}\n")
        })
        .collect();
    std::fs::write(root.join("works/index.jsonl"), body).expect("a catalogue");
}

/// Import the sefer, over whatever the last run left.
fn import(root: &Path, seifim: Vec<RawSegment>) -> ImportedWork {
    let previous = Previous::on_the_shelf(root, SEFER);
    let imported = ImportedWork::assemble_after(a_work(SEFER), seifim, &previous);
    girsa_corpus::import::write(root, &imported).expect("the sefer writes");
    imported
}

/// One outgoing edge, in the shard of the sefer it points from — which is where
/// the panel reads the outgoing half from (spec.md §8.2).
fn link_from(root: &Path, from: &SegmentId) {
    let mut writer = girsa_link::store::Writer::default();
    writer.push(&Edge {
        from: Anchor::point(from.clone()),
        to: Anchor::point(far_end()),
        edge_type: EdgeType::CommentsOn,
        method: Method::SefariaSeed,
        direction: girsa_link::Direction::NotRecorded,
        source_label: "commentary".into(),
    });
    writer.flush(root).expect("the shard writes");
}

/// The commentary's end of every link here.
///
/// At an ordinal nothing else in this file uses, because the gate in
/// `girsa_link::store` searches the whole row: a mefaresh sitting at `#1` would
/// admit its rows whenever a test stood on se'if 1, and the reanchor test below
/// would pass without the thing it is testing.
fn far_end() -> SegmentId {
    SegmentId::new(
        MEFARESH,
        vec!["1".into(), "500".into()],
        girsa_corpus::segment::Ordinal::root(500),
    )
}

/// The id of the se'if whose text is this, as the shelf has it now.
fn id_saying(imported: &ImportedWork, text: &str) -> SegmentId {
    imported
        .segments
        .iter()
        .find(|s| s.text == text)
        .map(|s| s.id.clone())
        .unwrap_or_else(|| panic!("no segment says {text}"))
}

/// What touches a place, asked the way the window asks it.
fn links_on(shelf: &Shelf, at: &SegmentId) -> usize {
    let sefer = shelf.read(SEFER).expect("the sefer opens");
    let standing = sefer.standing(at);
    girsa_app::touching(shelf, shelf.repairs(), &standing)
        .links
        .iter()
        .filter(|link| link.work == MEFARESH)
        .count()
}

#[test]
fn an_edge_onto_a_seif_upstream_folded_away_still_reaches_the_reader() {
    let root = scratch("merged");
    catalogue(&root);

    // Three se'ifim, and the Mishnah Berurah comments on the third.
    let first = import(&root, vec![seif(1, "אלף"), seif(2, "בית"), seif(3, "גימל")]);
    let third = id_saying(&first, "גימל");
    link_from(&root, &third);

    // Upstream folds se'if 3 into se'if 2. Nothing renumbers; the importer
    // records where the words went.
    let after = import(&root, vec![seif(1, "אלף"), seif(2, "בית גימל")]);
    let row = after
        .redirects
        .iter()
        .find(|row| row.from == third)
        .expect("the importer recorded where se'if 3 went");
    assert_eq!(row.why, Why::Resegmented);
    let merged = id_saying(&after, "בית גימל");
    assert_eq!(row.to, vec![merged.clone()]);

    // The trap this exists for: the old name is not an ancestor of the new one,
    // and never will be. Nothing about the *names* can find this.
    assert!(
        !Anchor::point(third.clone()).covers(&merged),
        "descent cannot express a merge, which is the whole reason for the table"
    );

    let shelf = Shelf::open(&root, &scratch("merged-personal")).expect("the shelf opens");
    assert_eq!(
        links_on(&shelf, &merged),
        1,
        "the comment on se'if 3 is a comment on the words, wherever upstream put them"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_edge_does_not_leak_onto_a_seif_upstream_inserted_beside_it() {
    let root = scratch("inserted");
    catalogue(&root);

    // Two se'ifim, and the Mishnah Berurah comments on the first.
    let first = import(&root, vec![seif(1, "אלף"), seif(2, "בית")]);
    let one = id_saying(&first, "אלף");
    link_from(&root, &one);

    // Upstream inserts a se'if between them. `mint_between` has to name it
    // something that sorts between `#1` and `#2`, and the only such name is a
    // child of `#1`.
    let after = import(&root, vec![seif(1, "אלף"), seif(2, "חדש"), seif(3, "בית")]);
    let inserted = id_saying(&after, "חדש");
    assert_eq!(after.continuity.minted, 1, "one name was minted");
    assert!(
        one.covers(&inserted),
        "and it is spelled like a piece of se'if 1 — {inserted} — which is the trap"
    );
    assert!(
        !after.redirects.iter().any(|row| row.from == one),
        "se'if 1 kept its name and its words; nothing was redirected away from it"
    );

    let shelf = Shelf::open(&root, &scratch("inserted-personal")).expect("the shelf opens");
    assert_eq!(
        links_on(&shelf, &inserted),
        0,
        "the Mishnah Berurah has never seen this se'if — it did not exist when the comment was written"
    );
    assert_eq!(
        links_on(&shelf, &one),
        1,
        "and the comment is still on the se'if it was actually made about"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_edge_onto_a_seif_this_importer_cut_up_reaches_every_piece() {
    // The case that always worked, kept here so that fixing the two above cannot
    // quietly break it: a cut *does* hand its name to its pieces, because a cut
    // takes the parent off the shelf. Same dotted name, opposite answer.
    let root = scratch("cut");
    catalogue(&root);

    let sentence = "מאימתי קורין את שמע בערבית משעה שהכהנים נכנסין לאכול בתרומתן: ";
    let mut long = String::new();
    while long.chars().count() < 60_000 {
        long.push_str(sentence);
    }
    let imported = import(&root, vec![seif(1, "אלף"), seif(2, &long)]);
    assert!(imported.oversized.split > 0, "the fixture was actually cut");

    let parent = imported
        .redirects
        .iter()
        .find(|row| row.why == Why::Cut)
        .expect("the cut was recorded");
    link_from(&root, &parent.from);
    assert!(
        !imported.segments.iter().any(|s| s.id == parent.from),
        "and the parent is not a segment any more, which is what makes it a cut"
    );

    let shelf = Shelf::open(&root, &scratch("cut-personal")).expect("the shelf opens");
    for piece in &parent.to {
        assert_eq!(
            links_on(&shelf, piece),
            1,
            "an anchor on the parent names {piece}, because these are its words"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_link_you_moved_by_hand_is_not_lost_by_the_thing_that_skips_rows() {
    // The panel no longer builds an `Edge` out of every row in a shard — it
    // gates them as text first, because 63 of Orach Chayim's 159,273 inbound
    // rows are wanted and the other 159,210 cost three allocations each.
    //
    // `Repair::Reanchored` is what makes that dangerous. It puts an edge
    // somewhere its stored ends do not mention, so a gate reading only the
    // stored text would skip the row and the link would vanish — and it would
    // be a link the reader moved there themselves, which is the worst thing in
    // the corpus to lose silently.
    let root = scratch("moved");
    let personal = scratch("moved-personal");
    catalogue(&root);

    let imported = import(&root, vec![seif(1, "אלף"), seif(2, "בית"), seif(3, "גימל")]);
    let third = id_saying(&imported, "גימל");
    let first = id_saying(&imported, "אלף");
    link_from(&root, &third);

    let shipped = Edge {
        from: Anchor::point(third.clone()),
        to: Anchor::point(far_end()),
        edge_type: EdgeType::CommentsOn,
        method: Method::SefariaSeed,
        direction: girsa_link::Direction::NotRecorded,
        source_label: "commentary".into(),
    };

    // Nothing moved yet: the link is on se'if 3 and se'if 1 is not its business.
    let shelf = Shelf::open(&root, &personal).expect("the shelf opens");
    assert_eq!(links_on(&shelf, &third), 1);
    assert_eq!(links_on(&shelf, &first), 0);
    drop(shelf);

    // The reader says the corpus put it in the wrong place.
    let (mut repairs, trouble) = girsa_link::repair::Repairs::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    repairs
        .reanchor(
            &shipped,
            Anchor::point(first.clone()),
            Anchor::point(far_end()),
            "the test",
        )
        .expect("the layer takes it");

    let shelf = Shelf::open(&root, &personal).expect("the shelf reopens");
    assert_eq!(
        links_on(&shelf, &first),
        1,
        "the link is where the reader put it, and the gate let its row through"
    );
    assert_eq!(
        links_on(&shelf, &third),
        0,
        "and it is no longer where the corpus had it"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&personal);
}
