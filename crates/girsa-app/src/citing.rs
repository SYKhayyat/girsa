//! Turning prose full of citations into refs.
//!
//! spec.md §10.5, BUILDER.md W19. **High-confidence patterns only** (decision
//! 12). Prose full of citations becomes live refs, and *anything ambiguous
//! stays plain text*. The rules are deliberately narrow:
//!
//! 1. the resolver must return [`Resolution::Exact`] — a citation that names
//!    two seforim is left alone, not narrowed to the first;
//! 2. the citation must carry an **address**. A bare title in prose is usually
//!    a subject and not a mekor: *the Mishnah Berurah writes at length* is not
//!    a citation of anything, and linking it would put a ref on a sentence
//!    nobody was citing in;
//! 3. **every level of that address is a number or a daf.** The resolver will
//!    read a named level, which is right for a citation somebody typed into a
//!    box and wrong for prose: `ברכות ב. ועיין שו"ע` resolves as Berakhot at a
//!    section called `ועיין שו"ע`, swallowing the next citation whole. In prose
//!    a mekor is a title and numbers.
//!
//! Every one of those refuses more than it accepts, on purpose. A wrong link in
//! a sefer somebody prints is worse than a plain string, and there is no way to
//! tell from the printed page which one it was.
//!
//! # Why this lives here and not on the desk
//!
//! It was in `girsa-desk`, beside *where did I use this*, because both were
//! written for the Ksav loop on the same day. They are not the same shape:
//! `who_cites` scans your documents and needs a desk, and this is a function of
//! **a lexicon and a string** and needs nothing at all. Leaving it up there made
//! it unreachable from the reading pane, because `girsa-desk` depends on this
//! crate and a crate cannot depend back. The pane is where a note is actually
//! read, so a citation typed into a note stayed dead text on the one surface
//! that shows it. `girsa_desk::linkify` is still that path — it re-exports this.

use girsa_ref::{resolve, Lexicon, Resolution};

/// A citation found in prose, and where it is.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Linked {
    /// Where it sits in the text, in **characters**.
    pub from: usize,
    pub to: usize,
    /// The citation as it was written.
    pub text: String,
    /// The ref it resolves to, exactly.
    pub reference: String,
}

/// The longest citation a lexicon can start with is this many words.
///
/// A title is up to `longest_variant_words` and the address after it is at
/// most a handful: `רמב"ם הלכות תפילה פרק ד' הלכה א'` is seven. The cap keeps
/// linkify linear in the length of the prose rather than quadratic.
const ADDRESS_WORDS: usize = 6;

/// Find the citations in a piece of prose — **the ones that are certain**.
///
/// See the module note for the three rules. What this returns is safe to turn
/// into links; everything else is left as somebody's words.
#[must_use]
pub fn linkify(lexicon: &Lexicon, prose: &str) -> Vec<Linked> {
    let words: Vec<(usize, &str)> = word_positions(prose);
    let longest = lexicon.longest_variant_words() + ADDRESS_WORDS;

    let mut out: Vec<Linked> = Vec::new();
    let mut at = 0;
    while at < words.len() {
        let mut taken = None;
        // Longest first: `שו"ע או"ח סימן א' סעיף ב'` has to beat `שו"ע`, or a
        // citation is linked to the whole Shulchan Arukh and lands on its
        // first page.
        for take in (1..=longest.min(words.len() - at)).rev() {
            let (start, _) = words[at];
            let (last_start, last_word) = words[at + take - 1];
            let end = last_start + last_word.chars().count();
            let text: String = prose.chars().skip(start).take(end - start).collect();

            // A leading prefix letter is how citations are written in prose:
            // *וכתב בשו"ע* is the commonest shape there is. Peeled only from
            // the first word, and only when what is left resolves exactly —
            // the rules below still have to hold, so this widens where a
            // citation is found and never what it is found to be.
            let (offset, reading) = match resolve(lexicon, text.trim()) {
                Resolution::Exact(_) => (0, text.trim().to_string()),
                _ => match peel(text.trim()) {
                    Some(peeled) => (1, peeled),
                    None => continue,
                },
            };
            let Resolution::Exact(reference) = resolve(lexicon, &reading) else {
                continue;
            };
            // Rule 2: a bare title in prose is a subject, not a mekor.
            if reference.from().is_empty() {
                continue;
            }
            // Rule 3: in prose, a mekor is a title and numbers. Anything else
            // is the resolver reading the rest of the sentence as a section
            // name.
            let numbered = |address: &girsa_ref::Address| {
                address.levels().iter().all(girsa_ref::Level::is_numbered)
            };
            if !numbered(reference.from()) || reference.to().is_some_and(|to| !numbered(to)) {
                continue;
            }
            taken = Some((
                take,
                Linked {
                    from: start + offset,
                    to: end,
                    text: reading,
                    reference: reference.to_string(),
                },
            ));
            break;
        }
        match taken {
            Some((take, linked)) => {
                out.push(linked);
                at += take;
            }
            None => at += 1,
        }
    }
    out
}

