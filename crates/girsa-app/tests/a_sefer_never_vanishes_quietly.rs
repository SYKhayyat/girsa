//! A catalogue line that will not parse is a sefer that is not on the shelf,
//! and it has to be said out loud.
//!
//! **Red on purpose** — written by a grader, pinning a defect (`GRADE` finding
//! N-2).
//!
//! `Shelf::open` is careful about this everywhere else. Corrections, link
//! repairs, scans, notes, marks, saved queries and collections each return
//! their bad lines, and all of them are folded into `trouble()` so the window
//! can say what would not read. The comment there is explicit:
//!
//! > *A correction that will not read is one correction, and it is said out
//! > loud — not a library that refuses to open.*
//!
//! The corpus catalogue is the one loader that does not get this treatment:
//!
//! ```rust,ignore
//! fn catalogue(body: &str) -> Vec<Work> {
//!     body.lines()
//!         .filter(|l| !l.trim().is_empty())
//!         .filter_map(|l| serde_json::from_str::<Work>(l).ok())   // <-- dropped
//!         .collect()
//! }
//! ```
//!
//! So a truncated write, a bad merge, or a half-finished import removes seforim
//! from the library with no report anywhere. It is the highest-consequence file
//! in the corpus and the only one whose failures are silent.
//!
//! Observed, with a three-line catalogue whose middle line was corrupt:
//!
//! ```text
//! $ girsa-shelf <that corpus>
//! …
//! 3 shelves · 4 seforim counted of 4 on the shelf
//! $ echo $?
//! 0
//! ```
//!
//! Note the shape of that last line: it counts what it loaded against what it
//! loaded, so it reads as a self-check and cannot fail. spec.md §5 promises the
//! opposite of this — *"Nothing is ever missing, and 'is it in there?' stops
//! being a question you have to think about."*

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use girsa_app::shelf::Shelf;
use girsa_corpus::work::{Source, Work};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-vanish-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn a_work(slug: &str) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: slug.to_string(),
        en_title: slug.to_string(),
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

/// Two good lines and one that was cut off mid-write.
fn corpus_with_a_torn_catalogue(root: &std::path::Path) {
    std::fs::create_dir_all(root.join("works")).expect("a works dir");
    let good_one = serde_json::to_string(&a_work("berakhot")).expect("serializes");
    let good_two = serde_json::to_string(&a_work("shabbat")).expect("serializes");
    std::fs::write(
        root.join("works/index.jsonl"),
        // The middle line is what a write interrupted by a full disk or a
        // killed importer leaves behind.
        format!("{good_one}\n{{\"slug\":\"eruvin\",\"he_title\":\n{good_two}\n"),
    )
    .expect("a catalogue");
}

/// **Red today**: the shelf opens with two of the three lines loaded, reports no
/// trouble at all, and `eruvin` is simply not in the library.
#[test]
fn a_catalogue_line_that_will_not_parse_is_reported() {
    let root = scratch("torn");
    corpus_with_a_torn_catalogue(&root);

    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");

    // Opening is right — one bad line is not a reason to refuse the library.
    assert_eq!(
        shelf.works().len(),
        2,
        "the two readable works are on the shelf"
    );

    let said = shelf.trouble().unwrap_or("");
    assert!(
        !said.is_empty(),
        "a sefer went missing and the shelf reported no trouble at all"
    );
}

/// The report has to be actionable — the same bar every other loader here
/// meets: name the file, and say which line.
///
/// **Red today**, for the same reason as above.
#[test]
fn the_report_names_the_file_and_the_line() {
    let root = scratch("named");
    corpus_with_a_torn_catalogue(&root);

    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let said = shelf.trouble().unwrap_or("");

    assert!(
        said.contains("index.jsonl"),
        "the reader is told which file to look at: {said:?}"
    );
    assert!(
        said.contains('2'),
        "the reader is told which line would not read: {said:?}"
    );
}
