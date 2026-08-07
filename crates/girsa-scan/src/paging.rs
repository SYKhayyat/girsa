//! Which page is which daf.
//!
//! # A page and a daf are two coordinate systems
//!
//! A PDF has pages, numbered 1..n by the file. A sefer has dafim, simanim,
//! printed page numbers — what is written **on** the page. They are not the
//! same sequence and they never line up: a Vilna Shas scan opens with a shaar
//! blatt and haskamos, so daf ב is page 5, and somewhere around daf כ a plate
//! is bound in and from there they are one apart again.
//!
//! The whole of this module is the declaration that relates the two, and the
//! shape it takes is the only interesting decision in it.
//!
//! # Why it is a list of anchors and not one number
//!
//! One number — *the daf is the page plus three* — is the obvious design, is
//! two lines of arithmetic, and is right until the first plate. Then the only
//! repair is to change the number, and changing it moves **every citation in
//! the sefer**: the four hundred pages that were already right move with the
//! ones that were wrong, silently, with nothing anywhere saying that a mekor
//! written last month now points a daf away.
//!
//! That is BUILDER.md T1 wearing a different hat. An Otzaria link is
//! `file + line index`, and inserting one line above it re-points every link
//! below — the defect this whole project exists to leave behind. A scan paged
//! by a single offset reintroduces it exactly, one level up: **the page number
//! is being used as the address.**
//!
//! So the mapping is a list of anchors, each of which says *this page is that
//! place*, and the count runs on from the nearest one behind. Declaring a new
//! anchor cannot move a page in front of it, because no page's address is ever
//! computed from an anchor after it. That property is the test
//! `a_plate_bound_into_the_middle_moves_no_page_before_it`, and it is why this
//! is shaped the way it is.
//!
//! # And what an anchor may say instead
//!
//! [`Anchor::unpaged`] — *from here, these pages are not pages of the sefer.*
//! The plates, an inserted index, a blank verso. Without it a mapping has to
//! pretend that every page between two anchors carries text, and the plates
//! come out cited as dafim that are printed somewhere else in the sefer.
//!
//! # What this deliberately does not model
//!
//! **One unit per page.** A daf a page, a leaf a page, a printed number a page.
//! A sefer with four simanim to the page is not describable here, and pretending
//! otherwise — by interpolating, by rounding to the siman that starts nearest —
//! is how a mekor ends up naming a siman the reader was not looking at. The
//! honest answer for that sefer is that the scan is citable by page and its
//! text is citable by siman, and W26's OCR is what joins the two.

use girsa_ref::{daf, Address, Level};

/// What a page of a scan carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scheme {
    /// One **amud** to the page — a scan of one side of a leaf at a time, which
    /// is what nearly every Shas PDF is. Page 5 is `ב.`, page 6 is `ב:`.
    #[default]
    Amud,
    /// One **daf** to the page — a photograph of the open sefer, so the page
    /// carries both amudim and is a span rather than a point.
    Daf,
    /// One **number** to the page: a printed page number, or a sefer whose
    /// divisions happen to run one to a page.
    Numbered,
}

girsa_corpus::spelled!(Scheme {
    Amud => "amud",
    Daf => "daf",
    Numbered => "numbered",
});

