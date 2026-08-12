//! Sending a source into a document: what one Ctrl+C puts down.
//!
//! spec.md §10.2, BUILDER.md W15. The design target is *AirDrop between two of
//! your own devices*: no export dialog, no file, no format decision, no
//! cleanup. **The user does nothing different** — one Ctrl+C, and where it is
//! pasted decides which of the three flavours is taken.
//!
//! | Flavour | Who takes it | What it has to survive |
//! |---|---|---|
//! | `text/plain` | WhatsApp, a terminal, anything | being read by a person with no formatting at all |
//! | `text/html` | Word, an email, a browser | keeping its shape *and its direction* — a Hebrew quote pasted LTR is unreadable |
//! | `application/x-girsa-source+json` | Ksav | carrying the **ref**, so the citation stays alive |
//!
//! # Why the ref and not only the printed string
//!
//! A document that stores `שולחן ערוך, אורח חיים סימן א' סעיף א'` has a string.
//! A document that stores `girsa:shulchan-arukh/orach-chayim/1:1` has a place:
//! it can be re-printed in another style, regenerated against a corrected
//! edition (spec.md §7), or followed back to the sefer. No paste-based workflow
//! can do that, and it is the whole argument for the pairing.
//!
//! # What is sent is what was shown
//!
//! The nikud toggle, the markup the corpus carries, the part of a passage the
//! reader highlighted — all of it is settled here, against the same
//! [`crate::display`] the pane drew with. A clipboard that put down the raw
//! corpus string would paste `<big><strong>מאימתי</strong></big>` into somebody's
//! WhatsApp; one that ignored the highlight would send the whole se'if when
//! four words were wanted.

use girsa_cite::{cite, CiteStyle, Sefer};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::Work;
use girsa_ref::{Address, Ref};
use girsa_source::{Range, SourcePacket, Version};

use crate::display;
use crate::session::Pointing;
use crate::shelf::{address_of, Open};

/// What the reader highlighted.
///
/// Offsets are in **characters of the text as it was shown** — after the
/// markup came off and after the nikud toggle was applied — because that is
/// what the reader was looking at when they dragged the mouse. Counting in
/// bytes would put every offset in a Hebrew line one letter out of two on the
/// wrong side of a character, and `str::split_at` panics when it lands there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub from: SegmentId,
    pub to: SegmentId,
    /// Where the highlight starts in the first segment.
    pub from_char: usize,
    /// Where it ends in the last segment, exclusive. `None` is *to the end*.
    pub to_char: Option<usize>,
}

impl Selection {
    /// A whole segment — what a reader gets by pressing Ctrl+C with nothing
    /// selected, standing on a line.
    #[must_use]
    pub fn whole(id: SegmentId) -> Self {
        Self {
            from: id.clone(),
            to: id,
            from_char: 0,
            to_char: None,
        }
    }

    /// A run of whole segments.
    #[must_use]
    pub fn run(from: SegmentId, to: SegmentId) -> Self {
        Self {
            from,
            to,
            from_char: 0,
            to_char: None,
        }
    }
}

/// Why a selection could not be sent.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SendError {
    #[error("{0} is not in this sefer")]
    NotHere(String),
    /// A highlight of nothing. Refused rather than sent empty: a quote block
    /// with no words in it arrives in the document looking like a failure of
    /// the paste, and the reader has no way to tell which end it happened at.
    #[error("nothing is selected")]
    Empty,
    /// The sefer is here and the address is not in it. **Never rounded to the
    /// nearest one**: a quote taken from the se'if next door is exactly the
    /// silent wrongness this system exists to make impossible.
    #[error("{work} is on the shelf and has no {address}")]
    NoSuchPlace { work: String, address: String },
    /// The ref names a different sefer than the one handed over.
    ///
    /// [`Open::at`] matches on the **address** — `1:1` is `1:1` — so a ref for
    /// one work looked up in another resolves happily and comes back with the
    /// wrong sefer's words under the right sefer's citation. Every caller in
    /// this tree opens the sefer the ref names first; this is here so that the
    /// one that forgets gets an error rather than a plausible quote.
    #[error("{asked} is not {holding}")]
    NotThisSefer { asked: String, holding: String },
}

