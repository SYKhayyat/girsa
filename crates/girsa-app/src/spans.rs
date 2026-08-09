//! Which words a link is about (spec.md §8.4, BUILDER.md W24).
//!
//! *Links attach to specific words, not whole segments — selecting a phrase
//! highlights only the links touching it.*
//!
//! Sefaria's links address a segment and so do Otzaria's, so a span has to come
//! from somewhere, and there are exactly three honest places for it:
//!
//! 1. **The anchors the volume itself carries.** Sefaria writes an empty
//!    `<i data-commentator="…"></i>` where each commentary attaches — 43,883 of
//!    them in Shulchan Arukh Orach Chayim alone — and `girsa_corpus::anchors`
//!    mines them out of the text at ingest, records the character offset,
//!    rebases them across every segment split and persists them.
//!
//!    This file used to open with *"Nothing in the shipped data says which
//!    words"*, which was true when it was written and stopped being true at
//!    W34 — while `anchors.rs` said, in its own module note, that they are
//!    *"spec.md §8.4's span anchoring, already computed upstream and sitting in
//!    the corpus unused"*. Two files, one spec section, one saying the datum
//!    exists and one saying it does not. See [`anchor_span`].
//! 2. **The dibur hamatchil.** A commentary says which words it is on, in the
//!    text, in bold — `<b>משעה שהכהנים נכנסים</b> – כהנים שנטמאו`. Sefaria marks
//!    43,890 of them in Berakhot alone (see [`crate::display`]). Finding those
//!    words in the base segment is reading, not guessing — but it needs the far
//!    sefer **open**, which an anchor does not.
//! 3. **You said so** — a link you drew from a highlight, or pinned onto one
//!    (W23's layer).
//!
//! # The exactly-once rule
//!
//! A dibur hamatchil that appears **twice** in the base segment gives two
//! candidate spans and no way to choose, so it gives none. BUILDER.md rule 6:
//! ambiguity is surfaced, never guessed — and the failure here would be
//! peculiarly nasty, because a highlight on the wrong half of a line looks
//! exactly like a highlight on the right one.
//!
//! # Compared through the normalizer
//!
//! Berakhot ships fully menukad and its commentaries mostly do not (spec.md
//! §2.1), so the dibur hamatchil is `משעה שהכהנים` and the line it is quoting is
//! `מִשָּׁעָה שֶׁהַכֹּהֲנִים`. Comparing those as strings finds nothing at all. W2's
//! sibling rule: nothing here compares two Hebrew strings with `==`.

use crate::display::{Shown, Style};

/// The words a commentary segment declares it is about — every way it might
/// have said so.
///
/// **Two conventions, both real, and the corpus uses whichever the volume
/// used.** Sefaria marks a dibur hamatchil `<b>` in some texts and separates it
/// with a dash in others — Rashi on Berakhot, in the copy on this shelf, is
/// entirely the second:
///
/// ```text
/// עד סוף האשמורה הראשונה – שליש הלילה כדמפרש בגמרא
/// └── the words he is on ──┘
/// ```
///
/// So this hands back **candidates**, in the order they are worth trying, and
/// the caller checks each against the line. That is not a guess dressed up as a
/// list: a candidate is taken only if it is in the base segment exactly once
/// (see [`dibur_span`]), and one that is not there costs nothing.
#[must_use]
pub fn diburim(commentary: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut offer = |words: &str| {
        let words = words.trim().trim_end_matches([':', '.', '،', ',']).trim();
        // One word is not a quotation, it is a coincidence waiting to happen:
        // the commonest opening word in Shas is `אמר`. And a whole paragraph is
        // not a dibur hamatchil, it is the comment.
        let count = words.split_whitespace().count();
        if (2..=12).contains(&count) && !out.iter().any(|kept| kept == words) {
            out.push(words.to_string());
        }
    };

    // 1. The markup, where the text carries it.
    let runs = crate::display::runs(commentary);
    if let Some(first) = runs.first() {
        if first.style == Style::Opening {
            offer(&first.text);
        }
    }

    // 2. The dash, which is how the rest of it is written. En dash, em dash and
    //    a hyphen with spaces around it — all three are in the corpus.
    let plain = crate::display::plain(commentary);
    for dash in [" – ", " — ", " - "] {
        if let Some((head, _)) = plain.split_once(dash) {
            offer(head);
            // …and its first clause, because a comment often quotes two
            // phrases before it begins: `מאימתי קורין. משעה שהכהנים – …`.
            if let Some((clause, _)) = head.split_once(". ") {
                offer(clause);
            }
            break;
        }
    }

    // 3. A first clause with no dash after it at all.
    if let Some((clause, _)) = plain.split_once(". ") {
        offer(clause);
    }
    out
}

