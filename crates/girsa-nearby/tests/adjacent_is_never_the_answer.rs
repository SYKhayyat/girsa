//! The semantic lane: off means off, coverage is always said, and the results
//! are adjacent rather than found.
//!
//! spec.md §9.9, ruled in §16 #20, built as BUILDER.md W30. Four claims, and
//! this file is what holds each of them:
//!
//! 1. **A query that shares no words with its target finds it.**
//! 2. **The same query with the lane off finds nothing and says why** — and
//!    literal search is byte-for-byte what it was.
//! 3. **Coverage is stated in every surface**, and a partial lane never reads as
//!    a complete one.
//! 4. **The lane is drawn as adjacent**, never merged into the literal hits.
//!
//! # Why the model here is a stub, and where the real one is measured
//!
//! The embedder in this file is a few lines long and knows about four topics.
//! That is deliberate, and it is the same split W26 made for OCR: the machinery
//! — the store, the resumable job, the ranking, the coverage sentence, the
//! refusals — is what has to be right and what a test can establish, and it must
//! be establishable **without 738 MB of weights on the machine running the
//! tests**. What a real model does to real Hebrew is a *measurement*, not an
//! assertion; it was taken with `girsa-lane ask` against side-loaded BEREL over
//! this corpus and the numbers are in the commit message and in
//! `girsa_lane::model`'s own documentation.
//!
//! The stub is honest about what it stands in for. It maps a line to a topic by
//! the words in it and returns a one-hot vector, so *this line is about the time
//! of krias shema* is something it knows and *these two lines share a word* is
//! something it does not — which is exactly the property claim 1 is about.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use girsa_app::naming::Names;
use girsa_app::Shelf;
use girsa_corpus::import::{self, ImportedWork, RawSegment, SegmentKind};
use girsa_corpus::work::{Source, Work};
use girsa_lane::model::{Embedded, Embedder, ModelError};
use girsa_lane::{Chosen, Lane, State};
use girsa_nearby::Adjacency;

// ---------------------------------------------------------------------------
// A model that knows meaning and nothing about spelling
// ---------------------------------------------------------------------------

/// Four topics, one dimension each.
///
/// A line's topic is decided by which of these words it holds; a line with none
/// of them gets the fourth dimension, which is *something else*. Two lines on
/// the same topic come out identical whether or not they share a single word,
/// and two lines that share words but not a topic come out orthogonal. That is
/// the whole of what an embedding buys and the whole of what this stands in for.
const TOPICS: [&[&str]; 3] = [
    // The time of krias shema, in four vocabularies that barely overlap.
    &[
        "שמע",
        "קריאת",
        "מאימתי",
        "ערבית",
        "לילה",
        "בין",
        "השמשות",
        "צאת",
        "הכוכבים",
    ],
    // Sukkah and lulav.
    &["סוכה", "לולב", "אתרוג", "הדס", "ערבה", "סכך"],
    // Damages.
    &["נזק", "שור", "בור", "מבעה", "הבער", "תשלומין"],
];

struct Topics;

impl Embedder for Topics {
    fn fingerprint(&self) -> &str {
        "topics-stub-1"
    }

