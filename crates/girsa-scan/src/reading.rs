//! What is written on a page of a scan, and where on the page it is written.
//!
//! spec.md §6.3 and §9.7, BUILDER.md W26. A scan arrives with pages and no
//! words: the importer gives a dropped PDF one segment per page and refuses to
//! guess at Hebrew it cannot read (`girsa_corpus::import::mine`). This module is
//! what fills those pages in — and the shape it takes is decided by one
//! sentence of the spec:
//!
//! > **The image stays ground truth**, which makes fixing OCR errors safe by
//! > construction — the original is always right there to check against.
//!
//! # A page's words are an index and a highlight, not a text
//!
//! This is the distinction the whole module turns on. Nothing here is ever
//! shown to a reader as *what the sefer says*: the reader is looking at the
//! photograph, which is what the sefer says. What a [`Read`] is for is
//! answering *which pages contain these words* and *where on this page are
//! they* — a search index and a rectangle.
//!
//! That is why a [`Read`] is stored in the personal layer rather than written
//! into the sefer's segments, and it is why re-reading a page with a better
//! engine is a cheap, safe act rather than a migration. Nothing is anchored to
//! the OCR's opinion.
//!
//! # An address is a rectangle on the image, not an offset in the text
//!
//! Every word here carries an [`Area`] — where its ink is, in fractions of the
//! page. Fractions and not pixels, because pixels are a fact about the
//! resolution somebody rendered at: the same word is at x=840 in a 300-dpi
//! raster and x=420 in a 150-dpi one, and a highlight stored in pixels lands in
//! the margin the first time the reader zooms. The ink does not move.
//!
//! And it is the ink that a correction is anchored to. Re-read a page with a
//! different engine and you get a different number of words, in a different
//! order, spelled differently — so every character offset into the old reading
//! now points at something else, silently, which is BUILDER.md T1 for the third
//! time. An [`Area`] survives it, because the engine's opinion changed and the
//! photograph did not. `tests/the_image_is_ground_truth.rs` is that property,
//! and it fails on an offset-anchored implementation.
//!
//! # Where the words come from
//!
//! Two readers, one type ([`Reader`]), because a result row has to be able to
//! say which one it came from — they are not close in quality:
//!
//! | source | measured on a real sefer |
//! |---|---|
//! | [`Reader::Embedded`] — the PDF's own text layer | exact where it exists |
//! | [`Reader::Ocr`] — tesseract on modern square print | 99% of the words |
//! | [`Reader::Ocr`] — tesseract on Rashi script with nikud | 27%, and 4 invented words for every 1 found |
//!
//! See [`crate::engine`] for the run those numbers come from and what they
//! decided. spec.md §9.7's *"scanned hits carry a badge"* is not a nicety; it
//! is the difference between those rows.

use serde::{Deserialize, Serialize};

/// A rectangle on a page, in fractions of the page: `0.0` is the left edge and
/// the top edge, `1.0` is the right edge and the bottom.
///
/// **Top-down**, like a canvas and unlike a PDF, because the only thing that
/// ever draws one of these is the window. A PDF's own coordinates count up from
/// the bottom of the page and the caller that reads them converts once, here,
/// rather than every consumer converting and one of them forgetting.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Area {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Area {
    /// A rectangle, with the edges put the right way round.
    ///
    /// Swapped edges are not an error worth refusing — a caller converting from
    /// a bottom-up coordinate system gets them the wrong way round exactly
    /// once — but a rectangle with a negative width overlaps nothing, so a
    /// correction anchored to one would quietly never relocate.
    #[must_use]
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left: left.min(right),
            top: top.min(bottom),
            right: left.max(right),
            bottom: top.max(bottom),
        }
    }

    /// The smallest rectangle containing both.
    #[must_use]
    pub fn with(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    #[must_use]
    pub fn width(self) -> f32 {
        (self.right - self.left).max(0.0)
    }

    #[must_use]
    pub fn height(self) -> f32 {
        (self.bottom - self.top).max(0.0)
    }

    #[must_use]
    pub fn area(self) -> f32 {
        self.width() * self.height()
    }

    /// How much of *this* rectangle the other one covers, 0.0 to 1.0.
    ///
    /// Asymmetric on purpose. Relocating a correction asks *how much of what I
    /// marked is under this word*, and a word twice the size of the mark still
    /// covers all of it.
    #[must_use]
    pub fn covered_by(self, other: Self) -> f32 {
        let width = (self.right.min(other.right) - self.left.max(other.left)).max(0.0);
        let height = (self.bottom.min(other.bottom) - self.top.max(other.top)).max(0.0);
        if self.area() <= f32::EPSILON {
            return 0.0;
        }
        width * height / self.area()
    }
}

