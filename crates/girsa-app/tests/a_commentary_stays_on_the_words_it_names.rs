//! A commentary stays on the words it names — through the things that move them.
//!
//! # What this is
//!
//! Issue #43: bind every commentary to an **anchor ref plus a range**, so that a
//! re-import which moves the base text re-anchors the commentary with it. The
//! primitives for that already exist and are tested separately — `Standing` for
//! *which place*, `girsa_corpus::span::locate` for *which words* — and every other
//! anchored thing in this tree is written down as both an id and the words it
//! named. **A commentary's attachment was the one that was not.**
//!
//! Three ways it was not, all of them silent, all of them the same disease as
//! `span.rs`'s own header — *"an offset is not a place"*:
//!
//! 1. **The mined anchor's offset was measured against the text as imported, and
//!    applied to the text as corrected.** `girsa_corpus::anchors` mines
//!    `<i data-commentator="…"></i>` out of the raw sefer text at ingest and
//!    records a character offset into it. The pane draws `Open::corrected_by`'s
//!    text. So one correction anywhere above an anchor inside a segment shifts
//!    every commentary highlight in that segment — and Shulchan Arukh Orach
//!    Chayim has **3,850 of 4,171** segments with an anchor, so this is the
//!    common case rather than a corner. No words are recorded, so nothing can
//!    find them again.
//! 2. **A pin stored offsets with no words,** so when the line moved under it —
//!    a correction above it, an upstream re-typeset — the stored numbers still
//!    resolved, to the wrong letters. `girsa_note::mark` and `girsa_fix::Patch`
//!    both carry `was` for exactly this. The pin, which is the one a reader
//!    vouched for, was the one that could not follow.
//! 3. **A pin resolved its segment by `SegmentId::covers`,** the prefix test
//!    `girsa_corpus::standing` exists to replace. `#7.1` is what a cut of `#7`
//!    mints *and* what upstream inserting a se'if after `#7` is named, so a
//!    prefix test cannot tell a parent's words from a neighbour's. **That one is
//!    not demonstrated here**, and saying so is the point: an inserted se'if is
//!    not reachable by the link either, so the two predicates happened to agree
//!    on every input the panel can produce. It is fixed as *the right
//!    predicate*, not as a witnessed leak — see
//!    `a_pin_on_a_seif_this_importer_cut_up_still_lands_on_every_piece` for the
//!    half that is reachable, and `girsa_corpus::span` for the disagreement
//!    itself.
//!
//! Each test below is the assertion that goes red first, and each says which
//! line of the mechanism it is about.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::session::Pointing;
use girsa_app::shelf::Shelf;
use girsa_corpus::import::{ImportedWork, Previous, RawSegment, SegmentKind};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::{Source, Work};
use girsa_link::repair::Repairs;
use girsa_link::{Anchor, Edge, EdgeType, Method};

const SEFER: &str = "shulchan-arukh/orach-chayim";
const MEFARESH: &str = "mishnah-berurah";

/// The name the corpus spells the anchor's commentator with, and so the name
/// `spans::anchor_span` matches the shelf's `en_title` against.
const COMMENTATOR: &str = "Mishnah Berurah";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-anchor-range-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn a_work(slug: &str) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: slug.to_string(),
        // The mefaresh's English title is what the anchor is matched against,
        // so it has to be the name the corpus writes inside the `<i …>`.
        en_title: if slug == MEFARESH {
            COMMENTATOR.to_string()
        } else {
            slug.to_string()
        },
        categories: vec!["Halakhah".into()],
        order: Vec::new(),
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

/// The commentary's end of every link here.
///
/// At an ordinal nothing else in this file uses, because the gate in
/// `girsa_link::store` searches the whole row.
fn far_end() -> SegmentId {
    SegmentId::new(
        MEFARESH,
        vec!["1".into(), "500".into()],
        girsa_corpus::segment::Ordinal::root(500),
    )
}

