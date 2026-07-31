//! The window, and the twelve commands behind it.
//!
//! Everything here is adapter: it holds the open seforim, forwards a question
//! to [`girsa_app`], and hands back JSON. **Nothing is decided in this crate**
//! — where a pane lands, what may sit beside what, what the nikud toggle takes
//! off — because none of that can be tested in a webview and all of it can be
//! tested one directory up.
//!
//! # Where the corpus is
//!
//! The shelf is looked for at `GIRSA_CORPUS`, then `corpus/` beside the
//! executable, then `../../corpus` for a dev build. If it is not found the
//! window still opens and says so, rather than failing to start with a message
//! only a terminal would show.
//!
//! Your own layer — the arrangement of the shelf, and the seforim you dropped
//! in — is at `GIRSA_PERSONAL`, else `personal/` in the app's data directory.
//! It is **never** under the corpus root: the corpus is re-downloadable and
//! this is not.

mod clipboard;
mod post;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use girsa_app::shelf::{Companion, Open};
use girsa_app::taxonomy::Branch;
use girsa_app::workspace::{Axis, PaneId};
use girsa_app::{display, Beside, Place, Session, Shelf, Workspace};
use girsa_corpus::segment::SegmentId;
use girsa_search::bar::{Answer, Bar};
use girsa_search::chips::{Chip, Chips, Sounding};
use girsa_search::facets::{self, Dimension, Facets, Row};
use girsa_search::index::{Paging, SearchIndex};
use girsa_search::torat_emet::{Match, Together};
use girsa_search::Mode;
use serde::{Deserialize, Serialize};

/// How many seforim are kept in memory at once.
///
/// A masechta with its commentaries is four or five; the number is small
/// because a work is tens of megabytes of text and a reader has a handful open,
/// not a library.
const KEEP_OPEN: usize = 12;

pub(crate) struct State {
    pub(crate) shelf: Option<Shelf>,
    /// The search bar, if there is an index to search. Kept beside the shelf
    /// rather than inside it because an index is a rebuildable cache and a
    /// shelf is not: a window with no index still reads seforim, and says why
    /// it cannot search rather than returning nothing.
    bar: Option<Bar>,
    /// Why there is no search, if there is none.
    no_search: Option<String>,
    /// The chip row as it stands (spec.md §9.5). Held here, not in the webview,
    /// so that what the chips say and what the engine does cannot drift.
    chips: Chips,
    /// Why the shelf is not there, if it is not. Shown in the window.
    trouble: Option<String>,
    pub(crate) session: Session,
    session_path: PathBuf,
    /// The loopback desk (W16). Held because dropping it withdraws the endpoint
    /// file — which is exactly how presence stops being reported the moment
    /// this application stops.
    desk: Option<girsa_post::desk::Desk>,
    /// Why there is no pairing, if there is none. Shown beside the presence
    /// chip: a Ksav button that quietly does nothing is worse than one that
    /// says what is wrong.
    no_post: Option<String>,
    /// The resolver's lexicon, for linkify (W19). Read once: it is 24,731
    /// spellings and a citation is looked up per word of prose.
    pub(crate) lexicon: Option<girsa_ref::Lexicon>,
    /// The OCR queue, once it has been looked at (W21). Written by
    /// `girsa-suspects`, which is a batch job outside this window, so it is
    /// re-read whenever the drawer is opened rather than held as truth.
    queue: Option<girsa_fix::suspect::Queue>,
    /// The semantic lane (spec.md §9.9, W30). Held open like the index, because
    /// turning it on loads a model — and **off costs nothing**, which is what
    /// makes off-by-default a real default rather than a checkbox with a price.
    lane: Option<girsa_app::Adjacency>,
    /// Set to stop the embedding job. It is checked between batches, so
    /// stopping costs the batch in flight and nothing else.
    stop_embedding: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Slug → the sefer, read once. Cleared oldest-first.
    open: HashMap<String, Open>,
    order: Vec<String>,
    /// Slug → which mefarshim speak on which of its lines (W43).
    ///
    /// Held because ticking a box asks again, and a reader ticking six of them
    /// should not pay six reads of a 3.4 MB file for an answer that cannot have
    /// changed. One `Marks` is the whole sefer's answer; the read that builds it
    /// is 0.07s for Berakhot.
    marks: HashMap<String, girsa_app::mefarshim::Marks>,
}

impl State {
    pub(crate) fn sefer(&mut self, slug: &str) -> Result<&Open, String> {
        if !self.open.contains_key(slug) {
            let shelf = self.shelf.as_ref().ok_or_else(|| self.trouble())?;
            let read = shelf.read(slug).map_err(|e| e.to_string())?;
            if self.order.len() >= KEEP_OPEN {
                if let Some(oldest) = self.order.first().cloned() {
                    self.order.remove(0);
                    self.open.remove(&oldest);
                }
            }
            self.order.push(slug.to_string());
            self.open.insert(slug.to_string(), read);
        }
        self.open.get(slug).ok_or_else(|| "not open".to_string())
    }

    /// Which mefarshim speak on which line of one sefer, read once.
    fn marks(&mut self, slug: &str) -> Result<&girsa_app::mefarshim::Marks, String> {
        if !self.marks.contains_key(slug) {
            let trouble = self.trouble();
            let shelf = self.shelf.as_ref().ok_or(trouble)?;
            let read =
                girsa_app::mefarshim::Marks::of(shelf, slug).map_err(|e| e.to_string())?;
            self.marks.insert(slug.to_string(), read);
        }
        self.marks
            .get(slug)
            .ok_or_else(|| "no mefarshim read".to_string())
    }

    /// Forget a sefer we are holding, so the next read of it picks up a
    /// correction. Cheaper than it looks — it is one work, and the reader is
    /// standing in it.
    fn reread(&mut self, slug: &str) {
        self.open.remove(slug);
        self.order.retain(|held| held != slug);
    }

    /// Forget all of them, which is what *show as printed* costs.
    fn reread_everything(&mut self) {
        self.open.clear();
        self.order.clear();
    }

    fn trouble(&self) -> String {
        self.trouble
            .clone()
            .unwrap_or_else(|| "there is no shelf here".to_string())
    }

    fn no_search(&self) -> String {
        self.no_search
            .clone()
            .unwrap_or_else(|| "there is no index here".to_string())
    }

    fn save(&self) {
        // A preference file that will not write is not a reason to stop
        // reading. It is a reason to say so once, on the terminal.
        if let Err(e) = self.session.save(&self.session_path) {
            eprintln!("could not save the session: {e}");
        }
    }
}

pub(crate) type Shared = Mutex<State>;

/// A sefer, as the shelf lists it.
#[derive(Serialize)]
struct Card {
    slug: String,
    he_title: String,
    en_title: String,
    categories: Vec<String>,
    author: Option<String>,
    era: Option<String>,
    /// `sefaria`, `otzaria` or `mine`. Shown on the row: a sefer of yours
    /// should be recognisable as yours without being second-class.
    source: &'static str,
    /// Whether this sefer is a scan (W25). Carried on the card because the
    /// window has to know **before** it opens a pane which of the two reading
    /// modes it is opening — and because a shelf row for a scan should say so.
    scan: bool,
}

impl Card {
    fn of(work: &girsa_corpus::work::Work) -> Self {
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
            scan: girsa_app::is_scan(work),
        }
    }
}

/// What came of dropping files on the window.
///
/// Both halves are reported. A file that was not read has to say so by name —
/// a drop that half-worked and said nothing is the reader believing a sefer is
/// on the shelf when it is not.
#[derive(Serialize)]
struct Dropped {
    added: Vec<Card>,
    refused: Vec<Refusal>,
}

#[derive(Serialize)]
struct Refusal {
    file: String,
    why: String,
}

/// One line of a sefer, ready to be put on the page.
#[derive(Serialize)]
struct Line {
    id: String,
    /// `2a:1` — what the address says, for the margin.
    address: String,
    kind: &'static str,
    /// The words, split by how they are set. Not a string of HTML: see
    /// [`display::runs`].
    runs: Vec<display::Run>,
    /// The corrections on this line (W20). Empty on all but a handful of lines
    /// in a library, so it costs nothing to send.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fixed: Vec<FixMark>,
    /// What the line says on disk, where a correction changed it. The reader
    /// can see what was printed without turning the whole sefer back.
    #[serde(skip_serializing_if = "Option::is_none")]
    printed: Option<String>,
}

