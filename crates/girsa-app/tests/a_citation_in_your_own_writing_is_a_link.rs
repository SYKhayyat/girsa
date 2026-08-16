//! A mekor you typed into a note is somewhere to go, and not a printed string.
//!
//! W19, spec.md §10.5. Linkify was written for the Ksav loop and lived in
//! `girsa-desk`, which depends on `girsa-app` — so the reading pane, which is
//! where a note is actually read, had no way to call it. A note that said
//! *ועיין ברכות ב.* said it in ink.
//!
//! The three things asserted here are the three that can go wrong:
//!
//! 1. the citation becomes a run of its own, carrying the ref;
//! 2. **the corpus does not get the same treatment** — a sefer from Sefaria
//!    already has a link layer built from `links0.csv`, and a second, weaker
//!    set of edges laid over the same words would be indistinguishable from it
//!    on the page;
//! 3. the offsets survive the nikud coming off. Linkify is run over the string
//!    the pane is about to draw, not over the segment as it stands on disk, and
//!    getting that backwards puts the link a few letters to the left of the
//!    words it was about — which is exactly the defect `display::Shown` exists
//!    to prevent for corrections.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_app::session::Pointing;
use girsa_app::view::Line;
use girsa_corpus::work::Source;

fn lexicon() -> girsa_ref::Lexicon {
    let mut lex = girsa_ref::Lexicon::default();
    lex.add(
        girsa_ref::Work {
            slug: "bavli/berakhot".into(),
            he_title: "ברכות".into(),
            en_title: "Berakhot".into(),
        },
        &["ברכות", "Berakhot"],
    );
    lex
}

/// A sefer of your own with one line in it.
fn yours(text: &str) -> girsa_app::shelf::Open {
    let mut open = girsa_app::pretend::sefer("note/חבורה", "חבורה", &["א"], &[text]);
    open.work.source = Source::Mine;
    open
}

fn drawn(
    open: &girsa_app::shelf::Open,
    pointing: Pointing,
    lex: Option<&girsa_ref::Lexicon>,
) -> Line {
    Line::of(
        open,
        open.segments.first().expect("one line"),
        pointing,
        girsa_app::shemos::Shemos::AsWritten,
        girsa_cite::CiteStyle::HebrewShort,
        lex,
    )
}

/// Every run that carries a ref, as (words, ref).
fn cited(line: &Line) -> Vec<(String, String)> {
    line.runs
        .iter()
        .filter_map(|run| {
            run.cite
                .as_ref()
                .map(|reference| (run.text.clone(), reference.clone()))
        })
        .collect()
}

#[test]
fn a_mekor_typed_into_a_note_becomes_somewhere_to_go() {
    let open = yours("ועיין ברכות ב. ושם מבואר כדבריו");
    let line = drawn(&open, Pointing::Full, Some(&lexicon()));

    assert_eq!(
        cited(&line),
        vec![(
            "ברכות ב.".to_string(),
            "girsa:bavli/berakhot/2a".to_string()
        )],
        "the citation is one run and it carries the place"
    );

    // And the rest of the line is still the rest of the line. A linkifier that
    // swallowed the sentence would pass the assertion above and lose the words.
    let whole: String = line.runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(whole, "ועיין ברכות ב. ושם מבואר כדבריו");
}

#[test]
fn the_corpus_keeps_the_link_layer_it_already_has() {
    // The same words, in a sefer that came from Sefaria. Nothing is linkified,
    // and that is the rule rather than an oversight: 1.4 million edges built
    // from `links0.csv` are already drawn on these lines, and three narrow
    // rules over a string must not lay a second set beside them.
    let mut open = yours("ועיין ברכות ב. ושם מבואר כדבריו");
    open.work.source = Source::Sefaria;
    assert!(cited(&drawn(&open, Pointing::Full, Some(&lexicon()))).is_empty());
}

#[test]
fn no_lexicon_is_a_plain_line_and_not_a_broken_one() {
    // `girsa-import` may not have run. The words stay words.
    let open = yours("ועיין ברכות ב. ושם מבואר כדבריו");
    let line = drawn(&open, Pointing::Full, None);
    assert!(cited(&line).is_empty());
    let whole: String = line.runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(whole, "ועיין ברכות ב. ושם מבואר כדבריו");
}

#[test]
fn the_link_lands_on_the_words_after_the_nikud_comes_off() {
    // The finding this test exists for. `Line::of` draws
    // `pointed(&segment.text, pointing)` — taking the marks out **shortens the
    // string**, and a citation found in the stored text and reported against
    // the drawn one lands wherever the difference put it. Nine nikud points
    // before the mekor here, so an off-by-that link would sit on `ושם מבואר`.
    let open = yours("וְעַיֵּן ברכות ב. ושם מבואר");
    let line = drawn(&open, Pointing::Plain, Some(&lexicon()));
    assert_eq!(
        cited(&line),
        vec![(
            "ברכות ב.".to_string(),
            "girsa:bavli/berakhot/2a".to_string()
        )]
    );

    // And with the pointing left on, the same citation is still the citation —
    // the marks are on the word before it, not on the mekor.
    let full = drawn(&open, Pointing::Full, Some(&lexicon()));
    assert_eq!(
        cited(&full),
        vec![(
            "ברכות ב.".to_string(),
            "girsa:bavli/berakhot/2a".to_string()
        )]
    );
}
