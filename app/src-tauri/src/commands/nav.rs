//! Where the reader says they want to be, and what is done about it.
//!
//! The first domain of `lib.rs` to move out (#68), picked because #63 is the
//! domain being worked on rather than because it is the largest. The rule the
//! split keeps: this file is still **adapter and nothing else** — every
//! decision about what a typed reference *means* is in
//! [`girsa_app::nav::read`], where it is testable without a webview.
//!
//! # Why the answer is five shapes rather than a ref or an error
//!
//! `girsa_ref::resolve` already refuses to guess, and the window has to be able
//! to *show* that refusal. A single `Result<Landing, String>` would flatten
//! "you meant two places", "these words name two seforim" and "nothing matched"
//! into one red message, and those are three different conversations with the
//! reader. So the command returns what the words were:

use girsa_app::nav::{Ask, Nav, Suggest, Unread};
use girsa_app::trouble::{refuse, Code};
use serde::Serialize;

use crate::{post, Shared};

/// What the box got.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum Answer {
    /// One place, and the reader is standing in it.
    Go { landing: post::Landing },
    /// Several places. `sheet` says the reader wrote them in parentheses, which
    /// is the source-sheet spelling: the same refs, assembled rather than
    /// opened one after another.
    ///
    /// `unread` is what could not be answered, and it travels with the refs
    /// rather than replacing them: a list with one bad member is a true partial
    /// answer, and answering *nothing matched* to it was a refusal that was not
    /// true (issue #74).
    Several {
        refs: Vec<String>,
        unread: Vec<UnreadRow>,
        sheet: bool,
    },
    /// Several places those words genuinely name. Always shown as a choice.
    Choose { refs: Vec<String> },
    /// A location with no sefer — `2a` — and the window asks which one.
    Ask { about: String },
    /// Nothing matched, and the words are neither a place nor a name.
    Nothing,
    /// The lexicon does not hold that spelling and one sefer's name is close.
    ///
    /// **A question and not a landing** (#63): the box shows it and the reader
    /// decides. This is the answer `nav` alone could not give — `read` stops at
    /// the lexicon, and *that sefer is not in my lexicon* is not the same sentence
    /// as *there is nothing to say*.
    DidYouMean {
        /// What they typed, so the question can quote it back.
        typed: String,
        /// The sefer that is close.
        slug: String,
        /// Its name as the shelf has it, which is what the question offers.
        title: String,
        /// Their own line with the name corrected and the address kept: what the
        /// window puts back in the box when the reader accepts it.
        corrected: String,
    },
    /// …and more than one sefer is close, so it is a choice rather than a guess.
    Which {
        typed: String,
        candidates: Vec<MaybeRow>,
    },
}

/// One sefer a typed name could have meant.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MaybeRow {
    slug: String,
    title: String,
    corrected: String,
}

/// Assemble a parenthesised list into one sheet (#63, spec.md §9.7).
///
/// `(berachos 2a, shabbos 3b)` is one page with both passages on it, and the
/// reading of the parentheses is `nav`'s — this command is handed the refs the
/// reader's line produced and does the finding, which is `girsa_app::source_sheet`
/// because *which segments does this reference name* is a question about the
/// corpus and not about a webview.
///
/// A reference that cannot be found comes back in `unread` beside the ones that
/// could, with a sentence: a sheet with a hole in it that said nothing would read
/// as *these are the passages you asked for* and be one short.
///
/// # Errors
///
/// If the lock is poisoned.
#[tauri::command(async)]
pub(crate) fn sheet_open(
    shared: tauri::State<'_, Shared>,
    refs: Vec<String>,
) -> Result<girsa_app::source_sheet::Sheet, String> {
    let state = shared.lock().map_err(|_| crate::State::poisoned())?;
    // The lock is taken, read through, and dropped before anything is built: the
    // sheet is a read of the corpus and nothing about it wants the state.
    let shelf = state.shelf()?;
    Ok(girsa_app::source_sheet::assemble(&shelf, &refs))
}

/// One part of a list that could not be read, and why in a word.
///
/// The window says all three differently, because they are three different
/// things for a reader to do: pick a sefer, correct a name, or choose which
/// seforim those words meant.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum UnreadRow {
    /// A place with no sefer, exactly as [`Ask`] carries it on its own.
    WhichSefer { typed: String },
    /// A name the lexicon does not hold.
    Nothing { typed: String },
    /// Words that name more than one sefer, with the choice still attached.
    Choice { typed: String, refs: Vec<String> },
}

impl From<Unread> for UnreadRow {
    fn from(unread: Unread) -> Self {
        match unread {
            Unread::WhichSefer(typed) => Self::WhichSefer { typed },
            Unread::Nothing(typed) => Self::Nothing { typed },
            Unread::Choice { typed, candidates } => Self::Choice {
                typed,
                refs: candidates.iter().map(ToString::to_string).collect(),
            },
        }
    }
}

/// Read the box: go, assemble, choose, ask, or say nothing matched.
///
/// # Errors
///
/// If there is no lexicon — which means the corpus did not open, and which the
/// window already says in the shelf's own words — or if the lock is poisoned.
#[tauri::command]
pub(crate) fn nav_ask(shared: tauri::State<'_, Shared>, typed: String) -> Result<Answer, String> {
    let mut state = shared.lock().map_err(|_| crate::State::poisoned())?;

    // `suggest` and not `read`: the lexicon first, and the shelf's own names only
    // where the lexicon had nothing — a question in that case, never a landing.
    let said = {
        let shelf = state.shelf()?;
        match state.lexicon.as_ref() {
            Some(lexicon) => girsa_app::nav::suggest(lexicon, shelf.works(), &typed),
            None => {
                return Err(refuse(
                    Code::NoShelf,
                    "no corpus is open, so there is nothing to read a reference against",
                ));
            }
        }
    };
    let nav = match said {
        Suggest::Read(nav) => nav,
        Suggest::DidYouMean {
            typed,
            slug,
            title,
            corrected,
        } => {
            return Ok(Answer::DidYouMean {
                typed,
                slug,
                title,
                corrected,
            });
        }
        Suggest::Which { typed, candidates } => {
            return Ok(Answer::Which {
                typed,
                candidates: candidates
                    .into_iter()
                    .map(|maybe| MaybeRow {
                        slug: maybe.slug,
                        title: maybe.title,
                        corrected: maybe.corrected,
                    })
                    .collect(),
            });
        }
    };

    // The decision is made before the state is borrowed mutably again, because
    // landing the reader moves it: the borrow `nav` holds is over.
    match nav {
        Nav::Go(one) => Ok(Answer::Go {
            landing: post::landing(&mut state, &one)?,
        }),
        Nav::Several {
            refs,
            unread,
            sheet,
        } => Ok(Answer::Several {
            refs: refs.iter().map(ToString::to_string).collect(),
            unread: unread.into_iter().map(UnreadRow::from).collect(),
            sheet,
        }),
        Nav::Choose(refs) => Ok(Answer::Choose {
            refs: refs.iter().map(ToString::to_string).collect(),
        }),
        Nav::Ask(Ask::WhichSefer(about)) => Ok(Answer::Ask { about }),
        Nav::Nothing => Ok(Answer::Nothing),
    }
}
