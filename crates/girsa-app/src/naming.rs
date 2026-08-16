//! A segment, described for a surface that is about to draw it.
//!
//! # The four that each invented this
//!
//! A search hit, a lane result, an MCP answer and a printed line all have to
//! turn a [`SegmentId`] into something a reader can look at, and all four
//! composed it separately:
//!
//! | | title | address | dated |
//! |---|---|---|---|
//! | `HitRow` (window search) | `Language::title_of`, falling back to the slug | `path().join(":")` | no |
//! | `girsa_app::Near` (lane) | `he_title`, falling back to **the empty string** | *nothing* | no |
//! | `girsa_mcp::named` | `he_title`, falling back to the slug | `path().join(":")` | yes |
//! | `girsa-chain`'s `Printer::said` | `he_title`, falling back to the slug | `path().join(":")` | yes, and `[no date]` where the others say nothing |
//!
//! Read the columns rather than the rows. **One** of the four honours the
//! language the window is in, so a reader who set the window to English gets
//! English titles in search results and Hebrew ones in the lane beside it.
//! **One** of the four has no address at all, so `NearRow` in the shell and
//! `girsa-lane ask` on the command line each invented one — and they invented
//! different ones: the window shows `58:1` and the terminal shows
//! `girsa:sefaria/shulchan_arukh…#58.1`, a permanent id printed where an
//! address goes. **Two** of the four carry the date, which is the column that
//! makes a chain readable, and the two that don't are the two a reader looks at
//! most.
//!
//! None of that is a bug anybody wrote. It is four people answering the same
//! question four times without knowing it had been answered, which is this
//! repository's one recurring defect and the reason there is a type here now.
//!
//! # What this carries and what it does not
//!
//! Identity, and nothing else: which place this is, what to call it, where it
//! sits, when it was written. **Not the text** — a search hit draws marked-up
//! runs, a lane result draws a whole segment, `girsa-chain` draws twelve words
//! and `girsa-index find` draws a snippet windowed on the match. Those are four
//! genuinely different jobs and collapsing them would be inventing a fifth
//! problem to solve the first one.
//!
//! # Not a citation
//!
//! [`Naming::said`] is `שולחן ערוך, אורח חיים 58:1`. A **citation** is
//! `girsa_cite::cite`, which knows that this work's sections are called *סימן*
//! and *סעיף* and prints them, and which is compiled into Ksav so that the
//! application producing a mekor and the application printing it cannot
//! disagree. Everything that leaves this program as a mekor goes through that
//! (`crate::sending`). This is the row label.

use girsa_corpus::era::{Timeline, When};
use girsa_corpus::segment::SegmentId;

use crate::session::Language;
use crate::shelf::Shelf;

/// A segment, named and placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Naming {
    pub id: SegmentId,
    /// The work slug — `sefaria/shulchan_arukh_orach_chayim`.
    pub work: String,
    /// What to call the sefer, **in the language the window is in**.
    ///
    /// Falls back to the slug, never to the empty string: a row with no name on
    /// it is a row a reader cannot act on, and the slug is at least a name.
    pub title: String,
    /// `סימן נ״ח סעיף א׳` — the address as a person says it, through
    /// [`crate::sending::printed_address`], which is the one formatter in this
    /// application. It used to be [`SegmentId::address`], the **id's** own
    /// spelling, so a Hebrew search result read `שבת 31a`.
    pub address: String,
    /// `1565`, or `1488–1575` where the corpus gives a span.
    pub written: Option<String>,
    /// The era, in Hebrew, where the years are not known.
    pub era: Option<String>,
}

impl Naming {
    /// The whole of the composition, with the two lookups already done.
    ///
    /// Separate from [`Names::of`] because this is the part that had four
    /// answers and a `Shelf` has twenty fields — a rule worth testing should not
    /// need a corpus on disk to test it.
    #[must_use]
    pub fn named(
        id: &SegmentId,
        titles: Option<(&str, &str)>,
        when: When,
        language: Language,
        // `address` is how this work's addresses are printed, from
        // `crate::sending::printed_address`. `None` for a sefer the shelf does
        // not have: nobody knows what its levels are called, and the id's own
        // spelling is all there is.
        address: Option<String>,
    ) -> Self {
        let slug = id.work();
        let title = titles.map_or_else(String::new, |(he, en)| {
            language.title_of(he, en).trim().to_string()
        });
        Self {
            work: slug.to_string(),
            title: if title.is_empty() {
                slug.to_string()
            } else {
                title
            },
            address: address.unwrap_or_else(|| id.address()),
            written: when.written(),
            era: when.era.map(|era| era.he().to_string()),
            id: id.clone(),
        }
    }