/// One glyph, as a PDF's text layer hands it over: what it draws and where.
///
/// The input to [`group`]. A page of the sefer this was built against arrives
/// as about 1,400 of these, one per letter and one per nikud mark, because the
/// font positions every glyph itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    pub text: String,
    pub at: Area,
}

/// One word, and where its ink is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    pub text: String,
    pub at: Area,
    /// How sure the engine was, 0.0 to 1.0. `1.0` from a text layer, which is
    /// not reading anything — it is being told.
    pub confidence: f32,
}

/// Who worked out what is on the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reader {
    /// The file said so. A PDF that was typeset rather than photographed
    /// carries its own text, and asking it is exact, instant, needs no model
    /// and cannot invent a word.
    Embedded,
    /// An OCR engine looked at the picture, and this is which one — the name
    /// and the version, because a page read by tesseract 5.4 and one read by
    /// whatever replaces it are not the same claim, and `git blame` does not
    /// work on somebody's personal layer.
    Ocr { engine: String },
}

impl Reader {
    /// Whether this reading is a machine's opinion about a picture.
    ///
    /// What the badge of spec.md §9.7 is drawn from. **Badge them, don't demote
    /// them** — the row ranks where it ranks and says where it came from.
    #[must_use]
    pub fn is_ocr(&self) -> bool {
        matches!(self, Self::Ocr { .. })
    }

    /// The word that goes in the index and on the badge.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Embedded => "embedded",
            Self::Ocr { engine } => engine,
        }
    }

    /// Read one back out of the index.
    #[must_use]
    pub fn named(name: &str) -> Self {
        if name == "embedded" {
            Self::Embedded
        } else {
            Self::Ocr {
                engine: name.to_string(),
            }
        }
    }
}

/// A page of a scan, read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Read {
    /// Which page of the **file**. Not which daf — that is [`crate::paging`],
    /// it is a declaration the reader makes and re-makes, and a reading may not
    /// move when it changes.
    pub page: usize,
    pub by: Reader,
    pub words: Vec<Word>,
}

impl Read {
    #[must_use]
    pub fn new(page: usize, by: Reader, words: Vec<Word>) -> Self {
        Self { page, by, words }
    }

    /// The words, in reading order, separated by single spaces. What goes into
    /// the search index as this page's text.
    ///
    /// The line breaks are gone and so is every other thing about the page's
    /// shape, because the page's shape is in the photograph and this is an
    /// index. What it must preserve is **order and adjacency**: spec.md §9.3's
    /// *these words within X words of each other* is a question about this
    /// string, and a reading that joined the columns of a daf in the wrong
    /// order would answer it wrongly.
    #[must_use]
    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Whether anything was found on this page.
    ///
    /// A page can be read and turn out to be blank — a verso, a plate — and
    /// that is a different state from a page nobody has read, which is why the
    /// store keeps the empty reading rather than leaving the page out.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Where on the page the words a search asked for are, as rectangles to
    /// draw over the image.
    ///
    /// The predicate is the caller's, and in the app it is the search's own —
    /// `Plan::matches` through the normalizer, the same rule `Hit::marks` uses
    /// on a text sefer. Passing the raw query string in here instead would
    /// highlight nothing on a menukad page, which is most of them.
    #[must_use]
    pub fn marks(&self, matches: impl Fn(&str) -> bool) -> Vec<Area> {
        self.words
            .iter()
            .filter(|w| matches(&w.text))
            .map(|w| w.at)
            .collect()
    }