/// One correction, as the page shows it.
#[derive(Serialize)]
struct FixMark {
    id: String,
    /// `ocr` or `girsa` — a repair or a claim (spec.md §7.2).
    kind: &'static str,
    was: String,
    now: String,
    who: String,
    /// Whether it is in the words on the page, or only noted beside them.
    applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

impl FixMark {
    fn of(applied: &girsa_fix::Applied, is_applied: bool) -> Self {
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

/// One line, drawn — corrections and all.
///
/// The one place a line is built, because a line built two ways is a line that
/// is corrected in the pane and printed in the search result.
fn line_of(sefer: &Open, segment: &girsa_corpus::import::Segment, nikud: bool) -> Line {
    let corrected = sefer.correction(&segment.id);
    Line {
        id: segment.id.to_string(),
        address: segment.id.path().join(":"),
        kind: segment.kind.as_str(),
        runs: display::runs(&if nikud {
            segment.text.clone()
        } else {
            display::without_marks(&segment.text)
        }),
        fixed: corrected.map_or_else(Vec::new, |c| {
            c.applied
                .iter()
                .map(|a| FixMark::of(a, true))
                .chain(c.noted.iter().map(|a| FixMark::of(a, false)))
                .collect()
        }),
        printed: corrected.map(|_| {
            display::Shown::of(sefer.as_printed(&segment.id), nikud)
                .text()
                .to_string()
        }),
    }
}

/// A sefer opened into a pane.
#[derive(Serialize)]
struct Text {
    work: Card,
    lines: Vec<Line>,
    /// Whether this sefer has any nikud at all, so the window can grey out a
    /// toggle that would do nothing.
    has_nikud: bool,
}

/// A follower pane and where it has to go.
#[derive(Serialize)]
struct Move {
    pane: PaneId,
    place: Place,
    /// What relates the two seforim, so the pane can say *why* it moved — or
    /// why it did not.
    relation: girsa_app::Relation,
    /// For a pane holding a **scan**, the page to turn to (W25). A scan has no
    /// lines to scroll to, so the place it goes is a page — and it is counted
    /// here rather than worked out in the window from a segment id, which
    /// would be the window deriving an address from an ordinal.
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<usize>,
}

#[tauri::command]
fn state(shared: tauri::State<'_, Shared>) -> Result<serde_json::Value, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    // The queue is 28,124 lines on the real corpus and this is asked on every
    // redraw, so it is read once and held. `suspects` re-reads it, which is
    // where a run of the batch job is noticed.
    if state.queue.is_none() {
        if let Some(personal) = state.shelf.as_ref().map(|s| s.personal().to_path_buf()) {
            state.queue = Some(girsa_fix::suspect::Queue::open(&personal).0);
        }
    }
    Ok(serde_json::json!({
        "workspace": state.session.workspace,
        "nikud": state.session.nikud,
        "text_size": state.session.text_size,
        "positions": state.session.positions,
        "works": state.shelf.as_ref().map_or(0, |s| s.works().len()),
        "trouble": state.trouble,
        "cite": state.session.cite,
        "pairing": state.no_post,
        "showing": state.session.showing,
        "fixes": state.shelf.as_ref().map_or(0, |s| s.fixes().count()),
        "suspects": state.queue.as_ref().map_or(0, girsa_fix::suspect::Queue::waiting),
    }))
}

#[tauri::command]
fn search(shared: tauri::State<'_, Shared>, query: String) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf.search(&query, 40).into_iter().map(Card::of).collect())
}

/// The seforim a reader has been in, most recent first — what the picker shows
/// before anything has been typed.
#[tauri::command]
fn recent(shared: tauri::State<'_, Shared>) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let Some(shelf) = state.shelf.as_ref() else {
        return Ok(Vec::new());
    };
    Ok(state
        .session
        .positions
        .keys()
        .rev()
        .filter_map(|slug| shelf.work(slug))
        .map(Card::of)
        .take(12)
        .collect())
}

// ── Searching (spec.md §9, BUILDER.md W14) ──────────────────────────────────

/// One hit, as a row of results.
#[derive(Serialize)]
struct HitRow {
    id: String,
    address: String,
    work: String,
    he_title: String,
    /// The text as printed, cut into runs — the same shape a reading pane
    /// draws, so a result reads like the page it came from and inline markup
    /// never reaches the window as markup.
    runs: Vec<display::Run>,
    /// Which page of a scan this is, where it is one. The row opens the viewer
    /// at it rather than a reading pane at a line that has no words in it.
    page: Option<usize>,
    /// Who read the words (spec.md §9.7's badge, W26). Absent for the corpus,
    /// which was not read off anything; `embedded` where the file said what its
    /// own words are; the engine's name and version where a machine guessed.
    ///
    /// **Badge them, don't demote them** — the row is where the score put it
    /// and this is printed beside it, because OCR text is dirtier and a reader
    /// is entitled to know which kind of result is in front of them.
    by: Option<String>,
    /// Whether that reader was an OCR engine, worked out here so the window
    /// does not parse the name.
    guessed: bool,
    /// The words of this hit that answered the query.
    ///
    /// Worked out by the search's own `Marker` — a literal search marks the
    /// words it asked for, a widened one marks the word that actually answered.
    /// Carried on the row because a page of a scan is highlighted with a
    /// **rectangle on the photograph** rather than a span of text, and the
    /// window cannot work out which words those are: searching the drawn text
    /// for what the reader typed finds nothing on a menukad page, which is most
    /// of them (spec.md §9.7 — *only the highlight differs*).
    marked: Vec<String>,
}

/// The words a hit matched, sliced out of its own text.
fn marked(marker: &girsa_search::bar::Marker, hit: &girsa_search::index::Hit) -> Vec<String> {
    marker
        .marks(hit)
        .into_iter()
        .filter_map(|(from, to)| hit.text.get(from..to).map(ToString::to_string))
        .collect()
}

/// The badge and the page number of a hit, in one place — the two rows of
/// `HitRow` that are about scans.
fn scanned(hit: &girsa_search::index::Hit) -> (Option<usize>, Option<String>, bool) {
    (
        hit.is_a_page()
            .then(|| hit.id.path().last().and_then(|p| p.parse().ok()))
            .flatten(),
        hit.by.as_ref().map(|by| by.name().to_string()),
        hit.is_scanned(),
    )
}

/// What one search has to say.
#[derive(Serialize)]
struct FoundPage {
    /// What was searched for, read off the query that ran.
    header: String,
    /// What the mode did, where that is worth announcing.
    note: Option<String>,
    hits: Vec<HitRow>,
    total: usize,
    page: usize,
    pages: usize,
    facets: Option<Facets>,
    /// The chip row, as it stands after any sigils were read.
    chips: Vec<Chip>,
    /// The relaxation ladder, priced and not applied (spec.md §9.6).
    offers: Vec<OfferRow>,
    /// A refusal, in the words the engine gave.
    refused: Option<String>,
    /// A citation, when the mode was Citation.
    landing: Option<LandingRow>,
}

/// One rung, with the count clicking it will give.
#[derive(Serialize)]
struct OfferRow {
    label: String,
    count: usize,
    /// What to send back to apply it.
    rung: String,
}

/// A mareh makom: where it lands, or what it could be.
#[derive(Serialize)]
struct LandingRow {
    said: String,
    /// One entry per candidate the shelf could not rule out. **Never narrowed
    /// to one by this crate** — a choice is shown as a choice.
    places: Vec<PlaceRow>,
    near: Vec<String>,
}

#[derive(Serialize)]
struct PlaceRow {
    reference: String,
    id: String,
    work: String,
}

/// Search, and hand back everything the panel draws.
///
/// The chips are read from what was typed first (a sigil flips a chip — §9.5),
/// so the row that comes back is the row the search actually ran under.
#[tauri::command]
fn find(shared: tauri::State<'_, Shared>, query: String, page: usize) -> Result<FoundPage, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let size = PAGE;
    let paging = Paging {
        from: size * page.saturating_sub(1).min(usize::MAX / size.max(1)),
        size,
    };
    // A sigil sets a chip, and the chip stays set — that is what makes typing
    // one a way of *finding* the chips rather than a syntax beside them.
    let (chips, _) = state.chips.read(&query);
    state.chips = chips;
    let nikud = state.session.nikud;
    let chips = state.chips.clone();
    let Some(bar) = state.bar.as_ref() else {
        let why = state.no_search();
        return Ok(FoundPage::refused(&chips, why));
    };

    let answer = bar.ask(
        &query,
        &chips,
        paging,
        &girsa_ref::resolve::Context::default(),
    );
    Ok(match answer {
        Answer::Segments {
            results,
            offers,
            note,
        } => {
            let pages = results.total.div_ceil(size.max(1));
            FoundPage {
                header: results.header.clone(),
                note,
                hits: results
                    .hits
                    .iter()
                    .map(|hit| HitRow {
                        id: hit.id.to_string(),
                        address: hit.id.path().join(":"),
                        work: hit.id.work().to_string(),
                        he_title: bar
                            .catalogue()
                            .facts(hit.id.work())
                            .map_or_else(|| hit.id.work().to_string(), |f| f.title.clone()),
                        runs: display::runs(&if nikud {
                            hit.text.clone()
                        } else {
                            display::without_marks(&hit.text)
                        }),
                        page: scanned(hit).0,
                        by: scanned(hit).1,
                        guessed: scanned(hit).2,
                        marked: marked(&results.marker, hit),
                    })
                    .collect(),
                total: results.total,
                page: page.max(1),
                pages,
                facets: Some(results.facets.clone()),
                chips: chips.row(),
                offers: offers
                    .offers
                    .iter()
                    .map(|offer| OfferRow {
                        label: offer.label.to_string(),
                        count: offer.count,
                        rung: offer.rung.name().to_string(),
                    })
                    .collect(),
                refused: None,
                landing: None,
            }
        }
        Answer::Cited(landing) => FoundPage {
            header: landing.describe(),
            note: None,
            hits: Vec::new(),
            total: landing.places.len(),
            page: 1,
            pages: 1,
            facets: None,
            chips: chips.row(),
            offers: Vec::new(),
            refused: None,
            landing: Some(LandingRow {
                said: landing.describe(),
                places: landing
                    .places
                    .iter()
                    .map(|place| PlaceRow {
                        reference: place.reference.to_string(),
                        id: place.run.first.to_string(),
                        work: place.run.first.work().to_string(),
                    })
                    .collect(),
                near: landing.near.iter().map(near_said).collect(),
            }),
        },
        Answer::Refused(why) => FoundPage::refused(&chips, why),
    })
}

impl FoundPage {
    /// A refusal is an answer, and it says why. What it never is, is a shorter
    /// list of results with nothing attached.
    fn refused(chips: &Chips, why: String) -> Self {
        Self {
            header: String::new(),
            note: None,
            hits: Vec::new(),
            total: 0,
            page: 1,
            pages: 0,
            facets: None,
            chips: chips.row(),
            offers: Vec::new(),
            refused: Some(why),
            landing: None,
        }
    }
}

fn near_said(near: &girsa_search::citation::NearMiss) -> String {
    use girsa_search::citation::NearMiss;
    match near {
        NearMiss::AddressNotThere { reference, work } => {
            format!("{work} is here, and has no {}", reference.from())
        }
        NearMiss::NotOnTheShelf { work, .. } => {
            format!("{work} would answer it, and is not on this shelf")
        }
        NearMiss::OtherTitle { spelling, .. } => spelling.clone(),
    }
}

/// Apply one rung of the ladder — the click on an offer (spec.md §9.6).
///
/// A search of its own, and it reports itself: the header that comes back is
/// [`girsa_search::ladder::Widening::describe`], which is read off the query
/// that ran. The undo is not a flag but the literal query, which is what
/// `find` does without a rung.
#[tauri::command]
fn find_rung(
    shared: tauri::State<'_, Shared>,
    query: String,
    page: usize,
    rung: String,
) -> Result<FoundPage, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let (chips, text) = state.chips.read(&query);
    state.chips = chips.clone();
    let nikud = state.session.nikud;
    let Some(rung) = girsa_search::ladder::Rung::named(&rung) else {
        return Err(format!("no such rung: {rung}"));
    };
    let Some(bar) = state.bar.as_ref() else {
        let why = state.no_search();
        return Ok(FoundPage::refused(&chips, why));
    };
    let query = girsa_search::torat_emet::Query::new(text)
        .matching(chips.matching)
        .together(chips.together);
    let widened = girsa_search::ladder::Widened::new(query, [rung]);
    let paging = Paging {
        from: PAGE * page.saturating_sub(1),
        size: PAGE,
    };
    let found = bar
        .index()
        .search_widened_in(&widened, &chips.scope, paging)
        .map_err(|e| e.to_string())?;
    let header = found
        .widening
        .as_ref()
        .map_or_else(|| found.asked.describe(), |w| w.describe());
    // What actually answered: the widened form where a rung was applied, and
    // the words as typed where the search ran as it stood.
    let marker = found.widening.clone().map_or_else(
        || girsa_search::bar::Marker::Literal(found.asked.clone()),
        |widening| girsa_search::bar::Marker::Widened(Box::new(widening)),
    );
    Ok(FoundPage {
        header,
        note: Some("החל — לחזרה, חפש שוב בלי הצעה".to_string()),
        hits: found
            .hits
            .iter()
            .map(|hit| HitRow {
                id: hit.id.to_string(),
                address: hit.id.path().join(":"),
                work: hit.id.work().to_string(),
                he_title: bar
                    .catalogue()
                    .facts(hit.id.work())
                    .map_or_else(|| hit.id.work().to_string(), |f| f.title.clone()),
                runs: display::runs(&if nikud {
                    hit.text.clone()
                } else {
                    display::without_marks(&hit.text)
                }),
                page: scanned(hit).0,
                by: scanned(hit).1,
                guessed: scanned(hit).2,
                marked: marked(&marker, hit),
            })
            .collect(),
        total: found.total,
        page: page.max(1),
        pages: found.total.div_ceil(PAGE),
        facets: None,
        chips: chips.row(),
        offers: Vec::new(),
        refused: None,
        landing: None,
    })
}

/// Set one chip.
///
/// Named rather than free-form: the window may choose among the options the
/// engine offered and may not invent one.
#[tauri::command]
fn find_chip(shared: tauri::State<'_, Shared>, chip: String, key: String) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    set_chip(&mut state.chips, &chip, &key)
}

/// One chip, set by the key the row itself sends.
///
/// Factored out because a saved query (W27) is replayed by setting the chips it
/// was saved with, and a second copy of this mapping would let a recalled query
/// and a clicked chip mean different things.
fn set_chip(chips: &mut Chips, chip: &str, key: &str) -> Result<(), String> {
    match chip {
        "mode" => {
            chips.mode = match key {
                "Smart" => Mode::Smart,
                "Regex" => Mode::Regex,
                "Citation" => Mode::Citation,
                "Instruments" => Mode::Instruments,
                _ => Mode::ToratEmet,
            }
        }
        "the word" => {
            chips.matching = match key {
                "Contains" => Match::Contains,
                "Letters" => Match::Letters,
                _ => Match::Word,
            }
        }
        "together" => {
            chips.together = match key {
                "Phrase" => Together::Phrase,
                other => match other.strip_prefix("Near").and_then(|n| n.parse().ok()) {
                    Some(words) => Together::Near { words },
                    None => Together::Anywhere,
                },
            }
        }
        "instrument" => {
            chips.sounding = match key {
                "Rashei" => Sounding::Rashei,
                "Sofei" => Sounding::Sofei,
                "Atbash" => Sounding::Atbash,
                "Dilug" => Sounding::Dilug,
                _ => Sounding::Gematria,
            }
        }
        other => return Err(format!("no such chip: {other}")),
    }
    Ok(())
}

/// Click a facet row: narrow to it, or rule it out (spec.md §9.8).
#[tauri::command]
fn find_narrow(
    shared: tauri::State<'_, Shared>,
    dimension: Dimension,
    row: Row,
    exclude: bool,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let Some(bar) = state.bar.as_ref() else {
        return Err(state.no_search());
    };
    let scope = if exclude {
        facets::exclude(&state.chips.scope, bar.catalogue(), dimension, &row)
    } else {
        facets::narrow(&state.chips.scope, bar.catalogue(), dimension, &row)
    };
    state.chips.scope = scope;
    Ok(())
}

/// Back to the whole shelf.
#[tauri::command]
fn find_whole_shelf(shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.chips.scope = girsa_search::scope::Scope::everything();
    Ok(())
}

/// How many results to a page.
const PAGE: usize = 25;

/// The lexicon `girsa-import` wrote, both halves of it.
///
/// Without it linkify finds nothing — which is the right failure: a citation
/// this build cannot resolve is a citation it must not link.
fn read_lexicon(corpus: &std::path::Path) -> Option<girsa_ref::Lexicon> {
    let mut body = std::fs::read_to_string(corpus.join("lexicon.tsv")).ok()?;
    if let Ok(more) = std::fs::read_to_string(corpus.join("lexicon-otzaria.tsv")) {
        body.push('\n');
        body.push_str(&more);
    }
    Some(girsa_ref::Lexicon::from_tsv(&body))
}

/// Where the index is, if it is anywhere.
///
/// `GIRSA_INDEX`, else beside the corpus. An index is a rebuildable cache
/// (spec.md §4.1) and a window without one still reads: what it must not do is
/// look like a library with nothing in it, so the reason is carried through to
/// the search panel and shown there.
fn find_index(corpus: &std::path::Path) -> Result<PathBuf, String> {
    let mut tried = Vec::new();
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(from_env) = std::env::var("GIRSA_INDEX") {
        candidates.push(PathBuf::from(from_env));
    }
    if let Some(beside) = corpus.parent() {
        candidates.push(beside.join("index"));
    }
    candidates.push(corpus.join("index"));
    for candidate in candidates {
        if candidate.join(girsa_search::index::CACHE_STAMP).is_file() {
            return Ok(candidate);
        }
        tried.push(candidate.display().to_string());
    }
    Err(format!(
        "no search index. Looked in: {}. Run girsa-index build, or set GIRSA_INDEX.",
        tried.join(", ")
    ))
}

/// Open the index and put a bar over it.
///
/// The shelf knows where the corpus is; the index is beside it. A window with a
/// shelf and no index reads perfectly well and cannot search, and it says which
/// of those two it is rather than returning nothing.
fn open_bar_for(shelf: &Option<Shelf>) -> (Option<Bar>, Option<String>) {
    let Some(shelf) = shelf.as_ref() else {
        return (None, Some("there is no shelf to search".to_string()));
    };
    open_bar(shelf.root(), Some(shelf))
}

fn open_bar(corpus: &std::path::Path, shelf: Option<&Shelf>) -> (Option<Bar>, Option<String>) {
    let Some(shelf) = shelf else {
        return (None, Some("there is no shelf to search".to_string()));
    };
    let index_dir = match find_index(corpus) {
        Ok(dir) => dir,
        Err(why) => return (None, Some(why)),
    };
    // A stale index is refused rather than read (spec.md §4.1, W11). The reason
    // it gives names the rules it was built under, and it reaches the reader.
    let index = match SearchIndex::open(&index_dir) {
        Ok(index) => index,
        Err(e) => return (None, Some(e.to_string())),
    };
    let mut catalogue = girsa_search::facets::Catalogue::of(shelf.works());
    // The reader's own arrangement, over the shipped shelf: a result list that
    // filed seforim by the shipped taxonomy while the bookcase beside it used
    // theirs would be two answers to one question (spec.md §5).
    for work in shelf.works() {
        let key = girsa_app::taxonomy::shelf_key_of(work, shelf.arrangement());
        catalogue.filed(&work.slug, key.split('/').map(str::to_string).collect());
    }
    // Your own tags, so the tag facet has rows to count and a click has somewhere
    // to narrow to (B18). Tags were counted and shown with no code path by which
    // clicking one could narrow anything.
    let (notes, _) = girsa_note::note::Notes::open(shelf.personal());
    let catalogue = catalogue.tagged(&notes);
    (Some(Bar::new(index, catalogue, corpus)), None)
}

/// The shelf, as a tree — the shipped taxonomy with your arrangement on top.
///
/// Counts only; the seforim themselves come one shelf at a time from
/// [`shelf_works`]. 7,189 cards is not a browse, it is a dump.
#[tauri::command]
fn shelf_tree(shared: tauri::State<'_, Shared>) -> Result<Vec<Branch>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf.tree())
}

#[tauri::command]
fn shelf_works(shared: tauri::State<'_, Shared>, key: String) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf.works_on(&key).into_iter().map(Card::of).collect())
}

/// Put a sefer on a shelf.
#[tauri::command]
fn shelf_put_work(
    shared: tauri::State<'_, Shared>,
    slug: String,
    shelf: String,
) -> Result<(), String> {
    edit_shelf(&shared, move |a| {
        a.put_work(&slug, &shelf);
        Ok(())
    })
}

/// Put a shelf under another one. Refused if that would make it its own
/// ancestor, and the refusal is shown rather than repaired.
#[tauri::command]
fn shelf_put_shelf(
    shared: tauri::State<'_, Shared>,
    key: String,
    parent: String,
) -> Result<(), String> {
    edit_shelf(&shared, move |a| a.put_shelf(&key, &parent))
}

#[tauri::command]
fn shelf_rename(
    shared: tauri::State<'_, Shared>,
    key: String,
    title: String,
) -> Result<(), String> {
    edit_shelf(&shared, move |a| {
        a.rename(&key, &title);
        Ok(())
    })
}

/// Pin a shelf, or a sefer, to the front of the one it is on.
#[tauri::command]
fn shelf_pin(shared: tauri::State<'_, Shared>, parent: String, key: String) -> Result<(), String> {
    edit_shelf(&shared, move |a| {
        let mut order = a.order.get(&parent).cloned().unwrap_or_default();
        order.retain(|k| *k != key);
        order.insert(0, key);
        a.reorder(&parent, order);
        Ok(())
    })
}

#[tauri::command]
fn shelf_make(
    shared: tauri::State<'_, Shared>,
    parent: String,
    title: String,
) -> Result<String, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let mut made = String::new();
    shelf
        .edit(|a| {
            made = a.make(&parent, &title);
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    Ok(made)
}

/// Put the shelf back the way it shipped. Your seforim stay; only the
/// arrangement goes.
#[tauri::command]
fn shelf_reset(shared: tauri::State<'_, Shared>) -> Result<(), String> {
    edit_shelf(&shared, |a| {
        a.reset();
        Ok(())
    })
}

/// Files dropped on the window become seforim.
#[tauri::command]
fn add_mine(shared: tauri::State<'_, Shared>, paths: Vec<String>) -> Result<Dropped, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;

    let mut added = Vec::new();
    let mut refused = Vec::new();
    for path in paths {
        let file = PathBuf::from(&path);
        match shelf.add_mine(&file, None) {
            Ok(slug) => {
                if let Some(work) = shelf.work(&slug) {
                    added.push(Card::of(work));
                }
            }
            Err(e) => refused.push(Refusal {
                file: path,
                why: e.to_string(),
            }),
        }
    }
    Ok(Dropped { added, refused })
}

fn edit_shelf(
    shared: &tauri::State<'_, Shared>,
    change: impl FnOnce(&mut girsa_app::Arrangement) -> Result<(), girsa_app::arrangement::Refused>,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    shelf.edit(change).map_err(|e| e.to_string())
}

#[tauri::command]
fn companions(shared: tauri::State<'_, Shared>, slug: String) -> Result<Vec<Companion>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf.companions(&slug))
}

// ── The mefarshim on the line (W43) ──────────────────────────────────────────
//
// Two ways to open a commentary, and they answer different questions. `beside`
// puts one sefer in a column and keeps it in step: *Rashi down the side of the
// Gemara*. This is the other one: *of the six mefarshim I follow, which said
// something about this line, and what?* Otzaria's model, and the reason the
// split is untouched by any of it.

/// One mefaresh, as the tick-list shows it.
#[derive(Serialize)]
struct Mefaresh {
    slug: String,
    he_title: String,
    en_title: String,
    /// Whether the reader has ticked it on this sefer.
    chosen: bool,
    /// The folder it is drawn in (W44), or absent for one drawn loose.
    #[serde(skip_serializing_if = "Option::is_none")]
    shelf: Option<String>,
}

/// The tick-list, and which lines to mark given what is ticked.
#[derive(Serialize)]
struct Mefarshim {
    works: Vec<Mefaresh>,
    /// The folders they stand in — rishonim, acharonim, and the authors with
    /// more than one sefer among them (W44). Empty when there is nothing worth
    /// grouping, and then the list is drawn flat.
    folders: Vec<Branch>,
    /// The segments a **ticked** mefaresh speaks on. Only these get a marker:
    /// 2,749 of Berakhot's segments carry commentary from somebody, and a mark
    /// on nearly every line is not a mark.
    marked: Vec<String>,
    /// How many segments carry commentary from anybody. For the sentence under
    /// the list, so *you have ticked nobody* does not read as *nobody wrote*.
    touched: usize,
}

/// The mefarshim on one sefer, and what the reader has ticked.
#[tauri::command]
fn mefarshim(shared: tauri::State<'_, Shared>, slug: String) -> Result<Mefarshim, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let chosen: Vec<String> = state.session.chosen_for(&slug).to_vec();
    let marks = state.marks(&slug)?;
    let commentators = marks.commentators();
    let touched = marks.segments_touched();
    let marked: Vec<String> = marks.marked(&chosen).into_iter().collect();

    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    // The folders they stand in, over the same works the list offers — through
    // `taxonomy::tree`'s idea of a shelf, so a sefer is in one place here and on
    // the bookcase.
    let works: Vec<girsa_corpus::work::Work> = commentators
        .iter()
        .filter_map(|slug| shelf.work(slug).cloned())
        .collect();
    let folders = girsa_app::mefarshim::folders(&works, shelf.arrangement());
    Ok(Mefarshim {
        works: commentators
            .into_iter()
            .map(|work| {
                let named = shelf.work(&work);
                Mefaresh {
                    he_title: named.map_or_else(|| work.clone(), |w| w.he_title.clone()),
                    en_title: named.map_or_else(|| work.clone(), |w| w.en_title.clone()),
                    chosen: chosen.contains(&work),
                    shelf: folders.of.get(&work).cloned(),
                    slug: work,
                }
            })
            .collect(),
        folders: folders.tree,
        marked,
        touched,
    })
}

/// Tick or untick one mefaresh, and say which lines are marked now.
#[tauri::command]
fn choose_mefaresh(
    shared: tauri::State<'_, Shared>,
    slug: String,
    work: String,
    on: bool,
) -> Result<Vec<String>, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.session.choose(&slug, &work, on);
    state.save();
    let chosen: Vec<String> = state.session.chosen_for(&slug).to_vec();
    Ok(state.marks(&slug)?.marked(&chosen).into_iter().collect())
}

/// One mefaresh's words on one line.
#[derive(Serialize)]
struct Said {
    work: String,
    he_title: String,
    en_title: String,
    /// Where this is, in the commentary — what a citation would name.
    address: String,
    lines: Vec<Line>,
}

/// What the ticked mefarshim say about one line, and whether anybody else did.
#[derive(Serialize)]
struct Comments {
    said: Vec<Said>,
    /// True when something comments here that the reader has **not** ticked.
    /// *Nobody wrote about this line* and *none of the six you follow wrote
    /// about this line* are different sentences, and the window says which.
    others: bool,
}

/// Click a line: read the ticked mefarshim on it.
#[tauri::command]
fn mefarshim_at(
    shared: tauri::State<'_, Shared>,
    slug: String,
    at: String,
) -> Result<Comments, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let chosen: Vec<String> = state.session.chosen_for(&slug).to_vec();

