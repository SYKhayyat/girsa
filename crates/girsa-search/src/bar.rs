//! The search bar: five modes, three chips, five facets, one call (W14).
//!
//! Everything under this module is a mode or a control; this is the thing that
//! puts them together, and it exists so there is exactly **one** place that
//! decides what a typed query means. The window and the command line are two
//! surfaces over this, not two implementations of it — the alternative is a
//! search box that behaves differently depending on where you typed into it.
//!
//! # What every answer carries
//!
//! A header saying what was searched for, the hits, the total, and the facets.
//! The header is built from the thing that **ran** — a plan, a widening, an
//! instrument — and never from the text the reader typed, so it cannot describe
//! a search that did not happen.

use std::path::{Path, PathBuf};

use girsa_corpus::import;
use girsa_ref::resolve::Context;

use crate::chips::{Chips, Sounding};
use crate::citation::{Citations, Landing};
use crate::facets::{Catalogue, Facets};
use crate::index::{Counts, Found, Hit, Paging, Prepared, SearchIndex};
use crate::instruments::{Counting, Instrument, InstrumentError, Stream, Where};
use crate::ladder::{Offers, Widening};
use crate::regex_mode;
use crate::smart::Smart;
use crate::torat_emet::{self, Plan};
use crate::Mode;

/// How many seforim a dilug will read through.
///
/// A dilug is a scan over the letters of a sefer — it is not an index question
/// (see [`SearchIndex::search_instrument`]) — so it is bounded by the scope
/// chip. Past this many seforim it is refused **with the number**, rather than
/// run over some of them.
pub const MOST_SEFORIM_FOR_A_DILUG: usize = 8;

/// How many of an instrument's words are named in the note.
///
/// 1,407 words of this corpus come to 611, and a line with all of them in it is
/// a line nobody reads. What is cut is **counted** — a list that silently stops
/// reads as all of them — and the words themselves are all still on
/// [`crate::index::Sounded::words`], which is what the highlight marks by.
pub const WORDS_NAMED: usize = 12;

/// What is highlighted in a hit, and by which rule.
///
/// A highlight has to agree with the search exactly. The literal mode marks the
/// words it asked for, a widened search marks the word that actually answered,
/// and an instrument marks whatever the instrument reached — so the rule
/// travels with the answer rather than being guessed at by whoever draws it.
#[derive(Debug, Clone)]
pub enum Marker {
    Literal(Plan),
    Widened(Box<Widening>),
    /// These exact words, whichever of them a segment holds.
    Words(Vec<String>),
    /// The words of a notarikon, wherever a run of them stands together.
    Notarikon {
        letters: String,
        at: Where,
    },
    /// Nothing to mark — a regex hit, or a citation.
    Nothing,
}

impl Marker {
    /// Where in a hit's printed text to draw the marks, as byte spans.
    #[must_use]
    pub fn marks(&self, hit: &Hit) -> Vec<(usize, usize)> {
        match self {
            Self::Literal(plan) => hit.marks(plan),
            Self::Widened(widening) => {
                crate::index::spans_where(&hit.text, |word| widening.matches_word(word))
            }
            Self::Words(words) => {
                crate::index::spans_where(&hit.text, |word| words.iter().any(|w| w == word))
            }
            Self::Notarikon { letters, at } => {
                crate::instruments::notarikon_in(&hit.text, letters, *at)
            }
            Self::Nothing => Vec::new(),
        }
    }
}

/// One page of results, whatever mode found them.
#[derive(Debug, Clone)]
pub struct Results {
    /// What was searched for, read off what ran.
    pub header: String,
    pub hits: Vec<Hit>,
    pub total: usize,
    pub facets: Facets,
    pub marker: Marker,
}

