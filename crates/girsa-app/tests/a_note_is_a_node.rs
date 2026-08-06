//! W27's acceptance, on the real corpus (spec.md §11).
//!
//! > **Your notes are nodes.** A note has the same typed edges as anything
//! > else, so *"what have I already written that touches this sugya?"* is the
//! > same query as *"who quotes this Rishon?"*
//!
//! That is a claim about the code and not about a panel, so it is tested as
//! one: **one call**, [`girsa_app::touching`], standing on the first mishnah of
//! Berakhot, returning the Rambam on it and a note of mine in one list — with
//! nothing in the call that knows a note from a commentary.
//!
//! # Why it skips when the corpus is absent
//!
//! It needs the fetched corpus, imported, with links imported over it — not
//! committed, and not present on a fresh clone. A test that failed there would
//! be noise everybody learns to ignore. The claim above is also asserted
//! against a scratch shelf in `girsa-note`'s own tests, which run everywhere.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::shelf::Shelf;
use girsa_corpus::segment::SegmentId;
use girsa_link::{EdgeType, Method};
use girsa_note::{Collection, Mark, Member, SavedQuery};

const MISHNAH: &str = "mishnah-berakhot";
const RAMBAM: &str = "rambam-on-mishnah-berakhot";

fn corpus() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    root.join("links").is_dir().then_some(root)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-notes-{name}"));
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

/// The first mishnah of Berakhot, by its address rather than by its ordinal.
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

/// The place, as the sefer on disk describes it: every name those words have
/// carried. This is what the window builds before asking what touches a line,
/// and going through it here is the point — a test that made its own
/// [`Standing`] would not be exercising `Open::standing`.
fn standing(shelf: &Shelf, at: &SegmentId) -> girsa_corpus::standing::Standing {
    shelf.read(at.work()).expect("the sefer opens").standing(at)
}

#[test]
fn what_i_wrote_and_who_quotes_it_come_back_from_one_call() {
    let root = corpus_or_skip!();
    let personal = scratch("one-call");
    let mut shelf = Shelf::open(&root, &personal).expect("the shelf opens");
    let at = first_mishnah(&shelf);

    let before = girsa_app::touching(&shelf, shelf.repairs(), &standing(&shelf, &at));
    let rambam = before
        .links
        .iter()
        .find(|link| link.work == RAMBAM)
        .expect("the Rambam on this mishnah is one of them")
        .clone();

    // Three seconds' worth of interaction: a place, some words, done. W20 put
    // the guardrail on the clock and this order inherits it — what is timed is
    // everything after the words are typed, which here includes putting the
    // note on the shelf as a sefer.
    let began = std::time::Instant::now();
    let note = girsa_app::note_here(
        &mut shelf,
        &at,
        None,
        "וצריך עיון מה שכתב הרמב\"ם כאן, דמשמע דהוי חיוב גמור.",
        "the test",
    )
    .expect("writes");
    let took = began.elapsed();
    println!("writing a note took {} ms", took.as_millis());
    assert!(
        took < std::time::Duration::from_millis(500),
        "writing a note took {took:?} — §7.5's guardrail is about how this feels"
    );

    let after = girsa_app::touching(&shelf, shelf.repairs(), &standing(&shelf, &at));
    assert_eq!(
        after.links.len(),
        before.links.len() + 1,
        "the note is one more link on the line, not a second list beside it"
    );

    let mine = after
        .links
        .iter()
        .find(|link| link.work == note.slug)
        .expect("what I wrote is in the same list as what the library says");

    // The same type. This is the whole claim: not `Note` beside `Link`, but a
    // `Link` whose far end happens to be something I wrote.
    assert_eq!(mine.repaired.edge.method, Method::ByHand);
    assert_eq!(mine.repaired.edge.edge_type, EdgeType::CommentsOn);
    assert_eq!(mine.repaired.confidence(), 1.0);
    assert!(mine.repaired.is_curated(), "you wrote it, so it is a claim");
    assert!(mine.repaired.mine);
    assert_eq!(mine.he_title, note.title, "and the row says what it is");
    println!(
        "{} · {} · {:.0}%   ⟷   {} · {} · {:.0}%",
        rambam.said(),
        rambam.repaired.edge.edge_type.as_str(),
        rambam.repaired.confidence() * 100.0,
        mine.said(),
        mine.repaired.edge.edge_type.as_str(),
        mine.repaired.confidence() * 100.0,
    );

    // Sorted together, by the same rule — the note is first because you are
    // the authority on your own layer and the seed is not.
    assert_eq!(
        after.links.first().map(|link| link.work.clone()),
        Some(note.slug.clone()),
        "the strongest claim first, and yours is the strongest"
    );

    // And not one byte into the shipped graph.
    let shard = girsa_link::store::edges_path(&root, at.work());
    let before_bytes = std::fs::read(&shard).expect("the shard reads");
    girsa_app::note_here(&mut shelf, &at, Some("עוד"), "ועיין עוד", "the test").expect("writes");
    assert_eq!(
        std::fs::read(&shard).expect("the shard reads"),
        before_bytes,
        "a note may not write into the corpus"
    );

    let _ = std::fs::remove_dir_all(&personal);
}

