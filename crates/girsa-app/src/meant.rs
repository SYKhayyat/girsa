//! Which sefer a reader meant by the name they typed.
//!
//! The first half of smart navigation (spec.md §9.7, issue #63): `berachos 2a`,
//! `ברכות ב א`, `brachos 2a` are one question about one sefer, and the location
//! half is `girsa-ref`'s. What has no home anywhere in this tree is the *name* —
//! the reader types a transliteration, an abbreviation, or a title with a
//! collection word in front of it, and something has to decide which of the
//! seforim on the shelf that is.
//!
//! Deliberately **not** a search. A search answers *where does this word appear*
//! and is allowed to return a thousand things in an order; this answers *did you
//! mean this one sefer*, and its honest answers are one, several, and none. The
//! difference is what happens when it is unsure: a search shows the ranking and
//! a reader picks, and a navigation that guessed from a ranking would open the
//! wrong masechet without saying it guessed.
//!
//! The comparison is against two things a `Work` already carries: its Hebrew
//! title and its slug. The slug is the transliteration — `bavli/berakhot` is
//! how the shelf says ברכות in Latin letters — so `brachos 2a` is matched against
//! `berakhot` and not against a table of English spellings nobody would keep up
//! to date. That is the whole of the "fuzzy name matching" #63 asks for, and it
//! needs no model: name resolution is *nearly* a string comparison, and the
//! remaining third of a word is what edit distance is for.
//!
//! What it will not do is decide between two candidates. Two seforim the reader
//! could have meant is the question this module returns instead of an answer.
//!
//! # Where it lives, and why not in the search crate
//!
//! The table a name is compared against is a `Work`'s own — its Hebrew title and
//! its slug — and `girsa-search`'s `Catalogue` has those only because it was
//! handed the same works. It started there, in front of `Catalogue`, and moved:
//! `girsa-app` cannot depend on `girsa-search` (that crate's manifest says so in
//! the entry for `girsa-note`), and the command that needs to answer *did you
//! mean this sefer* — `nav_ask` — is the one that has a `Shelf`. So the input is
//! the shelf's own list of works and neither crate has to reach for the other.
//!
//! # Why this is not `girsa_app::nav`'s matcher
//!
//! `nav` resolves names through the lexicon and comes back `Nav::Nothing` for
//! spellings the lexicon does not hold — and says in its own header that it does
//! **not** paper that gap over with a fuzzy match, because a near-miss sefer is
//! the wrong-page hazard the whole address system avoids (#69). That decision
//! stands, and this module does not overturn it: it is what answers *when the
//! lexicon has already said nothing*, and what it returns is meant to be
//! **asked**, not followed. A caller that navigates on [`Meant::One`] without
//! showing it has turned a suggestion into a guess; a caller that shows
//! `berachos → ברכות?` has repaired a lexicon gap in front of the reader.
//!
//! [`Nothing`]: girsa_app::nav::Nav::Nothing

use girsa_corpus::work::Work;
use girsa_hebrew::normalize;

/// How much of a name may differ and still be the same name.
///
/// Measured rather than guessed, on the spellings #63 lists: `brachos` →
/// `berakhot` is 3 edits of 8 characters (0.38), `shabbos` → `shabbat` 2 of 7
/// (0.29), `berachos` → `berakhot` 2 of 8 (0.25), `teshuva` → `teshuvah` 1 of 8
/// (0.12). So 0.4 is the smallest bound that answers the first of those, and it
/// is loose on purpose: **choosing the wrong sefer is what the ambiguity rule is
/// for, and missing the right one is a question the reader has to retype.**
const AS_CLOSE_AS: f32 = 0.4;

/// Shorter than this and an edit is most of the name, so ratio stops meaning
/// anything: `דם` and `אם` are one edit apart and nothing alike.
const TOO_SHORT_TO_GUESS: usize = 3;

/// What the reader meant, as far as the shelf can tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Meant {
    /// One sefer, and no other reading of the name was as close.
    One(String),
    /// Several seforim the name fits equally well. The caller asks.
    ///
    /// Sorted, so a panel that lists them lists them in the same order every
    /// time the same name is typed.
    Several(Vec<String>),
    /// Nothing on the shelf is close to that. Not an error: a reader is allowed
    /// to type a sefer they do not own.
    None,
}

