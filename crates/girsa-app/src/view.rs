//! What the window is sent, in the crate that knows the facts.
//!
//! # The wire format was described four times
//!
//! 1. `girsa-app`'s own model types — Rust, checked, and sent untranslated for
//!    fourteen of them ([`crate::taxonomy::Branch`], [`crate::Companion`],
//!    [`crate::beside::Place`], `Relation`, `Session`, …).
//! 2. `app/src-tauri/src/lib.rs` — fifty-two more structs, in the shell.
//! 3. `app/src/api.ts` — fifty-nine TypeScript interfaces, hand-mirrored, and
//!    **nothing verified that they agreed**.
//! 4. `crates/girsa-app/examples/dev-fixtures.rs` — the one tool whose entire
//!    job is to emit that same JSON for the browser build, and which **could
//!    not import (2)**, because `girsa-app` cannot depend on the shell. So it
//!    rebuilt the shapes with `serde_json::json!` by hand.
//!
//! # The fourth copy had already drifted, three times
//!
//! Not hypothetically. On 6 August 2026, before this module existed:
//!
//! - `state.json` carried nine keys where the command sends fifteen. The
//!   example's own comment listed five of the six that were missing, so **the
//!   comment documenting the drift had itself drifted**.
//! - `card()` was missing `scan`, under a doc comment reading *"the same fields
//!   the shell's command sends"*.
//! - and `text-*.json` built a **second** inline copy of the same card, missing
//!   `source` and `scan`, and emitting `"era": work.era` — the raw code — where
//!   `card()` seventy lines below emitted `display::era_said(code)`. Two
//!   hand-written copies of one shape inside one 202-line file, disagreeing
//!   with each other about the value under a key they both spell the same way.
//!
//! The two shapes the example got right for free were `Branch` and `Companion`
//! — the only two commands in the shell whose return type was a `girsa-app`
//! type rather than a shell struct. The example's own behaviour is the argument
//! for this module.
//!
//! # What stays in the shell
//!
//! Not everything can move, and the ones that cannot are now the visible
//! exceptions rather than the invisible rule:
//!
//! - `FoundPage` carries `girsa_search::facets::Facets` and
//!   `girsa_search::chips::Chip`, and `girsa-app` does not depend on
//!   `girsa-search` — `reading::gap_over` takes a slice of slugs rather than a
//!   `Scope` specifically so that it need not.
//! - `Copied` carries a `clipboard_rs` handle.
//! - [`HitRow`] moves and **its constructor does not**: it reads a
//!   `girsa_search::index::Hit`. The struct is the shape; filling it from a hit
//!   is the shell's, because the hit is.
//!
//! # Not a second model
//!
//! These are the *rows a surface draws*, and where a row is a model type
//! already it stays one — `Move` carries a [`crate::beside::Place`] and a
//! `Relation` verbatim. A row exists here only where the window wants a
//! flattened, `Serialize`-shaped view of something the library holds in a
//! richer form.

use serde::{Deserialize, Serialize};

use crate::beside::Place;
use crate::display;
use crate::taxonomy::Branch;
use crate::workspace::PaneId;

/// One of your notes, as a row.
#[derive(Serialize)]
pub struct NoteRow {
    pub slug: String,
    pub name: String,
    pub title: String,
    pub opening: String,
    pub tags: Vec<String>,
    pub paragraphs: usize,
    pub edited: u64,
    /// What it is about, as segment ids.
    pub on: Vec<String>,
}

impl NoteRow {
    /// Most recently touched first, and alphabetical among notes touched in the
    /// same second.
    ///
    /// The tiebreak is the point. Two notes written in the same second — which
    /// is what happens when one errand writes both — came back in whatever
    /// order the shelf's map iterated, so the list reordered itself between
    /// two openings of the same panel with nothing having changed.
    pub fn newest_first(rows: &mut [Self]) {
        rows.sort_by(|a, b| b.edited.cmp(&a.edited).then_with(|| a.title.cmp(&b.title)));
    }

    pub fn of(note: &girsa_note::Note) -> Self {
        let opening = note
            .paras()
            .iter()
            .map(|p| p.text.as_str())
            .find(|text| !text.trim().is_empty())
            .unwrap_or_default();
        Self {
            slug: note.slug.clone(),
            name: note.name().to_string(),
            title: note.title.clone(),
            opening: opening.chars().take(120).collect(),
            tags: note.tags.clone(),
            paragraphs: note.paras().len(),
            edited: note.edited,
            on: note.on.iter().map(ToString::to_string).collect(),
        }
    }
}

/// One paragraph of a note, for editing it.
#[derive(Serialize)]
pub struct ParaRow {
    pub id: String,
    pub text: String,
}

/// One mark, and where it lands in the line as it is drawn now.
#[derive(Serialize)]
pub struct MarkRow {
    pub id: String,
    pub kind: &'static str,
    pub at: String,
    pub label: Option<String>,
    pub colour: Option<String>,
    pub was: String,
    pub tags: Vec<String>,
    /// The characters it is on, in the text the pane drew — `None` for a
    /// bookmark, and `None` with `stale` set when its words have gone.
    pub span: Option<(usize, usize)>,
    /// The words had to be looked for. Shown, because a highlight that moved
    /// is a thing a reader is entitled to know about.
    pub moved: bool,
    /// Its words are gone, or are now there twice. **Not drawn and not
    /// deleted** — reported, so it can be put right.
    pub stale: bool,
}

impl MarkRow {
    pub fn of(marked: &crate::Marked) -> Self {
        use girsa_note::mark::Placed;
        let (span, moved, stale) = match &marked.placed {
            Placed::Whole => (None, false, false),
            Placed::At { span, moved } => (Some((span.start, span.end)), *moved, false),
            Placed::Stale => (None, false, true),
        };
        Self {
            id: marked.mark.id.as_str().to_string(),
            kind: marked.mark.kind.as_str(),
            at: marked.mark.at.to_string(),
            label: marked.mark.label.clone(),
            colour: marked.mark.colour.clone(),
            was: marked.mark.was.clone(),
            tags: marked.mark.tags.clone(),
            span,
            moved,
            stale,
        }
    }
}