/// What the bar has to say.
#[derive(Debug, Clone)]
pub enum Answer {
    /// Segments, from any mode that returns them.
    Segments {
        results: Box<Results>,
        /// The relaxation ladder, priced and **not applied** — empty except in
        /// the literal mode on a zero (spec.md §9.6).
        offers: Offers,
        /// What the mode did, where it did something worth announcing: Smart's
        /// widening, or the words a gematria added up.
        note: Option<String>,
        /// A place the words also read as, **offered and not taken**.
        ///
        /// # The best thing in this engine was behind a sigil
        ///
        /// The resolver lands `שבת לא.` on the daf and `משנה ברורה סימן ש` on
        /// siman 300 of a 17,418-segment sefer — and it could only be reached by
        /// typing `@`, which nothing on any screen taught. What a reader who
        /// typed those got instead was 92,384 and 12 word hits, and there is no
        /// other *go to a place* control in the application.
        ///
        /// Switching the mode for them would be the one thing spec.md §9 and
        /// BUILDER.md rule 6 forbid: changing what was asked without saying so.
        /// So the words search runs, its count is honest, and the place is put
        /// **above** the hits as an offer — the same shape as the relaxation
        /// ladder, which is priced before the click and applied only by one.
        landing: Option<Box<Landing>>,
    },
    /// A mareh makom: a jump, or a choice, or neither.
    Cited(Box<Landing>),
    /// Refused, and why. Never a shorter list of results with no note attached.
    Refused(String),
}

/// The bar: an index, a catalogue, and the resolver's lexicon.
pub struct Bar {
    index: SearchIndex,
    catalogue: Catalogue,
    /// The citation resolver, **read on first use and not before**.
    ///
    /// 3.7 MB of `lexicon.tsv`, walked twice — once by `read_spellings` and
    /// once by `Lexicon::from_tsv` — and it was read in `Bar::new`, so every
    /// `girsa-index find` in every one of the five modes paid for Citation
    /// mode. Four of them never look at it.
    citations: std::sync::OnceLock<Option<Citations>>,
    root: PathBuf,
}

impl std::fmt::Debug for Bar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bar")
            .field("index", &self.index)
            .field("seforim", &self.catalogue.len())
            .field("citations", &self.citations.get().map(Option::is_some))
            .finish()
    }
}

impl Bar {
    /// Put a bar over an index and the corpus it was built from.
    ///
    /// The citation mode needs the lexicon, which lives with the corpus. A
    /// shelf without one still searches — every other mode is the index alone —
    /// and citation then refuses **with the reason** rather than resolving
    /// nothing and looking like an empty library.
    #[must_use]
    pub fn new(index: SearchIndex, catalogue: Catalogue, root: &Path) -> Self {
        Self {
            index,
            catalogue,
            citations: std::sync::OnceLock::new(),
            root: root.to_path_buf(),
        }
    }

    /// The citation resolver, reading the lexicon if this is the first ask.
    ///
    /// `None` when there is no lexicon, which is a shelf that has not been
    /// imported — and Citation mode then refuses **with the reason** rather
    /// than resolving nothing and looking like an empty library.
    fn citations(&self) -> Option<&Citations> {
        self.citations
            .get_or_init(|| Citations::open(&self.root).ok())
            .as_ref()
    }

    #[must_use]
    pub fn index(&self) -> &SearchIndex {
        &self.index
    }

    #[must_use]
    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    /// Where a sefer this bar can see sits on the shelf, and what it is called.
    pub fn catalogue_mut(&mut self) -> &mut Catalogue {
        &mut self.catalogue
    }

    /// Ask.
    ///
    /// `context` is where the reader is standing, and it is used by exactly one
    /// mode: a partial citation completed against the pane they are in
    /// (spec.md §4.3). Nothing else in this engine reads it, because nothing
    /// else in this engine is allowed to use where you are to change what you
    /// asked for.
    #[must_use]
    pub fn ask(&self, typed: &str, chips: &Chips, paging: Paging, context: &Context) -> Answer {
        let (chips, text) = chips.read(typed);
        if text.trim().is_empty() {
            return Answer::Refused("nothing to search for".to_string());
        }
        let result = match chips.mode {
            Mode::ToratEmet => self.literally(&text, &chips, paging),
            Mode::Smart => self.smartly(&text, &chips, paging),
            Mode::Regex => self.by_pattern(&text, &chips, paging),
            Mode::Citation => return self.cited(&text, context),
            Mode::Instruments => self.by_instrument(&text, &chips, paging),
        };
        match result {
            Ok(Answer::Segments {
                results,
                offers,
                note,
                landing: _,
            }) => Answer::Segments {
                results,
                offers,
                note,
                // The words were searched for and the count is honest. If they
                // also read as a place, that is put in front of the reader as
                // an **offer** — see the field's note. `Regex` is left out
                // because a pattern is not a mareh makom in any reading of it.
                landing: (chips.mode != Mode::Regex)
                    .then(|| self.also_a_place(&text, context))
                    .flatten(),
            },
            Ok(answer) => answer,
            Err(why) => Answer::Refused(why),
        }
    }