/// The same citation with a leading prefix letter taken off its first word.
///
/// `בשו"ע` → `שו"ע`. Only the eight letters that are prefixes in Hebrew, and
/// only when there is something left after them.
fn peel(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if !matches!(first, 'ו' | 'ה' | 'ב' | 'כ' | 'ל' | 'מ' | 'ש' | 'ד') {
        return None;
    }
    let rest = chars.as_str();
    (!rest.trim().is_empty()).then(|| rest.to_string())
}

/// Every word of the prose, with where it starts in characters.
fn word_positions(prose: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    let mut byte_start = 0;
    for (at, (byte, c)) in prose.char_indices().enumerate() {
        if c.is_whitespace() {
            if let Some(from) = start.take() {
                out.push((from, &prose[byte_start..byte]));
            }
        } else if start.is_none() {
            start = Some(at);
            byte_start = byte;
        }
    }
    if let Some(from) = start {
        out.push((from, &prose[byte_start..]));
    }
    out
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_ref::Work;

    fn lexicon() -> Lexicon {
        let mut lex = Lexicon::default();
        lex.add(
            Work {
                slug: "shulchan-arukh/orach-chayim".into(),
                he_title: "שולחן ערוך, אורח חיים".into(),
                en_title: "Shulchan Arukh, Orach Chayim".into(),
            },
            &["שו\"ע או\"ח", "שולחן ערוך אורח חיים", "או\"ח"],
        );
        lex.add(
            Work {
                slug: "tur/orach-chayim".into(),
                he_title: "טור, אורח חיים".into(),
                en_title: "Tur, Orach Chayim".into(),
            },
            // The same spelling as the Shulchan Arukh's volume: this is what
            // makes `או"ח` genuinely ambiguous, and it is true of the corpus.
            &["או\"ח"],
        );
        lex.add(
            Work {
                slug: "bavli/berakhot".into(),
                he_title: "ברכות".into(),
                en_title: "Berakhot".into(),
            },
            &["ברכות", "Berakhot"],
        );
        lex
    }

    #[test]
    fn a_citation_in_prose_becomes_a_ref() {
        let found = linkify(
            &lexicon(),
            "וכתב בשו\"ע או\"ח סימן א' סעיף ג' דראוי לכל ירא שמים",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].reference, "girsa:shulchan-arukh/orach-chayim/1:3");
        assert_eq!(found[0].text, "שו\"ע או\"ח סימן א' סעיף ג'");
    }

    #[test]
    fn a_citation_that_names_two_seforim_stays_plain_text() {
        // BUILDER rule 6 and spec.md §10.5: ambiguity is surfaced, never
        // resolved. `או"ח` is the Orach Chayim of the Shulchan Arukh *and* of
        // the Tur, and a linkify that picked one would be wrong half the time
        // in a way nobody could see on the printed page.
        assert!(linkify(&lexicon(), "עיין או\"ח סימן א'").is_empty());
    }

    #[test]
    fn a_bare_title_is_a_subject_and_not_a_mekor() {
        // *The Shulchan Arukh writes at length* is not a citation of anything.
        assert!(linkify(&lexicon(), "השולחן ערוך אורח חיים מאריך בזה").is_empty());
        assert!(linkify(&lexicon(), "ברכות היא מסכת ארוכה").is_empty());
    }

    #[test]
    fn the_longest_citation_wins_rather_than_the_first_title_that_matches() {
        let found = linkify(&lexicon(), "שולחן ערוך אורח חיים סימן קכ\"א סעיף ג'");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].reference,
            "girsa:shulchan-arukh/orach-chayim/121:3"
        );
    }

    #[test]
    fn two_citations_in_one_line_are_two_links() {
        let found = linkify(
            &lexicon(),
            "ברכות ב. ועיין שו\"ע או\"ח סימן נ\"ח סעיף א' בהגה",
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].reference, "girsa:bavli/berakhot/2a");
        assert_eq!(found[1].reference, "girsa:shulchan-arukh/orach-chayim/58:1");
        // And the spans are where they are: a linkifier that reported the
        // wrong offsets would put the link on the words beside the citation.
        let one: String = "ברכות ב. ועיין שו\"ע או\"ח סימן נ\"ח סעיף א' בהגה"
            .chars()
            .skip(found[1].from)
            .take(found[1].to - found[1].from)
            .collect();
        assert_eq!(one, found[1].text);
    }
}
