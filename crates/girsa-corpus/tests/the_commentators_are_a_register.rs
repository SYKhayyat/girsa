//! #46 — the authors the corpus names, as people rather than as 195 rows.
//!
//! The register's job is to answer three questions a reader actually asks when a
//! mefaresh they have never met opens: *who is this*, *when*, and *why here*. The
//! things it can get wrong while still looking like a register:
//!
//! 1. **merge two people** — `רמב״ם` and `רמב"ן` are one letter and one man each;
//! 2. **invent a date** — an era is a century and a year is a year, and a list
//!    that put an undated person in the middle of a chronology has asserted
//!    something nobody wrote down;
//! 3. **lose the doubt** — three works in this corpus are attributed rather than
//!    authored, and a register that prints those as fact is a register that lies
//!    about exactly the cases the corpus was careful about.

// A panic in a test is a failure report.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use girsa_corpus::commentator::{Attribution, Register};
use girsa_corpus::work::{BaseText, Mapping, Source, Work};

fn commentary(slug: &str, he: &str, en: &str, author: &str, on: &[&str]) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: he.to_string(),
        en_title: en.to_string(),
        categories: Vec::new(),
        order: Vec::new(),
        source: Source::Sefaria,
        origin: PathBuf::new(),
        schema: None,
        he_sections: Vec::new(),
        author: Some(author.to_string()),
        era: None,
        comp_date: None,
        version: None,
        commentary_on: on
            .iter()
            .map(|slug| BaseText {
                slug: (*slug).to_string(),
                mapping: Mapping::ManyToOne,
            })
            .collect(),
    }
}

fn dated(mut work: Work, era: &str, comp_date: &str) -> Work {
    work.era = Some(era.to_string());
    work.comp_date = Some(comp_date.to_string());
    work
}

fn base(slug: &str, author: &str) -> Work {
    Work {
        slug: slug.to_string(),
        author: Some(author.to_string()),
        ..commentary(slug, slug, slug, author, &[])
    }
}

#[test]
fn one_man_with_three_seforim_is_one_row() {
    let works = vec![
        dated(
            commentary(
                "rashi-on-berakhot",
                "רש\"י על ברכות",
                "Rashi on Berakhot",
                "רש״י",
                &["bavli/berakhot"],
            ),
            "RI",
            "c.1065  – c.1115 CE",
        ),
        dated(
            commentary(
                "rashi-on-shabbat",
                "רש\"י על שבת",
                "Rashi on Shabbat",
                "רש״י",
                &["bavli/shabbat"],
            ),
            "RI",
            "c.1065  – c.1115 CE",
        ),
        dated(
            commentary(
                "rashi-on-bereshit",
                "רש\"י על בראשית",
                "Rashi on Genesis",
                "רש״י",
                &["torah/genesis"],
            ),
            "RI",
            "c.1065  – c.1115 CE",
        ),
    ];
    let register = Register::of(&works);
    assert_eq!(register.people().len(), 1);
    let rashi = &register.people()[0];
    assert_eq!(rashi.name, "רש״י");
    assert_eq!(rashi.works.len(), 3);
    assert_eq!(
        rashi.on,
        ["bavli/berakhot", "bavli/shabbat", "torah/genesis"]
    );
    assert_eq!(rashi.en_name.as_deref(), Some("Rashi"));
    assert_eq!(rashi.era_spoken(), Some("ראשונים"));
    assert_eq!(rashi.written().as_deref(), Some("1065–1115"));
}

#[test]
fn two_spellings_of_one_name_are_one_row_and_two_men_stay_two() {
    // The corpus writes both because Sefaria and Otzaria punctuate differently.
    // `girsa_hebrew::normalize` is the rule that says they are the same string —
    // the same rule the index is built under, and not a fuzzy match.
    let works = vec![
        commentary(
            "rambam-a",
            "פירוש המשנה לרמב״ם",
            "Rambam on the Mishnah",
            "רמב״ם",
            &["mishnah/berakhot"],
        ),
        commentary(
            "rambam-b",
            "פירוש המשנה לרמב\"ם",
            "Rambam on the Mishnah",
            "רמב\"ם",
            &["mishnah/peah"],
        ),
        commentary(
            "ramban-a",
            "חידושי הרמב״ן",
            "Ramban on Berakhot",
            "רמב״ן",
            &["bavli/berakhot"],
        ),
    ];
    let register = Register::of(&works);
    assert_eq!(register.people().len(), 2, "{:?}", register.people());
    // The most frequent spelling is the name; the other is kept, because a
    // register that showed one spelling and dropped the other would make the
    // reader who searched for the second think he was not there.
    let rambam = register.find("רמב״ם").expect("the Rambam");
    assert_eq!(rambam.works.len(), 2);
    assert!(
        rambam.name == "רמב״ם" || rambam.name == "רמב\"ם",
        "{}",
        rambam.name
    );
    assert_eq!(1, rambam.also_called.len());
    assert_eq!(Some(&rambam.key), register.find("רמב\"ם").map(|p| &p.key));
    // And the Ramban is not the Rambam.
    assert_ne!(rambam.key, register.find("רמב״ן").expect("the Ramban").key);
}