/// The sefer whose name that is, if it is one sefer's name.
///
/// `typed` is the name alone — the caller splits `{sefer} {location}` before
/// asking, because *which sefer* and *where in it* are two questions with two
/// different sets of candidates.
#[must_use]
pub fn meant(works: &[Work], typed: &str) -> Meant {
    let typed = key(typed);
    if typed.chars().count() < TOO_SHORT_TO_GUESS {
        return Meant::None;
    }

    // The best distance each sefer offers, and the name that was closest.
    let mut best: Vec<(f32, &str)> = Vec::new();
    for work in works {
        // Both halves of what a `Work` knows, and the slug twice: once whole and
        // once without the shelf it sits on, because `mishnayos peah` is the
        // tail and `bavli/berakhot` is a name a reader might paste.
        let mut closest = f32::INFINITY;
        for name in [work.he_title.as_str(), work.slug.as_str(), tail(&work.slug)] {
            closest = closest.min(closeness(&typed, &key(name)));
        }
        if closest.is_finite() {
            best.push((closest, work.slug.as_str()));
        }
    }

    let Some((closest, _)) = best
        .iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .copied()
        .filter(|(how, _)| *how <= AS_CLOSE_AS)
    else {
        return Meant::None;
    };

    // Everything within a rounding error of the best is owed to the reader too.
    // A tolerance and not an exact tie: `brachos` is 0.375 from both `berakhot`
    // and `berachot`, and asking *which of these two* about two names that
    // differ by one letter is the right behaviour, not a coin toss.
    let mut tied: Vec<&str> = best
        .iter()
        .filter(|(how, _)| *how <= closest + 0.05)
        .map(|(_, slug)| *slug)
        .collect();
    tied.sort_unstable();
    tied.dedup();
    match tied.len() {
        0 => Meant::None,
        1 => Meant::One(tied[0].to_string()),
        _ => Meant::Several(tied.into_iter().map(str::to_string).collect()),
    }
}

/// How far apart two names are, as a fraction of the longer one.
///
/// A reader types a name into a box that also holds a location and sometimes a
/// word or two of the shelf it came from, so their words are read twice: from the
/// front (`berachos 2a` is `berachos` and then a location) and from the back
/// (`mishnayos peah` is a collection and then the sefer). Whichever is closer
/// wins.
///
/// The **name's** words are always read from the front, and that asymmetry is the
/// point: the shelf says what a sefer is called, and a sefer is not renamed by
/// what follows its name. Reading the name from the back too would make
/// `berakhot` as close to `rashi on berakhot` as to `berakhot` — the commentary
/// that comments on a masechet would come back tied with the masechet, and a
/// reader who typed the Gemara would be asked *which of these two*.
fn closeness(typed: &str, name: &str) -> f32 {
    compare(typed, name, From::Front)
        .0
        .min(compare(typed, name, From::Back).0)
}

/// Which of the reader's words the closest name accounted for, and from which end.
///
/// The point of knowing is that the reader's line is **reusable**: `brachos 2a` and
/// the corrected `ברכות 2a` differ in the name and not in the address, and a
/// question the reader can accept should put the whole line back in the box rather
/// than the name alone — otherwise accepting `did you mean ברכות` quietly throws
/// away the `2a` they had already typed (#63).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Which end of their words the name was read from.
    pub from: From,
    /// How many of them it took.
    pub take: usize,
}

/// The best name on the shelf for a line, and the words it covers.
///
/// Same comparison and same bound as [`meant`], and deliberately the same walk: two
/// functions that decided *which sefer* differently would be able to disagree about
/// a line the box had already shown a question for.
#[must_use]
pub fn named(works: &[Work], typed: &str) -> Option<(String, Span)> {
    let words = words(typed);
    if key(typed).chars().count() < TOO_SHORT_TO_GUESS {
        return None;
    }
    let keyed = key(typed);
    let mut best: Option<(f32, &str, Span)> = None;
    for work in works {
        for name in [work.he_title.as_str(), work.slug.as_str(), tail(&work.slug)] {
            for from in [From::Front, From::Back] {
                let (how, span) = compare(&keyed, &key(name), from);
                let better = best.is_none_or(|(was, _, _)| how < was);
                if better && how.is_finite() {
                    best = Some((how, work.slug.as_str(), span_with(span, &words)));
                }
            }
        }
    }
    let (how, slug, span) = best?;
    if how > AS_CLOSE_AS {
        return None;
    }
    Some((slug.to_string(), span))
}

/// The span with the take checked against the reader's own words: `compare` counts
/// in the folded name, which is the same count, but a caller is about to slice the
/// reader's line with it and that is worth not assuming.
fn span_with(span: Span, words: &[&str]) -> Span {
    Span {
        from: span.from,
        take: span.take.min(words.len()),
    }
}

/// The words of a line, in the order they were typed.
fn words(line: &str) -> Vec<&str> {
    line.split(' ').filter(|word| !word.is_empty()).collect()
}

/// Where the reader's words are taken from when there are more of them than the
/// name has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum From {
    /// Their first words: the location after the name is extra.
    Front,
    /// Their last words: a collection word before the name is extra.
    Back,
}

