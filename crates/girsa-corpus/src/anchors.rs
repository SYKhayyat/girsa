//! Sefaria's commentary anchors: mined to spans, and out of the text.
//!
//! # The defect this closes
//!
//! `spec.md` §9.5's own worked example searches `יתגבר כארי`. Here is Shulchan
//! Arukh Orach Chayim 1:1 as it sits in the corpus:
//!
//! ```text
//! יתגבר <i data-commentator="Ba'er Hetev" data-order="1"></i><i data-commentator="Sha'arei Teshuvah" data-order="1"></i>כארי לעמוד בבוקר
//! ```
//!
//! Those anchors are indexed as words. `girsa_hebrew::normalize` keeps
//! `is_ascii_alphanumeric`, so `<i data-commentator="Ba'er Hetev" data-order="1">`
//! tokenises to roughly `i · data · commentator · ba · er · hetev · data · order ·
//! 1 · i` and lands **between** `יתגבר` and `כארי` in the position list. The schema
//! indexes `WithFreqsAndPositions` precisely so phrases work, and those two
//! positions are now a dozen apart.
//!
//! Measured against the real index rather than argued:
//!
//! ```text
//! find index corpus "יתגבר כארי"           → shulchan-arukh/orach-chayim/1:1#1 present
//! find index corpus "יתגבר כארי" --phrase  → absent, 0 of 63 hits
//! ```
//!
//! So the engine tells a reader that the first line of the Shulchan Arukh does not
//! contain a phrase printed in front of them — which is the exact failure
//! `tokenizer.rs` opens by saying it exists to prevent. And §9.6's ladder cannot
//! rescue it: the ladder widens a query, it does not remove junk from an index.
//!
//! 3,850 of Shulchan Arukh Orach Chayim's 4,171 segments — **92%** — carry an anchor
//! with a Hebrew letter on each side. `mishnah-berakhot` and `bavli/berakhot` carry
//! none, so this is a heavily-commented-halacha phenomenon and not a flat tax; that
//! makes it cheaper to fix and no less severe, because Shulchan Arukh is the shelf
//! most likely to be searched for an exact phrase.
//!
//! # Mined, not deleted
//!
//! The anchor's *position* is what §8.4 span anchoring wants: it is where a
//! commentary attaches, already computed upstream and sitting in the corpus unused.
//! So this does not throw it away — it moves it from being noise in a string to
//! being the datum §8.4 asks for, and takes it out of the text on the way.
//!
//! # What is kept, deliberately
//!
//! Only **empty** `<i …>` elements carrying `data-commentator` are removed. Measured
//! over Shulchan Arukh Orach Chayim: 43,883 `<i>` pairs, every one of them empty and
//! every one carrying `data-commentator`; against 2,353 `<small>`, 684 `<b>` and 685
//! `<br>` which are real emphasis and real line breaks that display and export (W22)
//! still want. A naive `<[^>]*>` sweep would take those too.
//!
//! # The trap
//!
//! Eight anchors in that sefer read `data-commentator=Mishnah Berurah"` — **opening
//! quote missing**, all eight Mishnah Berurah. A strict attribute parser drops them
//! silently, which is the T2/T4 family: an upstream defect that a careful reader
//! mistakes for absence. The name is read up to the closing quote whether or not an
//! opening one arrived.

use serde::{Deserialize, Serialize};

/// Where a commentary attaches, as Sefaria wrote it into the text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    /// The commentator's name as the corpus spells it — `Mishnah Berurah`.
    ///
    /// Not resolved to a slug here. Which work that name is depends on the sefer it
    /// is anchored in, and guessing at it is `girsa-link`'s job with `girsa-link`'s
    /// lexicon; a wrong resolution is a wrong ref, and rule 6 applies.
    pub commentator: String,
    /// The character offset in the **cleaned** text where the anchor sat.
    ///
    /// Characters, not bytes: every consumer of a span in this project counts in
    /// characters, and `display.rs` is emphatic that nothing may work an offset out
    /// by arithmetic on the other unit.
    pub at: usize,
    /// Sefaria's `data-label` — the letter printed in the margin, `א`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Sefaria's `data-order`, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
}