/// The three flavours, and the source they all say.
#[derive(Debug, Clone)]
pub struct Sent {
    /// Works in WhatsApp.
    pub plain: String,
    /// Keeps its shape, and its direction, in Word.
    pub html: String,
    /// The full packet. Ksav takes this one silently.
    pub packet: SourcePacket,
}

impl Sent {
    /// How the citation is printed — the same string in all three flavours.
    #[must_use]
    pub fn display(&self) -> &str {
        &self.packet.display
    }
}

/// What a citation needs to know about a sefer, from what the shelf knows.
///
/// The one place the two vocabularies meet. Everything past here is
/// [`girsa_cite`], and the app that produces a citation and the app that prints
/// it cannot disagree about what one is.
///
/// **Not because Ksav compiles this crate — it does not, and never has.** Its
/// manifests name `girsa-source`, `girsa-ksav`, `girsa-post` and `girsa-hebrew`,
/// and no `girsa-cite` at any depth. The real mechanism is stronger than the one
/// this comment used to claim: Ksav has no citation formatter at all. It prints
/// `packet.display` — a string formatted here — and asks the loopback for a
/// re-print in another style. A formatter Ksav cannot reach cannot disagree with
/// this one.
#[must_use]
pub fn about(work: &Work) -> Sefer {
    Sefer::new(work.he_title.trim(), work.en_title.trim()).with_sections(work.he_sections.clone())
}

/// Build the three flavours for a selection.
///
/// # Errors
///
/// If either end of the selection is not in this sefer, or the highlight is
/// empty.
pub fn send(
    sefer: &Open,
    selection: &Selection,
    style: CiteStyle,
    pointing: Pointing,
    note: Option<String>,
) -> Result<Sent, SendError> {
    let first = sefer
        .position_of(&selection.from)
        .ok_or_else(|| SendError::NotHere(selection.from.to_string()))?;
    let last = sefer
        .position_of(&selection.to)
        .ok_or_else(|| SendError::NotHere(selection.to.to_string()))?;
    // A highlight dragged upwards arrives with its ends the other way round.
    // Reading order is what a quote is in, so they are put back into it here
    // rather than at every later point that would have to remember.
    let (first, last, from_char, to_char) = if first <= last {
        (first, last, selection.from_char, selection.to_char)
    } else {
        (last, first, 0, None)
    };

    let mut lines: Vec<String> = Vec::new();
    for (at, segment) in sefer
        .segments
        .get(first..=last)
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let shown = shown(&segment.text, pointing);
        let head = if at == 0 { from_char } else { 0 };
        let tail = if first + at == last { to_char } else { None };
        let line = slice(&shown, head, tail);
        if !line.is_empty() {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        return Err(SendError::Empty);
    }

    let (Some(from), Some(to)) = (sefer.segments.get(first), sefer.segments.get(last)) else {
        return Err(SendError::Empty);
    };
    let work: Vec<String> = sefer.work.slug.split('/').map(str::to_string).collect();
    // A quote is a range, so a ref is a range (spec.md §4.2). `Ref::span`
    // collapses a range of one back to a point, so a one-segment quote is not
    // a second spelling of the same place.
    let reference = Ref::span(work, address_of(&from.id), address_of(&to.id));
    let text = lines.join("\n");
    let display = cite(&about(&sefer.work), &reference, style);

    // The ref names places; the range names which characters of them this
    // quote actually is. Without it, a corrected edition regenerates the
    // **whole** se'if for a reader who highlighted half of one — spec.md §7
    // and §10.2 contradicting each other at the regeneration step.
    let mut packet = SourcePacket::part(
        &reference,
        display.clone(),
        text.clone(),
        Range {
            from: from_char,
            to: to_char,
        },
    );
    // Read off the text being sent rather than off the toggle: a sefer with no
    // nikud in it sends none whatever the toggle says, and the receiving
    // document should not be told to expect any.
    packet.nikud = display::has_marks(&text);
    packet.version = provenance(&sefer.work);
    packet.note = note.clone();

    Ok(Sent {
        plain: plain_flavour(&text, &display, note.as_deref()),
        html: html_flavour(&lines, &display, &reference, note.as_deref()),
        packet,
    })
}