/// One outgoing edge, in the shard of the sefer it points from.
fn link_from(root: &Path, from: &SegmentId) -> Edge {
    let edge = Edge {
        from: Anchor::point(from.clone()),
        to: Anchor::point(far_end()),
        edge_type: EdgeType::CommentsOn,
        method: Method::SefariaSeed,
        direction: girsa_link::Direction::NotRecorded,
        source_label: "commentary".into(),
    };
    let mut writer = girsa_link::store::Writer::default();
    writer.push(&edge);
    writer.flush(root).expect("the shard writes");
    edge
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

/// What the links panel would draw on this line: the text it drew, and the
/// words it would put the commentary on.
///
/// Every input is the one `app/src-tauri/src/lib.rs` hands `span_on`, and two of
/// them matter: `base` is the text **the pane drew**, which is the corrected
/// text, and `printed` is the same segment **as it is on disk**. The mined
/// anchors' offsets are positions in the second one, so a fixture that passed
/// only the first is the fixture this file exists to say is wrong.
fn panel_on(shelf: &Shelf, at: &SegmentId) -> (String, Option<std::ops::Range<usize>>) {
    let sefer = shelf.read(at.work()).expect("the sefer opens");
    let nth = sefer.position_of(at).expect("the se'if is here");
    let segment = sefer.segments.get(nth).expect("a segment");
    let standing = sefer.standing(at);
    let touching = girsa_app::touching(shelf, shelf.repairs(), &standing);
    let mefaresh = touching
        .links
        .iter()
        .find(|link| link.work == MEFARESH)
        .cloned();
    let span = mefaresh.as_ref().and_then(|link| {
        girsa_app::links::span_on(
            link,
            &standing,
            &segment.text,
            sefer.as_printed(at),
            &segment.anchors,
            None,
            Pointing::Full,
        )
    });
    (segment.text.clone(), span)
}

/// The letters a span covers of `text`.
fn letters(text: &str, span: &std::ops::Range<usize>) -> String {
    text.chars().skip(span.start).take(span.len()).collect()
}

// ---------------------------------------------------------------------------

/// Sefaria's own line, verbatim: an empty `<i data-commentator=…>` sitting
/// between two words of the Shulchan Arukh, which is where the commentary
/// attaches. `anchors.rs` takes it out of the text and keeps its place.
fn sa_with_a_mined_anchor() -> String {
    format!("יתגבר <i data-commentator=\"{COMMENTATOR}\" data-order=\"1\"></i>כארי לעמוד בבוקר")
}

#[test]
fn a_correction_above_a_mined_anchor_does_not_move_the_commentary_off_its_words() {
    // The anchor says: the Mishnah Berurah is on `כארי לעמוד בבוקר`.
    //
    // That offset is a position in the text **as it was imported**. The pane
    // draws the text as the reader has corrected it, and one correction above
    // the anchor makes every character after it sit one place further on.
    // Nothing records the words, so nothing can look for them again, and the
    // highlight lands on the wrong letters — which looks exactly like a
    // highlight on the right ones.
    let root = scratch("correction-above-an-anchor");
    let personal = root.join("personal");
    catalogue(&root);

    let imported = import(&root, vec![seif(1, &sa_with_a_mined_anchor())]);
    let at = id_saying(&imported, "יתגבר כארי לעמוד בבוקר");
    link_from(&root, &at);

    // The mined anchor is really there, and it is really at character 6.
    let mined = imported
        .segments
        .iter()
        .find(|s| s.id == at)
        .map(|s| s.anchors.clone())
        .unwrap_or_default();
    assert_eq!(mined.len(), 1, "the fixture carries one anchor");
    assert_eq!(mined[0].at, 6);

    // Now the reader fixes a word above it. `יתגבר` becomes `וְתִגְבֵּר`, which
    // is longer — so everything below it moves, and the anchor's 6 is wrong.
    let mut layer = girsa_fix::Layer::open(&personal).0;
    layer
        .add(girsa_fix::Patch::new(
            at.clone(),
            0.."יתגבר".chars().count(),
            "יתגבר",
            "וְתִגְבֵּר",
            girsa_fix::Kind::Ocr,
            "me",
        ))
        .expect("the correction is written");
    drop(layer);

    let shelf = Shelf::open(&root, &personal).expect("the shelf opens");
    let (base, span) = panel_on(&shelf, &at);
    assert!(
        base.starts_with("וְתִגְבֵּר"),
        "the correction did not apply: {base}"
    );

    let span = span.expect(
        "the volume itself says where this commentary attaches, and it does not \
         need the far sefer open to say it",
    );
    assert_eq!(
        letters(&base, &span),
        "כארי לעמוד בבוקר",
        "the commentary is highlighted on {} — the anchor's offset was measured \
         against the text as imported and applied to the text as corrected",
        letters(&base, &span),
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The words the pin below names. **Unique in the whole se'if**, which is the
/// fixture's job and not a detail: `span::locate` refuses a word that is there
/// twice, so pinning `מאימתי` in a line of repeated `מאימתי קורין…` would be a
/// test of the refusal rather than of the placement.
const ONCE: &str = "פתח דלתו של מקום טוב";

#[test]
fn a_pin_on_a_seif_this_importer_cut_up_lands_on_the_piece_that_holds_its_words() {
    // The reachable half of the predicate, and why `span_on` asks `Standing`
    // rather than the ordinal prefix it asked before.
    //
    // A pin is written down under the name the place had when the reader made
    // it, and a **cut** takes the parent off the shelf: `#2` becomes `#2.1`,
    // `#2.2`, … and is no longer a segment. So the pin is on a name that is not
    // on the shelf, and the question is whether it still names these words — for
    // the piece that has them, and for no other.
    //
    // The disagreement the rewrite is about is the other direction, and it is
    // **not reachable through the panel**: `#7.1` is what a cut of `#7` mints
    // *and* what upstream inserting a se'if after `#7` is named, so a prefix test
    // says yes to a se'if that never held those words. But an inserted se'if is
    // not reachable by the link either — the edge is stored under `#7` and
    // `Anchor::names` asks `Standing`, which is why
    // `an_edge_does_not_leak_onto_a_seif_inserted_beside_it` says the link is
    // absent there rather than misplaced. So `SegmentId::covers` and
    // `Standing::named_by` agreed on every input the panel can produce, and the
    // honest account of this change is **the right predicate, not a witnessed
    // leak**: `covers` was right by a property of `Standing`'s walk that nothing
    // states, and the walk is free to change. `girsa_corpus::span` holds the
    // disagreement itself, in `a_mark_belonging_to_another_seif_places_nothing`.
    let root = scratch("pin-and-a-cut");
    let personal = root.join("personal");
    catalogue(&root);

    // A se'if too long to be a place, so the cutter takes it up: one phrase that
    // occurs once, then the same sentence over and over.
    let sentence = "מאימתי קורין את שמע בערבית משעה שהכהנים נכנסין לאכול בתרומתן: ";
    let mut long = ONCE.to_string();
    while long.chars().count() < 60_000 {
        long.push_str(sentence);
    }
    let imported = import(&root, vec![seif(1, "יתגבר כארי"), seif(2, &long)]);
    assert!(imported.oversized.split > 0, "the fixture was actually cut");
    let parent = imported
        .redirects
        .iter()
        .find(|row| row.why == girsa_corpus::import::Why::Cut)
        .expect("the cut was recorded")
        .clone();
    assert!(
        !imported.segments.iter().any(|s| s.id == parent.from),
        "and the parent is not a segment any more, which is what makes it a cut"
    );
    assert!(
        parent.to.len() > 2,
        "and it made more than two pieces, so `no other piece` is a real claim"
    );

    // The reader is standing on it when they pin, so the pin is on the parent.
    let edge = link_from(&root, &parent.from);
    let mut repairs = Repairs::open(&personal).0;
    repairs
        .pin_named(
            &girsa_link::repair::name_of(&edge),
            &girsa_corpus::span::Anchored::new(parent.from.clone(), 0..ONCE.chars().count(), ONCE),
            "me",
        )
        .expect("the pin is written");
    drop(repairs);

    let shelf = Shelf::open(&root, &personal).expect("the shelf opens");
    for piece in &parent.to {
        let (text, span) = panel_on(&shelf, piece);
        if text.starts_with(ONCE) {
            assert_eq!(
                span,
                Some(0..ONCE.chars().count()),
                "a pin on a parent that was cut up does not reach {piece}, which is \
                 the piece holding its words"
            );
        } else {
            let drawn = span
                .as_ref()
                .map(|s| letters(&text, s))
                .unwrap_or_else(|| "<nothing>".into());
            assert_eq!(
                span, None,
                "the pin is drawn on {drawn} of {piece}, which does not hold those \
                 words"
            );
        }
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_pin_finds_again_the_words_it_named_after_the_line_moved_under_it() {
    // The other half of "a pin is a place, not an offset".
    //
    // `Repairs::Pinned` stores `at`, `from_char` and `to_char`. A correction
    // above them — one word, three characters longer — and those numbers still
    // resolve: to three letters that are no longer the ones they named.
    // `girsa_note::mark` and `girsa_fix::Patch` both carry the words as well as
    // the offsets for exactly this, and both go through
    // `girsa_corpus::span::locate`. The pin, the only placement in this panel a
    // reader vouched for, could not follow.
    //
    // The record is written by hand rather than through `pin_named`, because the
    // field under test is one the repair API does not take yet: serde drops an
    // unknown key, so today this reads as a pin with no words and lands on the
    // wrong letters.
    let root = scratch("pin-words");
    let personal = root.join("personal");
    catalogue(&root);

    let first = import(&root, vec![seif(1, "יתגבר כארי לעמוד")]);
    let at = id_saying(&first, "יתגבר כארי לעמוד");
    let edge = link_from(&root, &at);

    std::fs::create_dir_all(&personal).expect("a personal layer");
    let record = serde_json::json!({
        "edge": girsa_link::repair::name_of(&edge),
        "does": "pinned",
        "at": at.to_string(),
        "from_char": 6,
        "to_char": 10,
        "was": "כארי",
        "who": "me",
        "when": 1u64,
    });
    std::fs::write(
        girsa_link::repair::path_in(&personal),
        format!("{record}\n"),
    )
    .expect("the pin is written");

    // And the reader fixes the word above it — one character longer.
    let mut layer = girsa_fix::Layer::open(&personal).0;
    layer
        .add(girsa_fix::Patch::new(
            at.clone(),
            0.."יתגבר".chars().count(),
            "יתגבר",
            "ויתגבר",
            girsa_fix::Kind::Ocr,
            "me",
        ))
        .expect("the correction is written");
    drop(layer);

    let shelf = Shelf::open(&root, &personal).expect("the shelf opens");
    let (base, span) = panel_on(&shelf, &at);
    assert_eq!(
        base, "ויתגבר כארי לעמוד",
        "the correction did not apply: {base}"
    );

    let span = span.expect("the reader pinned something");
    assert_eq!(
        letters(&base, &span),
        "כארי",
        "a pin on `כארי` is drawn on `{}` after a correction above it moved \
         every character on",
        letters(&base, &span),
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_pin_whose_words_are_gone_places_nothing_rather_than_the_wrong_letters() {
    // Rule 6, in the one place a reader would never check. `span::locate`'s
    // own test says it about a highlight; a pin is a highlight somebody made
    // deliberately, so it is at least as bad here — and the failure is
    // invisible, because a highlight on the wrong three letters looks exactly
    // like a highlight on the right three.
    let root = scratch("pin-gone");
    let personal = root.join("personal");
    catalogue(&root);

    let first = import(&root, vec![seif(1, "יתגבר כארי לעמוד")]);
    let at = id_saying(&first, "יתגבר כארי לעמוד");
    let edge = link_from(&root, &at);

    std::fs::create_dir_all(&personal).expect("a personal layer");
    let record = serde_json::json!({
        "edge": girsa_link::repair::name_of(&edge),
        "does": "pinned",
        "at": at.to_string(),
        "from_char": 6,
        "to_char": 10,
        "was": "בראשית",
        "who": "me",
        "when": 1u64,
    });
    std::fs::write(
        girsa_link::repair::path_in(&personal),
        format!("{record}\n"),
    )
    .expect("the pin is written");

    let shelf = Shelf::open(&root, &personal).expect("the shelf opens");
    let (base, span) = panel_on(&shelf, &at);
    let drawn = span
        .as_ref()
        .map(|s| letters(&base, s))
        .unwrap_or_else(|| "<nothing>".into());
    assert_eq!(
        span, None,
        "a pin naming words this line does not contain is drawn on {drawn} of {base}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
