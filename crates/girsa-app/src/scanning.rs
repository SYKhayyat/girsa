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
