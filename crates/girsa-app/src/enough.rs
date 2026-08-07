//! How much of a thing is enough to show, and where the numbers live.
//!
//! Every one of these was a literal in the Tauri shell — `40` inside a call,
//! `const PAGE: usize = 25` beside one command, `const WORDS: usize = 90`
//! inside a function body — under a README that says the shell holds *"nothing
//! that decides anything"*. How many results a page has is a decision. So is
//! how many characters of a neighbouring sefer a sidebar row is allowed to
//! quote, and so is what happens at the cut.
//!
//! They are small numbers and none of them is subtle. That is exactly why they
//! drifted: nobody rereads a `40`.

/// How many results to a page.
///
/// Enough that scrolling is rare and few enough that the count under the box
/// stays honest about what you are looking at.
pub const A_PAGE: usize = 25;

/// How many seforim the shelf offers while you type.
///
/// The picker shows a list, not a result set: past about forty a reader has
/// stopped reading and started typing more.
pub const NAMES_OFFERED: usize = 40;

/// How many characters of the far end of a link a sidebar row may quote.
///
/// A row names the sefer and the place; the first words are a courtesy so the
/// reader can tell a Rashi about the word from a Rashi about the sugya without
/// opening it. Long enough to be that, short enough that ten rows are still a
/// list.
pub const FIRST_WORDS: usize = 90;

/// What a cut looks like.
const CUT: char = '…';

/// The first [`FIRST_WORDS`] characters, with an ellipsis **only if something
/// was cut off**.
///
/// Characters, not bytes: Hebrew is two bytes a letter and a byte count would
/// land mid-character about half the time. The conditional ellipsis is the part
/// worth having a function for — a short comment that arrives with a trailing
/// `…` reads as a comment the application truncated, and the reader goes
/// looking for words that were never there.
#[must_use]
pub fn first_words(text: &str) -> String {
    shortened(text, FIRST_WORDS)
}

/// [`first_words`] at any length.
#[must_use]
pub fn shortened(text: &str, most: usize) -> String {
    let mut out: String = text.chars().take(most).collect();
    if text.chars().count() > most {
        out.push(CUT);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_cut_means_nothing_added() {
        assert_eq!(shortened("מאימתי", 10), "מאימתי");
    }

    #[test]
    fn exactly_the_limit_is_not_a_cut() {
        // Off by one here is a permanent ellipsis on every row that happens to
        // be exactly the limit, which is the length rows cluster at.
        let text: String = "א".repeat(FIRST_WORDS);
        assert_eq!(first_words(&text), text);
        assert!(!first_words(&text).ends_with(CUT));
    }

    #[test]
    fn one_over_is() {
        let text: String = "א".repeat(FIRST_WORDS + 1);
        let cut = first_words(&text);
        assert!(cut.ends_with(CUT));
        assert_eq!(
            cut.chars().count(),
            FIRST_WORDS + 1,
            "the words plus the mark"
        );
    }

    #[test]
    fn it_counts_letters_and_not_bytes() {
        // Every letter here is two bytes. A byte count would cut this in half
        // and land inside a letter.
        let text = "מאימתי קורין את שמע בערבית";
        assert_eq!(shortened(text, 6), "מאימתי…");
        assert_eq!(shortened(text, 6).chars().count(), 7);
    }

    #[test]
    fn the_numbers_are_the_ones_the_window_was_using() {
        // Moved, not changed. A refactor that quietly re-tunes a page size is
        // two changes reported as one.
        assert_eq!(A_PAGE, 25);
        assert_eq!(NAMES_OFFERED, 40);
        assert_eq!(FIRST_WORDS, 90);
    }
}