/// Everything a ref names, as a packet.
///
/// What the loopback's `/quote` answers (spec.md §10.6): Ksav has a ref in a
/// document and asks the library for the words again — after a correction
/// (§7), or into a fresh document. The address is turned into segments by the
/// **same index the link graph was built with**, so a quote regenerated a year
/// later is the same passage the citation always meant.
///
/// `range` is the packet's own [`Range`], handed back: the characters the
/// reader highlighted the first time. `None` — a packet written before the
/// field existed, or a ref typed by hand — regenerates the whole place, which
/// is the only honest answer when nobody recorded what was highlighted.
///
/// The range comes back **on the returned packet**, so a quote regenerated
/// twice is still the same half-se'if the reader chose. Losing it on the first
/// regeneration would make the second one whole, which is the same bug one
/// round later and much harder to see.
///
/// # Errors
///
/// If the sefer has no such address. Never the nearest thing: a quote silently
/// taken from the se'if next door is the failure this whole system is built to
/// make impossible.
pub fn quote(
    sefer: &Open,
    reference: &Ref,
    range: Option<Range>,
    style: CiteStyle,
    pointing: Pointing,
) -> Result<Sent, SendError> {
    if reference.work_slug() != sefer.work.slug {
        return Err(SendError::NotThisSefer {
            asked: reference.work_slug(),
            holding: sefer.work.slug.clone(),
        });
    }
    let missing = |address: &Address| SendError::NoSuchPlace {
        work: sefer.work.he_title.clone(),
        address: address.to_string(),
    };
    let head = sefer.at(reference.from());
    let Some(from) = head.first() else {
        return Err(missing(reference.from()));
    };
    // A span ref names two ends. `to()` was not being read at all, so a quote
    // of three se'ifim regenerated as its first se'if — the same words the
    // reader chose, minus most of them, and no error to say so.
    let tail = match reference.to() {
        Some(address) => sefer.at(address),
        None => head.clone(),
    };
    let Some(to) = tail.last() else {
        return Err(missing(reference.to().unwrap_or(reference.from())));
    };
    let range = range.unwrap_or_else(Range::all);
    send(
        sefer,
        &Selection {
            from: from.clone(),
            to: to.clone(),
            from_char: range.from,
            to_char: range.to,
        },
        style,
        pointing,
        None,
    )
}

/// A segment as it was shown: markup off, and as much pointing as the reader
/// has on.
fn shown(text: &str, pointing: Pointing) -> String {
    display::pointed(&display::plain(text), pointing)
}

/// `text[from_char..to_char]`, counted in characters and clamped.
///
/// Clamped rather than checked: an offset past the end of a line is what
/// arrives when the reader drags past the last word, and refusing to send
/// their selection because they were enthusiastic about it would be absurd.
fn slice(text: &str, from_char: usize, to_char: Option<usize>) -> String {
    let mut out: String = text
        .chars()
        .skip(from_char)
        .take(to_char.unwrap_or(usize::MAX).saturating_sub(from_char))
        .collect();
    // The corpus is full of trailing spaces around markup, and a quote that
    // starts with one is a quote block that looks indented by mistake.
    out = out.trim().to_string();
    out
}

/// Which edition this is and under what terms, carried onto the packet.
///
/// spec.md §13: it costs nothing now and it is the only thing preserving the
/// option to distribute publicly later. A quote whose provenance was dropped
/// cannot be un-dropped.
pub(crate) fn provenance(work: &Work) -> Version {
    work.version
        .as_ref()
        .map_or_else(Version::default, |v| Version {
            edition: v.edition.clone(),
            license: v.license.clone().unwrap_or_default(),
            provenance: v.provenance.clone().unwrap_or_default(),
        })
}

/// The flavour that has to survive having no formatting at all.
fn plain_flavour(text: &str, display: &str, note: Option<&str>) -> String {
    let mut out = String::with_capacity(text.len() + display.len() + 8);
    out.push_str(text);
    out.push('\n');
    out.push('(');
    out.push_str(display);
    out.push(')');
    if let Some(note) = note {
        out.push('\n');
        out.push_str(note);
    }
    out
}