impl Scheme {
    /// The name it goes by on the wire and in the file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Amud => "amud",
            Self::Daf => "daf",
            Self::Numbered => "numbered",
        }
    }

    /// Read one back.
    #[must_use]
    /// How the levels of this scheme are counted: `2a` is 4 and `2b` is 5 when
    /// the unit is an amud; a daf is its own number; a printed number is itself.
    ///
    /// Everything in this module is arithmetic on these, which is what keeps
    /// the three schemes one implementation instead of three.
    fn unit_of(self, address: &Address) -> Option<u32> {
        let [level] = address.levels() else {
            return None;
        };
        match self {
            Self::Amud => amud_index(level),
            // Either amud names the leaf it is printed on, and so does the bare
            // number a reader is most likely to type.
            Self::Daf => amud_index(level)
                .map(|index| index / 2)
                .or_else(|| level.as_number())
                .filter(|daf| *daf >= FIRST_DAF),
            Self::Numbered => level.as_number().filter(|n| *n >= 1),
        }
    }

    /// The place a unit names — a point, or a span where a page is a whole daf.
    fn placed(self, unit: u32) -> Placed {
        match self {
            Self::Amud => Placed::At {
                from: amud_address(unit),
                to: None,
            },
            Self::Daf => Placed::At {
                from: amud_address(unit * 2),
                to: Some(amud_address(unit * 2 + 1)),
            },
            Self::Numbered => Placed::At {
                from: Address::new(vec![Level::number(unit)]),
                to: None,
            },
        }
    }

    /// The smallest unit this scheme can name. There is no daf א in any
    /// masechta — the first leaf is the title page — so a mapping that counted
    /// below it would be citing a place that has never been printed.
    fn floor(self) -> u32 {
        match self {
            Self::Amud => FIRST_DAF * 2,
            Self::Daf => FIRST_DAF,
            Self::Numbered => 1,
        }
    }

    /// What an anchor of this scheme has to be written as, for the message a
    /// reader gets when they write the other one.
    fn wants(self) -> &'static str {
        match self {
            Self::Amud => "a daf and an amud — ב. or ב ע\"ב",
            Self::Daf => "a daf — ב",
            Self::Numbered => "a number",
        }
    }
}

/// The first daf of any masechta.
const FIRST_DAF: u32 = 2;

/// Why a mapping was refused.
///
/// Every one of these is a mapping that would have cited a page as somewhere it
/// is not, so all of them are refusals rather than warnings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    #[error("`{written}` is not {wants}")]
    NotThisScheme {
        written: String,
        wants: &'static str,
    },
    #[error("page {0} is not a page — a file's pages start at 1")]
    NoSuchPage(usize),
    #[error("page {page} is declared twice")]
    Twice { page: usize },
    #[error("page {page} is declared {at}, and page {behind} is already {was} — a scan cannot go backwards")]
    Backwards {
        page: usize,
        at: String,
        behind: usize,
        was: String,
    },
}

/// One thing the reader can see: *this page is that place*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    /// The page of the **file**, from 1.
    pub page: usize,
    /// What is printed on it — or nothing, for pages that are not pages of the
    /// sefer at all.
    pub at: Option<Address>,
}

impl Anchor {
    /// *Page 5 is `ב.`*, written the way a reader writes a daf.
    ///
    /// # Errors
    ///
    /// If there is no such page, or the address will not read. Whether it is
    /// the *right kind* of address is settled by [`Paging::declare`], which is
    /// where the scheme is known.
    pub fn written(page: usize, at: &str) -> Result<Self, Refused> {
        if page == 0 {
            return Err(Refused::NoSuchPage(page));
        }
        let address = Address::parse(at).ok_or_else(|| Refused::NotThisScheme {
            written: at.to_string(),
            wants: "a place — a daf, or a number",
        })?;
        Ok(Self {
            page,
            at: Some(address),
        })
    }

    /// *From page 43, these are not pages of the sefer* — the plates, an
    /// inserted index, a blank verso.
    #[must_use]
    pub fn unpaged(page: usize) -> Self {
        Self { page, at: None }
    }
}

/// Where a page of a scan sits in the sefer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placed {
    /// The page carries this. `to` is set only where a page is a whole daf, and
    /// then the page is a span — which is what a ref has been since W3.
    At { from: Address, to: Option<Address> },
    /// This page is not in the mapping: it is in front of the first anchor, or
    /// inside a run the reader has marked as not part of the sefer, or the scan
    /// has never been paged.
    ///
    /// **Not an error and not an empty address.** A page with nothing printed
    /// on it that a mekor could name is an ordinary thing — a title page, a
    /// haskama, a plate — and the window says *page 2 of the file* rather than
    /// inventing a daf for it.
    Unpaged,
}

impl Placed {
    /// The address, where there is one.
    #[must_use]
    pub fn address(&self) -> Option<&Address> {
        match self {
            Self::At { from, .. } => Some(from),
            Self::Unpaged => None,
        }
    }

