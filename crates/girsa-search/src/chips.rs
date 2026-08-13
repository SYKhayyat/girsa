//! The chip row — controls are objects, not incantations (spec.md §9.5).
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │  יתגבר כארי                                          🔍    │
//! │  [torat emet ▾] [whole shelf ▾] [words near each other ▾]  │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! Three chips, and every one of them is a thing you can see, click and read
//! back. Nothing in this engine is reachable only by typing a syntax, which is
//! the rule §9.5 exists to state: *nobody should ever have to learn a syntax*.
//!
//! # And typing still works, because a sigil flips the chip
//!
//! §9.5's second sentence: *typing a sigil flips the matching chip, so the
//! power syntax teaches itself and a power user can always type instead of
//! click*. So `"יתגבר כארי"` does not search for a string with quote marks in
//! it — it flips the shape chip to **one after the other** and takes the quotes
//! out of the box. The reader sees the chip move, which is how they find out
//! the chip is there.
//!
//! | typed | chip |
//! |---|---|
//! | `"…"` | one after the other |
//! | `*קדש*` | the word contains these letters |
//! | `~קדש` | these letters, in this order |
//! | `~5` | within 5 words of each other |
//! | `/…/` | Regex |
//! | `@…` | Citation |
//! | `=613` | Instruments — gematria |
//!
//! **A sigil never changes what is searched for without showing itself.** It
//! sets a chip, the chip is on screen, and the chip says in words what it does.
//! That is the difference between this and a query language: there is no state
//! here that is not visible.

use crate::instruments::Where;
use crate::scope::Scope;
use crate::torat_emet::{Match, Together};
use crate::Mode;

/// Which instrument, when the mode is Instruments.
///
/// A chip like any other, because *which* instrument is exactly the sort of
/// thing spec.md §9.5 says must be visible and clickable rather than encoded in
/// what you type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sounding {
    /// Words that come to a number.
    #[default]
    Gematria,
    /// Words whose first letters spell it.
    Rashei,
    /// Words whose last letters spell it.
    Sofei,
    /// The word it becomes under atbash.
    Atbash,
    /// These letters at a fixed distance through a sefer.
    Dilug,
}

girsa_corpus::spelled!(Sounding {
    Gematria => "Gematria",
    Rashei => "Rashei",
    Sofei => "Sofei",
    Atbash => "Atbash",
    Dilug => "Dilug",
});

impl Sounding {
    /// Every instrument, in the order spec.md §9.3 names them.
    pub const ALL: [Self; 5] = [
        Self::Gematria,
        Self::Rashei,
        Self::Sofei,
        Self::Atbash,
        Self::Dilug,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Gematria => "gematria",
            Self::Rashei => "rashei tevot",
            Self::Sofei => "sofei tevot",
            Self::Atbash => "atbash",
            Self::Dilug => "dilug",
        }
    }

    /// Where a notarikon takes its letters from, for the two that are one.
    #[must_use]
    pub const fn at(self) -> Option<Where> {
        match self {
            Self::Rashei => Some(Where::Start),
            Self::Sofei => Some(Where::End),
            _ => None,
        }
    }
}

/// One option on a chip.
///
/// Serialized, because the window draws the row rather than deciding it: what
/// the chips are, what they can be set to and which is set is worked out here
/// and sent as it stands. A webview that assembled its own chip row would be a
/// second opinion about what the engine can do.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Choice {
    /// What the caller sends back when it is clicked.
    pub key: String,
    /// What it says on the row.
    pub label: String,
    /// What typing this does the same thing, where there is such a thing.
    pub sigil: Option<&'static str>,
    pub chosen: bool,
}

/// One chip: what it is, and what it can be set to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Chip {
    /// **The protocol, not the label.** `Chips::choose` is called back with
    /// this, so it is a stable key and never a sentence.
    ///
    /// It used to be `name`, and it used to be what the reader saw — which is
    /// finding 7: opening the search in a fully Hebrew window greeted them with
    /// `torat emet ▾ | whole shelf ▾ | the word ▾ | anywhere in a segment ▾`,
    /// and the chip could not be translated without changing the protocol
    /// because the two were one field. They are two now. `label` on each choice
    /// stays English on the wire, as a self-describing fallback and for the
    /// tests; what a reader sees comes from `say.ts`, which is where every other
    /// word in this window comes from.
    pub key: &'static str,
    pub choices: Vec<Choice>,
}

