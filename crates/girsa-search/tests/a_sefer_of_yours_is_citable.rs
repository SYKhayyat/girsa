//! A sefer you put on the shelf yourself can be named, not only opened.
//!
//! spec.md §5 says your own material is first-class, and `girsa-corpus`'s
//! `your_own_seforim.rs` already asserts most of what that means: permanent
//! ids, a catalogue line, survival of a corpus rebuild. This is the piece that
//! was missing, and the one a reader meets fastest — **the resolver's
//! vocabulary came out of the corpus and only out of the corpus**, so a sefer
//! of yours could be opened by title, filed by title, and could not be cited by
//! name from the search bar, from linkify or from anything that sends a mareh
//! makom to the pen.
//!
//! Two halves, and passing the first without the second would be worse than
//! failing both: a lexicon that knows the title and a resolver that then reads
//! the corpus root for its text would answer *this sefer is not on the shelf*
//! about a sefer sitting on the shelf.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_ref::resolve::Context;
use girsa_search::citation::Citations;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A corpus with a lexicon and one sefer in it, so that the personal half is
/// being added to something rather than being the only thing there.
fn corpus(dir: &Path) -> PathBuf {
    let root = dir.join("corpus");
    std::fs::create_dir_all(&root).expect("a corpus root");
    std::fs::write(
        root.join("lexicon.tsv"),
        "ברכות\tbavli/berakhot\tברכות\tBerakhot\n",
    )
    .expect("a lexicon");
    root
}

/// Your layer, with a real sefer read in through the real importer.
fn personal_with(dir: &Path, file_name: &str, body: &str) -> (PathBuf, String) {
    let personal = dir.join("personal");
    let file = dir.join(file_name);
    std::fs::write(&file, body).expect("the file writes");
    let added = girsa_corpus::import::mine::add(&personal, &file, None).expect("it is added");
    (personal, added.work.slug)
}

#[test]
fn a_sefer_you_dropped_in_can_be_cited_by_the_name_you_gave_it() {
    let dir = scratch("girsa-citable-mine");
    let root = corpus(&dir);
    let (personal, slug) = personal_with(&dir, "חבורה על הסוגיא.txt", "ראשון\n\nשני\n\nשלישי\n");

    let bar = Citations::open(&root, Some(&personal)).expect("a lexicon");
    let landing = bar.look_up("חבורה על הסוגיא ב", &Context::default());

    // One place, and it is the second paragraph of your own sefer — which means
    // the title reached the lexicon *and* the segments were read from the root
    // that actually holds them.
    let place = landing
        .only()
        .expect("one place, not a choice and not a near miss");
    assert_eq!(place.run.first.work(), slug);
    assert_eq!(place.run.first.to_string(), format!("girsa:{slug}/2#2"));
}

#[test]
fn without_your_layer_the_same_citation_resolves_to_nothing() {
    // The other side of the same claim. `Citations::open(root, None)` is what
    // `girsa-index find` uses when no `--personal` was given, and it must not
    // quietly reach into a personal root nobody named.
    let dir = scratch("girsa-citable-corpus-only");
    let root = corpus(&dir);
    let (_personal, _slug) = personal_with(&dir, "חבורה על הסוגיא.txt", "ראשון\n\nשני\n\nשלישי\n");

    let bar = Citations::open(&root, None).expect("a lexicon");
    let landing = bar.look_up("חבורה על הסוגיא ב", &Context::default());
    assert!(
        landing.only().is_none() && landing.places.is_empty(),
        "the corpus-only resolver has never heard of it"
    );
}

#[test]
fn your_title_and_a_masechta_of_the_same_name_are_a_choice_and_not_a_pick() {
    // BUILDER.md rule 6, applied to the case this change created. A sefer of
    // yours called ברכות must not shadow Berakhot, and Berakhot must not
    // shadow it. Both, offered.
    let dir = scratch("girsa-citable-collision");
    let root = corpus(&dir);
    let (personal, slug) = personal_with(&dir, "ברכות.txt", "ראשון\n\nשני\n");

    let bar = Citations::open(&root, Some(&personal)).expect("a lexicon");
    let landing = bar.look_up("ברכות א", &Context::default());

    assert!(landing.is_a_choice(), "two seforim answer to this name");
    assert!(
        landing.only().is_none(),
        "and nothing is allowed to pick between them"
    );
    // Yours is the one that is really here, so it is the candidate with
    // segments; Sefaria's Berakhot is catalogued in the lexicon and its text was
    // never fetched, which the shelf reports as its own kind of near miss
    // rather than as a typo.
    assert!(landing.places.iter().any(|p| p.run.first.work() == slug));
    assert_eq!(landing.unrefuted(), 1);
}
