//! *Where is this phrase from?* — and *who quotes this Gemara?*
//!
//! spec.md §10.4, BUILDER.md W18: **one feature asked from two directions.**
//! Highlight a phrase in a document and ask where it is from; stand on a
//! Gemara and ask who quotes it. Both are the same question put to the same
//! index — *which segments contain these words, one after the other* — and the
//! only difference is which sefer you leave out of the answer.
//!
//! # What makes this hard is not finding, it is not lying
//!
//! A phrase search always returns something. The failure mode is not an empty
//! list, it is a **confident wrong mekor**: `אמר רבי יוחנן` is in 4,000 places
//! and the first of them is not the source of anything. So this module returns
//! the count first and the candidates second, and says plainly when a phrase is
//! too common to be a citation (BUILDER rule 6 — a wrong ref is worse than no
//! ref).
//!
//! # And it says when it widened
//!
//! A quotation in somebody's own writing is rarely letter for letter: a prefix,
//! a male spelling, a word left out. The literal search runs first; only if it
//! finds nothing does the ladder climb (W13's Smart), and [`Found::how`] then
//! carries the rung so what the reader is shown is *these are near matches*
//! rather than *this is the source*.

use girsa_corpus::segment::SegmentId;

use crate::bar::{Answer, Bar};
use crate::chips::Chips;
use crate::index::Paging;
use crate::scope::Scope;
use crate::torat_emet::Together;
use crate::Mode;

/// Above this many hits, a phrase is a turn of speech and not a quotation.
///
/// Not a cutoff on the results — every one of them is still returned and
/// counted. It is a cutoff on the *claim*: at 4,000 hits, offering the first as
/// "the mekor" would be the system inventing an answer, which is the one thing
/// it may never do.
pub const TOO_COMMON: usize = 200;

/// How many candidates `where_from` answers with when the asker did not name a
/// number.
///
/// A default is a decision about the question, so it lives beside the question
/// rather than in whichever caller got there first — the desk used to unwrap
/// its own private 8, with no test and nothing tying it to [`TOO_COMMON`]'s
/// idea of how many hits a quotation has. Ksav cycles what comes back; eight is
/// enough to cycle and small enough to answer.
pub const SUGGESTED: usize = 8;

/// One place the phrase turns up.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Candidate {
    /// The permanent name — what a citation in a document will carry.
    pub id: SegmentId,
    pub work: String,
    pub he_title: String,
    /// The words around the match, so the reader can see it is the right one:
    /// the match in `[brackets]`, and any elision shown as `…`.
    ///
    /// **Not the whole segment.** It used to be, which meant the CLI cut it to its
    /// first twelve words — so the answer to *where is this from* could be evidence
    /// that did not contain the phrase — and the MCP server serialised the lot, and
    /// the largest segment in the corpus is 1,275,307 characters. One renderer now,
    /// `snippet::of`, windowed on the match.
    pub shown: String,
    /// How long the whole segment is, so a caller knows what `shown` is a window
    /// into. A snippet with no size beside it reads as the whole thing.
    pub characters: usize,
}

/// How the phrase was matched.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "how", rename_all = "snake_case")]
pub enum How {
    /// Letter for letter, with nikud off — which is off in every mode.
    Exactly,
    /// Nothing matched literally, so the ladder was climbed. The rungs are
    /// carried **by name** (`Rung::name()`, the same names an offer travels
    /// under), because a name is data and a sentence is one renderer's
    /// English. This used to hold the engine's whole announcement — counts and
    /// all — and [`Found::describe`] then printed it inside a Hebrew clause:
    /// the count twice, and mid-sentence English.
    Widened { rungs: Vec<String> },
}

/// The rungs, in the language this crate's own sentences are written in.
///
/// The window has its own words for these (`search.ts`'s table); a wire name
/// on the left, a Hebrew clause on the right, and nothing here composing a
/// second English sentence to translate back.
const RUNG_SPOKEN: [(&str, &str); 7] = [
    ("nikud", "ניקוד"),
    ("prefixes", "תחיליות"),
    ("spellings", "כתיב מלא וחסר"),
    ("gershayim", "גרשיים"),
    ("abbreviations", "ראשי תיבות"),
    ("root", "השורש"),
    ("proximity", "רחבה לפסוק"),
];

/// What the corpus has to say about a phrase.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Found {
    /// What was looked for, as it was looked for.
    pub phrase: String,
    pub how: How,
    /// Every place it turns up, not only the ones returned.
    pub total: usize,
    pub candidates: Vec<Candidate>,
    /// Which sefer was left out — the one you are standing in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub except: Option<String>,
    /// Why the ladder was **not** climbed after a literal zero.
    ///
    /// It happens, and the reason is a good one: widening a five-word phrase
    /// into every form of every word is 34,300 exact searches, and W13 refuses
    /// past a limit rather than freezing the window. The refusal is carried
    /// here instead of being swallowed, because *nothing was found* and
    /// *nothing was looked for beyond the literal* are different answers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_widened: Option<String>,
}

