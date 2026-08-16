//! Writing the shemos so the page can be thrown away.
//!
//! A reader asked for this and the reason is not decoration: a sheet of paper
//! with a shem written on it may not be discarded, and neither may a printout
//! of a page from here. Every sefer for a hundred years has solved it the same
//! way — write the shem with a letter changed, so what is on the paper is not
//! the Name. `יקוק`, `אלקים`, `קל`, `שקי`. This is that, as a setting.
//!
//! Otzaria has the same setting and applies it to **search results only** — the
//! header and the snippet, in `text_manipulation.dart` — and only to the
//! Tetragrammaton. Here it is the page, the search, the quote, the export and
//! the print, and it is six shemos.
//!
//! # One letter for one letter, and why that is the whole design
//!
//! Every substitution below replaces a single Hebrew letter with a single
//! Hebrew letter. Nothing is added, nothing is removed, and the string that
//! comes out is the same length in characters **and in bytes** as the string
//! that went in — every Hebrew letter is two bytes in UTF-8 and so is ק.
//!
//! That is not a coincidence, it is the requirement. A span in this application
//! is a pair of offsets: a mark the reader drew, a link's anchor, the words a
//! search matched, the range a quote covers, the place a correction applies. If
//! `יהוה` became `ה'` the page would still read correctly and every one of
//! those would silently point two characters to the left of where it was drawn.
//! So the invariant is asserted, in [`tests`], on every shem this module knows.
//!
//! It is also why **אדני and אהיה are not touched**. The conventions for those
//! two — `אדנ-י`, spelling it out — all change the length, and there is no
//! one-letter swap that anybody actually prints. A shem this does not handle is
//! left exactly as the corpus has it, which is honest; a shem it handled by
//! shifting every offset on the page would not be.
//!
//! # Where a guess would be worse than nothing
//!
//! Three of the six are only shemos *sometimes*, and the difference is not
//! visible in the letters:
//!
//! | letters | the shem | the ordinary word |
//! |---|---|---|
//! | `אל` | אֵל — G-d | אֶל — *to* |
//! | `שדי` | שַׁדַּי | שָׂדַי — *my field* |
//! | `צבאות` | ה' צבאות | צבאות — *armies* |
//!
//! `אל` is the one that matters, because *to* is one of the commonest words in
//! the language and a rule that changed every one of them would rewrite the
//! sefer. The nikud settles it — tzere is the shem, segol is the preposition —
//! and the shin dot settles שדי the same way. So those two are substituted
//! **only where the text is pointed**, and left alone where it is not.
//! צבאות is substituted only directly after a shem, which is the only place it
//! is one.
//!
//! That means a bare Gemara page shows `אל` as it is. It is the right answer:
//! the alternative is a page where *to* has been turned into `קל` a hundred
//! times, which is not a page anybody can read.

use std::borrow::Cow;
use std::collections::BTreeMap;

/// Whether the shemos are written as they are, or with a letter changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Shemos {
    /// As the corpus has them. What a sefer says.
    #[default]
    AsWritten,
    /// With a letter changed, so the page may be discarded.
    Changed,
}

girsa_corpus::spelled!(Shemos {
    AsWritten => "as-written",
    Changed => "changed",
});

impl Shemos {
    /// Every setting, in the order a control rounds them.
    pub const ALL: [Self; 2] = [Self::AsWritten, Self::Changed];

    /// The next one round, for a control that cycles.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::AsWritten => Self::Changed,
            Self::Changed => Self::AsWritten,
        }
    }

    /// What the control says it will do **next**, which is what a button is
    /// for — the same convention as [`crate::session::Pointing::said`].
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::AsWritten => "שמות הקודש ככתבם",
            Self::Changed => "שמות הקודש בשינוי אות",
        }
    }

    /// The same, for a window running in English.
    #[must_use]
    pub const fn said_en(self) -> &'static str {
        match self {
            Self::AsWritten => "holy names as written",
            Self::Changed => "holy names with a letter changed",
        }
    }
}

/// The letter every substitution puts in.
const INSTEAD: char = 'ק';

/// The letters that attach to the front of a word and are not part of it.
///
/// Two at most — `וכשה` happens and `וכשהב` does not — and each is tried in
/// turn, so `ובאלהים` is matched on the `אלהים` inside it.
const PREFIX: &[char] = &['ו', 'ה', 'ב', 'כ', 'ל', 'מ', 'ש', 'ד'];

/// What may follow `אלה` and still be the shem.
///
/// A suffix is **required** for that stem, and that is the whole guard on it:
/// `אלה` on its own is *these*, and `האלה` is *these* with the article on it.
/// Every one of these is a possessive that no other Hebrew word takes after
/// those three letters.
const SUFFIXES: &[&str] = &[
    "ים", "י", "יך", "יכם", "יכן", "ינו", "יהם", "יהן", "יה", "יו", "ימו", "יהמה",
];

