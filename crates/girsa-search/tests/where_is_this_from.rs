//! W18 — *where is this phrase from?* and *who quotes this Gemara?*
//!
//! spec.md §10.4 says these are one feature asked from two directions, and this
//! is the test that they are literally one call: the same function, with the
//! sefer you are standing in left out or not.
//!
//! The thing being guarded is not finding. A phrase search always finds
//! something; what it must not do is offer a **confident wrong mekor**. So the
//! assertions are about the count, about the refusal to call a common phrase a
//! quotation, and about a widened match never being presented as an exact one.

// A panic in a test is a failure report.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::import;
use girsa_corpus::work::{Source, Work};
use girsa_search::bar::Bar;
use girsa_search::facets::Catalogue;
use girsa_search::index::{BuildReport, SearchIndex};
use girsa_search::mekoros::{where_from, How};

/// A Gemara, a Shulchan Arukh that quotes it, and a Mishnah Berurah that quotes
/// them both. Three seforim is the smallest shelf on which *who quotes this*
/// and *where is this from* are different questions with different answers.
const BERAKHOT: [&str; 3] = [
    "מאימתי קורין את שמע בערבין",
    "משעה שהכהנים נכנסים לאכול בתרומתן",
    "אמר רבי יוחנן משום רבי שמעון בן יוחי",
];

const ORACH_CHAYIM: [&str; 3] = [
    "יתגבר כארי לעמוד בבוקר לעבודת בוראו",
    "וקורין קריאת שמע משעה שהכהנים נכנסים לאכול בתרומתן",
    "אמר רבי יוחנן בענין אחר",
];

const MISHNAH_BERURAH: [&str; 3] = [
    "משעה שהכהנים נכנסים לאכול בתרומתן כמו שכתב הטור",
    "יתגבר כארי לעמוד בבוקר ואף שהוא קשה",
    "אמר רבי יוחנן וכן הוא בכמה מקומות",
];

struct Shelf {
    root: PathBuf,
    works: Vec<Work>,
}

impl Shelf {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a root");

        let works = vec![
            work("bavli/berakhot", "ברכות"),
            work("shulchan-arukh/orach-chayim", "שולחן ערוך אורח חיים"),
            work("mishnah-berurah", "משנה ברורה"),
        ];
        let mut catalogue = String::new();
        for work in &works {
            catalogue.push_str(&serde_json::to_string(work).expect("a work"));
            catalogue.push('\n');
        }
        std::fs::create_dir_all(root.join("works")).expect("a works dir");
        std::fs::write(root.join("works/index.jsonl"), catalogue).expect("the catalogue");

        write_work(&root, &works[0], &BERAKHOT);
        write_work(&root, &works[1], &ORACH_CHAYIM);
        write_work(&root, &works[2], &MISHNAH_BERURAH);
        Self { root, works }
    }

    fn bar(&self) -> Bar {
        let mut index = SearchIndex::in_memory().expect("an index in memory");
        let mut writer = index.writer().expect("a writer");
        let mut segments = 0;
        for work in &self.works {
            let read = import::read_back(&self.root, &work.slug).expect("reading a work back");
            for segment in &read.segments {
                writer.add(segment, &[]).expect("adding a segment");
                segments += 1;
            }
        }
        writer.commit().expect("committing");
        index.reload().expect("reloading");
        index
            .declare(BuildReport {
                works: self.works.len(),
                segments,
                link_types: true,
            })
            .expect("declaring what went in");
        Bar::new(index, Catalogue::of(&self.works), &self.root)
    }
}

