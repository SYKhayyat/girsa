//! Instruments — mode 5: gematria, notarikon, atbash, dilug (spec.md §9.3).
//!
//! The four that get used. Each is a **stated** transformation the reader asked
//! for by name, which is why they can live in the same engine as Torat Emet:
//! nothing here happens to a query that was not typed into this mode.
//!
//! # What each one is asked of
//!
//! | | asked of | how |
//! |---|---|---|
//! | gematria | the index's own words | every distinct word is added up once, and the ones that come to the number are searched for |
//! | atbash | the index | the word is transformed, then searched for literally — the ordinary literal search, on a different word |
//! | notarikon | the **text**, word by word | the first letters of words standing together, which as an index query is four patterns each matching half the vocabulary |
//! | dilug | the **text**, in reading order | letters at a fixed distance, across segments, which no inverted index can answer |
//!
//! Two of the four are questions the index can answer and two are not, and the
//! two that are not are **scans bounded by the scope chip** rather than by a
//! ceiling nobody chose.
//!
//! Notarikon is the one that looks like an index question and is not. `מקאש`
//! is four one-letter patterns — `מ.*`, `ק.*`, `א.*`, `ש.*` — and on this
//! corpus each of them matches more distinct words than a phrase query will
//! hold, so the index answers it with a refusal about postings lists. That
//! refusal is true and useless. Read off the text instead, it is a window of
//! four words and a comparison of four letters, and the reader has said which
//! sefer to read.
//!
//! # What the folded index costs, said out loud
//!
//! W11 folds final letters, so `ך` and `כ` are one letter on disk. That means
//! **mispar gadol** — the count where a final kaf is 500 — cannot be asked
//! here, and neither can any instrument that needs to tell a final letter from
//! its plain form. It is not approximated with the plain values and called
//! mispar gadol; it is [`Counting::Gadol`]'s absence, stated in the one place a
//! reader would look for it.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

/// The value of each letter. Finals are absent because the index folds them.
const VALUES: [(char, u32); 22] = [
    ('א', 1),
    ('ב', 2),
    ('ג', 3),
    ('ד', 4),
    ('ה', 5),
    ('ו', 6),
    ('ז', 7),
    ('ח', 8),
    ('ט', 9),
    ('י', 10),
    ('כ', 20),
    ('ל', 30),
    ('מ', 40),
    ('נ', 50),
    ('ס', 60),
    ('ע', 70),
    ('פ', 80),
    ('צ', 90),
    ('ק', 100),
    ('ר', 200),
    ('ש', 300),
    ('ת', 400),
];

/// Which way of counting. There is one, and the other is named so its absence
/// is not read as an oversight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counting {
    /// The plain values: א is 1 and ת is 400, a final letter counting as its
    /// plain form.
    Standard,
    /// Mispar gadol, where a final kaf is 500. **Not available**: W11's index
    /// folds final letters, so nothing on disk can tell `ך` from `כ`. Asking
    /// for it is refused rather than answered with the standard values under
    /// the wrong name.
    Gadol,
}

/// What one written word comes to.
///
/// `None` when the word has anything in it that is not a Hebrew letter — a
/// digit, a gershayim, an ASCII word. A gematria of a word that is partly not
/// Hebrew is a number nobody meant.
#[must_use]
pub fn value_of(word: &str) -> Option<u32> {
    let mut total = 0u32;
    let mut letters = 0usize;
    for c in word.chars() {
        let (_, value) = VALUES.iter().find(|(letter, _)| *letter == c)?;
        total = total.checked_add(*value)?;
        letters += 1;
    }
    (letters > 0).then_some(total)
}

/// Atbash: the first letter for the last, the second for the second-to-last.
///
/// Anything that is not one of the twenty-two letters is left where it is —
/// a space stays a space — so a phrase transforms word by word.
#[must_use]
pub fn atbash(text: &str) -> String {
    text.chars()
        .map(|c| {
            VALUES
                .iter()
                .position(|(letter, _)| *letter == c)
                .and_then(|i| VALUES.get(VALUES.len() - 1 - i))
                .map_or(c, |(letter, _)| *letter)
        })
        .collect()
}