/// The flavour that has to keep its shape *and its direction*.
///
/// `dir="rtl"` is on the elements rather than left to the receiving
/// application: Word decides direction from the paragraph it is pasted into,
/// so a Hebrew quote dropped into an English document comes out with its
/// punctuation at the wrong end and the lines in the wrong order unless the
/// markup says otherwise.
///
/// The citation is a link to the ref, which is what makes a mekor in a Word
/// document — or in a PDF printed from one — clickable back into the library
/// (spec.md §10.6).
fn html_flavour(lines: &[String], display: &str, reference: &Ref, note: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("<blockquote dir=\"rtl\" lang=\"he\" style=\"direction:rtl;text-align:right\">");
    for line in lines {
        out.push_str("<p dir=\"rtl\">");
        out.push_str(&crate::markup::text(line));
        out.push_str("</p>");
    }
    out.push_str("<footer dir=\"rtl\"><cite><a href=\"");
    out.push_str(&crate::markup::attr(&reference.to_string()));
    out.push_str("\">");
    out.push_str(&crate::markup::text(display));
    out.push_str("</a></cite></footer>");
    if let Some(note) = note {
        out.push_str("<p dir=\"rtl\"><small>");
        out.push_str(&crate::markup::text(note));
        out.push_str("</small></p>");
    }
    out.push_str("</blockquote>");
    out
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::pretend::{sefer, shulchan_arukh};

    #[test]
    fn a_ref_for_another_sefer_is_refused_and_not_answered() {
        // `Open::at` matches on the address, and every sefer has a `1:1`. A ref
        // for משנה ברורה looked up in שולחן ערוך used to come back with the
        // Shulchan Arukh's words under the Shulchan Arukh's citation — a quote
        // nobody asked for, printed as though somebody had.
        //
        // Found by `girsa_desk::refreshing`, whose test hands one sefer to
        // every ref in a document on purpose.
        let sefer = shulchan_arukh();
        let elsewhere: Ref = "girsa:mishnah-berurah/1:1".parse().expect("a ref");
        let refused = quote(
            &sefer,
            &elsewhere,
            None,
            CiteStyle::HebrewFull,
            Pointing::Plain,
        );
        assert!(
            matches!(refused, Err(SendError::NotThisSefer { .. })),
            "{refused:?}"
        );
    }

    fn id(n: u32) -> SegmentId {
        format!("girsa:shulchan-arukh/orach-chayim/1:{n}#{n}")
            .parse()
            .expect("a segment id")
    }

    fn sent(selection: &Selection, pointing: Pointing) -> Sent {
        send(
            &shulchan_arukh(),
            selection,
            CiteStyle::HebrewFull,
            pointing,
            None,
        )
        .expect("sends")
    }

    #[test]
    fn a_source_travels_as_its_words_and_the_place_they_are_from() {
        let one = sent(&Selection::whole(id(1)), Pointing::Full);
        assert_eq!(one.packet.text, "יִתְגַּבֵּר כָּאֲרִי לַעֲמוֹד בַּבֹּקֶר לַעֲבוֹדַת בּוֹרְאוֹ");
        assert_eq!(one.display(), "שולחן ערוך, אורח חיים סימן א' סעיף א'");
        // And the place is kept as a *place*, not only as the printed string.
        assert_eq!(
            one.packet.reference,
            "girsa:shulchan-arukh/orach-chayim/1:1"
        );
    }

    #[test]
    fn only_the_highlighted_part_travels() {
        // spec.md §10.2 — *highlight part of a passage; only that goes.* The
        // module was written first with the offsets ignored, which sends the
        // whole se'if and looks entirely reasonable until you compare it with
        // what the reader had highlighted.
        let selection = Selection {
            from: id(1),
            to: id(1),
            from_char: 0,
            to_char: Some(10),
        };
        let sent = sent(&selection, Pointing::Plain);
        assert_eq!(sent.packet.text, "יתגבר כארי");
        assert!(sent.plain.starts_with("יתגבר כארי\n("));
        assert!(sent.html.contains("<p dir=\"rtl\">יתגבר כארי</p>"));
        // The citation still names the segment: the ref is as fine-grained as
        // the corpus is addressed, and a character span is W24's to add.
        assert_eq!(sent.display(), "שולחן ערוך, אורח חיים סימן א' סעיף א'");
    }

    #[test]
    fn a_selection_across_seifim_takes_the_head_of_one_and_the_tail_of_another() {
        let selection = Selection {
            from: id(1),
            to: id(3),
            from_char: 11,
            to_char: Some(6),
        };
        let sent = sent(&selection, Pointing::Plain);
        assert_eq!(
            sent.packet.text,
            "לעמוד בבקר לעבודת בוראו\nשויתי ה' לנגדי תמיד\nהמשכים"
        );
        // A quote is a range, so the ref is one.
        assert_eq!(
            sent.packet.reference,
            "girsa:shulchan-arukh/orach-chayim/1:1-1:3"
        );
        assert_eq!(
            sent.display(),
            "שולחן ערוך, אורח חיים סימן א' סעיף א'-סימן א' סעיף ג'"
        );
    }

    #[test]
    fn a_highlight_dragged_upwards_is_read_in_reading_order() {
        let backwards = Selection::run(id(3), id(1));
        assert_eq!(
            sent(&backwards, Pointing::Plain).packet.reference,
            "girsa:shulchan-arukh/orach-chayim/1:1-1:3"
        );
    }

    #[test]
    fn what_is_sent_is_what_was_shown() {
        // The nikud toggle is a reading decision, and a quote that arrived
        // pointed in a document written without nikud is a paste the writer
        // has to go and clean up by hand — which is the cleanup this whole
        // design exists to remove.
        let with = sent(&Selection::whole(id(1)), Pointing::Full);
        assert!(with.packet.nikud);
        assert!(with.packet.text.contains('\u{05B4}'));

        let without = sent(&Selection::whole(id(1)), Pointing::Plain);
        assert!(!without.packet.nikud);
        assert_eq!(without.packet.text, "יתגבר כארי לעמוד בבקר לעבודת בוראו");
    }

    #[test]
    fn the_corpus_markup_never_reaches_the_clipboard() {
        // `<b>שויתי</b>` is a dibur hamatchil in the file. Pasted raw into
        // WhatsApp it is four characters of angle brackets.
        let sent = sent(&Selection::whole(id(2)), Pointing::Plain);
        assert_eq!(sent.packet.text, "שויתי ה' לנגדי תמיד");
        assert!(!sent.html.contains("<b>"), "{}", sent.html);
    }

    #[test]
    fn a_character_the_receiving_document_would_read_as_markup_is_escaped() {
        let sefer = sefer("x", "סֵפֶר", &[], &["א < ב & ג \"ד\""]);
        let sent = send(
            &sefer,
            &Selection::whole(sefer.segments[0].id.clone()),
            CiteStyle::HebrewShort,
            Pointing::Plain,
            None,
        )
        .expect("sends");
        assert!(
            sent.html.contains("א &lt; ב &amp; ג \"ד\""),
            "{}",
            sent.html
        );
        // And the plain flavour is exactly what it says it is.
        assert!(sent.plain.starts_with("א < ב & ג \"ד\""));
    }

    #[test]
    fn the_html_says_which_way_the_words_run() {
        // Word takes its direction from the paragraph the quote is pasted
        // into. A Hebrew quote landing in an English document without this
        // comes out with its punctuation at the wrong end.
        let sent = sent(&Selection::whole(id(1)), Pointing::Plain);
        assert!(sent.html.contains("dir=\"rtl\""));
        assert!(sent.html.contains("direction:rtl"));
    }

    #[test]
    fn the_citation_in_a_word_document_is_a_way_back_into_the_library() {
        let sent = sent(&Selection::whole(id(1)), Pointing::Plain);
        assert!(
            sent.html
                .contains("href=\"girsa:shulchan-arukh/orach-chayim/1:1\""),
            "{}",
            sent.html
        );
    }

    #[test]
    fn the_packet_survives_the_wire_as_a_place_and_not_only_as_words() {
        let sent = sent(&Selection::whole(id(1)), Pointing::Full);
        let json = sent.packet.to_json().expect("serializes");
        let back = SourcePacket::from_json(&json).expect("deserializes");
        assert_eq!(back, sent.packet);
        let reference = back.reference().expect("the ref survived");
        assert_eq!(reference.work_slug(), "shulchan-arukh/orach-chayim");
        assert_eq!(reference.from().to_string(), "1:1");
    }

    #[test]
    fn provenance_travels_with_the_quote() {
        let sent = sent(&Selection::whole(id(1)), Pointing::Plain);
        assert_eq!(sent.packet.version.license, "Public Domain");
        assert!(sent.packet.version.edition.contains("Lemberg"));
    }

    #[test]
    fn a_note_of_yours_travels_beside_the_quote_and_not_inside_it() {
        let sent = send(
            &shulchan_arukh(),
            &Selection::whole(id(1)),
            CiteStyle::HebrewShort,
            Pointing::Plain,
            Some("צריך עיון".into()),
        )
        .expect("sends");
        assert_eq!(sent.packet.note.as_deref(), Some("צריך עיון"));
        assert!(!sent.packet.text.contains("צריך עיון"));
        assert!(sent.plain.ends_with("צריך עיון"));
        assert!(sent.html.contains("<small>צריך עיון</small>"));
    }

    #[test]
    fn a_highlight_that_runs_past_the_end_of_the_line_is_clamped_rather_than_fatal() {
        // Hebrew is two bytes a letter, so an offset counted anywhere else
        // lands mid-character about half the time and `split_at` panics. These
        // are characters, and a reader who dragged past the last word gets
        // what they meant.
        let selection = Selection {
            from: id(1),
            to: id(1),
            from_char: 0,
            to_char: Some(9_999),
        };
        assert_eq!(
            sent(&selection, Pointing::Plain).packet.text,
            "יתגבר כארי לעמוד בבקר לעבודת בוראו"
        );
    }

    #[test]
    fn a_ref_can_be_asked_for_again_and_comes_back_as_the_same_passage() {
        // The loopback's `/quote`: Ksav has a ref in a document and asks the
        // library for the words. This is what makes a citation alive — the
        // quote can be regenerated against a corrected edition without
        // touching the prose (spec.md §7).
        let sefer = shulchan_arukh();
        let reference: girsa_ref::Ref = "girsa:shulchan-arukh/orach-chayim/1:2"
            .parse()
            .expect("a ref");
        let sent = quote(
            &sefer,
            &reference,
            None,
            CiteStyle::HebrewFull,
            Pointing::Plain,
        )
        .expect("quotes");
        assert_eq!(sent.packet.text, "שויתי ה' לנגדי תמיד");
        assert_eq!(sent.display(), "שולחן ערוך, אורח חיים סימן א' סעיף ב'");
    }

    #[test]
    fn half_a_seif_says_it_is_half_a_seif() {
        // The ref names the se'if. Without the range on the packet, the ref is
        // everything the receiving document is told, and there is no reading
        // of it that gets these five words back.
        let selection = Selection {
            from: id(1),
            to: id(1),
            from_char: 0,
            to_char: Some(10),
        };
        let sent = sent(&selection, Pointing::Plain);
        assert_eq!(sent.packet.text, "יתגבר כארי");
        let range = sent.packet.range.expect("a range");
        assert_eq!(range.from, 0);
        assert_eq!(range.to, Some(10));
        assert!(!range.is_all());
    }

    #[test]
    fn a_whole_line_still_says_the_whole_line() {
        let sent = sent(&Selection::whole(id(1)), Pointing::Plain);
        assert!(sent.packet.range.expect("a range").is_all());
    }

    #[test]
    fn the_half_seif_regenerates_as_half_a_seif() {
        // The promise this whole field exists for: the same words back after a
        // correction, not the whole place around them. Before the range, the
        // second assertion here was the first line entire.
        let sefer = shulchan_arukh();
        let selection = Selection {
            from: id(1),
            to: id(1),
            from_char: 0,
            to_char: Some(10),
        };
        let first = sent(&selection, Pointing::Plain);
        let reference = first.packet.reference().expect("a ref");
        let again = quote(
            &sefer,
            &reference,
            first.packet.range,
            CiteStyle::HebrewFull,
            Pointing::Plain,
        )
        .expect("quotes");
        assert_eq!(again.packet.text, first.packet.text);
        // And it can be asked a third time: the range came back on the packet.
        assert_eq!(again.packet.range, first.packet.range);
    }

    #[test]
    fn a_quote_with_no_range_recorded_comes_back_whole() {
        // A packet written by a Girsa older than the field. Whole is the only
        // honest answer — nobody knows what was highlighted — and it must not
        // be an error.
        let sefer = shulchan_arukh();
        let reference: girsa_ref::Ref = "girsa:shulchan-arukh/orach-chayim/1:1"
            .parse()
            .expect("a ref");
        let sent = quote(
            &sefer,
            &reference,
            None,
            CiteStyle::HebrewFull,
            Pointing::Plain,
        )
        .expect("quotes");
        assert_eq!(sent.packet.text, "יתגבר כארי לעמוד בבקר לעבודת בוראו");
    }

    #[test]
    fn a_span_regenerates_both_of_its_ends() {
        // `Ref::to()` was not read at all: a quote of two se'ifim came back as
        // its first se'if, with no error to say the rest had been dropped.
        let sefer = shulchan_arukh();
        let whole = sent(&Selection::run(id(1), id(2)), Pointing::Plain);
        let reference = whole.packet.reference().expect("a ref");
        assert!(reference.is_span());
        let again = quote(
            &sefer,
            &reference,
            None,
            CiteStyle::HebrewFull,
            Pointing::Plain,
        )
        .expect("quotes");
        assert_eq!(again.packet.text, whole.packet.text);
        assert!(again.packet.text.contains(char::from(10)));
    }

    #[test]
    fn a_span_whose_far_end_is_missing_is_refused_and_not_shortened() {
        let sefer = shulchan_arukh();
        let reference: girsa_ref::Ref = "girsa:shulchan-arukh/orach-chayim/1:1-1:99"
            .parse()
            .expect("a ref");
        assert!(matches!(
            quote(
                &sefer,
                &reference,
                None,
                CiteStyle::HebrewFull,
                Pointing::Plain
            ),
            Err(SendError::NoSuchPlace { .. })
        ));
    }

    #[test]
    fn an_address_the_sefer_does_not_have_is_refused_and_not_rounded() {
        let sefer = shulchan_arukh();
        let reference: girsa_ref::Ref = "girsa:shulchan-arukh/orach-chayim/1:99"
            .parse()
            .expect("a ref");
        assert!(matches!(
            quote(
                &sefer,
                &reference,
                None,
                CiteStyle::HebrewFull,
                Pointing::Plain
            ),
            Err(SendError::NoSuchPlace { .. })
        ));
    }

    #[test]
    fn a_highlight_of_nothing_is_refused_rather_than_sent_empty() {
        let selection = Selection {
            from: id(1),
            to: id(1),
            from_char: 4,
            to_char: Some(4),
        };
        assert_eq!(
            send(
                &shulchan_arukh(),
                &selection,
                CiteStyle::HebrewShort,
                Pointing::Plain,
                None
            )
            .err(),
            Some(SendError::Empty)
        );
    }

    #[test]
    fn a_segment_from_another_sefer_is_refused_rather_than_guessed_at() {
        let stranger: SegmentId = "girsa:bavli/berakhot/2a:1#1".parse().expect("an id");
        assert_eq!(
            send(
                &shulchan_arukh(),
                &Selection::whole(stranger),
                CiteStyle::HebrewShort,
                Pointing::Plain,
                None
            )
            .err(),
            Some(SendError::NotHere("girsa:bavli/berakhot/2a:1#1".into()))
        );
    }

    #[test]
    fn the_three_flavours_are_three_ways_of_saying_one_thing() {
        // The promise of the layered clipboard: paste anywhere and get
        // something sane, paste into Ksav and get everything.
        let sent = sent(&Selection::whole(id(1)), Pointing::Plain);
        for flavour in [&sent.plain, &sent.html] {
            assert!(flavour.contains("יתגבר כארי"), "{flavour}");
            assert!(flavour.contains(sent.display()), "{flavour}");
        }
        assert!(sent.packet.text.contains("יתגבר כארי"));
        assert_eq!(sent.packet.display, sent.display());
    }
}
