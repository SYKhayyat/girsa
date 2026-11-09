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

use girsa_ref::{resolve, Level, Lexicon, Ref, Resolution};

/// What the box got, in the terms the window needs to act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Nav {
    /// One place, and nothing else it could equally have been.
    Go(Ref),
    /// Two or more places. `sheet` says the reader wrote them in parentheses.
    Several { refs: Vec<Ref>, sheet: bool },
    /// Several places the words genuinely name. Always shown as a choice, in
    /// the order the resolver gave them, because narrowing it would be a guess.
    Choose(Vec<Ref>),
    /// A question, because the words name a place but not what it is inside.
    Ask(Ask),
    /// Nothing in the lexicon. Better than a wrong answer, and the window says so.
    Nothing,
}

/// The question [`Nav::Ask`] is asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// A location with no sefer — `2a`, `סימן א`. The window offers the seforim
    /// whose address scheme the location belongs to.
    WhichSefer(String),
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
                    sheet: true,
                },
                false => Nav::Go(one),
            };
        }
        Resolution::Ambiguous(candidates) => return Nav::Choose(candidates),
        _ => {}
    }

    // Not one place: read it as a list of them.
    let mut refs = Vec::new();
    for part in split_at_commas(body) {
        match resolve(lexicon, part.as_str()) {
            Resolution::Exact(one) => refs.push(one),
            // A list where one target is itself a choice has no shape in the
            // window yet, and picking for the reader would be the guess this
            // system is arranged to avoid. Reported as unmatched rather than
            // half-answered.
            Resolution::Ambiguous(_) => return Nav::Nothing,
            Resolution::Unresolved => {
                return match bare_place(part.as_str()) {
                    Some(bare) => Nav::Ask(Ask::WhichSefer(bare)),
                    None => Nav::Nothing,
                };
            }
        }
    }

    match refs.len() {
        0 => Nav::Nothing,
        1 => Nav::Several { refs, sheet: true },
        _ => Nav::Several { refs, sheet },
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

/// The bare location, if the words are only one: `2a`, `3b`, `2.`, `1:2`.
///
/// Deliberately narrow. It is the *only* unresolved shape that becomes a
/// question rather than `Nothing`, so it has to be a shape no sefer is named by.
fn bare_place(part: &str) -> Option<String> {
    let part = part.trim();
    let mut seen_digit = false;
    for ch in part.chars() {
        match ch {
            '0'..='9' => seen_digit = true,
            'a' | 'b' | ':' | '.' | ' ' | '\'' | '"' => {}
            _ => return None,
        }
    }
    seen_digit.then(|| part.to_string())
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
        assert_eq!(
            read(&lexicon(), "סימן א"),
            Nav::Nothing,
            "Hebrew numerals are a place, but this is not the shape yet — see #63"
        );
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
    fn a_comma_inside_a_title_is_not_a_second_target() {
        assert_eq!(
            refs(&read(&lexicon(), "Shulchan Arukh, Orach Chayim 1:1")),
            vec!["girsa:shulchan-arukh/orach-chayim/1:1"]
        );
    }
}
