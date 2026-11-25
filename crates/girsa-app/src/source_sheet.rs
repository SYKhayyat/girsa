//! Several places, assembled into one sheet.
//!
//! # What this is for
//!
//! spec.md §9.7 and #63: a reader types `(berachos 2a, shabbos 3b)` and gets
//! **one sheet with both passages on it**, not two visits. The parentheses are
//! the whole of the difference — `berachos 2a, shabbos 3b` without them is two
//! places to go, and with them it is one page to look at. `girsa_app::nav`
//! already reads that difference and carries it as `sheet: bool`; this is what
//! the flag is *for*.
//!
//! # Why it is assembled from references and not from what the panes hold
//!
//! A sheet outlives the panes. Opening two seforim one after another and copying
//! their words into a list would give a sheet that says what the window happened
//! to have; reading each reference back out of the corpus gives one that says
//! what the reader asked for. It is also the only way to answer honestly about a
//! sefer that is **not on this shelf** — a sheet with a hole in it must say so
//! where the hole is, because a blank line reads as a passage with no words.
//!
//! # What it is not
//!
//! Not a `.docx` (`girsa-export` writes those) and not Ksav's block
//! (`girsa-source` and `girsa-sending` make that one). This is the list of
//! passages the window draws, in the order the reader named them, with the
//! reference that puts each back in reach.

use girsa_ref::{Address, Ref};

use crate::shelf::Shelf;

/// One passage on the sheet.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Entry {
    /// The reference as it will be printed and as it can be followed again.
    pub reference: String,
    /// The sefer it came from.
    pub slug: String,
    /// The sefer's name, as the shelf has it.
    pub title: String,
    /// The words themselves, in reading order, paragraphs separated by a space.
    pub words: String,
}

/// One reference that could not be put on the sheet, and the sentence saying why.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Unread {
    /// What the reader wrote, or the canonical reference that failed.
    pub reference: String,
    /// One sentence, in the shelf's own terms.
    pub why: String,
}

/// A source sheet: what could be assembled, and what could not.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Sheet {
    pub entries: Vec<Entry>,
    pub unread: Vec<Unread>,
}

impl Sheet {
    /// True when nothing at all could be put on it.
    ///
    /// The caller says so out loud rather than showing an empty sheet: *none of
    /// those could be found* is a different sentence from *here is your sheet*,
    /// and a blank one reads as the second.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Assemble the passages a reader named, in the order they named them.
///
/// A repeat is kept: if somebody wrote the same reference twice it is on the
/// sheet twice. Dropping it would be this module deciding which of the reader's
/// words to leave out, and a sheet is a request rather than an index.
#[must_use]
pub fn assemble(shelf: &Shelf, references: &[String]) -> Sheet {
    let mut sheet = Sheet::default();
    for raw in references {
        let place: Ref = match raw.parse() {
            Ok(place) => place,
            Err(_) => {
                sheet.unread.push(Unread {
                    reference: raw.clone(),
                    why: format!("{raw} is not a reference this library can read"),
                });
                continue;
            }
        };
        let slug = place.work_slug();
        let sefer = match shelf.read(&slug) {
            Ok(sefer) => sefer,
            Err(_) => {
                sheet.unread.push(Unread {
                    reference: place.to_string(),
                    why: format!("{slug} is not on this shelf"),
                });
                continue;
            }
        };
        let ats = sefer.at(place.from());
        if ats.is_empty() {
            // The sefer is here and the place is not. Said in those words,
            // because *the sefer is missing* and *that place is missing* send a
            // reader to two different places to fix it.
            sheet.unread.push(Unread {
                reference: place.to_string(),
                why: format!("{} has no {}", sefer.work.he_title, spoken(place.from())),
            });
            continue;
        }
        let words: Vec<&str> = ats.iter().map(|at| sefer.as_printed(at)).collect();
        sheet.entries.push(Entry {
            reference: place.to_string(),
            slug,
            title: sefer.work.he_title.clone(),
            words: words.join(" "),
        });
    }
    sheet
}

/// The address as a person would say it: `דף ב׳ ע״א` and not `2a`.
///
/// Deliberately the canonical spelling rather than a pretty one: the sentence it
/// goes into is *this sefer has no …*, and what a reader can act on is the
/// address they typed, in the form the library stores.
fn spoken(address: &Address) -> String {
    if address.levels().is_empty() {
        // A reference to the sefer and nothing inside it: the place that is
        // missing is the sefer's own text, which is a different failure and one
        // the caller hears as the work being empty rather than mis-addressed.
        return "that place".to_string();
    }
    address.to_string()
}