    let marks = state.marks(&slug)?;
    let spoken = marks.said(&at, &chosen);
    let mine = marks.on(&at, &chosen).works.len();
    let all = marks.on(&at, &marks.commentators()).works.len();
    let others = all > mine;

    let mut said: Vec<Said> = Vec::new();
    for one in spoken {
        // The commentary itself, read like any other sefer — the same cache, the
        // same corrections, the same nikud toggle. A comment shown by a
        // different path from the pane would be a second idea of what the text
        // says.
        let lines: Vec<Line> = {
            let sefer = state.sefer(&one.work)?;
            let Some(first) = sefer.position_of(&one.at) else {
                // The graph points at a segment this sefer does not have.
                // Skipped rather than reported as an empty comment: it is a fact
                // about the link, and W23's panel is where a bad link is
                // repaired.
                continue;
            };
            let last = one
                .to
                .as_ref()
                .and_then(|to| sefer.position_of(to))
                .map_or(first, |to| to.max(first));
            sefer.segments[first..=last]
                .iter()
                .map(|s| line_of(sefer, s, nikud))
                .collect()
        };
        let named = state.shelf.as_ref().and_then(|s| s.work(&one.work));
        said.push(Said {
            he_title: named.map_or_else(|| one.work.clone(), |w| w.he_title.clone()),
            en_title: named.map_or_else(|| one.work.clone(), |w| w.en_title.clone()),
            address: one.at.path().join(":"),
            work: one.work,
            lines,
        });
    }
    Ok(Comments { said, others })
}

/// Read a sefer for a pane.
///
/// The nikud toggle is applied **here**, by [`girsa_app::display`], rather than
/// in the window. There is one implementation of what a mark is (`girsa-hebrew`,
/// W2) and a second one written in TypeScript would drift from it — and the
/// place it would show is a reader turning nikud off and finding one word in
/// forty still pointed.
#[tauri::command]
fn open_sefer(shared: tauri::State<'_, Shared>, slug: String) -> Result<Text, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let sefer = state.sefer(&slug)?;
    let has_nikud = sefer.segments.iter().any(|s| display::has_marks(&s.text));
    Ok(Text {
        work: Card::of(&sefer.work),
        lines: sefer
            .segments
            .iter()
            .map(|s| line_of(sefer, s, nikud))
            .collect(),
        has_nikud,
    })
}

// ── Scans (spec.md §6.3, BUILDER.md W25) ────────────────────────────────────