#[test]
fn a_name_the_corpus_does_not_hold_is_an_answer_and_not_an_empty_profile() {
    let works = vec![commentary(
        "rashi-on-berakhot",
        "רש\"י על ברכות",
        "Rashi on Berakhot",
        "רש״י",
        &["bavli/berakhot"],
    )];
    let register = Register::of(&works);
    assert!(register.find("אבן עזרא").is_none());
    assert!(register.find("").is_none());
}

#[test]
fn the_order_is_by_year_then_by_era_then_by_name_and_never_by_guess() {
    let works = vec![
        // Undated, and last however it is spelled.
        commentary(
            "unknown-a",
            "פירוש",
            "A commentary",
            "אלמוני",
            &["bavli/berakhot"],
        ),
        commentary(
            "unknown-b",
            "פירוש",
            "A commentary",
            "בנימין",
            &["bavli/shabbat"],
        ),
        // Era only: a century-wide bucket, so it sorts after everybody whose
        // years are known and before everybody whose are not.
        dated(
            commentary(
                "geonim",
                "שאילתות",
                "Sheiltot",
                "אחד מהגאונים",
                &["bavli/berakhot"],
            ),
            "GN",
            "",
        ),
        dated(
            commentary(
                "later",
                "בית יוסף",
                "Beit Yosef",
                "רבי יוסף קארו",
                &["shulchan-arukh/orach-chayim"],
            ),
            "AH",
            "1488  – 1575 CE",
        ),
        dated(
            commentary(
                "earlier",
                "רש\"י על ברכות",
                "Rashi on Berakhot",
                "רש״י",
                &["bavli/berakhot"],
            ),
            "RI",
            "c.1065  – c.1115 CE",
        ),
    ];
    let register = Register::of(&works);
    let order: Vec<&str> = register
        .chronological()
        .iter()
        .map(|person| person.key.as_str())
        .collect();
    assert_eq!(
        order,
        vec![
            girsa_hebrew::normalize("רש״י"),
            girsa_hebrew::normalize("רבי יוסף קארו"),
            // The era-only row, in the era it names and not in a year.
            girsa_hebrew::normalize("אחד מהגאונים"),
            // Undated, by name — putting them earlier would be a claim.
            girsa_hebrew::normalize("אלמוני"),
            girsa_hebrew::normalize("בנימין"),
        ]
    );
}

#[test]
fn a_title_that_doubts_its_own_author_carries_the_doubt() {
    // The three works in this corpus whose titles say so. The register keeps both
    // halves — who the catalogue filed it under, and what the title claims — and
    // decides neither.
    let works = vec![
        commentary(
            "teshuvot-harashba-meyuchas-leharamban",
            "תשובות הרשב\"א מיוחס להרמב\"ן",
            "Responsa of the Rashba attributed to the Ramban",
            "רשב\"א",
            &["bavli/berakhot"],
        ),
        commentary(
            "chiddushei-haran",
            "חידושי הרן (מיוחס לו) על שבת",
            "Chiddushei HaRan on Shabbat",
            "הרן",
            &["bavli/shabbat"],
        ),
        commentary(
            "plain",
            "רש\"י על ברכות",
            "Rashi on Berakhot",
            "רש״י",
            &["bavli/berakhot"],
        ),
    ];
    let register = Register::of(&works);
    let rashba = register.find("רשב\"א").expect("the Rashba");
    assert!(rashba.is_doubted());
    // As written, gershayim and all — the title in the corpus uses a plain
    // ASCII quote, and a register that rewrote it would be quoting itself.
    assert_eq!(
        rashba.works[0].attribution,
        Attribution::Doubted {
            said: "מיוחס להרמב\"ן".to_string(),
            to: Some("הרמב\"ן".to_string()),
        }
    );
    // `מיוחס לו` names nobody, and a name invented for it would be this module
    // deciding an identity the title did not state.
    let ran = register.find("הרן").expect("the Ran");
    assert_eq!(
        ran.works[0].attribution,
        Attribution::Doubted {
            said: "מיוחס לו".to_string(),
            to: None,
        }
    );
    assert!(!register.find("רש״י").expect("Rashi").is_doubted());
}

