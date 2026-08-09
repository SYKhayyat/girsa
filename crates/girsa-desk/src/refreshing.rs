//! *Give me this whole document again, as the corpus stands now* (spec.md
//! §10.2).
//!
//! # The promise, and the thing that could not perform it
//!
//! §10.2 makes two promises about a document that stores **refs** rather than
//! printed strings: switch a whole sefer from abbreviated citations to
//! full-form ones, and **regenerate every quote against a corrected edition**.
//! Both are stated about a *document*, and until this module the desk could
//! only answer about a *place*: `/quote` takes one ref and returns one source.
//!
//! A pen holding forty citations could of course call `/quote` forty times. It
//! would also have to decide, forty times, what to do when the eleventh names a
//! sefer this shelf does not have — and a document-wide operation that stops
//! at the first missing sefer is not the promise, it is a report of the first
//! missing sefer. That decision is the library's, it is made once, and it is
//! made here: **a citation that cannot be refreshed is a row with a reason in
//! it, not a refusal for the other thirty-nine.**
//!
//! # Why this is the errand that earns the desk
//!
//! Everything else the two applications hand each other, an operating system
//! could carry. A source travels on the clipboard perfectly well — push, one
//! direction, no reply, and Ctrl+V is the whole protocol.
//!
//! This is a **pull**: a question with an answer that has to come back into the
//! asking application's own document, sized by the document rather than by the
//! selection. A clipboard has no reply channel and no way to express *forty
//! answers, in this order, three of which failed differently*. That is what the
//! loopback is for, and it is why `girsa-post` pays for a listener, a token, a
//! per-run endpoint file and a three-state presence protocol.
//!
//! # What comes back, and what the pen does with it
//!
//! One row per citation, **in the order they appear in the document**, from the
//! same [`girsa_ksav::cited_in`] both applications compile. The pen re-runs
//! that scanner on its own buffer and zips by position: one scanner, one order,
//! and no ref matched by string.
//!
//! The rows carry the citation as it prints *today* and the words as they read
//! *today*. What to replace, whether to ask first, and where the cursor ends up
//! are the pen's — this module has no opinion about somebody else's buffer.

use girsa_app::sending::Sent;
use girsa_ref::{RedirectTable, Ref};
use girsa_source::Range;
use serde::{Deserialize, Serialize};

/// One citation a document holds, ready to be asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wanted {
    /// The place, parsed.
    pub reference: Ref,
    /// Which characters of it the citation quoted, as the document spelled it.
    ///
    /// `None` is *the whole of what the ref names*. A document written before
    /// the field existed says nothing, and regenerating the whole place is all
    /// anyone can honestly do with it.
    pub range: Option<Range>,
}

/// Every citation in the document that names a place this library could look
/// up, in the order they appear.
///
/// A `מקור:` whose value does not parse as a ref is dropped rather than
/// reported: it is not a citation into this library, and a document is allowed
/// to contain other people's markup.
#[must_use]
pub fn wanted(markup: &str) -> Vec<Wanted> {
    girsa_ksav::cited_in(markup)
        .into_iter()
        .filter_map(|cited| {
            Some(Wanted {
                reference: cited.reference.parse().ok()?,
                range: cited.range,
            })
        })
        .collect()
}

/// One citation, as the corpus stands now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refreshed {
    /// The ref, echoed back so a row is readable on its own.
    #[serde(rename = "ref")]
    pub reference: String,
    /// The characters of it this citation quoted, unchanged: refreshing a quote
    /// is *the same words, re-read*, and a range that moved would be a
    /// different quote.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    /// The citation as it prints today, in the style asked for.
    pub display: String,
    /// The words today — corrected, and with the nikud the reader asked for.
    pub text: String,
    /// Why this one could not be refreshed, if it could not.
    ///
    /// Present *instead of* words. A row with a reason in it is the whole
    /// reason this is one errand and not forty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trouble: Option<String>,
}

