//! A scan in the reading workspace: the second reading mode.
//!
//! spec.md §6.2 and §6.3 are one decision taken twice. Text seforim get modern
//! columns and **no tzuras hadaf**, because reconstructing the traditional page
//! from a string of words is a typesetting project. A scan needs no engine at
//! all: the photograph *is* the daf, with the Rashi in its column and the
//! Tosfos in its, exactly as it was set. That is why the PDF layer is a reading
//! mode here and not a file attachment.
//!
//! # What this module is
//!
//! The join between three things that already existed:
//!
//! - a **sefer on the shelf** — `girsa-corpus` gives a dropped PDF one segment
//!   per page and a permanent id for each (W6), so a page can be noted on,
//!   highlighted and linked before anything is known about what is printed on
//!   it;
//! - a **mapping** — `girsa-scan`, which says which page is which daf;
//! - a **citation** — `girsa-cite`, the formatter both applications compile.
//!
//! Nothing here opens a PDF or draws anything. The window draws the page out of
//! the file the reader dropped; what it asks this crate is *what am I looking
//! at*, and *what do I write down*.
//!
//! # A page of a scan is cited with no words in it
//!
//! [`send`](crate::sending::send) refuses an empty selection, and it is right
//! to: a quote block with nothing in it arrives in a document looking like a
//! paste that failed. A page of an un-OCR'd scan has no words **by
//! construction** — the importer will not invent Hebrew it cannot read — and
//! yet it is citable, because the reader can see the daf. So a page is sent as
//! a **mareh makom**: the ref, the printed citation, and no quote. `girsa-ksav`
//! writes that as `#מראה_מקום(…)` alone rather than as an empty `#ציטוט[]`,
//! which is the one change this work order made in the shared crates.

use girsa_cite::{cite, CiteStyle, Sefer};
use girsa_corpus::import::{mine, SegmentKind};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::{Source, Work};
use girsa_scan::reading::Area;
use girsa_scan::Scan;
use girsa_source::SourcePacket;

use crate::sending::{about, provenance, Sent};
use crate::shelf::{Open, Shelf, ShelfError};

/// Whether a sefer on the shelf is a scan.
///
/// Asked by the shelf row and by the pane, because opening a scan into the text
/// pane shows a reader a sefer of blank lines — which is what this window did
/// until this work order, and which reads as a corrupt import rather than as a
/// scan nobody has OCR'd.
#[must_use]
pub fn is_scan(work: &Work) -> bool {
    work.source == Source::Mine && matches!(mine::Kind::of(&work.origin), Ok(mine::Kind::Pdf))
}

/// How many pages the scan has, counted from the sefer rather than from the
/// file.
///
/// The importer counted them through the PDF's page tree when the file was
/// dropped and minted an id for each; counting again here would be a second
/// implementation that can disagree with the ids, and then a mapping would run
/// off the end of a sefer that has fewer pages than the mapping believes.
#[must_use]
pub fn pages_of(sefer: &Open) -> usize {
    sefer
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::Page)
        .count()
}

/// The permanent id of a page of the **file** — what a note, a highlight or a
/// link anchors to.
///
/// **Not affected by the mapping**, and that is the point of it being here: the
/// reader can re-declare which page is daf ב as often as they like, and every
/// note they have made on the scan stays on the page they made it on. The
/// mapping says what a page is *called*; the id says which page it *is*.
///
/// Which is why this counts through the segments rather than asking
/// [`Open::at`]: once a scan is paged, an address of it is a place in the
/// **sefer**, and this is the one question in the app that is still about the
/// file.
#[must_use]
pub fn page_id(sefer: &Open, page: usize) -> Option<SegmentId> {
    sefer
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::Page)
        .nth(page.checked_sub(1)?)
        .map(|s| s.id.clone())
}

/// Which page of the file a segment id is — the other way round.
///
/// A **count** through the pages, not arithmetic on the ordinal: `#47` is the
/// forty-seventh segment minted, and reading a page number out of it would be
/// the same derivation this project spent W6 removing. Splitting a page (a
/// correction to an OCR'd scan, W26) mints `#47.1`, and this still counts it as
/// one page.
#[must_use]
pub fn page_of_id(sefer: &Open, id: &SegmentId) -> Option<usize> {
    sefer
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::Page)
        .position(|s| s.id == *id)
        .map(|at| at + 1)
}