/// Tzere — what tells `אֵל` from `אֶל`.
const TZERE: char = '\u{05B5}';
/// The shin dot — what tells `שַׁדַּי` from `שָׂדַי`.
const SHIN_DOT: char = '\u{05C1}';

/// One letter of a word, with the marks written on it.
#[derive(Debug, Clone)]
struct Letter {
    /// Where the letter itself starts, in bytes.
    at: usize,
    ch: char,
    marks: Vec<char>,
}

impl Letter {
    fn has(&self, mark: char) -> bool {
        self.marks.contains(&mark)
    }
}

/// The same text with the shemos written the way `how` asks for.
///
/// Borrowed and untouched when there is nothing to change, which is the common
/// case on most of the shelf.
#[must_use]
pub fn written(text: &str, how: Shemos) -> Cow<'_, str> {
    if how == Shemos::AsWritten {
        return Cow::Borrowed(text);
    }
    let mut swaps: BTreeMap<usize, char> = BTreeMap::new();
    let mut after_a_shem = false;
    for word in words(text) {
        after_a_shem = change(&word, after_a_shem, &mut swaps);
    }
    if swaps.is_empty() {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    for (at, ch) in text.char_indices() {
        out.push(*swaps.get(&at).unwrap_or(&ch));
    }
    Cow::Owned(out)
}

