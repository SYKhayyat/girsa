//! Regex — mode 3, *full power, no hand-holding* (spec.md §9.3).
//!
//! The one mode with no ladder, no widening and no offers: §9.6's table says
//! **nothing** happens on a zero here. You wrote a pattern; it matched nothing.
//! That is the whole contract, and adding help to it would make the other four
//! modes' promises unreadable.
//!
//! # What a pattern is matched against
//!
//! A **whole word of the index**, which is a normalized word: nikud and te'amim
//! off, final letters folded, geresh and gershayim folded onto `'` and `"`
//! (W2, W11). So `ק.*ש` is a word beginning ק and ending ש — the anchors are
//! implied, because there is nothing either side of a word to match — and there
//! is no way to write a pattern that spans two words. For that, put a space in
//! and the words are matched in order, each by its own pattern.
//!
//! # No hand-holding is not the same as no honesty
//!
//! Three patterns are **refused** rather than run, and every one of them would
//! otherwise return nothing for ever while looking like a legitimate empty
//! result:
//!
//! - one carrying **nikud or te'amim**, because no term in the index has any;
//! - one carrying a **final letter** (`ך ם ן ף ץ`), because the normalizer
//!   folds them and no term has one of those either;
//! - one **anchored** with `^` or `$`, because a pattern is already matched
//!   against the whole of a word and an anchor cannot mean anything else here.
//!
//! # What a pattern may ask for
//!
//! A pattern is compiled to a DFA and then matched against the whole term
//! dictionary — 5.2 million words on the full corpus — so *how much* a query
//! asks for is a question about the query and not only about what it says.
//! Two of the bounds are [`MOST_PATTERNS`] and [`MOST_PATTERN_CHARS`]: a query
//! of twenty patterns is twenty passes over every word in the index, and both
//! are reachable without a reader writing anything, since the MCP server offers
//! `mode: regex` to whatever is on the other end of the socket (issue #62).
//!
//! The third bound is not ours and is worth naming so nobody looks for it here:
//! a pattern whose automaton will not fit in `tantivy-fst`'s ten-megabyte
//! instruction budget is **refused by the compiler**, and that error reaches
//! the reader unedited — which is the right treatment for a regex parser's
//! complaint, and means a nested-quantifier blowup costs a compile, not a
//! search. Nothing here screens patterns for shapes it dislikes: in the one
//! mode whose contract is that the engine does not edit what you wrote, a
//! refusal has to be about the query being unrunnable and not about taste.
//!
//! Each refusal names the thing and says what it would have to be instead. That
//! is not the engine changing a query — nothing is run, and the reader retypes.
//! Silently running a pattern that cannot match is the failure this whole
//! project is arranged against: the answer is empty, it looks like an answer,
//! and the sefer is on the page in front of them.

use crate::torat_emet::Together;

/// Why a pattern was not run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegexError {
    #[error("nothing to search for")]
    Empty,
    /// More patterns than the mode will run.
    #[error(
        "`{count}` patterns is more than this mode runs at once (it runs {allowed}) — a pattern is \
         matched against every word in the index, and {count} of them is {count} passes over all \
         of it. Ask for fewer, or use the words mode"
    )]
    TooManyPatterns { count: usize, allowed: usize },
    /// One pattern longer than the mode will compile.
    #[error(
        "`{length}` characters of pattern is longer than this mode runs ({allowed}) — is this a \
         pattern, or a file? Look for a shorter one, or search the words"
    )]
    PatternTooLong { length: usize, allowed: usize },
    /// The pattern carries marks the index does not have.
    #[error(
        "the index holds words with their marks off, so `{mark}` in `{pattern}` can never match — \
         take the nikud out"
    )]
    Marks { pattern: String, mark: char },
    /// The pattern carries a final letter, which the normalizer folds.
    #[error(
        "the index folds final letters, so `{final_letter}` in `{pattern}` can never match — \
         write `{plain}`"
    )]
    FinalLetter {
        pattern: String,
        final_letter: char,
        plain: char,
    },
    /// The pattern is anchored, and a pattern here is already whole-word.
    ///
    /// Not stripped. `^` and `$` mean nothing different from their absence in
    /// this mode, so taking them off would change no result — but it would be
    /// the engine editing a pattern somebody wrote, in the one mode whose whole
    /// contract is that it does not.
    #[error(
        "`{pattern}` is anchored, and a pattern here is matched against the whole of a word, so \
         `{mark}` is already implied — write `{bare}`"
    )]
    Anchored {
        pattern: String,
        mark: char,
        bare: String,
    },
    /// Order-free proximity, which this mode does not have.
    ///
    /// W12 refused to answer *within X words of each other* with a slop, and
    /// the reason holds here word for word: tantivy's slop is a budget that
    /// lets terms reorder at a cost, so a phrase with slop 2 matches *two words
    /// apart in order* **and** *reversed and adjacent* — a window the reader did
    /// not ask for. The literal mode pays for the exact answer by asking each
    /// ordering separately; patterns cannot be asked that way without
    /// multiplying automata, so this mode says it cannot rather than answering
    /// a near-enough question.
    #[error(
        "this mode has no *within {words} words of each other* — a pattern proximity would be a \
         window you did not ask for. Ask for the patterns one after the other, or search the \
         words instead"
    )]
    NoProximity { words: u32 },
}

