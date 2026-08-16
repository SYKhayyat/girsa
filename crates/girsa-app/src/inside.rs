//! Finding a phrase **inside the sefer in front of you**.
//!
//! # Why this is not the search bar
//!
//! Girsa searches the whole shelf and narrows by facet, and that is a good
//! search. It is not this gesture. A person reading Mishnah Berurah who wants
//! the next place it says `שהחינו` does not want a ranked list of every sefer
//! that says it, filtered back down to the one they are already holding — they
//! want the next one, then the one after that, and the count of how many there
//! are. Ctrl+F, which every application has had for forty years and this one
//! did not.
//!
//! Otzaria has it and Girsa did not, and that was the only axis on which
//! Otzaria was plainly better at *reading*.
//!
//! # Why it is a command and not a `find()` in the window
//!
//! The pane holds a window of lines around where the reader is standing, not
//! the sefer. A find written in the window would search what happens to be
//! loaded — a few hundred lines out of Mishnah Berurah's seventeen thousand —
//! and report *no more matches* while sitting on eleven of them. So the whole
//! sefer is scanned here, where the whole sefer is.
//!
//! # What counts as a match
//!
//! What a person types, against what the page shows, both folded the same way:
//!
//! * **the pointing comes off both sides**, so `שהחינו` finds `שֶׁהֶחֱיָנוּ`.
//!   Berakhot is fully menukad and a find that needed the nikud typed would be
//!   a find nobody could use on it;
//! * **punctuation and gershayim come off**, so `שוע` finds `שו"ע`;
//! * **runs of space collapse**, so a phrase across a line break is still one
//!   phrase.
//!
//! And nothing else. No stemming, no prefix-stripping, no widening — this is
//! Ctrl+F, and spec.md §9's rule about never widening a query silently applies
//! with more force here than anywhere, because a reader watching a highlight
//! move down a page can see exactly what was matched.

use crate::display::Shown;
use crate::session::Pointing;
use crate::shemos::Shemos;
use crate::Open;

/// How many places are reported before the list is cut.
///
/// Cut and **counted**, never silently: a find that stops at 500 and says
/// `500` reads as *that is all of them*, and on a common word in a large sefer
/// it is not.
pub const MOST: usize = 500;

/// One place in this sefer where the phrase is.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Found {
    /// The segment it is in.
    pub id: String,
    /// Which line of the sefer that is, so the pane can go there without
    /// asking a second question.
    pub at: usize,
    /// The address, printed the way the margin prints it.
    pub address: String,
    /// Where the match is in the **drawn** text, in characters.
    ///
    /// The same coordinate the pane highlights in and the same one a copy
    /// counts in — see [`crate::display::Shown`], which is what produced it.
    /// Not an offset into the segment on disk: those differ by every mark and
    /// every tag in the line, which in Berakhot is most of it.
    pub from: usize,
    pub to: usize,
}

/// Everything a find found.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Inside {
    pub places: Vec<Found>,
    /// How many there are, which is not `places.len()` once [`MOST`] has cut
    /// the list.
    pub total: usize,
}

/// What the find bar's scope is called, where the engine says what it searched.
pub const THIS_SEFER: &str = "this sefer";