impl Refreshed {
    /// A citation that came back.
    #[must_use]
    fn found(wanted: &Wanted, sent: &Sent) -> Self {
        Self {
            reference: wanted.reference.to_string(),
            range: wanted.range,
            display: sent.display().to_string(),
            text: sent.packet.text.clone(),
            trouble: None,
        }
    }

    /// A citation that did not, and why.
    #[must_use]
    fn lost(wanted: &Wanted, why: String) -> Self {
        Self {
            reference: wanted.reference.to_string(),
            range: wanted.range,
            display: String::new(),
            text: String::new(),
            trouble: Some(why),
        }
    }

    /// Whether this row is a reason rather than a source.
    #[must_use]
    pub const fn is_trouble(&self) -> bool {
        self.trouble.is_some()
    }
}

/// Ask the library for every citation in a document.
///
/// `ask` is *how to reach a sefer* and nothing else — the shelf, the cache and
/// the reader's settings belong to whoever is holding them. Everything that is
/// a decision about a **document** is here: which citations there are, what
/// order they come back in, and that one failure is one row.
///
/// The closure returns an owned [`Sent`] rather than a borrowed sefer on
/// purpose: the caller's shelf is usually behind a lock, and a signature that
/// let a reference out of it would make the lock the errand's problem.
pub fn refreshed(
    markup: &str,
    ask: impl FnMut(&Ref, Option<Range>) -> Result<Sent, String>,
) -> Vec<Refreshed> {
    refreshed_reporting(markup, ask).0
}