/// Which word of a page's reading a candidate from the OCR queue is (W21
/// meeting W26).
///
/// The counterpart of [`crate::fixing::where_word`], and the reason the two are
/// not one function is the reason this gap existed at all: **a page is the one
/// segment whose words are not in its text.** The importer gives a dropped PDF
/// one segment per page with an empty string in it — what is printed there is a
/// machine's opinion, kept in `personal/words/<slug>/pages.jsonl` — so
/// `where_word` tokenizes nothing and finds nothing, and every candidate the
/// queue placed on a photograph opened on *that word is not in that line any
/// more*. The candidates were ranked; they were not reachable.
///
/// The comparison is on the **normalized** form, the same rule `where_word`
/// uses and for the same reason: the queue works in the index's spelling —
/// nikud off, final letters folded — and the reading is in whatever the engine
/// saw. Tokenizing rather than normalizing the whole string is deliberate: one
/// `Word` can hold a whole line when the file positions lines rather than words
/// (see *Two words with one rectangle*), and then the word wanted is one token
/// inside it.
///
/// The first occurrence, where a page has the word twice — the box opens
/// somewhere the reader can see it, and the second one is the next item in the
/// queue.
#[must_use]
pub fn where_word_on_page(read: &girsa_scan::Read, word: &str) -> Option<usize> {
    read.words.iter().position(|on_page| {
        let mut found = false;
        girsa_hebrew::for_each_token(&on_page.text, |token, _, _| {
            found = found || token == word;
        });
        found
    })
}

/// How much of a word has to sit inside a rectangle to be under it.
///
/// Most of it, and the number is the same shape as `girsa-scan`'s own rule for
/// re-finding a correction: a word that a mark clips the edge of was not
/// marked, and a word a mark covers three quarters of was. The failure this
/// guards is the one W24 named about a dibur hamatchil and W26 named again
/// about a rectangle — **a highlight two letters off looks exactly like one
/// that landed right**, so the rule is stated rather than tuned by eye.
const MOSTLY_INSIDE: f32 = 0.6;

/// How much two words have to overlap vertically to be on one line.
///
/// The same question `girsa-scan` answers when it groups glyphs into lines, and
/// deliberately the same shape of answer: Hebrew type has no descenders to
/// speak of, so two words on one line share almost all of their height and two
/// words on neighbouring lines share almost none.
const ONE_LINE: f32 = 0.5;

/// The ink a run of words sits on, one rectangle per line, and the words.
///
/// **Not one rectangle for the run.** A highlight over three lines of a daf has
/// a bounding box that also covers everything between its first word and its
/// last, including the ends of the lines it passes through — so redrawing from
/// that box would grow the highlight by however many words happened to lie in
/// the middle. One rectangle per line is tight on every line, which is also
/// what a highlight over running text looks like.
///
/// `None` when the range names no words, which is a caller asking about a page
/// nobody has read or a range that ran off the end of one.
#[must_use]
pub fn ink_of(
    read: &girsa_scan::Read,
    words: std::ops::Range<usize>,
) -> Option<(Vec<Area>, String)> {
    let picked = read.words.get(words)?;
    if picked.is_empty() {
        return None;
    }
    let mut lines: Vec<Area> = Vec::new();
    for word in picked {
        match lines.last_mut() {
            // Same line: widen the rectangle to take this word in.
            Some(line) if shares_a_line(*line, word.at) => *line = line.with(word.at),
            _ => lines.push(word.at),
        }
    }
    let said = picked
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    Some((lines, said))
}

/// Whether a word sits on the line a rectangle already covers.
fn shares_a_line(line: Area, word: Area) -> bool {
    let overlap = (line.bottom.min(word.bottom) - line.top.max(word.top)).max(0.0);
    let tall = (word.bottom - word.top).max(f32::EPSILON);
    overlap / tall >= ONE_LINE
}

