//! BUILDER.md W10: *browse by the real taxonomy — **with the arrangement
//! editable**. The shipped taxonomy is a default, not a fact.*
//!
//! Four things have to hold at once, and only the first is obvious:
//!
//! 1. an edit moves the sefer;
//! 2. **the corpus is not touched** — the arrangement is a file of yours, the
//!    way corrections (spec.md §7.1) and link judgments (§8.3) are;
//! 3. **it survives a re-import** — `girsa-import` rewrites all 7,189
//!    catalogue records on every run, and an arrangement keyed to anything but
//!    the slug would quietly come apart there;
//! 4. **nothing is ever lost.** However the shelf is rearranged, every sefer is
//!    on exactly one of them. A work that falls out of the tree is a work that
//!    is on the shelf and cannot be browsed to, and nothing would say so.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use girsa_app::taxonomy::Branch;
use girsa_app::Shelf;

/// A corpus and a personal layer, both empty.
fn scratch(name: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    let corpus = dir.join("corpus");
    std::fs::create_dir_all(corpus.join("works")).expect("a corpus");
    (corpus, dir.join("personal"))
}

/// Write a catalogue of works: slug, Hebrew title, categories.
fn catalogue(corpus: &Path, works: &[(&str, &str, &[&str])]) {
    let mut body = String::new();
    for (slug, title, categories) in works {
        let record = serde_json::json!({
            "slug": slug,
            "he_title": title,
            "en_title": slug,
            "categories": categories,
            "source": "sefaria",
            "origin": "",
        });
        body.push_str(&record.to_string());
        body.push('\n');
    }
    std::fs::write(corpus.join("works/index.jsonl"), body).expect("a catalogue");
}

const SHAS: &[(&str, &str, &[&str])] = &[
    (
        "bavli/berakhot",
        "ברכות",
        &["Talmud", "Bavli", "Seder Zeraim"],
    ),
    ("bavli/shabbat", "שבת", &["Talmud", "Bavli", "Seder Moed"]),
    (
        "shulchan-arukh/orach-chayim",
        "שולחן ערוך אורח חיים",
        &["Halakhah", "Shulchan Arukh"],
    ),
    ("קרן-אורה", "קרן אורה", &["תלמוד בבלי", "אחרונים"]),
];

/// Every branch of the tree, flattened, by key.
fn flat(tree: &[Branch]) -> BTreeMap<String, Branch> {
    let mut out = BTreeMap::new();
    let mut stack: Vec<&Branch> = tree.iter().collect();
    while let Some(branch) = stack.pop() {
        stack.extend(branch.children.iter());
        out.insert(branch.key.clone(), branch.clone());
    }
    out
}

/// Every sefer the tree can be browsed to, and how many times.
fn reachable(shelf: &Shelf) -> BTreeMap<String, usize> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for key in flat(&shelf.tree()).keys() {
        for work in shelf.works_on(key) {
            *seen.entry(work.slug.clone()).or_default() += 1;
        }
    }
    seen
}

#[test]
fn a_sefer_moved_to_another_shelf_is_there_and_is_not_on_two() {
    let (corpus, personal) = scratch("girsa-shelf-move");
    catalogue(&corpus, SHAS);
    let mut shelf = Shelf::open(&corpus, &personal).expect("the shelf opens");

    assert_eq!(
        shelf
            .works_on("תלמוד/בבלי/סדר זרעים")
            .iter()
            .map(|w| w.slug.as_str())
            .collect::<Vec<_>>(),
        ["bavli/berakhot"]
    );

    let mine = {
        let mut key = String::new();
        shelf
            .edit(|a| {
                key = a.make(girsa_app::arrangement::TOP, "לחבורה");
                a.put_work("bavli/berakhot", &key);
                Ok(())
            })
            .expect("the edit takes");
        key
    };

    assert_eq!(
        shelf
            .works_on(&mine)
            .iter()
            .map(|w| w.slug.as_str())
            .collect::<Vec<_>>(),
        ["bavli/berakhot"]
    );
    assert!(
        shelf.works_on("תלמוד/בבלי/סדר זרעים").is_empty(),
        "and it is not still on the shelf it came off"
    );

    let tree = flat(&shelf.tree());
    let made = tree.get(&mine).expect("the made shelf is in the tree");
    assert_eq!(made.title, "לחבורה");
    assert_eq!(made.count, 1);
    assert!(made.mine);
    // The counts still add up to the whole shelf.
    assert_eq!(
        shelf.tree().iter().map(|b| b.count).sum::<usize>(),
        SHAS.len()
    );
}