impl Drop for Shelf {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn work(slug: &str, title: &str) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: title.to_string(),
        en_title: slug.to_string(),
        categories: Vec::new(),
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

fn write_work(root: &Path, work: &Work, lines: &[&str]) {
    let slug = work.slug.as_str();
    let dir = import::work_dir(root, slug);
    std::fs::create_dir_all(&dir).expect("a work dir");
    std::fs::write(
        dir.join("work.json"),
        serde_json::to_string(work).expect("a work"),
    )
    .expect("work.json");
    let mut body = String::new();
    for (i, text) in lines.iter().enumerate() {
        let n = i + 1;
        let id = girsa_corpus::segment::SegmentId::new(
            slug,
            vec!["1".to_string(), n.to_string()],
            #[allow(clippy::cast_possible_truncation)]
            girsa_corpus::segment::Ordinal::root(n as u32),
        );
        body.push_str(&format!(
            "{}\n",
            serde_json::json!({"id": id.to_string(), "kind": "text", "text": text})
        ));
    }
    std::fs::write(dir.join("segments.jsonl"), body).expect("segments");
}

#[test]
fn a_phrase_out_of_a_document_finds_the_sefer_it_came_from() {
    // The Ksav direction: a writer highlights a phrase and asks where it is
    // from. The Gemara is among the answers, and so is everybody quoting it —
    // which is honest, and is why the count is part of the answer.
    let shelf = Shelf::new("girsa-w18-where");
    let found = where_from(&shelf.bar(), "משעה שהכהנים נכנסים לאכול בתרומתן", None, 10)
        .expect("looks it up");

    assert_eq!(found.how, How::Exactly);
    assert_eq!(found.total, 3);
    assert!(found.is_a_quotation());
    let works: Vec<&str> = found.candidates.iter().map(|c| c.work.as_str()).collect();
    assert!(works.contains(&"bavli/berakhot"), "{works:?}");
    assert!(works.contains(&"mishnah-berurah"), "{works:?}");
}

#[test]
fn standing_in_the_gemara_the_same_call_answers_who_quotes_it() {
    // The other direction, and the point of the whole module: it is the same
    // function. The only difference is that the sefer you are in is left out,
    // or the answer would begin by telling you where you are standing.
    let shelf = Shelf::new("girsa-w18-who");
    let found = where_from(
        &shelf.bar(),
        "משעה שהכהנים נכנסים לאכול בתרומתן",
        Some("bavli/berakhot"),
        10,
    )
    .expect("looks it up");

    assert_eq!(found.total, 2);
    assert_eq!(found.except.as_deref(), Some("bavli/berakhot"));
    for candidate in &found.candidates {
        assert_ne!(candidate.work, "bavli/berakhot");
    }
}

#[test]
fn a_phrase_that_is_a_turn_of_speech_is_counted_and_not_called_a_source() {
    // `אמר רבי יוחנן` is in every one of these seforim, and in four thousand
    // places in the real one. Offering the first as "the mekor" would be the
    // system inventing an answer — the one thing it may never do.
    let shelf = Shelf::new("girsa-w18-common");
    let bar = shelf.bar();
    let found = where_from(&bar, "אמר רבי יוחנן", None, 10).expect("looks it up");
    assert_eq!(found.total, 3);

    // On this shelf three is distinctive; the rule is a threshold, so the
    // assertion is on the rule and not on the number.
    assert_eq!(
        found.is_a_quotation(),
        found.total <= girsa_search::mekoros::TOO_COMMON
    );
    let common = girsa_search::mekoros::Found {
        total: girsa_search::mekoros::TOO_COMMON + 1,
        ..found
    };
    assert!(!common.is_a_quotation());
    assert!(common.describe().contains("ביטוי"), "{}", common.describe());
}

#[test]
fn a_quotation_that_is_not_letter_for_letter_says_it_was_widened() {
    // A writer's own quotation carries a prefix, a male spelling, a word left
    // out. The ladder is climbed only when nothing literal matched, and what
    // comes back is marked — a near match shown as an exact one is a wrong
    // mekor with a confident face on it.
    let shelf = Shelf::new("girsa-w18-widened");
    let found = where_from(&shelf.bar(), "וכשהכהנים נכנסים", None, 10).expect("looks it up");

    match &found.how {
        How::Widened { rung } => assert!(!rung.is_empty()),
        How::Exactly => assert_eq!(
            found.total, 0,
            "an exact answer must really have been exact"
        ),
    }
}

#[test]
fn a_phrase_in_no_sefer_at_all_says_so_rather_than_offering_the_nearest() {
    let shelf = Shelf::new("girsa-w18-nothing");
    let bar = shelf.bar();
    let found = where_from(&bar, "פרה אדומה", None, 10).expect("looks");
    assert_eq!(found.total, 0);
    assert!(found.candidates.is_empty());
    assert!(!found.is_a_quotation());
    assert_eq!(found.describe(), "אין בשום ספר");
}

#[test]
fn a_phrase_too_long_to_widen_says_that_rather_than_saying_it_is_nowhere() {
    // Found by writing the test above with a five-word phrase: widening it is
    // 34,300 exact searches and W13 refuses past a limit rather than freezing
    // the window. That refusal has to reach the reader — *nothing was found*
    // and *nothing beyond the literal was looked for* are different answers,
    // and only one of them means "keep looking".
    let shelf = Shelf::new("girsa-w18-toolong");
    let bar = shelf.bar();
    let found = where_from(&bar, "פרה אדומה תמימה בלי מום", None, 10).expect("still answers");
    assert_eq!(found.total, 0);
    let why = found.only_literally().expect("the reason is carried");
    assert!(why.contains("1024") || why.contains("limit"), "{why}");
    assert!(found.describe().contains("כלשונו"), "{}", found.describe());
}

#[test]
fn an_empty_phrase_is_refused_rather_than_matching_everything() {
    let shelf = Shelf::new("girsa-w18-empty");
    assert!(where_from(&shelf.bar(), "   ", None, 10).is_err());
}