/// Where the letters of a notarikon are taken from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    /// Rashei tevot — the first letter of each word.
    Start,
    /// Sofei tevot — the last.
    End,
}

impl Where {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Start => "first letters",
            Self::End => "last letters",
        }
    }
}

/// Why an instrument could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InstrumentError {
    #[error("nothing to work with")]
    Empty,
    #[error("`{0}` is not a run of Hebrew letters, so it has no gematria")]
    NotLetters(String),
    #[error(
        "mispar gadol needs to tell `ך` from `כ`, and the index folds them (spec.md §9.1) — the \
         standard count is what is available, and it is not the same thing"
    )]
    NoGadol,
    /// A skip so large that no sefer is long enough for it to find anything.
    #[error("a skip of {0} is longer than any sefer on the shelf")]
    SkipTooLong(usize),
}

/// One instrument, ready to be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instrument {
    /// Every word that comes to this number.
    Gematria {
        value: u32,
        /// The word it was read off, when a word was typed rather than a
        /// number. Shown, so a reader can see what the engine added up.
        of: Option<String>,
    },
    /// Words whose first — or last — letters spell this.
    Notarikon { letters: String, at: Where },
    /// The word this one becomes under atbash.
    Atbash { typed: String, becomes: String },
    /// These letters at a fixed distance through the text of a sefer.
    Dilug {
        letters: String,
        skips: RangeInclusive<usize>,
    },
}

impl Instrument {
    /// Gematria of a number, or of a word.
    ///
    /// `613` is the number; `תורה` is a word and its value is read off it. The
    /// two are told apart by whether it is written in digits, which is the only
    /// unambiguous rule — `תריג` is *also* six hundred and thirteen and is also
    /// a word, and searching for the words that share a value is what was asked
    /// for either way.
    ///
    /// # Errors
    ///
    /// If the text is empty, is not a run of Hebrew letters, or asks for a
    /// count this index cannot support.
    pub fn gematria(text: &str, counting: Counting) -> Result<Self, InstrumentError> {
        if counting == Counting::Gadol {
            return Err(InstrumentError::NoGadol);
        }
        let text = text.trim();
        if text.is_empty() {
            return Err(InstrumentError::Empty);
        }
        if let Ok(value) = text.parse::<u32>() {
            return Ok(Self::Gematria { value, of: None });
        }
        let normal = girsa_hebrew::normalize(text);
        let value =
            value_of(&normal).ok_or_else(|| InstrumentError::NotLetters(text.to_string()))?;
        Ok(Self::Gematria {
            value,
            of: Some(normal),
        })
    }

    /// Words whose first or last letters spell this.
    ///
    /// # Errors
    ///
    /// If there are no letters in it.
    pub fn notarikon(text: &str, at: Where) -> Result<Self, InstrumentError> {
        let letters: String = girsa_hebrew::normalize(text)
            .chars()
            .filter(|c| VALUES.iter().any(|(letter, _)| letter == c))
            .collect();
        if letters.is_empty() {
            return Err(InstrumentError::Empty);
        }
        Ok(Self::Notarikon { letters, at })
    }

    /// What this becomes under atbash, and then a literal search for it.
    ///
    /// # Errors
    ///
    /// If there is nothing to transform.
    pub fn atbash(text: &str) -> Result<Self, InstrumentError> {
        let typed = girsa_hebrew::normalize(text);
        if typed.is_empty() {
            return Err(InstrumentError::Empty);
        }
        Ok(Self::Atbash {
            becomes: atbash(&typed),
            typed,
        })
    }

    /// These letters at every distance in a range, through a sefer's text.
    ///
    /// # Errors
    ///
    /// If there are no letters, or the skip is longer than any sefer.
    pub fn dilug(text: &str, skips: RangeInclusive<usize>) -> Result<Self, InstrumentError> {
        let letters: String = girsa_hebrew::normalize(text)
            .chars()
            .filter(|c| VALUES.iter().any(|(letter, _)| letter == c))
            .collect();
        if letters.is_empty() {
            return Err(InstrumentError::Empty);
        }
        if *skips.start() == 0 || *skips.end() > MOST_SKIP {
            return Err(InstrumentError::SkipTooLong(*skips.end()));
        }
        Ok(Self::Dilug { letters, skips })
    }