/// The same, and where the citations that have **moved** now point.
///
/// # The half a refresh was answering silently
///
/// `Open::at` resolves an address through `covered_by`, which walks the
/// corpus's redirect rows and the ancestry — so a mareh makom whose place
/// upstream re-segmented comes back with **the right words** and no sign that
/// anything happened. Showing the reader the right words is correct. Leaving
/// the document holding the old name is not: the ref in the `.typ` file now
/// resolves only because a redirect row exists, and rows are kept against a
/// shelf, not against a document somebody emailed you.
///
/// So the fact travels. The packet knows where the words are *today*
/// (`packet.reference`), the document says where they were, and when the two
/// disagree that is one row of a [`RedirectTable`] — the type `girsa-ref` has
/// carried since day one for exactly this errand, under a header about refs
/// that *"get stored inside Ksav documents"*, and with no consumer until now.
///
/// What to do with it is the pen's. Offering to rewrite the mareh makom is the
/// obvious thing; keeping the table beside the document, so that a later
/// **offline** open can still follow it with no library to ask, is the one this
/// crate cannot do and Ksav can.
pub fn refreshed_reporting(
    markup: &str,
    mut ask: impl FnMut(&Ref, Option<Range>) -> Result<Sent, String>,
) -> (Vec<Refreshed>, RedirectTable) {
    let mut moved = RedirectTable::new();
    let rows = wanted(markup)
        .into_iter()
        .map(|one| match ask(&one.reference, one.range) {
            Ok(sent) => {
                // Only when it parses, and only when it differs. A packet whose
                // ref this build cannot read is not evidence that anything
                // moved; and a table with a row for every citation in the
                // document would be saying *everything moved*, which carries
                // the same information as saying nothing.
                if let Ok(now) = sent.packet.reference.parse::<Ref>() {
                    if now != one.reference {
                        moved.insert(&one.reference, vec![now]);
                    }
                }
                Refreshed::found(&one, &sent)
            }
            Err(why) => Refreshed::lost(&one, why),
        })
        .collect();
    (rows, moved)
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_app::pretend::shulchan_arukh;
    use girsa_app::sending::{quote, Selection};
    use girsa_cite::CiteStyle;
    use girsa_ksav::CitationPlacement;

    /// A document citing the first se'if whole and the second in part.
    fn document() -> String {
        let sefer = shulchan_arukh();
        let whole = girsa_app::send(
            &sefer,
            &Selection::whole(sefer.segments[0].id.clone()),
            CiteStyle::HebrewFull,
            false,
            None,
        )
        .expect("sends");
        let part = girsa_app::send(
            &sefer,
            &Selection {
                from: sefer.segments[1].id.clone(),
                to: sefer.segments[1].id.clone(),
                from_char: 0,
                to_char: Some(5),
            },
            CiteStyle::HebrewFull,
            false,
            None,
        )
        .expect("sends");
        format!(
            "#כותרת1[סוגיא]\n{}\nונראה לי\n{}\n",
            girsa_ksav::to_ksav(&whole.packet, CitationPlacement::Mekor),
            girsa_ksav::to_ksav(&part.packet, CitationPlacement::Mekor)
        )
    }

    fn against(markup: &str) -> Vec<Refreshed> {
        let sefer = shulchan_arukh();
        refreshed(markup, |reference, range| {
            quote(&sefer, reference, range, CiteStyle::HebrewFull, false).map_err(|e| e.to_string())
        })
    }

    #[test]
    fn every_citation_comes_back_in_the_order_the_document_has_them() {
        let rows = against(&document());
        assert_eq!(rows.len(), 2, "{rows:#?}");
        assert_eq!(rows[0].reference, "girsa:shulchan-arukh/orach-chayim/1:1");
        assert_eq!(rows[1].reference, "girsa:shulchan-arukh/orach-chayim/1:2");
        assert!(rows.iter().all(|r| !r.is_trouble()), "{rows:#?}");
    }

    #[test]
    fn half_a_seif_is_refreshed_as_half_a_seif() {
        // The whole point of the range reaching the document (§10.1): a
        // document that quoted five characters gets five characters back, not
        // the se'if they were taken from.
        let rows = against(&document());
        assert_eq!(rows[0].text, "יתגבר כארי לעמוד בבקר לעבודת בוראו");
        assert_eq!(rows[1].range.map(|r| r.is_all()), Some(false));
        assert_eq!(rows[1].text.chars().count(), 5, "{:?}", rows[1].text);
    }

    #[test]
    fn one_citation_that_cannot_be_found_is_one_row_and_not_a_refusal() {
        // The decision this module exists to make once. A document is forty
        // citations and a shelf is somebody's actual shelf; the eleventh naming
        // a sefer they have not imported is ordinary, and it is not a reason to
        // refuse to refresh the other thirty-nine.
        let mut markup = document();
        markup.push_str(&girsa_ksav::mekor(
            "משנה ברורה סימן א'",
            Some("girsa:mishnah-berurah/1:1"),
            None,
        ));
        let rows = against(&markup);
        assert_eq!(rows.len(), 3, "{rows:#?}");
        assert!(!rows[0].is_trouble());
        assert!(!rows[1].is_trouble());
        assert!(rows[2].is_trouble(), "{:#?}", rows[2]);
        assert!(rows[2].text.is_empty());
        assert_eq!(rows[2].reference, "girsa:mishnah-berurah/1:1");
    }

    #[test]
    fn markup_that_cites_nothing_is_an_empty_answer_and_not_an_error() {
        assert!(against("#כותרת1[סוגיא]\n\nסתם דברים\n").is_empty());
    }

    #[test]
    fn a_mekor_that_is_not_a_girsa_ref_is_not_this_librarys_business() {
        // A document is allowed to hold somebody else's markup. `cited_in`
        // already drops what does not start `girsa:`; this is the second half —
        // a `girsa:` string that is not a ref this library can parse.
        let markup = "#מראה_מקום(מקור: \"girsa:\")[כלום]";
        assert!(against(markup).is_empty());
        assert!(wanted(markup).is_empty());
    }

    #[test]
    fn the_scanner_is_the_one_both_applications_compile() {
        // Not a second reader of the format. If this ever stops agreeing with
        // `girsa_ksav::cited_in`, the pen and the library are zipping two
        // different lists by index — which is the one way this errand can
        // silently put the wrong words in somebody's document.
        let markup = document();
        let mine: Vec<String> = wanted(&markup)
            .iter()
            .map(|w| w.reference.to_string())
            .collect();
        let theirs: Vec<String> = girsa_ksav::cited_in(&markup)
            .into_iter()
            .map(|c| c.reference)
            .collect();
        assert_eq!(mine, theirs);
    }
}
