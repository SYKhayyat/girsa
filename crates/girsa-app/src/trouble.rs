//! A refusal, with a name on it.
//!
//! # What the window was doing instead
//!
//! `app/src/trouble.ts` turned an error into a Hebrew sentence by matching
//! **twenty-one regular expressions against the English prose of Rust's
//! `Display` impls**:
//!
//! ```ts
//! { match: /no search index/i, said: () => "אין אינדקס חיפוש — יש לבנות אותו: girsa-index build" },
//! { match: /no sefer here called/i, said: () => "אין ספר בשם הזה במדף" },
//! { match: /state is poisoned/i, said: () => "המצב הפנימי נפגם — יש לפתוח את החלון מחדש" },
//! ```
//!
//! Which makes **every error string in this repository load-bearing API**, and
//! the only test asserting any of them is on the TypeScript side, against a
//! hand-typed copy. Reword `"there is no index here"` to `"no index has been
//! built"` and both halves stay green while the reader stops being told what to
//! run and gets the generic fallback instead.
//!
//! # Which half of the table this fixes, and which half it does not
//!
//! Seven of the twenty-one match prose this project does not own — `os error
//! 2`, `connection refused`, `EOF while parsing`, what a `PostError` says. Those
//! are somebody else's `Display` and matching their words is the only thing
//! available. They stay regexes, and that is honest.
//!
//! The other fourteen are **this codebase refusing on purpose**. Every one of
//! them is a deliberate sentence somebody wrote, and now every one of them
//! carries a name the window can read:
//!
//! ```text
//! no-index: there is no index here
//! ```
//!
//! The prose after the colon is still there, still English, still for whoever is
//! reading a log — and it is no longer what decides the sentence a reader sees.
//!
//! # Why a prefix and not a struct
//!
//! Because a hundred Tauri commands return `Result<T, String>`, and a typed
//! error across all of them is a change to a hundred signatures for one
//! question. A prefix costs one `format!`, reads fine in a log, and is
//! parseable. When the wire grows a place for structured errors, this is the one
//! place that has to move.

use std::fmt::Display;

/// What kind of refusal this is, in a word the window can match on.
///
/// Not an error type: the error is still whatever it was, and this names the
/// **family** so that a reader's sentence and a developer's prose can be
/// written separately without either one being a hostage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Code {
    /// There is no search index to search.
    NoIndex,
    /// There is no shelf here at all — the import has probably not run.
    NoShelf,
    /// The reader pointed the window at a folder with no seforim in it.
    ///
    /// Not [`Code::NoShelf`], and the difference is the whole reason it is its
    /// own name: *no shelf* is a state the window opened in and *this folder
    /// will not do* is an answer to something the reader just did. Telling
    /// somebody who has picked their Downloads folder that the import has not
    /// run sends them to a command line they do not need.
    NotACorpus,
    /// No sefer of that slug is on this shelf.
    NoSefer,
    /// It is in the catalogue and the file will not read.
    WillNotOpen,
    /// A lock was poisoned: something panicked while holding the state.
    Poisoned,
    /// A rung, chip, lens or mode that does not exist was asked for.
    NoSuch,
    /// A shelf dragged inside itself, a folder into its own child.
    Cycle,
    /// The personal layer will not take a write.
    ReadOnly,
    /// There is no lane, or no model for it.
    NoLane,
    /// The desk is not paired, so there is nothing to send to.
    NoDesk,
    /// A scan has no page there.
    NoPage,
    /// There is no clipboard to write to — no window server, no session.
    NoClipboard,
    /// The clipboard was there and would not take it.
    ClipboardRefused,
    /// A Source Packet would not turn into JSON.
    WillNotSerialize,
    /// The reader has not chosen the thing the command needs — nothing
    /// highlighted, no folder picked. Not a failure: a step not taken.
    NothingChosen,
    /// The one request this application makes, and it did not land. The
    /// network is not there, or GitHub is not, or the reader is on a machine
    /// that has never been on a network — which is a perfectly ordinary way to
    /// run this and is why the word is *offline* and not *failed*.
    Offline,
    /// A widening from the ladder was applied, and here is how to go back. Not
    /// a refusal at all — a **note** — and it is in this table because the
    /// window says it, and everything the window says has to come from one
    /// place.
    RungApplied,
}

