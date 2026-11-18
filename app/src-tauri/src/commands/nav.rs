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

use girsa_app::nav::{Ask, Nav, Unread};
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
    /// Nothing in the lexicon. Better than a wrong answer, and said so.
    Nothing,
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

    let nav = match state.lexicon.as_ref() {
        Some(lexicon) => girsa_app::nav::read(lexicon, &typed),
        None => {
            return Err(refuse(
                Code::NoShelf,
                "no corpus is open, so there is nothing to read a reference against",
            ));
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