/// The first way this comment says which words it is on, if it says at all.
#[must_use]
pub fn dibur(commentary: &str) -> Option<String> {
    diburim(commentary).into_iter().next()
}

/// Where those words are in the base segment, as characters of the text **as
/// the pane drew it**.
///
/// `None` when they are not there, or are there more than once. Both are
/// answers; a nearest match is not.
#[must_use]
pub fn dibur_span(base: &str, commentary: &str, nikud: bool) -> Option<std::ops::Range<usize>> {
    let drawn = Shown::of(base, nikud);
    // The candidates in order, and the first one that is **in this line exactly
    // once** wins. A candidate that is not there is not evidence of anything,
    // so it costs a lookup and nothing else.
    diburim(commentary)
        .into_iter()
        .find_map(|words| span_of(&drawn, &words))
}

/// The one place a phrase sits in a drawn line, or nowhere.
///
/// Matched word by word through [`girsa_hebrew::tokenize`], because the two
/// texts do not agree about nikud and may not agree about spacing either.
fn span_of(drawn: &Shown, phrase: &str) -> Option<std::ops::Range<usize>> {
    let wanted: Vec<String> = girsa_hebrew::tokenize(phrase)
        .into_iter()
        .map(|token| token.text)
        .collect();
    if wanted.is_empty() {
        return None;
    }
    let line = girsa_hebrew::tokenize(drawn.text());
    if line.len() < wanted.len() {
        return None;
    }

    let mut found = None;
    for start in 0..=line.len() - wanted.len() {
        let matches = line[start..start + wanted.len()]
            .iter()
            .zip(&wanted)
            .all(|(token, word)| token.text == *word);
        if !matches {
            continue;
        }
        if found.is_some() {
            // Twice. Two candidate spans and no way to choose — so neither.
            return None;
        }
        found = Some((line[start].start, line[start + wanted.len() - 1].end));
    }
    let (from, to) = found?;
    // `tokenize` counts bytes; everything the window is told counts characters.
    let text = drawn.text();
    Some(text.get(..from)?.chars().count()..text.get(..to)?.chars().count())
}

/// Where a named commentary attaches to this segment, from the anchors mined at
/// ingest.
///
/// **The single-resolution case only.** A commentator named once in the segment
/// gives one offset and one answer. Named twice — three notes of Mishnah Berurah
/// on one se'if, which is ordinary — gives two candidates and no way to choose
/// between them without knowing which of the two links this is, so it gives
/// none. BUILDER.md rule 6: ambiguity is surfaced, never guessed, and the
/// failure here would be peculiarly nasty because a highlight on the wrong half
/// of a line looks exactly like a highlight on the right one.
///
/// The extent runs to the **next anchor of any commentator**, or to the end of
/// the segment. That is what an anchor means in Sefaria's own layout: the mark
/// sits where the note begins, and what it is about is the text from there until
/// something else begins.
///
/// Offsets are characters, not bytes, in the *cleaned* text — the same unit
/// every span in this project counts in, and the unit `girsa_corpus::anchors`
/// records.
#[must_use]
pub fn anchor_span(
    anchors: &[girsa_corpus::anchors::Anchor],
    text: &str,
    commentator: &str,
) -> Option<std::ops::Range<usize>> {
    let wanted = fold_name(commentator);
    if wanted.is_empty() {
        return None;
    }
    let mut mine = anchors
        .iter()
        .filter(|a| fold_name(&a.commentator) == wanted);
    let only = mine.next()?;
    if mine.next().is_some() {
        return None; // named twice: two candidates, no way to choose
    }
    let chars = text.chars().count();
    if only.at >= chars {
        // An anchor past the end of the text it is on. Not reachable from a
        // clean import — the offsets are rebased when a segment splits — but a
        // span that runs backwards would be drawn as a highlight over the whole
        // line, which is worse than no highlight.
        return None;
    }
    let end = anchors
        .iter()
        .map(|a| a.at)
        .filter(|at| *at > only.at)
        .min()
        .unwrap_or(chars)
        .min(chars);
    Some(only.at..end)
}

