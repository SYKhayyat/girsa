//! A permanent id that names 1.2 MB of text names a volume, not a place.
//!
//! # What was measured
//!
//! A full scan of `corpus/works` — 7,189 works, 5,000,545 segments:
//!
//! ```text
//! >10,000 chars:  5,733       >50,000: 119       >200,000: 19
//! max: 1,275,307   girsa:bavli/chiddushei-harambam-on-rosh-hashanah/20b:7#32
//! works affected: 926 — including tur (68), beit-yosef (55),
//!                 akeidat-yitzchak (183), abarbanel-on-torah (70),
//!                 שות-פרי-עץ-חיים (468)
//! ```
//!
//! Those are not obscure works. The whole architecture rests on *"each record
//! carries its own id, so every anchor still names the same words"*, and three
//! things degrade together at that size: the citation is unusable as a mareh
//! makom, a highlight cannot help, and a search result is *"it is somewhere in
//! here."* It was found while chasing a search hit whose displayed text contained
//! neither typed word, because the segment was 495,726 characters long.
//!
//! It was not counted anywhere, not surfaced anywhere, and not split anywhere.
//!
//! # Why splitting does not break an anchor
//!
//! Because [`Ordinal::covers`](crate::segment::Ordinal::covers) already says so.
//! `#32` split gives `#32.1 #32.2 #32.3`, and `#32` **covers** every one of them:
//! a citation, a link, a highlight or a correction anchored to `#32` still names
//! the same words, now as a group rather than as one record. That is what the
//! ordinal scheme was designed for and it is why this can be done at all —
//! spec.md §3's *"splitting a segment mints a child ID rather than shifting
//! seventeen thousand others"*.
//!
//! # Where it cuts
//!
//! Never mid-word, never mid-character, and by preference never mid-sentence. A
//! sefer's own punctuation is tried in order — a blank line, then a `:` (which is
//! how a Hebrew sentence ends in print), then a full stop, then any whitespace —
//! and only if a window holds none of those at all does it cut at the nearest
//! character boundary, which is a run of 3,000 characters with no space in it and
//! is not text.

use std::collections::BTreeMap;

/// Above this, an id stops naming a place.
///
/// Ten thousand characters is roughly three pages of a printed sefer. Below it a
/// citation is a mareh makom somebody can follow; above it the reader is being
/// told the words are *somewhere in here*. The number is the same one the audit
/// counted against, so the tally in a report and the threshold in the importer
/// are one number and cannot drift.
pub const NAMES_A_PLACE: usize = 10_000;

/// What to aim for when cutting one up.
///
/// Deliberately well under [`NAMES_A_PLACE`], so a split segment is comfortably a
/// place rather than sitting on the line — and so the cut can move to a sentence
/// end without crossing back over the threshold.
pub const TARGET: usize = 4_000;

/// How far from the target a better cut is worth taking.
const REACH: usize = 1_200;

/// How many oversized segments there are, and how bad the worst of them is.
///
/// The link table's six lines sum exactly to 5,108,893 in `girsa-link-import`'s
/// report; this is the same standard applied to the thing that had no number at
/// all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tally {
    /// Over [`NAMES_A_PLACE`].
    pub over: usize,
    /// Over five times it.
    pub well_over: usize,
    /// Over twenty times it — a segment that is a whole sefer.
    pub a_volume: usize,
    /// The longest one, in characters, and what it is called.
    pub largest: usize,
    pub largest_id: String,
    /// How many were cut up, and into how many children.
    pub split: usize,
    pub children: usize,
    /// Works with at least one oversized segment in them.
    works: std::collections::BTreeSet<String>,
}

impl Tally {
    /// Note one segment.
    pub fn saw(&mut self, id: &str, characters: usize, work: &str) {
        if characters <= NAMES_A_PLACE {
            return;
        }
        self.over += 1;
        if characters > NAMES_A_PLACE * 5 {
            self.well_over += 1;
        }
        if characters > NAMES_A_PLACE * 20 {
            self.a_volume += 1;
        }
        if characters > self.largest {
            self.largest = characters;
            self.largest_id = id.to_string();
        }
        self.works.insert(work.to_string());
    }

    /// Note that one was cut up.
    pub fn cut(&mut self, into: usize) {
        self.split += 1;
        self.children += into;
    }

    /// Take another tally into this one — the import runs one per work, in parallel.
    pub fn absorb(&mut self, other: &Self) {
        self.over += other.over;
        self.well_over += other.well_over;
        self.a_volume += other.a_volume;
        self.split += other.split;
        self.children += other.children;
        if other.largest > self.largest {
            self.largest = other.largest;
            self.largest_id = other.largest_id.clone();
        }
        self.works.extend(other.works.iter().cloned());
    }

    #[must_use]
    pub fn works(&self) -> usize {
        self.works.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.over == 0
    }