/// A scan opened into a pane.
///
/// The window is given the **file** and the mapping, and draws the page itself
/// — the scan is the daf and there is nothing to typeset. What it is not given
/// is any way to work out which daf a page is: that is arithmetic on a
/// declaration, it lives in `girsa-scan`, and it is asked one page at a time.
#[derive(Serialize)]
struct ScanView {
    work: Card,
    pages: usize,
    /// The page to open on: where this scan was left last time (spec.md §6.1's
    /// position memory), or its first page.
    at: usize,
    /// The PDF itself, as a path the window turns into an `asset:` URL.
    file: String,
    /// Whether the once-per-sefer chore has been done. *No mapping yet* and
    /// *nothing printed on this page* are different sentences.
    paged: bool,
    /// The sefer this is a scan of, where the reader has said.
    of: Option<String>,
    scheme: &'static str,
    anchors: Vec<AnchorRow>,
    /// Why nothing here can be cited, where that is so — a scan whose sefer is
    /// not on this shelf, so far. Said rather than fallen back from.
    trouble: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct AnchorRow {
    page: usize,
    /// Absent where the anchor says *these are not pages of the sefer*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    at: Option<String>,
}

/// What one page of a scan is, for the header and for Ctrl+C.
#[derive(Serialize)]
struct PageSaid {
    page: usize,
    /// The whole mareh makom — `ברכות כג.`. Absent for a page the mapping does
    /// not cover, where the window says *page 3 of the file* instead of
    /// inventing a daf.
    display: Option<String>,
    reference: Option<String>,
    /// The permanent id of the page, which is what a note anchors to and what
    /// no mapping ever moves.
    id: Option<String>,
}

/// Open a scan into a pane.
#[tauri::command]
fn scan(shared: tauri::State<'_, Shared>, slug: String) -> Result<ScanView, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.sefer(&slug)?;
    // Where this scan was left last time. Looked up here rather than worked out
    // in the window from the id it remembered, for the reason in `page_of_id`.
    let left = state.session.positions.get(&slug).cloned();
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let sefer = state.open.get(&slug).ok_or("not open")?;
    let scan = girsa_app::scan_of(shelf, sefer).ok_or_else(|| format!("{slug} is not a scan"))?;
    let at = left
        .and_then(|id| girsa_app::scanning::page_of_id(sefer, &id))
        .unwrap_or(1);
    Ok(view_of(shelf, sefer, &scan, at))
}

// ---------------------------------------------------------------------------
// Reading a scan — spec.md §6.3 and §9.7, W26
//
// The division of labour is W25's, applied to words instead of pixels: the
// window is the only thing here that opens a PDF, because pdf.js is one
// renderer on all three platforms and a second PDF stack in Rust would be a
// second opinion about the same file. So the window hands over **glyphs**, or a
// **picture**, and everything after that — where the words are, what is left to
// read, which pages nobody can search yet — is decided in `girsa-scan`, where
// it can be tested without a webview.
//
// And it hands them over **one page at a time**. spec.md §6.3 asks for a job
// that is *background, resumable, never blocking reading*, and the shape of
// that promise is a call that returns after one page: the reader can turn to
// the sugya they were on, and the only cost of stopping is the page it was on.
// ---------------------------------------------------------------------------

/// One glyph the window read off a page, in pixels of the page at scale 1.
#[derive(Deserialize)]
struct DrawnRow {
    text: String,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

/// Where a scan has got to, and what it is being read by.
#[derive(Serialize)]
struct ReadingRow {
    slug: String,
    pages: usize,
    read: usize,
    /// The next page to read, or `null` when there is none left.
    next: Option<usize>,
    /// The engines that have been over it. More than one is normal: a PDF can
    /// carry its own text for the pages that were typeset and none for the
    /// plates.
    by: Vec<String>,
    /// Whether an OCR engine is installed at all. The window offers *read the
    /// pictures* only when there is something to read them with — an offer that
    /// cannot be taken is worse than no offer (spec.md §6.3: OCR is optional).
    engine: Option<String>,
    /// Corrections whose ink the current reading has no word under.
    stranded: usize,
}

/// One word on a page, and the rectangle of the page its ink is on.
#[derive(Serialize)]
struct WordRow {
    text: String,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    confidence: f32,
}

impl WordRow {
    fn of(word: &girsa_scan::Word) -> Self {
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
struct PageWordsRow {
    page: usize,
    by: Option<String>,
    guessed: bool,
    words: Vec<WordRow>,
}

/// *4 PDFs on this shelf aren't searchable yet*, and what it is about.
#[derive(Serialize)]
struct GapRow {
    said: String,
    pages: usize,
    scans: Vec<ScannedRow>,
    /// Notes written since the index was built, or `null` when there is no index
    /// at all — a different answer from zero, and the larger gap of the two.
    notes: Option<usize>,
    /// Corrections made since then, same convention.
    fixes: Option<usize>,
}

#[derive(Serialize)]
struct ScannedRow {
    slug: String,
    title: String,
    pages: usize,
    read: usize,
}

/// How far a scan has been read.
#[tauri::command]
fn scan_reading(shared: tauri::State<'_, Shared>, slug: String) -> Result<ReadingRow, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.sefer(&slug)?;
    let personal = state
        .shelf
        .as_ref()
        .ok_or("there is no shelf here")?
        .personal()
        .to_path_buf();
    let sefer = state.open.get(&slug).ok_or("not open")?;
    let pages = girsa_app::scanning::pages_of(sefer);
    let (words, trouble) = girsa_scan::Words::open(&personal, &slug);
    for line in trouble {
        eprintln!("{line}");
    }
    let job = girsa_scan::Job::of(&slug, pages, &words);
    Ok(ReadingRow {
        slug,
        pages,
        read: job.done(),
        next: job.next(),
        by: words.read_by(),
        engine: girsa_scan::Tesseract::found(Some(&personal)).map(|t| girsa_scan::Engine::name(&t)),
        stranded: words.stranded().len(),
    })
}

/// Take the glyphs the window read off one page of a PDF and make words of them.
#[tauri::command]
fn scan_read_page(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
    width: f32,
    height: f32,
    glyphs: Vec<DrawnRow>,
) -> Result<ReadingRow, String> {
    let personal = {
        let state = shared.lock().map_err(|_| "state is poisoned")?;
        state
            .shelf
            .as_ref()
            .ok_or("there is no shelf here")?
            .personal()
            .to_path_buf()
    };
    if width <= 0.0 || height <= 0.0 {
        return Err("a page with no size on it".to_string());
    }
    // Pixels in, fractions of the page out, converted here and once: a
    // rectangle in pixels is a fact about the size somebody rendered at, and a
    // highlight stored in one lands in the margin the first time a reader
    // zooms.
    let glyphs: Vec<girsa_scan::Glyph> = glyphs
        .into_iter()
        .map(|g| girsa_scan::Glyph {
            text: g.text,
            at: girsa_scan::Area::new(
                g.left / width,
                g.top / height,
                g.right / width,
                g.bottom / height,
            ),
        })
        .collect();
    let grouped = girsa_scan::group(&glyphs);
    if grouped.refused > 0 {
        eprintln!(
            "{slug} page {page}: {} words the file would not spell",
            grouped.refused
        );
    }
    let (mut words, _) = girsa_scan::Words::open(&personal, &slug);
    words
        .record(girsa_scan::Read::new(
            page,
            girsa_scan::Reader::Embedded,
            grouped.words,
        ))
        .map_err(|e| e.to_string())?;
    scan_reading(shared, slug)
}

/// Look at the picture instead, for a page that carries no text of its own.
#[tauri::command]
fn scan_ocr_page(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
    width: u32,
    height: u32,
    png: Vec<u8>,
) -> Result<ReadingRow, String> {
    let personal = {
        let state = shared.lock().map_err(|_| "state is poisoned")?;
        state
            .shelf
            .as_ref()
            .ok_or("there is no shelf here")?
            .personal()
            .to_path_buf()
    };
    let engine = girsa_scan::Tesseract::found(Some(&personal))
        .ok_or_else(|| girsa_scan::EngineError::NoEngine.to_string())?;
    let read = girsa_scan::Engine::read(&engine, page, &girsa_scan::Image { png, width, height })
        .map_err(|e| e.to_string())?;
    let (mut words, _) = girsa_scan::Words::open(&personal, &slug);
    words.record(read).map_err(|e| e.to_string())?;
    scan_reading(shared, slug)
}

/// What is on a page, for drawing a highlight over the photograph.
#[tauri::command]
fn scan_words(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<Option<PageWordsRow>, String> {
    let personal = {
        let state = shared.lock().map_err(|_| "state is poisoned")?;
        state
            .shelf
            .as_ref()
            .ok_or("there is no shelf here")?
            .personal()
            .to_path_buf()
    };
    let (words, _) = girsa_scan::Words::open(&personal, &slug);
    Ok(words.page(page).map(|read| PageWordsRow {
        page: read.page,
        by: Some(read.by.name().to_string()),
        guessed: read.by.is_ocr(),
        words: read.words.iter().map(WordRow::of).collect(),
    }))
}

/// Correct a word on a page, by its ink rather than by where it is in the text.
///
/// The whole of W26 in one call: what is written down is the rectangle, so the
/// correction is still on the same word after the page has been read again by
/// something better (`girsa-scan/tests/the_image_is_ground_truth.rs`).
#[tauri::command]
fn scan_fix(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
    word: usize,
    says: String,
) -> Result<Option<PageWordsRow>, String> {
    let personal = {
        let state = shared.lock().map_err(|_| "state is poisoned")?;
        state
            .shelf
            .as_ref()
            .ok_or("there is no shelf here")?
            .personal()
            .to_path_buf()
    };
    let (mut words, _) = girsa_scan::Words::open(&personal, &slug);
    let was = words
        .as_read(page)
        .ok_or_else(|| format!("nobody has read page {page} of {slug}"))?
        .words
        .get(word)
        .ok_or_else(|| format!("page {page} has no word {word} on it"))?
        .clone();
    words
        .fix(
            page,
            girsa_scan::Fix {
                at: was.at,
                was: was.text,
                says,
            },
        )
        .map_err(|e| e.to_string())?;
    scan_words(shared, slug, page)
}

/// What a search over this shelf cannot see — spec.md §9.7's results header, and
/// the two things it never used to include (B7).
///
/// The header used to be about scans alone, because `Gap` had one variant for
/// them and none for the reader's own writing. A note written this morning and a
/// typo fixed last night are equally absent from the index and were equally
/// unmentioned, which is the state a bochur is in every single day.
#[tauri::command]
fn scan_gap(shared: tauri::State<'_, Shared>) -> Result<Option<GapRow>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let personal = shelf.personal().to_path_buf();
    // Where the index is, if it is anywhere: two of the three gaps are *since the
    // index was built*, so a window that cannot find one has a bigger gap to
    // report, not a smaller one.
    let index = girsa_app::find_index(shelf.root());
    let gap = girsa_app::reading::gap(shelf, &personal, index.as_deref());
    Ok(gap.said().map(|said| GapRow {
        said,
        pages: gap.pages,
        scans: gap
            .scans
            .iter()
            .map(|scan| ScannedRow {
                slug: scan.slug.clone(),
                title: scan.title.clone(),
                pages: scan.pages,
                read: scan.read,
            })
            .collect(),
        notes: gap.layer.notes.count(),
        fixes: gap.layer.fixes.count(),
    }))
}

fn view_of(shelf: &Shelf, sefer: &Open, scan: &girsa_scan::Scan, at: usize) -> ScanView {
    ScanView {
        work: Card::of(&sefer.work),
        pages: scan.pages(),
        at,
        file: sefer.work.origin.display().to_string(),
        paged: scan.is_paged(),
        of: scan.paging().of().map(ToString::to_string),
        scheme: scan.paging().scheme().name(),
        anchors: scan
            .paging()
            .anchors()
            .iter()
            .map(|a| AnchorRow {
                page: a.page,
                at: a.at.as_ref().map(ToString::to_string),
            })
            .collect(),
        trouble: girsa_app::scanning::naming(shelf, scan)
            .err()
            .map(|e| e.to_string()),
    }
}

/// What is printed on a page.
#[tauri::command]
fn scan_at(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<PageSaid, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let style = state.session.cite;
    state.sefer(&slug)?;
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let sefer = state.open.get(&slug).ok_or("not open")?;
    let scan = girsa_app::scan_of(shelf, sefer).ok_or_else(|| format!("{slug} is not a scan"))?;

    // A scan whose sefer is not on the shelf still shows its pages; what it
    // cannot do is print a mekor naming a sefer nobody here has.
    let sent = girsa_app::scanning::naming(shelf, &scan)
        .ok()
        .and_then(|naming| girsa_app::mareh_makom(&scan, page, &naming, &sefer.work, style));
    Ok(PageSaid {
        page,
        display: sent.as_ref().map(|s| s.display().to_string()),
        reference: sent.as_ref().map(|s| s.packet.reference.clone()),
        id: girsa_app::scanning::page_id(sefer, page).map(|id| id.to_string()),
    })
}

/// Say which page is which daf — the once-per-sefer chore (spec.md §6.3).
#[tauri::command]
fn scan_map(
    shared: tauri::State<'_, Shared>,
    slug: String,
    scheme: String,
    anchors: Vec<AnchorRow>,
    of: Option<String>,
) -> Result<ScanView, String> {
    let scheme = girsa_scan::Scheme::named(&scheme)
        .ok_or_else(|| format!("{scheme}: this reads `amud`, `daf` or `numbered`"))?;
    let anchors: Result<Vec<girsa_scan::Anchor>, String> = anchors
        .into_iter()
        .map(|row| match row.at {
            Some(at) => girsa_scan::Anchor::written(row.page, &at).map_err(|e| e.to_string()),
            None => Ok(girsa_scan::Anchor::unpaged(row.page)),
        })
        .collect();
    let of = of.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    // The mapping is checked before it is stored, so a `Paging` that exists is
    // one that has been checked — and the reader is told which anchor was
    // refused rather than finding out from a mekor that lands elsewhere.
    let paging = girsa_scan::Paging::declare(of, scheme, anchors?).map_err(|e| e.to_string())?;

    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_mut().ok_or("there is no shelf here")?;
    shelf
        .declare_paging(&slug, paging)
        .map_err(|e| e.to_string())?;
    // What an address of this sefer means has changed, so the copy held open
    // is out of date (see `Open::paging`).
    state.reread(&slug);
    drop(state);
    scan(shared, slug)
}

/// Take a mapping back — better no mareh makom than a wrong one.
#[tauri::command]
fn scan_forget(shared: tauri::State<'_, Shared>, slug: String) -> Result<ScanView, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_mut().ok_or("there is no shelf here")?;
    shelf.forget_paging(&slug).map_err(|e| e.to_string())?;
    state.reread(&slug);
    drop(state);
    scan(shared, slug)
}

/// The page a place is printed on — the *go to daf* box.
///
/// `None` where this scan does not carry it, and never the nearest page it
/// does: a scan opened one daf away with the header naming the daf that was
/// asked for is wrong in the way nobody checks.
#[tauri::command]
fn scan_page_of(
    shared: tauri::State<'_, Shared>,
    slug: String,
    written: String,
) -> Result<Option<usize>, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.sefer(&slug)?;
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let sefer = state.open.get(&slug).ok_or("not open")?;
    let scan = girsa_app::scan_of(shelf, sefer).ok_or_else(|| format!("{slug} is not a scan"))?;

    // A ref pasted in, or a place typed the way a reader writes one. Both are
    // read by `girsa-ref`, which is the one thing in this system that knows
    // what `ב ע"ב` is.
    if let Ok(reference) = written.parse::<girsa_ref::Ref>() {
        return Ok(scan.page_of_ref(&reference));
    }
    Ok(girsa_ref::Address::parse(&written).and_then(|address| scan.page_of(&address)))
}

/// Ctrl+C on a page of a scan: the mareh makom, in the three flavours.
///
/// There is nothing to quote — the importer will not invent Hebrew it cannot
/// read — so what goes down is the citation and the ref. `girsa-ksav` writes
/// that as a mareh makom rather than as an empty quote block.
#[tauri::command]
fn scan_copy(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<Copied, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let style = state.session.cite;
    state.sefer(&slug)?;
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let sefer = state.open.get(&slug).ok_or("not open")?;
    let scan = girsa_app::scan_of(shelf, sefer).ok_or_else(|| format!("{slug} is not a scan"))?;
    let naming = girsa_app::scanning::naming(shelf, &scan).map_err(|e| e.to_string())?;
    let sent =
        girsa_app::mareh_makom(&scan, page, &naming, &sefer.work, style).ok_or_else(|| {
            format!("there is nothing printed on page {page} that a mekor could name")
        })?;
    Ok(Copied {
        display: sent.display().to_string(),
        reference: sent.packet.reference.clone(),
        lines: 0,
        put: clipboard::put(&sent),
    })
}

// ── Corrections (spec.md §7, BUILDER.md W20) ────────────────────────────────

/// A correction, and the line it landed on.
#[derive(Serialize)]
struct Fixed {
    line: Line,
    /// What to say: the words, and what they now read.
    said: String,
}

/// Correct a typo from where you are reading (spec.md §7.5).
///
/// The offsets are the ones the pane reports for a highlight — the same call
/// Ctrl+C makes — so there is nothing for the reader to look up and nothing to
/// navigate to. What comes back is the one line, redrawn, so the window does
/// not rebuild the sefer around them while they are reading it.
#[tauri::command]
fn fix(
    shared: tauri::State<'_, Shared>,
    at: String,
    from_char: usize,
    to_char: usize,
    now: String,
    kind: String,
    note: Option<String>,
) -> Result<Fixed, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let kind = girsa_fix::Kind::named(&kind).ok_or_else(|| format!("no such kind: {kind}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let slug = at.work().to_string();

    let patch = {
        let sefer = state.sefer(&slug)?;
        let mut patch =
            girsa_app::correction(sefer, &at, from_char..to_char, &now, kind, &who(), nikud)
                .map_err(|e| e.to_string())?;
        if let Some(note) = note.filter(|n| !n.trim().is_empty()) {
            patch = patch.with_note(note);
        }
        patch
    };
    let was = patch.was.clone();

    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    shelf.fix(patch).map_err(|e| e.to_string())?;
    state.reread(&slug);

    let sefer = state.sefer(&slug)?;
    let position = sefer
        .position_of(&at)
        .ok_or_else(|| format!("{at} is not in this sefer"))?;
    let segment = sefer
        .segments
        .get(position)
        .ok_or_else(|| format!("{at} is not in this sefer"))?;
    Ok(Fixed {
        line: line_of(sefer, segment, nikud),
        said: format!("{was} → {now}"),
    })
}

/// Take a correction back.
#[tauri::command]
fn unfix(shared: tauri::State<'_, Shared>, at: String, patch: String) -> Result<Fixed, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let slug = at.work().to_string();

    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let gone = shelf
        .unfix(&girsa_fix::PatchId::from(patch))
        .map_err(|e| e.to_string())?;
    if !gone {
        return Err("there is no such correction".to_string());
    }
    state.reread(&slug);

    let sefer = state.sefer(&slug)?;
    let position = sefer
        .position_of(&at)
        .ok_or_else(|| format!("{at} is not in this sefer"))?;
    let segment = sefer
        .segments
        .get(position)
        .ok_or_else(|| format!("{at} is not in this sefer"))?;
    Ok(Fixed {
        line: line_of(sefer, segment, nikud),
        said: "הוחזר כפי שנדפס".to_string(),
    })
}

/// *Show as printed / show corrected* (spec.md §7.1).
///
/// Three states rather than two, because a scanning error and an emendation are
/// different claims — see [`girsa_fix::Showing`]. Everything open is re-read,
/// which the window does by drawing again.
#[tauri::command]
fn set_showing(shared: tauri::State<'_, Shared>, showing: String) -> Result<(), String> {
    let showing =
        girsa_fix::Showing::named(&showing).ok_or_else(|| format!("no such setting: {showing}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.session.showing = showing;
    let trouble = state.trouble();
    state.shelf.as_mut().ok_or(trouble)?.set_showing(showing);
    state.reread_everything();
    state.save();
    Ok(())
}

/// Your corrections — all of them, or one sefer's.
#[derive(Serialize)]
struct PatchRow {
    id: String,
    segment: String,
    work: String,
    he_title: String,
    address: String,
    kind: &'static str,
    was: String,
    now: String,
    who: String,
    when: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
}

#[tauri::command]
fn fixes(shared: tauri::State<'_, Shared>, slug: Option<String>) -> Result<Vec<PatchRow>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let mut rows: Vec<PatchRow> = shelf
        .fixes()
        .all()
        .filter(|p| slug.as_ref().is_none_or(|s| p.segment.work() == s))
        .map(|p| PatchRow {
            id: p.id.to_string(),
            segment: p.segment.to_string(),
            work: p.segment.work().to_string(),
            he_title: shelf
                .work(p.segment.work())
                .map_or_else(|| p.segment.work().to_string(), |w| w.he_title.clone()),
            address: p.segment.path().join(":"),
            kind: p.kind.as_str(),
            was: p.was.clone(),
            now: p.now.clone(),
            who: p.who.clone(),
            when: p.when,
            note: p.note.clone(),
            source: p.source.clone(),
        })
        .collect();
    // Newest first: a correction queue is read from the top.
    rows.sort_by_key(|row| std::cmp::Reverse(row.when));
    Ok(rows)
}

// ── The links on a line, and repairing them (spec.md §8.3, W23) ─────────────

/// One link, as the panel shows it.
///
/// Everything §8.3 asks a repair UI to show its work with: which end, what the
/// corpus said, what it says now, how it was found, how much to believe it, and
/// which of those were you.
#[derive(Serialize)]
struct LinkRow {
    /// What names this edge in your layer — handed back to repair it.
    edge: String,
    /// `comments-on`, `quotes`, … as it stands now.
    kind: &'static str,
    /// What the corpus shipped, where your layer changed it.
    was: Option<&'static str>,
    outgoing: bool,
    at: String,
    work: String,
    he_title: String,
    address: String,
    said: String,
    /// `sefaria-seed`, `otzaria-seed`, `by-hand`.
    method: &'static str,
    confidence: f32,
    /// The label the corpus used, verbatim — blank for 40% of them (T5).
    label: String,
    confirmed: bool,
    rejected: bool,
    mine: bool,
    /// Which words of the line this link is about, where anything says (§8.4).
    span: Option<(usize, usize)>,
    /// Where that came from: `pinned` (you said) or `dibur` (the commentary
    /// says). Absent when the link is on the whole segment.
    span_from: Option<&'static str>,
    /// Which of the four repairs have been applied to it.
    changed: Vec<&'static str>,
    who: Option<String>,
    /// Whether this may be shown as a statement about the texts, rather than
    /// as *these two are connected somehow*.
    curated: bool,
}

/// What the links panel needs to draw itself.
#[derive(Serialize)]
struct Links {
    links: Vec<LinkRow>,
    /// No inbound cache, so the incoming half is missing. Said out loud: a
    /// sidebar quietly short of half its links reads as a sefer nobody comments
    /// on.
    incoming_unknown: bool,
    /// The types a link may be retyped to, in the order they are offered.
    types: Vec<&'static str>,
    /// Your lenses (§8.5, W24): saved filters, not hardcoded lists.
    lenses: Vec<LensRow>,
    /// Which one is on, if any.
    lens: Option<String>,
}

#[derive(Serialize)]
struct LensRow {
    key: String,
    title: String,
}

/// The links on a line — through a lens, and against a highlight (W23, W24).
///
/// `lens` is one of yours by key, or nothing for all of them. `from_char`/
/// `to_char` are a highlight in the line, and then only the links **not known
/// to be about other words** come back (spec.md §8.4).
#[tauri::command]
fn links(
    shared: tauri::State<'_, Shared>,
    at: String,
    lens: Option<String>,
    from_char: Option<usize>,
    to_char: Option<usize>,
) -> Result<Links, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let lens = lens.filter(|key| !key.is_empty());

    // The line itself, as the pane drew it, because a span is in those
    // characters (W20's two coordinate systems, again).
    let base = {
        let sefer = state.sefer(at.work())?;
        let nth = sefer
            .position_of(&at)
            .ok_or_else(|| format!("{at} is not in this sefer"))?;
        sefer
            .segments
            .get(nth)
            .map(|segment| segment.text.clone())
            .unwrap_or_default()
    };

    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let touching = girsa_app::touching(shelf, shelf.repairs(), &at);
    let mut links = touching.links;

    // The words each link is about, where anything says — and the far end's
    // text is only consulted for seforim that are **already open**.
    for link in &mut links {
        let far = state.open.get(&link.work);
        link.span = girsa_app::links::span_on(link, &at, &base, far, nikud);
    }
    if let (Some(from), Some(to)) = (from_char, to_char) {
        if from < to {
            links = girsa_app::links::touching_words(links, from..to);
        }
    }
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let (lenses, trouble) = girsa_app::Lenses::load(shelf.personal());
    if let Some(said) = trouble {
        eprintln!("{said}");
    }
    if let Some(key) = lens.as_deref() {
        links = lenses.through(key, shelf, links);
    }

    Ok(Links {
        links: links.iter().map(LinkRow::of).collect(),
        incoming_unknown: touching.incoming_unknown,
        types: EDGE_TYPES.iter().map(|t| t.as_str()).collect(),
        lenses: lenses
            .lenses
            .iter()
            .map(|(key, lens)| LensRow {
                key: key.clone(),
                title: lens.title.clone(),
            })
            .collect(),
        lens,
    })
}

/// Pin a link onto the words it is about (spec.md §8.4).
#[tauri::command]
fn link_pin(
    shared: tauri::State<'_, Shared>,
    edge: String,
    at: String,
    from_char: usize,
    to_char: usize,
) -> Result<(), String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    if from_char >= to_char {
        return Err("nothing is selected".to_string());
    }
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = who();
    shelf
        .repairs_mut()
        .pin_named(&edge, &at, from_char..to_char, &who)
        .map_err(|e| e.to_string())
}

/// The types a reader may set, strongest claim first — the order `EdgeType` is
/// declared in, which is the order the facets list them in too.
const EDGE_TYPES: [girsa_link::EdgeType; 9] = [
    girsa_link::EdgeType::CommentsOn,
    girsa_link::EdgeType::Quotes,
    girsa_link::EdgeType::Paraphrases,
    girsa_link::EdgeType::Codifies,
    girsa_link::EdgeType::Disputes,
    girsa_link::EdgeType::Emends,
    girsa_link::EdgeType::ParallelTo,
    girsa_link::EdgeType::Translates,
    girsa_link::EdgeType::References,
];

impl LinkRow {
    fn of(link: &girsa_app::Link) -> Self {
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
            he_title: link.he_title.clone(),
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

/// Confirm, reject, retype, reanchor, or take it all back.
///
/// One command, because they are one thing: a statement about an edge, written
/// into your layer. Which statement is named rather than free-form — the window
/// may choose among what the engine offered and may not invent one.
#[tauri::command]
fn link_repair(
    shared: tauri::State<'_, Shared>,
    edge: String,
    does: String,
    value: Option<String>,
) -> Result<(), String> {
    use girsa_link::repair::Verdict;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = who();
    let repairs = shelf.repairs_mut();
    match does.as_str() {
        "confirm" => repairs.judge_named(&edge, Verdict::Confirmed, &who),
        "reject" => repairs.judge_named(&edge, Verdict::Rejected, &who),
        "retype" => {
            let name = value.ok_or("which type?")?;
            let edge_type = girsa_link::touching::type_named(&name)
                .ok_or_else(|| format!("no such link type: {name}"))?;
            repairs.retype_named(&edge, edge_type, &who)
        }
        "undo" => {
            return repairs
                .undo_named(&edge)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        other => return Err(format!("no such repair: {other}")),
    }
    .map_err(|e| e.to_string())
}

/// Move a link onto the segment you are standing on.
///
/// The end being moved is named, because a link has two and moving the wrong
/// one silently is exactly the class of guess rule 6 forbids.
#[tauri::command]
fn link_reanchor(
    shared: tauri::State<'_, Shared>,
    edge: String,
    end: String,
    to: String,
) -> Result<(), String> {
    let to: SegmentId = to.parse().map_err(|e| format!("{e}"))?;
    let (from_text, to_text) = edge.split_once(" → ").ok_or("that is not an edge")?;
    let (from_anchor, to_anchor) = (parse_anchor(from_text)?, parse_anchor(to_text)?);
    let (from_anchor, to_anchor) = match end.as_str() {
        "from" => (girsa_link::Anchor::point(to), to_anchor),
        "to" => (from_anchor, girsa_link::Anchor::point(to)),
        other => return Err(format!("no such end: {other}")),
    };
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = who();
    shelf
        .repairs_mut()
        .reanchor_named(&edge, from_anchor, to_anchor, &who)
        .map_err(|e| e.to_string())
}

/// Draw a link by hand, from one place to another.
#[tauri::command]
fn link_draw(
    shared: tauri::State<'_, Shared>,
    from: String,
    to: String,
    kind: String,
) -> Result<(), String> {
    let from: SegmentId = from.parse().map_err(|e| format!("{e}"))?;
    let to: SegmentId = to.parse().map_err(|e| format!("{e}"))?;
    let edge_type = girsa_link::touching::type_named(&kind)
        .ok_or_else(|| format!("no such link type: {kind}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = who();
    shelf
        .repairs_mut()
        .draw(
            girsa_link::Anchor::point(from),
            girsa_link::Anchor::point(to),
            edge_type,
            &who,
        )
        .map_err(|e| e.to_string())
}

fn parse_anchor(text: &str) -> Result<girsa_link::Anchor, String> {
    match text.split_once("-girsa:") {
        Some((from, to)) => Ok(girsa_link::Anchor::span(
            from.parse().map_err(|e| format!("{e}"))?,
            format!("girsa:{to}").parse().map_err(|e| format!("{e}"))?,
        )),
        None => Ok(girsa_link::Anchor::point(
            text.parse().map_err(|e| format!("{e}"))?,
        )),
    }
}

// ── Exporting a fixed sefer (spec.md §7.4, BUILDER.md W22) ──────────────────

/// What came out, and where it went.
#[derive(Serialize)]
struct Written {
    path: String,
    segments: usize,
    corrections: usize,
    stale: usize,
    noted: usize,
    /// What to say: the file, and what is in it.
    said: String,
}

/// Write a sefer out with your corrections in it.
///
/// Into your own layer, at `personal/exports/`, rather than through a save
/// dialog: the file is the point and where it goes is not, and a reader who
/// wants it somewhere else has a file manager. The path comes back so the
/// window can say it.
#[tauri::command]
fn export_sefer(
    shared: tauri::State<'_, Shared>,
    slug: String,
    format: String,
) -> Result<Written, String> {
    let format = girsa_app::export::Format::named(&format)
        .ok_or_else(|| format!("no such format: {format}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let showing = state.session.showing;
    let personal = state
        .shelf
        .as_ref()
        .ok_or_else(|| state.trouble())?
        .personal()
        .to_path_buf();

    // The sefer as it is being read — corrections already applied, because
    // that is what `Open` is (W20). Nothing is applied here.
    let sefer = state.sefer(&slug)?;
    let to = personal
        .join("exports")
        .join(girsa_app::export::suggested_name(sefer, format));
    let fixes = state
        .shelf
        .as_ref()
        .ok_or("there is no shelf here")?
        .fixes();
    let sefer = state.open.get(&slug).ok_or("not open")?;
    let done = girsa_app::export(sefer, fixes, format, nikud, &to).map_err(|e| e.to_string())?;
    Ok(Written {
        said: format!(
            "{} · {} · {}",
            done.path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            girsa_app::export::showing_said(showing),
            match done.corrections {
                0 => "בלי תיקונים".to_string(),
                1 => "תיקון אחד".to_string(),
                n => format!("{n} תיקונים"),
            }
        ),
        path: done.path.display().to_string(),
        segments: done.segments,
        corrections: done.corrections,
        stale: done.stale,
        noted: done.noted,
    })
}

// ── The OCR queue (spec.md §7.3, BUILDER.md W21) ────────────────────────────

/// One candidate, as the queue shows it.
#[derive(Serialize)]
struct SuspectRow {
    id: String,
    rare: String,
    common: String,
    rare_count: u64,
    common_count: u64,
    /// `ד/ר`, where the letters are a pair that look alike in print.
    confusion: Option<String>,
    /// What the scanner did — `letter`, `added`, `dropped`, `swapped`.
    how: &'static str,
    /// Where to go and look: the first place, with the sefer named.
    at: Option<String>,
    work: Option<String>,
    he_title: Option<String>,
    address: Option<String>,
}

/// The next candidates to review, best first.
///
/// Re-read from disk every time: `girsa-suspects` is a batch job that runs
/// outside this window, and a queue held in memory would be the one from
/// before it ran.
#[tauri::command]
fn suspects(shared: tauri::State<'_, Shared>, limit: usize) -> Result<Vec<SuspectRow>, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let (queue, trouble) = girsa_fix::suspect::Queue::open(shelf.personal());
    for line in trouble {
        eprintln!("{line}");
    }
    let rows = queue
        .ranked(limit.clamp(1, 500))
        .into_iter()
        .map(|suspect| {
            let at = suspect.places.first();
            SuspectRow {
                id: suspect.id.clone(),
                rare: suspect.rare.clone(),
                common: suspect.common.clone(),
                rare_count: suspect.rare_count,
                common_count: suspect.common_count,
                confusion: suspect.confusion.clone(),
                how: suspect.how.as_str(),
                at: at.map(ToString::to_string),
                work: at.map(|id| id.work().to_string()),
                he_title: at.and_then(|id| shelf.work(id.work()).map(|w| w.he_title.clone())),
                address: at.map(|id| id.path().join(":")),
            }
        })
        .collect();
    state.queue = Some(queue);
    Ok(rows)
}

/// Where on the page a candidate's word is, and what to put in the box.
#[derive(Serialize)]
struct Standing {
    at: String,
    from_char: usize,
    to_char: usize,
    /// The word as printed, which is what the reader is about to change.
    printed: String,
    /// The common spelling, where it can be given without inventing text —
    /// see [`girsa_fix::suspect::Suspect::suggestion`]. `null` on a pointed
    /// word, and then the reader types.
    suggestion: Option<String>,
}

/// Open a candidate: where its word sits in the segment the queue named.
#[tauri::command]
fn suspect_at(
    shared: tauri::State<'_, Shared>,
    id: String,
    at: String,
) -> Result<Standing, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let (queue, _) = girsa_fix::suspect::Queue::open(shelf.personal());
    let suspect = queue.get(&id).ok_or("there is no such candidate")?.clone();
    state.queue = Some(queue);

    let sefer = state.sefer(at.work())?;
    let span = girsa_app::fixing::where_word(sefer, &at, &suspect.rare, nikud)
        .ok_or("that word is not in that line any more")?;
    let drawn = girsa_app::display::Shown::of(
        &sefer
            .segments
            .get(sefer.position_of(&at).ok_or("not in this sefer")?)
            .ok_or("not in this sefer")?
            .text,
        nikud,
    );
    let printed: String = drawn
        .text()
        .chars()
        .skip(span.start)
        .take(span.len())
        .collect();
    Ok(Standing {
        at: at.to_string(),
        from_char: span.start,
        to_char: span.end,
        suggestion: suspect.suggestion(&printed),
        printed,
    })
}

/// Say what was done about a candidate: corrected, or not an error.
///
/// Recorded so the batch job does not ask again — never so that anything is
/// applied. The correction itself, if there is one, went through `fix`.
#[tauri::command]
fn suspect_decide(
    shared: tauri::State<'_, Shared>,
    id: String,
    decision: String,
) -> Result<(), String> {
    let decision = match decision.as_str() {
        "dismissed" => girsa_fix::suspect::Decision::Dismissed,
        "fixed" => girsa_fix::suspect::Decision::Fixed,
        other => return Err(format!("no such decision: {other}")),
    };
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let personal = state
        .shelf
        .as_ref()
        .ok_or_else(|| state.trouble())?
        .personal()
        .to_path_buf();
    let mut queue = match state.queue.take() {
        Some(queue) => queue,
        None => girsa_fix::suspect::Queue::open(&personal).0,
    };
    let known = queue.decide(&id, decision).map_err(|e| e.to_string())?;
    state.queue = Some(queue);
    if known {
        Ok(())
    } else {
        Err("there is no such candidate".to_string())
    }
}

/// Whose correction this is, for the provenance a patch carries.
///
/// The machine's idea of who is sitting at it. There is no account and no
/// registry (spec.md §11); this is a name on a line in your own file, and it is
/// what makes a patch file handed to somebody else say where it came from.
fn who() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "me".to_string())
}

// ── The Ksav loop (spec.md §10, BUILDER.md W15) ─────────────────────────────

/// What one Ctrl+C put down, and where it points.
#[derive(Serialize)]
struct Copied {
    /// The citation as printed — what the window shows in its confirmation, so
    /// a reader can see they copied the place they meant.
    display: String,
    /// The ref the document will store.
    reference: String,
    /// How many segments went.
    lines: usize,
    put: clipboard::Put,
}

/// Copy a selection: the quote, the citation, and the source packet.
///
/// The window sends **character offsets into the text it drew**, which is the
/// text this crate handed it — markup already off, nikud already applied. That
/// is the only way the two ends can agree about where a highlight starts
/// without the webview knowing what a mark is.
#[tauri::command]
fn copy(
    shared: tauri::State<'_, Shared>,
    from: String,
    to: Option<String>,
    from_char: usize,
    to_char: Option<usize>,
    note: Option<String>,
) -> Result<Copied, String> {
    let from: SegmentId = from.parse().map_err(|e| format!("{e}"))?;
    let to: SegmentId = match to {
        Some(to) => to.parse().map_err(|e| format!("{e}"))?,
        None => from.clone(),
    };
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent = girsa_app::send(sefer, &selection, style, nikud, note).map_err(|e| e.to_string())?;
    Ok(Copied {
        display: sent.display().to_string(),
        reference: sent.packet.reference.clone(),
        lines: sent.packet.text.lines().count(),
        put: clipboard::put(&sent),
    })
}

// ── The buffer (spec.md §10.3, BUILDER.md W17) ──────────────────────────────

/// What you are writing, and where it is kept.
#[derive(Serialize)]
struct Writing {
    name: String,
    text: String,
    /// The file it lives in — a `.ksav` document in your own layer, which is
    /// the whole of what "opens in real Ksav with zero conversion" means.
    path: String,
}

#[tauri::command]
fn buffers(shared: tauri::State<'_, Shared>) -> Result<Vec<String>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(girsa_app::Buffer::list(shelf.personal()))
}

#[tauri::command]
fn buffer_open(shared: tauri::State<'_, Shared>, name: String) -> Result<Writing, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let buffer = girsa_app::Buffer::open(shelf.personal(), &name).map_err(|e| e.to_string())?;
    let path = girsa_app::Buffer::path(shelf.personal(), &name).map_err(|e| e.to_string())?;
    Ok(Writing {
        name: buffer.name,
        text: buffer.text,
        path: path.display().to_string(),
    })
}

#[tauri::command]
fn buffer_save(
    shared: tauri::State<'_, Shared>,
    name: String,
    text: String,
) -> Result<String, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let mut buffer = girsa_app::Buffer::new(name);
    buffer.text = text;
    Ok(buffer
        .save(shelf.personal())
        .map_err(|e| e.to_string())?
        .display()
        .to_string())
}

/// The markup for a selection, ready to go into the buffer.
///
/// **The window does not build this string.** It is `girsa-ksav`'s, the writer
/// Ksav itself compiles — a second one written in TypeScript is precisely the
/// drift spec.md §10.3 forbids, and it would show up as documents that differ
/// depending on which end wrote them.
#[tauri::command]
fn source_markup(
    shared: tauri::State<'_, Shared>,
    from: String,
    to: Option<String>,
    from_char: usize,
    to_char: Option<usize>,
) -> Result<String, String> {
    let from: SegmentId = from.parse().map_err(|e| format!("{e}"))?;
    let to: SegmentId = match to {
        Some(to) => to.parse().map_err(|e| format!("{e}"))?,
        None => from.clone(),
    };
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent = girsa_app::send(sefer, &selection, style, nikud, None).map_err(|e| e.to_string())?;
    Ok(girsa_ksav::to_ksav(
        &sent.packet,
        girsa_ksav::CitationPlacement::Mekor,
    ))
}

/// Hand the whole buffer to the real Ksav (spec.md §10.3 — *open the real Ksav
/// editor here*).
///
/// It is saved first, so what Ksav is given and what is on disk are the same
/// words, and only offered when presence says Ksav would take it.
#[tauri::command]
fn buffer_to_ksav(
    shared: tauri::State<'_, Shared>,
    name: String,
    text: String,
) -> Result<(), String> {
    buffer_save(shared, name.clone(), text.clone())?;
    let errand = serde_json::json!({ "name": name, "text": text }).to_string();
    girsa_post::send(girsa_post::App::Ksav, "/document", Some(&errand))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Which of your own documents cite this place (spec.md §10.4).
///
/// Only possible because the documents store **refs**: this is a scan, not a
/// guess, and it is why `מקור:` exists.
#[tauri::command]
fn who_cites(
    shared: tauri::State<'_, Shared>,
    reference: String,
) -> Result<Vec<girsa_app::Citing>, String> {
    let place: girsa_ref::Ref = reference.parse().map_err(|e| format!("{e}"))?;
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(girsa_app::who_cites(shelf.personal(), &place))
}

/// The citations in a piece of prose — **the certain ones** (spec.md §10.5).
///
/// Everything ambiguous stays plain text. See `girsa_app::citing` for the three
/// rules and why each of them refuses more than it accepts.
#[tauri::command]
fn linkify(
    shared: tauri::State<'_, Shared>,
    text: String,
) -> Result<Vec<girsa_app::Linked>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let lexicon = state
        .lexicon
        .as_ref()
        .ok_or("there is no lexicon here — has girsa-import run?")?;
    Ok(girsa_app::linkify(lexicon, &text))
}

/// Whether Ksav is there (spec.md §10.6 — *presence*).
///
/// Asked of Ksav rather than assumed from a file: an endpoint left behind by a
/// crash is not presence. The window uses this to decide whether to *offer*
/// sending at all, which is the whole point — an affordance that would fail is
/// never shown.
#[tauri::command]
fn ksav_presence() -> girsa_post::Presence {
    girsa_post::presence(girsa_post::App::Ksav)
}

/// Send a selection straight into the open Ksav document.
///
/// The clipboard path (W15) works whether or not Ksav is running; this is the
/// one that feels like AirDrop, and it is only offered when presence says it
/// would land.
#[tauri::command]
fn send_to_ksav(
    shared: tauri::State<'_, Shared>,
    from: String,
    to: Option<String>,
    from_char: usize,
    to_char: Option<usize>,
    note: Option<String>,
) -> Result<Copied, String> {
    let from: SegmentId = from.parse().map_err(|e| format!("{e}"))?;
    let to: SegmentId = match to {
        Some(to) => to.parse().map_err(|e| format!("{e}"))?,
        None => from.clone(),
    };
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent = girsa_app::send(sefer, &selection, style, nikud, note).map_err(|e| e.to_string())?;
    let packet = sent.packet.to_json().map_err(|e| e.to_string())?;
    girsa_post::send(girsa_post::App::Ksav, "/insert", Some(&packet)).map_err(|e| e.to_string())?;
    Ok(Copied {
        display: sent.display().to_string(),
        reference: sent.packet.reference.clone(),
        lines: sent.packet.text.lines().count(),
        // Nothing was put on the clipboard: this went the other way.
        put: clipboard::Put::default(),
    })
}

/// How citations print. A preference, and it moves nothing: the document
/// stores the ref.
#[tauri::command]
fn set_cite_style(shared: tauri::State<'_, Shared>, style: String) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.session.cite =
        girsa_cite::CiteStyle::named(&style).ok_or_else(|| format!("no such style: {style}"))?;
    state.save();
    Ok(())
}

#[tauri::command]
fn open_tab(shared: tauri::State<'_, Shared>, slug: String) -> Result<PaneId, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    // A sefer reopens where it was left, which is the whole point of
    // remembering (BUILDER.md W9, per-sefer position memory).
    let at = state.session.where_i_was(&slug).cloned();
    let pane = state.session.workspace.open_tab(slug, at);
    state.save();
    Ok(pane)
}

#[tauri::command]
fn split(
    shared: tauri::State<'_, Shared>,
    pane: PaneId,
    axis: String,
    slug: String,
    follow: bool,
) -> Result<Option<PaneId>, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let axis = if axis == "horizontal" {
        Axis::Horizontal
    } else {
        Axis::Vertical
    };
    let new = state.session.workspace.split(pane, axis, slug, follow);
    state.save();
    Ok(new)
}

#[tauri::command]
fn close_pane(shared: tauri::State<'_, Shared>, pane: PaneId) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.session.workspace.close(pane);
    state.save();
    Ok(())
}

#[tauri::command]
fn focus(shared: tauri::State<'_, Shared>, pane: PaneId) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.session.workspace.focus(pane);
    state.save();
    Ok(())
}

#[tauri::command]
fn set_follows(
    shared: tauri::State<'_, Shared>,
    pane: PaneId,
    leader: Option<PaneId>,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.session.workspace.set_follows(pane, leader);
    state.save();
    Ok(())
}

#[tauri::command]
fn set_ratio(shared: tauri::State<'_, Shared>, pane: PaneId, ratio: u16) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.session.workspace.set_ratio(pane, ratio);
    state.save();
    Ok(())
}

#[tauri::command]
fn set_nikud(shared: tauri::State<'_, Shared>, on: bool) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.session.nikud = on;
    state.save();
    Ok(())
}

#[tauri::command]
fn set_text_size(shared: tauri::State<'_, Shared>, percent: u16) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    state.session.text_size = percent.clamp(60, 250);
    state.save();
    Ok(())
}

// ---------------------------------------------------------------------------
// The semantic lane (spec.md §9.9, W30)
// ---------------------------------------------------------------------------
//
// Eight commands, and the shape of them is the ruling. **Nothing here runs
// unless the reader turned the lane on**: `lane_state` over an off lane opens
// no model and reads no vector, and every other command refuses. The two long
// jobs — bringing a model in and embedding — run on their own thread and emit
// progress, because §9.9 says embedding never blocks reading and a command that
// held the state lock for thirteen days would be a novel way to disagree.

/// Where the lane stands, as the settings panel and the search header show it.
#[derive(Serialize)]
struct LaneRow {
    /// `off`, `adrift` or `on`. Three states, drawn as three states.
    state: &'static str,
    /// The sentence for the header. `None` when the lane is off, which is not a
    /// line — there is no lane to be partial about.
    said: Option<String>,
    /// What the lane covers and what it does not. **Always present**, because a
    /// partial lane that reads as a complete one is what §9.9 exists to prevent.
    coverage: String,
    /// The model directory, as the reader set it.
    model: Option<String>,
    /// Whether Girsa may go and get one. False in a fresh install.
    may_fetch: bool,
    /// The whole library, rather than a list.
    everything: bool,
    /// The seforim chosen, with what is embedded of each.
    chosen: Vec<CoveredRow>,
    /// How many seforim on the shelf are not in the lane at all.
    outside: usize,
    /// Seforim whose vectors were made by another model and are not being read.
    other_model: Vec<String>,
    /// What `lane_bring` would fetch, with its licence — shown before the
    /// button does anything, because the terms are not Girsa's to grant.
    offer: ModelOffer,
}

#[derive(Serialize)]
struct CoveredRow {
    slug: String,
    title: String,
    wanted: usize,
    embedded: usize,
}

#[derive(Serialize)]
struct ModelOffer {
    name: &'static str,
    by: &'static str,
    licence: &'static str,
    about: &'static str,
    what: &'static str,
    bytes: u64,
}

/// One adjacent result.
#[derive(Serialize)]
struct NearRow {
    id: String,
    work: String,
    title: String,
    address: String,
    text: String,
    nearness: f32,
}

/// What the lane answered. Four fields and all four are drawn.
#[derive(Serialize)]
struct LaneAnswer {
    /// The label these must be drawn under. From `girsa-lane`, worded once.
    label: &'static str,
    near: Vec<NearRow>,
    coverage: String,
    /// Why there is nothing. Never an empty list with no reason attached.
    refused: Option<String>,
}

/// How far a background job has got. One shape for both jobs.
#[derive(Serialize, Clone)]
struct LaneProgress {
    /// `bring`, `embed` or `done`.
    doing: &'static str,
    /// What it is working on — a file name, or a sefer's title.
    what: String,
    done: u64,
    of: u64,
    /// Set when the job stopped for a reason worth showing.
    trouble: Option<String>,
}

const BRING_EVENT: &str = "lane-bring";
const EMBED_EVENT: &str = "lane-embed";

fn lane_row(state: &State) -> Result<LaneRow, String> {
    let lane = state.lane.as_ref().ok_or_else(|| state.trouble())?;
    let settings = lane.lane().settings();
    let coverage = lane.coverage();
    let lane_state = lane.state();
    Ok(LaneRow {
        state: match &lane_state {
            girsa_lane::State::Off => "off",
            girsa_lane::State::Adrift(_) => "adrift",
            girsa_lane::State::On { .. } => "on",
        },
        said: lane_state.said(),
        coverage: coverage.said(),
        model: settings.model.as_ref().map(|dir| dir.display().to_string()),
        may_fetch: settings.may_fetch,
        everything: lane.lane().chosen().is_everything(),
        chosen: coverage
            .chosen
            .iter()
            .map(|covered| CoveredRow {
                slug: covered.slug.clone(),
                title: covered.title.clone(),
                wanted: covered.wanted,
                embedded: covered.embedded,
            })
            .collect(),
        outside: coverage.outside.len(),
        other_model: coverage.other_model.clone(),
        offer: ModelOffer {
            name: girsa_lane::BEREL.name,
            by: girsa_lane::BEREL.by,
            licence: girsa_lane::BEREL.licence,
            about: girsa_lane::BEREL.about,
            what: girsa_lane::BEREL.what,
            bytes: girsa_lane::BEREL.bytes,
        },
    })
}

#[tauri::command]
fn lane_state(shared: tauri::State<'_, Shared>) -> Result<LaneRow, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    lane_row(&state)
}

/// Ask the lane. Never an error — a lane that is off, adrift or empty comes
/// back with `refused` set and the coverage sentence said.
#[tauri::command]
fn lane_ask(
    shared: tauri::State<'_, Shared>,
    text: String,
    limit: Option<usize>,
) -> Result<LaneAnswer, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let lane = state.lane.as_ref().ok_or_else(|| state.trouble())?;
    // Scoped by the same chip the literal search is scoped by, so *the whole
    // shelf* and *this sefer* mean the same thing in both columns.
    let scoped: Vec<String> = state.chips.scope.works().into_iter().collect();
    let answer = lane.ask(shelf, &text, &scoped, limit.unwrap_or(girsa_lane::MOST));
    Ok(LaneAnswer {
        label: answer.label,
        near: answer
            .near
            .iter()
            .map(|near| NearRow {
                id: near.id.to_string(),
                address: near.id.path().join(":"),
                work: near.work.clone(),
                title: near.title.clone(),
                text: near.text.clone(),
                nearness: near.nearness,
            })
            .collect(),
        coverage: answer.coverage,
        refused: answer.refused,
    })
}

/// Turn the lane on or off, and point it at a model.
///
/// Turning it on loads the model, which is hundreds of megabytes — so this can
/// take a moment, and a model that will not load is **not** an error here. It is
/// [`girsa_lane::State::Adrift`], which the header says out loud rather than a
/// click that failed silently.
#[tauri::command]
fn lane_set(
    shared: tauri::State<'_, Shared>,
    on: bool,
    model: Option<String>,
) -> Result<LaneRow, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let no_shelf = state.trouble();
    // Two disjoint fields: the shelf read, the lane written. Taken apart by
    // hand because a method on `State` would borrow the whole of it.
    let State { shelf, lane, .. } = &mut *state;
    let shelf = shelf.as_ref().ok_or(no_shelf)?;
    let lane = lane.as_mut().ok_or("there is no lane here")?;
    let was = lane.lane().settings().clone();
    let settings = girsa_lane::Settings {
        on,
        model: model.map(PathBuf::from).or(was.model),
        may_fetch: was.may_fetch,
    };
    lane.set(settings, shelf).map_err(|e| e.to_string())?;
    lane_row(&state)
}

/// Let Girsa go and get a model, or stop it being able to.
///
/// Its own command rather than a field on `lane_set`, because it is its own
/// decision: spec.md §14 says Girsa never *needs* the network, and this is the
/// switch that makes that sentence true in a fresh install.
#[tauri::command]
fn lane_allow_fetch(shared: tauri::State<'_, Shared>, allow: bool) -> Result<LaneRow, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let no_shelf = state.trouble();
    // Two disjoint fields: the shelf read, the lane written. Taken apart by
    // hand because a method on `State` would borrow the whole of it.
    let State { shelf, lane, .. } = &mut *state;
    let shelf = shelf.as_ref().ok_or(no_shelf)?;
    let lane = lane.as_mut().ok_or("there is no lane here")?;
    let settings = girsa_lane::Settings {
        may_fetch: allow,
        ..lane.lane().settings().clone()
    };
    lane.set(settings, shelf).map_err(|e| e.to_string())?;
    lane_row(&state)
}

/// Bring a model in. Needs `lane_allow_fetch` first.
///
/// Runs on its own thread and emits [`BRING_EVENT`], so the panel draws a bar
/// and the reader can carry on learning. Stopping is closing the panel: the
/// `.part` file stays and the next press resumes where it left off.
#[tauri::command]
fn lane_bring(app: tauri::AppHandle, shared: tauri::State<'_, Shared>) -> Result<(), String> {
    use tauri::Emitter;
    let (personal, may_fetch) = {
        let state = shared.lock().map_err(|_| "state is poisoned")?;
        let lane = state.lane.as_ref().ok_or("there is no lane here")?;
        (
            lane.lane().personal().to_path_buf(),
            lane.lane().settings().may_fetch,
        )
    };
    if !may_fetch {
        return Err(girsa_lane::BringError::NotAllowed.to_string());
    }
    std::thread::spawn(move || {
        let mut last = 0u64;
        let brought = girsa_lane::bring(&personal, true, &mut |progress| {
            let mb = progress.bytes / 1_048_576;
            if mb != last {
                last = mb;
                let _ = app.emit(
                    BRING_EVENT,
                    LaneProgress {
                        doing: "bring",
                        what: progress.file.clone(),
                        done: progress.bytes,
                        of: progress.want.unwrap_or(0),
                        trouble: None,
                    },
                );
            }
            true
        });
        let trouble = match &brought {
            Ok(dir) => {
                // Point the lane at what just landed, and turn it on. The reader
                // pressed a button that says *bring it in*; making them then find
                // the directory they never chose would be a joke.
                if let Ok(mut state) = tauri::Manager::state::<Shared>(&app).lock() {
                    let State { shelf, lane, .. } = &mut *state;
                    if let (Some(shelf), Some(lane)) = (shelf.as_ref(), lane.as_mut()) {
                        let settings = girsa_lane::Settings {
                            on: true,
                            model: Some(dir.clone()),
                            may_fetch: true,
                        };
                        if let Err(e) = lane.set(settings, shelf) {
                            eprintln!("the model came in but the setting would not save: {e}");
                        }
                    }
                }
                None
            }
            Err(e) => Some(e.to_string()),
        };
        let _ = app.emit(
            BRING_EVENT,
            LaneProgress {
                doing: "done",
                what: girsa_lane::BEREL.name.to_string(),
                done: 0,
                of: 0,
                trouble,
            },
        );
    });
    Ok(())
}

/// Put a sefer in the lane, or take it out. `all` chooses the whole library.
#[tauri::command]
fn lane_choose(
    shared: tauri::State<'_, Shared>,
    slug: Option<String>,
    add: bool,
    all: bool,
) -> Result<LaneRow, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let no_shelf = state.trouble();
    // Two disjoint fields: the shelf read, the lane written. Taken apart by
    // hand because a method on `State` would borrow the whole of it.
    let State { shelf, lane, .. } = &mut *state;
    let shelf = shelf.as_ref().ok_or(no_shelf)?;
    let lane = lane.as_mut().ok_or("there is no lane here")?;
    let mut chosen = lane.lane().chosen().clone();
    if all {
        chosen = if add {
            girsa_lane::Chosen::everything()
        } else {
            girsa_lane::Chosen::nothing()
        };
    } else if let Some(slug) = slug {
        if add {
            chosen = chosen.with_work(&slug);
        } else if chosen.is_everything() {
            return Err(
                "the whole library is in the lane — turn that off first, then choose seforim"
                    .to_string(),
            );
        } else if !chosen.without_work(&slug) {
            return Err(format!("{slug} was not in the lane"));
        }
    }
    lane.choose(chosen, shelf).map_err(|e| e.to_string())?;
    lane_row(&state)
}

/// Embed what is chosen, on its own thread, emitting [`EMBED_EVENT`].
///
/// The lane is **cloned** for the thread, which shares the one loaded model
/// rather than loading a second — see `girsa_lane::Lane`. The state lock is held
/// only long enough to take that clone, so nothing about reading a sefer waits
/// on this.
#[tauri::command]
fn lane_embed(app: tauri::AppHandle, shared: tauri::State<'_, Shared>) -> Result<(), String> {
    use tauri::Emitter;
    let (lane, root, slugs, titles, stop) = {
        let state = shared.lock().map_err(|_| "state is poisoned")?;
        let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
        let held = state.lane.as_ref().ok_or("there is no lane here")?;
        if !held.state().is_on() {
            return Err(girsa_lane::LaneError::Off.to_string());
        }
        let slugs = girsa_app::adjacent::in_the_lane(shelf, held.lane().chosen());
        let titles: HashMap<String, String> = slugs
            .iter()
            .map(|slug| {
                (
                    slug.clone(),
                    shelf
                        .work(slug)
                        .map_or_else(|| slug.clone(), |work| work.he_title.clone()),
                )
            })
            .collect();
        (
            held.for_thread(),
            shelf.root().to_path_buf(),
            slugs,
            titles,
            state.stop_embedding.clone(),
        )
    };
    stop.store(false, std::sync::atomic::Ordering::Relaxed);

    std::thread::spawn(move || {
        let mut trouble: Vec<String> = Vec::new();
        'seforim: for slug in slugs {
            let title = titles.get(&slug).cloned().unwrap_or_else(|| slug.clone());
            let mut run = match lane.run(&root, &slug) {
                Ok(run) => run,
                Err(e) => {
                    trouble.push(format!("{title}: {e}"));
                    continue;
                }
            };
            trouble.extend(run.trouble().iter().cloned());
            if let Some(made_by) = run.made_by_something_else() {
                trouble.push(format!(
                    "{title}: its vectors were made by {made_by} and are not being added to"
                ));
                continue;
            }
            loop {
                if stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break 'seforim;
                }
                match run.step() {
                    Ok(0) => break,
                    Ok(_) => {
                        let _ = app.emit(
                            EMBED_EVENT,
                            LaneProgress {
                                doing: "embed",
                                what: title.clone(),
                                done: run.job().done() as u64,
                                of: run.job().wanted() as u64,
                                trouble: None,
                            },
                        );
                    }
                    Err(e) => {
                        trouble.push(format!("{title}: {e}"));
                        break;
                    }
                }
            }
        }
        // The coverage sentence is recomputed once, at the end, off the disk —
        // never cached and never guessed at from the counters above.
        if let Ok(mut state) = tauri::Manager::state::<Shared>(&app).lock() {
            let State { shelf, lane, .. } = &mut *state;
            if let (Some(shelf), Some(held)) = (shelf.as_ref(), lane.as_mut()) {
                held.refresh(shelf);
            }
        }
        let _ = app.emit(
            EMBED_EVENT,
            LaneProgress {
                doing: "done",
                what: String::new(),
                done: 0,
                of: 0,
                trouble: (!trouble.is_empty()).then(|| trouble.join("\n")),
            },
        );
    });
    Ok(())
}

/// Stop the embedding job. Costs the batch it is on and nothing else.
#[tauri::command]
fn lane_stop(shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    state
        .stop_embedding
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// A pane moved. Record it, and say where the panes following it have to go.
///
/// This is W9's acceptance in one function. The answer for each follower is a
/// [`Place`], which is three-valued on purpose: somewhere, *nowhere on this
/// line*, or *nothing relates these two seforim*. The window shows the second
/// and third rather than moving the column to something near.
#[tauri::command]
fn moved(shared: tauri::State<'_, Shared>, pane: PaneId, at: String) -> Result<Vec<Move>, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;

    state.session.remember(at.clone());
    let followers = state.session.workspace.moved(pane, at.clone());
    if followers.is_empty() {
        state.save();
        return Ok(Vec::new());
    }

    let leader_slug = at.work().to_string();
    let root = state
        .shelf
        .as_ref()
        .map(|s| s.root().to_path_buf())
        .ok_or_else(|| state.trouble())?;

    let wanted: Vec<(PaneId, String)> = followers
        .iter()
        .filter_map(|id| {
            state
                .session
                .workspace
                .pane(*id)
                .map(|p| (*id, p.slug.clone()))
        })
        .collect();

    // Every sefer involved is read before any of them is placed: `sefer`
    // borrows the map mutably, and the leader has to stay borrowed while the
    // follower is looked at.
    state.sefer(&leader_slug)?;
    for (_, slug) in &wanted {
        state.sefer(slug)?;
    }

    let mut moves = Vec::new();
    for (id, slug) in wanted {
        let (Some(leader), Some(follower)) = (state.open.get(&leader_slug), state.open.get(&slug))
        else {
            continue;
        };
        // A scan follows the sefer beside it by turning to the page the daf is
        // printed on — but only where the reader has said it is a scan **of**
        // that sefer (W25). Everything else is left where it is, which is W9's
        // rule and not a special case of it.
        if let Some(shelf) = state.shelf.as_ref() {
            if let Some(scan) = girsa_app::scan_of(shelf, follower) {
                let page = girsa_app::scanning::beside(&scan, &at);
                moves.push(Move {
                    pane: id,
                    place: match page.and_then(|p| girsa_app::scanning::page_id(follower, p)) {
                        Some(id) => Place::At(vec![id]),
                        // The scan is of this sefer and does not carry this
                        // daf — a scan of one masechta open beside another
                        // volume of it. *Related, and nothing here*, which is
                        // the sentence W9 wrote `NoPlace` for.
                        None if scan.paging().of() == Some(leader_slug.as_str()) => Place::NoPlace,
                        None => Place::Unrelated,
                    },
                    relation: if scan.paging().of() == Some(leader_slug.as_str()) {
                        girsa_app::Relation::Declared {
                            follower_is_commentary: false,
                        }
                    } else {
                        girsa_app::Relation::Unrelated
                    },
                    page,
                });
                continue;
            }
        }
        let beside = Beside::between(leader, follower, &root);
        moves.push(Move {
            pane: id,
            place: beside.place(&at),
            relation: beside.relation(),
            page: None,
        });
    }
    state.save();
    Ok(moves)
}

/// Where the shelf is.
///
/// `GIRSA_CORPUS` wins, then a `corpus` directory beside the executable — how
/// an installed copy finds it — then two levels up, which is how it is found
/// when run out of the source tree.
fn find_corpus() -> Result<PathBuf, String> {
    let mut tried = Vec::new();
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(from_env) = std::env::var("GIRSA_CORPUS") {
        candidates.push(PathBuf::from(from_env));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("corpus"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("corpus"));
        candidates.push(cwd.join("../../corpus"));
    }
    for candidate in candidates {
        if candidate.join("works/index.jsonl").is_file() {
            return Ok(candidate);
        }
        tried.push(candidate.display().to_string());
    }
    Err(format!(
        "no shelf found. Looked in: {}. Run girsa-fetch and girsa-import, or \
         set GIRSA_CORPUS.",
        tried.join(", ")
    ))
}

/// Where your own layer is: the arrangement, and the seforim you added.
///
/// Beside the session file, in the app's data directory — not under the corpus
/// root, which a re-download is entitled to replace wholesale.
fn find_personal(data: &std::path::Path) -> PathBuf {
    std::env::var("GIRSA_PERSONAL").map_or_else(|_| data.join("personal"), PathBuf::from)
}

/// Open the window.
///
/// If it cannot be opened at all there is nothing to carry on into, so this
/// says so and exits non-zero. It used to `expect`, which is the same outcome
/// wearing a backtrace — and it was the only `unwrap`/`expect` in this file,
/// which is now denied here rather than merely avoided (see `Cargo.toml`).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let built = tauri::Builder::default()
        // `girsa://…` — and a ref, which is already a `girsa:` URI, so the
        // citation a Word document or a compiled PDF has been carrying all
        // along is a link that lands on the page it names (spec.md §10.6).
        .plugin(tauri_plugin_deep_link::init())
        // One dialog: *choose the directory your model is in* (W30). The default
        // way to use the semantic lane is to point it at weights the reader
        // already has, and a native picker is the difference between that being
        // the default and being the fallback.
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let data = tauri::Manager::path(app)
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let session_path = data.join("session.json");
            let session = Session::load(&session_path);
            let personal = find_personal(&data);

            // The one directory of the reader's own disk the window may read:
            // where a dropped PDF was copied to (W25). A scan is hundreds of
            // megabytes and cannot travel over the IPC channel a page at a
            // time, so the webview opens the file itself — and nothing outside
            // this directory is reachable, which is why the scope is opened
            // here rather than declared as a pattern in the config.
            if let Err(e) = tauri::Manager::asset_protocol_scope(app)
                .allow_directory(personal.join("files"), false)
            {
                eprintln!("scans will not open: {e}");
            }

            let (shelf, trouble) = match find_corpus() {
                Ok(root) => match Shelf::open(&root, &personal) {
                    // An unreadable arrangement is the shelf's own trouble to
                    // report, and it is not a reason to open no shelf.
                    Ok(mut shelf) => {
                        let trouble = shelf.trouble().map(ToString::to_string);
                        // How much of the correction layer to apply, as it was
                        // left last time (W20).
                        shelf.set_showing(session.showing);
                        (Some(shelf), trouble)
                    }
                    Err(e) => (None, Some(e.to_string())),
                },
                Err(e) => (None, Some(e)),
            };
            let (bar, no_search) = open_bar_for(&shelf);
            let lexicon = shelf.as_ref().and_then(|shelf| read_lexicon(shelf.root()));
            // The lane, once. With it off — the default — this opens nothing and
            // reads nothing; with it on it loads the side-loaded model, which is
            // why it happens here and not on the first query.
            let lane = shelf.as_ref().map(|shelf| {
                let (lane, trouble) = girsa_app::Adjacency::open(shelf.root(), &personal, shelf);
                for line in trouble {
                    eprintln!("the semantic lane: {line}");
                }
                lane
            });
            tauri::Manager::manage(
                app,
                Mutex::new(State {
                    shelf,
                    bar,
                    no_search,
                    chips: Chips::default(),
                    trouble,
                    session,
                    session_path,
                    desk: None,
                    no_post: None,
                    lexicon,
                    queue: None,
                    lane,
                    stop_embedding: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    open: HashMap::new(),
                    order: Vec::new(),
                    marks: HashMap::new(),
                }),
            );

            // The loopback, after the state it answers out of exists. A
            // failure here costs the pairing and not the library: the window
            // still reads seforim and the presence chip says why.
            let handle = tauri::Manager::app_handle(app).clone();
            let (desk, no_post) = match post::open(&handle) {
                Ok(desk) => (Some(desk), None),
                Err(e) => (None, Some(e.to_string())),
            };
            if let Ok(mut state) = tauri::Manager::state::<Shared>(app).lock() {
                state.desk = desk;
                state.no_post = no_post;
            }

            // A citation clicked anywhere on the machine. On Windows and Linux
            // the scheme is registered by the installer; in a dev build it is
            // registered here, which is why this is not only a listener.
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(e) = app.deep_link().register_all() {
                    eprintln!("could not register girsa:// with the system: {e}");
                }
            }
            let opener = handle.clone();
            tauri_plugin_deep_link::DeepLinkExt::deep_link(app).on_open_url(move |event| {
                for url in event.urls() {
                    post::opened_url(&opener, url.as_str());
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            state,
            search,
            recent,
            companions,
            mefarshim,
            choose_mefaresh,
            mefarshim_at,
            open_sefer,
            open_tab,
            scan,
            scan_at,
            scan_map,
            scan_reading,
            scan_read_page,
            scan_ocr_page,
            scan_words,
            scan_fix,
            scan_gap,
            lane_state,
            lane_ask,
            lane_set,
            lane_allow_fetch,
            lane_bring,
            lane_choose,
            lane_embed,
            lane_stop,
            scan_forget,
            scan_page_of,
            scan_copy,
            split,
            close_pane,
            focus,
            set_follows,
            set_ratio,
            set_nikud,
            set_text_size,
            moved,
            shelf_tree,
            shelf_works,
            shelf_put_work,
            shelf_put_shelf,
            shelf_rename,
            shelf_pin,
            shelf_make,
            shelf_reset,
            add_mine,
            find,
            find_chip,
            find_rung,
            find_narrow,
            find_whole_shelf,
            copy,
            set_cite_style,
            ksav_presence,
            send_to_ksav,
            buffers,
            buffer_open,
            buffer_save,
            source_markup,
            buffer_to_ksav,
            who_cites,
            linkify,
            fix,
            unfix,
            set_showing,
            fixes,
            suspects,
            suspect_at,
            suspect_decide,
            export_sefer,
            links,
            link_repair,
            link_reanchor,
            link_draw,
            link_pin,
            yours,
            notes,
            note_write,
            note_read,
            note_edit,
            note_forget,
            mark_here,
            mark_forget,
            marks_in,
            bookmarks,
            query_keep,
            queries,
            query_recall,
            query_forget,
            folders,
            folder_edit,
            folder_forget,
            tags,
            export_layer,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = built {
        // A sentence a reader can act on, not a panic message. The rest of this
        // shell refuses legibly; the one path that can only stop should too.
        eprintln!("Girsa could not open its window: {e}");
        std::process::exit(1);
    }
}

// ── Your own layer (spec.md §11, BUILDER.md W27) ────────────────────────────
//
// Notes, highlights, bookmarks, tags, saved queries and chaburah folders.
//
// **What is not here is the interesting part.** There is no `notes_on` command
// returning your notes about a line, because your notes about a line come back
// from `links` — a note's edge is a `girsa_link::Edge` and the panel that draws
// what the library says about a sugya draws what you said about it in the same
// list, sorted by the same rule. Adding a second endpoint for it here is what
// the whole crate exists to avoid.
//
// What is left is the two things that are not edges — marks and folders — and
// the writing side.

/// One of your notes, as a row.
#[derive(Serialize)]
struct NoteRow {
    slug: String,
    name: String,
    title: String,
    opening: String,
    tags: Vec<String>,
    paragraphs: usize,
    edited: u64,
    /// What it is about, as segment ids.
    on: Vec<String>,
}

/// One paragraph of a note, for editing it.
#[derive(Serialize)]
struct ParaRow {
    id: String,
    text: String,
}

/// One mark, and where it lands in the line as it is drawn now.
#[derive(Serialize)]
struct MarkRow {
    id: String,
    kind: &'static str,
    at: String,
    label: Option<String>,
    colour: Option<String>,
    was: String,
    tags: Vec<String>,
    /// The characters it is on, in the text the pane drew — `None` for a
    /// bookmark, and `None` with `stale` set when its words have gone.
    span: Option<(usize, usize)>,
    /// The words had to be looked for. Shown, because a highlight that moved
    /// is a thing a reader is entitled to know about.
    moved: bool,
    /// Its words are gone, or are now there twice. **Not drawn and not
    /// deleted** — reported, so it can be put right.
    stale: bool,
}

/// Everything of yours on one line, less the notes — those are links.
#[derive(Serialize)]
struct Yours {
    notes: Vec<NoteRow>,
    marks: Vec<MarkRow>,
    folders: Vec<String>,
}

impl NoteRow {
    fn of(note: &girsa_note::Note) -> Self {
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

impl MarkRow {
    fn of(marked: &girsa_app::Marked) -> Self {
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

/// What you have on the line you are standing on.
#[tauri::command]
fn yours(shared: tauri::State<'_, Shared>, at: String) -> Result<Yours, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;

    // The line as the pane drew it — corrected, and with the nikud the reader
    // has on — because that is the string a highlight's offsets are against.
    let base = {
        let sefer = state.sefer(at.work())?;
        sefer
            .position_of(&at)
            .and_then(|nth| sefer.segments.get(nth))
            .map(|segment| segment.text.clone())
            .unwrap_or_default()
    };
    let nikud = state.session.nikud;
    let drawn = display::Shown::of(&base, nikud).text().to_string();

    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let found = girsa_app::yours(shelf, &at, &drawn);
    Ok(Yours {
        notes: found
            .notes
            .iter()
            .filter_map(|wrote| shelf.notes().get(&wrote.slug))
            .map(NoteRow::of)
            .collect(),
        marks: found.marks.iter().map(MarkRow::of).collect(),
        folders: found.folders,
    })
}

/// Every note you have, most recently touched first.
#[tauri::command]
fn notes(shared: tauri::State<'_, Shared>) -> Result<Vec<NoteRow>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let mut rows: Vec<NoteRow> = shelf.notes().all().map(NoteRow::of).collect();
    rows.sort_by(|a, b| b.edited.cmp(&a.edited).then_with(|| a.title.cmp(&b.title)));
    Ok(rows)
}

/// Write a note about where you are standing. The three-second one.
#[tauri::command]
fn note_write(
    shared: tauri::State<'_, Shared>,
    at: String,
    title: Option<String>,
    text: String,
) -> Result<NoteRow, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = who();
    let note = girsa_app::note_here(shelf, &at, title.as_deref(), &text, &who)
        .map_err(|e| e.to_string())?;
    Ok(NoteRow::of(&note))
}

/// One note, paragraph by paragraph, for editing it.
#[tauri::command]
fn note_read(shared: tauri::State<'_, Shared>, note: String) -> Result<Vec<ParaRow>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let held = shelf
        .notes()
        .get(&note)
        .ok_or_else(|| format!("there is no note called {note}"))?;
    Ok(held
        .paras()
        .iter()
        .map(|para| ParaRow {
            id: para.id.to_string(),
            text: para.text.clone(),
        })
        .collect())
}

/// Change a note: a paragraph's words, another paragraph, one taken out, a tag,
/// an anchor.
///
/// One command, because they are one thing — an edit to a note in your own
/// layer — and which edit is **named rather than free-form**, the same rule as
/// `link_repair`.
///
/// Every one of these writes the whole note back, and none of them renumbers a
/// paragraph: a paragraph put in the middle mints a child ordinal (spec.md §3),
/// so an id the window is holding is still the id it was holding.
#[tauri::command]
fn note_edit(
    shared: tauri::State<'_, Shared>,
    note: String,
    does: String,
    value: Option<String>,
    text: Option<String>,
) -> Result<Vec<ParaRow>, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let mut held = shelf
        .notes()
        .get(&note)
        .cloned()
        .ok_or_else(|| format!("there is no note called {note}"))?;

    let words = || text.clone().unwrap_or_default();
    let paragraph = |value: &Option<String>| -> Result<SegmentId, String> {
        value
            .as_deref()
            .ok_or("which paragraph?")?
            .parse()
            .map_err(|e| format!("{e}"))
    };
    match does.as_str() {
        "append" => {
            held.append(words());
        }
        "after" => {
            held.insert_after(&paragraph(&value)?, words())
                .map_err(|e| e.to_string())?;
        }
        "set" => {
            if !held.set(&paragraph(&value)?, words()) {
                return Err("that paragraph is not in this note".to_string());
            }
        }
        "remove" => {
            if !held.remove(&paragraph(&value)?) {
                return Err("that paragraph is not in this note".to_string());
            }
        }
        "title" => held.title = words(),
        "tag" => {
            held.tag(&words());
        }
        "untag" => {
            held.untag(&words());
        }
        "anchor" => {
            held.anchor(paragraph(&value)?);
        }
        "unanchor" => {
            held.unanchor(&paragraph(&value)?);
        }
        other => return Err(format!("no such edit: {other}")),
    }
    let written = shelf.write_note(held).map_err(|e| e.to_string())?;
    Ok(written
        .paras()
        .iter()
        .map(|para| ParaRow {
            id: para.id.to_string(),
            text: para.text.clone(),
        })
        .collect())
}

/// Throw a note away — the file, the sefer and the catalogue line.
#[tauri::command]
fn note_forget(shared: tauri::State<'_, Shared>, note: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    // A note is a sefer, and a sefer that has gone may not stay open in a pane
    // holding text nothing on the shelf accounts for.
    let slug = shelf.notes().get(&note).map(|held| held.slug.clone());
    let gone = shelf.forget_note(&note).map_err(|e| e.to_string())?;
    if let Some(slug) = slug {
        state.open.remove(&slug);
        state.order.retain(|kept| kept != &slug);
    }
    Ok(gone)
}

/// Highlight some words, or mark the place.
///
/// The words are read out of the line as the pane drew it and stored with the
/// mark, because an offset is not a place (`girsa_corpus::span`).
#[tauri::command]
fn mark_here(
    shared: tauri::State<'_, Shared>,
    at: String,
    from_char: Option<usize>,
    to_char: Option<usize>,
    label: Option<String>,
    colour: Option<String>,
) -> Result<MarkRow, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let base = {
        let sefer = state.sefer(at.work())?;
        sefer
            .position_of(&at)
            .and_then(|nth| sefer.segments.get(nth))
            .map(|segment| segment.text.clone())
            .unwrap_or_default()
    };
    let drawn = display::Shown::of(&base, nikud).text().to_string();

    let who = who();
    let mut made = match (from_char, to_char) {
        (Some(from), Some(to)) if from < to => {
            let letters: Vec<char> = drawn.chars().collect();
            let was: String = letters
                .get(from..to)
                .ok_or("those characters are not in the line")?
                .iter()
                .collect();
            girsa_note::Mark::highlight(at, from..to, was, &who)
        }
        _ => girsa_note::Mark::bookmark(at, &who),
    };
    if let Some(label) = label.filter(|l| !l.trim().is_empty()) {
        made = made.called(label);
    }
    if let Some(colour) = colour.filter(|c| !c.trim().is_empty()) {
        made = made.coloured(colour);
    }

    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let held = shelf
        .marks_mut()
        .add(made)
        .map_err(|e| e.to_string())?
        .clone();
    let placed = held.place(&drawn);
    Ok(MarkRow::of(&girsa_app::Marked { mark: held, placed }))
}

/// Take a mark back.
#[tauri::command]
fn mark_forget(shared: tauri::State<'_, Shared>, mark: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    shelf
        .marks_mut()
        .remove(&girsa_note::MarkId::from(mark))
        .map_err(|e| e.to_string())
}

/// Every mark in one sefer, placed against the lines as the pane draws them.
///
/// One call for a whole sefer rather than one per line: a highlight has to be
/// **painted where it is**, and asking line by line would make that a hundred
/// round trips a page. Where each one lands is still decided here — the window
/// is handed offsets into the text it was already sent, and works nothing out.
#[tauri::command]
fn marks_in(shared: tauri::State<'_, Shared>, slug: String) -> Result<Vec<MarkRow>, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let nikud = state.session.nikud;
    let drawn: HashMap<String, String> = {
        let sefer = state.sefer(&slug)?;
        sefer
            .segments
            .iter()
            .map(|segment| {
                (
                    segment.id.to_string(),
                    display::Shown::of(&segment.text, nikud).text().to_string(),
                )
            })
            .collect()
    };
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf
        .marks()
        .in_work(&slug)
        .map(|mark| {
            let text = drawn.get(&mark.at.to_string()).map_or("", String::as_str);
            MarkRow::of(&girsa_app::Marked {
                mark: mark.clone(),
                placed: mark.place(text),
            })
        })
        .collect())
}

/// Every bookmark, most recent first — the *take me back* list.
#[tauri::command]
fn bookmarks(shared: tauri::State<'_, Shared>) -> Result<Vec<MarkRow>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf
        .marks()
        .bookmarks()
        .into_iter()
        .map(|mark| {
            MarkRow::of(&girsa_app::Marked {
                mark: mark.clone(),
                placed: girsa_note::mark::Placed::Whole,
            })
        })
        .collect())
}

/// One saved query, as a row.
#[derive(Serialize)]
struct QueryRow {
    name: String,
    typed: String,
    said: String,
    tags: Vec<String>,
}

/// Keep the question you just asked.
///
/// The chips are saved as the `chip → key` pairs the row itself sends, so
/// recalling one goes through the same [`set_chip`] a click does. The scope is
/// saved as the seforim it comes to — a scope narrowed by three clicks comes
/// back as one clause over the same seforim, which matches the same segments
/// and no longer remembers the three clicks. Said here rather than discovered.
#[tauri::command]
fn query_keep(
    shared: tauri::State<'_, Shared>,
    name: String,
    typed: String,
) -> Result<QueryRow, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let mut kept = girsa_note::SavedQuery::new(name, typed);
    for chip in state.chips.row() {
        if let Some(chosen) = chip.choices.iter().find(|choice| choice.chosen) {
            // The scope chip's key is not an option among others — it is the
            // whole scope — so it is saved as the slugs below instead.
            if chip.name != "where" {
                kept = kept.with_chip(chip.name, chosen.key.clone());
            }
        }
    }
    kept = kept
        .within(state.chips.scope.works())
        .excluding(state.chips.scope.excluded_works().iter().cloned());

    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let held = shelf
        .queries_mut()
        .save(kept)
        .map_err(|e| e.to_string())?
        .clone();
    Ok(QueryRow {
        name: held.name.clone(),
        typed: held.typed.clone(),
        said: held.said(),
        tags: held.tags.clone(),
    })
}

/// The questions you have kept.
#[tauri::command]
fn queries(shared: tauri::State<'_, Shared>) -> Result<Vec<QueryRow>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf
        .queries()
        .all()
        .map(|query| QueryRow {
            name: query.name.clone(),
            typed: query.typed.clone(),
            said: query.said(),
            tags: query.tags.clone(),
        })
        .collect())
}

/// Ask one again: set the chips and the scope back, and hand over the line.
///
/// The line comes back rather than being searched here, because what goes in
/// the box is the window's business and *what the chips are* is not.
#[tauri::command]
fn query_recall(shared: tauri::State<'_, Shared>, name: String) -> Result<String, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let held = shelf
        .queries()
        .get(&name)
        .ok_or_else(|| format!("there is no saved query called {name}"))?
        .clone();

    state.chips = Chips::default();
    for (chip, key) in &held.chips {
        set_chip(&mut state.chips, chip, key)?;
    }
    let mut scope = girsa_search::scope::Scope::everything();
    if !held.only.is_empty() {
        scope = scope.only(held.only.clone(), &held.name);
    }
    if !held.without.is_empty() {
        scope = scope.without(held.without.clone(), &held.name);
    }
    state.chips.scope = scope;
    Ok(held.typed)
}

/// Forget a saved query.
#[tauri::command]
fn query_forget(shared: tauri::State<'_, Shared>, name: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    shelf.queries_mut().remove(&name).map_err(|e| e.to_string())
}

/// One chaburah folder, as a row.
#[derive(Serialize)]
struct FolderRow {
    name: String,
    title: String,
    /// Its members, in the order you put them in — which is the order a shiur
    /// goes in, so it is never sorted.
    members: Vec<FolderMember>,
    tags: Vec<String>,
}

#[derive(Serialize)]
struct FolderMember {
    /// The member as it is written down: a segment id, `work:…` or `query:…`.
    key: String,
    /// What to put on the row.
    said: String,
    /// Where clicking it goes, for the two kinds that are places.
    work: Option<String>,
    at: Option<String>,
}

/// Your chaburah folders.
#[tauri::command]
fn folders(shared: tauri::State<'_, Shared>) -> Result<Vec<FolderRow>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf
        .collections()
        .all()
        .map(|folder| FolderRow {
            name: folder.name.clone(),
            title: folder.title.clone(),
            members: folder
                .members
                .iter()
                .map(|member| match member {
                    girsa_note::Member::Place(id) => FolderMember {
                        key: member.to_string(),
                        said: shelf.work(id.work()).map_or_else(
                            || id.to_string(),
                            |work| format!("{} {}", work.he_title, id.path().join(":")),
                        ),
                        work: Some(id.work().to_string()),
                        at: Some(id.to_string()),
                    },
                    girsa_note::Member::Work(slug) => FolderMember {
                        key: member.to_string(),
                        said: shelf
                            .work(slug)
                            .map_or_else(|| slug.clone(), |work| work.he_title.clone()),
                        work: Some(slug.clone()),
                        at: None,
                    },
                    girsa_note::Member::Query(name) => FolderMember {
                        key: member.to_string(),
                        said: name.clone(),
                        work: None,
                        at: None,
                    },
                })
                .collect(),
            tags: folder.tags.clone(),
        })
        .collect())
}

