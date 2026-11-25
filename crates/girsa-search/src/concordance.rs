//! A concordance: one form, and **every** place the shelf says it.
//!
//! The other half of [`crate::mekoros`], asked upside-down. There the reader has
//! a phrase and wants the places it came from; here they have a word and want
//! every place it is said. Same index, same tokenizer, and the difference is what
//! the answer is allowed to leave out: a mekor may be one candidate, a concordance
//! that silently dropped places would be a concordance of nothing in particular.
//!
//! # What it is not
//!
//! **Not a search.** A search ranks, pages and — when nothing matches — climbs
//! the ladder (W13). A concordance does none of those three:
//!
//! - it **does not rank**, because there is nothing to rank: every hit matched the
//!   same form, so a "best" one would be tantamount to a claim about which verse
//!   matters more;
//! - it **does not page**, because *list every instance* is the question. It does
//!   stop at [`AT_MOST`] places and says so — a bound that is printed is not the
//!   same promise as a page that pretends there is no other;
//! - it **does not widen**. `ויאמר` is in ten thousand places and none of them was
//!   matched by a prefix. An answer of zero here means the form is not in the
//!   shelf, and the reader can take it to the search bar, which is the surface that
//!   guesses.
//!
//! # One form, not a sentence
//!
//! A two-word ask is refused by name rather than quietly turned into a phrase
//! search, because *"every place this word is said"* and *"every place these two
//! words are said together"* are different questions with different counts, and a
//! reader who typed the second one and got the first would never know. `find` and
//! `where_from` are the two commands that answer sentences.
//!
//! # The form as typed and the form as indexed
//!
//! The index holds **normal forms**: nikud off, final letters folded, maqaf a
//! boundary (W11). So a reader who selects `וַיֹּאמֶר` and asks for a concordance is
//! answered about `ויאמר`, and the two are reported separately. That is not pedantry:
//! it is the difference between *this pointed word, these three places* and *this
//! word however it is spelled, these nine hundred*, and it is the same distinction
//! [`crate::mekoros::Found::as_typed`] makes for phrases.
//!
//! # Address order
//!
//! Places come back in [`SegmentId`] order — reading order within a work
//! (`girsa_corpus::segment` says why path order would put siman 10 before siman 9).
//! A reader walking a word through the shelf is going somewhere; ordering by score
//! would have them jump about, and ordering by the index's own insert order would
//! make the list change when the index was rebuilt.

use girsa_corpus::segment::SegmentId;

use crate::bar::{Answer, Bar};
use crate::chips::Chips;
use crate::index::Paging;
use crate::scope::Scope;
use crate::torat_emet::Together;
use crate::Mode;

/// How many places an answer lists when the asker did not name a number.
///
/// Five hundred is a scroll rather than a dump, and it is far above what any word
/// a reader asks a concordance about actually needs: the corpus's common words
/// (`את`, `ויאמר`) run to the tens of thousands, which is not a list anybody reads
/// — and it is exactly those words whose count is the interesting part. The total
/// travels beside the places so the count is never lost to the bound.
pub const AT_MOST: usize = 500;

/// What the asker will accept in an answer.
#[derive(Debug, Clone, Default)]
pub struct Asking {
    /// Where to look, when it is not the whole shelf.
    ///
    /// The pane's concordance and the shelf-wide one are the same question with
    /// one chip different (spec.md §9.8), and this is that chip.
    pub scope: Option<Scope>,
    /// How many places to list. Zero means [`AT_MOST`].
    pub at_most: usize,
}

impl Asking {
    /// The bound this ask resolves to.
    #[must_use]
    pub fn bound(&self) -> usize {
        if self.at_most == 0 {
            AT_MOST
        } else {
            self.at_most
        }
    }
}

/// One place the form is said.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Place {
    /// The permanent name — what a landing opens and what a citation carries.
    pub id: SegmentId,
    pub work: String,
    /// What the shelf calls this sefer, so a list of forty places can say which
    /// seforim they are in without the window holding a catalogue of its own.
    pub he_title: String,
    /// The words around the match, the match in `[brackets]` — one renderer,
    /// [`crate::snippet::of`], windowed on the match (see its own note for why
    /// there is only one).
    pub shown: String,
    /// How long the whole segment is, so `shown` is a window into something sized.
    pub characters: usize,
    /// Where the form sits in the segment **as printed**: byte spans, the same
    /// currency [`crate::bar::Marker::marks`] speaks.
    ///
    /// Not derived from `shown`: that string has brackets added and characters
    /// dropped, so offsets into it are offsets into a rendering and not into the
    /// sefer. These are the marks `Marker` produced, which are what the pane
    /// highlights when it lands — the acceptance of #48 is that a concordance hit
    /// is never a bare jump to a verse with no position.
    pub marks: Vec<(usize, usize)>,
}

/// What the shelf has to say about a form.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Found {
    /// The form as the reader wrote it.
    pub form: String,
    /// The form the index was asked about: the normalized one.
    pub indexed: String,
    /// Every place it is said, not only the ones listed.
    pub total: usize,
    /// How many seforim the **total** is spread over — counted over the whole
    /// result set, not over the places listed.
    ///
    /// It came from the listed places first, and on the real shelf that said
    /// *975 places in 5 seforim* while listing five of them: the bound was
    /// recomputing a fact about a set it had cut. The facet column already counts
    /// per sefer over everything (`Results::facets`, built from the same query the
    /// hits came from), so the number is taken from there.
    pub seforim: usize,
    /// How many places are in [`Found::places`].
    pub shown: usize,
    /// The places, in reading order.
    pub places: Vec<Place>,
}