    /// What a result header says this search was.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Gematria { value, of: None } => format!("words that come to {value}"),
            Self::Gematria {
                value,
                of: Some(word),
            } => format!("words that come to {value}, as {word} does"),
            Self::Notarikon { letters, at } => {
                format!("words whose {} spell {letters}", at.label())
            }
            Self::Atbash { typed, becomes } => {
                format!("{becomes}, which is {typed} under atbash")
            }
            Self::Dilug { letters, skips } => format!(
                "{letters}, every {} to {} letters through the text",
                skips.start(),
                skips.end()
            ),
        }
    }
}

/// The longest skip a dilug may ask for.
///
/// Longer than the letters of the longest sefer on the shelf and there is
/// nothing to find; the bound exists so that a mistyped skip is refused rather
/// than scanned for.
pub const MOST_SKIP: usize = 10_000;

/// Where a dilug landed: the letters, and the segments they fell in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequence {
    /// The distance between one letter and the next.
    pub skip: usize,
    /// Where in the work's letter stream it starts.
    pub at: usize,
    /// Whether it reads forwards or backwards through the text.
    pub backwards: bool,
    /// The segments the letters fall in, in reading order, by their place in
    /// the list this was searched over.
    pub segments: Vec<usize>,
}

/// One sefer's letters, in reading order, remembering which segment each came
/// from.
///
/// Built from segments rather than from a file because a dilug crosses segment
/// boundaries — the letters of a sefer are one stream and the boundaries are
/// typography — but a **result** has to be reported as segments, because that
/// is what a reader can open.
#[derive(Debug, Clone, Default)]
pub struct Stream {
    letters: Vec<char>,
    /// Which segment each letter came from, by index into the caller's list.
    from: Vec<usize>,
}

impl Stream {
    /// Read the letters of these segments, in the order they were given.
    #[must_use]
    pub fn of<'a>(segments: impl IntoIterator<Item = &'a str>) -> Self {
        let mut letters = Vec::new();
        let mut from = Vec::new();
        for (at, text) in segments.into_iter().enumerate() {
            for c in girsa_hebrew::normalize(text).chars() {
                if VALUES.iter().any(|(letter, _)| *letter == c) {
                    letters.push(c);
                    from.push(at);
                }
            }
        }
        Self { letters, from }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.letters.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.letters.is_empty()
    }

    /// Every equidistant sequence spelling these letters, in both directions.
    ///
    /// Both directions because that is what the instrument is: a sequence read
    /// right to left at a skip of 50 and the same one read left to right are
    /// two different findings and a tool that returned only one would be
    /// quietly answering half the question.
    #[must_use]
    pub fn dilug(&self, letters: &str, skips: &RangeInclusive<usize>) -> Vec<Sequence> {
        let wanted: Vec<char> = letters.chars().collect();
        let mut out = Vec::new();
        if wanted.is_empty() || self.letters.is_empty() {
            return out;
        }
        // A palindrome — and any single letter — reads the same in both
        // directions, and the same letters found twice would be reported as
        // two findings.
        let reads_both_ways = wanted.iter().eq(wanted.iter().rev());
        let directions: &[bool] = if reads_both_ways {
            &[false]
        } else {
            &[false, true]
        };
        for skip in skips.clone() {
            let span = skip.saturating_mul(wanted.len().saturating_sub(1));
            if span >= self.letters.len() {
                break;
            }
            for at in 0..self.letters.len() - span {
                for &backwards in directions {
                    let hit = wanted.iter().enumerate().all(|(i, want)| {
                        let step = i * skip;
                        let index = if backwards { span - step } else { step };
                        self.letters.get(at + index) == Some(want)
                    });
                    if !hit {
                        continue;
                    }
                    let mut segments: Vec<usize> = (0..wanted.len())
                        .filter_map(|i| self.from.get(at + i * skip).copied())
                        .collect();
                    segments.dedup();
                    out.push(Sequence {
                        skip,
                        at,
                        backwards,
                        segments,
                    });
                }
            }
        }
        out
    }
}

