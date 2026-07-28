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

/// One chip: a name, and what it can be set to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Chip {
    pub name: &'static str,
    pub choices: Vec<Choice>,
}

impl Chip {
    /// What the chip reads as when it is shut.
    #[must_use]
    pub fn shown(&self) -> &str {
        self.choices
            .iter()
            .find(|c| c.chosen)
            .map_or(self.name, |c| c.label.as_str())
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
                name: "mode",
                choices: MODES
                    .iter()
                    .map(|(mode, label, sigil)| Choice {
                        key: format!("{mode:?}"),
                        label: (*label).to_string(),
                        sigil: *sigil,
                        chosen: *mode == self.mode,
                    })
                    .collect(),
            },
            Chip {
                name: "where",
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
                name: "the word",
                choices: vec![
                    self.matching_choice(Match::Word, "the word", None),
                    self.matching_choice(Match::Contains, "contains these letters", Some("*…*")),
                    self.matching_choice(Match::Letters, "these letters, in order", Some("~…")),
                ],
            });
            out.push(Chip {
                name: "together",
                choices: vec![
                    self.together_choice(Together::Anywhere, "anywhere in a segment", None),
                    self.together_choice(Together::Phrase, "one after the other", Some("\"…\"")),
                    self.together_choice(
                        Together::Near {
                            words: self.near_words(),
                        },
                        &format!("within {} words of each other", self.near_words()),
                        Some("~5"),
                    ),
                ],
            });
        }
        if self.mode == Mode::Instruments {
            out.push(Chip {
                name: "instrument",
                choices: Sounding::ALL
                    .iter()
                    .map(|sounding| Choice {
                        key: format!("{sounding:?}"),
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
            key: format!("{matching:?}"),
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
            key: match together {
                Together::Near { words } => format!("Near{words}"),
                other => format!("{other:?}"),
            },
            label: label.to_string(),
            sigil,
            chosen: self.together == together,
        }
    }

    /// The distance the proximity chip shows. What was set, or five.
    fn near_words(&self) -> u32 {
        match self.together {
            Together::Near { words } => words,
            _ => 5,
        }
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

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
                "whole shelf",
                "the word",
                "within 5 words of each other"
            ]
        );
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
        assert_eq!(instrument.name, "instrument");
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
