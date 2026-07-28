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

struct State {
    shelf: Option<Shelf>,
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
    session: Session,
    session_path: PathBuf,
    /// Slug → the sefer, read once. Cleared oldest-first.
    open: HashMap<String, Open>,
    order: Vec<String>,
}

impl State {
    fn sefer(&mut self, slug: &str) -> Result<&Open, String> {
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

type Shared = Mutex<State>;

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
    let state = shared.lock().map_err(|_| "state is poisoned")?;
    Ok(serde_json::json!({
        "workspace": state.session.workspace,
        "nikud": state.session.nikud,
        "text_size": state.session.text_size,
        "positions": state.session.positions,
        "works": state.shelf.as_ref().map_or(0, |s| s.works().len()),
        "trouble": state.trouble,
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
fn find(
    shared: tauri::State<'_, Shared>,
    query: String,
    page: usize,
) -> Result<FoundPage, String> {
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

    let answer = bar.ask(&query, &chips, paging, &girsa_ref::resolve::Context::default());
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
    open_bar(&shelf.root().to_path_buf(), Some(shelf))
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
fn shelf_rename(shared: tauri::State<'_, Shared>, key: String, title: String) -> Result<(), String> {
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
            .map(|s| Line {
                id: s.id.to_string(),
                address: s.id.path().join(":"),
                kind: s.kind.as_str(),
                runs: display::runs(&if nikud {
                    s.text.clone()
                } else {
                    display::without_marks(&s.text)
                }),
            })
            .collect(),
        has_nikud,
    })
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
    std::env::var("GIRSA_PERSONAL")
        .map_or_else(|_| data.join("personal"), PathBuf::from)
}

/// # Panics
///
/// If the window cannot be created at all, which is not a condition the app can
/// carry on from.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
                    Ok(shelf) => {
                        let trouble = shelf.trouble().map(ToString::to_string);
                        (Some(shelf), trouble)
                    }
                    Err(e) => (None, Some(e.to_string())),
                },
                Err(e) => (None, Some(e)),
            };
            let (bar, no_search) = open_bar_for(&shelf);
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
                    open: HashMap::new(),
                    order: Vec::new(),
                }),
            );
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