    /// The word whose ink is under this rectangle, if one is.
    ///
    /// **The most covered, and nothing when nothing is covered.** Not the
    /// nearest — the same refusal to round that [`crate::Scan::page_of`] makes
    /// about a daf the scan does not carry. A correction that relocated onto
    /// the nearest word would move a reader's fix onto a word they never
    /// looked at, and it would look exactly like one that landed right.
    #[must_use]
    pub fn covering(&self, area: Area) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        let mut tied = false;
        for (at, word) in self.words.iter().enumerate() {
            let covered = area.covered_by(word.at);
            if covered <= MOST_OF_IT {
                continue;
            }
            match best {
                Some((_, so_far)) if (covered - so_far).abs() < f32::EPSILON => tied = true,
                Some((_, so_far)) if covered <= so_far => {}
                _ => {
                    best = Some((at, covered));
                    tied = false;
                }
            }
        }
        // Two words with the same box — a line the file handed over whole, so
        // it said which words are on it and not where they are. Refused rather
        // than resolved to the first one: a correction on the wrong half of a
        // line looks exactly like a correction on the right half, which is the
        // failure W24 built its span anchoring to avoid.
        if tied {
            return None;
        }
        best.map(|(at, _)| at)
    }
}

/// How much of a mark has to be under a word before that word is what was
/// marked.
///
/// Half. Two engines box the same word slightly differently — tesseract's boxes
/// are drawn round the ink and a text layer's round the glyph cell, which
/// differ by the ascender — so exact containment finds nothing, and a low bar
/// makes the word beside it a candidate too. `covering` takes the best of what
/// clears the bar, so this is a floor and not a decision.
const MOST_OF_IT: f32 = 0.5;

/// A correction to a word on a scan, anchored to the ink.
///
/// # Why not a character offset
///
/// Because a re-read renumbers every offset on the page. Corrections to a text
/// sefer are `segment id + character span` (`girsa_fix::Patch`, W20) and that is
/// right there: the base text is a file that does not change under them. A
/// scan's words are not a base text, they are an engine's current opinion, and
/// the whole point of W26 is that a better engine can replace them tomorrow.
///
/// So the durable part of a correction here is [`Fix::at`] — a rectangle on the
/// photograph — and [`relocate`] is what re-finds the word under it. The
/// property this buys is the one spec.md §6.3 claims: *the image stays ground
/// truth, which makes fixing OCR errors safe by construction*.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fix {
    /// The ink this is about. Written down when the correction is made and
    /// never recomputed.
    pub at: Area,
    /// What the engine read there, as it read it. Kept for the same reason
    /// `girsa_fix::Patch` keeps `was`: the rectangle says *where* and this says
    /// *what*, and a reader reviewing their own corrections a year later needs
    /// to see what they were correcting.
    pub was: String,
    /// What is actually printed there.
    pub says: String,
}

/// Apply a page's corrections to a reading of it.
///
/// Every fix is re-found by its ink, so this is the same answer before and
/// after a re-read with a different engine. A fix whose ink no reading covers
/// is **returned unapplied** rather than dropped: the reader marked something
/// and the new engine put no word there at all, and a correction that silently
/// vanished is a correction the reader will make again next year.
#[must_use]
pub fn corrected(read: &Read, fixes: &[Fix]) -> (Read, Vec<Fix>) {
    let mut words = read.words.clone();
    let mut lost = Vec::new();
    for fix in fixes {
        match read.covering(fix.at) {
            Some(at) => words[at].text.clone_from(&fix.says),
            None => lost.push(fix.clone()),
        }
    }
    (Read::new(read.page, read.by.clone(), words), lost)
}

/// How far apart two glyphs can be and still be one word, as a fraction of how
/// tall they are.
///
/// **Measured, not chosen.** Over five pages of a real sefer — 5,500 gaps
/// between adjacent glyphs — the distribution has a valley here: 3,759 gaps
/// fall between 0.05 and 0.20 of the glyph height and are inside words, 1,300
/// fall above 0.35 and are between words, and 39 land in between. The
/// reproduction is in the commit message.
const ONE_WORD: f32 = 0.28;

/// How much two glyphs have to overlap vertically to be on one line.
const ONE_LINE: f32 = 0.35;