/// Every word of the text, as letters with their marks.
///
/// A word is a run of Hebrew letters and the marks on them. Anything else —
/// a space, a comma, a maqaf, a digit, a Latin letter — ends it, which is what
/// keeps `אלהים` inside `ואלהים` matched and `אלהים` inside a URL not a word at
/// all.
fn words(text: &str) -> Vec<Vec<Letter>> {
    let mut out: Vec<Vec<Letter>> = Vec::new();
    let mut word: Vec<Letter> = Vec::new();
    for (at, ch) in text.char_indices() {
        if is_letter(ch) {
            word.push(Letter {
                at,
                ch,
                marks: Vec::new(),
            });
        } else if girsa_hebrew::is_mark(ch) {
            if let Some(last) = word.last_mut() {
                last.marks.push(ch);
            }
        } else if !word.is_empty() {
            out.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

/// Whether a character is a Hebrew letter, final forms included.
const fn is_letter(ch: char) -> bool {
    matches!(ch, '\u{05D0}'..='\u{05EA}')
}

/// Note the changes one word needs, and say whether it was a shem.
///
/// The answer feeds the next word, because צבאות is a shem only after one.
fn change(word: &[Letter], after_a_shem: bool, swaps: &mut BTreeMap<usize, char>) -> bool {
    // The word, and the word with one or two prefix letters taken off the
    // front. A shem with `ו` and `ב` in front of it is still the shem.
    for from in 0..=2.min(word.len()) {
        if word[..from].iter().any(|l| !PREFIX.contains(&l.ch)) {
            break;
        }
        let body = &word[from..];
        if let Some((at, is_a_shem)) = shem(body, after_a_shem) {
            swaps.insert(body[at].at, INSTEAD);
            // The Tetragrammaton is the one with two letters to change.
            if spelled(body) == "יהוה" {
                swaps.insert(body[1].at, INSTEAD);
            }
            return is_a_shem;
        }
    }
    false
}

/// Which letter of this word is the one that gets changed, if it is a shem —
/// and whether it is a shem in its own right.
///
/// The second half is for צבאות, which is the only one of the six that is a
/// shem because of the word beside it. It is changed and it does not make the
/// **next** word a shem, so `ה' צבאות צבאות` changes two words and not three.
fn shem(body: &[Letter], after_a_shem: bool) -> Option<(usize, bool)> {
    let letters = spelled(body);
    // יהוה → יקוק. Both hei's; the caller does the first one.
    if letters == "יהוה" {
        return Some((3, true));
    }
    // אלוה → אלוק, with or without a possessive after it.
    if let Some(rest) = letters.strip_prefix("אלוה") {
        if rest.is_empty() || SUFFIXES.contains(&rest) {
            return Some((3, true));
        }
    }
    // אלהים, אלהי, אלהיך … → אלקים. The suffix is required: `אלה` alone is
    // *these*.
    if let Some(rest) = letters.strip_prefix("אלה") {
        if SUFFIXES.contains(&rest) {
            return Some((2, true));
        }
    }
    // אֵל → קל, and only where the nikud says it is not `אֶל`.
    if letters == "אל" && body[0].has(TZERE) {
        return Some((0, true));
    }
    // שַׁדַּי → שקי, and only where the dot says it is not a field.
    if letters == "שדי" && body[0].has(SHIN_DOT) {
        return Some((1, true));
    }
    // צבאות → צבקות, and only straight after a shem, which is the only place
    // it is one rather than an ordinary plural.
    if letters == "צבאות" && after_a_shem {
        return Some((2, false));
    }
    None
}

/// The letters of a word, without their marks.
fn spelled(body: &[Letter]) -> String {
    body.iter().map(|l| l.ch).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn changed(text: &str) -> String {
        written(text, Shemos::Changed).into_owned()
    }

    #[test]
    fn nothing_moves_when_the_setting_is_off() {
        let pasuk = "בְּרֵאשִׁית בָּרָא אֱלֹהִים";
        assert!(matches!(
            written(pasuk, Shemos::AsWritten),
            Cow::Borrowed(_)
        ));
        assert_eq!(written(pasuk, Shemos::AsWritten), pasuk);
    }

    #[test]
    fn the_shem_is_written_with_a_letter_changed() {
        assert_eq!(changed("יהוה"), "יקוק");
        assert_eq!(changed("אלהים"), "אלקים");
        assert_eq!(changed("אלהי"), "אלקי");
        assert_eq!(changed("אלהיך"), "אלקיך");
        assert_eq!(changed("אלהינו"), "אלקינו");
        assert_eq!(changed("אלוה"), "אלוק");
        // With the prefixes a sentence puts on them.
        assert_eq!(changed("ויהוה"), "ויקוק");
        assert_eq!(changed("ובאלהים"), "ובאלקים");
        // Two prefixes, which is as many as a word takes.
        assert_eq!(changed("ולאלהים"), "ולאלקים");
    }

    #[test]
    fn the_pointing_survives_the_change() {
        // The marks stay on the letters they were written on, because only the
        // letter is swapped.
        assert_eq!(changed("יְהוָה"), "יְקוָק");
        assert_eq!(changed("אֱלֹהִים"), "אֱלֹקִים");
    }

    #[test]
    fn every_change_is_one_letter_for_one_letter() {
        // The invariant the whole module is built on: a span drawn on this page
        // still covers the words it was drawn on. Characters **and** bytes,
        // because offsets in this application are byte offsets in some places
        // and character offsets in others, and the substitution has to be safe
        // for both.
        for one in [
            "יהוה",
            "אלהים",
            "אלהיהם",
            "אלוה",
            "אֵל",
            "שַׁדַּי",
            "יְהוָה צְבָאוֹת",
            "וַיֹּאמֶר אֱלֹהִים יְהִי אוֹר",
        ] {
            let after = changed(one);
            assert_eq!(after.chars().count(), one.chars().count(), "{one}");
            assert_eq!(after.len(), one.len(), "{one}");
        }
    }

    #[test]
    fn these_is_not_a_shem_and_neither_is_to() {
        // `אלה` with nothing after it is *these*, and it is everywhere.
        assert_eq!(changed("ואלה שמות בני ישראל"), "ואלה שמות בני ישראל");
        assert_eq!(changed("האלה"), "האלה");
        // `אל` unpointed could be either, so it is left alone — the alternative
        // is a page where *to* reads `קל` a hundred times.
        assert_eq!(changed("ויאמר אל משה"), "ויאמר אל משה");
        // Pointed, the tzere settles it and the segol settles it the other way.
        assert_eq!(changed("אֵל"), "קֵל");
        assert_eq!(changed("אֶל"), "אֶל");
    }

    #[test]
    fn a_field_is_not_a_shem_and_the_dot_says_which() {
        assert_eq!(changed("שַׁדַּי"), "שַׁקַּי");
        assert_eq!(changed("שָׂדַי"), "שָׂדַי");
        // Bare, it could be either.
        assert_eq!(changed("שדי"), "שדי");
    }

    #[test]
    fn armies_are_a_shem_only_after_a_shem() {
        assert_eq!(changed("יהוה צבאות"), "יקוק צבקות");
        assert_eq!(changed("צבאות ישראל"), "צבאות ישראל");
        // And the word after the pair is back to being an ordinary word.
        assert_eq!(changed("יהוה צבאות צבאות"), "יקוק צבקות צבאות");
    }

    #[test]
    fn a_page_that_has_already_been_changed_is_left_where_it_is() {
        // Idempotent: what comes out of this goes back in unchanged, which is
        // what makes it safe to apply on a corpus that already prints some
        // shemos this way.
        for one in ["יקוק", "אלקים", "יהוה", "אֱלֹהִים צְבָאוֹת"] {
            let once = changed(one);
            assert_eq!(changed(&once), once, "{one}");
        }
    }

    #[test]
    fn the_setting_names_what_it_will_do_next() {
        // The same convention as `Pointing::said`: a button says what pressing
        // it does, not what state you are already in.
        assert_eq!(Shemos::default(), Shemos::AsWritten);
        assert_eq!(Shemos::AsWritten.next(), Shemos::Changed);
        assert_eq!(Shemos::Changed.next(), Shemos::AsWritten);
        assert_eq!(Shemos::ALL.len(), 2);
    }
}