/// Where the words a search matched are **on the drawn page**.
///
/// # Why the engine does not answer this itself
///
/// > *"the search should be the same as regular girsa search (with all the
/// > options)."*
///
/// It is: the modes, the match, the together and the refusals all come from
/// `girsa_search`, scoped to one sefer. What the engine cannot hand back is an
/// offset the pane can highlight. Its marks are byte ranges into **the text it
/// indexed**, and the pane draws a different string — the reader's corrections
/// applied, the shemos rewritten, as much of the pointing as they asked for.
/// The two differ by every mark, every tag and every shem on the line.
///
/// So the division is: the engine says *which segments, and which words*, and
/// this says *where those words are on the page in front of you*. One idea of
/// what matches, one idea of where things are, and neither one guessing at the
/// other's job.
///
/// Every occurrence of a matched word is reported, which can be more than the
/// engine marked — a word twice in one se'if is two stops for a reader walking
/// with the arrow keys, and it is what every find bar does.
#[must_use]
pub fn where_marked(
    sefer: &Open,
    hits: &[(girsa_corpus::segment::SegmentId, Vec<String>)],
    pointing: Pointing,
    shemos: Shemos,
    style: girsa_cite::CiteStyle,
) -> Inside {
    let mut places: Vec<Found> = Vec::new();
    for (id, words) in hits {
        let Some(at) = sefer.position_of(id) else {
            continue;
        };
        let Some(segment) = sefer.segments.get(at) else {
            continue;
        };
        let said = crate::shemos::written(&segment.text, shemos);
        let shown = Shown::of(&said, pointing);
        let folded = fold(shown.text());
        let address = crate::sending::printed_address_in(
            &sefer.work,
            Some(sefer.sections()),
            &segment.id,
            style,
        );
        for word in words {
            let wanted = fold(word).chars;
            if wanted.is_empty() {
                continue;
            }
            for hit in folded.every(&wanted) {
                let (Some(&from), Some(&last)) =
                    (folded.at.get(hit), folded.at.get(hit + wanted.len() - 1))
                else {
                    continue;
                };
                places.push(Found {
                    id: segment.id.to_string(),
                    at,
                    address: address.clone(),
                    from,
                    to: last + 1,
                });
            }
        }
    }
    // **Reading order, and each place once.** The engine answers by rank, which
    // is right for a list you read and wrong for a bar you walk down a page
    // with; and two matched words that overlap in one se'if would otherwise be
    // two stops on the same letters.
    places.sort_unstable_by_key(|place| (place.at, place.from, place.to));
    places.dedup_by(|a, b| a.at == b.at && a.from == b.from && a.to == b.to);
    let total = places.len();
    places.truncate(MOST);
    Inside { places, total }
}

/// Every place in this sefer that says `query`, matched plainly.
///
/// The fold and nothing else — no modes, no widening. Kept because it is what
/// [`where_marked`] is measured against and because a caller with no search
/// index still has a find: `girsa_search` is an index on disk, and a fresh
/// install has none until `girsa-index` has run.
///
/// An empty or all-punctuation query finds nothing rather than everything,
/// which is what a find bar with nothing typed in it should do.
#[must_use]
pub fn find(
    sefer: &Open,
    query: &str,
    pointing: Pointing,
    shemos: Shemos,
    style: girsa_cite::CiteStyle,
) -> Inside {
    let wanted = fold(query).chars;
    if wanted.is_empty() {
        return Inside {
            places: Vec::new(),
            total: 0,
        };
    }
    let mut places = Vec::new();
    let mut total = 0;
    for (at, segment) in sefer.segments.iter().enumerate() {
        // The line exactly as the pane draws it: the shemos as the reader asked
        // for them, then the pointing. A find that searched the text on disk
        // would report an offset the pane cannot highlight.
        let said = crate::shemos::written(&segment.text, shemos);
        let shown = Shown::of(&said, pointing);
        let folded = fold(shown.text());
        for hit in folded.every(&wanted) {
            total += 1;
            if places.len() >= MOST {
                continue;
            }
            let (Some(&from), Some(&last)) =
                (folded.at.get(hit), folded.at.get(hit + wanted.len() - 1))
            else {
                continue;
            };
            places.push(Found {
                id: segment.id.to_string(),
                at,
                address: crate::sending::printed_address_in(
                    &sefer.work,
                    Some(sefer.sections()),
                    &segment.id,
                    style,
                ),
                from,
                to: last + 1,
            });
        }
    }
    Inside { places, total }
}

/// A piece of text folded to what a person types, with the way back.
struct Folded {
    /// The folded characters.
    chars: Vec<char>,
    /// Per folded character, which character of the text it came from.
    at: Vec<usize>,
}

impl Folded {
    /// Where `wanted` sits in this, every time, **including overlaps**.
    ///
    /// Overlapping on purpose: `אאא` contains `אא` twice, and a find that
    /// reported one would be under-counting a real thing on the page. It is
    /// also what every editor's find does.
    fn every(&self, wanted: &[char]) -> Vec<usize> {
        if wanted.is_empty() || wanted.len() > self.chars.len() {
            return Vec::new();
        }
        (0..=self.chars.len() - wanted.len())
            .filter(|&at| self.chars[at..at + wanted.len()] == *wanted)
            .collect()
    }
}

