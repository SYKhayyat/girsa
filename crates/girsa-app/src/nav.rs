//! What a reader meant by what they typed into one box.
//!
//! #63 asks for a single input: type any way of naming a sefer and a place, and
//! either go there or be asked. [`girsa_ref::resolve`] already answers the
//! *single citation* half of that — `ברכות ב א`, `Berakhot 2a`, `רמב"ם הל' תשובה
//! ג ד` — and returns `Exact` / `Ambiguous` / `Unresolved` without ever guessing,
//! which is the rule the whole reference system rests on.
//!
//! Two things are left, and both are shape rather than parsing:
//!
//! 1. **More than one target.** `Berachos 2a, Shabbos 3b` is two places. Handed
//!    whole to the resolver it is *one* citation with a comma in it, and it comes
//!    back `Exact` with a fabricated address — `girsa:bavli/berakhot/2a:shabbos:3b`
//!    — which is worse than any failure this module can produce, because it
//!    resolves, it opens, and the page is wrong. So the split happens here,
//!    before the resolver sees anything.
//! 2. **A location with no sefer.** `2a` names no sefer, and only the Bavli
//!    paginates by daf, so the honest answer is the question rather than a pick
//!    from forty masechtos.
//!
//! Parentheses are the issue's source-sheet spelling: the same refs, assembled
//! rather than opened side by side. That is a difference in what the window does
//! with them, not in what they are, so it travels as a flag ([`Several::sheet`])
//! rather than as a second parser.
//!
//! # Where the names come from
//!
//! The lexicon is the shipped `corpus/lexicon.tsv`, generated from Sefaria's
//! schemas, so a sefer answers to what Sefaria calls it and to its Hebrew title.
//! What it does **not** cover is how people type: `brachos` (one vowel away from
//! the shipped `Berachos`), `rambam hilchos teshuva` (Sefaria files it as
//! `Mishneh Torah, Repentance`), `mishnayos peah`, `shulchan aruch orech chaim`.
//! Those come back `Nothing` — see #69, which is a lexicon gap and not a parsing
//! one, and which this module deliberately does not paper over with a fuzzy
//! match, because a near-miss sefer is the wrong-page hazard this whole system
//! is arranged to avoid.

use girsa_corpus::work::Work;
use girsa_ref::{resolve, Level, Lexicon, Ref, Resolution};

use crate::meant::{meant, named, From, Meant};

/// What the box got, in the terms the window needs to act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Nav {
    /// One place, and nothing else it could equally have been.
    Go(Ref),
    /// Two or more places. `sheet` says the reader wrote them in parentheses.
    ///
    /// `unread` is the parts this could not read, in the order they were
    /// typed, and it is what turns a list with one bad member from *nothing
    /// matched* — which is untrue and is the only answer that tells a reader
    /// their words meant nothing — into a true partial answer. Before it,
    /// `ברכות ב א, או"ח א'` came back [`Nav::Nothing`]: one ambiguous member and
    /// the two places that were perfectly clear went with it (issue #74).
    Several {
        refs: Vec<Ref>,
        unread: Vec<Unread>,
        sheet: bool,
    },
    /// Several places the words genuinely name. Always shown as a choice, in
    /// the order the resolver gave them, because narrowing it would be a guess.
    Choose(Vec<Ref>),
    /// A question, because the words name a place but not what it is inside.
    Ask(Ask),
    /// Nothing in the lexicon. Better than a wrong answer, and the window says so.
    Nothing,
}

/// Something in a list that could not be answered, in the terms the window
/// needs to say it.
///
/// Three kinds, because they are three different sentences to a reader: *I do
/// not know a sefer by that name*, *those words name more than one sefer*, and
/// *that is a place, but you did not say inside what*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unread {
    /// A bare location: `2b`, `סימן א`. The same fact [`Nav::Ask`] carries when
    /// it is the whole of what was typed, here beside the parts that worked.
    WhichSefer(String),
    /// Words the lexicon does not hold. See #69, which is the gap this is
    /// honest about rather than papering over.
    Nothing(String),
    /// Words that name more than one place. The candidates travel with them so
    /// the window can offer the choice rather than only complain.
    Choice { typed: String, candidates: Vec<Ref> },
}

/// The question [`Nav::Ask`] is asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// A location with no sefer — `2a`, `סימן א`. The window offers the seforim
    /// whose address scheme the location belongs to.
    WhichSefer(String),
}