    #[must_use]
    pub fn is_paged(&self) -> bool {
        matches!(self, Self::At { .. })
    }
}

/// The declaration: what this scan's pages are, and what sefer they are of.
///
/// Constructed only through [`Paging::declare`], so a `Paging` that exists is a
/// mapping that has been checked. There is no way to hold an unchecked one, and
/// that is deliberate — the file on disk is hand-editable and is read through
/// the same door.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Paging {
    of: Option<String>,
    scheme: Scheme,
    anchors: Vec<Anchor>,
}

impl Paging {
    /// Declare a mapping, checking it.
    ///
    /// `of` is the slug of the work on the shelf this is a scan **of**, where
    /// the reader has said. It is what makes a scan of Berakhot cite as
    /// `ברכות ב.` — the mekor everybody else writes — instead of as a place in
    /// a file on one person's disk.
    ///
    /// # Errors
    ///
    /// If an anchor is not the kind of place the scheme counts, if two anchors
    /// name the same page, or if the anchors would run backwards — which is a
    /// mapping under which two pages carry one address, and therefore one under
    /// which [`crate::Scan::page_of`] silently stops being able to find one of
    /// them.
    pub fn declare(
        of: Option<String>,
        scheme: Scheme,
        mut anchors: Vec<Anchor>,
    ) -> Result<Self, Refused> {
        anchors.sort_by_key(|a| a.page);
        for anchor in &anchors {
            if anchor.page == 0 {
                return Err(Refused::NoSuchPage(anchor.page));
            }
            let Some(at) = anchor.at.as_ref() else {
                continue;
            };
            if scheme.unit_of(at).is_none_or(|unit| unit < scheme.floor()) {
                return Err(Refused::NotThisScheme {
                    written: at.to_string(),
                    wants: scheme.wants(),
                });
            }
        }
        for pair in anchors.windows(2) {
            let [before, after] = pair else { continue };
            if before.page == after.page {
                return Err(Refused::Twice { page: after.page });
            }
        }

        // The check that keeps `page_of` a function. Inside one run the
        // addresses increase by construction; the only place two pages can end
        // up carrying one address is at an anchor, where the reader has just
        // said what the count is. So each anchor is compared with the page
        // behind it, under the runs already accepted.
        let mut checked: Vec<Anchor> = Vec::with_capacity(anchors.len());
        for anchor in anchors {
            if let (Some(at), Some(behind)) = (anchor.at.as_ref(), anchor.page.checked_sub(1)) {
                if let Placed::At { from, to } = placed_in(&checked, scheme, behind) {
                    let was = to.unwrap_or(from);
                    let (Some(now), Some(then)) = (scheme.unit_of(at), scheme.unit_of(&was)) else {
                        continue;
                    };
                    if now <= then {
                        return Err(Refused::Backwards {
                            page: anchor.page,
                            at: at.to_string(),
                            behind,
                            was: was.to_string(),
                        });
                    }
                }
            }
            checked.push(anchor);
        }

        Ok(Self {
            of,
            scheme,
            anchors: checked,
        })
    }

    /// The work on the shelf this is a scan of, where the reader has said.
    #[must_use]
    pub fn of(&self) -> Option<&str> {
        self.of.as_deref()
    }

    #[must_use]
    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    #[must_use]
    pub fn anchors(&self) -> &[Anchor] {
        &self.anchors
    }

    /// Whether anybody has done the chore.
    #[must_use]
    pub fn is_declared(&self) -> bool {
        self.anchors.iter().any(|a| a.at.is_some())
    }

    /// What page `page` of the file carries. Unbounded above — the page count
    /// belongs to the scan, not to the declaration.
    #[must_use]
    pub fn at(&self, page: usize) -> Placed {
        placed_in(&self.anchors, self.scheme, page)
    }