/// A commentator's name, as loosely as two spellings of one name may differ.
///
/// Case and whitespace only. These are Latin names as the corpus spells them —
/// `Mishnah Berurah`, `Ba'er Hetev` — and this is deliberately **not**
/// `girsa_hebrew::normalize`, which is about Hebrew: folding an apostrophe out
/// of `Ba'er` would make it collide with nothing useful and is not a difference
/// anybody's data actually has.
fn fold_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod anchor_tests {
    use super::*;
    use girsa_corpus::anchors::Anchor;

    fn anchor(commentator: &str, at: usize) -> Anchor {
        Anchor {
            commentator: commentator.into(),
            at,
            label: None,
            order: None,
        }
    }

    /// The case the report calls the single-resolution one.
    #[test]
    fn one_anchor_for_a_commentator_is_the_span() {
        let text = "יתגבר כארי לעמוד בבוקר";
        let anchors = [anchor("Mishnah Berurah", 6), anchor("Ba'er Hetev", 11)];
        // From its own mark to where the next one begins.
        assert_eq!(anchor_span(&anchors, text, "Mishnah Berurah"), Some(6..11));
        // The last one runs to the end of the segment.
        assert_eq!(
            anchor_span(&anchors, text, "Ba'er Hetev"),
            Some(11..text.chars().count())
        );
    }

    #[test]
    fn a_commentator_named_twice_gives_no_span() {
        // Three notes of one commentary on one se'if is ordinary, and there is
        // no way to tell which of them *this* link is from the anchors alone.
        let text = "יתגבר כארי לעמוד בבוקר";
        let anchors = [anchor("Mishnah Berurah", 6), anchor("Mishnah Berurah", 11)];
        assert_eq!(anchor_span(&anchors, text, "Mishnah Berurah"), None);
    }

    #[test]
    fn a_commentator_that_is_not_there_gives_no_span() {
        let text = "יתגבר כארי";
        let anchors = [anchor("Ba'er Hetev", 3)];
        assert_eq!(anchor_span(&anchors, text, "Mishnah Berurah"), None);
        assert_eq!(anchor_span(&anchors, text, ""), None);
    }

    #[test]
    fn the_name_is_matched_loosely_and_only_loosely() {
        let text = "יתגבר כארי";
        let anchors = [anchor("Mishnah  Berurah", 3)];
        assert_eq!(anchor_span(&anchors, text, "mishnah berurah"), Some(3..10));
        // …and not so loosely that two different commentaries collide.
        assert_eq!(anchor_span(&anchors, text, "Mishnah"), None);
    }

    #[test]
    fn an_offset_past_the_text_is_no_span_rather_than_a_backwards_one() {
        let text = "יתגבר";
        let anchors = [anchor("Ba'er Hetev", 99)];
        assert_eq!(anchor_span(&anchors, text, "Ba'er Hetev"), None);
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// The first mishnah of Berakhot, as it is shipped: fully menukad.
    const MISHNAH: &str = "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן";

    #[test]
    fn a_commentary_says_which_words_it_is_on_and_they_are_found_in_the_base() {
        // Rashi, as Sefaria marks it — and with no nikud at all, against a base
        // text that is fully pointed. This is the pair spec.md §2.1 names.
        let rashi = "<b>משעה שהכהנים נכנסים</b> – כהנים שנטמאו וטבלו";
        assert_eq!(dibur(rashi).as_deref(), Some("משעה שהכהנים נכנסים"));

        let span = dibur_span(MISHNAH, rashi, true).expect("found in the mishnah");
        let letters: Vec<char> = MISHNAH.chars().collect();
        let covered: String = letters[span].iter().collect();
        assert_eq!(covered, "מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים");
    }

    #[test]
    fn the_span_is_in_the_characters_the_reader_is_looking_at() {
        // With nikud off the same words are half as many characters, and the
        // span has to be the ones on the screen.
        let rashi = "<b>משעה שהכהנים</b> – כהנים שנטמאו";
        let bare = crate::display::Shown::of(MISHNAH, false);
        let span = dibur_span(MISHNAH, rashi, false).expect("found");
        let letters: Vec<char> = bare.text().chars().collect();
        assert_eq!(letters[span].iter().collect::<String>(), "משעה שהכהנים");
    }

    #[test]
    fn a_dibur_hamatchil_that_is_there_twice_gives_no_span_at_all() {
        // Rule 6, in the one place a reader would never check: a highlight on
        // the wrong half of a line looks exactly like a highlight on the right
        // one.
        let base = "אמר רבי יוחנן משום רבי שמעון וכן אמר רבי יוחנן משום רבי שמעון";
        let commentary = "<b>אמר רבי יוחנן</b> – פירוש";
        assert_eq!(dibur_span(base, commentary, true), None);
    }

    #[test]
    fn a_commentary_with_no_dibur_hamatchil_has_no_span() {
        assert_eq!(dibur("כהנים שנטמאו וטבלו"), None);
        assert_eq!(dibur_span(MISHNAH, "כהנים שנטמאו", true), None);
    }

    #[test]
    fn the_dash_convention_is_read_as_well_as_the_markup() {
        // Rashi on Berakhot, verbatim off this shelf: no `<b>` anywhere in it,
        // and the words he is on are the ones before the dash. A reader of
        // `<b>` alone finds nothing in this whole masechta.
        let rashi = "עד סוף האשמורה הראשונה – שליש הלילה כדמפרש בגמרא";
        assert_eq!(dibur(rashi).as_deref(), Some("עד סוף האשמורה הראשונה"));

        let gemara = "והא קא קרינן עד סוף האשמורה הראשונה דברי רבי אליעזר";
        let span = dibur_span(gemara, rashi, true).expect("found");
        let letters: Vec<char> = gemara.chars().collect();
        assert_eq!(
            letters[span].iter().collect::<String>(),
            "עד סוף האשמורה הראשונה"
        );
    }

    #[test]
    fn a_comment_that_quotes_two_phrases_offers_both_and_the_line_chooses() {
        // `מאימתי קורין את שמע. משעה שהכהנים נכנסים – …` is one comment on two
        // phrases. Which of them this line has is a question about the line.
        let rashi = "מאימתי קורין את שמע. משעה שהכהנים נכנסים לאכול בתרומתן – כהנים שנטמאו";
        let offered = diburim(rashi);
        assert!(offered.len() >= 2, "{offered:?}");
        assert!(offered.iter().any(|words| words == "מאימתי קורין את שמע"));

        // The whole head is tried first and is **not** in this line — this
        // Rashi quotes `בערבין` where the mishnah in front of him reads
        // `בערבית`, which is a girsa and not a typo. So the first clause is
        // what lands, and it lands on the words it actually quotes.
        let span = dibur_span(MISHNAH, rashi, true).expect("one of them is in the mishnah");
        let letters: Vec<char> = MISHNAH.chars().collect();
        assert_eq!(
            letters[span].iter().collect::<String>(),
            "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע"
        );
    }

    #[test]
    fn one_word_in_bold_is_not_a_quotation() {
        // `אמר` is bold at the head of thousands of comments and matches
        // half of Shas. A span built on it would be noise wearing precision.
        assert_eq!(dibur("<b>אמר</b> – פירוש"), None);
        assert_eq!(dibur("<b>אמר רבי</b> – פירוש").as_deref(), Some("אמר רבי"));
    }

    #[test]
    fn words_that_are_not_in_the_base_segment_give_nothing() {
        let elsewhere = "<b>מאימתי קורין בשחרית</b> – פירוש";
        assert_eq!(dibur_span(MISHNAH, elsewhere, true), None);
    }
}

// ------------------------------------------------- the anchors already mined
//
// §8.4 said twice, and the two halves never met.
//
// This module opens with *"**Nothing in the shipped data says which words.**"*
// That was true when it was written and stopped being true at W34:
// `girsa_corpus::anchors` mines Sefaria's inline `<i data-commentator="…"></i>`
// elements out of the text at ingest — **43,883 of them in Shulchan Arukh Orach
// Chayim alone** — records each one's character offset on the segment, rebases
// them across every segment split, and persists them. Its own module note calls
// them *"spec.md §8.4's span anchoring, already computed upstream and sitting in
// the corpus unused"*.
//
// Two files, one spec section, one saying the datum exists and one saying it
// does not. And the consequence was not academic: `dibur_span` needs the **far
// sefer already open** to read the dibur hamatchil out of it, so a link to a
// commentary the reader had not opened had no span at all — which is precisely
// the case where a span is worth most, because the words are the only thing
// that would tell them whether to open it.
//
// An anchor needs nothing open. It is on the segment in front of them.