/// Everything of yours on one line, less the notes — those are links.
#[derive(Serialize)]
pub struct Yours {
    pub notes: Vec<NoteRow>,
    pub marks: Vec<MarkRow>,
    pub folders: Vec<String>,
}

/// One saved query, as a row.
#[derive(Serialize)]
pub struct QueryRow {
    pub name: String,
    pub typed: String,
    pub said: String,
    pub tags: Vec<String>,
}

/// One chaburah folder, as a row.
#[derive(Serialize)]
pub struct FolderRow {
    pub name: String,
    pub title: String,
    /// Its members, in the order you put them in — which is the order a shiur
    /// goes in, so it is never sorted.
    pub members: Vec<FolderMember>,
    pub tags: Vec<String>,
}

#[derive(Serialize)]
pub struct FolderMember {
    /// The member as it is written down: a segment id, `work:…` or `query:…`.
    pub key: String,
    /// What to put on the row.
    pub said: String,
    /// Where clicking it goes, for the two kinds that are places.
    pub work: Option<String>,
    pub at: Option<String>,
}

/// One tag, and how many things carry it.
#[derive(Serialize)]
pub struct TagRow {
    pub tag: String,
    pub total: usize,
    /// What carries it, by kind — **only the kinds that do**.
    ///
    /// Four named columns before, and the window turned them into a sentence
    /// with four Hebrew plurals typed into `yoursview.ts`. A fifth taggable
    /// noun was a Rust edit, a wire edit and a TypeScript edit, in a file whose
    /// whole design is that it is told what things are called.
    pub carried: Vec<CarriedRow>,
}

/// One kind of thing carrying one tag, and how many.
#[derive(Serialize)]
pub struct CarriedRow {
    /// `note`, `mark`, `query`, `collection`.
    pub kind: girsa_note::Taggable,
    pub count: usize,
    /// What that kind is called, in Hebrew, in the plural.
    pub said: String,
}

impl TagRow {
    /// One tag's row.
    #[must_use]
    pub fn of(tag: &str, tally: &girsa_note::Tally) -> Self {
        Self {
            tag: tag.to_string(),
            total: tally.total(),
            carried: tally
                .iter()
                .filter(|(_, count)| *count > 0)
                .map(|(kind, count)| CarriedRow {
                    kind,
                    count,
                    said: kind.said().to_string(),
                })
                .collect(),
        }
    }
}

/// A sefer, as the shelf lists it.
#[derive(Serialize)]
pub struct Card {
    pub slug: String,
    pub he_title: String,
    pub en_title: String,
    pub categories: Vec<String>,
    pub author: Option<String>,
    pub era: Option<String>,
    /// `sefaria`, `otzaria` or `mine`. Shown on the row: a sefer of yours
    /// should be recognisable as yours without being second-class.
    pub source: &'static str,
    /// Whether this sefer is a scan (W25). Carried on the card because the
    /// window has to know **before** it opens a pane which of the two reading
    /// modes it is opening — and because a shelf row for a scan should say so.
    pub scan: bool,
}

impl Card {
    pub fn of(work: &girsa_corpus::work::Work) -> Self {
        Self {
            slug: work.slug.clone(),
            he_title: work.he_title.clone(),
            en_title: work.en_title.clone(),
            categories: work.categories.clone(),
            author: work.author.clone(),
            era: work
                .era
                .as_deref()
                .map(|code| display::era_said(code).to_string()),
            source: work.source.as_str(),
            scan: crate::is_scan(work),
        }
    }
}

/// What came of dropping files on the window.
///
/// Both halves are reported. A file that was not read has to say so by name —
/// a drop that half-worked and said nothing is the reader believing a sefer is
/// on the shelf when it is not.
#[derive(Serialize)]
pub struct Dropped {
    pub added: Vec<Card>,
    pub refused: Vec<Refusal>,
}

#[derive(Serialize)]
pub struct Refusal {
    pub file: String,
    pub why: String,
}

/// One line of a sefer, ready to be put on the page.
#[derive(Serialize)]
pub struct Line {
    pub id: String,
    /// `דף ב.` — the address, printed the way a citation prints it. See
    /// [`crate::sending::printed_address`], which is the one formatter.
    pub address: String,
    /// **Absent for an ordinary line of prose**, which nearly every line is —
    /// the same measurement `display::Run::style` records. `pane.ts` reads a
    /// missing kind as `text`.
    #[serde(skip_serializing_if = "is_text")]
    pub kind: &'static str,
    /// The words, split by how they are set. Not a string of HTML: see
    /// [`display::runs`].
    pub runs: Vec<display::Run>,
    /// The corrections on this line (W20). Empty on all but a handful of lines
    /// in a library, so it costs nothing to send.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fixed: Vec<FixMark>,
    /// What the line says on disk, where a correction changed it. The reader
    /// can see what was printed without turning the whole sefer back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printed: Option<String>,
}

/// Whether a line is ordinary prose, and so does not need to say what it is.
fn is_text(kind: &&'static str) -> bool {
    **kind == *"text"
}