/// Turn a page of glyphs into words.
///
/// # Why a PDF's own text needs this at all
///
/// Because a PDF does not have words. It has drawing instructions, and a Hebrew
/// sefer typeset properly positions **every letter and every nikud mark
/// separately** so the marks sit where the typesetter wanted them. Ask such a
/// file what its text is and it answers
/// `ֵמ ֵא יָמ ַת י` — spaces between the halves of every letter — because the
/// extractor put a space wherever the pen jumped. Half of those jumps are
/// inside a word.
///
/// So the words are worked out from the geometry: glyphs on one line, in
/// reading order, cut wherever the gap is wider than [`ONE_WORD`]. The spaces
/// the file itself supplies are ignored entirely, which is what makes this the
/// same code for a text layer and for an engine that hands back loose glyphs.
///
/// # Reading order
///
/// A line is ordered right to left, because that is the order the words are
/// read in and [`Read::text`] has to preserve adjacency. Inside a word it is
/// the other way round when the word is not Hebrew: a footnote marker set as
/// three separate digits is `876` and not `678`.
/// Whether a character is something a font actually drew, or the file failing
/// to say what it drew.
///
/// A PDF maps the codes in its content stream to Unicode through the font's
/// `ToUnicode` table, and a Hebrew font that positions its own nikud very often
/// has no entry for the mark glyphs — so they come back as control codes,
/// `U+000E`, `U+0010`. That is the encoding trap `girsa_corpus::import::mine`
/// refuses to walk into when it declines to read a PDF's text into a sefer.
///
/// Here they are dropped, and dropping them costs nothing: they are the nikud,
/// and the index strips nikud in every mode (spec.md §9.1). What would not be
/// free is a font whose *letters* are unmapped, which is why [`unmapped`]
/// counts them for a caller that has to decide whether to trust the page.
fn is_drawn(c: char) -> bool {
    !c.is_control() && !('\u{E000}'..='\u{F8FF}').contains(&c) && c != '\u{FFFD}'
}

/// How many of a page's code points the file declined to name.
///
/// The check a caller makes before believing a text layer. A handful is the
/// nikud and is expected; most of the page is a font whose `ToUnicode` table is
/// missing, and then the letters that *did* map cannot be trusted either — the
/// page wants OCR, which at least looks at the ink.
#[must_use]
pub fn unmapped(glyphs: &[Glyph]) -> usize {
    glyphs
        .iter()
        .flat_map(|g| g.text.chars())
        .filter(|c| !is_drawn(*c))
        .count()
}

/// What came off a page, and what would not come off it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Grouped {
    pub words: Vec<Word>,
    /// Words thrown away because the file would not say what one of their
    /// letters was.
    ///
    /// **Not a count of imperfections — a count of refusals**, and the
    /// difference matters. A word with a letter missing from it is a different
    /// word: `מֵאֵימָתַי` with its aleph unnamed comes out `מאימתי` minus a
    /// letter, which is a string that will be found by a search for something
    /// that is not printed on the page. A gap is recoverable and a wrong hit is
    /// not, so the word is dropped and counted (BUILDER.md rule 6).
    pub refused: usize,
}

#[must_use]
pub fn group(glyphs: &[Glyph]) -> Grouped {
    // A glyph the file names in part keeps its letters and is **marked**: a run
    // holding it is a run whose word this file cannot spell.
    let drawn: Vec<Mark> = glyphs
        .iter()
        .filter_map(|glyph| {
            let text: String = glyph.text.chars().filter(|c| is_drawn(*c)).collect();
            // A mark the file would not name, drawn on its own, disappears
            // without condemning anything: it is the nikud, the index strips
            // nikud in every mode (spec.md §9.1), and refusing a line over
            // every one of them would leave a vocalized page empty.
            (!text.trim().is_empty()).then(|| Mark {
                unnamed: glyph.text.chars().any(|c| !is_drawn(c)),
                text,
                at: glyph.at,
            })
        })
        .collect();

    let mut lines: Vec<Vec<Mark>> = Vec::new();
    for glyph in drawn {
        match lines.iter_mut().find(|line| shares_a_line(line, &glyph)) {
            Some(line) => line.push(glyph),
            None => lines.push(vec![glyph]),
        }
    }
    // Down the page, then right to left across each line.
    lines.sort_by(|a, b| top_of(a).total_cmp(&top_of(b)));

    let mut grouped = Grouped::default();
    for line in lines {
        // A line holding a letter the file would not name is refused whole.
        // Not squeamishness: the unnamed glyph is *drawn*, it takes up room,
        // and the words either side of it are cut where its ink is not. On the
        // sefer this was built against those lines come out
        // `יַת5? ים דִס ֹוף` — fragments of real words, which is what a search
        // index must never be given (BUILDER.md rule 6). The rest of the page
        // is read normally, and on that sefer the rest of the page is most of
        // it.
        let unspellable = line.iter().any(|glyph| glyph.unnamed);
        let mut of_line = Grouped::default();
        let mut line = squared_up(line);
        line.sort_by(|a, b| b.at.right.total_cmp(&a.at.right));
        let mut run: Vec<Mark> = Vec::new();
        for glyph in line {
            let broken = run.last().is_some_and(|previous| {
                let gap = previous.at.left - glyph.at.right;
                let height = previous
                    .at
                    .height()
                    .max(glyph.at.height())
                    .max(f32::EPSILON);
                gap / height > ONE_WORD
            });
            if broken {
                assemble(&run, &mut of_line);
                run.clear();
            }
            run.push(glyph);
        }
        assemble(&run, &mut of_line);

        if unspellable {
            grouped.refused += of_line.words.len();
        } else {
            grouped.words.extend(of_line.words);
        }
    }
    grouped
}