/// Which words of the reading a mark's ink covers, in reading order.
///
/// Asked of whatever reading the page has **now**. A page read again by a
/// better engine has different words in slightly different places, and the
/// honest answer is *these are the words under that ink today* rather than the
/// words that were under it when the mark was made — which is exactly the
/// property that makes an ink anchor worth having over an offset one.
#[must_use]
pub fn words_under(read: &girsa_scan::Read, ink: &[Area]) -> Vec<usize> {
    read.words
        .iter()
        .enumerate()
        .filter(|(_, word)| {
            ink.iter()
                .any(|area| word.at.covered_by(*area) >= MOSTLY_INSIDE)
        })
        .map(|(at, _)| at)
        .collect()
}

/// The scan a sefer is, if it is one.
///
/// A scan with no mapping is still a scan — it is one the chore has not been
/// done for, and [`Scan::is_paged`] is how the window tells the two apart.
#[must_use]
pub fn scan_of(shelf: &Shelf, sefer: &Open) -> Option<Scan> {
    if !is_scan(&sefer.work) {
        return None;
    }
    let paging = shelf.scans().of(sefer.slug()).cloned().unwrap_or_default();
    Some(Scan::new(sefer.slug(), pages_of(sefer), paging))
}

/// The page of this scan that a segment of the sefer beside it is printed on.
///
/// This is W9's rule applied to a photograph: **a column follows another only
/// when something says the two are the same sefer.** Here that something is the
/// reader saying so — `--of bavli/berakhot` — and once they have, the page is a
/// count from an anchor rather than a resemblance. A scan and a text that
/// merely share an address shape line up beautifully and mean nothing, and a
/// scan that slid to the wrong daf with the header naming the right one is the
/// failure W9 built `NoPlace` to avoid.
///
/// `None` for a scan of something else, an unpaged scan, or a line whose daf
/// this scan does not carry — and the window leaves the pane where it is.
#[must_use]
pub fn beside(scan: &Scan, leader: &SegmentId) -> Option<usize> {
    if scan.paging().of()? != leader.work() {
        return None;
    }
    // The first level, because a scan knows which daf a line is on and makes no
    // claim about where on the daf.
    let address = crate::shelf::address_of(leader);
    let first = address.levels().first()?;
    scan.page_of(&girsa_ref::Address::new(vec![first.clone()]))
}

/// What a page of this scan is cited as being in.
///
/// The scan's own work, or — where the reader has said what the scan is a scan
/// **of** — that sefer, so a citation off a scan of Berakhot is `ברכות ב.` and
/// resolves to the same place as everybody else's.
///
/// # Errors
///
/// If the scan names a sefer that is not on this shelf. Refused rather than
/// fallen back to the scan's own title: the reader said this is Berakhot, and
/// printing it under the filename instead would be answering a different
/// question without saying so.
pub fn naming(shelf: &Shelf, scan: &Scan) -> Result<Sefer, ShelfError> {
    let slug = scan.paging().of().unwrap_or_else(|| scan.slug());
    shelf
        .work(slug)
        .map(about)
        .ok_or_else(|| ShelfError::NoSuchWork(slug.to_string()))
}

/// The mareh makom for a page: the three flavours, with no quote.
///
/// `None` for a page the mapping does not cover — the shaar blatt, the
/// haskamos, a plate. The window says *page 2 of the file*, which describes
/// where the reader is without pretending it is a place anyone could look up.
#[must_use]
pub fn mareh_makom(
    scan: &Scan,
    page: usize,
    naming: &Sefer,
    scanned: &Work,
    style: CiteStyle,
) -> Option<Sent> {
    let reference = scan.reference(page)?;
    let display = cite(naming, &reference, style);

    let mut packet = SourcePacket::new(&reference, display.clone(), String::new());
    // Which scan it was read in. A mekor off somebody's photograph of the
    // Lemberg edition and one off the Vilna are the same place and not the same
    // page, and the packet is the only thing that carries the difference.
    packet.version = provenance(scanned);

    Some(Sent {
        // Bare, and not `(…)` the way a quote's citation is: there is nothing
        // in front of it for the brackets to be attached to.
        plain: display.clone(),
        html: format!(
            "<cite dir=\"rtl\" lang=\"he\" style=\"direction:rtl\"><a href=\"{}\">{}</a></cite>",
            crate::markup::attr(&reference.to_string()),
            crate::markup::text(&display)
        ),
        packet,
    })
}