/// One correction, as the page shows it.
#[derive(Serialize)]
pub struct FixMark {
    pub id: String,
    /// `ocr` or `girsa` — a repair or a claim (spec.md §7.2).
    pub kind: &'static str,
    pub was: String,
    pub now: String,
    pub who: String,
    /// Whether it is in the words on the page, or only noted beside them.
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl FixMark {
    pub fn of(applied: &girsa_fix::Applied, is_applied: bool) -> Self {
        Self {
            id: applied.id.to_string(),
            kind: applied.kind.as_str(),
            was: applied.was.clone(),
            now: applied.now.clone(),
            who: applied.who.clone(),
            applied: is_applied,
            source: applied.source.clone(),
            note: applied.note.clone(),
        }
    }
}

/// A sefer opened into a pane — **a window of it**, and how big the sefer is.
///
/// # Why this is not the whole sefer any more
///
/// It was, and the cost was measured (`examples/measure-opening.rs`): opening
/// Mishnah Berurah handed the webview **7.7 MB of JSON** for 17,418 segments, of
/// which the pane draws four hundred. Every byte of the rest was serialized in
/// Rust, pushed across the IPC boundary, parsed by the webview and held in its
/// heap so that a reader could look at one daf.
///
/// A pane already draws a window and grows it at the edges (`pane.ts`'s `WINDOW`
/// and `STEP`); it simply had the whole sefer in hand to slice from. Now it asks
/// — `sefer_lines` for a stretch, `sefer_index_of` for a segment it has not
/// loaded — which is the same shape as everything else in this window: the
/// corpus answers questions about the corpus.
///
/// [`Text::from`] and [`Text::total`] are what make that possible without the
/// window counting anything: it knows how long the sefer is and where in it the
/// lines it has begin, and the scrollbar tells the same small lie about length
/// that a book does.
#[derive(Serialize)]
pub struct Text {
    pub work: Card,
    /// A stretch of the sefer, beginning at [`Text::from`].
    pub lines: Vec<Line>,
    /// Where that stretch begins, counted in segments from the start.
    pub from: usize,
    /// How many segments the sefer has altogether.
    pub total: usize,
    /// Whether this sefer has any nikud at all, so the window can grey out a
    /// toggle that would do nothing.
    pub has_nikud: bool,
}

/// A follower pane and where it has to go.
#[derive(Serialize)]
pub struct Move {
    pub pane: PaneId,
    pub place: Place,
    /// What relates the two seforim, so the pane can say *why* it moved — or
    /// why it did not.
    pub relation: crate::Relation,
    /// For a pane holding a **scan**, the page to turn to (W25). A scan has no
    /// lines to scroll to, so the place it goes is a page — and it is counted
    /// here rather than worked out in the window from a segment id, which
    /// would be the window deriving an address from an ordinal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
}

/// Which place a row is about — the four fields every row of results carries,
/// plus the two only some of them used to.
///
/// [`crate::Naming`] on the wire. Flattened into `HitRow` and `NearRow`, so
/// the shapes stay exactly what `api.ts` already declared and the search column
/// and the lane column beside it can no longer disagree about what a place is
/// called, where it sits, or when it was written.
#[derive(Serialize)]
pub struct AtRow {
    pub id: String,
    pub work: String,
    /// What to call the sefer, in the language the window is in (W41). One
    /// title and not two, because a row carries a name to print rather than a
    /// sefer — and which of the two names that is was decided in Rust.
    pub title: String,
    /// `58:1`. Not a citation — a mekor is `girsa_cite::cite`, which knows this
    /// work's section words, and everything that leaves the window as one goes
    /// through [`crate::sending`].
    pub address: String,
    /// `1565`, or `1488–1575`. `null` where the corpus cannot date the work.
    pub written: Option<String>,
    /// The era, in Hebrew, where the years are not known.
    pub era: Option<String>,
}

impl AtRow {
    pub fn of(at: &crate::Naming) -> Self {
        Self {
            id: at.id.to_string(),
            work: at.work.clone(),
            title: at.title.clone(),
            address: at.address.clone(),
            written: at.written.clone(),
            era: at.era.clone(),
        }
    }
}

/// One mefaresh, as the tick-list shows it.
#[derive(Serialize)]
pub struct Mefaresh {
    pub slug: String,
    pub he_title: String,
    pub en_title: String,
    /// Whether the reader has ticked it on this sefer.
    pub chosen: bool,
    /// The folder it is drawn in (W44), or absent for one drawn loose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shelf: Option<String>,
}

/// The tick-list, and which lines to mark given what is ticked.
#[derive(Serialize)]
pub struct Mefarshim {
    pub works: Vec<Mefaresh>,
    /// Seforim that keep this one's order without commenting on it: the Shulchan
    /// Arukh under the Tur, the Arukh HaShulchan under the Shulchan Arukh, the
    /// Rambam's hilchos beside the chelek of Yoreh De'ah on the same subject.
    ///
    /// Its own list because it is its own claim. These were thrown away before —
    /// not by a decision, but because the rule that found mefarshim answered a
    /// bool, and *not a commentary* and *nothing to do with this sefer* had to
    /// share the one `false`.
    ///
    /// Drawn flat, and no folders: there are two of these on Orach Chayim and
    /// four on the Tur, and a tree over four rows is what W44 already calls
    /// worse than the rows.
    pub alongside: Vec<Mefaresh>,
    /// The folders they stand in — rishonim, acharonim, and the authors with
    /// more than one sefer among them (W44). Empty when there is nothing worth
    /// grouping, and then the list is drawn flat.
    pub folders: Vec<Branch>,
    /// The segments a **ticked** mefaresh speaks on, and how many of them speak
    /// there.
    ///
    /// Only these get a marker: 2,749 of Berakhot's segments carry commentary
    /// from somebody, and a mark on nearly every line is not a mark. It was a
    /// list of segments — a bool per line — until a reader ticked a targum and
    /// got a diamond on 1,533 of Bereishis' 1,533 lines, which is the same
    /// defect one level in: taking the chosen set does not help when the sets
    /// people choose first are the ones that speak everywhere. The count is what
    /// varies where the bool cannot, and `marking` in `mefarshim.ts` decides
    /// whether even that is worth drawing.
    pub marked: std::collections::BTreeMap<String, usize>,
    /// How many segments carry commentary from anybody. For the sentence under
    /// the list, so *you have ticked nobody* does not read as *nobody wrote*.
    pub touched: usize,
    /// The list behind the door, in reading order — headings and seforim, woven
    /// once (W44).
    ///
    /// **The four arrays above are what it was woven from**, and they are kept
    /// because the picker and the tick-list each want a different one of them.
    /// What moved into Rust is the *weave*: four sections, three Hebrew
    /// headings, an ordering rule and a no-sefer-twice rule, which were 277
    /// lines of TypeScript beside a module carrying twenty-five Rust tests
    /// about this same list.
    pub listed: Vec<crate::mefarshim::Listed>,
    /// Why the list is empty when it is empty for a reason.
    ///
    /// `mefarshim::Marks::of` reads `inbound.jsonl`, and a sefer with no such
    /// file has nothing commenting on it — which is true, and is **not the same
    /// statement** as *`girsa-link-types` has never run here*. `Marks::of`'s own
    /// doc says as much and leaves it to the caller; the caller did not make it,
    /// so a corpus with no inbound cache answered *nobody comments on this
    /// sefer* about all 7,189 of them.
    pub unbuilt: Option<String>,
}

impl Mefarshim {
    /// Weave the whole tick-list for one sefer.
    ///
    /// # Why this is here and not in the shell
    ///
    /// It was thirty lines of the `mefarshim` command: read the marks, ask for
    /// the folders, name both lists the same way, work out `unbuilt`, call
    /// `mefarshim::listed`. All of it is a decision about what the door shows,
    /// none of it needs a window, and the shell was the only thing that could
    /// run it — so `dev-fixtures` could not, and the browser build answered every
    /// `mefarshim` call with an **empty list** while the door beside it said
    /// `מפרשים · 34` off the companions fixture.
    ///
    /// A button that promises thirty-four over an empty list is the exact defect
    /// this pass has been pulling out of the shell all day, sitting in the build
    /// whose whole purpose is looking at the window. One composer, two callers.
    #[must_use]
    pub fn of(
        shelf: &crate::shelf::Shelf,
        marks: &crate::mefarshim::Marks,
        slug: &str,
        chosen: &[String],
    ) -> Self {
        let commentators = marks.commentators();
        let alongside = marks.alongside();
        let works: Vec<girsa_corpus::work::Work> = commentators
            .iter()
            .filter_map(|slug| shelf.work(slug).cloned())
            .collect();
        // The folders they stand in, over the same works the list offers —
        // through `taxonomy`'s idea of a shelf, so a sefer is in one place here
        // and on the bookcase. Only the mefarshim: the seforim running alongside
        // are drawn flat, so grouping them would be a tree nobody asked for.
        let folders = crate::mefarshim::folders(&works, shelf.arrangement(), shelf.shipped());
        // One naming, both lists. Two copies would drift the day one of them
        // learns to say something the other does not.
        let named = |work: String| Mefaresh {
            he_title: shelf
                .work(&work)
                .map_or_else(|| work.clone(), |w| w.he_title.clone()),
            en_title: shelf
                .work(&work)
                .map_or_else(|| work.clone(), |w| w.en_title.clone()),
            chosen: chosen.contains(&work),
            shelf: folders.of.get(&work).cloned(),
            slug: work,
        };
        let listed = crate::mefarshim::listed(
            &shelf.companions(slug),
            &commentators,
            &alongside,
            &folders,
            chosen,
            shelf,
        );
        Self {
            marked: marks.marked(chosen),
            touched: marks.segments_touched(),
            works: commentators.into_iter().map(&named).collect(),
            alongside: alongside.into_iter().map(&named).collect(),
            folders: folders.tree,
            listed,
            // The third answer. Nothing here and no cache are not the same.
            unbuilt: (!girsa_link::inbound::built(shelf.root())).then(|| {
                "the link graph has not been walked yet — run girsa-link-types".to_string()
            }),
        }
    }
}

/// One mefaresh's words on one line.
#[derive(Serialize)]
pub struct Said {
    pub work: String,
    pub he_title: String,
    pub en_title: String,
    /// Where this is, in the commentary — what a citation would name.
    pub address: String,
    pub lines: Vec<Line>,
}

/// What the ticked mefarshim say about one line, and whether anybody else did.
#[derive(Serialize)]
pub struct Comments {
    pub said: Vec<Said>,
    /// True when something comments here that the reader has **not** ticked.
    /// *Nobody wrote about this line* and *none of the six you follow wrote
    /// about this line* are different sentences, and the window says which.
    pub others: bool,
}

/// A scan opened into a pane.
///
/// The window is given the **file** and the mapping, and draws the page itself
/// — the scan is the daf and there is nothing to typeset. What it is not given
/// is any way to work out which daf a page is: that is arithmetic on a
/// declaration, it lives in `girsa-scan`, and it is asked one page at a time.
#[derive(Serialize)]
pub struct ScanView {
    pub work: Card,
    pub pages: usize,
    /// The page to open on: where this scan was left last time (spec.md §6.1's
    /// position memory), or its first page.
    pub at: usize,
    /// The PDF itself, as a path the window turns into an `asset:` URL.
    pub file: String,
    /// Whether the once-per-sefer chore has been done. *No mapping yet* and
    /// *nothing printed on this page* are different sentences.
    pub paged: bool,
    /// The sefer this is a scan of, where the reader has said.
    pub of: Option<String>,
    pub scheme: &'static str,
    pub anchors: Vec<AnchorRow>,
    /// Why nothing here can be cited, where that is so — a scan whose sefer is
    /// not on this shelf, so far. Said rather than fallen back from.
    pub trouble: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AnchorRow {
    pub page: usize,
    /// Absent where the anchor says *these are not pages of the sefer*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// What one page of a scan is, for the header and for Ctrl+C.
#[derive(Serialize)]
pub struct PageSaid {
    pub page: usize,
    /// The whole mareh makom — `ברכות כג.`. Absent for a page the mapping does
    /// not cover, where the window says *page 3 of the file* instead of
    /// inventing a daf.
    pub display: Option<String>,
    pub reference: Option<String>,
    /// The permanent id of the page, which is what a note anchors to and what
    /// no mapping ever moves.
    pub id: Option<String>,
}

/// One glyph the window read off a page, in pixels of the page at scale 1.
#[derive(Deserialize)]
pub struct DrawnRow {
    pub text: String,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// Where a scan has got to, and what it is being read by.
#[derive(Serialize)]
pub struct ReadingRow {
    pub slug: String,
    pub pages: usize,
    pub read: usize,
    /// The next page to read, or `null` when there is none left.
    pub next: Option<usize>,
    /// The engines that have been over it. More than one is normal: a PDF can
    /// carry its own text for the pages that were typeset and none for the
    /// plates.
    pub by: Vec<String>,
    /// Whether an OCR engine is installed at all. The window offers *read the
    /// pictures* only when there is something to read them with — an offer that
    /// cannot be taken is worse than no offer (spec.md §6.3: OCR is optional).
    pub engine: Option<String>,
    /// Corrections whose ink the current reading has no word under.
    pub stranded: usize,
}

/// One word on a page, and the rectangle of the page its ink is on.
#[derive(Serialize)]
pub struct WordRow {
    pub text: String,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub confidence: f32,
}

impl WordRow {
    pub fn of(word: &girsa_scan::Word) -> Self {
        Self {
            text: word.text.clone(),
            left: word.at.left,
            top: word.at.top,
            right: word.at.right,
            bottom: word.at.bottom,
            confidence: word.confidence,
        }
    }
}

/// What is on one page, for drawing over it.
#[derive(Serialize)]
pub struct PageWordsRow {
    pub page: usize,
    pub by: Option<String>,
    pub guessed: bool,
    pub words: Vec<WordRow>,
}

/// *4 PDFs on this shelf aren't searchable yet*, and what it is about.
#[derive(Serialize)]
pub struct GapRow {
    pub said: String,
    pub pages: usize,
    pub scans: Vec<ScannedRow>,
    /// Notes written since the index was built, or `null` when there is no index
    /// at all — a different answer from zero, and the larger gap of the two.
    pub notes: Option<usize>,
    /// Corrections made since then, same convention.
    pub fixes: Option<usize>,
    /// Scans carrying word corrections the index has not seen — `Unindexed`'s
    /// third kind, which was counted in Rust and serialized nowhere, so the one
    /// gap a reader creates by *fixing* something reached no surface at all.
    pub corrected_scans: Option<usize>,
}

#[derive(Serialize)]
pub struct ScannedRow {
    pub slug: String,
    pub title: String,
    pub pages: usize,
    pub read: usize,
}

/// A correction, and the line it landed on.
#[derive(Serialize)]
pub struct Fixed {
    pub line: Line,
    /// What to say: the words, and what they now read.
    pub said: String,
}

/// Your corrections — all of them, or one sefer's.
#[derive(Serialize)]
pub struct PatchRow {
    pub id: String,
    pub segment: String,
    pub work: String,
    /// The sefer, in the window's language (W41).
    pub title: String,
    pub address: String,
    pub kind: &'static str,
    pub was: String,
    pub now: String,
    pub who: String,
    pub when: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl PatchRow {
    /// Newest first: a correction queue is read from the top, and the tiebreak
    /// is the id so that two corrections stamped in the same second do not
    /// swap places between two openings of the panel.
    pub fn newest_first(rows: &mut [Self]) {
        rows.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| a.id.cmp(&b.id)));
    }
}

/// One link, as the panel shows it.
///
/// Everything §8.3 asks a repair UI to show its work with: which end, what the
/// corpus said, what it says now, how it was found, how much to believe it, and
/// which of those were you.
#[derive(Serialize)]
pub struct LinkRow {
    /// What names this edge in your layer — handed back to repair it.
    pub edge: String,
    /// `comments-on`, `quotes`, … as it stands now.
    pub kind: &'static str,
    /// What the corpus shipped, where your layer changed it.
    pub was: Option<&'static str>,
    pub outgoing: bool,
    pub at: String,
    pub work: String,
    /// The sefer at the other end, in the window's language (W41).
    pub title: String,
    /// The first words at the other end (W37), so the row reads as a row of
    /// reading rather than a row of provenance.
    ///
    /// Absent unless that sefer is **already open**. `span_on` set this precedent
    /// and its reason holds here: the panel is not entitled to read forty seforim
    /// off the disk to decorate a list. The case where it matters most is the one
    /// where the commentary is in the column beside you anyway.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    pub address: String,
    pub said: String,
    /// `sefaria-seed`, `otzaria-seed`, `by-hand`.
    pub method: &'static str,
    pub confidence: f32,
    /// The label the corpus used, verbatim — blank for 40% of them (T5).
    pub label: String,
    pub confirmed: bool,
    pub rejected: bool,
    pub mine: bool,
    /// Which words of the line this link is about, where anything says (§8.4).
    pub span: Option<(usize, usize)>,
    /// Where that came from: `pinned` (you said) or `dibur` (the commentary
    /// says). Absent when the link is on the whole segment.
    pub span_from: Option<&'static str>,
    /// Which of the four repairs have been applied to it.
    pub changed: Vec<&'static str>,
    pub who: Option<String>,
    /// Whether this may be shown as a statement about the texts, rather than
    /// as *these two are connected somehow*.
    pub curated: bool,
}

impl LinkRow {
    pub fn of(
        link: &crate::Link,
        language: crate::session::Language,
        preview: Option<String>,
    ) -> Self {
        Self {
            edge: girsa_link::repair::name_of(
                link.repaired
                    .shipped
                    .as_ref()
                    .unwrap_or(&link.repaired.edge),
            ),
            kind: link.repaired.edge.edge_type.as_str(),
            was: link.repaired.shipped.as_ref().map(|e| e.edge_type.as_str()),
            outgoing: link.outgoing,
            at: link.other.from.to_string(),
            work: link.work.clone(),
            title: language
                .title_of(&link.he_title, &link.en_title)
                .to_string(),
            preview,
            address: link.address.clone(),
            said: link.said(),
            method: link.repaired.edge.method.as_str(),
            confidence: link.repaired.confidence(),
            label: link.repaired.edge.source_label.clone(),
            confirmed: link.repaired.confirmed,
            rejected: link.repaired.rejected,
            mine: link.repaired.mine,
            span: link.span.as_ref().map(|span| (span.start, span.end)),
            span_from: link.span.as_ref().map(|_| {
                if link.repaired.pinned.is_some() {
                    "pinned"
                } else {
                    "dibur"
                }
            }),
            changed: link.repaired.changed.clone(),
            who: link.repaired.who.clone(),
            curated: link.repaired.is_curated(),
        }
    }
}

/// What the links panel needs to draw itself.
#[derive(Serialize)]
pub struct Links {
    pub links: Vec<LinkRow>,
    /// No inbound cache, so the incoming half is missing. Said out loud: a
    /// sidebar quietly short of half its links reads as a sefer nobody comments
    /// on.
    pub incoming_unknown: bool,
    /// The types a link may be retyped to, **labelled**, in the order they are
    /// offered. From `crate::links::kinds`, which is where the Hebrew for a
    /// kind of link lives — it used to be a lookup table in `linksview.ts` with
    /// a `?? kind` fallback, so a tenth edge type printed an English slug into a
    /// Hebrew interface and said nothing.
    pub types: Vec<crate::links::Named>,
    /// Your lenses (§8.5, W24): saved filters, not hardcoded lists.
    pub lenses: Vec<LensRow>,
    /// Which one is on, if any.
    pub lens: Option<String>,
}

#[derive(Serialize)]
pub struct LensRow {
    pub key: String,
    pub title: String,
}

/// What came out, and where it went.
#[derive(Serialize)]
pub struct Written {
    pub path: String,
    pub segments: usize,
    pub corrections: usize,
    pub stale: usize,
    pub noted: usize,
    /// What to say: the file, and what is in it.
    pub said: String,
}

/// One candidate, as the queue shows it.
#[derive(Serialize)]
pub struct SuspectRow {
    pub id: String,
    pub rare: String,
    pub common: String,
    pub rare_count: u64,
    pub common_count: u64,
    /// `ד/ר`, where the letters are a pair that look alike in print.
    pub confusion: Option<String>,
    /// What the scanner did — `letter`, `added`, `dropped`, `swapped`.
    pub how: &'static str,
    /// Where to go and look: the first place, with the sefer named.
    pub at: Option<String>,
    pub work: Option<String>,
    /// The sefer, in the window's language (W41). Absent only when the
    /// candidate names no place at all — a sefer the catalogue has not caught
    /// up with is named by its slug, because a row with no name on it is a row
    /// a reader cannot act on.
    pub title: Option<String>,
    pub address: Option<String>,
}

/// Where on the page a candidate's word is, and what to put in the box.
#[derive(Serialize)]
pub struct Standing {
    pub at: String,
    pub from_char: usize,
    pub to_char: usize,
    /// The word as printed, which is what the reader is about to change.
    pub printed: String,
    /// The common spelling, where it can be given without inventing text —
    /// see [`girsa_fix::suspect::Suspect::suggestion`]. `null` on a pointed
    /// word, and then the reader types.
    pub suggestion: Option<String>,
}

/// What you are writing, and where it is kept.
#[derive(Serialize)]
pub struct Writing {
    pub name: String,
    pub text: String,
    /// The file it lives in — a `.ksav` document in your own layer, which is
    /// the whole of what "opens in real Ksav with zero conversion" means.
    pub path: String,
}

/// The whole settings surface, in one call (B13).
///
/// > *"There is no settings panel … This is a step backwards from what you are
/// > replacing."*
///
/// One command and one struct rather than a command per field: a panel that asks
/// eleven questions to draw itself is a panel that draws itself wrong once, and
/// the shortcut table has to come from `crate::keys` or the card and the keys
/// disagree.
#[derive(Serialize)]
pub struct SettingsView {
    pub pointing: crate::session::Pointing,
    pub text_size: u16,
    /// Which language the **seforim** are named in.
    pub language: crate::session::Language,
    /// And which language the **window** speaks. Two settings, because a reader
    /// asked for two: *"there should be 2 seperate commands."*
    pub interface: crate::session::Language,
    pub cite: girsa_cite::CiteStyle,
    pub showing: girsa_fix::Showing,
    pub theme: &'static str,
    pub hebrew_font: String,
    pub latin_font: String,
    pub line_height: u16,
    pub column_ch: u16,
    /// The narrowest and widest a pane may be, in tenths of a per cent.
    ///
    /// Sent because the window has to draw a drag inside them, **and it used to
    /// know them by heart**: `Math.min(85, Math.max(15, share))` in
    /// `layout.ts`, against `ratio.min(1000)` in `crate::workspace`. Two
    /// clamps, two answers, and the one that decided what a reader could
    /// actually do was the one in TypeScript.
    pub share_bounds: [u16; 2],
    /// Every shortcut, with what it is bound to now — the reader's binding where
    /// they set one, the shipped default where they did not.
    pub shortcuts: Vec<Shortcut>,
    /// The families the reader may pick from: what this machine has, as far as the
    /// window could tell us, plus what the stylesheet falls back to.
    pub fonts: Vec<String>,
}

#[derive(Serialize)]
pub struct Shortcut {
    pub id: &'static str,
    pub he: &'static str,
    pub en: &'static str,
    /// What it answers to now.
    pub bound: Option<String>,
    /// What it shipped bound to, so *reset* has something to reset to.
    pub shipped: &'static str,
}

/// Where the lane stands, as the settings panel and the search header show it.
#[derive(Serialize)]
pub struct LaneRow {
    /// `off`, `adrift` or `on`. Three states, drawn as three states.
    pub state: &'static str,
    /// The sentence for the header. `None` when the lane is off, which is not a
    /// line — there is no lane to be partial about.
    pub said: Option<String>,
    /// What the lane covers and what it does not. **Always present**, because a
    /// partial lane that reads as a complete one is what §9.9 exists to prevent.
    pub coverage: String,
    /// The model directory, as the reader set it.
    pub model: Option<String>,
    /// Whether Girsa may go and get one. False in a fresh install.
    pub may_fetch: bool,
    /// The whole library, rather than a list.
    pub everything: bool,
    /// The seforim chosen, with what is embedded of each.
    pub chosen: Vec<CoveredRow>,
    /// How many seforim on the shelf are not in the lane at all.
    pub outside: usize,
    /// Seforim whose vectors were made by another model and are not being read.
    pub other_model: Vec<String>,
    /// What `lane_bring` would fetch, with its licence — shown before the
    /// button does anything, because the terms are not Girsa's to grant.
    pub offer: ModelOffer,
}

#[derive(Serialize)]
pub struct CoveredRow {
    pub slug: String,
    pub title: String,
    pub wanted: usize,
    pub embedded: usize,
}

#[derive(Serialize)]
pub struct ModelOffer {
    pub name: &'static str,
    pub by: &'static str,
    pub licence: &'static str,
    pub about: &'static str,
    pub what: &'static str,
    pub bytes: u64,
}

/// One adjacent result.
#[derive(Serialize)]
pub struct NearRow {
    #[serde(flatten)]
    pub at: AtRow,
    pub text: String,
    pub nearness: f32,
}

/// What the lane answered. Five fields and all five are drawn.
#[derive(Serialize)]
pub struct LaneAnswer {
    /// The label these must be drawn under. From `girsa-lane`, worded once.
    pub label: &'static str,
    /// What the lane was measured to do, and at what size. From `girsa-lane`.
    pub measured: &'static str,
    pub near: Vec<NearRow>,
    pub coverage: String,
    /// Why there is nothing. Never an empty list with no reason attached.
    pub refused: Option<String>,
    /// Set when the ranking came off a signature shortlist rather than from
    /// reading every vector — `girsa_lane::SHORTLISTED`, worded once.
    pub shortlisted: Option<&'static str>,
}

/// How far a background job has got. One shape for both jobs.
#[derive(Serialize, Clone)]
pub struct LaneProgress {
    /// `bring`, `embed` or `done`.
    pub doing: &'static str,
    /// What it is working on — a file name, or a sefer's title.
    pub what: String,
    pub done: u64,
    pub of: u64,
    /// Set when the job stopped for a reason worth showing.
    pub trouble: Option<String>,
}

/// One rung, with the count clicking it will give.
#[derive(Serialize)]
pub struct OfferRow {
    pub label: String,
    pub count: usize,
    /// What to send back to apply it.
    pub rung: String,
}

/// A mareh makom: where it lands, or what it could be.
#[derive(Serialize)]
pub struct LandingRow {
    pub said: String,
    /// One entry per candidate the shelf could not rule out. **Never narrowed
    /// to one by this crate** — a choice is shown as a choice.
    pub places: Vec<PlaceRow>,
    pub near: Vec<String>,
}

#[derive(Serialize)]
pub struct PlaceRow {
    /// The place as a **person** says it — `שבת דף לא.`. Through
    /// [`crate::sending::cite_of`], the same formatter Ctrl+C uses, because the
    /// panel used to print `girsa:bavli/shabbat/31a` at a reader three times
    /// over while the copy of the same line said it properly.
    pub said: String,
    /// The ref, as the machine spells it. For the hover, and never the label.
    pub reference: String,
    pub id: String,
    pub work: String,
}

/// One hit, as a row of results.
#[derive(Serialize)]
pub struct HitRow {
    #[serde(flatten)]
    pub at: AtRow,
    /// The text as printed, cut into runs — the same shape a reading pane
    /// draws, so a result reads like the page it came from and inline markup
    /// never reaches the window as markup.
    pub runs: Vec<display::Run>,
    /// Which page of a scan this is, where it is one. The row opens the viewer
    /// at it rather than a reading pane at a line that has no words in it.
    pub page: Option<usize>,
    /// Who read the words (spec.md §9.7's badge, W26). Absent for the corpus,
    /// which was not read off anything; `embedded` where the file said what its
    /// own words are; the engine's name and version where a machine guessed.
    ///
    /// **Badge them, don't demote them** — the row is where the score put it
    /// and this is printed beside it, because OCR text is dirtier and a reader
    /// is entitled to know which kind of result is in front of them.
    pub by: Option<String>,
    /// Whether that reader was an OCR engine, worked out here so the window
    /// does not parse the name.
    pub guessed: bool,
    /// The words of this hit that answered the query.
    ///
    /// Worked out by the search's own `Marker` — a literal search marks the
    /// words it asked for, a widened one marks the word that actually answered.
    /// Carried on the row because a page of a scan is highlighted with a
    /// **rectangle on the photograph** rather than a span of text, and the
    /// window cannot work out which words those are: searching the drawn text
    /// for what the reader typed finds nothing on a menukad page, which is most
    /// of them (spec.md §9.7 — *only the highlight differs*).
    pub marked: Vec<String>,
}

/// What the window is holding when it opens, and after anything that changes
/// the shape of it.
///
/// The one row that was **not a struct at all** — fifteen keys built with
/// `serde_json::json!` in the shell, and a sixteenth hand-written copy of the
/// same shape in `dev-fixtures.rs` carrying nine of them. Six keys were missing
/// from the fixture and the fixture's own comment named five of the six, so the
/// comment documenting the drift had drifted.
#[derive(Serialize)]
pub struct Opening {
    pub workspace: crate::Workspace,
    pub pointing: crate::session::Pointing,
    pub text_size: u16,
    /// Where you were in each sefer.
    pub positions: std::collections::BTreeMap<String, girsa_corpus::segment::SegmentId>,
    /// How many seforim are on the shelf.
    pub works: usize,
    /// Something wrong the reader should be told about, or nothing.
    pub trouble: Option<String>,
    pub cite: girsa_cite::CiteStyle,
    /// Which language the seforim are named in, and which the window speaks.
    pub language: crate::session::Language,
    pub interface: crate::session::Language,
    /// The resolved shortcut table (B13), keyed by the one spelling of each
    /// combination. Sent with the state because a `keydown` handler has to
    /// decide synchronously whether to swallow the key, and cannot await.
    pub keys: std::collections::BTreeMap<String, String>,
    pub look: crate::session::Look,
    /// What a pane may be squeezed to, from [`crate::workspace`]. Sent because
    /// `layout.ts` has to draw a drag inside them and **used to know them by
    /// heart** — `Math.min(85, Math.max(15, share))` against `ratio.min(1000)`
    /// in Rust, which is two clamps with two answers.
    pub share_bounds: [u16; 2],
    /// Why the desk is not paired, or nothing.
    pub pairing: Option<String>,
    pub showing: girsa_fix::Showing,
    /// How many corrections your layer holds.
    pub fixes: usize,
    /// How many candidates are waiting in the OCR queue.
    pub suspects: usize,
}

impl Line {
    /// One line, drawn — corrections and all.
    ///
    /// The one place a line is built, because a line built two ways is a line
    /// that is corrected in the pane and printed in the search result. It was a
    /// free function in the shell, which is a fine place for it right up to the
    /// moment `dev-fixtures.rs` has to draw a line too — and it did, and it
    /// hand-rolled four of this struct's six fields instead.
    /// # The lexicon, and why only your own writing gets one
    ///
    /// `lexicon` is what turns a citation typed into a note into somewhere to
    /// go (W19). It is used **only when the work is yours** —
    /// [`girsa_corpus::work::Source::Mine`], which covers both a note and a
    /// sefer you dropped in — and that is a rule and not a shortcut.
    ///
    /// A sefer from Sefaria already has a link layer: 1.4 million edges built
    /// from `links0.csv` by somebody who had the whole corpus in front of them,
    /// drawn in the links panel and repairable there. Linkify is three narrow
    /// rules over a string. Running it over Berakhot as well would lay a weaker
    /// set of edges beside a stronger one on the same words, and the reader
    /// would have no way to tell which was which. Your own writing has no link
    /// layer at all, and no prospect of one, which is the whole reason the
    /// words have to answer for themselves.
    ///
    /// Pass `None` where there is no lexicon — `girsa-import` may not have run
    /// — and nothing is linkified, which is the right failure: a citation that
    /// is plain text is a citation you can still read.
    #[must_use]
    pub fn of(
        sefer: &crate::Open,
        segment: &girsa_corpus::import::Segment,
        pointing: crate::session::Pointing,
        style: girsa_cite::CiteStyle,
        lexicon: Option<&girsa_ref::Lexicon>,
    ) -> Self {
        let corrected = sefer.correction(&segment.id);
        // The text the pane is about to draw, and the one linkify is run over.
        // Not `segment.text`: taking the nikud out shortens the string, and a
        // citation found in the pointed text and reported against the stored
        // text lands a few letters to the left of the words it was about.
        let shown = display::pointed(&segment.text, pointing);
        let cites = match lexicon {
            Some(lexicon) if sefer.work.source == girsa_corpus::work::Source::Mine => {
                crate::linkify(lexicon, &shown)
            }
            _ => Vec::new(),
        };
        Self {
            id: segment.id.to_string(),
            // **The margin says what a citation would say.** It used to be
            // `SegmentId::address` — the id's own spelling of where it is,
            // never meant to be read by a person — so a Hebrew daf carried
            // `30b:11` and `31a:4` down its side while `girsa_cite` one call
            // away rendered the same place as `שבת דף לא. שורה א'`. One
            // formatter now: `crate::sending::printed_address`.
            address: crate::sending::printed_address(&sefer.work, &segment.id, style),
            kind: segment.kind.as_str(),
            runs: display::runs_citing(&shown, &[], &cites),
            fixed: corrected.map_or_else(Vec::new, |c| {
                c.applied
                    .iter()
                    .map(|a| FixMark::of(a, true))
                    .chain(c.noted.iter().map(|a| FixMark::of(a, false)))
                    .collect()
            }),
            printed: corrected.map(|_| {
                display::Shown::of(sefer.as_printed(&segment.id), pointing)
                    .text()
                    .to_string()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(title: &str, edited: u64) -> NoteRow {
        NoteRow {
            slug: format!("note/{title}"),
            name: title.to_string(),
            title: title.to_string(),
            opening: String::new(),
            tags: Vec::new(),
            paragraphs: 0,
            edited,
            on: Vec::new(),
        }
    }

    fn patch(id: &str, when: u64) -> PatchRow {
        PatchRow {
            id: id.to_string(),
            segment: String::new(),
            work: String::new(),
            title: String::new(),
            address: String::new(),
            kind: "text",
            was: String::new(),
            now: String::new(),
            who: String::new(),
            when,
            note: None,
            source: None,
        }
    }

    #[test]
    fn notes_are_newest_first() {
        let mut rows = vec![note("א", 10), note("ב", 30), note("ג", 20)];
        NoteRow::newest_first(&mut rows);
        assert_eq!(
            rows.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(),
            ["ב", "ג", "א"]
        );
    }

    #[test]
    fn two_notes_written_in_the_same_second_do_not_swap_places() {
        // Which is what happens when one errand writes both. Without the
        // tiebreak the order was the shelf's map iteration, and the panel
        // reordered itself between two openings with nothing having changed.
        let mut rows = vec![note("ג", 10), note("א", 10), note("ב", 10)];
        NoteRow::newest_first(&mut rows);
        let once: Vec<String> = rows.iter().map(|r| r.title.clone()).collect();
        rows.reverse();
        NoteRow::newest_first(&mut rows);
        let twice: Vec<String> = rows.iter().map(|r| r.title.clone()).collect();
        assert_eq!(once, twice);
        assert_eq!(once, ["א", "ב", "ג"]);
    }

    #[test]
    fn a_correction_queue_is_read_from_the_top() {
        let mut rows = vec![patch("a", 1), patch("b", 3), patch("c", 2)];
        PatchRow::newest_first(&mut rows);
        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["b", "c", "a"]
        );
    }

    #[test]
    fn two_corrections_stamped_in_the_same_second_hold_still() {
        let mut rows = vec![patch("c", 5), patch("a", 5), patch("b", 5)];
        PatchRow::newest_first(&mut rows);
        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
    }
}