/// Put something in a folder, or take it out. The folder is made if it is not
/// there yet.
#[tauri::command]
fn folder_edit(
    shared: tauri::State<'_, Shared>,
    name: String,
    title: Option<String>,
    does: String,
    member: String,
) -> Result<usize, String> {
    let member: girsa_note::Member = member.parse()?;
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let mut folder = shelf.collections().get(&name).cloned().unwrap_or_else(|| {
        girsa_note::Collection::new(&name, title.unwrap_or_else(|| name.clone()))
    });
    match does.as_str() {
        "put" => {
            folder.put(member);
        }
        "take-out" => {
            folder.take_out(&member);
        }
        other => return Err(format!("no such edit: {other}")),
    }
    let held = folder.members.len();
    shelf
        .collections_mut()
        .save(folder)
        .map_err(|e| e.to_string())?;
    Ok(held)
}

/// Throw a folder away. What was in it is untouched — it held members, not
/// copies.
#[tauri::command]
fn folder_forget(shared: tauri::State<'_, Shared>, name: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| "state is poisoned")?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    shelf
        .collections_mut()
        .remove(&name)
        .map_err(|e| e.to_string())
}

/// One tag, and how many things carry it.
#[derive(Serialize)]
struct TagRow {
    tag: String,
    total: usize,
    notes: usize,
    marks: usize,
    queries: usize,
    collections: usize,
}