#[test]
fn an_edit_is_a_file_of_yours_and_the_corpus_is_not_touched() {
    let (corpus, personal) = scratch("girsa-shelf-untouched");
    catalogue(&corpus, SHAS);
    let before = fingerprint(&corpus);

    let mut shelf = Shelf::open(&corpus, &personal).expect("the shelf opens");
    shelf
        .edit(|a| {
            let key = a.make(girsa_app::arrangement::TOP, "חבורה");
            a.put_work("bavli/shabbat", &key);
            a.put_shelf("הלכה", &key)?;
            a.rename("תלמוד", "הש״ס");
            a.reorder(girsa_app::arrangement::TOP, vec![key]);
            Ok(())
        })
        .expect("the edits take");

    assert_eq!(
        before,
        fingerprint(&corpus),
        "the corpus is exactly as it was"
    );
    assert!(
        personal.join("shelf.json").is_file(),
        "and the whole of the change is one file in your own layer"
    );
}

#[test]
fn an_edit_survives_the_corpus_being_reimported() {
    let (corpus, personal) = scratch("girsa-shelf-reimport");
    catalogue(&corpus, SHAS);

    let mut shelf = Shelf::open(&corpus, &personal).expect("the shelf opens");
    shelf
        .edit(|a| {
            a.put_work("bavli/berakhot", "הלכה");
            a.rename("תלמוד/בבלי", "הש״ס בבלי");
            Ok(())
        })
        .expect("the edits take");

    // What a re-import does: every record rewritten, in a different order,
    // with a work added and one work's categories changed upstream.
    catalogue(
        &corpus,
        &[
            ("קרן-אורה", "קרן אורה", &["תלמוד בבלי", "אחרונים"]),
            ("bavli/shabbat", "שבת", &["Talmud", "Bavli", "Seder Moed"]),
            (
                "bavli/berakhot",
                "ברכות",
                &["Talmud", "Bavli", "Seder Moed"],
            ),
            (
                "bavli/eruvin",
                "עירובין",
                &["Talmud", "Bavli", "Seder Moed"],
            ),
            (
                "shulchan-arukh/orach-chayim",
                "שולחן ערוך אורח חיים",
                &["Halakhah", "Shulchan Arukh"],
            ),
        ],
    );

    let shelf = Shelf::open(&corpus, &personal).expect("the shelf opens again");
    assert_eq!(
        shelf
            .works_on("הלכה")
            .iter()
            .map(|w| w.slug.as_str())
            .collect::<Vec<_>>(),
        ["bavli/berakhot"],
        "the sefer you filed is where you filed it"
    );
    assert_eq!(
        flat(&shelf.tree())
            .get("תלמוד/בבלי")
            .map(|b| b.title.clone()),
        Some("הש״ס בבלי".to_string()),
        "and the shelf you named is still called what you called it"
    );
    // The work that arrived in the re-import is on the shipped shelf, and the
    // whole shelf still accounts for everybody exactly once.
    let seen = reachable(&shelf);
    assert_eq!(seen.len(), 5);
    assert!(seen.values().all(|n| *n == 1), "{seen:#?}");
}

#[test]
fn every_sefer_is_on_exactly_one_shelf_after_a_pile_of_edits() {
    let (corpus, personal) = scratch("girsa-shelf-pile");
    catalogue(&corpus, SHAS);
    let mut shelf = Shelf::open(&corpus, &personal).expect("the shelf opens");

    shelf
        .edit(|a| {
            let one = a.make(girsa_app::arrangement::TOP, "ראשונה");
            let two = a.make(&one, "שניה");
            a.put_work("bavli/berakhot", &two);
            a.put_work("קרן-אורה", &one);
            // A shelf moved under a made shelf, taking what stands on it.
            a.put_shelf("הלכה/שולחן ערוך", &two)?;
            // A shelf moved onto another shipped shelf.
            a.put_shelf("תלמוד/בבלי", "מוסר")?;
            a.rename(&one, "החבורה");
            Ok(())
        })
        .expect("the edits take");

    let seen = reachable(&shelf);
    assert_eq!(
        seen.len(),
        SHAS.len(),
        "every sefer is browsable: {seen:#?}"
    );
    assert!(
        seen.values().all(|n| *n == 1),
        "and on one shelf: {seen:#?}"
    );
    assert_eq!(
        shelf.tree().iter().map(|b| b.count).sum::<usize>(),
        SHAS.len(),
        "and the counts say the same thing"
    );
}