impl Chip {
    /// What the chip is set to, in the wire's own English. The **window** draws
    /// the reader's language from `key` and the chosen choice's `key`.
    #[must_use]
    pub fn shown(&self) -> &str {
        self.choices
            .iter()
            .find(|c| c.chosen)
            .map_or(self.key, |c| c.label.as_str())
    }
}

/// The row, as it stands.
#[derive(Debug, Clone, Default)]
pub struct Chips {
    pub mode: Mode,
    pub matching: Match,
    pub together: Together,
    pub scope: Scope,
    /// Which instrument, when the mode is Instruments.
    pub sounding: Sounding,
    /// How far apart a dilug's letters are. Only read by that instrument.
    pub skips: Skips,
}

/// The range of distances a dilug is looked for at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skips {
    pub from: usize,
    pub to: usize,
}

impl Default for Skips {
    /// Every distance up to fifty, which is where a dilug is usually looked
    /// for. A range rather than one number because a sequence at 49 and one at
    /// 50 are the same finding to a reader and two different searches to a
    /// computer.
    fn default() -> Self {
        Self { from: 1, to: 50 }
    }
}

/// How the modes are named on the chip.
const MODES: [(Mode, &str, Option<&'static str>); 5] = [
    (Mode::ToratEmet, "torat emet", None),
    (Mode::Smart, "smart", None),
    (Mode::Regex, "regex", Some("/…/")),
    (Mode::Citation, "citation", Some("@…")),
    (Mode::Instruments, "instruments", Some("=613")),
];

impl Chips {
    /// The row, ready to draw.
    ///
    /// Always all three, always with every option on them — a chip whose other
    /// settings are invisible until you know they exist is a syntax with a
    /// mouse.
    #[must_use]
    pub fn row(&self) -> Vec<Chip> {
        let mut out = vec![
            Chip {
                key: "mode",
                choices: MODES
                    .iter()
                    .map(|(mode, label, sigil)| Choice {
                        key: mode.as_str().to_string(),
                        label: (*label).to_string(),
                        sigil: *sigil,
                        chosen: *mode == self.mode,
                    })
                    .collect(),
            },
            Chip {
                key: DOORWAY,
                choices: vec![Choice {
                    key: "scope".to_string(),
                    label: self.scope.describe(),
                    sigil: None,
                    chosen: true,
                }],
            },
        ];
        // The operator chips only mean anything where the words are words. In
        // Regex the pattern says it; in Citation there is one address; in
        // Instruments the instrument is the operator. Showing them there would
        // be showing a control that does nothing.
        if matches!(self.mode, Mode::ToratEmet | Mode::Smart) {
            out.push(Chip {
                key: "match",
                choices: vec![
                    self.matching_choice(Match::Word, "the word", None),
                    self.matching_choice(Match::Contains, "contains these letters", Some("*…*")),
                    self.matching_choice(Match::Letters, "these letters, in order", Some("~…")),
                ],
            });
            // The distances, and there is more than one of them.
            //
            // > *"within 5 words - this should be easily customizable."*
            //
            // It was not customizable at all by clicking: the chip offered
            // exactly one proximity, `within {near_words} words`, and
            // `near_words` is five unless the chip is *already* set to a
            // proximity — so the only way to reach any other distance was to
            // know that typing `~12` did it. §9.5's whole rule is that nothing
            // is reachable only by typing a syntax.
            //
            // A ladder rather than a number box: two words is one phrase apart,
            // twenty is the same se'if, and the sigil is still there for the
            // reader who wants seventeen.
            let mut choices = vec![
                self.together_choice(Together::Anywhere, "anywhere in a segment", None),
                self.together_choice(Together::Phrase, "one after the other", Some("\"…\"")),
            ];
            for words in self.distances() {
                choices.push(self.together_choice(
                    Together::Near { words },
                    &format!("within {words} words of each other"),
                    (words == 5).then_some("~5"),
                ));
            }
            out.push(Chip {
                key: "together",
                choices,
            });
        }
        if self.mode == Mode::Instruments {
            out.push(Chip {
                key: "instrument",
                choices: Sounding::ALL
                    .iter()
                    .map(|sounding| Choice {
                        key: sounding.as_str().to_string(),
                        label: sounding.label().to_string(),
                        sigil: (*sounding == Sounding::Gematria).then_some("=613"),
                        chosen: *sounding == self.sounding,
                    })
                    .collect(),
            });
        }
        out
    }

    fn matching_choice(&self, matching: Match, label: &str, sigil: Option<&'static str>) -> Choice {
        Choice {
            key: matching.as_str().to_string(),
            label: label.to_string(),
            sigil,
            chosen: self.matching == matching,
        }
    }

    fn together_choice(
        &self,
        together: Together,
        label: &str,
        sigil: Option<&'static str>,
    ) -> Choice {
        Choice {
            key: together.key(),
            label: label.to_string(),
            sigil,
            chosen: self.together == together,
        }
    }

    /// The distances the chip offers, with whatever the reader typed among them.
    ///
    /// Fixed rungs so the row is the same every time it is opened, and the
    /// reader's own number folded in and sorted rather than appended — a `~17`
    /// typed once must be visible on the chip afterwards, or the control and the
    /// search disagree about what is set, which is the one thing §9.5 forbids.
    fn distances(&self) -> Vec<u32> {
        let mut rungs = vec![2, 3, 5, 10, 20];
        if let Together::Near { words } = self.together {
            if !rungs.contains(&words) {
                rungs.push(words);
            }
        }
        rungs.sort_unstable();
        rungs
    }

    /// Read the sigils out of what was typed, and set the chips they name.
    ///
    /// Returns the text with the sigils taken off — what the query bar shows
    /// afterwards, and what is actually searched for.
    ///
    /// The chips this does **not** mention are left exactly as they were. A
    /// reader who set the scope by clicking a facet does not lose it by typing
    /// a quotation mark.
    #[must_use]
    pub fn read(&self, typed: &str) -> (Self, String) {
        let mut chips = self.clone();
        let mut text = typed.trim().to_string();

        if let Some(inner) = wrapped(&text, '/') {
            chips.mode = Mode::Regex;
            return (chips, inner);
        }
        if let Some(rest) = text.strip_prefix('@') {
            chips.mode = Mode::Citation;
            return (chips, rest.trim().to_string());
        }
        if let Some(rest) = text.strip_prefix('=') {
            chips.mode = Mode::Instruments;
            // `=` is the gematria sigil in particular, not the mode's in
            // general: it is what a person types when they mean a number.
            chips.sounding = Sounding::Gematria;
            return (chips, rest.trim().to_string());
        }
        if let Some(inner) = wrapped(&text, '"') {
            chips.together = Together::Phrase;
            text = inner;
        }

        let mut words: Vec<String> = Vec::new();
        for word in text.split_whitespace() {
            // `~5` is a distance and `~קדש` is a way of matching a word. Both
            // are "looser", which is the whole of what the tilde says.
            if let Some(number) = word.strip_prefix('~').and_then(|n| n.parse::<u32>().ok()) {
                chips.together = Together::Near { words: number };
                continue;
            }
            if let Some(rest) = word.strip_prefix('~') {
                chips.matching = Match::Letters;
                words.push(rest.to_string());
                continue;
            }
            if word.len() > 2 && word.starts_with('*') && word.ends_with('*') {
                chips.matching = Match::Contains;
                words.push(word.trim_matches('*').to_string());
                continue;
            }
            words.push(word.to_string());
        }
        (chips, words.join(" "))
    }
}

/// What is inside a pair of these, if the whole string is wrapped in them.
fn wrapped(text: &str, mark: char) -> Option<String> {
    let rest = text.strip_prefix(mark)?.strip_suffix(mark)?;
    (!rest.is_empty()).then(|| rest.trim().to_string())
}

impl Chips {
    /// Set one chip to one of its choices, by the keys [`Chips::row`] sent.
    ///
    /// # Why this is not four `match` arms in the window
    ///
    /// It was, and each of them ended `_ => Mode::ToratEmet` — a silent
    /// fallback to the default for anything unrecognised. Four families, four
    /// default-on-unknown, and forty lines away in the same file `link_repair`
    /// refused an unknown candidate by name. Two policies about the same
    /// question, in one file, and the quiet one was the one the search bar
    /// used: a typo in a chip key came back as a search that ran, answered,
    /// and answered a different question than the one asked.
    ///
    /// The keys are the ones `row` writes, from the same tables. A choice that
    /// round-trips is now a compile-time fact rather than two lists that
    /// happened to agree.
    ///
    /// # Errors
    ///
    /// If no chip is called that, or the chip does not offer that choice.
    pub fn choose(&mut self, chip: &str, key: &str) -> Result<(), ChipError> {
        let missing = || ChipError::NoSuchChoice {
            chip: chip.to_string(),
            key: key.to_string(),
        };
        match chip {
            "mode" => self.mode = Mode::named(key).ok_or_else(missing)?,
            "match" => self.matching = Match::named(key).ok_or_else(missing)?,
            "together" => self.together = Together::named(key).ok_or_else(missing)?,
            "instrument" => self.sounding = Sounding::named(key).ok_or_else(missing)?,
            // A chip whose one choice is a doorway rather than a setting: it
            // shows what the scope is and clicking it opens the facet panel.
            // Refused by name, because *this chip is not set this way* and
            // *there is no such chip* are different things to whoever is
            // looking at the window.
            DOORWAY => return Err(ChipError::NotASetting(chip.to_string())),
            other => return Err(ChipError::NoSuchChip(other.to_string())),
        }
        Ok(())
    }

    /// Every chip currently offered **that is set by choosing**, and every
    /// choice under it.
    ///
    /// What [`Chips::choose`] is tested against: everything offered can be
    /// chosen. [`DOORWAY`] is left out because it is not offered as a choice
    /// in the first place — it is offered as a way in.
    #[must_use]
    pub fn settable(&self) -> Vec<(String, Vec<String>)> {
        self.row()
            .into_iter()
            .filter(|chip| chip.key != DOORWAY)
            .map(|chip| {
                (
                    chip.key.to_string(),
                    chip.choices.into_iter().map(|c| c.key).collect(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn everything_offered_can_be_chosen() {
        // The round trip that used to be two lists agreeing by luck: `row`
        // wrote the keys with `format!("{:?}")` and the window read them back
        // with a hand-written `match`, in a different file, in a different
        // repository directory. Renaming a variant changed one of them.
        for mode in [
            Mode::ToratEmet,
            Mode::Smart,
            Mode::Regex,
            Mode::Citation,
            Mode::Instruments,
        ] {
            let mut chips = Chips {
                mode,
                ..Chips::default()
            };
            for (chip, keys) in chips.clone().settable() {
                for key in keys {
                    assert!(
                        chips.choose(&chip, &key).is_ok(),
                        "{mode:?} offered `{chip}`/`{key}` and would not take it back"
                    );
                }
            }
        }
    }

    #[test]
    fn a_key_no_chip_offers_is_refused_and_not_defaulted() {
        // Four families, four `_ =>` arms, four silent falls back to the
        // default — and forty lines away in the same file, `link_repair`
        // refused an unknown candidate by name. The quiet one was the one the
        // search bar used: a typo came back as a search that ran and answered
        // a different question.
        let mut chips = Chips::default();
        assert!(chips.choose("mode", "Smrat").is_err());
        assert_eq!(chips.mode, Mode::default(), "and it changed nothing");
        assert!(chips.choose("match", "Contians").is_err());
        assert!(chips.choose("instrument", "Gemtria").is_err());
        assert!(matches!(
            chips.choose("colour", "blue"),
            Err(ChipError::NoSuchChip(_))
        ));
    }

    #[test]
    fn the_chip_that_is_a_doorway_says_so_rather_than_saying_it_does_not_exist() {
        // `where` reports the scope and opens the facet panel. A window that
        // sent it here has a wiring bug, and *no such chip* would send whoever
        // is reading the message looking for a typo.
        let mut chips = Chips::default();
        assert!(matches!(
            chips.choose(DOORWAY, "scope"),
            Err(ChipError::NotASetting(_))
        ));
        assert!(chips.row().iter().any(|chip| chip.key == DOORWAY));
    }

    #[test]
    fn a_proximity_of_nothing_in_particular_is_not_a_proximity() {
        // `Near` carried a number and the old parser was
        // `strip_prefix("Near").and_then(parse).unwrap_or(Anywhere)`, so
        // `Nearbanana` searched the whole segment while the chip showed a
        // proximity search. Two different searches, one label.
        let mut chips = Chips::default();
        assert!(chips.choose("together", "Nearbanana").is_err());
        assert!(chips.choose("together", "Near").is_err());
        assert!(chips.choose("together", "Near12").is_ok());
        assert_eq!(chips.together, Together::Near { words: 12 });
        assert_eq!(chips.together.key(), "Near12");
    }

    #[test]
    fn the_keys_on_the_wire_are_the_ones_they_always_were() {
        // Moved from `format!("{:?}")` to a written-down table. If the two
        // disagreed, every reader with a saved session would find their chips
        // reset to the defaults on the next launch.
        assert_eq!(Mode::ToratEmet.as_str(), "ToratEmet");
        assert_eq!(Mode::Instruments.as_str(), "Instruments");
        assert_eq!(Match::Contains.as_str(), "Contains");
        assert_eq!(Sounding::Gematria.as_str(), "Gematria");
        assert_eq!(Together::Anywhere.key(), "Anywhere");
        assert_eq!(Together::Phrase.key(), "Phrase");
    }

    #[test]
    fn a_chip_that_is_not_showing_is_still_not_a_chip_that_does_not_exist() {
        // The instrument chip is only offered in Instruments mode. Setting it
        // from a window that has just switched modes is a race, not a typo,
        // and refusing it would lose the reader's click.
        let mut chips = Chips::default();
        assert_ne!(chips.mode, Mode::Instruments);
        assert!(chips.choose("instrument", "Atbash").is_ok());
        assert_eq!(chips.sounding, Sounding::Atbash);
    }

    #[test]
    fn the_row_reads_the_way_the_spec_draws_it() {
        let chips = Chips {
            together: Together::Near { words: 5 },
            ..Chips::default()
        };
        let row = chips.row();
        let shown: Vec<&str> = row.iter().map(Chip::shown).collect();
        assert_eq!(
            shown,
            [
                "torat emet",
                // The scope chip says nothing while nothing is narrowed; the
                // window draws `כל המדף` over it.
                "",
                "the word",
                "within 5 words of each other"
            ]
        );
    }

    #[test]
    fn the_proximity_chip_offers_more_than_one_distance() {
        // *"within 5 words - this should be easily customizable."* It offered
        // one distance, always five unless a proximity was already set, so every
        // other distance was reachable only by knowing to type `~12`.
        let chips = Chips::default();
        let row = chips.row();
        let together = row
            .iter()
            .find(|chip| chip.key == "together")
            .expect("the together chip");
        let labels: Vec<&str> = together.choices.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "anywhere in a segment",
                "one after the other",
                "within 2 words of each other",
                "within 3 words of each other",
                "within 5 words of each other",
                "within 10 words of each other",
                "within 20 words of each other",
            ]
        );
        // And every one of them round-trips through `choose`, which is what
        // `everything_offered_can_be_chosen` asserts over the whole row.
        let mut chips = Chips::default();
        assert!(chips.choose("together", "Near10").is_ok());
        assert_eq!(chips.together, Together::Near { words: 10 });
    }

    #[test]
    fn a_distance_the_reader_typed_appears_on_the_chip() {
        // The other half of §9.5: a control that does not show what is set is a
        // control that lies. `~17` is a real search and the chip has to say so.
        let (chips, _) = Chips::default().read("יתגבר ~17 כארי");
        let row = chips.row();
        let together = row
            .iter()
            .find(|chip| chip.key == "together")
            .expect("the together chip");
        assert_eq!(together.shown(), "within 17 words of each other");
        assert!(together
            .choices
            .iter()
            .any(|c| c.label == "within 17 words of each other" && c.chosen));
    }

    #[test]
    fn a_quoted_phrase_flips_the_chip_and_loses_its_quotes() {
        // spec.md §9.5: the sigil teaches the chip. What must never happen is
        // the quotes being searched for as characters, or the chip staying put
        // while the search quietly changes.
        let (chips, text) = Chips::default().read("\"יתגבר כארי\"");
        assert_eq!(chips.together, Together::Phrase);
        assert_eq!(text, "יתגבר כארי");
    }

    #[test]
    fn the_instrument_is_a_chip_and_not_something_you_have_to_type() {
        let chips = Chips {
            mode: Mode::Instruments,
            sounding: Sounding::Atbash,
            ..Chips::default()
        };
        let row = chips.row();
        let instrument = row.last().expect("an instrument chip");
        assert_eq!(instrument.key, "instrument");
        assert_eq!(instrument.shown(), "atbash");
        assert_eq!(instrument.choices.len(), 5, "all five are on the chip");
    }

    #[test]
    fn the_sigils_that_name_a_mode_set_the_mode() {
        assert_eq!(Chips::default().read("/ק.*ש/").0.mode, Mode::Regex);
        assert_eq!(Chips::default().read("@ברכות ב.").0.mode, Mode::Citation);
        assert_eq!(Chips::default().read("=613").0.mode, Mode::Instruments);
        // And the sigil comes off, so the mode gets what was meant.
        assert_eq!(Chips::default().read("@ברכות ב.").1, "ברכות ב.");
    }

    #[test]
    fn a_tilde_and_a_number_is_a_distance_a_tilde_and_a_word_is_not() {
        let (chips, text) = Chips::default().read("יתגבר ~5 כארי");
        assert_eq!(chips.together, Together::Near { words: 5 });
        assert_eq!(text, "יתגבר כארי", "the sigil is not part of the query");

        let (chips, text) = Chips::default().read("~קדש");
        assert_eq!(chips.matching, Match::Letters);
        assert_eq!(text, "קדש");
    }

    #[test]
    fn stars_around_a_word_ask_for_the_words_it_is_inside() {
        let (chips, text) = Chips::default().read("*קדש*");
        assert_eq!(chips.matching, Match::Contains);
        assert_eq!(text, "קדש");
    }

    #[test]
    fn reading_a_sigil_leaves_every_chip_it_did_not_name_alone() {
        // A reader who narrowed to the Bavli by clicking a facet does not lose
        // it by typing a quotation mark.
        let chips = Chips {
            scope: Scope::everything().only(["bavli/berakhot".to_string()], "תלמוד/בבלי"),
            ..Chips::default()
        };
        let (after, _) = chips.read("\"יתגבר כארי\"");
        assert_eq!(after.scope, chips.scope);
        assert_eq!(after.mode, Mode::ToratEmet);
    }

    #[test]
    fn a_lone_mark_is_a_word_and_not_a_sigil() {
        // `*` alone, or a quote in the middle of a phrase, is text somebody
        // typed. Reading it as a control would change the search under them.
        assert_eq!(Chips::default().read("*").1, "*");
        let (chips, text) = Chips::default().read("שו\"ע");
        assert_eq!(chips.together, Together::Anywhere);
        assert_eq!(text, "שו\"ע", "the gershayim is part of the word");
    }
}

/// A chip the window named, or a choice under it, that this project does not
/// write.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChipError {
    /// No chip is called that.
    #[error("no such chip: {0}")]
    NoSuchChip(String),
    /// The chip exists and does not offer that.
    #[error("`{key}` is not a choice under `{chip}`")]
    NoSuchChoice { chip: String, key: String },
    /// The chip exists and is not set by choosing among its choices.
    #[error("`{0}` is not set that way — it opens the facet panel")]
    NotASetting(String),
}

/// The chip that is a doorway rather than a setting.
///
/// It reports the scope and opens the panel that changes it; the scope itself
/// is a `Scope`, not a key, and comes back through its own errand.
pub const DOORWAY: &str = "where";
