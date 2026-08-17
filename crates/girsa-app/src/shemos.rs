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
//! That requirement is also what kept `אדני` out of this module for a long
//! time: the conventions written down for it — `אדנ-י`, spelling it out — all
//! change the length. The one-letter swap **ד → מ** does not, so `אֲדֹנָי`
//! is here now, under the guard the next section describes.
//!
//! `אהיה` is still not touched. See *the one that is still open*, below.
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
//! | `אדני` | אֲדֹנָי | אֲדֹנִי — *my master* |
//!
//! `אל` is the one that matters, because *to* is one of the commonest words in
//! the language and a rule that changed every one of them would rewrite the
//! sefer. The nikud settles it — tzere is the shem, segol is the preposition —
//! and the shin dot settles שדי the same way. So those two are substituted
//! **only where the text is pointed**, and left alone where it is not.
//! צבאות is substituted only directly after a shem, which is the only place it
//! is one.
//!
//! `אדני` joins that list rather than the unconditional one, and it is the
//! one where the ordinary word is commonest of all: `אֲדֹנִי הַמֶּלֶךְ` is
//! *my lord the king*, said to a person. Only the vowel under the nun
//! separates them — kamatz is the shem, chirik is the man.
//!
//! That means a bare Gemara page shows `אל` as it is. It is the right answer:
//! the alternative is a page where *to* has been turned into `קל` a hundred
//! times, which is not a page anybody can read. The same cost is paid for
//! `אדני`: on an unpointed page it does nothing.
//!
//! # The one that is still open
//!
//! `אהיה` has a length-preserving swap — **ה → ק**, giving `אקיק` — and it is
//! still not written, because unlike every case above **there is no mark that
//! separates it from the ordinary word.** `אֶהְיֶה` is the shem in
//! `אֶהְיֶה אֲשֶׁר אֶהְיֶה`, and `אֶהְיֶה` is also the plain verb *I will
//! be*, as in `וְאֶהְיֶה עִמָּךְ` — pointed identically, letter for letter
//! and mark for mark.
//!
//! So there is no guard of the kind `אל` and `אדני` get. The choices are to
//! change every `אהיה` in Tanach, including the verbs, or to match the one
//! phrase it is unambiguously a shem in — which needs the word *after* it,
//! and this module only ever looks backwards (`after_a_shem`, for צבאות).
//! Neither is a decision to make quietly, so it is written down instead.

use std::borrow::Cow;
use std::collections::BTreeMap;

/// Whether the shemos are written as they are, or with a letter changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Shemos {
    /// As the corpus has them. What a sefer says.
    AsWritten,
    /// With a letter changed, so the page may be discarded.
    ///
    /// **The default**, and deliberately. A reader who has not been asked is a
    /// reader who might print, and a page that came out of a printer with a
    /// shem on it cannot be thrown away — the harm runs one way only. Turning
    /// this off is one click for a reader who wants the corpus's own spelling;
    /// there is no click that un-prints a page.
    #[default]
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
/// Kamatz — what tells `אֲדֹנָי` from `אֲדֹנִי`, *my master*.
const KAMATZ: char = '\u{05B8}';
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

/// Which letters of one shem change, and to what.
///
/// At most two, because יהוה and אהיה each carry two hei's. And the letter put
/// in is **not** always ק — אדני takes מ. Everything else here is indifferent
/// to which letter it is so long as it is exactly one: the offsets, the byte
/// count, and the invariant asserted in [`tests`] all hold for any Hebrew
/// letter, because they are all two bytes.
#[derive(Debug, Clone, Copy)]
struct Change {
    at: usize,
    also: Option<usize>,
    instead: char,
}

impl Change {
    /// One letter, changed to ק.
    const fn one(at: usize) -> Self {
        Self {
            at,
            also: None,
            instead: INSTEAD,
        }
    }

    /// Two letters, changed to ק — the shemos spelled with two hei's.
    const fn two(at: usize, also: usize) -> Self {
        Self {
            at,
            also: Some(also),
            instead: INSTEAD,
        }
    }