/// A segment's text with its anchors taken out, and the anchors.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mined<'a> {
    /// The text with the anchors taken out — **borrowed** when there were none.
    ///
    /// Which is the overwhelming majority: `mishnah-berakhot` and
    /// `bavli/berakhot` carry no anchors at all, and even in Shulchan Arukh
    /// Orach Chayim, where 92% of segments have one, the other 8% do not. This
    /// was a `String`, so the fast path — *this segment has no anchors, here it
    /// is unchanged* — **cloned every segment's text to say so**. A full corpus
    /// import is millions of allocate-and-drop for a value the caller already
    /// had.
    ///
    /// `Cow` and not a separate `Option`, because every caller wants the same
    /// thing: the text as it should be stored. Where it was already right,
    /// nothing is copied.
    pub text: std::borrow::Cow<'a, str>,
    pub anchors: Vec<Anchor>,
}

impl Mined<'_> {
    /// Whether anything was found. `false` means `text` is the input, untouched.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }
}

/// Take Sefaria's empty commentary anchors out of a segment, keeping their places.
///
/// Cheap and allocation-free when there is nothing to do, which is the common case:
/// two thirds of the corpus carries no anchors at all.
#[must_use]
pub fn mine(text: &str) -> Mined<'_> {
    // The fast path. `data-commentator` is the only marker worth scanning for, and a
    // segment without one needs no work at all.
    if !text.contains("data-commentator") {
        return Mined {
            text: std::borrow::Cow::Borrowed(text),
            anchors: Vec::new(),
        };
    }

    let mut out = String::with_capacity(text.len());
    let mut anchors = Vec::new();
    let mut chars = 0usize;
    let bytes = text.as_bytes();
    let mut at = 0usize;

    while at < text.len() {
        if bytes[at] == b'<' {
            if let Some((anchor, after)) = an_empty_commentary_anchor(text, at) {
                anchors.push(Anchor {
                    at: chars,
                    ..anchor
                });
                at = after;
                continue;
            }
        }
        // Not an anchor: copy one character, whatever it is.
        let ch = text[at..].chars().next().unwrap_or('\u{fffd}');
        out.push(ch);
        chars += 1;
        at += ch.len_utf8();
    }

    Mined {
        text: std::borrow::Cow::Owned(out),
        anchors,
    }
}

/// An `<i …data-commentator…></i>` pair at `at`, and the byte after it.
///
/// `None` for anything else — including an `<i>` that has content, a `<b>`, a
/// `<small>` and a `<br>`, all of which are real markup this must not touch.
fn an_empty_commentary_anchor(text: &str, at: usize) -> Option<(Anchor, usize)> {
    let rest = text.get(at..)?;
    // `<i ` or `<i\t`. `<img` must not match, which is what the boundary is for.
    let after_name = rest.strip_prefix("<i")?;
    if !after_name.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    let open_end = at + rest.find('>')? + 1;
    let tag = text.get(at..open_end)?;
    if !tag.contains("data-commentator") {
        return None;
    }
    // It must be *empty*: the closing tag immediately follows. An `<i>` with content
    // is emphasis and stays.
    let after = text.get(open_end..)?;
    let closed = after.strip_prefix("</i>")?;
    let _ = closed;

    Some((
        Anchor {
            commentator: attribute(tag, "data-commentator")?,
            at: 0, // filled in by `mine`, which knows the character offset
            label: attribute(tag, "data-label"),
            order: attribute(tag, "data-order"),
        },
        open_end + "</i>".len(),
    ))
}