    /// The lines a report prints, or nothing at all when there is nothing to say.
    ///
    /// One implementation, so the importer's tally, a search result's note and a
    /// test's expectation cannot disagree about a count.
    #[must_use]
    pub fn said(&self) -> Vec<String> {
        if self.is_empty() {
            return Vec::new();
        }
        let mut out = vec![
            format!(
                "  over {NAMES_A_PLACE} chars  {} segments in {} works",
                self.over,
                self.works()
            ),
            format!("  over {}          {}", NAMES_A_PLACE * 5, self.well_over),
            format!("  over {}         {}", NAMES_A_PLACE * 20, self.a_volume),
            format!(
                "  largest            {} — {}",
                self.largest, self.largest_id
            ),
        ];
        if self.split > 0 {
            out.push(format!(
                "  split              {} into {} children (anchors on the parent still \
                 name the same words — see Ordinal::covers)",
                self.split, self.children
            ));
        } else {
            out.push(
                "  split              none — a permanent id naming that much text names a \
                 volume, not a place"
                    .to_string(),
            );
        }
        out
    }
}

/// Where to cut a long segment, as byte offsets, ascending.
///
/// Empty when the text does not need cutting. Every offset is a character
/// boundary, and by preference the end of a sentence.
///
/// **Counted in characters, cut in bytes.** The threshold has to be characters
/// because that is what the audit counted and what a reader perceives — a Hebrew
/// letter is two bytes and a pointed one is four, so a byte threshold would cut a
/// menukad Chumash at a quarter of the length of an unpointed Shas and neither
/// number would mean anything.
#[must_use]
pub fn cuts(text: &str, target: usize) -> Vec<usize> {
    // The guard **before** the index, not after it.
    //
    // 4,994,812 of the corpus's 5,000,545 segments are under the threshold, and
    // this used to build an 8-bytes-per-character index of every one of them
    // and *then* ask whether it needed to — roughly 6 GB of allocate-and-drop
    // across an import, a measurable slice of the hour.
    //
    // `take(NAMES_A_PLACE + 1)` is what makes this cheap and correct at once: it
    // answers *is this longer than the threshold* without counting the
    // 1,275,307 characters of the longest segment in the corpus to find out.
    if target == 0 || text.chars().take(NAMES_A_PLACE + 1).count() <= NAMES_A_PLACE {
        return Vec::new();
    }

    // Character index → byte offset, once. The whole function then works in
    // character positions, which is the unit the decisions are about.
    let at_byte: Vec<usize> = text
        .char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(text.len()))
        .collect();
    let characters = at_byte.len() - 1;

    let mut out = Vec::new();
    let mut from = 0usize; // in characters
    while characters - from > NAMES_A_PLACE {
        let want = from + target;
        if want >= characters {
            break;
        }
        let cut = best_cut(text, &at_byte, from, want);
        // No progress means no cut is possible; stop rather than loop.
        if cut <= from || cut >= characters {
            break;
        }
        out.push(at_byte[cut]);
        from = cut;
    }
    out
}

/// The best character position to cut at near `want`, never at or before `after`.
fn best_cut(text: &str, at_byte: &[usize], after: usize, want: usize) -> usize {
    let characters = at_byte.len() - 1;
    let low = want.saturating_sub(REACH).max(after + 1);
    let high = (want + REACH).min(characters);
    if low >= high {
        return want;
    }
    let window = text.get(at_byte[low]..at_byte[high]).unwrap_or("");

    // A sefer's own punctuation, in order of how much it means. `׃` is the sof
    // pasuk; `:` is how a Hebrew sentence ends in print and is far more common in
    // this corpus than a full stop. The cut goes *after* the mark, so the sentence
    // it ends stays with the piece it belongs to.
    for pattern in ["\n\n", "\n", "׃", ":", ".", "! ", "? ", " "] {
        if let Some(found) = nearest(window, pattern, at_byte[want] - at_byte[low]) {
            let byte = at_byte[low] + found + pattern.len();
            let cut = at_byte.partition_point(|b| *b < byte);
            if cut > after && cut < characters {
                return cut;
            }
        }
    }
    // A window of a couple of thousand characters with no space in it is not text.
    want
}

/// The occurrence of `pattern` in `window` closest to `about`, in bytes.
fn nearest(window: &str, pattern: &str, about: usize) -> Option<usize> {
    window
        .match_indices(pattern)
        .map(|(at, _)| at)
        .min_by_key(|at| at.abs_diff(about))
}

/// Cut one text into pieces at [`cuts`].
///
/// One piece — the text itself — when it does not need cutting, so a caller can
/// use this unconditionally.
#[must_use]
pub fn pieces(text: &str, target: usize) -> Vec<&str> {
    let at = cuts(text, target);
    if at.is_empty() {
        return vec![text];
    }
    let mut out = Vec::with_capacity(at.len() + 1);
    let mut from = 0;
    for cut in at {
        out.push(text.get(from..cut).unwrap_or(""));
        from = cut;
    }
    out.push(text.get(from..).unwrap_or(""));
    out
}