    /// The place these words also name, when they name one that is really there.
    ///
    /// **Only a landing, never a near miss.** `look_up` reports what it could
    /// not rule out as well as what it found, which is right for Citation mode
    /// — a reader who typed `@` asked a question and is owed the whole answer,
    /// including *this sefer is here and has no such daf*. Here nobody asked:
    /// these are words that happen to parse. Offering *did you mean* over a
    /// perfectly good word search would be the resolver interrupting somebody
    /// looking for a phrase.
    fn also_a_place(&self, text: &str, context: &Context) -> Option<Box<Landing>> {
        let landing = self.citations()?.look_up(text, context);
        (!landing.places.is_empty()).then(|| Box::new(landing))
    }

    /// Torat Emet: what you typed is what was searched for.
    fn literally(&self, text: &str, chips: &Chips, paging: Paging) -> Result<Answer, String> {
        let query = torat_emet::Query::new(text)
            .matching(chips.matching)
            .together(chips.together);
        // Built once, and asked twice — the hits and the facet counts. This
        // used to build one here for the facets and let `search_in` build a
        // second, private one for the hits, against a doc comment on `Prepared`
        // saying it is *"built once and asked three times, because a facet
        // computed from a differently-built copy of it would be a column of
        // numbers that did not add up to the header."*
        let prepared = self.index.prepare(&query, &chips.scope).map_err(say)?;
        let found = self
            .index
            .found_with(&prepared, query.plan(), None, paging)
            .map_err(say)?;
        // The ladder is offered on a zero and **never applied** — the counts
        // are worked out from the query the click would run, before the click.
        let offers = if found.total == 0 {
            self.index.offers_in(&query, &chips.scope)
        } else {
            Offers::default()
        };
        let results = self.results(found.asked.describe(), &found, Some(&prepared))?;
        Ok(Answer::Segments {
            results: Box::new(results),
            offers,
            note: None,
            landing: None,
        })
    }

    /// Smart: widen, and say so.
    fn smartly(&self, text: &str, chips: &Chips, paging: Paging) -> Result<Answer, String> {
        let query = torat_emet::Query::new(text)
            .matching(chips.matching)
            .together(chips.together);
        let answered = Smart::new(query)
            .run_in(&self.index, &chips.scope, paging)
            .map_err(say)?;
        // The query that ran, not one built afterwards to look like it — see
        // `Answered::prepared`. This was `prepare_widened(&answered.widened)`,
        // reading the very field whose doc comment forbade rebuilding.
        let prepared = &answered.prepared;
        let header = answered
            .found
            .widening
            .as_ref()
            .map_or_else(|| answered.found.asked.describe(), Widening::describe);
        let results = self.results(header, &answered.found, Some(prepared))?;
        Ok(Answer::Segments {
            results: Box::new(results),
            offers: Offers::default(),
            note: Some(answered.announcement()),
            landing: None,
        })
    }

    /// Regex: full power, no hand-holding, and no ladder on a zero.
    fn by_pattern(&self, text: &str, chips: &Chips, paging: Paging) -> Result<Answer, String> {
        let query = regex_mode::Query::parse(text, chips.together).map_err(say)?;
        let prepared = self
            .index
            .prepare_regex(&query, &chips.scope)
            .map_err(say)?;
        let (hits, total) = self.index.page(&prepared, paging).map_err(say)?;
        Ok(Answer::Segments {
            results: Box::new(self.assembled(
                query.describe(),
                hits,
                total,
                Marker::Nothing,
                Some(&prepared),
            )?),
            offers: Offers::default(),
            note: None,
            landing: None,
        })
    }

