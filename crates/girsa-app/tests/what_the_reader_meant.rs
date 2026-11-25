// What the reader's name for a sefer resolves to, on a shelf that has two
// seforim called Berakhot and one of them a Mishnah.
//
// The unit tests beside `meant` check the arithmetic — how far two spellings are
// and how the words of a name are allowed to differ. This file checks the thing
// the arithmetic is for: that the names a reader actually types reach the sefer
// they meant, that a name which fits two seforim comes back as a question, and
// that a name which fits none is refused instead of being matched to the nearest
// thing on the shelf.
//
// That last one is the whole of the design. A navigation box that always
// resolves is a box that opens the wrong masechet with no sign it guessed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use girsa_app::meant::{meant, Meant};
use girsa_corpus::work::{Source, Work};

/// The shelf: one masechet, the same masechet as a Mishnah, the Shulchan Arukh,
/// and a commentary whose name contains the masechet it comments on.
///
/// The commentary is the trap. `berakhot` is in three of these slugs, and a name
/// resolver that matched on *contains* would answer a question about Rashi's
/// commentary to a reader who typed the name of the Gemara.
fn shelf() -> Vec<Work> {
    let works = vec![
        work("bavli/berakhot", "ברכות"),
        work("mishnah-berakhot", "משנה ברכות"),
        work("bavli/rashi-on-berakhot", "רש\"י על ברכות"),
        work("shulchan-arukh/orach-chayim", "שולחן ערוך אורח חיים"),
        work("bavli/bava-metzia", "בבא מציעא"),
        work("bavli/bava-kamma", "בבא קמא"),
        work("genesis", "בראשית"),
    ];
    works
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

fn one(works: &[Work], typed: &str) -> String {
    match meant(works, typed) {
        Meant::One(slug) => slug,
        other => panic!("{typed} should be one sefer, got {other:?}"),
    }
}

#[test]
fn the_three_ways_the_issue_spells_berakhot_all_reach_berakhot() {
    // The table in #63: the English spelling, the Hebrew one, and the phonetic
    // one. The Hebrew is folded — nikud would go, and so do the gershayim.
    let catalogue = shelf();
    for typed in ["berachos", "brachos", "berachot", "ברכות", "berakhot"] {
        assert_eq!(one(&catalogue, typed), "bavli/berakhot", "{typed}");
    }

    // And the location is not part of the name, so the caller hands this module
    // the name alone. What it does with a whole sentence is still an answer, not
    // a crash: the trailing words are tried, and the sefer still comes back.
    assert_eq!(one(&catalogue, "berachos 2a"), "bavli/berakhot");
}

#[test]
fn a_long_name_with_the_shelf_in_front_of_it_still_resolves() {
    // `shulchan aruch orech chaim` is three words the reader knows and one slug
    // the shelf knows, spelled differently in every one of them: aruch/arukh,
    // orech/orach, chaim/chayim.
    let catalogue = shelf();
    for typed in [
        "shulchan aruch orech chaim",
        "shulchan arukh orach chayim",
        "שולחן ערוך אורח חיים",
        "שולחן ערוך",
    ] {
        assert_eq!(
            one(&catalogue, typed),
            "shulchan-arukh/orach-chayim",
            "{typed}"
        );
    }
}

#[test]
fn two_seforim_called_berakhot_is_a_question_and_not_a_choice() {
    // `ברכות` is the Gemara and the Mishnah is `משנה ברכות`, so the plain name is
    // one sefer. `ברכות` typed by somebody looking for the Mishnah is the
    // reader's problem, not this module's.
    let catalogue = shelf();
    assert_eq!(one(&catalogue, "ברכות"), "bavli/berakhot");

    // `berakhot` alone is the same answer. The commentary is *not* tied with it:
    // its own name is `rashi on berakhot`, and a name that differs by two words
    // is not the same name.
    let resolved = meant(&catalogue, "berakhot");
    match resolved {
        Meant::One(slug) => assert_eq!(slug, "bavli/berakhot"),
        Meant::Several(slugs) => assert!(
            !slugs.iter().any(|s| s.contains("rashi")),
            "a commentary is not a tie with the sefer it comments on: {slugs:?}"
        ),
        Meant::None => panic!("berakhot is on this shelf"),
    }
}

#[test]
fn a_name_two_seforim_share_is_returned_as_a_question() {
    // The issue's rule 7, and the reason this returns an enum rather than a
    // slug: `bava` is the whole first word of two masechtot, and the honest
    // answer to it is *which do you mean* — never whichever one sorted first.
    let catalogue = shelf();
    assert_eq!(
        meant(&catalogue, "bava"),
        Meant::Several(vec![
            "bavli/bava-kamma".to_string(),
            "bavli/bava-metzia".to_string()
        ])
    );
    // Sorted, so the same name is the same list every time it is typed.
    assert_eq!(meant(&catalogue, "bava"), meant(&catalogue, "bava"));
}

#[test]
fn two_masechtot_that_share_a_word_are_never_confused_for_each_other() {
    // `bava metzia` and `bava kamma` share their first word and nothing else.
    // The distance between the words that tell them apart is what decides, which
    // is why the per-word comparison takes the worst word and not the average.
    let catalogue = shelf();
    assert_eq!(one(&catalogue, "bava metzia"), "bavli/bava-metzia");
    assert_eq!(one(&catalogue, "bava kamma"), "bavli/bava-kamma");
    assert_eq!(one(&catalogue, "בבא קמא"), "bavli/bava-kamma");
    assert_eq!(one(&catalogue, "בבא מציעא"), "bavli/bava-metzia");
}

#[test]
fn a_name_that_is_on_no_sefer_comes_back_as_none_and_not_as_the_nearest() {
    // The failure this module exists to prevent. `genesis` is on the shelf and
    // `qeualqwe` is four letters from nothing; a resolver that returned its best
    // guess would open Bereshit for a typo.
    let catalogue = shelf();
    assert_eq!(meant(&catalogue, "qeualqwe"), Meant::None);
    assert_eq!(meant(&catalogue, "nothing like this"), Meant::None);
    // A name too short to hold a difference: `ב` is one letter and one edit from
    // every other letter.
    assert_eq!(meant(&catalogue, "ב"), Meant::None);
    assert_eq!(meant(&catalogue, ""), Meant::None);
    // And a sefer that is on the shelf is still found, so the refusals above are
    // not the resolver being broken.
    assert_eq!(one(&catalogue, "genesis"), "genesis");
    assert_eq!(one(&catalogue, "בראשית"), "genesis");
}

#[test]
fn every_slug_on_the_shelf_resolves_to_itself() {
    // The one property that has to hold for every sefer and not just the ones a
    // test happens to name: pasting a slug — which is what `girsa:` links carry
    // and what the command line prints — finds that sefer. Not *one* answer: a
    // slug that is also the tail of another slug is legitimately a question, so
    // the requirement is that the sefer is among the answers.
    let catalogue = shelf();
    for slug in [
        "bavli/berakhot",
        "mishnah-berakhot",
        "bavli/rashi-on-berakhot",
        "shulchan-arukh/orach-chayim",
        "bavli/bava-metzia",
        "bavli/bava-kamma",
        "genesis",
    ] {
        match meant(&catalogue, slug) {
            Meant::One(found) => assert_eq!(found, slug),
            Meant::Several(found) => assert!(found.iter().any(|s| s == slug), "{slug}: {found:?}"),
            Meant::None => panic!("{slug} does not resolve to itself"),
        }
    }
}