    /// One letter, changed to something other than ק.
    const fn to(at: usize, instead: char) -> Self {
        Self {
            at,
            also: None,
            instead,
        }
    }
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
        if let Some((change, is_a_shem)) = shem(body, after_a_shem) {
            swaps.insert(body[change.at].at, change.instead);
            if let Some(also) = change.also {
                swaps.insert(body[also].at, change.instead);
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
fn shem(body: &[Letter], after_a_shem: bool) -> Option<(Change, bool)> {
    let letters = spelled(body);
    // יהוה → יקוק. Both hei's.
    if letters == "יהוה" {
        return Some((Change::two(3, 1), true));
    }
    // אלוה → אלוק, with or without a possessive after it.
    if let Some(rest) = letters.strip_prefix("אלוה") {
        if rest.is_empty() || SUFFIXES.contains(&rest) {
            return Some((Change::one(3), true));
        }
    }
    // אלהים, אלהי, אלהיך … → אלקים. The suffix is required: `אלה` alone is
    // *these*.
    if let Some(rest) = letters.strip_prefix("אלה") {
        if SUFFIXES.contains(&rest) {
            return Some((Change::one(2), true));
        }
    }
    // אֲדֹנָי → אמני, and only where the kamatz says it is not `אֲדֹנִי`,
    // *my master* — which is an ordinary word, and a common one. Same guard
    // as אל and שדי below, for the same reason and with the same cost: on an
    // unpointed page this does nothing, which is the right answer. The letter
    // put in is מ rather than ק because that is the swap that gets printed
    // for this shem.
    if letters == "אדני" && body[2].has(KAMATZ) {
        return Some((Change::to(1, 'מ'), true));
    }
    // אֵל → קל, and only where the nikud says it is not `אֶל`.
    if letters == "אל" && body[0].has(TZERE) {
        return Some((Change::one(0), true));
    }
    // שַׁדַּי → שקי, and only where the dot says it is not a field.
    if letters == "שדי" && body[0].has(SHIN_DOT) {
        return Some((Change::one(1), true));
    }
    // צבאות → צבקות, and only straight after a shem, which is the only place
    // it is one rather than an ordinary plural.
    if letters == "צבאות" && after_a_shem {
        return Some((Change::one(2), false));
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
            "אֲדֹנָי",
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
    fn my_master_is_not_a_shem_and_the_vowel_under_the_nun_says_which() {
        // The commonest ordinary word of any of them: `אֲדֹנִי הַמֶּלֶךְ` is
        // *my lord the king*, said to a person, and it is the same four
        // letters. Only the vowel under the nun separates them.
        assert_eq!(changed("אֲדֹנָי"), "אֲמֹנָי");
        assert_eq!(changed("אֲדֹנִי הַמֶּלֶךְ"), "אֲדֹנִי הַמֶּלֶךְ");
        // Bare, it could be either, so nothing happens — the same answer this
        // module gives for אל and for שדי, and for the same reason.
        assert_eq!(changed("אדני"), "אדני");
    }

    /// `אהיה` is not written, and this is where that is asserted.
    ///
    /// The swap exists — ה → ק gives `אקיק`, one letter for one — so nothing
    /// about the length invariant is stopping it. What is stopping it is that
    /// `אֶהְיֶה` the shem and `אֶהְיֶה` the plain verb *I will be* are pointed
    /// identically, so there is no guard to write. Changing it unconditionally
    /// would rewrite `וְאֶהְיֶה עִמָּךְ` — a promise, not a Name.
    #[test]
    fn the_one_with_no_mark_to_tell_it_apart_is_left_alone() {
        assert_eq!(changed("אהיה"), "אהיה");
        assert_eq!(changed("אֶהְיֶה אֲשֶׁר אֶהְיֶה"), "אֶהְיֶה אֲשֶׁר אֶהְיֶה");
        assert_eq!(changed("וְאֶהְיֶה עִמָּךְ"), "וְאֶהְיֶה עִמָּךְ");
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

    /// The default is a ruling, not a Rust convention, so it gets its own test.
    ///
    /// A reader who has not been asked might print, and a page that came out of
    /// a printer with a shem on it cannot be thrown away. The harm runs one
    /// way: turning this off is one click, and there is no click that un-prints
    /// a page. This assertion used to sit inside the `next` test as an aside,
    /// where a deliberate product decision read as an implementation detail.
    #[test]
    fn the_shemos_are_changed_unless_a_reader_says_otherwise() {
        assert_eq!(Shemos::default(), Shemos::Changed);
    }

    #[test]
    fn the_setting_names_what_it_will_do_next() {
        // The same convention as `Pointing::said`: a button says what pressing
        // it does, not what state you are already in.
        assert_eq!(Shemos::AsWritten.next(), Shemos::Changed);
        assert_eq!(Shemos::Changed.next(), Shemos::AsWritten);
        assert_eq!(Shemos::ALL.len(), 2);
    }
}