/// How many characters each of a work's segments has, for a report.
///
/// Keyed by id so a caller can name the largest rather than only count it.
#[must_use]
pub fn measure<'a>(segments: impl Iterator<Item = (&'a str, &'a str)>) -> BTreeMap<String, usize> {
    segments
        .map(|(id, text)| (id.to_string(), text.chars().count()))
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_segment_that_names_a_place_is_left_alone() {
        assert!(cuts("קצר", TARGET).is_empty());
        // Exactly at the threshold, in characters — two per repeat.
        let three_pages = "א ".repeat(NAMES_A_PLACE / 2);
        assert_eq!(three_pages.chars().count(), NAMES_A_PLACE);
        assert!(
            cuts(&three_pages, TARGET).is_empty(),
            "at the threshold, nothing is cut"
        );
        // One character over, and it is cut — the threshold is a threshold.
        let over = format!("{three_pages}א");
        assert!(!cuts(&over, TARGET).is_empty());
        assert_eq!(pieces("קצר", TARGET), vec!["קצר"]);
    }

    #[test]
    fn the_1_275_307_character_segment_becomes_places() {
        // The largest in the real corpus, to the character.
        let text = "מאימתי קורין את שמע בערבית: ".repeat(1_275_307 / 28);
        let parts = pieces(&text, TARGET);
        assert!(parts.len() > 100, "{} pieces", parts.len());
        for part in &parts {
            assert!(
                part.chars().count() <= NAMES_A_PLACE,
                "a piece is still {} characters",
                part.chars().count()
            );
        }
        // Nothing is lost and nothing is duplicated.
        assert_eq!(parts.concat(), text);
    }

    #[test]
    fn it_cuts_where_a_sentence_ends() {
        // `:` is how a Hebrew sentence ends in print, and it is what this corpus
        // is full of.
        let sentence = "מאימתי קורין את שמע בערבית: ";
        let text = sentence.repeat(2_000);
        for cut in cuts(&text, TARGET) {
            let before = &text[..cut];
            assert!(
                before.ends_with(": ") || before.ends_with(':'),
                "cut at {cut} is mid-sentence: …{:?}",
                &before[before.len().saturating_sub(20)..]
            );
        }
    }

    #[test]
    fn it_prefers_a_paragraph_to_a_sentence() {
        let para = format!("{}\n\n", "מאימתי קורין את שמע בערבית: ".repeat(140));
        let text = para.repeat(20);
        let at = cuts(&text, TARGET);
        assert!(!at.is_empty());
        let on_a_paragraph = at
            .iter()
            .filter(|cut| text[..**cut].ends_with("\n\n"))
            .count();
        assert!(
            on_a_paragraph >= at.len() / 2,
            "{on_a_paragraph} of {} cuts landed on a paragraph",
            at.len()
        );
    }

    #[test]
    fn a_hebrew_letter_is_never_cut_in_half() {
        // Every letter is two bytes, so a cut taken on a byte count splits one and
        // produces a replacement character in a sefer.
        let text = "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֶת הַשָּׁמַיִם וְאֵת הָאָרֶץ".repeat(600);
        let parts = pieces(&text, TARGET);
        assert!(parts.len() > 1);
        assert_eq!(parts.concat(), text, "no byte was lost or repeated");
        for part in &parts {
            // Round-trips as text, which it cannot if a code point was split.
            assert!(std::str::from_utf8(part.as_bytes()).is_ok());
        }
    }

    #[test]
    fn a_run_with_no_punctuation_at_all_is_still_cut() {
        // Not text, and not a reason to hand back a 400 KB segment.
        let text = "א".repeat(400_000);
        let parts = pieces(&text, TARGET);
        assert!(parts.len() > 30, "{} pieces", parts.len());
        for part in &parts {
            assert!(part.chars().count() <= NAMES_A_PLACE);
        }
        assert_eq!(parts.concat(), text);
    }

    #[test]
    fn the_tally_counts_what_the_audit_counted() {
        let mut tally = Tally::default();
        tally.saw("girsa:a/1#1", 500, "a");
        tally.saw("girsa:a/1#2", 12_000, "a");
        tally.saw("girsa:b/1#1", 60_000, "b");
        tally.saw("girsa:c/1#1", 1_275_307, "c");
        assert_eq!(tally.over, 3);
        assert_eq!(tally.well_over, 2);
        assert_eq!(tally.a_volume, 1);
        assert_eq!(tally.largest, 1_275_307);
        assert_eq!(tally.largest_id, "girsa:c/1#1");
        assert_eq!(tally.works(), 3);
        assert!(!tally.is_empty());

        let said = tally.said();
        assert!(
            said.iter().any(|l| l.contains("3 segments in 3 works")),
            "{said:?}"
        );
        assert!(said.iter().any(|l| l.contains("1275307")), "{said:?}");
        // With nothing split, the report says so rather than being silent about it.
        assert!(said.iter().any(|l| l.contains("none")), "{said:?}");

        tally.cut(3);
        assert!(tally.said().iter().any(|l| l.contains("1 into 3 children")));
    }

    #[test]
    fn nothing_oversized_says_nothing() {
        let mut tally = Tally::default();
        tally.saw("girsa:a/1#1", 500, "a");
        assert!(tally.is_empty());
        assert!(tally.said().is_empty());
    }
}
