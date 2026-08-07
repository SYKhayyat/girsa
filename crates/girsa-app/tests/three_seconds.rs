//! The guardrail, measured.
//!
//! spec.md §7.5: *if correcting a typo is not a three-second interaction from
//! where you are reading, nobody does it — including you.* BUILDER.md W20 says
//! **measure it**, so this does, on a sefer the size of the one the spec keeps
//! quoting: Mishnah Berurah, 18,120 segments.
//!
//! # What is measured, and what is not
//!
//! Three seconds is a human number and most of it is the human: reading the
//! word, deciding, typing the letters. What a test can measure is the machine's
//! share of it — everything between the reader pressing a key and the corrected
//! words being on the page — and that is what is measured here, on the whole
//! path and not on a function in the middle of it:
//!
//! 1. open the shelf, with your corrections on it;
//! 2. read the sefer, applying them;
//! 3. turn a highlight into a correction;
//! 4. write it into your layer;
//! 5. read the sefer again, so the page has it.
//!
//! Step 5 is the expensive one and it is in the measurement on purpose: it is
//! what the window does today.
//!
//! The budget below is the spec's three seconds. The numbers are printed, so a
//! change that makes this eight times slower is visible in the run and not only
//! when it crosses the line — run with `--nocapture` to see them.
//!
//! # Why there are three sizes and not two
//!
//! This test used to stop at a thousand corrections, and a thousand was the last
//! size at which it passed. The layer was serialized in full on every mutation,
//! so the cost of correcting *one* typo grew with how many you had already
//! fixed: 142 ms of the 217 ms it printed was attributable to the thousand
//! patches already on the file, linearly, which put the three-second line at
//! about twenty thousand corrections and the test's last measurement at
//! one-twentieth of it.
//!
//! Naming a failure and then measuring up to just short of it is how a guardrail
//! goes green over the thing it guards. So the third size is the whole sefer
//! corrected — every line of Mishnah Berurah that has a word to fix, which is
//! sixteen years at three typos a day. The layer is an append-only log now
//! (`girsa-personal`), and what is left of the slope is reading your corrections
//! in order to apply them, which no design gets out of.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use girsa_app::shelf::Shelf;
use girsa_corpus::import::{self, ImportedWork, RawSegment, SegmentKind};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::{Source, Work};
use girsa_fix::{Kind, Patch};

/// spec.md §2.1, measured: Mishnah Berurah is 18,120 segments with 701
/// headings. The biggest sefer on the shelf is the one to measure on.
const SEGMENTS: usize = 18_120;
const SLUG: &str = "mishnah-berurah";

/// The whole of spec.md §7.5, in milliseconds.
const BUDGET: Duration = Duration::from_secs(3);

/// Every line of the sefer that has a word to fix — three typos a day for
/// sixteen years, and more corrections than one person will ever make on one
/// sefer.
const A_LIFETIME: usize = 16_000;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-three-seconds-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A sefer of the right size, with a scanning error on one line of it.
fn shelf_with_a_typo(root: &Path) -> SegmentId {
    let mut raw = Vec::with_capacity(SEGMENTS);
    for n in 1..=SEGMENTS {
        let siman = (n / 26) + 1;
        raw.push(RawSegment {
            path: vec![siman.to_string(), n.to_string()],
            kind: if n % 26 == 0 {
                SegmentKind::Heading
            } else {
                SegmentKind::Text
            },
            text: if n == SEGMENTS / 2 {
                "<b>ובו סעיף אחד</b> כל הרבר הזה מוקדם וּבַשַּׁבָּת".to_string()
            } else {
                format!("<b>סעיף {n}</b> יתגבר כארי לעמוד בבוקר לעבודת בוראו")
            },
        });
    }
    let work = Work {
        slug: SLUG.to_string(),
        he_title: "משנה ברורה".into(),
        en_title: "Mishnah Berurah".into(),
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
    };
    let imported = ImportedWork::assemble(work, raw);
    import::write(root, &imported).expect("writes");
    let mut index = String::new();
    index.push_str(&serde_json::to_string(&imported.work).expect("writes"));
    index.push('\n');
    std::fs::create_dir_all(root.join("works")).expect("dir");
    std::fs::write(root.join("works/index.jsonl"), index).expect("writes");
    imported.segments[SEGMENTS / 2 - 1].id.clone()
}