/// How many patterns one query may hold.
///
/// A pattern is matched against the whole term dictionary, so a query of `n`
/// patterns is `n` passes over five million words. Sixteen is far past what
/// anyone types into a phrase and far short of what turns a command into a
/// five-minute one — and past it the query is **refused**, not trimmed, because
/// answering sixteen of twenty patterns is the partial answer that looks like a
/// whole one (issue #62).
pub const MOST_PATTERNS: usize = 16;

/// How long one pattern may be, in characters.
///
/// The pattern is compiled to a DFA before it is run, and `tantivy-fst` already
/// refuses one that compiles past ten megabytes of instructions — so the work a
/// pattern can ask for is bounded, and this is not that bound. This is the
/// smaller one: two hundred characters is longer than any pattern a person
/// writes against a word of Hebrew, and a caller that sends four thousand is
/// not writing a pattern, it is generating one at a program that answers
/// questions about seforim.
pub const MOST_PATTERN_CHARS: usize = 200;

/// The anchor a pattern begins or ends with, if it has one.
fn anchor_of(pattern: &str) -> Option<char> {
    if pattern.starts_with('^') {
        return Some('^');
    }
    // An escaped `$` is a dollar sign somebody is looking for, and the corpus
    // does have those.
    if pattern.ends_with('$') && !pattern.ends_with(r"\$") {
        return Some('$');
    }
    None
}

/// The final letters, and what the normalizer folds each onto.
const FINALS: [(char, char); 5] = [('ך', 'כ'), ('ם', 'מ'), ('ן', 'נ'), ('ף', 'פ'), ('ץ', 'צ')];

/// One regex query: a pattern per word, and how the words relate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    patterns: Vec<String>,
    together: Together,
}

impl Query {
    /// Read what was typed, refusing a pattern that cannot match.
    ///
    /// Whitespace separates patterns, because a term automaton cannot cross a
    /// word boundary — so a space in a regex query is not a space to be
    /// matched, it is the end of one pattern and the start of the next. Said
    /// here because it is the one thing about this mode that surprises people.
    ///
    /// # Errors
    ///
    /// [`RegexError`], every arm of which is a pattern that would have matched
    /// nothing for a reason the reader could not have seen.
    pub fn parse(text: &str, together: Together) -> Result<Self, RegexError> {
        let patterns: Vec<String> = text
            .split_whitespace()
            .map(str::to_string)
            .filter(|p| !p.is_empty())
            .collect();
        if patterns.is_empty() {
            return Err(RegexError::Empty);
        }
        // Bounded before anything is compiled or run: the two refusals below
        // are the only ones here that are about the query's *size* rather than
        // what it says, and a caller on the other side of a socket can send
        // either without writing a pattern at all (issue #62).
        if patterns.len() > MOST_PATTERNS {
            return Err(RegexError::TooManyPatterns {
                count: patterns.len(),
                allowed: MOST_PATTERNS,
            });
        }
        if let Some(long) = patterns
            .iter()
            .find(|p| p.chars().count() > MOST_PATTERN_CHARS)
        {
            return Err(RegexError::PatternTooLong {
                length: long.chars().count(),
                allowed: MOST_PATTERN_CHARS,
            });
        }
        if let Together::Near { words } = together {
            return Err(RegexError::NoProximity { words });
        }
        for pattern in &patterns {
            if let Some(mark) = anchor_of(pattern) {
                return Err(RegexError::Anchored {
                    bare: pattern
                        .trim_start_matches('^')
                        .trim_end_matches('$')
                        .to_string(),
                    pattern: pattern.clone(),
                    mark,
                });
            }
            for c in pattern.chars() {
                if girsa_hebrew::is_mark(c) {
                    return Err(RegexError::Marks {
                        pattern: pattern.clone(),
                        mark: c,
                    });
                }
                if let Some((_, plain)) = FINALS.iter().find(|(f, _)| *f == c) {
                    return Err(RegexError::FinalLetter {
                        pattern: pattern.clone(),
                        final_letter: c,
                        plain: *plain,
                    });
                }
            }
        }
        Ok(Self { patterns, together })
    }