impl Found {
    /// Whether this is distinctive enough to be offered as a mekor at all.
    ///
    /// A phrase in 4,000 segments has no source; it has a language. The window
    /// shows the count and offers a search instead of a citation.
    #[must_use]
    pub fn is_a_quotation(&self) -> bool {
        self.total > 0 && self.total <= TOO_COMMON
    }

    /// What to say about it, in one line.
    #[must_use]
    pub fn describe(&self) -> String {
        if self.total == 0 {
            return match &self.not_widened {
                None => "אין בשום ספר".to_string(),
                // The honest form of it: the literal search found nothing and
                // the wider one was not run, which is not the same as "it is
                // nowhere".
                Some(_) => "אין כלשונו — והביטוי ארוך מכדי לחפש בכל צורותיו".to_string(),
            };
        }
        let where_ = if self.total == 1 {
            "במקום אחד".to_string()
        } else {
            format!("ב־{} מקומות", self.total)
        };
        match &self.how {
            How::Exactly if self.is_a_quotation() => where_,
            How::Exactly => format!("{where_} — ביטוי, לא ציטוט"),
            How::Widened { rungs } => {
                let said: Vec<&str> = rungs
                    .iter()
                    .map(|rung| {
                        RUNG_SPOKEN
                            .iter()
                            .find(|(name, _)| name == rung)
                            .map_or(rung.as_str(), |(_, spoken)| spoken)
                    })
                    .collect();
                format!("{where_} (בהרחבה: {})", said.join(", "))
            }
        }
    }

    /// Whether anything beyond the literal was tried, and why not.
    #[must_use]
    pub fn only_literally(&self) -> Option<&str> {
        self.not_widened.as_deref()
    }
}

/// Look a phrase up.
///
/// `except` is the sefer to leave out — the one the phrase came from. With it,
/// the question is *who quotes this*; without it, *where is this from*. One
/// call, because they are one question.
///
/// # Errors
///
/// If the index refuses the query, with the engine's own words.
pub fn where_from(
    bar: &Bar,
    phrase: &str,
    except: Option<&str>,
    limit: usize,
) -> Result<Found, String> {
    let phrase = phrase.trim();
    if phrase.is_empty() {
        return Err("nothing to look for".to_string());
    }

    let mut scope = Scope::everything();
    if let Some(slug) = except {
        scope = scope.without([slug.to_string()], slug);
    }
    let chips = Chips {
        mode: Mode::ToratEmet,
        // The words, one after the other. That is what a quotation is, and it
        // is the one search where anything looser would be an invention.
        together: Together::Phrase,
        scope,
        ..Chips::default()
    };
    let paging = Paging {
        from: 0,
        size: limit.max(1),
    };

    let mut found = gather(bar, phrase, &chips, paging, How::Exactly, except)?;
    if found.total > 0 {
        return Ok(found);
    }

    // Nothing literal. Climb, and say so — a near match presented as a source
    // is the whole failure this module is written around.
    let smart = Chips {
        mode: Mode::Smart,
        ..chips
    };
    match gather(bar, phrase, &smart, paging, How::Exactly, except) {
        Ok(widened) => Ok(widened),
        // The engine refused to widen — a long phrase is too many forms to try
        // (W13). Carried through rather than turned into an error: the literal
        // answer is still an answer, and *nothing was found* is not the same
        // statement as *nothing beyond the literal was looked for*.
        Err(why) => {
            found.not_widened = Some(why);
            Ok(found)
        }
    }
}

fn gather(
    bar: &Bar,
    phrase: &str,
    chips: &Chips,
    paging: Paging,
    how: How,
    except: Option<&str>,
) -> Result<Found, String> {
    match bar.ask(
        phrase,
        chips,
        paging,
        &girsa_ref::resolve::Context::default(),
    ) {
        Answer::Segments { results, rungs, .. } => Ok(Found {
            phrase: phrase.to_string(),
            // The names of what actually ran — not the announcement sentence,
            // which is one renderer's English and used to be printed as
            // though it were the rung.
            how: if rungs.is_empty() {
                how
            } else {
                How::Widened {
                    rungs: rungs.iter().map(|rung| rung.name().to_string()).collect(),
                }
            },
            total: results.total,
            candidates: results
                .hits
                .iter()
                .map(|hit| Candidate {
                    id: hit.id.clone(),
                    work: hit.id.work().to_string(),
                    he_title: bar
                        .catalogue()
                        .facts(hit.id.work())
                        .map_or_else(|| hit.id.work().to_string(), |f| f.title.clone()),
                    shown: crate::snippet::of(&hit.text, &results.marker.marks(hit)).text,
                    characters: hit.text.chars().count(),
                })
                .collect(),
            except: except.map(ToString::to_string),
            not_widened: None,
        }),
        Answer::Refused(why) => Err(why),
        Answer::Cited(_) => Err("a phrase is not a citation".to_string()),
    }
}