/// The reader is on the line, has highlighted `הרבר`, and has typed `הדבר`.
/// Everything from there to the corrected words being on the page.
fn correct_it(corpus: &Path, personal: &Path, at: &SegmentId) -> (Duration, String) {
    let began = Instant::now();
    let mut shelf = Shelf::open(corpus, personal).expect("the shelf opens");
    let sefer = shelf.read(SLUG).expect("the sefer opens");
    let patch = girsa_app::correction(&sefer, at, 16..20, "הדבר", Kind::Ocr, "me", false)
        .expect("a correction");
    assert_eq!(patch.was, "הרבר", "the highlight covered the typo");
    shelf.fix(patch).expect("it is taken");
    let again = shelf.read(SLUG).expect("the sefer opens again");
    let corrected = girsa_app::display::Shown::of(
        &again.segments[again.position_of(at).expect("it is here")].text,
        false,
    )
    .text()
    .to_string();
    (began.elapsed(), corrected)
}

#[test]
fn correcting_a_typo_from_where_you_are_reading_fits_in_the_budget() {
    let root = scratch("fresh");
    let at = shelf_with_a_typo(&root);
    let personal = root.join("personal");

    let (took, corrected) = correct_it(&root, &personal, &at);
    assert_eq!(corrected, "ובו סעיף אחד כל הדבר הזה מוקדם ובשבת");
    println!(
        "{SEGMENTS} segments, no corrections yet: {} ms",
        took.as_millis()
    );
    assert!(
        took < BUDGET,
        "correcting one typo took {took:?}, and spec.md §7.5 says three seconds is the whole \
         interaction — including the reader"
    );
}

/// Fill the layer with a correction on each of the first `how_many` lines that
/// have a word to fix. Returns how many were made.
fn corrections_already(
    root: &Path,
    personal: &Path,
    skipping: &SegmentId,
    how_many: usize,
) -> usize {
    let mut layer = girsa_fix::Layer::open(personal).0;
    let read = import::read_back(root, SLUG).expect("reads");
    for (n, segment) in read.segments.iter().enumerate() {
        if layer.count() >= how_many {
            break;
        }
        // `<b>סעיף 1</b> יתגבר …` — the ninth letter onwards is `יתגבר`, on
        // every line but the one the test corrects.
        if segment.id == *skipping {
            continue;
        }
        let letters: Vec<char> = segment.text.chars().collect();
        let start = 3 + format!("סעיף {}", n + 1).chars().count() + 5;
        let Some(was) = letters
            .get(start..start + 5)
            .map(|w| w.iter().collect::<String>())
        else {
            continue;
        };
        if was != "יתגבר" {
            continue;
        }
        layer
            .add(Patch::new(
                segment.id.clone(),
                start..start + 5,
                was,
                "יתחזק",
                Kind::Ocr,
                "me",
            ))
            .expect("takes it");
    }
    layer.count()
}

#[test]
fn it_is_still_in_the_budget_after_a_year_of_corrections() {
    // The failure this guards against is the one that arrives later: an
    // overlay that is fast when it is empty and quadratic when it is not. A
    // thousand corrections is a reader who fixes three typos a day for a year.
    let root = scratch("after-a-year");
    let at = shelf_with_a_typo(&root);
    let personal = root.join("personal");

    let made = corrections_already(&root, &personal, &at, 1_000);
    assert!(made > 900, "{made} corrections");

    let (took, corrected) = correct_it(&root, &personal, &at);
    assert_eq!(corrected, "ובו סעיף אחד כל הדבר הזה מוקדם ובשבת");
    println!(
        "{SEGMENTS} segments, {made} corrections already: {} ms",
        took.as_millis()
    );
    assert!(took < BUDGET, "correcting one typo took {took:?}");
}

#[test]
fn it_is_still_in_the_budget_with_the_whole_sefer_corrected() {
    // Sixteen thousand corrections: every line of the sefer that has a word to
    // fix, which is three typos a day for sixteen years. Under a layer that
    // rewrote itself on every mutation this was ~2.3 seconds of file on top of
    // the reading, and getting *here* meant writing 128 million lines to make
    // sixteen thousand corrections — so the test could not have been written,
    // never mind passed.
    let root = scratch("whole-sefer");
    let at = shelf_with_a_typo(&root);
    let personal = root.join("personal");

    let made = corrections_already(&root, &personal, &at, A_LIFETIME);
    assert!(made >= A_LIFETIME, "{made} corrections");

    let (took, corrected) = correct_it(&root, &personal, &at);
    assert_eq!(corrected, "ובו סעיף אחד כל הדבר הזה מוקדם ובשבת");
    println!(
        "{SEGMENTS} segments, {made} corrections already: {} ms",
        took.as_millis()
    );
    assert!(
        took < BUDGET,
        "correcting one typo among {made} took {took:?}, and spec.md §7.5 says three seconds is \
         the whole interaction — including the reader"
    );
}