impl Found {
    /// Whether the list is shorter than the count — the one thing a bounded list
    /// must never be quiet about.
    #[must_use]
    pub fn is_cut(&self) -> bool {
        self.shown < self.total
    }

    /// Whether the reader typed the form the index holds.
    ///
    /// `false` when nikud, a final letter or gershayim were folded away, which is
    /// the difference between *these places* and *these places, for that word*.
    #[must_use]
    pub fn as_typed(&self) -> bool {
        self.form == self.indexed
    }

    /// The count, in the language this crate's sentences are written in.
    ///
    /// The window has its own words (`say.ts`); this is for the two callers that
    /// print: `girsa-index concordance` and whatever comes next.
    #[must_use]
    pub fn describe(&self) -> String {
        if self.total == 0 {
            return if self.as_typed() {
                "אין בשום ספר".to_string()
            } else {
                format!("אין בשום ספר בצורה ״{}״", self.indexed)
            };
        }
        let where_ = if self.total == 1 {
            "במקום אחד".to_string()
        } else {
            format!("ב־{} מקומות", self.total)
        };
        let seforim = match self.seforim {
            0 => String::new(),
            1 => "בספר אחד".to_string(),
            n => format!("ב־{n} ספרים"),
        };
        let mut said = if seforim.is_empty() {
            where_
        } else {
            format!("{where_} {seforim}")
        };
        if !self.as_typed() {
            said.push_str(&format!(" — לפי ״{}״", self.indexed));
        }
        if self.is_cut() {
            said.push_str(&format!(" (מוצגים {})", self.shown));
        }
        said
    }
}

/// Every place the shelf says this form.
///
/// # Errors
///
/// If the form is empty, or is more than one word — a concordance is a form, and
/// the sentence says which two commands answer sentences instead.
pub fn concordance(bar: &Bar, form: &str, asking: Asking) -> Result<Found, String> {
    let form = form.trim();
    if form.is_empty() {
        return Err("אין מה לחפש — קונקורדנציה היא מילה אחת".to_string());
    }
    // **The words, not the string.** `tokenize` is the same rule the index was
    // built under, so `בית־המקדש` is two words here for the same reason it is two
    // terms in the index. A form whose token count is not one is a phrase, and
    // phrases have their own two commands.
    let words: Vec<String> = girsa_hebrew::tokenize(form)
        .into_iter()
        .map(|word| word.text)
        .filter(|word| !word.trim().is_empty())
        .collect();
    if words.len() != 1 {
        return Err(format!(
            "קונקורדנציה היא מילה אחת, וזו {} מילים — לביטוי יש `find` ו־`where-from`",
            words.len()
        ));
    }
    let indexed = girsa_hebrew::normalize(form);

    let bound = asking.bound();
    let chips = Chips {
        mode: Mode::ToratEmet,
        // One token, so *all of them anywhere* and *all of them in order* are the
        // same query; phrase keeps this function from acquiring a second meaning
        // the day a token stops being one token.
        together: Together::Phrase,
        scope: asking.scope.unwrap_or_else(Scope::everything),
        ..Chips::default()
    };

    // Pages, until the bound or the end. Each ask re-runs the query, so the loop
    // is `total`-bounded and stops on the first page that comes back short — a
    // reader asking for 500 places of a word in 30 segments makes one request.
    let mut places: Vec<Place> = Vec::new();
    let mut total = 0;
    let mut seforim = 0;
    let mut seen = 0;
    while places.len() < bound {
        let want = bound - places.len();
        match bar.ask(
            form,
            &chips,
            Paging {
                from: seen,
                size: want,
            },
            &girsa_ref::resolve::Context::default(),
        ) {
            Answer::Segments { results, .. } => {
                total = results.total;
                // Over the whole set, from the facet column — see `Found::seforim`
                // for what it said when this was counted from the page.
                seforim = results.facets.sefer.len();
                if results.hits.is_empty() {
                    break;
                }
                seen += results.hits.len();
                for hit in &results.hits {
                    places.push(Place {
                        id: hit.id.clone(),
                        work: hit.id.work().to_string(),
                        he_title: bar
                            .catalogue()
                            .facts(hit.id.work())
                            .map_or_else(|| hit.id.work().to_string(), |f| f.title.clone()),
                        shown: crate::snippet::of(&hit.text, &results.marker.marks(hit)).text,
                        characters: hit.text.chars().count(),
                        marks: results.marker.marks(hit),
                    });
                }
                if seen >= total {
                    break;
                }
            }
            Answer::Refused(why) => return Err(why),
            // A citation or an instrument answer cannot come back from these
            // chips; carrying them as "nothing" would be a lie about the form.
            other => return Err(format!("{other:?}")),
        }
    }

    // Reading order, not the index's ranking. `SegmentId`'s `Ord` is ordinal
    // order within a work and slug order across works — the shelf's own order.
    places.sort_by(|a, b| a.id.cmp(&b.id));
    places.dedup_by(|a, b| a.id == b.id);

    Ok(Found {
        form: form.to_string(),
        indexed,
        total,
        seforim,
        shown: places.len(),
        places,
    })
}