/// What the box got, once the shelf has had its say about a name it did not know.
///
/// [`read`] answers from the lexicon and nothing else, deliberately: a near-miss
/// sefer is the wrong-page hazard this whole system is arranged against (#69, and
/// the module doc above). But *the lexicon does not hold that spelling* is not the
/// same as *there is nothing to say* — the shelf knows what its seforim are called,
/// and a reader who typed `brachos` is owed the fact that it is close to one of
/// them, in the form of a question.
///
/// So this is the layer above [`read`], and its whole content is the rule about
/// when the second answer is allowed:
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Suggest {
    /// The lexicon read it. Whatever it said is the answer, including *nothing*.
    Read(Nav),
    /// The lexicon could not read it and one sefer's name is close. **A question,
    /// not a landing**: the window shows it and the reader decides.
    DidYouMean {
        /// What they typed, so the question can quote it back.
        typed: String,
        /// The sefer that is close.
        slug: String,
        /// Its name as the shelf has it, which is what the question offers.
        title: String,
        /// Their own line with the name corrected and **the address left alone**:
        /// `brachos 2a` is offered as `ברכות 2a`, because accepting the question
        /// should not throw away the place they already typed (#63).
        corrected: String,
    },
    /// …and more than one sefer is close. A choice, in the order the shelf lists
    /// them, because narrowing it would be the guess this file refuses.
    Which {
        typed: String,
        candidates: Vec<Maybe>,
    },
}

/// One sefer a name could have meant: its slug and its name.
///
/// A struct and not a pair, because this crosses the wire to the window (through
/// the shell's own `Answer`, which is where `serde` lives) and a tuple arrives as
/// an array whose two elements nothing names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Maybe {
    /// The ref slug, which is what a question has to be turned into if chosen.
    pub slug: String,
    /// The name as the shelf has it, which is what the reader is shown.
    pub title: String,
    /// The reader's line with this name in it, address and all.
    pub corrected: String,
}

/// The reader's line with the matched name replaced, and every other word left
/// exactly where they put it.
///
/// The span comes from [`named`], which walks the same comparison [`meant`] does,
/// so the words replaced are the words that were compared. A line with nothing left
/// to keep is just the name.
fn corrected(works: &[Work], typed: &str, title: &str) -> String {
    let Some((_, span)) = named(works, typed) else {
        return title.to_string();
    };
    let mut words: Vec<&str> = typed.split_whitespace().collect();
    let end = words.len();
    match span.from {
        From::Front => words.splice(0..span.take.min(end), [title]),
        From::Back => words.splice(end.saturating_sub(span.take)..end, [title]),
    };
    words.join(" ")
}

/// The name a slug's sefer has, or the slug itself where the shelf has no name for
/// it — the same fallback `naming` uses, so a question cannot offer less than a
/// search hit does.
fn titled(works: &[Work], slug: &str) -> String {
    works
        .iter()
        .find(|work| work.slug == slug)
        .map_or_else(|| slug.to_string(), |work| work.he_title.clone())
}

/// Read the box, and if the lexicon had nothing, ask the shelf whether the name is
/// almost one of its seforim.
///
/// The order is the whole design: [`read`] first, and [`meant`] **only** on
/// [`Nav::Nothing`]. A sefer the lexicon read is not second-guessed by a nearer
/// spelling, because the lexicon is the authority on what a sefer is called and
/// fuzzy matching is not. What this adds is the one thing the lexicon cannot do:
/// notice a name it does not hold and say what it is probably meant to be.
#[must_use]
pub fn suggest(lexicon: &Lexicon, works: &[Work], typed: &str) -> Suggest {
    let read = read(lexicon, typed);
    if read != Nav::Nothing {
        return Suggest::Read(read);
    }
    // The **name** alone: a trailing location is `girsa-ref`'s half of the line,
    // and `meant` reads the words it is given from the front or the back to find
    // it. A bare `2a` has no name in it at all and `meant` says so.
    match meant(works, typed) {
        Meant::One(slug) => {
            let title = titled(works, &slug);
            let corrected = corrected(works, typed, &title);
            Suggest::DidYouMean {
                title,
                corrected,
                typed: typed.to_string(),
                slug,
            }
        }
        Meant::Several(slugs) => Suggest::Which {
            typed: typed.to_string(),
            candidates: slugs
                .into_iter()
                .map(|slug| {
                    let title = titled(works, &slug);
                    let corrected = corrected(works, typed, &title);
                    Maybe {
                        title,
                        corrected,
                        slug,
                    }
                })
                .collect(),
        },
        Meant::None => Suggest::Read(Nav::Nothing),
    }
}

