//! Which words a link is about (spec.md §8.4, BUILDER.md W24).
//!
//! *Links attach to specific words, not whole segments — selecting a phrase
//! highlights only the links touching it.*
//!
//! Nothing in the shipped data says which words. Sefaria's links address a
//! segment and so do Otzaria's, so a span has to come from somewhere, and there
//! are exactly two honest places for it:
//!
//! 1. **The dibur hamatchil.** A commentary says which words it is on, in the
//!    text, in bold — `<b>משעה שהכהנים נכנסים</b> – כהנים שנטמאו`. Sefaria marks
//!    43,890 of them in Berakhot alone (see [`crate::display`]). Finding those
//!    words in the base segment is reading, not guessing.
//! 2. **You said so** — a link you drew from a highlight, or pinned onto one
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
