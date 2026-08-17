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

use girsa_app::session::Pointing;
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

/// How much slower correcting a typo may get between an empty layer and a
/// lifetime of corrections.
///
/// # Why a ratio and not only a wall clock
///
/// The budget above is about a reader at an application that is running. This
/// test runs inside `cargo test --workspace`, on a machine building and
/// running thirty other test binaries — measured on 20 cores, before the tests
/// took turns: 1,151 ms alone, 3,962 ms in the suite. A wall-clock assertion
/// there reports the state of the machine and calls it the state of the code,
/// and it did: it failed about one run in four and passed every time the file
/// was run by itself.
///
/// So the failure this file exists to catch is asserted as a **ratio measured
/// on the same machine at the same moment**: the empty layer and the corrected
/// one, minutes apart at most, under whatever load is there. Contention
/// divides out. The failure is *the cost of correcting one typo grows with how
/// many you have already fixed* — a slope — and a slope survives a busy
/// machine while an absolute number does not.
///
/// Measured: 311 ms with nothing on the layer, 866 ms after sixteen thousand
/// corrections, which is 2.8. Six is room for the disk to be having a bad
/// afternoon and not room for a second implementation of reading the layer.
///
/// **That argument holds for a busy machine and not for a fast one**, which is
/// half of it and was the whole of it until a macOS runner said otherwise. See
/// [`MEASURABLE`], which is the other half and the condition on this one.
const ALLOWED_SLOPE: u32 = 6;

/// The baseline below which the ratio above is measuring something else.
///
/// # The half of the argument that was missing
///
/// *Contention divides out* is true of a machine that is **busy**: both ends
/// inflate together and the ratio survives. It is not true of a machine that is
/// **fast**, and on 17 August 2026 the macOS runner proved it — `cargo test`
/// went red at 8.4 against an allowance of 6, on these numbers:
///
/// | | with nothing on the layer | with 16,000 corrections |
/// |---|---|---|
/// | where the constant was set | 311 ms | 866 ms |
/// | this machine, 17 Aug | 225 ms | 866 ms |
/// | the macOS runner, 17 Aug | **33 ms** | 277 ms |
///
/// The corrected end barely moved and the empty one fell by a factor of nine.
/// So the two ends are not the same measurement scaled: the empty case is
/// mostly reading a sefer off a disk, which that runner does very fast, and the
/// corrected case is mostly walking a layer, which is arithmetic. Divide a
/// stable numerator by a collapsing denominator and the ratio grows without
/// anything having got slower — and it had not: 277 ms is *faster in wall clock
/// than the 866 ms this constant calls healthy*.
///
/// A hundred milliseconds is where the reading stops dominating: the printed
/// breakdown of the empty case is two reads of about 45 ms each and a write of
/// one, so below that figure the ratio is comparing fixed costs. When the
/// baseline is under it the slope is not asserted and the run says so out loud
/// — a skipped assertion that prints nothing is the thing this repository keeps
/// finding a year later — and [`BUDGET`], which is what a reader actually
/// experiences, carries the test.
const MEASURABLE: Duration = Duration::from_millis(100);

/// Every line of the sefer that has a word to fix — three typos a day for
/// sixteen years, and more corrections than one person will ever make on one
/// sefer.
const A_LIFETIME: usize = 16_000;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-three-seconds-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Which lines carry the scanning error.
///
/// Three, not one, because the budget is measured as the fastest of three
/// walks of the whole path — see [`best_of`] — and a walk has to correct a
/// line nobody has corrected yet.
fn typos() -> [usize; ATTEMPTS] {
    [SEGMENTS / 2, SEGMENTS / 2 + 1_000, SEGMENTS / 2 + 2_000]
}

/// How many times the whole path is walked before the fastest is believed.
const ATTEMPTS: usize = 3;

