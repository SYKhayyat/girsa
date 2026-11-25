//! A name the lexicon does not hold, and the question that is owed instead.
//!
//! `girsa_app::nav::read` answers from the lexicon and stops there, on purpose:
//! #69 is the lexicon's gap, and papering it over with a fuzzy match would be the
//! near-miss sefer the whole address system is arranged against.
//!
//! But *the lexicon does not hold that spelling* and *there is nothing to say* are
//! different sentences, and the difference is a reader who typed `brachos` and got
//! silence about a sefer they can see on their own shelf. `suggest` is the layer
//! that adds the second sentence — **as a question**. What is asserted here is
//! mostly the boundary: the shelf's opinion is consulted only when the lexicon had
//! nothing, and it never becomes a landing by itself.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use girsa_app::meant::Meant;
use girsa_app::nav::{read, suggest, Nav, Suggest};
use girsa_corpus::work::{Source, Work};
use girsa_ref::Lexicon;

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

/// The shelf, and a lexicon holding exactly the names a reader might type.
fn shelf() -> (Vec<Work>, Lexicon) {
    let berakhot = work("bavli/berakhot", "ברכות");
    let mishnayos = work("mishnah-berakhot", "משנה ברכות");
    let metzia = work("bavli/bava-metzia", "בבא מציעא");
    let kamma = work("bavli/bava-kamma", "בבא קמא");
    let works = vec![berakhot.clone(), mishnayos.clone(), metzia.clone(), kamma];

    // The lexicon's own `Work` and not the corpus's: a lexicon entry is a slug and
    // two names, which is deliberately less than a `Work` — see `girsa_ref`.
    let mut lexicon = Lexicon::default();
    for (work, variants) in [
        (&works[0], vec!["ברכות", "Berachos"]),
        (&works[1], vec!["משנה ברכות", "Mishnayos Berachos"]),
        (&works[2], vec!["בבא מציעא", "Bava Metzia"]),
        (&works[3], vec!["בבא קמא", "Bava Kamma"]),
    ] {
        lexicon.add(
            girsa_ref::Work {
                slug: work.slug.clone(),
                he_title: work.he_title.clone(),
                en_title: work.en_title.clone(),
            },
            &variants,
        );
    }
    (works, lexicon)
}

#[test]
fn a_name_the_lexicon_reads_is_not_second_guessed_by_a_nearer_spelling() {
    // The rule the whole design rests on. `Berachos` is a lexicon variant and it
    // resolves; `brachos` — one vowel away — is not, and the two must not produce
    // the same kind of answer, or the window could stop showing which is which.
    let (works, lexicon) = shelf();
    assert!(
        matches!(
            suggest(&lexicon, &works, "Berachos 2a"),
            Suggest::Read(Nav::Go(_))
        ),
        "a name the lexicon holds resolves, and `suggest` says nothing extra"
    );
    assert!(
        !matches!(
            suggest(&lexicon, &works, "brachos 2a"),
            Suggest::Read(Nav::Go(_))
        ),
        "a spelling the lexicon does not hold is not turned into a landing"
    );
}

#[test]
fn a_name_the_lexicon_does_not_hold_comes_back_as_a_question_naming_the_sefer() {
    let (works, lexicon) = shelf();
    let said = suggest(&lexicon, &works, "brachos 2a");
    match said {
        Suggest::DidYouMean {
            typed,
            slug,
            title,
            corrected,
        } => {
            assert_eq!(typed, "brachos 2a", "the question can quote what was typed");
            assert_eq!(slug, "bavli/berakhot");
            assert_eq!(title, "ברכות", "and it offers the sefer's own name");
            assert_eq!(
                corrected, "ברכות 2a",
                "and their own address is kept, so accepting it does not lose the 2a"
            );
        }
        other => panic!("brachos should be a question about Berakhot, got {other:?}"),
    }
}

#[test]
fn a_collection_word_in_front_survives_the_correction() {
    // The name read from the back, which is the `mishnayos peah` shape: the word
    // in front says which shelf it came from and is the reader's, so the corrected
    // line keeps it and replaces only the sefer.
    let (works, lexicon) = shelf();
    match suggest(&lexicon, &works, "תלמוד ברכש") {
        Suggest::DidYouMean {
            corrected, title, ..
        } => {
            assert_eq!(title, "ברכות");
            assert_eq!(corrected, "תלמוד ברכות", "{corrected}");
        }
        other => panic!("a typo at the end of the line is still that sefer, got {other:?}"),
    }
}

#[test]
fn two_seforim_the_name_fits_are_a_choice_and_not_the_first_of_them() {
    // `bava` is the first word of two masechtot. Returning one of them would be
    // the guess `nav` exists to refuse; the answer is both, in shelf order.
    let (works, lexicon) = shelf();
    match suggest(&lexicon, &works, "bava") {
        Suggest::Which { typed, candidates } => {
            assert_eq!(typed, "bava");
            assert_eq!(candidates.len(), 2, "{candidates:?}");
            assert_eq!(candidates[0].slug, "bavli/bava-kamma");
            assert_eq!(candidates[1].slug, "bavli/bava-metzia");
        }
        other => panic!("bava should be a choice of two, got {other:?}"),
    }
}

#[test]
fn a_bare_location_is_still_the_lexicons_question_and_not_a_name() {
    // `2a` names a place and no sefer. The shelf has no opinion about it, and
    // `suggest` must not invent one by matching `a` against every sefer.
    let (works, lexicon) = shelf();
    match suggest(&lexicon, &works, "2a") {
        Suggest::Read(Nav::Ask(question)) => {
            assert!(format!("{question:?}").contains("2a"), "{question:?}");
        }
        other => panic!("a bare location is `read`'s question, got {other:?}"),
    }
}

#[test]
fn nothing_close_to_anything_on_the_shelf_is_still_nothing() {
    let (works, lexicon) = shelf();
    assert_eq!(
        suggest(&lexicon, &works, "qeualqwe"),
        Suggest::Read(Nav::Nothing),
        "a typo four letters from no sefer is not a question about one"
    );
    // And `read` alone still answers nothing for the name that `suggest` turns
    // into a question — the two functions are not the same function.
    assert_eq!(read(&lexicon, "brachos 2a"), Nav::Nothing);
    assert!(matches!(meant_of(&works, "brachos 2a"), Meant::One(_)));
}

/// The name lane, called directly, so the last assertion is about it and not about
/// `suggest`.
fn meant_of(works: &[Work], typed: &str) -> Meant {
    girsa_app::meant::meant(works, typed)
}
