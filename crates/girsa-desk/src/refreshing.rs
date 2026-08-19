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
//! **Total, and that is the load-bearing word.** *One row per citation* is what
//! makes zipping by position sound, so the count this returns has to be the
//! count `cited_in` found — not the count of those this build happened to be
//! able to parse. It was the second of those for a while: [`wanted`] dropped a
//! `מקור:` whose value this build's ref parser rejected, so a document with one
//! unreadable citation in the middle handed the pen *n* − 1 rows for *n*
//! citations and every quote after the bad one was re-quoted **from the wrong
//! place**, each row individually well-formed and carrying a plausible
//! citation. `"total"` was the post-drop count too, so even a caller that
//! wanted to check got the number that agreed with itself.
//!
//! `sending.rs` says what class that is: *a quote taken from the se'if next
//! door is exactly the silent wrongness this system exists to make impossible*.
//! So [`Wanted`] has a variant for *this one did not parse*, and a citation
//! this library cannot look up is a row with a reason in it — the same answer
//! this module already gave for a sefer the shelf does not have.
//!
//! The rows carry the citation as it prints *today* and the words as they read
//! *today*. What to replace, whether to ask first, and where the cursor ends up
//! are the pen's — this module has no opinion about somebody else's buffer.

use girsa_app::sending::Sent;
use girsa_ref::{RedirectTable, Ref};
use girsa_source::Range;
use serde::{Deserialize, Serialize};

/// One citation a document holds, ready to be asked about.
///
/// Two variants and not one, because the answer has to be **total**: see the
/// module note. A citation this build cannot read is still a citation the
/// document has, still a position in the order, and still a row the pen is
/// going to zip against its own scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wanted {
    /// A place this library can look up.
    Parsed {
        /// The place, parsed.
        reference: Ref,
        /// Which characters of it the citation quoted, as the document spelled
        /// it.
        ///
        /// `None` is *the whole of what the ref names*. A document written
        /// before the field existed says nothing, and regenerating the whole
        /// place is all anyone can honestly do with it.
        range: Option<Range>,
    },
    /// A `מקור:` whose value this build's ref parser rejects.
    ///
    /// Not a failure of the document. A ref written by a newer Girsa whose
    /// syntax this build does not know, a ref hand-edited in the `.typ`, a
    /// `girsa:` string a copy-paste truncated — all of them reach here, and all
    /// of them are things a reader can be told about and act on. What none of
    /// them may do is vanish.
    Unreadable {
        /// The value as the document spells it, echoed back so the reader can
        /// see which citation is meant.
        text: String,
        /// Which characters of it the citation claimed, if it said.
        range: Option<Range>,
    },
}