/// Where a notarikon lands in one segment, as byte spans of the printed text.
///
/// A run of words standing together whose first — or last — letters spell what
/// was asked for. Read off the words as the normalizer sees them, so `שֶׁל`
/// begins with ש whatever is printed over it, and reported as spans of the text
/// **as printed**, so the highlight lands on the page rather than on a normal
/// form nobody is looking at.
///
/// # A tag is not a word
///
/// The corpus's text carries inline markup — Berakhot alone has 43,890 `</i>`
/// — and the first line of Shas is stored as
/// `<big><strong>מֵאֵימָתַי</strong></big> קוֹרִין אֶת שְׁמַע`. Tokenized as it
/// stands, `strong` and `big` are words standing between `מֵאֵימָתַי` and
/// `קוֹרִין`, and the notarikon a reader can see on the page is not found.
///
/// So only words written in Hebrew letters count as words here. On the page the
/// tags are invisible and those four words **do** stand together, which is what
/// the instrument is about.
#[must_use]
pub fn notarikon_in(text: &str, letters: &str, at: Where) -> Vec<(usize, usize)> {
    let wanted: Vec<char> = letters.chars().collect();
    // A word is held as the two letters this instrument can ask about and the
    // span it sits on — never as a string. It reads `chars().next()` and
    // `chars().next_back()` and nothing else, so keeping the word itself meant
    // an allocation per word of the segment to carry two `char`s, and a
    // notarikon search runs across whole oversized segments.
    struct Word {
        first: char,
        last: char,
        start: usize,
        end: usize,
    }
    let mut tokens: Vec<Word> = Vec::new();
    girsa_hebrew::for_each_token(text, |word, start, end| {
        let (Some(first), Some(last)) = (word.chars().next(), word.chars().next_back()) else {
            return;
        };
        if girsa_hebrew::is_hebrew_letter(first) {
            tokens.push(Word {
                first,
                last,
                start,
                end,
            });
        }
    });
    if wanted.is_empty() || tokens.len() < wanted.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for start in 0..=tokens.len() - wanted.len() {
        let run = &tokens[start..start + wanted.len()];
        let spells = run.iter().zip(&wanted).all(|(token, want)| {
            let letter = match at {
                Where::Start => token.first,
                Where::End => token.last,
            };
            letter == *want
        });
        if spells {
            out.extend(run.iter().map(|token| (token.start, token.end)));
        }
    }
    out
}