/// One attribute out of a tag, tolerating a missing opening quote.
///
/// `data-commentator=Mishnah Berurah"` occurs eight times in Shulchan Arukh Orach
/// Chayim and is upstream. Reading up to the closing quote gets the name right in
/// both shapes; refusing the malformed one would drop an anchor and read, downstream,
/// as a commentator who does not comment there.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let at = tag.find(name)?;
    let after = tag.get(at + name.len()..)?.trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    let value = after.strip_prefix('"').unwrap_or(after);
    let end = value.find('"')?;
    let found = value.get(..end)?.trim();
    (!found.is_empty()).then(|| found.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// The exact text of Shulchan Arukh Orach Chayim 1:1, abridged to the finding.
    const SA_1_1: &str = "יתגבר <i data-commentator=\"Ba'er Hetev\" data-order=\"1\"></i>\
<i data-commentator=\"Sha'arei Teshuvah\" data-order=\"1\"></i>כארי לעמוד בבוקר";

    #[test]
    fn the_phrase_spec_9_5_searches_for_becomes_contiguous() {
        // The whole finding, in one assertion.
        assert!(
            !SA_1_1.contains("יתגבר כארי"),
            "the fixture is the real defect"
        );
        let mined = mine(SA_1_1);
        assert!(
            mined.text.contains("יתגבר כארי"),
            "still not contiguous: {}",
            mined.text
        );
        assert_eq!(mined.text, "יתגבר כארי לעמוד בבוקר");
    }

    #[test]
    fn the_anchor_keeps_its_place_rather_than_being_thrown_away() {
        let mined = mine(SA_1_1);
        assert_eq!(mined.anchors.len(), 2);
        // Both sat between `יתגבר ` and `כארי` — character 6 of the cleaned text,
        // which is exactly the span offset §8.4 wants.
        assert_eq!(mined.anchors[0].at, 6);
        assert_eq!(mined.anchors[1].at, 6);
        assert_eq!(mined.anchors[0].commentator, "Ba'er Hetev");
        assert_eq!(mined.anchors[1].commentator, "Sha'arei Teshuvah");
        assert_eq!(mined.anchors[0].order.as_deref(), Some("1"));
        assert_eq!(mined.anchors[0].label, None);
        // And the offset is a character offset, so it indexes the cleaned text.
        let at = mined.anchors[0].at;
        assert_eq!(mined.text.chars().nth(at), Some('כ'));
    }

    #[test]
    fn a_label_is_kept_because_it_is_what_is_printed_in_the_margin() {
        let mined =
            mine("א<i data-commentator=\"Be'er HaGolah\" data-label=\"ב\" data-order=\"2\"></i>ב");
        assert_eq!(mined.text, "אב");
        assert_eq!(mined.anchors[0].label.as_deref(), Some("ב"));
        assert_eq!(mined.anchors[0].order.as_deref(), Some("2"));
    }

    /// Eight of these are in the corpus, all Mishnah Berurah, all upstream.
    #[test]
    fn an_anchor_with_its_opening_quote_missing_is_still_read() {
        let mined = mine("א<i data-commentator=Mishnah Berurah\" data-label=\"א\"></i>ב");
        assert_eq!(mined.text, "אב");
        assert_eq!(mined.anchors.len(), 1, "the malformed anchor was dropped");
        assert_eq!(mined.anchors[0].commentator, "Mishnah Berurah");
        assert_eq!(mined.anchors[0].label.as_deref(), Some("א"));
    }

    #[test]
    fn real_markup_survives() {
        // 2,353 `<small>`, 684 `<b>` and 685 `<br>` in that one sefer are emphasis and
        // line breaks that display and export still want. A `<[^>]*>` sweep takes them.
        for kept in [
            "<b>חוץ מב\"ה. </b> ובטור",
            "<small>הגהה</small> טקסט",
            "שורה<br>שורה",
            "<i>נטוי אמיתי</i> טקסט",
        ] {
            let mined = mine(kept);
            assert_eq!(mined.text, kept, "markup was eaten: {kept}");
            assert!(mined.anchors.is_empty());
        }
        // And an `<i>` with content *and* a data-commentator is still content.
        let odd = "<i data-commentator=\"X\">יש כאן טקסט</i>";
        assert_eq!(mine(odd).text, odd);
    }

    #[test]
    fn a_segment_with_no_anchors_is_returned_unchanged() {
        // Two thirds of the corpus. `mishnah-berakhot` and `bavli/berakhot` are 0%.
        for plain in ["מאימתי קורין את שמע בערבית", "", "שורה\nשורה"]
        {
            let mined = mine(plain);
            assert_eq!(mined.text, plain);
            assert!(mined.is_empty());
        }
    }

    #[test]
    fn offsets_are_characters_and_not_bytes() {
        // Every Hebrew letter is two bytes and a pointed one is four. An offset in
        // bytes would land inside a letter.
        let text = "בְּרֵאשִׁית<i data-commentator=\"X\"></i>בָּרָא";
        let mined = mine(text);
        let at = mined.anchors[0].at;
        assert_eq!(at, "בְּרֵאשִׁית".chars().count());
        assert_eq!(mined.text.chars().nth(at), Some('ב'));
        // And nothing was lost.
        assert_eq!(mined.text, "בְּרֵאשִׁיתבָּרָא");
    }

    #[test]
    fn several_anchors_in_a_row_each_keep_their_own_place() {
        let text = "א<i data-commentator=\"X\"></i>ב<i data-commentator=\"Y\"></i>ג";
        let mined = mine(text);
        assert_eq!(mined.text, "אבג");
        assert_eq!(mined.anchors.len(), 2);
        assert_eq!(mined.anchors[0].at, 1);
        assert_eq!(mined.anchors[1].at, 2);
    }

    #[test]
    fn an_img_is_not_an_i() {
        let text = "<img src=\"x\" data-commentator=\"X\">";
        assert_eq!(
            mine(text).text,
            text,
            "the tag name boundary was not checked"
        );
    }
}