/// Every tag across your whole layer.
#[tauri::command]
fn tags(shared: tauri::State<'_, Shared>) -> Result<Vec<TagRow>, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let counted = girsa_note::Tags::of(
        shelf.notes(),
        shelf.marks(),
        shelf.queries(),
        shelf.collections(),
    );
    Ok(counted
        .iter()
        .map(|(tag, tally)| TagRow {
            tag: tag.to_string(),
            total: tally.total(),
            notes: tally.notes,
            marks: tally.marks,
            queries: tally.queries,
            collections: tally.collections,
        })
        .collect())
}

/// Write your whole layer out somewhere, as plain files.
///
/// Into `personal/exports/` by default, the way a corrected sefer goes out
/// (W22): the files are the point and where they land is not.
#[tauri::command]
fn export_layer(shared: tauri::State<'_, Shared>, into: Option<String>) -> Result<String, String> {
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let into = into.map_or_else(
        || shelf.personal().join("exports").join("my-layer"),
        PathBuf::from,
    );
    let written = girsa_note::export(
        shelf.notes(),
        shelf.marks(),
        shelf.queries(),
        shelf.collections(),
        &into,
    )
    .map_err(|e| e.to_string())?;
    Ok(format!(
        "{} · {} הערות · {} סימונים · {} שאילתות · {} תיקיות",
        into.display(),
        written.notes,
        written.marks,
        written.queries,
        written.collections
    ))
}

/// Kept honest: the workspace type the window draws is the one the tests are
/// written against, not a second copy of it living in TypeScript.
#[allow(dead_code)]
fn _assert_workspace_is_the_tested_one(w: &Workspace) -> usize {
    w.tabs.len()
}