/// A glyph on its way through [`group`], carrying whether the file managed to
/// say what it was.
#[derive(Debug, Clone)]
struct Mark {
    text: String,
    at: Area,
    unnamed: bool,
}

/// Give the glyphs on a line that were reported with no width the width of the
/// line's other glyphs.
///
/// A font that positions its own nikud draws some glyphs with **no advance** —
/// the mark is placed by the instruction that follows it — and a PDF's text
/// layer reports those as zero-width. A zero-width box neither overlaps its
/// neighbours nor sits a measurable gap from them, so a line with a dozen of
/// them cuts into a dozen extra words. They are given the median width of the
/// glyphs on their own line, which is a letter of the size that line is set in.
///
/// Not a guess about what is drawn — the letter and its position are what the
/// file said. It is a guess about how wide it is, made only where the file
/// declined to say, and only to decide where the word breaks are.
fn squared_up(line: Vec<Mark>) -> Vec<Mark> {
    let mut widths: Vec<f32> = line
        .iter()
        .map(|g| g.at.width())
        .filter(|w| *w > f32::EPSILON)
        .collect();
    if widths.is_empty() {
        return line;
    }
    widths.sort_by(f32::total_cmp);
    let median = widths[widths.len() / 2];
    line.into_iter()
        .map(|glyph| {
            if glyph.at.width() > f32::EPSILON {
                return glyph;
            }
            Mark {
                at: Area::new(
                    glyph.at.left,
                    glyph.at.top,
                    glyph.at.left + median,
                    glyph.at.bottom,
                ),
                ..glyph
            }
        })
        .collect()
}

/// Whether a glyph belongs to a line already started: its ink overlaps that
/// line's vertically by more than [`ONE_LINE`] of the shorter of the two.
fn shares_a_line(line: &[Mark], glyph: &Mark) -> bool {
    let Some(first) = line.first() else {
        return false;
    };
    let overlap = (first.at.bottom.min(glyph.at.bottom) - first.at.top.max(glyph.at.top)).max(0.0);
    let shorter = first.at.height().min(glyph.at.height());
    shorter > f32::EPSILON && overlap / shorter > ONE_LINE
}

fn top_of(line: &[Mark]) -> f32 {
    line.iter().map(|g| g.at.top).fold(f32::MAX, f32::min)
}

/// One run of glyphs, in reading order, as words — or as a refusal.
///
/// # Why this can be more than one word
///
/// Because a file does not always hand its text over a glyph at a time. The
/// same PDF gives a vocalized page as 707 separately-positioned glyphs and an
/// unvocalized one as 35 items, each of them a whole line with its spaces in
/// it — and on that page the file has said *which* words are on the line and
/// not *where* they are.
///
/// So the line is split into its words, which is what the index needs, and
/// **every one of them carries the line's rectangle**, which is what is
/// actually known. Apportioning the box across the letters would put a word
/// break wherever the arithmetic fell — Hebrew letters run from a yud to a
/// shin in width — and a highlight two letters off looks exactly like one that
/// landed right. That is the refusal W24 made about a dibur hamatchil, made
/// again about a rectangle.
///
fn assemble(run: &[Mark], into: &mut Grouped) {
    let Some((first, rest)) = run.split_first() else {
        return;
    };
    let at = rest.iter().fold(first.at, |so_far, g| so_far.with(g.at));
    let mut ordered: Vec<&Mark> = run.iter().collect();
    // The run arrived right to left. A word with no Hebrew letter in it is a
    // number or a Latin word and reads the other way.
    if !run.iter().any(|g| g.text.chars().any(is_hebrew_letter)) {
        ordered.sort_by(|a, b| a.at.left.total_cmp(&b.at.left));
    }
    let text: String = ordered.iter().map(|g| g.text.as_str()).collect();
    into.words.extend(text.split_whitespace().map(|word| Word {
        text: word.to_string(),
        at,
        confidence: 1.0,
    }));
}