/// One at a time.
///
/// The three tests in this file each build an 18,120-segment sefer and write
/// up to sixteen thousand corrections. Run concurrently — which is what
/// `cargo test` does with the tests in one binary — they are three copies of
/// the heaviest work in the repository competing for one disk, and the
/// measurement they take is of each other.
///
/// Measured before they took turns: 1,151 ms alone, 4,801 ms with the other
/// two running. The
/// assertion is about the reader's machine doing this one thing, so the tests
/// take turns. It costs nothing — the file is I/O-bound and they were not
/// finishing sooner in parallel.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Take the turn, surviving a panic in whoever had it before.
fn alone() -> std::sync::MutexGuard<'static, ()> {
    ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A sefer of the right size, with a scanning error on [`ATTEMPTS`] lines.
fn shelf_with_a_typo(root: &Path) -> [SegmentId; ATTEMPTS] {
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
            text: if typos().contains(&n) {
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
    };
    let imported = ImportedWork::assemble(work, raw);
    import::write(root, &imported).expect("writes");
    let mut index = String::new();
    index.push_str(&serde_json::to_string(&imported.work).expect("writes"));
    index.push('\n');
    std::fs::create_dir_all(root.join("works")).expect("dir");
    std::fs::write(root.join("works/index.jsonl"), index).expect("writes");
    typos().map(|n| imported.segments[n - 1].id.clone())
}

/// The fastest of [`ATTEMPTS`] walks of the whole path, one per typo'd line.
///
/// # Why the fastest and not the only one
///
/// The budget is spec.md §7.5's three seconds, and that sentence is about *a
/// reader at an application that is running* — not about a machine building
/// and running thirty other test binaries at the same time. Measured as one
/// wall-clock reading inside `cargo test --workspace`, this assertion failed
/// about one run in four and passed every time the file was run alone, which
/// is a test that reports the state of the machine and calls it the state of
/// the code.
///
/// Three complete walks, and the fastest is believed. Nothing is skipped and
/// nothing is scaled: each attempt opens the shelf, applies every correction,
/// makes one more, and reads the sefer again. A regression that matters is
/// slower every time, and contention that does not matter is not.
fn best_of(root: &Path, personal: &Path, at: &[SegmentId]) -> (Duration, String) {
    let mut best: Option<(Duration, String)> = None;
    for one in at {
        let (took, corrected) = correct_it(root, personal, one);
        if best.as_ref().is_none_or(|(had, _)| took < *had) {
            best = Some((took, corrected));
        }
    }
    best.expect("there is at least one line to correct")
}

/// The reader is on the line, has highlighted `הרבר`, and has typed `הדבר`.
/// Everything from there to the corrected words being on the page.
fn correct_it(corpus: &Path, personal: &Path, at: &SegmentId) -> (Duration, String) {
    let began = Instant::now();
    let mut shelf = Shelf::open(corpus, personal).expect("the shelf opens");
    eprintln!("  open: {} ms", began.elapsed().as_millis());
    let one = Instant::now();
    let sefer = shelf.read(SLUG).expect("the sefer opens");
    eprintln!("  read: {} ms", one.elapsed().as_millis());
    let patch = girsa_app::correction(&sefer, at, 16..20, "הדבר", Kind::Ocr, "me", Pointing::Plain)
        .expect("a correction");
    assert_eq!(patch.was, "הרבר", "the highlight covered the typo");
    let two = Instant::now();
    shelf.fix(patch).expect("it is taken");
    eprintln!("  fix: {} ms", two.elapsed().as_millis());
    let three = Instant::now();
    let again = shelf.read(SLUG).expect("the sefer opens again");
    eprintln!("  read again: {} ms", three.elapsed().as_millis());
    let corrected = girsa_app::display::Shown::of(
        &again.segments[again.position_of(at).expect("it is here")].text,
        Pointing::Plain,
    )
    .text()
    .to_string();
    (began.elapsed(), corrected)
}

