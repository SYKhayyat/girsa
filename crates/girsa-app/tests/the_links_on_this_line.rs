//! The repair UI's acceptance, on the real graph (spec.md §8.3, W23).
//!
//! W8's acceptance was *Mishnah Berakhot segment 3 → Rambam segment 5, correct
//! text*. This is the same fact asked the way a reader asks it: **standing on
//! the first mishnah of Berakhot, what is linked to this line** — and then the
//! four things §8.3 says you may do about the answer.
//!
//! # It used to skip, and a skip is why nobody noticed
//!
//! This gated on the fetched corpus and `return`ed when it was absent — so on
//! every fresh clone and in CI it printed `ok` in 0.00s having asserted nothing.
//! It runs on [`girsa_fixture`], a shelf the real importer builds from real
//! `merged.json` files in about a second, so the claim above is now checked
//! everywhere rather than nowhere.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::shelf::Shelf;
use girsa_corpus::segment::SegmentId;
use girsa_link::repair::Verdict;
use girsa_link::EdgeType;

/// The place, as the sefer on disk describes it: every name those words have
/// carried. What the window builds before asking what touches a line.
fn standing(shelf: &Shelf, at: &SegmentId) -> girsa_corpus::standing::Standing {
    shelf.read(at.work()).expect("the sefer opens").standing(at)
}

const MISHNAH: &str = "mishnah-berakhot";
const RAMBAM: &str = "rambam-on-mishnah-berakhot";

/// The shelf: works, segments and an imported link graph over them.
fn corpus() -> &'static Path {
    girsa_fixture::linked().root()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-links-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
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
    let root = corpus();
    let mut shelf = Shelf::open(root, &scratch("panel")).expect("the shelf opens");
    let at = first_mishnah(&shelf);

    let began = std::time::Instant::now();
    let touching = girsa_app::touching(&shelf, shelf.repairs(), &standing(&shelf, &at));
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
    let shard = girsa_link::store::edges_path(root, at.work());
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

    let touching = girsa_app::touching(&shelf, shelf.repairs(), &standing(&shelf, &at));
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
    let touching = girsa_app::touching(&shelf, shelf.repairs(), &standing(&shelf, &at));
    let rambam = named(&touching);
    assert!(rambam.repaired.rejected);
    assert!(!rambam.repaired.is_curated());

    shelf.repairs_mut().undo_named(&name).expect("undoes");
    let touching = girsa_app::touching(&shelf, shelf.repairs(), &standing(&shelf, &at));
    let rambam = named(&touching);
    assert!(rambam.repaired.changed.is_empty());
    assert!(rambam.repaired.shipped.is_none());
}

#[test]
fn a_link_you_draw_shows_up_beside_the_shipped_ones() {
    let root = corpus();
    let mut shelf = Shelf::open(root, &scratch("drawn")).expect("the shelf opens");
    let at = first_mishnah(&shelf);
    let elsewhere = shelf
        .read(RAMBAM)
        .expect("the Rambam is on the shelf")
        .segments
        .first()
        .map(|s| s.id.clone())
        .expect("it has segments");

    let before = girsa_app::touching(&shelf, shelf.repairs(), &standing(&shelf, &at))
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

    let touching = girsa_app::touching(&shelf, shelf.repairs(), &standing(&shelf, &at));
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

#[test]
fn rashi_says_which_words_he_is_on_and_they_are_found_in_the_gemara() {
    // W24 on the real corpus. Sefaria marks the dibur hamatchil in the text —
    // 43,890 of them in Berakhot alone — so *which words a link is about* is
    // readable rather than guessable, for the commentaries that declare one.
    //
    // Asked of the texts rather than through the link panel: Rashi is a
    // **declared** commentary (its address extends the Gemara's), so the pair
    // to compare is knowable without reading a single edge, and the panel's own
    // cost is measured in the test above.
    let root = corpus();
    let shelf = Shelf::open(root, &scratch("dibur")).expect("the shelf opens");
    // Asserted rather than skipped over. Both are on the fixture shelf by
    // construction, so an absence here is a broken fixture and not a machine
    // without a download — and the whole point of this file no longer skipping
    // is that a check which cannot find what it checks must not report `ok`.
    let gemara = shelf
        .read("bavli/berakhot")
        .expect("Berakhot is on the shelf");
    let rashi = shelf
        .read("bavli/rashi-on-berakhot")
        .expect("Rashi on Berakhot is on the shelf");

    // Rashi on Berakhot 2a:1:3 is the third comment on Berakhot 2a:1, so the
    // base segment is his address with the last level dropped (spec.md §6.1's
    // rule, and the one `beside` follows).
    let mut looked_at = 0usize;
    let mut landed = 0usize;
    let mut shown = 0usize;
    for comment in &rashi.segments {
        if comment.id.path().len() < 3 {
            continue;
        }
        if girsa_app::spans::diburim(&comment.text).is_empty() {
            continue;
        }
        looked_at += 1;
        if looked_at > 500 {
            break;
        }
        let address: Vec<String> = comment.id.path()[..comment.id.path().len() - 1].to_vec();
        let Some(base) = gemara
            .segments
            .iter()
            .find(|segment| segment.id.path() == address.as_slice())
        else {
            continue;
        };
        let Some(span) = girsa_app::spans::dibur_span(&base.text, &comment.text, true) else {
            continue;
        };
        landed += 1;

        let drawn = girsa_app::display::Shown::of(&base.text, true);
        let letters: Vec<char> = drawn.text().chars().collect();
        assert!(span.end <= letters.len(), "the span is inside the line");
        let words: String = letters[span.clone()].iter().collect();
        assert!(!words.trim().is_empty());
        if shown < 3 {
            shown += 1;
            println!("{} — on: {words}", comment.id);
        }
    }

    println!("{landed} of {looked_at} diburim landed on their words");
    assert!(looked_at > 0, "Rashi on Berakhot declares diburim");
    // Not a rate to optimise — a fact to state. The ones that do not land are
    // refused on purpose: the words are not in the line, or are there twice.
    assert!(
        landed > 0,
        "at least some of Rashi's diburim are found in the Gemara he is on"
    );
}

#[test]
fn a_link_on_other_words_is_left_out_and_one_on_the_whole_line_is_not() {
    // The narrower question (spec.md §8.4): *which links are on these words*.
    // A link whose words are known and are elsewhere goes; a link with no span
    // stays, because the whole segment includes what was highlighted.
    let root = corpus();
    let shelf = Shelf::open(root, &scratch("words")).expect("the shelf opens");
    let at = first_mishnah(&shelf);
    let mut links = girsa_app::touching(&shelf, shelf.repairs(), &standing(&shelf, &at)).links;
    assert!(links.len() >= 2, "the first mishnah has links");

    // One link pinned to the first ten characters, one left as it came.
    links[0].span = Some(0..10);
    let on_the_words = girsa_app::links::touching_words(links.clone(), 2..6);
    assert!(
        on_the_words.iter().any(|link| link.span == Some(0..10)),
        "the one on those words is there"
    );
    let elsewhere = girsa_app::links::touching_words(links.clone(), 40..60);
    assert!(
        !elsewhere.iter().any(|link| link.span == Some(0..10)),
        "and it is not there when the highlight is elsewhere"
    );
    assert_eq!(
        elsewhere.len(),
        links.len() - 1,
        "everything with no span stays, because it is on the whole segment"
    );
}