fn is_hebrew_letter(c: char) -> bool {
    ('\u{05D0}'..='\u{05EA}').contains(&c)
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// A glyph on a notional line, sized like a letter in a book.
    fn glyph(text: &str, left: f32, width: f32) -> Glyph {
        Glyph {
            text: text.to_string(),
            at: Area::new(left, 0.10, left + width, 0.12),
        }
    }

    #[test]
    fn the_spaces_a_pdf_supplies_are_not_word_breaks_and_the_gaps_are() {
        // `מאימתי קורין` as the real file draws it: every letter its own glyph,
        // a two-thousandth of a page between the letters of a word and a
        // hundredth between the words. The letters run right to left.
        let mut glyphs = Vec::new();
        let mut x = 0.500;
        for letter in ["י", "ת", "מ", "י", "א", "מ"] {
            glyphs.push(glyph(letter, x, 0.008));
            x += 0.009;
        }
        x += 0.012;
        for letter in ["ן", "י", "ר", "ו", "ק"] {
            glyphs.push(glyph(letter, x, 0.008));
            x += 0.009;
        }

        let words = group(&glyphs).words;
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["קורין", "מאימתי"]
        );
    }

    #[test]
    fn a_number_inside_a_line_of_hebrew_keeps_its_digits_in_order() {
        // A footnote marker, set as three separate glyphs. The line reads right
        // to left and the number does not.
        let glyphs = vec![
            glyph("ד", 0.500, 0.008),
            glyph("ע", 0.509, 0.008),
            glyph("8", 0.530, 0.006),
            glyph("7", 0.536, 0.006),
            glyph("6", 0.542, 0.006),
        ];
        let words = group(&glyphs).words;
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["876", "עד"]
        );
    }

    #[test]
    fn lines_come_out_down_the_page() {
        let first = Glyph {
            text: "א".into(),
            at: Area::new(0.5, 0.10, 0.51, 0.12),
        };
        let second = Glyph {
            text: "ב".into(),
            at: Area::new(0.5, 0.20, 0.51, 0.22),
        };
        let words = group(&[second, first]).words;
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["א", "ב"]
        );
    }

    #[test]
    fn a_word_is_found_under_the_ink_and_nothing_is_found_beside_it() {
        let read = Read::new(
            3,
            Reader::Embedded,
            vec![
                Word {
                    text: "מאימתי".into(),
                    at: Area::new(0.50, 0.10, 0.56, 0.12),
                    confidence: 1.0,
                },
                Word {
                    text: "קורין".into(),
                    at: Area::new(0.44, 0.10, 0.49, 0.12),
                    confidence: 1.0,
                },
            ],
        );
        assert_eq!(read.covering(Area::new(0.51, 0.105, 0.55, 0.115)), Some(0));
        assert_eq!(read.covering(Area::new(0.45, 0.105, 0.48, 0.115)), Some(1));
        // Between the two words, over neither of them.
        assert_eq!(read.covering(Area::new(0.492, 0.105, 0.498, 0.115)), None);
        // Off the line entirely.
        assert_eq!(read.covering(Area::new(0.50, 0.80, 0.56, 0.82)), None);
    }

    #[test]
    fn the_text_of_a_reading_is_its_words_in_order() {
        let read = Read::new(
            1,
            Reader::Ocr {
                engine: "tesseract 5.4.0".into(),
            },
            vec![
                Word {
                    text: "עד".into(),
                    at: Area::new(0.50, 0.10, 0.52, 0.12),
                    confidence: 0.9,
                },
                Word {
                    text: "חצות".into(),
                    at: Area::new(0.45, 0.10, 0.49, 0.12),
                    confidence: 0.8,
                },
            ],
        );
        assert_eq!(read.text(), "עד חצות");
        assert!(read.by.is_ocr());
    }
}