    /// The one-line label — `שולחן ערוך, אורח חיים 58:1`.
    ///
    /// No date. See [`Naming::dated`], which is the shape `girsa-chain` needs and
    /// the reason the two used to be one hand-written `format!` in a bin.
    #[must_use]
    pub fn said(&self) -> String {
        if self.address.is_empty() {
            self.title.clone()
        } else {
            format!("{} {}", self.title, self.address)
        }
    }

    /// The label with when it was written — `שולחן ערוך, אורח חיים 58:1  [1565]`.
    ///
    /// `[no date]` rather than nothing where the corpus could not place the
    /// work, which is `girsa-chain`'s call and the right one: a trace is read
    /// down the years column, and a row with a blank there reads as *earlier
    /// than the one above* rather than as *unknown*.
    #[must_use]
    pub fn dated(&self) -> String {
        let when = match (&self.written, &self.era) {
            (Some(years), _) => format!("  [{years}]"),
            (None, Some(era)) => format!("  [{era}]"),
            (None, None) => "  [no date]".to_string(),
        };
        format!("{}{when}", self.said())
    }
}

/// What it takes to name a place: the shelf's titles, the corpus's dates, and
/// which of a sefer's two names the reader is reading in.
///
/// # Why this is a type and not three more arguments
///
/// The four composers each took what they happened to have. `Adjacency::ask`
/// took a `&Shelf`, so `Near` got a Hebrew title and no date; `HitRow` was built
/// where a `Session` was in scope, so it got the language; `mcp::named` was
/// built where a `Timeline` was in scope, so it got the years. Nobody chose
/// those differences — they are what was reachable from where the code was
/// written.
///
/// Passing this instead of a bare `&Shelf` costs no arity and makes the
/// difference impossible: a caller with no timeline says so once, here, rather
/// than by quietly leaving a column empty.
#[derive(Clone, Copy)]
pub struct Names<'a> {
    pub shelf: &'a Shelf,
    /// When each work was written. `None` where the caller has not read the
    /// catalogue — the date column then reads *no date*, which is true.
    pub timeline: Option<&'a Timeline>,
    pub language: Language,
    /// How a place is printed. The reader's setting, so a row label and the
    /// citation they copy off the same line agree.
    pub style: girsa_cite::CiteStyle,
}

impl<'a> Names<'a> {
    #[must_use]
    pub const fn new(
        shelf: &'a Shelf,
        timeline: Option<&'a Timeline>,
        language: Language,
        style: girsa_cite::CiteStyle,
    ) -> Self {
        Self {
            shelf,
            timeline,
            language,
            style,
        }
    }

    /// A shelf, and nothing else known — Hebrew titles, no dates.
    ///
    /// What three of the four composers effectively had. Spelled out, so that a
    /// surface drawing a blank date column is a surface that asked for one.
    #[must_use]
    pub const fn on(shelf: &'a Shelf) -> Self {
        Self::new(
            shelf,
            None,
            Language::Hebrew,
            girsa_cite::CiteStyle::HebrewFull,
        )
    }