/// Read the box.
///
/// The order matters: split first, resolve second. See the module doc for the
/// address the other order fabricates.
#[must_use]
pub fn read(lexicon: &Lexicon, typed: &str) -> Nav {
    let typed = typed.trim();
    if typed.is_empty() {
        return Nav::Nothing;
    }

    let (body, sheet) = if typed.len() > 2 && typed.starts_with('(') && typed.ends_with(')') {
        (typed[1..typed.len() - 1].trim(), true)
    } else {
        (typed, false)
    };

    // One citation first, when the address it produced is a place. It has to be
    // checked, because a comma is also a separator inside a title: Sefaria
    // writes `Shulchan Arukh, Orach Chayim`, and the whole string there *is* one
    // citation. What the check rejects is the other reading — `berakhot/2a`
    // followed by levels named `shabbos` and `3b` — which is not a place and so
    // cannot be the answer.
    match resolve(lexicon, body) {
        Resolution::Exact(one) if is_a_place(&one) => {
            return match sheet {
                true => Nav::Several {
                    refs: vec![one],
                    unread: Vec::new(),
                    sheet: true,
                },
                false => Nav::Go(one),
            };
        }
        Resolution::Ambiguous(candidates) => return Nav::Choose(candidates),
        _ => {}
    }

    // Not one place: read it as a list of them. **A part that fails does not
    // take the list down with it** — it is named in `unread` and the parts that
    // did resolve are answered, which is the difference between *two of these
    // are places and one I could not read* and *your words meant nothing*
    // (issue #74).
    let mut refs = Vec::new();
    let mut unread = Vec::new();
    for part in split_at_commas(body) {
        match resolve(lexicon, part.as_str()) {
            Resolution::Exact(one) => refs.push(one),
            Resolution::Ambiguous(candidates) => unread.push(Unread::Choice {
                typed: part.clone(),
                candidates,
            }),
            Resolution::Unresolved => match bare_place(&part) {
                Some(bare) => unread.push(Unread::WhichSefer(bare)),
                None => unread.push(Unread::Nothing(part)),
            },
        }
    }

    if refs.is_empty() {
        // Nothing came of any of it. The two shapes that are a question rather
        // than a refusal keep their question, because the reader's next step
        // differs: one wants a sefer chosen, the other wants a name corrected.
        return match unread.as_slice() {
            [Unread::WhichSefer(bare)] => Nav::Ask(Ask::WhichSefer(bare.clone())),
            [Unread::Choice { candidates, .. }] => Nav::Choose(candidates.clone()),
            _ => Nav::Nothing,
        };
    }
    if unread.is_empty() && refs.len() == 1 {
        return Nav::Several {
            refs,
            unread,
            sheet: true,
        };
    }
    Nav::Several {
        refs,
        unread,
        sheet,
    }
}

/// Whether the address the resolver produced is a place at all.
///
/// Every level must be a number or a daf. A level holding a sefer's name means
/// the words were read as one citation when they were two.
fn is_a_place(one: &Ref) -> bool {
    one.from().levels().iter().all(Level::is_numbered)
}

/// Split on commas that are not inside parentheses.
///
/// `Berachos 2a, Shabbos 3b` is two targets. A comma inside a title is not:
/// Sefaria writes `Shulchan Arukh, Orach Chayim`, and splitting that would turn
/// one sefer into two unresolvable halves.
fn split_at_commas(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (at, ch) in body.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(body[start..at].trim().to_string());
                start = at + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(body[start..].trim().to_string());
    parts.retain(|p| !p.is_empty());
    parts
}

/// The bare location, if the words are only one: `2a`, `3b`, `2.`, `1:2`,
/// `סימן א`, `קכ"א ג`.
///
/// Deliberately narrow. It is the one unresolved shape that becomes a question
/// rather than `Nothing`, so it has to be a shape no sefer is named by.
///
/// **It reads Hebrew numerals now, and it did not.** The rule was written over
/// ASCII digits, `a`, `b`, `:`, `.`, geresh and gershayim — so `סימן א`, which
/// is a location written in the alphabet every other part of this system reads
/// perfectly well, came back *nothing matched* (issue #74). `girsa_ref`'s
/// numeral reader is the same one the resolver sums addresses with, and it is
/// strict in exactly the way this function has to be: it refuses a word that
/// merely sums to something (`שבת` is 702 and is not a numeral), so a typo'd
/// sefer name does not become a question about which sefer.
///
/// A **section word** is allowed and is not a number: `סימן`, `פרק`, `עמ'` and
/// the rest label the number after them, and `girsa_ref::resolve::is_section_word`
/// is that list rather than a second copy of it.
fn bare_place(part: &str) -> Option<String> {
    let part = part.trim();
    let mut seen_number = false;
    for token in part.split_whitespace() {
        if let Some(numbered) = ascii_token(token) {
            seen_number = seen_number || numbered;
            continue;
        }
        if girsa_ref::resolve::is_section_word(token) {
            continue;
        }
        if girsa_ref::numerals::parse(token).is_some() {
            seen_number = true;
            continue;
        }
        return None;
    }
    seen_number.then(|| part.to_string())
}

