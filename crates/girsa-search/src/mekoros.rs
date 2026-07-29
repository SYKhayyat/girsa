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

/// One place the phrase turns up.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Candidate {
    /// The permanent name — what a citation in a document will carry.
    pub id: SegmentId,
    pub work: String,
    pub he_title: String,
    /// The segment as printed, so the reader can see it is the right one.
    pub text: String,
}

/// How the phrase was matched.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "how", rename_all = "snake_case")]
pub enum How {
    /// Letter for letter, with nikud off — which is off in every mode.
    Exactly,
    /// Nothing matched literally, so the ladder was climbed. The description
    /// is the engine's own, so what is shown is what ran.
    Widened { rung: String },
}

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
            How::Widened { rung } => format!("{where_} (בהרחבה: {rung})"),
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
        Answer::Segments { results, note, .. } => Ok(Found {
            phrase: phrase.to_string(),
            how: match note {
                // Smart announces what it did; that announcement *is* the
                // rung, so nothing here has to name it a second time.
                Some(announcement) if chips.mode == Mode::Smart => {
                    How::Widened { rung: announcement }
                }
                _ => how,
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
                    text: hit.text.clone(),
                })
                .collect(),
            except: except.map(ToString::to_string),
            not_widened: None,
        }),
        Answer::Refused(why) => Err(why),
        Answer::Cited(_) => Err("a phrase is not a citation".to_string()),
    }
}
