//! The repair UI's acceptance, on the real graph (spec.md §8.3, W23).
//!
//! W8's acceptance was *Mishnah Berakhot segment 3 → Rambam segment 5, correct
//! text*. This is the same fact asked the way a reader asks it: **standing on
//! the first mishnah of Berakhot, what is linked to this line** — and then the
//! four things §8.3 says you may do about the answer.
//!
//! # Why it skips when the corpus is absent
//!
//! It needs the fetched corpus, imported, with links imported over it — not
//! committed, and not present on a fresh clone. A test that failed there would
//! be noise everybody learns to ignore.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::shelf::Shelf;
use girsa_corpus::segment::SegmentId;
use girsa_link::repair::Verdict;
use girsa_link::EdgeType;

const MISHNAH: &str = "mishnah-berakhot";
const RAMBAM: &str = "rambam-on-mishnah-berakhot";

fn corpus() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    root.join("links").is_dir().then_some(root)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-links-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

macro_rules! corpus_or_skip {
    () => {
        match corpus() {
            Some(root) => root,
            None => {
                eprintln!("skipped: no imported link graph — run girsa-link-import first");
                return;
            }
        }
    };
}

/// The first mishnah of Berakhot, by its address rather than by its ordinal:
/// the ordinal is permanent but this test should say which *place* it means.
fn first_mishnah(shelf: &Shelf) -> SegmentId {
    let sefer = shelf.read(MISHNAH).expect("Berakhot is on the shelf");
    sefer
        .segments
        .iter()
        .find(|s| {
            s.id.path().first().map(String::as_str) == Some("1")
                && s.id.path().get(1).map(String::as_str) == Some("1")
        })
        .map(|s| s.id.clone())
        .expect("Mishnah Berakhot 1:1 is on the shelf")
}

#[test]
fn standing_on_the_first_mishnah_of_berakhot_shows_the_rambam_on_it() {
    let root = corpus_or_skip!();
    let mut shelf = Shelf::open(&root, &scratch("panel")).expect("the shelf opens");
    let at = first_mishnah(&shelf);

    let began = std::time::Instant::now();
    let touching = girsa_app::touching(&shelf, shelf.repairs(), &at);
    let took = began.elapsed();
    println!(
        "{} links on {at}, in {} ms",
        touching.links.len(),
        took.as_millis()
    );
    assert!(
        !touching.links.is_empty(),
        "the first mishnah of Berakhot is linked to something"
    );
    // Not a benchmark — a tripwire, and deliberately a loose one. This reads
    // every companion's shard, and the number printed above is a **debug**
    // build sharing a disk with the other test in this file; on its own it is
    // 2.5s, and the window builds in release. What the bound is here to catch
    // is somebody making this read the whole four-million-edge graph, which is
    // not four times slower, it is minutes.
    assert!(
        took < std::time::Duration::from_secs(30),
        "the links panel took {took:?} to open, on one of the most-linked lines in Shas"
    );
    let rambam = touching
        .links
        .iter()
        .find(|link| link.work == RAMBAM)
        .expect("the Rambam on this mishnah is one of them");
    println!(
        "{} · {} · {} · {:.0}%",
        rambam.said(),
        rambam.repaired.edge.edge_type.as_str(),
        rambam.repaired.edge.method.as_str(),
        rambam.repaired.confidence() * 100.0
    );

    // Now the four actions of §8.3, on a real edge, with the shipped graph
    // watched for changes throughout.
    let shard = girsa_link::store::edges_path(&root, at.work());
    let before = std::fs::read(&shard).expect("the shard reads");
    let name = girsa_link::repair::name_of(&rambam.repaired.edge);

    let who = "the test";
    shelf
        .repairs_mut()
        .retype_named(&name, EdgeType::CommentsOn, who)
        .expect("retypes");
    shelf
        .repairs_mut()
        .judge_named(&name, Verdict::Confirmed, who)
        .expect("confirms");

    // Found by **name** from here on, not by "the first Rambam row": there are
    // several links between this mishnah and the Rambam on it, and the list is
    // sorted by confidence — so confirming one moves it to the top and
    // rejecting it moves it to the bottom. Following the row rather than the
    // edge is how a repair UI ends up confirming one link and rejecting
    // another while the reader thinks they did both to one.
    let named = |touching: &girsa_app::Touching| {
        touching
            .links
            .iter()
            .find(|link| {
                girsa_link::repair::name_of(
                    link.repaired
                        .shipped
                        .as_ref()
                        .unwrap_or(&link.repaired.edge),
                ) == name
            })
            .cloned()
            .expect("the edge I repaired is still in the list")
    };

    let touching = girsa_app::touching(&shelf, shelf.repairs(), &at);
    let rambam = named(&touching);
    assert_eq!(rambam.repaired.edge.edge_type, EdgeType::CommentsOn);
    assert!(rambam.repaired.confirmed);
    assert!(rambam.repaired.is_curated(), "somebody looked at it");
    assert_eq!(rambam.repaired.who.as_deref(), Some(who));
    assert!(
        rambam.repaired.shipped.is_some(),
        "and it can still say what it was"
    );

    assert_eq!(
        std::fs::read(&shard).expect("the shard reads"),
        before,
        "a repair may not write one byte into the shipped graph"
    );

    // Rejecting it takes it out of what anything draws; undoing puts the
    // shipped edge back exactly as it came.
    shelf
        .repairs_mut()
        .judge_named(&name, Verdict::Rejected, who)
        .expect("rejects");
    let touching = girsa_app::touching(&shelf, shelf.repairs(), &at);
    let rambam = named(&touching);
    assert!(rambam.repaired.rejected);
    assert!(!rambam.repaired.is_curated());

    shelf.repairs_mut().undo_named(&name).expect("undoes");
    let touching = girsa_app::touching(&shelf, shelf.repairs(), &at);
    let rambam = named(&touching);
    assert!(rambam.repaired.changed.is_empty());
    assert!(rambam.repaired.shipped.is_none());
}

#[test]
fn a_link_you_draw_shows_up_beside_the_shipped_ones() {
    let root = corpus_or_skip!();
    let mut shelf = Shelf::open(&root, &scratch("drawn")).expect("the shelf opens");
    let at = first_mishnah(&shelf);
    let elsewhere = shelf
        .read(RAMBAM)
        .expect("the Rambam is on the shelf")
        .segments
        .first()
        .map(|s| s.id.clone())
        .expect("it has segments");

    let before = girsa_app::touching(&shelf, shelf.repairs(), &at)
        .links
        .len();
    shelf
        .repairs_mut()
        .draw(
            girsa_link::Anchor::point(at.clone()),
            girsa_link::Anchor::point(elsewhere),
            EdgeType::Codifies,
            "the test",
        )
        .expect("draws");

    let touching = girsa_app::touching(&shelf, shelf.repairs(), &at);
    assert_eq!(touching.links.len(), before + 1);
    let mine = touching
        .links
        .iter()
        .find(|link| link.repaired.mine)
        .expect("the one I drew");
    assert_eq!(mine.repaired.edge.method, girsa_link::Method::ByHand);
    assert_eq!(mine.repaired.confidence(), 1.0);
    assert!(mine.repaired.is_curated(), "you said it, so it is a claim");
}