/// An ASCII place token, and whether it carried a number itself.
///
/// `None` is *not an ASCII place token at all*, which is a different answer
/// from *an ASCII token with no digit in it* — a daf's `a` is the second of
/// those, and `2a` is the first.
fn ascii_token(token: &str) -> Option<bool> {
    let mut numbered = false;
    for ch in token.chars() {
        match ch {
            '0'..='9' => numbered = true,
            'a' | 'b' | ':' | '.' | '\'' | '"' => {}
            _ => return None,
        }
    }
    Some(numbered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use girsa_ref::Work;

    fn lexicon() -> Lexicon {
        let mut lexicon = Lexicon::default();
        lexicon.add(
            Work {
                slug: "bavli/berakhot".into(),
                he_title: "ברכות".into(),
                en_title: "Berakhot".into(),
            },
            &["Berakhot", "Berachos", "ברכות"],
        );
        lexicon.add(
            Work {
                slug: "bavli/shabbat".into(),
                he_title: "שבת".into(),
                en_title: "Shabbat".into(),
            },
            &["Shabbat", "Shabbos", "שבת"],
        );
        lexicon.add(
            Work {
                slug: "shulchan-arukh/orach-chayim".into(),
                he_title: "שולחן ערוך, אורח חיים".into(),
                en_title: "Shulchan Arukh, Orach Chayim".into(),
            },
            &["Shulchan Arukh, Orach Chayim", "שו\"ע או\"ח", "או\"ח"],
        );
        lexicon.add(
            Work {
                slug: "tur/orach-chayim".into(),
                he_title: "טור, אורח חיים".into(),
                en_title: "Tur, Orach Chayim".into(),
            },
            &["או\"ח"],
        );
        lexicon
    }

    fn unread(nav: &Nav) -> Vec<String> {
        match nav {
            Nav::Several { unread, .. } => unread
                .iter()
                .map(|u| match u {
                    Unread::WhichSefer(what) | Unread::Nothing(what) => what.clone(),
                    Unread::Choice { typed, .. } => typed.clone(),
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn refs(nav: &Nav) -> Vec<String> {
        match nav {
            Nav::Go(one) => vec![one.to_string()],
            Nav::Several { refs, .. } => refs.iter().map(ToString::to_string).collect(),
            Nav::Choose(candidates) => candidates.iter().map(ToString::to_string).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn one_place_plainly_named_is_followed() {
        assert!(matches!(read(&lexicon(), "Berachos 2a"), Nav::Go(_)));
        assert_eq!(
            refs(&read(&lexicon(), "Berachos 2a")),
            vec!["girsa:bavli/berakhot/2a"]
        );
        assert_eq!(
            refs(&read(&lexicon(), "ברכות ב א")),
            vec!["girsa:bavli/berakhot/2:1"]
        );
    }

    /// The regression this module exists for. Before it, the comma-separated
    /// form was one citation with a comma in it and the resolver answered with
    /// an address that is not a place: `girsa:bavli/berakhot/2a:shabbos:3b`.
    #[test]
    fn two_places_are_two_targets_and_never_one_invented_address() {
        let nav = read(&lexicon(), "Berachos 2a, Shabbos 3b");
        assert_eq!(
            refs(&nav),
            vec!["girsa:bavli/berakhot/2a", "girsa:bavli/shabbat/3b"],
            "two places, in the order they were typed"
        );
        assert!(matches!(nav, Nav::Several { sheet: false, .. }));
        assert!(
            !refs(&nav).iter().any(|r| r.contains("berakhot/2a:")),
            "the merged address must not survive the split"
        );
    }

    #[test]
    fn parentheses_are_the_same_places_assembled_rather_than_opened() {
        let nav = read(&lexicon(), "(Berachos 2a, Shabbos 3b)");
        assert_eq!(refs(&nav).len(), 2);
        assert!(matches!(nav, Nav::Several { sheet: true, .. }));
    }

    #[test]
    fn one_target_in_parentheses_is_still_a_sheet_of_one() {
        assert!(matches!(
            read(&lexicon(), "(Berachos 2a)"),
            Nav::Several { sheet: true, .. }
        ));
    }

    #[test]
    fn a_location_with_no_sefer_is_a_question_not_a_pick() {
        assert_eq!(
            read(&lexicon(), "2a"),
            Nav::Ask(Ask::WhichSefer("2a".into()))
        );
        // Hebrew numerals are how the rest of this system writes an address, and
        // this read only ASCII digits until issue #74 — so `סימן א` was *nothing
        // matched*, about a location the resolver beside it reads without
        // complaint. The numeral reader is `girsa_ref`'s, not a second one.
        assert_eq!(
            read(&lexicon(), "סימן א"),
            Nav::Ask(Ask::WhichSefer("סימן א".into()))
        );
        assert_eq!(
            read(&lexicon(), "קכ\"א"),
            Nav::Ask(Ask::WhichSefer("קכ\"א".into()))
        );
        assert_eq!(
            read(&lexicon(), "פרק ג משנה א"),
            Nav::Ask(Ask::WhichSefer("פרק ג משנה א".into()))
        );
    }

    #[test]
    fn a_word_that_only_sums_to_a_number_is_not_a_place() {
        // `שבת` sums to 702 and `ייחוד` to 38. A numeral is *written* high to
        // low and in its canonical spelling, which is the rule that keeps a
        // sefer's name from becoming an address — the same rule, borrowed, so
        // that a question about which sefer is only asked when the words really
        // are a location.
        assert_eq!(read(&lexicon(), "ייחוד"), Nav::Nothing);
        assert_eq!(read(&lexicon(), "סמין א"), Nav::Nothing);
    }

    #[test]
    fn a_name_two_seforim_answer_to_is_a_choice_in_the_resolvers_order() {
        let nav = read(&lexicon(), "או\"ח א' א'");
        match &nav {
            Nav::Choose(candidates) => assert_eq!(candidates.len(), 2),
            other => panic!("expected a choice, got {other:?}"),
        }
    }

    /// The lexicon gap, as absence rather than as a guess — see #69.
    #[test]
    fn a_spelling_the_lexicon_does_not_hold_is_nothing_rather_than_a_near_miss() {
        assert_eq!(read(&lexicon(), "brachos 2a"), Nav::Nothing);
        assert_eq!(read(&lexicon(), ""), Nav::Nothing);
        assert_eq!(read(&lexicon(), "   "), Nav::Nothing);
    }

    #[test]
    fn one_part_of_a_list_that_names_two_seforim_does_not_take_the_list_down() {
        // This was `Nav::Nothing`: one ambiguous member and the two places that
        // were clear went with it, and *nothing matched* is the only answer that
        // tells a reader their words meant nothing. It is a lie here (issue #74).
        let nav = read(&lexicon(), "ברכות ב א, או\"ח א'");
        assert_eq!(
            refs(&nav),
            vec!["girsa:bavli/berakhot/2:1"],
            "the part that resolved is still there"
        );
        assert_eq!(
            unread(&nav),
            vec!["או\"ח א'"],
            "and the part that did not is named"
        );
        let Nav::Several { unread, .. } = &nav else {
            panic!("expected several, got {nav:?}");
        };
        let Unread::Choice { candidates, .. } = &unread[0] else {
            panic!("a choice keeps its candidates: {unread:?}");
        };
        assert_eq!(candidates.len(), 2, "both seforim, so the window can ask");
    }

    #[test]
    fn a_list_with_one_part_nobody_knows_answers_the_parts_it_does() {
        let nav = read(&lexicon(), "Berachos 2a, סמין ג");
        assert_eq!(refs(&nav), vec!["girsa:bavli/berakhot/2a"]);
        assert_eq!(unread(&nav), vec!["סמין ג"]);
    }

    #[test]
    fn a_list_where_nothing_resolves_is_still_nothing() {
        assert_eq!(read(&lexicon(), "סמין ג, קוקו ב"), Nav::Nothing);
    }

    #[test]
    fn a_comma_inside_a_title_is_not_a_second_target() {
        assert_eq!(
            refs(&read(&lexicon(), "Shulchan Arukh, Orach Chayim 1:1")),
            vec!["girsa:shulchan-arukh/orach-chayim/1:1"]
        );
    }
}