    fn dims(&self) -> usize {
        TOPICS.len() + 1
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Embedded>, ModelError> {
        Ok(texts
            .iter()
            .map(|text| {
                let mut vector = vec![0.0f32; TOPICS.len() + 1];
                let words: Vec<&str> = text.split_whitespace().collect();
                let mut hits = 0;
                for (at, topic) in TOPICS.iter().enumerate() {
                    let n = words.iter().filter(|w| topic.contains(w)).count();
                    if n > 0 {
                        #[allow(clippy::cast_precision_loss)]
                        let weight = n as f32;
                        vector[at] = weight;
                        hits += n;
                    }
                }
                if hits == 0 {
                    vector[TOPICS.len()] = 1.0;
                }
                let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
                for value in &mut vector {
                    *value /= norm;
                }
                Embedded {
                    vector,
                    truncated: false,
                }
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// A shelf
// ---------------------------------------------------------------------------

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-lane-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn work(slug: &str, title: &str, lines: &[&str]) -> ImportedWork {
    let raw = lines
        .iter()
        .enumerate()
        .map(|(at, text)| RawSegment {
            path: vec![(at + 1).to_string()],
            kind: SegmentKind::Text,
            text: (*text).to_string(),
        })
        .collect();
    ImportedWork::assemble(
        Work {
            slug: slug.to_string(),
            he_title: title.to_string(),
            en_title: slug.to_string(),
            categories: vec!["Halakhah".into()],
            order: Vec::new(),
            source: Source::Sefaria,
            origin: PathBuf::new(),
            schema: None,
            he_sections: Vec::new(),
            author: None,
            era: None,
            comp_date: None,
            version: None,
            commentary_on: Vec::new(),
        },
        raw,
    )
}

/// Three seforim: one about the time of krias shema, one about sukkah, one
/// about neither.
fn shelf(dir: &Path) -> (Shelf, PathBuf, PathBuf) {
    let root = dir.join("corpus");
    let personal = dir.join("personal");
    let works = [
        work(
            "rishon-alef",
            "ראשון א",
            &[
                "מאימתי קורין את שמע בערבית",
                "משעה שהכהנים נכנסים לאכול בתרומתן",
                "ולא עוד אלא שכל מה שאמרו חכמים עד חצות",
            ],
        ),
        work(
            "rishon-beis",
            "ראשון ב",
            &[
                "צאת הכוכבים הוא בין השמשות ולא קודם",
                "ומצות לולב כל שבעה במקדש",
            ],
        ),
        work(
            "acharon-gimmel",
            "אחרון ג",
            &[
                "שור שנגח את הפרה ונמצא עוברה בצדה",
                "אין העדים חותמין על השטר אלא אם כן",
            ],
        ),
    ];
    let mut index = String::new();
    for imported in &works {
        import::write(&root, imported).expect("writes");
        index.push_str(&serde_json::to_string(&imported.work).expect("writes"));
        index.push('\n');
    }
    std::fs::create_dir_all(root.join("works")).expect("a corpus root");
    std::fs::write(root.join("works/index.jsonl"), index).expect("a catalogue");
    let shelf = Shelf::open(&root, &personal).expect("a shelf");
    (shelf, root, personal)
}

/// The lane on, over a stub model, with a selection.
fn lane_over(root: &Path, personal: &Path, shelf: &Shelf, chosen: Chosen) -> Adjacency {
    let mut lane = Lane::with(personal, Arc::new(Topics));
    lane.choose(chosen).expect("saves");
    Adjacency::with(root, lane, shelf)
}

// ---------------------------------------------------------------------------
// 1 · A query that shares no words with its target finds it
// ---------------------------------------------------------------------------

#[test]
fn a_query_sharing_no_words_with_its_target_finds_it() {
    let dir = scratch("no-shared-words");
    let (shelf, root, personal) = shelf(&dir);
    let mut lane = lane_over(&root, &personal, &shelf, Chosen::everything());

    let (wrote, trouble) = lane.embed(&shelf, &mut |_, _, _| true).expect("it embeds");
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(wrote, 7, "every line with words in it");

    // The query. Not one of these four words is anywhere in the line it should
    // find — check that first, because a test that accidentally shares a word is
    // testing literal search with extra steps.
    let asked = "זמן צאת הכוכבים";
    let target = "מאימתי קורין את שמע בערבית";
    for word in asked.split_whitespace() {
        assert!(
            !target.contains(word),
            "{word:?} is in the target, so this proves nothing"
        );
    }

    let answer = lane.ask(&Names::on(&shelf), asked, &[], 3);
    assert!(answer.refused.is_none(), "{:?}", answer.refused);
    assert!(!answer.near.is_empty(), "the lane found nothing");
    let found: Vec<&str> = answer.near.iter().map(|n| n.text.as_str()).collect();
    assert!(
        found.contains(&target),
        "the line about the time of krias shema is not in {found:?}"
    );
    // And what it is not: the line about a lulav, which shares the word `כל`
    // with nothing that matters, is nowhere near the top.
    assert!(
        answer.near[0].nearness > 0.5,
        "the nearest thing came back at {}",
        answer.near[0].nearness
    );
}

// ---------------------------------------------------------------------------
// 2 · With the lane off it finds nothing, and says why
// ---------------------------------------------------------------------------

#[test]
fn with_the_lane_off_the_same_query_finds_nothing_and_says_why() {
    let dir = scratch("off");
    let (shelf, root, personal) = shelf(&dir);

    // The default. Nobody has been near the setting.
    let (off, trouble) = Adjacency::open(&root, &personal, &shelf);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(off.state(), State::Off);
    assert_eq!(
        off.state().said(),
        None,
        "off is not a line in the header — there is no lane to be partial about"
    );

    let answer = off.ask(&Names::on(&shelf), "זמן צאת הכוכבים", &[], 3);
    assert!(answer.near.is_empty());
    let why = answer.refused.expect("a reason, not an empty list");
    assert!(why.contains("off"), "{why}");
    // And the coverage sentence is still there, saying the honest thing.
    assert_eq!(answer.coverage, "nothing is in the semantic lane yet");
}

#[test]
fn turning_the_lane_off_leaves_the_corpus_exactly_as_it_was() {
    // *Off means literal search is bit-for-bit what it was* — the strongest
    // form of it that can be checked here: nothing the lane does touches the
    // corpus, so a byte-for-byte comparison of the whole corpus tree before and
    // after embedding is the assertion. spec.md §4.1 and §9.9 in one line.
    let dir = scratch("untouched");
    let (shelf, root, personal) = shelf(&dir);
    let before = fingerprint_of(&root);

    let mut lane = lane_over(&root, &personal, &shelf, Chosen::everything());
    lane.embed(&shelf, &mut |_, _, _| true).expect("it embeds");
    let _ = lane.ask(&Names::on(&shelf), "זמן צאת הכוכבים", &[], 3);

    assert_eq!(
        before,
        fingerprint_of(&root),
        "the lane wrote into the corpus"
    );
    // Everything it wrote is in the personal layer, under `lane/`.
    assert!(personal.join("lane").is_dir());
}

/// Every file under a tree, by path and by bytes.
fn fingerprint_of(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = std::fs::read(&path) {
                out.push((path.display().to_string(), bytes));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

// ---------------------------------------------------------------------------
// 3 · Coverage, in every surface
// ---------------------------------------------------------------------------

#[test]
fn a_lane_over_one_sefer_says_what_the_other_two_are() {
    // The defect §9.9 exists to prevent, in its exact shape: everything chosen
    // is embedded, the answer is confident, and it is an answer about a third
    // of the shelf.
    let dir = scratch("partial");
    let (shelf, root, personal) = shelf(&dir);
    let mut lane = lane_over(
        &root,
        &personal,
        &shelf,
        Chosen::nothing().with_work("rishon-alef"),
    );
    lane.embed(&shelf, &mut |_, _, _| true).expect("it embeds");

    assert!(lane.coverage().is_whole(), "what was chosen is finished");
    let said = lane.coverage().said();
    assert!(said.contains("ראשון א"), "{said}");
    assert!(
        said.contains("2 other seforim on this shelf aren't in it"),
        "{said}"
    );
    // The same sentence travels with the answer, so a surface cannot draw the
    // results without it.
    let answer = lane.ask(&Names::on(&shelf), "זמן צאת הכוכבים", &[], 3);
    assert_eq!(answer.coverage, said);
    // The line about צאת הכוכבים is in `rishon-beis`, which is not in the lane.
    // So the top hit is the one in the sefer that is, and the reader is told
    // that two seforim were not looked at.
    assert!(!answer.near.is_empty());
    for near in &answer.near {
        assert_eq!(near.at.work, "rishon-alef");
    }
}

#[test]
fn a_half_embedded_lane_reports_both_numbers() {
    let dir = scratch("half");
    let (shelf, root, personal) = shelf(&dir);
    let mut lane = lane_over(&root, &personal, &shelf, Chosen::everything());

    // Stop after the first batch of the first sefer.
    let mut stop_after = 1;
    lane.embed(&shelf, &mut |_, _, _| {
        stop_after -= 1;
        stop_after > 0
    })
    .expect("it embeds");

    let said = lane.coverage().said();
    assert!(said.contains("so far"), "{said}");
    assert!(!lane.coverage().is_whole());
    assert!(!lane.coverage().is_nothing());

    // And it resumes: the second run finishes what the first started, without
    // redoing it.
    let (wrote, _) = lane.embed(&shelf, &mut |_, _, _| true).expect("it embeds");
    assert!(
        wrote > 0 && wrote < 7,
        "it resumed rather than restarted: {wrote}"
    );
    assert!(lane.coverage().is_whole());
    assert_eq!(lane.coverage().embedded(), 7);
}

#[test]
fn a_lane_scoped_away_from_everything_it_covers_says_that_rather_than_nothing() {
    let dir = scratch("scoped-out");
    let (shelf, root, personal) = shelf(&dir);
    let mut lane = lane_over(
        &root,
        &personal,
        &shelf,
        Chosen::nothing().with_work("rishon-alef"),
    );
    lane.embed(&shelf, &mut |_, _, _| true).expect("it embeds");

    let answer = lane.ask(
        &Names::on(&shelf),
        "זמן צאת הכוכבים",
        &["acharon-gimmel".to_string()],
        3,
    );
    assert!(answer.near.is_empty());
    let why = answer.refused.expect("a reason");
    assert!(why.contains("scope"), "{why}");
}

// ---------------------------------------------------------------------------
// 4 · Adjacent, and never merged
// ---------------------------------------------------------------------------

#[test]
fn every_answer_carries_the_adjacent_label_and_it_is_worded_once() {
    let dir = scratch("adjacent");
    let (shelf, root, personal) = shelf(&dir);
    let mut lane = lane_over(&root, &personal, &shelf, Chosen::everything());
    lane.embed(&shelf, &mut |_, _, _| true).expect("it embeds");

    // Found, refused, and off: all three carry it, because a reader must never
    // meet one of these lists without being told what kind of list it is.
    let found = lane.ask(&Names::on(&shelf), "זמן צאת הכוכבים", &[], 3);
    let refused = lane.ask(&Names::on(&shelf), "   ", &[], 3);
    let (off, _) = Adjacency::open(&root, &personal, &shelf);
    let when_off = off.ask(&Names::on(&shelf), "זמן צאת הכוכבים", &[], 3);

    for answer in [&found, &refused, &when_off] {
        assert_eq!(answer.label, girsa_lane::ADJACENT);
        assert!(answer.label.contains("rather than by these words"));
        assert!(!answer.coverage.is_empty());
    }
}

// ---------------------------------------------------------------------------
// And the model that made them
// ---------------------------------------------------------------------------

#[test]
fn vectors_from_another_model_are_not_read_and_are_not_silent() {
    // Two spaces, the same arithmetic, a ranked list that looks exactly like a
    // good one. The one failure mode of this feature a reader could never
    // notice from the results.
    struct Other;
    impl Embedder for Other {
        fn fingerprint(&self) -> &str {
            "some-other-model"
        }
        fn dims(&self) -> usize {
            TOPICS.len() + 1
        }
        fn embed(&self, texts: &[&str]) -> Result<Vec<Embedded>, ModelError> {
            Topics.embed(texts)
        }
    }

    let dir = scratch("other-model");
    let (shelf, root, personal) = shelf(&dir);
    let mut lane = lane_over(&root, &personal, &shelf, Chosen::everything());
    lane.embed(&shelf, &mut |_, _, _| true).expect("it embeds");
    assert_eq!(lane.coverage().embedded(), 7);

    // The reader points the setting at a different model.
    let mut other = Lane::with(&personal, Arc::new(Other));
    other.choose(Chosen::everything()).expect("saves");
    let mut swapped = Adjacency::with(&root, other, &shelf);

    assert_eq!(
        swapped.coverage().embedded(),
        0,
        "nothing is read out of them"
    );
    assert_eq!(swapped.coverage().other_model.len(), 3);
    let said = swapped.coverage().said();
    assert!(said.contains("another model"), "{said}");

    // And embedding under the new model refuses to add to them rather than
    // mixing, naming what made them.
    let (wrote, trouble) = swapped.embed(&shelf, &mut |_, _, _| true).expect("it runs");
    assert_eq!(wrote, 0);
    assert_eq!(trouble.len(), 3, "{trouble:?}");
    assert!(trouble[0].contains("topics-stub-1"), "{trouble:?}");

    // The old vectors are still there. Pointing the setting back costs nothing.
    let mut back = Lane::with(&personal, Arc::new(Topics));
    back.choose(Chosen::everything()).expect("saves");
    let restored = Adjacency::with(&root, back, &shelf);
    assert_eq!(restored.coverage().embedded(), 7);
}
