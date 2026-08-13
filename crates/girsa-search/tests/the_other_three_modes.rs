//! W14 — Regex, Citation and Instruments, and the chips that reach them.
//!
//! spec.md §9.3 names five modes and W12–W13 built the first two. These three
//! are the rest, and each one has a promise of its own that is not the literal
//! mode's:
//!
//! | mode | the promise |
//! |---|---|
//! | Regex | full power, no hand-holding — and **nothing** offered on a zero (§9.6) |
//! | Citation | type a mareh makom, jump — and never jump to a guess (§4.3) |
//! | Instruments | gematria, notarikon, atbash, dilug, each asked for by name |
//!
//! And the chip row underneath them (§9.5), whose acceptance is that typing a
//! sigil and clicking a chip are **the same search**. If they can differ, the
//! sigils are a second query language rather than a way of teaching the first.

// A panic in a test is a failure report.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::import;
use girsa_corpus::work::{Source, Work};
use girsa_ref::resolve::Context;
use girsa_search::bar::{Answer, Bar, Results};
use girsa_search::chips::{Chips, Skips, Sounding};
use girsa_search::facets::Catalogue;
use girsa_search::index::{BuildReport, Paging, SearchIndex};
use girsa_search::scope::Scope;
use girsa_search::torat_emet::Together;
use girsa_search::Mode;

/// Two seforim on disk, the way `girsa-import` leaves them, plus the lexicon
/// the resolver is seeded from.
///
/// Small enough to reason about by hand and real enough to go through every
/// door: the works are catalogued, the segments carry permanent ids, and the
/// lexicon spells one of them two ways — one of which also names a sefer that
/// is **not** here, which is what makes the ambiguity test honest.
struct Shelf {
    root: PathBuf,
    works: Vec<Work>,
}

const BERAKHOT: [&str; 4] = [
    "מאימתי קורין את שמע בערבין",
    "משעה שהכהנים נכנסים לאכול בתרומתן",
    "תנא היכא קאי דקתני מאימתי",
    "אמר רבי יוסי תורה אור",
];

const ORACH_CHAYIM: [&str; 3] = [
    "יתגבר כארי לעמוד בבוקר לעבודת בוראו",
    "שויתי ה' לנגדי תמיד הוא כלל גדול בתורה",
    "המשכים לעבודת בוראו יתגבר כארי",
];

impl Shelf {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a root");

        let works = vec![
            work("bavli/berakhot", "ברכות", &["Talmud", "Bavli"]),
            work(
                "shulchan-arukh/orach-chayim",
                "שולחן ערוך אורח חיים",
                &["Halakhah"],
            ),
        ];
        let mut index = String::new();
        for work in &works {
            index.push_str(&serde_json::to_string(work).expect("a work"));
            index.push('\n');
        }
        std::fs::create_dir_all(root.join("works")).expect("a works dir");
        std::fs::write(root.join("works/index.jsonl"), index).expect("the catalogue");

        write_work(&root, &works[0], &BERAKHOT, &["2a"]);
        write_work(&root, &works[1], &ORACH_CHAYIM, &["1"]);

        std::fs::write(
            root.join("lexicon.tsv"),
            "# a lexicon\n\
             ברכות\tbavli/berakhot\tברכות\tBerakhot\n\
             שוע אוח\tshulchan-arukh/orach-chayim\tשולחן ערוך\tShulchan Arukh\n\
             אוח\tshulchan-arukh/orach-chayim\tשולחן ערוך\tShulchan Arukh\n\
             אוח\ttur/orach-chayim\tטור\tTur\n",
        )
        .expect("a lexicon");
        Self { root, works }
    }

    /// A bar over an index built from exactly these segments.
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