/// Every citation in the document, in the order they appear.
///
/// One entry per `מקור:` [`girsa_ksav::cited_in`] found — the ones this build
/// can parse as [`Wanted::Parsed`], the ones it cannot as
/// [`Wanted::Unreadable`]. Nothing is dropped, because the caller zips by
/// position and a dropped row moves every citation after it onto the wrong
/// place.
#[must_use]
pub fn wanted(markup: &str) -> Vec<Wanted> {
    girsa_ksav::cited_in(markup)
        .into_iter()
        .map(|cited| match cited.reference.parse() {
            Ok(reference) => Wanted::Parsed {
                reference,
                range: cited.range,
            },
            Err(_) => Wanted::Unreadable {
                text: cited.reference,
                range: cited.range,
            },
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
    fn found(reference: &Ref, range: Option<Range>, sent: &Sent) -> Self {
        Self {
            reference: reference.to_string(),
            range,
            display: sent.display().to_string(),
            text: sent.packet.text.clone(),
            trouble: None,
        }
    }

    /// A citation that did not, and why.
    #[must_use]
    fn lost(reference: &Ref, range: Option<Range>, why: String) -> Self {
        Self {
            reference: reference.to_string(),
            range,
            display: String::new(),
            text: String::new(),
            trouble: Some(why),
        }
    }

    /// A citation whose ref this build cannot read.
    ///
    /// The `ref` field carries the value the document spells rather than a
    /// parsed one, which is the only honest thing to put there and is also the
    /// thing the reader needs in order to go and look at it.
    #[must_use]
    fn unreadable(text: &str, range: Option<Range>) -> Self {
        Self {
            reference: text.to_string(),
            range,
            display: String::new(),
            text: String::new(),
            trouble: Some(format!("this build cannot read the ref {text}")),
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
        .map(|one| match one {
            // A ref this build cannot read is a row saying so. It is not asked
            // about — there is nothing to ask with — and it is not skipped,
            // because the row list is what the pen zips against.
            Wanted::Unreadable { text, range } => Refreshed::unreadable(&text, range),
            Wanted::Parsed { reference, range } => match ask(&reference, range) {
                Ok(sent) => {
                    // Only when it parses, and only when it differs. A packet
                    // whose ref this build cannot read is not evidence that
                    // anything moved; and a table with a row for every citation
                    // in the document would be saying *everything moved*, which
                    // carries the same information as saying nothing.
                    if let Ok(now) = sent.packet.reference.parse::<Ref>() {
                        if now != reference {
                            moved.insert(&reference, vec![now]);
                        }
                    }
                    Refreshed::found(&reference, range, &sent)
                }
                Err(why) => Refreshed::lost(&reference, range, why),
            },
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
    use girsa_app::session::Pointing;
    use girsa_cite::CiteStyle;
    use girsa_ksav::CitationPlacement;

    /// A document citing the first se'if whole and the second in part.
    fn document() -> String {
        let sefer = shulchan_arukh();
        let whole = girsa_app::send(
            &sefer,
            &Selection::whole(sefer.segments[0].id.clone()),
            CiteStyle::HebrewFull,
            Pointing::Plain,
            girsa_app::shemos::Shemos::AsWritten,
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
            Pointing::Plain,
            girsa_app::shemos::Shemos::AsWritten,
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
            quote(
                &sefer,
                reference,
                range,
                CiteStyle::HebrewFull,
                Pointing::Plain,
                girsa_app::shemos::Shemos::AsWritten,
            )
            .map_err(|e| e.to_string())
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
    fn a_mekor_this_build_cannot_read_is_a_row_with_a_reason_in_it() {
        // This test used to assert the opposite — that such a citation is
        // dropped, because *a document is allowed to hold somebody else's
        // markup*. The premise is right and the conclusion was not: the pen
        // zips these rows against its own scan **by position**, so a dropped
        // row does not remove one citation, it moves every citation after it
        // onto the words of a different place.
        let markup = "#מראה_מקום(מקור: \"girsa:\")[כלום]";
        let rows = against(markup);
        assert_eq!(rows.len(), 1, "{rows:#?}");
        assert!(rows[0].is_trouble());
        assert!(rows[0].text.is_empty());
        assert_eq!(rows[0].reference, "girsa:");
        assert_eq!(wanted(markup).len(), 1);
    }

    #[test]
    fn one_unreadable_citation_does_not_move_the_ones_after_it() {
        // The fence for the whole finding: a malformed `מקור:` in the middle of
        // good ones. Three citations in, three rows out, and the third row is
        // still about the third citation.
        let good = document();
        let markup = format!(
            "{good}#מראה_מקום(מקור: \"girsa:\")[כלום]\n{}",
            girsa_ksav::mekor("שו\"ע או\"ח א' א'", Some(SEIF), None)
        );
        let rows = against(&markup);
        assert_eq!(
            rows.len(),
            girsa_ksav::cited_in(&markup).len(),
            "one row per citation, always: {rows:#?}"
        );
        assert_eq!(rows.len(), 4, "{rows:#?}");
        assert!(rows[2].is_trouble(), "the unreadable one: {:#?}", rows[2]);
        // The one after it is the one after it, and it came back with words.
        assert!(!rows[3].is_trouble(), "{:#?}", rows[3]);
        assert_eq!(rows[3].reference, SEIF);
        assert_eq!(rows[3].text, rows[0].text, "the same se'if, the same words");
    }

    /// The first se'if of the pretend Shulchan Arukh, which `document()` also
    /// cites first.
    const SEIF: &str = "girsa:shulchan-arukh/orach-chayim/1:1";

    #[test]
    fn the_scanner_is_the_one_both_applications_compile() {
        // Not a second reader of the format. If this ever stops agreeing with
        // `girsa_ksav::cited_in`, the pen and the library are zipping two
        // different lists by index — which is the one way this errand can
        // silently put the wrong words in somebody's document.
        //
        // Counted as well as compared, and over markup with an unreadable
        // citation in it: agreeing about the refs they can both read is not the
        // property that makes the zip sound. Agreeing about how many there are
        // is.
        let markup = format!("{}#מראה_מקום(מקור: \"girsa:\")[כלום]\n", document());
        let mine: Vec<String> = wanted(&markup)
            .iter()
            .map(|w| match w {
                Wanted::Parsed { reference, .. } => reference.to_string(),
                Wanted::Unreadable { text, .. } => text.clone(),
            })
            .collect();
        let theirs: Vec<String> = girsa_ksav::cited_in(&markup)
            .into_iter()
            .map(|c| c.reference)
            .collect();
        assert_eq!(mine, theirs);
    }
}