#[test]
fn a_shelf_that_will_not_read_costs_the_arrangement_and_not_the_library() {
    let (corpus, personal) = scratch("girsa-shelf-broken");
    catalogue(&corpus, SHAS);
    std::fs::create_dir_all(&personal).unwrap();
    std::fs::write(personal.join("shelf.json"), "{ not json at all").unwrap();

    let shelf = Shelf::open(&corpus, &personal).expect("the shelf still opens");
    assert!(shelf.trouble().is_some(), "and the reader is told why");
    assert_eq!(reachable(&shelf).len(), SHAS.len());
    assert!(
        personal.join("shelf.json.unreadable").is_file(),
        "the file itself is kept — it is the only copy of somebody's filing"
    );
}

#[test]
fn a_loop_in_a_hand_edited_shelf_does_not_take_the_seforim_with_it() {
    let (corpus, personal) = scratch("girsa-shelf-loop");
    catalogue(&corpus, SHAS);
    std::fs::create_dir_all(&personal).unwrap();
    // `put_shelf` refuses this; a text editor does not.
    std::fs::write(
        personal.join("shelf.json"),
        r#"{"shelves":{"תלמוד":"הלכה","הלכה":"תלמוד"}}"#,
    )
    .unwrap();

    let shelf = Shelf::open(&corpus, &personal).expect("the shelf opens");
    let seen = reachable(&shelf);
    assert_eq!(seen.len(), SHAS.len(), "{seen:#?}");
    assert!(seen.values().all(|n| *n == 1), "{seen:#?}");
}

/// spec.md §5: *your own material, whenever — not an onboarding step, not a
/// second-class attachment.* End to end, through the same shelf as Shas.
#[test]
fn a_file_you_drop_in_is_a_sefer_on_the_shelf_like_any_other() {
    let (corpus, personal) = scratch("girsa-shelf-mine");
    catalogue(&corpus, SHAS);
    let handout = corpus.parent().expect("a scratch dir").join("חבורה.txt");
    std::fs::write(&handout, "ראשית הענין\n\nוזה מה שנראה לי").expect("a file to drop");

    let mut shelf = Shelf::open(&corpus, &personal).expect("the shelf opens");
    let slug = shelf.add_mine(&handout, None).expect("it is added");
    assert_eq!(slug, "user/חבורה");

    // On the shelf, where spec.md §5 says yours go.
    assert_eq!(
        shelf
            .works_on("שלי")
            .iter()
            .map(|w| w.slug.as_str())
            .collect::<Vec<_>>(),
        [slug.as_str()]
    );
    // Openable, with its own permanent ids, through the same call a masechta
    // is opened by.
    let open = shelf.read(&slug).expect("it opens");
    assert_eq!(open.segments.len(), 2);
    assert_eq!(open.segments[0].text, "ראשית הענין");
    // And it can be filed like anything else.
    shelf
        .edit(|a| {
            a.put_work(&slug, "תלמוד/בבלי");
            Ok(())
        })
        .expect("the edit takes");
    assert_eq!(shelf.works_on("תלמוד/בבלי").len(), 1);

    // It is still there after a restart, and the corpus never heard of it.
    let again = Shelf::open(&corpus, &personal).expect("the shelf opens again");
    assert!(again.work(&slug).is_some());
    assert!(!std::fs::read_to_string(corpus.join("works/index.jsonl"))
        .unwrap()
        .contains("user/"));
    let seen = reachable(&again);
    assert_eq!(seen.len(), SHAS.len() + 1);
    assert!(seen.values().all(|n| *n == 1), "{seen:#?}");
}

/// Every file under a directory, and what is in it.
fn fingerprint(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(body) = std::fs::read(&path) {
                out.insert(path.display().to_string(), body);
            }
        }
    }
    out
}