#[test]
fn correcting_a_typo_from_where_you_are_reading_fits_in_the_budget() {
    let _turn = alone();
    let root = scratch("fresh");
    let at = shelf_with_a_typo(&root);
    let personal = root.join("personal");

    let (took, corrected) = best_of(&root, &personal, &at);
    assert_eq!(corrected, "ובו סעיף אחד כל הדבר הזה מוקדם ובשבת");
    println!(
        "{SEGMENTS} segments, no corrections yet: {} ms",
        took.as_millis()
    );
    // The one absolute assertion, on the cheapest of the three cases: 283 ms
    // measured, and ten times that before it says anything. What the other two
    // assert is the slope, for the reason in `ALLOWED_SLOPE`.
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
    skipping: &[SegmentId],
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
        if skipping.contains(&segment.id) {
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
    let _turn = alone();
    // The failure this guards against is the one that arrives later: an
    // overlay that is fast when it is empty and quadratic when it is not. A
    // thousand corrections is a reader who fixes three typos a day for a year.
    let root = scratch("after-a-year");
    let at = shelf_with_a_typo(&root);

    let personal = root.join("personal");
    let empty = root.join("personal-empty");

    let made = corrections_already(&root, &personal, &at, 1_000);
    assert!(made > 900, "{made} corrections");

    let (bare, _) = correct_it(&root, &empty, &at[0]);
    let (took, corrected) = correct_it(&root, &personal, &at[1]);
    assert_eq!(corrected, "ובו סעיף אחד כל הדבר הזה מוקדם ובשבת");
    println!(
        "{SEGMENTS} segments: {} ms with nothing on the layer, {} ms with {made} corrections",
        bare.as_millis(),
        took.as_millis()
    );
    the_cost_is_not_growing(bare, took, made);
}

#[test]
fn it_is_still_in_the_budget_with_the_whole_sefer_corrected() {
    let _turn = alone();
    // Sixteen thousand corrections: every line of the sefer that has a word to
    // fix, which is three typos a day for sixteen years. Under a layer that
    // rewrote itself on every mutation this was ~2.3 seconds of file on top of
    // the reading, and getting *here* meant writing 128 million lines to make
    // sixteen thousand corrections — so the test could not have been written,
    // never mind passed.
    let root = scratch("whole-sefer");
    let at = shelf_with_a_typo(&root);
    let personal = root.join("personal");
    // A second layer over the same corpus, so the two ends of the slope are
    // measured on the same shelf, on the same machine, minutes apart at most.
    let empty = root.join("personal-empty");

    let made = corrections_already(&root, &personal, &at, A_LIFETIME);
    assert!(made >= A_LIFETIME, "{made} corrections");

    let (bare, _) = correct_it(&root, &empty, &at[0]);
    let (took, corrected) = correct_it(&root, &personal, &at[1]);
    assert_eq!(corrected, "ובו סעיף אחד כל הדבר הזה מוקדם ובשבת");
    println!(
        "{SEGMENTS} segments: {} ms with nothing on the layer, {} ms with {made} corrections \
         ({}%)",
        bare.as_millis(),
        took.as_millis(),
        took.as_millis() * 100 / bare.as_millis().max(1)
    );
    the_cost_is_not_growing(bare, took, made);
}

/// The slope and the budget, asserted once so both tests hold the same rule.
///
/// Two assertions and they answer different questions. The slope is *is the
/// layer getting more expensive as it fills* — the failure spec.md §7.5 is
/// about, and the one no wall clock catches until it is far too late. The
/// budget is *would a reader notice*, which is the promise itself.
///
/// The slope is skipped, loudly, when the baseline is below [`MEASURABLE`];
/// the budget never is. See [`MEASURABLE`] for the machine that made this
/// necessary and the numbers it produced.
fn the_cost_is_not_growing(bare: Duration, took: Duration, made: usize) {
    if bare < MEASURABLE {
        println!(
            "the slope is not asserted here: {} ms with nothing on the layer is under the \
             {} ms a ratio needs to mean anything on this machine, so what holds below is \
             the budget",
            bare.as_millis(),
            MEASURABLE.as_millis()
        );
    } else {
        assert!(
            took <= bare * ALLOWED_SLOPE,
            "correcting one typo took {took:?} with {made} corrections on the layer and \
             {bare:?} with none — the cost of a correction is growing with how many you have \
             already made, which is the failure spec.md §7.5 is about"
        );
    }
    assert!(
        took < BUDGET,
        "correcting one typo among {made} took {took:?}, and spec.md §7.5 says three \
         seconds is the whole interaction — including the reader"
    );
}