#[test]
fn who_comments_on_a_sefer_is_asked_by_slug() {
    let works = vec![
        commentary(
            "rashi-on-berakhot",
            "רש\"י על ברכות",
            "Rashi on Berakhot",
            "רש״י",
            &["bavli/berakhot"],
        ),
        commentary(
            "tosafot-on-berakhot",
            "תוספות על ברכות",
            "Tosafot on Berakhot",
            "בעלי התוספות",
            &["bavli/berakhot"],
        ),
        commentary(
            "rashi-on-shabbat",
            "רש\"י על שבת",
            "Rashi on Shabbat",
            "רש״י",
            &["bavli/shabbat"],
        ),
    ];
    let register = Register::of(&works);
    let on_berakhot = register.on("bavli/berakhot");
    assert_eq!(on_berakhot.len(), 2, "{on_berakhot:?}");
    // Rashi is one row here as well: the same man on two seforim is one person
    // who appears in both answers, not two.
    assert_eq!(register.on("bavli/shabbat").len(), 1);
    assert!(register.on("bavli/eruvin").is_empty());
}

#[test]
fn a_base_text_author_is_not_a_commentator_and_an_unattributed_commentary_is_counted() {
    let works = vec![
        // The Mishneh Torah's author wrote a sefer and commented on nothing.
        base("mishneh-torah", "רמב״ם"),
        // A commentary with no author on it: in the corpus, not in a row.
        Work {
            author: None,
            ..commentary(
                "anonymous-on-berakhot",
                "פירוש על ברכות",
                "Anonymous on Berakhot",
                "x",
                &["bavli/berakhot"],
            )
        },
    ];
    let register = Register::of(&works);
    assert!(register.people().is_empty(), "{:?}", register.people());
    assert_eq!(register.anonymous(), 1);
    // And per sefer, because *nobody is named here* is what the list of
    // commentators on one sefer has to be able to say.
    assert_eq!(register.anonymous_on("bavli/berakhot"), 1);
    assert_eq!(register.anonymous_on("bavli/shabbat"), 0);
}

#[test]
fn the_real_catalogue_answers_the_two_acceptance_questions() {
    // The acceptance: *given "Rashi" → entity with era, corpus coverage,
    // canonical names, attribution*; *given a base sefer → the commentators on
    // it*. It is run on the catalogue on disk where there is one, because the
    // fixture answers what a fixture was written to answer — and the register's
    // whole subject is a real corpus.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/works/index.jsonl");
    let Ok(text) = std::fs::read_to_string(&root) else {
        eprintln!("no catalogue at {} — nothing to read", root.display());
        return;
    };
    let works: Vec<Work> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert!(works.len() > 1_000, "the catalogue is {}", works.len());
    let register = Register::of(&works);
    let rashi = register.find("רש״י").expect("Rashi in the catalogue");
    assert_eq!(rashi.era_spoken(), Some("ראשונים"), "{}", rashi.name);
    // The span is his life, not one sefer's: the earliest start and the latest
    // end across his works. 1060–1115 in this catalogue — 1040–1120 is the
    // assertion, because pinning the digits would make this test a hostage to
    // the next re-import (which is what the window's numbers guard says about
    // its own pins).
    let years = rashi.written().expect("years for Rashi");
    let (from, to) = years
        .split_once('–')
        .map(|(from, to)| (from.parse::<i32>(), to.parse::<i32>()))
        .map(|(from, to)| (from.expect("a year"), to.expect("a year")))
        .unwrap_or_else(|| panic!("not a span: {years}"));
    assert!((1040..=1070).contains(&from), "{years}");
    assert!((1100..=1120).contains(&to), "{years}");
    assert!(
        rashi.on.iter().any(|base| base == "bavli/berakhot"),
        "{:?}",
        rashi.on
    );
    assert!(rashi.works.len() > 10, "{} works", rashi.works.len());
    assert!(
        register.on("bavli/berakhot").len() > 1,
        "nobody comments on Berakhot"
    );
    // And the ordering is a chronology: wherever two people both have years, the
    // earlier one stands first.
    let dated: Vec<(i32, i32)> = register
        .chronological()
        .iter()
        .filter_map(|person| person.years)
        .collect();
    assert!(
        dated.windows(2).all(|pair| pair[0].0 <= pair[1].0),
        "{dated:?}"
    );
}