/// The words that line up, compared one for one.
///
/// With the same number of words on both sides this is simply word `i` against
/// word `i`, and the **worst word decides**. Averaging is what makes `bava batra`
/// and `bava kamma` look close: half of each name is the word `bava`, and only
/// the word that tells them apart differs. A name is not a bag of words where
/// five good ones outvote the one that matters.
fn compare(typed: &str, name: &str, from: From) -> (f32, Span) {
    let typed: Vec<&str> = words(typed);
    let name: Vec<&str> = words(name);
    let take = typed.len().min(name.len());
    if take == 0 {
        return (f32::INFINITY, Span { from, take: 0 });
    }
    let theirs = match from {
        From::Front => &typed[..take],
        From::Back => &typed[typed.len() - take..],
    };
    let how = theirs
        .iter()
        .zip(&name[..take])
        .map(|(x, y)| apart(x, y))
        .fold(0.0_f32, f32::max);
    (how, Span { from, take })
}

/// One word against one word, as a fraction of the longer one.
fn apart(a: &str, b: &str) -> f32 {
    if a == b {
        return 0.0;
    }
    let longest = a.chars().count().max(b.chars().count());
    if longest == 0 || longest < TOO_SHORT_TO_GUESS {
        return f32::INFINITY;
    }
    edits(a, b) as f32 / longest as f32
}

/// The name as it compares: folded the way the index folds it, with the
/// quotation marks folded out and every separator made a space.
///
/// `רמב"ם` and `רמבם` are the same sefer and a reader types both. Folding the
/// gershayim to a quote and then dropping the quote is the whole of that; a
/// `slug`'s `/` and `-` become spaces so `bavli/berakhot` and `bavli berakhot`
/// are one string.
fn key(name: &str) -> String {
    let folded = normalize(name);
    let mut out = String::with_capacity(folded.len());
    for ch in folded.chars() {
        match ch {
            '\'' | '"' => {}
            '/' | '-' | ',' | ':' | '.' | '(' | ')' => out.push(' '),
            _ => out.push(ch),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The sefer's own name, without the shelf it sits on.
fn tail(slug: &str) -> &str {
    slug.rsplit('/').next().unwrap_or(slug)
}

/// Levenshtein distance, in characters.
///
/// Characters and not bytes: Hebrew is two bytes a letter, and a byte-distance
/// would call every Hebrew name twice as far from its transliteration as it is.
fn edits(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = Vec::with_capacity(b.len() + 1);
        current.push(i + 1);
        for (j, cb) in b.iter().enumerate() {
            let change = previous[j] + usize::from(ca != cb);
            current.push(change.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::{closeness, edits, key};

    #[test]
    fn a_name_is_folded_the_way_the_index_folds_it() {
        // The final form folds too (`normalize` does that for the index as well),
        // so the key for רמב״ם is רמבמ — and the reader typing רמבם gets it.
        assert_eq!(key("רמב\"ם"), "רמבמ");
        // Final letters fold as well — the key is what the index holds, so
        // `חיים` is stored as `חיימ` and a reader typing either finds it.
        assert_eq!(key("שולחן ערוך, אורח חיים"), "שולחנ ערוכ אורח חיימ");
        assert_eq!(key("bavli/berakhot"), "bavli berakhot");
        assert_eq!(key("  Bava   Kamma "), "bava kamma");
    }

    #[test]
    fn distance_is_counted_in_letters_and_not_in_bytes() {
        // `אב` is four bytes; one letter changed is one edit.
        assert_eq!(edits("אב", "אג"), 1);
        assert_eq!(edits("brachos", "brachos"), 0);
        assert_eq!(edits("", "abc"), 3);
    }

    #[test]
    fn the_spellings_the_issue_lists_are_all_the_same_name() {
        for (typed, slug) in [
            ("berachos", "berakhot"),
            ("brachos", "berakhot"),
            ("berachot", "berakhot"),
            ("shabbos", "shabbat"),
            ("teshuva", "teshuvah"),
        ] {
            let how = closeness(typed, slug);
            assert!(how <= 0.4, "{typed} → {slug} was {how}");
        }
    }

    #[test]
    fn a_collection_word_in_front_does_not_bury_the_name() {
        assert_eq!(closeness("mishnayos peah", "peah"), 0.0);
        // `teshuvah` is one letter from `teshuva`, and the collection words in
        // front of it do not make it further: the trailing words are tried on
        // their own.
        assert!(closeness("rambam hilchos teshuva", "teshuvah") <= 0.125);
        // And two masechtot that share their first word are still two masechtot:
        // `batra` and `kamma` are what tells them apart, and they are not close.
        assert!(closeness("bava batra", "bava kamma") > 0.4);
        assert!(closeness("bava batra", "bava batra") == 0.0);
    }
}