/// Every distinct word in a list that comes to a value.
///
/// The list is the index's own vocabulary. Kept separate from the index so it
/// can be tested on words rather than on a corpus.
#[must_use]
pub fn words_worth<'a>(words: impl IntoIterator<Item = &'a str>, value: u32) -> BTreeSet<String> {
    words
        .into_iter()
        .filter(|word| value_of(word) == Some(value))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_word_is_worth_what_its_letters_are_worth() {
        assert_eq!(value_of("תורה"), Some(611));
        assert_eq!(value_of("תריג"), Some(613));
        assert_eq!(value_of("אמת"), Some(441));
    }

    #[test]
    fn a_word_that_is_not_all_letters_has_no_gematria() {
        // `שו"ע` is a word in the index and 376 is not a thing anybody means by
        // it. A number invented from the letters that happen to be Hebrew would
        // be a fact nobody could check.
        assert_eq!(value_of("שו\"ע"), None);
        assert_eq!(value_of("berakhot"), None);
        assert_eq!(value_of(""), None);
    }

    #[test]
    fn mispar_gadol_is_refused_rather_than_answered_with_the_wrong_count() {
        // The index folds `ך` onto `כ` (spec.md §9.1), so the information
        // mispar gadol needs is not on disk. Returning the standard count under
        // its name would be a wrong answer that looks right.
        assert_eq!(
            Instrument::gematria("מלך", Counting::Gadol).expect_err("refused"),
            InstrumentError::NoGadol
        );
    }

    #[test]
    fn a_number_and_a_word_both_ask_the_same_question() {
        let by_number = Instrument::gematria("613", Counting::Standard).expect("a number");
        let by_word = Instrument::gematria("תריג", Counting::Standard).expect("a word");
        let (Instrument::Gematria { value: a, .. }, Instrument::Gematria { value: b, of }) =
            (&by_number, &by_word)
        else {
            panic!("both are gematria");
        };
        assert_eq!(a, b);
        assert_eq!(of.as_deref(), Some("תריג"), "it says what it added up");
    }

    #[test]
    fn atbash_is_its_own_inverse() {
        assert_eq!(atbash("אבג"), "תשר");
        assert_eq!(atbash(&atbash("ששך")), "ששך");
        // Real: Jeremiah's ששך for בבל.
        assert_eq!(atbash("ששכ"), "בבל");
    }

    #[test]
    fn a_dilug_reads_both_ways_through_the_letters() {
        // תורה at a skip of 3, forwards, spread across two segments.
        let stream = Stream::of(["תאבוג", "דרהוה"]);
        let found = stream.dilug("תורה", &(3..=3));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].skip, 3);
        assert!(!found[0].backwards);
        assert_eq!(
            found[0].segments,
            [0, 1],
            "it crosses the segment boundary, and says which segments it touched"
        );
    }

    #[test]
    fn a_dilug_longer_than_the_text_finds_nothing_rather_than_reading_past_it() {
        let stream = Stream::of(["אבגד"]);
        assert!(stream.dilug("תורה", &(1..=100)).is_empty());
    }

    #[test]
    fn a_skip_no_sefer_is_long_enough_for_is_refused() {
        assert!(matches!(
            Instrument::dilug("תורה", 1..=MOST_SKIP + 1).expect_err("refused"),
            InstrumentError::SkipTooLong(_)
        ));
        assert!(matches!(
            Instrument::dilug("תורה", 0..=5).expect_err("refused"),
            InstrumentError::SkipTooLong(_)
        ));
    }

    #[test]
    fn words_are_picked_by_value_and_not_by_resemblance() {
        let words = ["תורה", "אמת", "תריג", "משה"];
        assert_eq!(
            words_worth(words, 611).into_iter().collect::<Vec<_>>(),
            ["תורה"]
        );
    }

    #[test]
    fn a_notarikon_finds_words_standing_together_and_says_which_they_are() {
        // The first letters of four consecutive words, and the spans come back
        // pointing at the text as printed — so the highlight lands on the page
        // and not on a normal form nobody is looking at.
        let line = "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין";
        let marks = notarikon_in(line, "מקאש", Where::Start);
        assert_eq!(marks.len(), 4, "{marks:?}");
        let first = line
            .get(marks[0].0..marks[0].1)
            .expect("a span of the printed text");
        assert_eq!(first, "מֵאֵימָתַי");

        // And a run that does not stand together is not one.
        assert!(notarikon_in(line, "משק", Where::Start).is_empty());

        // The first line of Shas as the corpus stores it: a tag stands between
        // the first word and the second, and on the page it is invisible.
        let stored = "<big><strong>מֵאֵימָתַי</strong></big> קוֹרִין אֶת שְׁמַע";
        assert_eq!(
            notarikon_in(stored, "מקאש", Where::Start).len(),
            4,
            "a tag is not a word standing between two words"
        );
    }

    #[test]
    fn a_notarikon_keeps_only_the_letters_it_can_look_for() {
        let Instrument::Notarikon { letters, at } =
            Instrument::notarikon("רמב\"ם", Where::Start).expect("letters")
        else {
            panic!("a notarikon");
        };
        assert_eq!(letters, "רמבמ", "the gershayim is not a letter to look for");
        assert_eq!(at, Where::Start);
    }
}
