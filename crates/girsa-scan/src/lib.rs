//! Scans you brought: which page is which daf, and what a page cites as.
//!
//! spec.md §6.3, BUILDER.md W25. **The scan *is* the daf** — tzuras hadaf comes
//! from a photograph of the page, not from a typesetting engine, which is what
//! makes the PDF layer a second reading mode rather than an attachment. Nothing
//! is shipped and nothing is fetched: every scan here is one the reader brought.
//!
//! # What is in this crate and what is not
//!
//! A PDF dropped on the window is already a sefer: `girsa-corpus`'s importer
//! gives it one segment per page and a permanent id for each (W6), and it is on
//! the shelf, addressable and notable, before anything here runs. What it does
//! **not** have is a mekor — nobody writes *"page 47 of berakhot-vilna.pdf"* in
//! a chaburah — and that is the whole of what this crate adds:
//!
//! ```text
//! page 47 of the file  ──[ the mapping ]──►  ברכות כ"ד:  ──►  girsa:bavli/berakhot/24b
//! ```
//!
//! Both directions, because both are asked. Forward is *what am I looking at*,
//! and it is what the window puts in the header and what Ctrl+C copies.
//! Backward is *where is daf כד* — a search hit, a link, a mekor clicked in a
//! Ksav document — and it is what makes a scan open on the right page instead
//! of at the beginning.
//!
//! # The one thing that is load-bearing
//!
//! [`paging`] is where the reasoning is. In one line: the mapping is a list of
//! anchors rather than a single offset, because a single offset cannot be
//! corrected without moving every citation already written against it — which
//! is the line-index defect of BUILDER.md T1, one level up.
//!
//! # Rendering
//!
//! Not here. A page of a PDF is drawn by the window, out of the file the reader
//! dropped; nothing in this crate opens the PDF at all. What it needs to know
//! about the file is how many pages it has, and the importer counted those when
//! the sefer was put on the shelf.

pub mod engine;
pub mod paging;
pub mod reading;
pub mod store;
pub mod words;

use girsa_cite::{cite, CiteStyle, Sefer};
use girsa_ref::{Address, Ref};

pub use engine::{Engine, EngineError, Image, Tesseract};
pub use paging::{Anchor, Paging, Placed, Refused, Scheme};
pub use reading::{corrected, group, Area, Fix, Glyph, Read, Reader, Word};
pub use store::{Scans, StoreError};
pub use words::{Job, Words, WordsError};

/// A scan on the shelf: a slug, a page count, and what the reader has said
/// about which page is which daf.
///
/// Cheap to build and holds no file. The window makes one per open pane out of
/// the work and the mapping.
#[derive(Debug, Clone)]
pub struct Scan {
    slug: String,
    pages: usize,
    paging: Paging,
}

impl Scan {
    #[must_use]
    pub fn new(slug: impl Into<String>, pages: usize, paging: Paging) -> Self {
        Self {
            slug: slug.into(),
            pages,
            paging,
        }
    }

    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    #[must_use]
    pub fn pages(&self) -> usize {
        self.pages
    }

    #[must_use]
    pub fn paging(&self) -> &Paging {
        &self.paging
    }

    /// Whether the once-per-sefer chore has been done.
    ///
    /// The window asks, because *this scan has no mareh makom yet* and *this
    /// page has nothing printed on it* are two different sentences and only one
    /// of them is a thing the reader can fix.
    #[must_use]
    pub fn is_paged(&self) -> bool {
        self.paging.is_declared()
    }

    /// What is printed on a page of the file.
    #[must_use]
    pub fn at(&self, page: usize) -> Placed {
        if page == 0 || page > self.pages {
            return Placed::Unpaged;
        }
        self.paging.at(page)
    }

    /// The page an address is printed on — `None` where this scan does not
    /// carry it, and never the nearest page it does.
    #[must_use]
    pub fn page_of(&self, address: &Address) -> Option<usize> {
        self.paging.page_of(address, self.pages)
    }

    /// The page a ref lands on.
    ///
    /// The ref may be deeper than a page can be — `girsa:bavli/berakhot/2a:5`
    /// is a line, and a scan knows only that the line is somewhere on the daf.
    /// The **first level** is what is looked up, which is the honest answer:
    /// the page it is printed on, with no claim about where on the page.
    ///
    /// A ref into another sefer is not this scan's business, and answering with
    /// a page anyway is how a reader ends up looking at Berakhot with the header
    /// saying Shabbos.
    #[must_use]
    pub fn page_of_ref(&self, reference: &Ref) -> Option<usize> {
        if reference.work_slug() != self.citing_slug() {
            return None;
        }
        let first = reference.from().levels().first()?;
        self.page_of(&Address::new(vec![first.clone()]))
    }

    /// The canonical ref for a page.
    ///
    /// Into the sefer the scan is **of**, where the reader has said what it is a
    /// scan of — so a citation from a scan of Berakhot is a citation of
    /// Berakhot, resolving to the same place as everyone else's, rather than a
    /// pointer into one person's file. Into the scan's own work otherwise,
    /// which is a real ref and a real place on a real shelf.
    #[must_use]
    pub fn reference(&self, page: usize) -> Option<Ref> {
        let work: Vec<String> = self.citing_slug().split('/').map(str::to_string).collect();
        match self.at(page) {
            Placed::At { from, to: None } => Some(Ref::point(work, from)),
            Placed::At {
                from,
                to: Some(end),
            } => Some(Ref::span(work, from, end)),
            Placed::Unpaged => None,
        }
    }

    /// The mareh makom for a page, printed.
    ///
    /// `None` for a page with nothing printed on it that a mekor could name.
    /// The window says *page 2 of the file*; this says nothing, because
    /// describing a page and citing one are different acts and only the second
    /// one is followed back a year later.
    #[must_use]
    pub fn cite(&self, page: usize, sefer: &Sefer, style: CiteStyle) -> Option<String> {
        Some(cite(sefer, &self.reference(page)?, style))
    }

    /// Which work a page of this scan is cited as being in.
    fn citing_slug(&self) -> String {
        self.paging
            .of()
            .map_or_else(|| self.slug.clone(), ToString::to_string)
    }
}
