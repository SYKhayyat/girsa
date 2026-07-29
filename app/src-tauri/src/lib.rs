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
use serde::Serialize;

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
    /// Slug → the sefer, read once. Cleared oldest-first.
    open: HashMap<String, Open>,
    order: Vec<String>,
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
    let chips = &mut state.chips;
    match chip.as_str() {
        "mode" => {
            chips.mode = match key.as_str() {
                "Smart" => Mode::Smart,
                "Regex" => Mode::Regex,
                "Citation" => Mode::Citation,
                "Instruments" => Mode::Instruments,
                _ => Mode::ToratEmet,
            }
        }
        "the word" => {
            chips.matching = match key.as_str() {
                "Contains" => Match::Contains,
                "Letters" => Match::Letters,
                _ => Match::Word,
            }
        }
        "together" => {
            chips.together = match key.as_str() {
                "Phrase" => Together::Phrase,
                other => match other.strip_prefix("Near").and_then(|n| n.parse().ok()) {
                    Some(words) => Together::Near { words },
                    None => Together::Anywhere,
                },
            }
        }
        "instrument" => {
            chips.sounding = match key.as_str() {
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
    /// No companions cache, so the incoming half is missing. Said out loud: a
    /// sidebar quietly short of half its links reads as a sefer nobody comments
    /// on.
    incoming_unknown: bool,
    /// The types a link may be retyped to, in the order they are offered.
    types: Vec<&'static str>,
}

#[tauri::command]
fn links(shared: tauri::State<'_, Shared>, at: String) -> Result<Links, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let touching = girsa_app::touching(shelf, shelf.repairs(), &at);
    Ok(Links {
        links: touching.links.iter().map(LinkRow::of).collect(),
        incoming_unknown: touching.incoming_unknown,
        types: EDGE_TYPES.iter().map(|t| t.as_str()).collect(),
    })
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
        let beside = Beside::between(leader, follower, &root);
        moves.push(Move {
            pane: id,
            place: beside.place(&at),
            relation: beside.relation(),
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

/// # Panics
///
/// If the window cannot be created at all, which is not a condition the app can
/// carry on from.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // `girsa://…` — and a ref, which is already a `girsa:` URI, so the
        // citation a Word document or a compiled PDF has been carrying all
        // along is a link that lands on the page it names (spec.md §10.6).
        .plugin(tauri_plugin_deep_link::init())
        .setup(move |app| {
            let data = tauri::Manager::path(app)
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let session_path = data.join("session.json");
            let session = Session::load(&session_path);
            let personal = find_personal(&data);

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
                    open: HashMap::new(),
                    order: Vec::new(),
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
            open_sefer,
            open_tab,
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
        ])
        .run(tauri::generate_context!())
        .expect("the window could not be created");
}

/// Kept honest: the workspace type the window draws is the one the tests are
/// written against, not a second copy of it living in TypeScript.
#[allow(dead_code)]
fn _assert_workspace_is_the_tested_one(w: &Workspace) -> usize {
    w.tabs.len()
}