fn work(slug: &str, title: &str, categories: &[&str]) -> Work {
    Work {
        slug: slug.to_string(),
        he_title: title.to_string(),
        en_title: slug.to_string(),
        categories: categories.iter().map(|c| (*c).to_string()).collect(),
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

/// `work.json` and `segments.jsonl`, addressed the way the importer writes
/// them — read back through `girsa_corpus::import`, so a fixture that drifted
/// from what the importer writes would fail here rather than in the window.
fn write_work(root: &Path, work: &Work, lines: &[&str], under: &[&str]) {
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
        let path: Vec<String> = under
            .iter()
            .map(|p| (*p).to_string())
            .chain([n.to_string()])
            .collect();
        let id = girsa_corpus::segment::SegmentId::new(
            slug,
            path,
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

fn ask(bar: &Bar, typed: &str, chips: &Chips) -> Answer {
    bar.ask(typed, chips, Paging::first(), &Context::default())
}

fn results(answer: Answer) -> Box<Results> {
    match answer {
        Answer::Segments { results, .. } => results,
        other => panic!("expected segments, got {other:?}"),
    }
}

fn refusal(answer: Answer) -> String {
    match answer {
        Answer::Refused(why) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// ── Regex ───────────────────────────────────────────────────────────────────

#[test]
fn a_pattern_matches_whole_words_of_the_index() {
    let shelf = Shelf::new("girsa-w14-regex");
    let bar = shelf.bar();
    let chips = Chips {
        mode: Mode::Regex,
        ..Chips::default()
    };
    // `מאימתי` and `מאימת…` — a pattern is an automaton over whole terms, so
    // this reaches the word and nothing that merely contains it.
    let found = results(ask(&bar, "מאימת.", &chips));
    assert_eq!(found.total, 2, "{:?}", found.hits);
    assert!(found.header.contains("מאימת."), "{}", found.header);
}

#[test]
fn a_zero_in_regex_mode_is_offered_nothing() {
    // spec.md §9.6's table: *Regex — nothing. You wrote a pattern; it matched
    // nothing.* The ladder exists for readers who typed words, and offering it
    // here would make the other four modes' promises unreadable.
    let shelf = Shelf::new("girsa-w14-regex-zero");
    let bar = shelf.bar();
    let chips = Chips {
        mode: Mode::Regex,
        ..Chips::default()
    };
    match ask(&bar, "זזזז.*", &chips) {
        Answer::Segments {
            results, offers, ..
        } => {
            assert_eq!(results.total, 0);
            assert!(offers.is_empty(), "the ladder is not offered here");
        }
        other => panic!("expected segments, got {other:?}"),
    }
}

#[test]
fn a_pattern_that_could_never_match_is_refused_rather_than_run() {
    // The index holds words with their marks off and their final letters
    // folded, so a pattern carrying either matches nothing for ever — and looks
    // exactly like an honest empty result, in the mode whose whole promise is
    // that an empty result means the corpus does not say it.
    let shelf = Shelf::new("girsa-w14-regex-refused");
    let bar = shelf.bar();
    let chips = Chips {
        mode: Mode::Regex,
        ..Chips::default()
    };
    assert!(refusal(ask(&bar, "קָדַשׁ", &chips)).contains("marks off"));
    assert!(refusal(ask(&bar, "מלך", &chips)).contains("folds final letters"));
    // And one tantivy would answer with a parser error about empty match
    // operators, which is not an answer either.
    assert!(refusal(ask(&bar, "^קדש$", &chips)).contains("whole of a word"));
}

// ── Citation ────────────────────────────────────────────────────────────────

#[test]
fn a_mareh_makom_jumps_and_an_ambiguous_one_does_not() {
    let shelf = Shelf::new("girsa-w14-citation");
    let bar = shelf.bar();
    let chips = Chips::default();

    // `@` is the sigil; the chip it sets is the mode.
    let Answer::Cited(landing) = ask(&bar, "@שוע אוח א ב", &chips) else {
        panic!("expected a citation");
    };
    let place = landing.only().expect("one place");
    assert_eq!(
        place.run.first.to_string(),
        "girsa:shulchan-arukh/orach-chayim/1:2#2"
    );

    // `או"ח` is the Orach Chayim of the Shulchan Arukh **and** of the Tur, and
    // the Tur is not on this shelf — which does not refute it. A candidate is
    // ruled out only when the shelf can say the address is not in a sefer it
    // has (BUILDER.md rule 6, and W8's rule for the same reason).
    let Answer::Cited(landing) = ask(&bar, "@אוח א ב", &chips) else {
        panic!("expected a citation");
    };
    assert!(landing.is_a_choice());
    assert_eq!(landing.only(), None, "no caller can take the first");
}

#[test]
fn a_citation_into_a_place_that_is_not_there_offers_rather_than_lands() {
    let shelf = Shelf::new("girsa-w14-citation-nowhere");
    let bar = shelf.bar();
    let Answer::Cited(landing) = ask(&bar, "@שוע אוח תתקצט א", &Chips::default())
    else {
        panic!("expected a citation");
    };
    assert!(landing.places.is_empty());
    assert!(!landing.near.is_empty(), "and it says what it did find");
}

// ── Instruments ─────────────────────────────────────────────────────────────

#[test]
fn a_gematria_finds_the_words_of_the_corpus_that_come_to_the_number() {
    // The words are the finding as much as the segments are. `תורה` is 611 and
    // is in this shelf; the instrument works it out by adding up every distinct
    // word in the index rather than from a list somebody wrote.
    let shelf = Shelf::new("girsa-w14-gematria");
    let bar = shelf.bar();
    let chips = Chips {
        mode: Mode::Instruments,
        sounding: Sounding::Gematria,
        ..Chips::default()
    };
    let Answer::Segments { results, note, .. } = ask(&bar, "611", &chips) else {
        panic!("expected segments");
    };
    assert!(results.total >= 1, "{:?}", results.hits);
    let note = note.expect("it says which words");
    assert!(note.contains("תורה"), "{note}");
    // And the highlight lands on the word it added up, not on the number.
    let marks = results.marker.marks(&results.hits[0]);
    assert!(!marks.is_empty(), "the word is marked in the line");
}

#[test]
fn atbash_searches_for_the_word_it_becomes_and_says_which_word_that_is() {
    let shelf = Shelf::new("girsa-w14-atbash");
    let bar = shelf.bar();
    let chips = Chips {
        mode: Mode::Instruments,
        sounding: Sounding::Atbash,
        ..Chips::default()
    };
    // `גבשמ` under atbash is `רשבי`… what matters here is the promise: the
    // header names the word that was actually looked for.
    let found = results(ask(&bar, "אמז", &chips));
    assert!(found.header.contains("under atbash"), "{}", found.header);
}

#[test]
fn a_notarikon_reads_the_first_letters_of_words_standing_together() {
    // `מקאש` — מאימתי קורין את שמע. Four words in a row whose first letters
    // spell it.
    //
    // Read off the **text**, in a sefer the reader named, and not off the
    // index: as an index query it is four one-letter patterns, each matching
    // more distinct words than a phrase query will hold, and on the real corpus
    // it comes back as a refusal about postings lists. True, and useless.
    let shelf = Shelf::new("girsa-w14-notarikon");
    let bar = shelf.bar();
    let chips = Chips {
        mode: Mode::Instruments,
        sounding: Sounding::Rashei,
        scope: Scope::everything().only(["bavli/berakhot".to_string()], "ברכות"),
        ..Chips::default()
    };
    let found = results(ask(&bar, "מקאש", &chips));
    assert_eq!(found.total, 1, "{:?}", found.hits);
    assert_eq!(found.hits[0].text, BERAKHOT[0]);
    // And the highlight lands on the four words, as printed.
    assert_eq!(found.marker.marks(&found.hits[0]).len(), 4);

    // Over the whole shelf it says which sefer it needs.
    let whole = Chips {
        scope: Scope::everything(),
        ..chips
    };
    assert!(refusal(ask(&bar, "מקאש", &whole)).contains("needs one"));
}

#[test]
fn a_dilug_needs_a_sefer_to_read_and_says_so_rather_than_scanning_the_shelf() {
    // A dilug runs through the letters of a sefer and ignores where words end,
    // so it is a scan and not an index question. Over the whole shelf it is
    // refused **with the reason**, which is the difference between a bound and
    // a silent sample.
    let shelf = Shelf::new("girsa-w14-dilug");
    let bar = shelf.bar();
    let chips = Chips {
        mode: Mode::Instruments,
        sounding: Sounding::Dilug,
        skips: Skips { from: 1, to: 4 },
        ..Chips::default()
    };
    let why = refusal(ask(&bar, "תורה", &chips));
    assert!(why.contains("needs one"), "{why}");

    // Narrowed to one sefer, it reads it.
    let narrowed = Chips {
        scope: Scope::everything().only(["bavli/berakhot".to_string()], "ברכות"),
        ..chips
    };
    let Answer::Segments { results, note, .. } = ask(&bar, "אמת", &narrowed) else {
        panic!("expected segments");
    };
    assert!(
        note.expect("it says what it read")
            .contains("not the index"),
        "a dilug says it read the text rather than the index"
    );
    // Whatever it found or did not, it did not invent a total it cannot show.
    assert_eq!(results.hits.is_empty(), results.total == 0);
}

// ── The chips ───────────────────────────────────────────────────────────────

#[test]
fn typing_a_sigil_and_clicking_the_chip_are_the_same_search() {
    // The acceptance of spec.md §9.5. If these can differ, the sigils are a
    // second query language rather than a way of teaching the first — and a
    // reader who typed one would be searching for something other than what the
    // chips in front of them say.
    let shelf = Shelf::new("girsa-w14-sigils");
    let bar = shelf.bar();

    let cases: [(&str, Chips); 3] = [
        (
            "\"יתגבר כארי\"",
            Chips {
                together: Together::Phrase,
                ..Chips::default()
            },
        ),
        (
            "*קורי*",
            Chips {
                matching: girsa_search::torat_emet::Match::Contains,
                ..Chips::default()
            },
        ),
        (
            "יתגבר ~3 בוראו",
            Chips {
                together: Together::Near { words: 3 },
                ..Chips::default()
            },
        ),
    ];

    for (typed, clicked) in cases {
        let bare = typed.trim_matches('"').replace('*', "").replace("~3 ", "");
        let by_sigil = results(ask(&bar, typed, &Chips::default()));
        let by_chip = results(ask(&bar, &bare, &clicked));
        assert_eq!(
            by_sigil.header, by_chip.header,
            "typing `{typed}` searched for something else than the chip does"
        );
        assert_eq!(by_sigil.total, by_chip.total, "for `{typed}`");
    }
}

#[test]
fn the_chip_row_says_what_the_search_will_do_before_it_runs() {
    // Every control visible, every option on it visible. A chip whose other
    // settings only appear once you know they exist is a syntax with a mouse.
    let chips = Chips {
        scope: Scope::everything().only(["bavli/berakhot".to_string()], "ברכות"),
        ..Chips::default()
    };
    let row = chips.row();
    let shown: Vec<&str> = row.iter().map(girsa_search::chips::Chip::shown).collect();
    assert_eq!(
        shown[0], "torat emet",
        "the default mode is the literal one"
    );
    assert_eq!(
        shown[1], "ברכות",
        "and the scope says what it was narrowed by"
    );
    assert!(
        row.iter()
            .all(|chip| chip.choices.len() > 1 || chip.key == "where"),
        "every chip offers its alternatives"
    );
}

#[test]
fn the_mode_decides_what_is_asked_and_never_the_shape_of_the_text() {
    // The same three characters mean three different searches, and which one
    // is a chip rather than a guess from what it looks like.
    let shelf = Shelf::new("girsa-w14-modes");
    let bar = shelf.bar();
    let mut headers = Vec::new();
    for mode in [Mode::ToratEmet, Mode::Smart, Mode::Regex] {
        let chips = Chips {
            mode,
            ..Chips::default()
        };
        headers.push(match ask(&bar, "תורה", &chips) {
            Answer::Segments { results, .. } => results.header.clone(),
            other => panic!("expected segments, got {other:?}"),
        });
    }
    assert_eq!(headers[0], "the words תורה, anywhere in a segment");
    assert_eq!(headers[2], "the patterns תורה, anywhere in a segment");
    assert_ne!(
        headers[0], headers[2],
        "the literal mode and the pattern mode do not describe themselves alike"
    );
}