    /// Citation: type a mareh makom, jump.
    fn cited(&self, text: &str, context: &Context) -> Answer {
        let Some(citations) = self.citations() else {
            return Answer::Refused(format!(
                "there is no lexicon under {} — citations cannot be resolved until girsa-import \
                 has run",
                self.root.display()
            ));
        };
        Answer::Cited(Box::new(citations.look_up(text, context)))
    }

    /// Instruments: gematria, notarikon, atbash — and dilug, which is a scan.
    fn by_instrument(&self, text: &str, chips: &Chips, paging: Paging) -> Result<Answer, String> {
        let instrument = read_instrument(text, chips).map_err(say)?;
        // The two that are read off the text rather than asked of the index.
        // Both are bounded by the scope chip, and both say so when it is not
        // narrow enough — see [`Bar::over_the_text`].
        match &instrument {
            Instrument::Dilug { letters, skips } => {
                let letters = letters.clone();
                let skips = skips.clone();
                return self.over_the_text(&instrument, chips, Marker::Nothing, |work| {
                    let stream = Stream::of(work.segments.iter().map(|s| s.text.as_str()));
                    let mut at: Vec<usize> = stream
                        .dilug(&letters, &skips)
                        .into_iter()
                        .flat_map(|found| found.segments)
                        .collect();
                    at.sort_unstable();
                    at.dedup();
                    at
                });
            }
            Instrument::Notarikon { letters, at } => {
                let letters = letters.clone();
                let at = *at;
                return self.over_the_text(
                    &instrument,
                    chips,
                    Marker::Notarikon {
                        letters: letters.clone(),
                        at,
                    },
                    move |work| {
                        work.segments
                            .iter()
                            .enumerate()
                            .filter(|(_, segment)| {
                                !crate::instruments::notarikon_in(&segment.text, &letters, at)
                                    .is_empty()
                            })
                            .map(|(i, _)| i)
                            .collect()
                    },
                );
            }
            _ => {}
        }
        let sounded = self
            .index
            .prepare_instrument(&instrument, &chips.scope)
            .map_err(say)?;
        let (hits, total) = self.index.page(&sounded.prepared, paging).map_err(say)?;
        // Which words of the corpus came to the number is half the answer, and
        // for gematria it is the more interesting half.
        let note = (!sounded.words.is_empty()).then(|| {
            let named = sounded
                .words
                .iter()
                .take(WORDS_NAMED)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            let rest = sounded.words.len().saturating_sub(WORDS_NAMED);
            format!(
                "{} word{} of the corpus: {named}{}",
                sounded.words.len(),
                if sounded.words.len() == 1 { "" } else { "s" },
                if rest > 0 {
                    format!(" … and {rest} more")
                } else {
                    String::new()
                }
            )
        });
        let marker = Marker::Words(sounded.words.clone());
        Ok(Answer::Segments {
            results: Box::new(self.assembled(
                instrument.describe(),
                hits,
                total,
                marker,
                Some(&sounded.prepared),
            )?),
            offers: Offers::default(),
            note,
            landing: None,
        })
    }