    #[must_use]
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    #[must_use]
    pub fn shape(&self) -> Together {
        self.together
    }

    /// What a result header says this search was.
    ///
    /// The patterns as typed. There is nothing else to say: no normalization
    /// was applied to them, which is the mode.
    #[must_use]
    pub fn describe(&self) -> String {
        let how = match self.together {
            Together::Anywhere => "anywhere in a segment",
            // `Near` never reaches here: it is refused at [`Query::parse`],
            // because a slop is not the question the chip asks.
            Together::Phrase | Together::Near { .. } => "one after the other",
        };
        format!("the patterns {}, {how}", self.patterns.join(" "))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_pattern_is_taken_exactly_as_typed() {
        let query = Query::parse("ק.*ש", Together::Anywhere).expect("a pattern");
        assert_eq!(query.patterns(), ["ק.*ש"]);
    }

    #[test]
    fn an_anchored_pattern_is_refused_rather_than_quietly_unanchored() {
        // A person who writes regexes writes `^…$` without thinking. Here it
        // means nothing — a pattern is matched against the whole of a word
        // already — and tantivy answers it with a parser error about empty
        // match operators. Neither of those is an answer, so it is refused in
        // this engine's own words, with the pattern to write instead.
        let error = Query::parse("^קדש$", Together::Anywhere).expect_err("refused");
        let RegexError::Anchored { bare, .. } = &error else {
            panic!("{error:?}");
        };
        assert_eq!(bare, "קדש");
        assert!(error.to_string().contains("whole of a word"), "{error}");
    }

    #[test]
    fn a_space_separates_two_patterns_because_one_cannot_cross_a_word() {
        let query = Query::parse("יתגבר כאר.", Together::Phrase).expect("two patterns");
        assert_eq!(query.patterns(), ["יתגבר", "כאר."]);
    }

    #[test]
    fn a_pattern_with_nikud_in_it_is_refused_rather_than_run() {
        // It would match nothing, for ever, and look exactly like an honest
        // empty result — in the mode whose whole promise is that an empty
        // result means the corpus does not say it.
        let error = Query::parse("קָדַשׁ", Together::Anywhere).expect_err("refused");
        assert!(matches!(error, RegexError::Marks { .. }), "{error:?}");
        assert!(error.to_string().contains("marks off"), "{error}");
    }

    #[test]
    fn a_pattern_with_a_final_letter_says_what_to_write_instead() {
        let error = Query::parse("מלך", Together::Anywhere).expect_err("refused");
        let RegexError::FinalLetter { plain, .. } = error else {
            panic!("{error:?}");
        };
        assert_eq!(plain, 'כ');
    }

    #[test]
    fn this_mode_says_it_has_no_order_free_proximity_rather_than_approximating_one() {
        // The same refusal W12 made, for the same reason: a slop is a budget
        // that lets terms reorder at a cost, and answering *within 5 words of
        // each other* with one would return a window nobody asked for.
        let error = Query::parse("קדש", Together::Near { words: 5 }).expect_err("refused");
        assert_eq!(error, RegexError::NoProximity { words: 5 });
    }

    #[test]
    fn a_query_of_more_patterns_than_the_mode_runs_is_refused_at_the_ceiling() {
        let inside = vec!["ק.*"; MOST_PATTERNS].join(" ");
        assert!(Query::parse(&inside, Together::Phrase).is_ok());

        let outside = vec!["ק.*"; MOST_PATTERNS + 1].join(" ");
        let error = Query::parse(&outside, Together::Phrase).expect_err("refused");
        let RegexError::TooManyPatterns { count, allowed } = error else {
            panic!("{error:?}");
        };
        assert_eq!((count, allowed), (MOST_PATTERNS + 1, MOST_PATTERNS));
        assert!(error.to_string().contains("fewer"), "{error}");
    }

    #[test]
    fn a_pattern_longer_than_the_mode_runs_is_refused_at_the_ceiling() {
        let inside = "ק".repeat(MOST_PATTERN_CHARS);
        assert!(Query::parse(&inside, Together::Anywhere).is_ok());

        let outside = "ק".repeat(MOST_PATTERN_CHARS + 1);
        let error = Query::parse(&outside, Together::Anywhere).expect_err("refused");
        assert_eq!(
            error,
            RegexError::PatternTooLong {
                length: MOST_PATTERN_CHARS + 1,
                allowed: MOST_PATTERN_CHARS,
            }
        );
    }

    #[test]
    fn an_empty_pattern_is_not_a_search() {
        assert_eq!(
            Query::parse("   ", Together::Anywhere).expect_err("refused"),
            RegexError::Empty
        );
    }
}