/// Fold text the way both sides of a find are folded.
///
/// Marks out, punctuation and gershayim to nothing, runs of space to one
/// space, no leading space. The map back is per character, because that is the
/// only way an offset survives — see the note on `Shown`, which makes the same
/// argument one layer down.
fn fold(text: &str) -> Folded {
    let mut chars = Vec::new();
    let mut at = Vec::new();
    let mut after_a_space = true;
    for (index, ch) in text.chars().enumerate() {
        if girsa_hebrew::is_mark(ch) {
            continue;
        }
        if ch.is_whitespace() {
            if !after_a_space {
                chars.push(' ');
                at.push(index);
                after_a_space = true;
            }
            continue;
        }
        if !ch.is_alphanumeric() {
            // Gershayim, geresh, a comma, a maqaf. A person typing `שוע` means
            // `שו"ע`, and one typing `אתהשמים` does not mean `אֶת־הַשָּׁמַיִם`
            // — so a maqaf folds to nothing here and to a **space** in the
            // index (spec.md §9.1, and the README records why). Two different
            // questions: the index has to keep two words findable apart, and a
            // find bar has to match a phrase somebody is looking at.
            continue;
        }
        chars.push(ch);
        at.push(index);
        after_a_space = false;
    }
    while chars.last() == Some(&' ') {
        chars.pop();
        at.pop();
    }
    Folded { chars, at }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_cite::CiteStyle;

    fn sefer() -> Open {
        crate::pretend::sefer(
            "s",
            "ספר",
            &["סימן", "סעיף"],
            &[
                "בָּרוּךְ אַתָּה יְיָ אֱלֹהֵינוּ מֶלֶךְ הָעוֹלָם",
                "שֶׁהֶחֱיָנוּ וְקִיְּמָנוּ וְהִגִּיעָנוּ לַזְּמַן הַזֶּה",
                "וכתב בשו\"ע שהחינו על פרי חדש",
                "אין כאן כלום",
            ],
        )
    }

    fn found(query: &str) -> Vec<(usize, usize, usize)> {
        find(
            &sefer(),
            query,
            Pointing::Full,
            Shemos::AsWritten,
            CiteStyle::HebrewFull,
        )
        .places
        .into_iter()
        .map(|p| (p.at, p.from, p.to))
        .collect()
    }

    #[test]
    fn a_phrase_typed_bare_is_found_in_menukad_text() {
        // The whole point. Berakhot is fully menukad and a find that needed
        // `שֶׁהֶחֱיָנוּ` typed with its nikud is a find nobody would use.
        let hits = found("שהחינו");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, 1);
        assert_eq!(hits[1].0, 2);
    }

    #[test]
    fn the_offsets_name_the_words_that_were_matched() {
        let sefer = sefer();
        let hits = find(
            &sefer,
            "שהחינו",
            Pointing::Full,
            Shemos::AsWritten,
            CiteStyle::HebrewFull,
        );
        for place in &hits.places {
            let shown = Shown::of(&sefer.segments[place.at].text, Pointing::Full);
            let words: String = shown
                .text()
                .chars()
                .skip(place.from)
                .take(place.to - place.from)
                .collect();
            // What was highlighted reads back as the word, pointing and all.
            let bare: String = words
                .chars()
                .filter(|c| !girsa_hebrew::is_mark(*c))
                .collect();
            assert_eq!(bare, "שהחינו", "{words}");
        }
    }

    #[test]
    fn gershayim_are_not_something_a_person_has_to_type() {
        assert_eq!(found("שוע").len(), 1);
        assert_eq!(found("שו\"ע").len(), 1);
    }

    #[test]
    fn a_phrase_of_several_words_is_one_match() {
        assert_eq!(found("שהחינו וקימנו").len(), 1);
        // And a phrase that is not there is not rounded to the nearest thing.
        assert!(found("שהחינו ומלכנו").is_empty());
    }

    #[test]
    fn nothing_typed_finds_nothing_rather_than_everything() {
        assert!(found("").is_empty());
        assert!(found("   ").is_empty());
        assert!(found("\"'").is_empty());
    }

    #[test]
    fn the_pointing_a_reader_has_off_does_not_move_the_offsets() {
        // The same words, found in the same lines, whichever way the page is
        // drawn — and the offsets are into **that** drawing, so the pane can
        // highlight what it is holding.
        for pointing in [Pointing::Full, Pointing::Nikud, Pointing::Plain] {
            let hits = find(
                &sefer(),
                "שהחינו",
                pointing,
                Shemos::AsWritten,
                CiteStyle::HebrewFull,
            );
            assert_eq!(hits.total, 2, "{pointing:?}");
            let sefer = sefer();
            for place in &hits.places {
                let shown = Shown::of(&sefer.segments[place.at].text, pointing);
                let words: String = shown
                    .text()
                    .chars()
                    .skip(place.from)
                    .take(place.to - place.from)
                    .collect();
                let bare: String = words
                    .chars()
                    .filter(|c| !girsa_hebrew::is_mark(*c))
                    .collect();
                assert_eq!(bare, "שהחינו", "{pointing:?}");
            }
        }
    }

    #[test]
    fn overlapping_matches_are_all_counted() {
        let sefer = crate::pretend::sefer("s", "ספר", &["סימן", "סעיף"], &["אאאא"]);
        let hits = find(
            &sefer,
            "אא",
            Pointing::Full,
            Shemos::AsWritten,
            CiteStyle::HebrewFull,
        );
        assert_eq!(hits.total, 3);
    }

    #[test]
    fn the_engines_hits_come_back_in_reading_order_and_each_place_once() {
        // The two things `where_marked` does that a `map` would not.
        //
        // **Order.** The engine answers by rank, which is right for a list you
        // read and wrong for a bar you walk down a page with — a reader
        // pressing ↓ expects the next one *below*, not the next best.
        //
        // **Once.** Two matched words that land on the same letters — the
        // engine marks a word and its own prefix, or one word twice in one
        // se'if — would otherwise be two stops that do not move.
        let sefer = sefer();
        let id = |n: usize| sefer.segments[n].id.clone();
        let hits = vec![
            // Deliberately out of order, which is what a ranked answer is.
            (id(2), vec!["שהחינו".to_string()]),
            (id(1), vec!["שהחינו".to_string(), "שהחינו".to_string()]),
        ];
        let found = where_marked(
            &sefer,
            &hits,
            Pointing::Full,
            Shemos::AsWritten,
            CiteStyle::HebrewFull,
        );
        let at: Vec<usize> = found.places.iter().map(|p| p.at).collect();
        assert_eq!(at, [1, 2], "reading order, not rank");
        assert_eq!(found.total, 2, "the same word twice is one place");
        // And the offsets are into the drawn text, exactly as `find`'s are.
        for place in &found.places {
            let shown = Shown::of(&sefer.segments[place.at].text, Pointing::Full);
            let words: String = shown
                .text()
                .chars()
                .skip(place.from)
                .take(place.to - place.from)
                .collect();
            let bare: String = words
                .chars()
                .filter(|c| !girsa_hebrew::is_mark(*c))
                .collect();
            assert_eq!(bare, "שהחינו");
        }
    }

    #[test]
    fn a_hit_on_a_segment_this_sefer_does_not_have_is_left_out() {
        // The engine answers out of an index, and an index is built from a
        // corpus that may have moved under it. A place that is not here is
        // dropped rather than reported at position zero, which is where a
        // `unwrap_or_default` would have put it.
        let sefer = sefer();
        let elsewhere = girsa_corpus::segment::SegmentId::new(
            "s",
            vec!["99".into(), "9".into()],
            girsa_corpus::segment::Ordinal::root(99),
        );
        let found = where_marked(
            &sefer,
            &[(elsewhere, vec!["שהחינו".to_string()])],
            Pointing::Full,
            Shemos::AsWritten,
            CiteStyle::HebrewFull,
        );
        assert_eq!(found.total, 0);
    }

    #[test]
    fn a_cut_list_still_reports_how_many_there_are() {
        // The rule the whole of §9.6 turns on: a list that stops must say it
        // stopped. Here that is the difference between *there are 500* and
        // *there are 500 shown*.
        let many: Vec<String> = (0..MOST + 20).map(|_| "מילה".to_string()).collect();
        let lines: Vec<&str> = many.iter().map(String::as_str).collect();
        let sefer = crate::pretend::sefer("s", "ספר", &["סימן", "סעיף"], &lines);
        let hits = find(
            &sefer,
            "מילה",
            Pointing::Full,
            Shemos::AsWritten,
            CiteStyle::HebrewFull,
        );
        assert_eq!(hits.places.len(), MOST);
        assert_eq!(hits.total, MOST + 20);
    }
}