girsa_corpus::spelled!(Code {
    NoIndex => "no-index",
    NoShelf => "no-shelf",
    NotACorpus => "not-a-corpus",
    NoSefer => "no-sefer",
    WillNotOpen => "will-not-open",
    Poisoned => "poisoned",
    NoSuch => "no-such",
    Cycle => "cycle",
    ReadOnly => "read-only",
    NoLane => "no-lane",
    NoDesk => "no-desk",
    NoPage => "no-page",
    NoClipboard => "no-clipboard",
    ClipboardRefused => "clipboard-refused",
    WillNotSerialize => "will-not-serialize",
    NothingChosen => "nothing-chosen",
    Offline => "offline",
    RungApplied => "rung-applied",
});

/// What separates the name from the prose.
///
/// A colon and a space, so the whole string still reads as a sentence in a log
/// — which is the thing the prose was written for and must go on being.
pub const AFTER: &str = ": ";

/// Refuse, with a name on it.
///
/// ```
/// # use girsa_app::trouble::{refuse, Code, named};
/// let said = refuse(Code::NoIndex, "there is no index here");
/// assert_eq!(said, "no-index: there is no index here");
/// assert_eq!(named(&said), Some(Code::NoIndex));
/// ```
#[must_use]
pub fn refuse(code: Code, prose: impl Display) -> String {
    format!("{}{AFTER}{prose}", code.as_str())
}

/// The name on a refusal, if it carries one.
///
/// `None` for anything that does not — an OS error, a `serde_json` message, a
/// refusal from a crate that has never heard of this module. Those are the seven
/// the window still matches by prose, and they always will be.
#[must_use]
pub fn named(said: &str) -> Option<Code> {
    Code::named(said.split_once(AFTER)?.0)
}

/// The prose, with the name taken off — for a log, or for a detail line.
#[must_use]
pub fn prose(said: &str) -> &str {
    match said.split_once(AFTER) {
        Some((code, rest)) if Code::named(code).is_some() => rest,
        _ => said,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_refusal_carries_its_name_and_its_prose() {
        let said = refuse(Code::NoSefer, "no sefer here called bavli/berakhot");
        assert_eq!(named(&said), Some(Code::NoSefer));
        assert_eq!(prose(&said), "no sefer here called bavli/berakhot");
        // And it still reads as a sentence, which is what the prose is for.
        assert!(said.ends_with("bavli/berakhot"), "{said}");
    }

    #[test]
    fn rewording_the_prose_does_not_change_what_a_reader_is_told() {
        // The whole point. `trouble.ts` matched `/no search index/i` against
        // this sentence, so rewording it silently downgraded the reader from
        // *build it, here is the command* to a generic failure — with both
        // halves of the test suite green.
        for prose in [
            "there is no index here",
            "no index has been built for this corpus",
            "",
        ] {
            assert_eq!(named(&refuse(Code::NoIndex, prose)), Some(Code::NoIndex));
        }
    }

    #[test]
    fn something_that_carries_no_name_says_so_rather_than_guessing() {
        // Seven of the twenty-one match prose this project does not own — an OS
        // error, a `serde_json` message. Those have no name and must not be
        // given one by accident.
        for foreign in [
            "os error 2",
            "connection refused",
            "EOF while parsing a value at line 1 column 0",
            "no-such-thing: not a code",
            "",
        ] {
            assert_eq!(named(foreign), None, "{foreign}");
            assert_eq!(prose(foreign), foreign);
        }
    }

    #[test]
    fn every_code_round_trips_through_its_own_spelling() {
        for (code, spelt) in Code::SPELLINGS {
            assert_eq!(Code::named(spelt), Some(*code));
            assert_eq!(named(&refuse(*code, "why")), Some(*code));
            // A spelling with a colon in it would split wrong.
            assert!(!spelt.contains(':'), "{spelt}");
        }
    }
}