    /// The page an address is printed on, searching the runs in order.
    ///
    /// `None` where the scan does not carry it — **never the nearest page it
    /// does carry.** A scan opened one daf away, with the header naming the daf
    /// that was asked for, is wrong in the way nobody checks.
    #[must_use]
    pub fn page_of(&self, address: &Address, pages: usize) -> Option<usize> {
        let want = self.scheme.unit_of(address)?;
        for (i, anchor) in self.anchors.iter().enumerate() {
            let Some(at) = anchor.at.as_ref() else {
                continue;
            };
            let Some(start) = self.scheme.unit_of(at) else {
                continue;
            };
            if want < start {
                continue;
            }
            // Where this run stops: the next anchor, or the end of the file.
            let ends = self
                .anchors
                .get(i + 1)
                .map_or(pages, |next| next.page.saturating_sub(1));
            let Some(page) = usize::try_from(want - start)
                .ok()
                .and_then(|step| anchor.page.checked_add(step))
            else {
                continue;
            };
            if page <= ends && page <= pages {
                return Some(page);
            }
        }
        None
    }
}

/// The one piece of arithmetic, used by `at` and by the check in `declare` —
/// which is why it is a free function over a slice of anchors rather than a
/// method: `declare` has to ask what a mapping says while it is still deciding
/// whether to accept it.
fn placed_in(anchors: &[Anchor], scheme: Scheme, page: usize) -> Placed {
    if page == 0 {
        return Placed::Unpaged;
    }
    // The nearest anchor behind. Nothing ahead of it is consulted, and that is
    // the property the whole shape exists for.
    let Some(anchor) = anchors.iter().rev().find(|a| a.page <= page) else {
        return Placed::Unpaged;
    };
    let Some(at) = anchor.at.as_ref() else {
        return Placed::Unpaged;
    };
    let Some(start) = scheme.unit_of(at) else {
        return Placed::Unpaged;
    };
    let Ok(step) = u32::try_from(page - anchor.page) else {
        return Placed::Unpaged;
    };
    match start.checked_add(step) {
        Some(unit) => scheme.placed(unit),
        None => Placed::Unpaged,
    }
}

/// `2a` is 4, `2b` is 5 — one number per side of a leaf, so that counting on by
/// a page is adding one.
fn amud_index(level: &Level) -> Option<u32> {
    let canonical = daf::parse(level.as_str())?;
    let mut chars = canonical.chars();
    let side = chars.next_back()?;
    let daf: u32 = chars.as_str().parse().ok()?;
    Some(daf * 2 + u32::from(side == 'b'))
}

/// And back: 4 is `2a`.
fn amud_address(index: u32) -> Address {
    let side = if index % 2 == 0 { 'a' } else { 'b' };
    Address::new(vec![Level::canonical(format!("{}{side}", index / 2))])
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn an_amud_is_a_number_and_comes_back_as_itself() {
        for written in ["2a", "2b", "15a", "121b"] {
            let level = Level::canonical(written);
            let index = amud_index(&level).expect("an amud");
            assert_eq!(amud_address(index).to_string(), written);
        }
    }

    #[test]
    fn the_notations_a_reader_writes_a_daf_in_all_count_the_same() {
        // `girsa-ref` reads six of them (W3). An anchor typed `ב ע"ב` and one
        // typed `2b` are the same declaration, and a mapping that took one and
        // not the other would be a mapping that depends on how you type.
        for written in ["ב.", "ב ע\"א", "2a"] {
            let anchor = Anchor::written(5, written).expect(written);
            let at = anchor.at.expect("an address");
            assert_eq!(Scheme::Amud.unit_of(&at), Some(4), "{written}");
        }
    }

    #[test]
    fn a_run_is_computed_from_the_anchor_behind_it_and_never_from_one_ahead() {
        // Said as an assertion rather than only in the module note: adding an
        // anchor at page 20 may not change what page 19 says, whatever it says.
        let first = vec![Anchor::written(5, "ב.").expect("an anchor")];
        let mut both = first.clone();
        both.push(Anchor::written(20, "כ.").expect("an anchor"));
        for page in 1..20 {
            assert_eq!(
                placed_in(&first, Scheme::Amud, page),
                placed_in(&both, Scheme::Amud, page),
                "page {page}"
            );
        }
    }
}