    /// Name a place.
    #[must_use]
    pub fn of(&self, id: &SegmentId) -> Naming {
        let slug = id.work();
        let work = self.shelf.work(slug);
        let titles = work.map(|work| (work.he_title.clone(), work.en_title.clone()));
        let when = self
            .timeline
            .map(|timeline| timeline.when(slug))
            .unwrap_or_default();
        Naming::named(
            id,
            titles.as_ref().map(|(he, en)| (&**he, &**en)),
            when,
            self.language,
            // **With the schema's own words**, which is what makes a chain
            // hop into the Tur read `אורח חיים סימן א' סעיף א'` rather than
            // `orach_chayim א' א'`. The pane had this the moment the sefer was
            // open; a row about a sefer that is *not* open had nowhere to get
            // it from until the shelf kept it.
            work.map(|work| {
                crate::sending::printed_address_in(
                    work,
                    Some(&self.shelf.sections(slug)),
                    id,
                    self.style,
                )
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use girsa_corpus::segment::Ordinal;

    use super::*;

    fn id(work: &str, path: &[&str]) -> SegmentId {
        SegmentId::new(
            work,
            path.iter().map(|p| (*p).to_string()).collect(),
            Ordinal::root(1),
        )
    }

    #[test]
    fn an_address_is_the_numbers_and_not_the_permanent_id() {
        // `girsa-lane ask` printed `near.id` where the window printed
        // `58:1` — one lane result, two strings, because `Near` had no address
        // field and each surface invented one.
        let at = id("sefaria/shulchan_arukh", &["58", "1"]);
        assert_eq!(at.address(), "58:1");
        assert!(at.to_string().contains("girsa:"), "{at}");
        assert!(!at.address().contains("girsa:"), "{}", at.address());
    }

    #[test]
    fn a_work_the_shelf_does_not_know_is_named_by_its_slug() {
        // `Near` fell back to the empty string here, so a lane result over a
        // sefer the catalogue had not caught up with drew a row with no name on
        // it at all — and a row with no name is a row a reader cannot act on.
        let at = id("user/משהו", &["3"]);
        let naming = Naming::named(&at, None, When::default(), Language::Hebrew, None);
        assert_eq!(naming.title, "user/משהו");
        assert_eq!(naming.said(), "user/משהו 3");
    }

    #[test]
    fn a_title_is_in_the_language_the_window_is_in() {
        // The column only `HitRow` had. A reader who set the window to English
        // got English titles in the search results and Hebrew ones in the lane
        // beside them, because `Near` asked for `he_title` and nothing else.
        let at = id("sefaria/berakhot", &["2", "1"]);
        let titles = Some(("ברכות", "Berakhot"));
        assert_eq!(
            Naming::named(&at, titles, When::default(), Language::English, None).said(),
            "Berakhot 2:1"
        );
        assert_eq!(
            Naming::named(&at, titles, When::default(), Language::Hebrew, None).said(),
            "ברכות 2:1"
        );
    }

    #[test]
    fn a_sefer_with_only_one_of_its_two_names_is_called_that() {
        // `Language::title_of`'s rule, reached from here so it is reached from
        // everywhere. Most of the corpus has both; a sefer you dropped on the
        // window has one.
        let at = id("user/שלי", &["1"]);
        assert_eq!(
            Naming::named(
                &at,
                Some(("שלי", "")),
                When::default(),
                Language::English,
                None
            )
            .title,
            "שלי"
        );
    }

    #[test]
    fn a_naming_with_no_address_is_the_sefer_itself() {
        let naming = Naming::named(
            &id("user/x", &[]),
            None,
            When::default(),
            Language::Hebrew,
            None,
        );
        assert_eq!(naming.said(), "user/x");
    }

    #[test]
    fn an_undated_work_says_so_rather_than_leaving_the_column_blank() {
        // `girsa-chain`'s call, kept. A blank years column in a trace reads as
        // *earlier than the row above*, which is the one thing a trace must not
        // say by accident.
        let naming = Naming::named(
            &id("user/x", &["1"]),
            None,
            When::default(),
            Language::Hebrew,
            None,
        );
        assert_eq!(naming.dated(), "user/x 1  [no date]");
    }

    #[test]
    fn a_span_of_years_is_one_column_and_a_single_year_is_not_a_span() {
        let at = id("sefaria/x", &["1"]);
        let span = When {
            era: Some(girsa_corpus::era::Era::Rishonim),
            years: Some((1488, 1575)),
        };
        assert_eq!(
            Naming::named(&at, None, span, Language::Hebrew, None).dated(),
            "sefaria/x 1  [1488\u{2013}1575]"
        );
        let one = When {
            era: None,
            years: Some((1565, 1565)),
        };
        assert_eq!(
            Naming::named(&at, None, one, Language::Hebrew, None).dated(),
            "sefaria/x 1  [1565]"
        );
        // Years beat the era where both are known: `[1565]` says more than
        // `[ראשונים]` and takes less width.
        let both = When {
            era: Some(girsa_corpus::era::Era::Rishonim),
            years: Some((1565, 1565)),
        };
        assert_eq!(
            Naming::named(&at, None, both, Language::Hebrew, None).dated(),
            "sefaria/x 1  [1565]"
        );
    }
}