#[test]
fn a_note_is_a_sefer_on_the_shelf_and_opens_like_one() {
    let root = corpus_or_skip!();
    let personal = scratch("a-sefer");
    let mut shelf = Shelf::open(&root, &personal).expect("the shelf opens");
    let at = first_mishnah(&shelf);
    let note = girsa_app::note_here(
        &mut shelf,
        &at,
        Some("מאימתי"),
        "פסקה ראשונה\n\nפסקה שנייה",
        "the test",
    )
    .expect("writes");

    // Openable in a pane, in this session, without a restart.
    let open = shelf.read(&note.slug).expect("a note opens like a sefer");
    assert_eq!(open.segments.len(), 3, "a title and two paragraphs");
    assert_eq!(open.work.he_sections, vec!["פסקה".to_string()]);
    assert_eq!(open.segments[1].text, "פסקה ראשונה");

    // And it is on the shelf under yours, beside anything else you added.
    let yours_shelf = shelf
        .tree()
        .into_iter()
        .find(|branch| branch.key.contains("שלי"))
        .expect("there is a shelf for your own material");
    assert!(
        yours_shelf.count > 0,
        "and the note is standing on it: {yours_shelf:?}"
    );
    assert!(shelf
        .works_on(&yours_shelf.key)
        .iter()
        .any(|work| work.slug == note.slug));
    assert!(shelf.work(&note.slug).is_some());

    // The file is the truth, and it is plain: readable with no program at all.
    let file = girsa_note::note::path_in(&personal, note.name());
    let body = std::fs::read_to_string(&file).expect("a note is a file");
    assert!(body.contains("פסקה ראשונה"));
    assert!(
        body.contains(&at.to_string()),
        "and what it is about is in it, not only in the graph"
    );
    println!("{}\n{body}", file.display());

    let _ = std::fs::remove_dir_all(&personal);
}

#[test]
fn a_note_survives_the_line_it_is_on_being_split() {
    // spec.md §11: *notes anchored to segment ids, **surviving corpus
    // updates***. W6 proved that for 501 links; this is the same proof for the
    // one kind of anchor that is yours rather than the corpus's.
    let root = corpus_or_skip!();
    let personal = scratch("split");
    let mut shelf = Shelf::open(&root, &personal).expect("the shelf opens");
    let at = first_mishnah(&shelf);
    girsa_app::note_here(&mut shelf, &at, Some("מאימתי"), "הא דתנן", "the test").expect("writes");
    let words = first_four(&shelf, &at);
    shelf
        .marks_mut()
        .add(Mark::highlight(at.clone(), 0..4, words, "the test"))
        .expect("marks");
    girsa_app::collect(&mut shelf, "thursday", "חבורה יום ה", &at).expect("collects");

    // Somebody corrects a typo in a way that splits the line in two.
    // Said as a cut rather than derived from the shelf, because the corpus on
    // disk has not been re-imported: a cut is exactly the event that takes the
    // parent off the shelf and hands its name to the pieces, and this is that
    // fact stated. `a_link_survives_a_resegmentation.rs` runs the same claim end
    // to end through a corpus that really was cut.
    for child in at.split(2) {
        let cut = girsa_corpus::standing::Standing::of(child.clone(), [at.clone()]);
        let found = girsa_app::touching(&shelf, shelf.repairs(), &cut);
        assert!(
            found
                .links
                .iter()
                .any(|link| link.work.starts_with("note/")),
            "the note is still on {child}"
        );
        let yours = girsa_app::yours(&shelf, &cut, "");
        assert_eq!(yours.notes.len(), 1, "and it is still listed as yours");
        assert_eq!(yours.marks.len(), 1, "so is the highlight");
        assert_eq!(yours.folders, vec!["thursday".to_string()]);
    }

    let _ = std::fs::remove_dir_all(&personal);
}

#[test]
fn the_whole_of_your_layer_is_plain_files_you_can_take_with_you() {
    // spec.md §11: *everything local, everything exportable as plain files, no
    // account.* Asserted by reading the export back with nothing but serde and
    // a string search — no Girsa on the other end.
    let root = corpus_or_skip!();
    let personal = scratch("export");
    let mut shelf = Shelf::open(&root, &personal).expect("the shelf opens");
    let at = first_mishnah(&shelf);
    let note = girsa_app::note_here(&mut shelf, &at, Some("מאימתי"), "הא דתנן", "the test")
        .expect("writes");
    shelf
        .marks_mut()
        .add(Mark::bookmark(at.clone(), "the test").called("להתחיל כאן"))
        .expect("marks");
    shelf
        .queries_mut()
        .save(SavedQuery::new("מאימתי", "\"מאימתי קורין\"").with_chip("mode", "ToratEmet"))
        .expect("saves");
    let mut folder = Collection::new("thursday", "חבורה יום ה");
    folder.put(Member::Place(at.clone()));
    folder.put(Member::Work(note.slug.clone()));
    folder.put(Member::Query("מאימתי".to_string()));
    shelf.collections_mut().save(folder).expect("saves");

    let into = scratch("export-into");
    let written = girsa_note::export(
        shelf.notes(),
        shelf.marks(),
        shelf.queries(),
        shelf.collections(),
        &into,
    )
    .expect("exports");
    assert_eq!(written.notes, 1);
    assert_eq!(written.marks, 1);
    assert_eq!(written.queries, 1);
    assert_eq!(written.collections, 1);

    let note_file = into
        .join("notes")
        .join(girsa_note::note::file_name(note.name()));
    let body = std::fs::read_to_string(&note_file).expect("the note came out");
    assert!(body.contains("הא דתנן"));
    let folders =
        std::fs::read_to_string(into.join("collections.jsonl")).expect("folders came out");
    assert!(folders.contains(&at.to_string()));
    assert!(folders.contains(&format!("work:{}", note.slug)));
    assert!(folders.contains("query:מאימתי"));

    let _ = std::fs::remove_dir_all(&personal);
    let _ = std::fs::remove_dir_all(&into);
}

fn first_four(shelf: &Shelf, at: &SegmentId) -> String {
    let sefer = shelf.read(at.work()).expect("the sefer opens");
    let text = sefer.as_printed(at);
    text.chars().take(4).collect()
}