    /// The two instruments that are read off the text (W14).
    ///
    /// A dilug runs through the letters of a sefer and ignores where words end;
    /// a notarikon is four patterns each matching half the vocabulary. Neither
    /// is a question an inverted index can answer, so both are scans — and a
    /// scan is bounded by **the scope chip** rather than by a ceiling nobody
    /// chose. Over more than [`MOST_SEFORIM_FOR_A_DILUG`] seforim it is refused
    /// with the number, because reading some of the shelf and reporting it as
    /// the shelf is a finding nobody could check.
    ///
    /// `found` is given one work and answers with the places in its reading
    /// order that hold the thing. One shape for both, so the bound, the
    /// refusal and the note cannot come to mean two different things.
    fn over_the_text(
        &self,
        instrument: &Instrument,
        chips: &Chips,
        marker: Marker,
        found: impl Fn(&import::ImportedWork) -> Vec<usize>,
    ) -> Result<Answer, String> {
        let seforim: Vec<String> = chips.scope.works().into_iter().collect();
        if seforim.is_empty() || seforim.len() > MOST_SEFORIM_FOR_A_DILUG {
            return Err(format!(
                "{} is read off the text of a sefer, so it needs one — narrow the scope to at \
                 most {MOST_SEFORIM_FOR_A_DILUG} seforim (this one names {})",
                instrument.describe(),
                if seforim.is_empty() {
                    "the whole shelf".to_string()
                } else {
                    seforim.len().to_string()
                }
            ));
        }

        let mut hits = Vec::new();
        for slug in &seforim {
            let work = import::read_back(&self.root, slug).map_err(say)?;
            for at in found(&work) {
                if let Some(segment) = work.segments.get(at) {
                    hits.push(Hit {
                        id: segment.id.clone(),
                        kind: segment.kind,
                        text: segment.text.clone(),
                        // Not scored. It is there or it is not, and a relevance
                        // number on it would be decoration.
                        score: 0.0,
                        // The instruments walk the segments on disk rather
                        // than the index, and the words of a scan are not
                        // there. A page of a scan reaches an instrument with
                        // no words, which is the honest answer: nobody has
                        // counted the gematria of a photograph.
                        by: None,
                    });
                }
            }
        }
        let total = hits.len();
        Ok(Answer::Segments {
            results: Box::new(self.assembled(instrument.describe(), hits, total, marker, None)?),
            offers: Offers::default(),
            note: Some(format!(
                "read through {} sefer{} of text, not the index",
                seforim.len(),
                if seforim.len() == 1 { "" } else { "im" }
            )),
            landing: None,
        })
    }

    /// One page, with its facets, from the query that produced it.
    fn results(
        &self,
        header: String,
        found: &Found,
        prepared: Option<&Prepared>,
    ) -> Result<Results, String> {
        // `Found::marker`, not a second reading of the same two fields. This
        // was four lines here and four more in `Found::marks`, and each had a
        // caller: `girsa-index find` highlighted through one, the window
        // through the other.
        self.assembled(
            header,
            found.hits.clone(),
            found.total,
            found.marker(),
            prepared,
        )
    }

    fn assembled(
        &self,
        header: String,
        hits: Vec<Hit>,
        total: usize,
        marker: Marker,
        prepared: Option<&Prepared>,
    ) -> Result<Results, String> {
        Ok(Results {
            facets: self.facets_of(&hits, total, prepared)?,
            header,
            hits,
            total,
            marker,
        })
    }

    /// Count the facets over the **whole** result set, not over the page.
    ///
    /// From the same built query the hits came from, so the facet column and
    /// the header cannot disagree. A dilug has no such query — it is a scan —
    /// and there the hits **are** all of them, so counting them is counting
    /// everything rather than counting a page.
    fn facets_of(
        &self,
        hits: &[Hit],
        total: usize,
        prepared: Option<&Prepared>,
    ) -> Result<Facets, String> {
        let counts = match prepared {
            Some(prepared) => self.index.tally(prepared).map_err(say)?,
            None => {
                let mut counts = Counts {
                    total,
                    link_types_built: self.index.report().is_some_and(|r| r.link_types),
                    ..Counts::default()
                };
                for hit in hits {
                    *counts.by_work.entry(hit.id.work().to_string()).or_default() += 1;
                }
                counts
            }
        };
        Ok(Facets::of(&counts, &self.catalogue))
    }
}

/// What the instrument chip says to do with what was typed.
///
/// The chip decides, not the text. A box that guessed *this looks like a
/// gematria* from what was typed into it would be the one thing spec.md §9.5
/// forbids: a control with no visible state.
fn read_instrument(text: &str, chips: &Chips) -> Result<Instrument, InstrumentError> {
    match chips.sounding {
        Sounding::Gematria => Instrument::gematria(text, Counting::Standard),
        Sounding::Rashei | Sounding::Sofei => {
            Instrument::notarikon(text, chips.sounding.at().unwrap_or(Where::Start))
        }
        Sounding::Atbash => Instrument::atbash(text),
        Sounding::Dilug => Instrument::dilug(text, chips.skips.from..=chips.skips.to),
    }
}

/// An error, in the words a reader gets.
fn say(error: impl std::fmt::Display) -> String {
    error.to_string()
}
