//! Searching (spec.md §9, BUILDER.md W14).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use std::path::PathBuf;
use std::sync::Arc;

use girsa_app::display;
use girsa_app::shelf::Shelf;
use girsa_app::view::{AtRow, HitRow, LandingRow, OfferRow, PlaceRow};
use girsa_search::bar::{Answer, Bar};
use girsa_search::chips::{Chip, Chips};
use girsa_search::facets::{self, Dimension, Facets, Row};
use girsa_search::index::{Paging, SearchIndex};
use serde::Serialize;

use crate::{Shared, State};

/// One hit, drawn. **One** of these, not two.
///
/// A free function rather than `HitRow::of`, and that is the boundary rather
/// than an inconvenience: the *shape* of a result row is `girsa-app`'s, and
/// filling it from a `girsa_search::index::Hit` is the shell's, because the hit
/// is. `girsa-app` does not depend on `girsa-search` — `reading::gap_over`
/// takes a slice of slugs rather than a `Scope` specifically so that it need
/// not — and this is where that boundary shows.
///
/// `find` and the widening ladder each carried a nine-field literal, and they
/// were character-for-character identical, so a tenth field added to one of
/// them would have reached a reader who searched and not a reader who took an
/// offer, which is the same query one keystroke later.
fn hit_row(
    hit: &girsa_search::index::Hit,
    marker: &girsa_search::bar::Marker,
    names: Option<&girsa_app::Names<'_>>,
    pointing: girsa_app::session::Pointing,
    shemos: girsa_app::shemos::Shemos,
) -> HitRow {
    let (page, by, guessed) = scanned(hit);
    HitRow {
        at: names.map_or_else(
            // No shelf is a state with no rows in it; this is here so the
            // type does not have to be an `Option` for a case that cannot
            // happen while there are hits to draw.
            || AtRow {
                id: hit.id.to_string(),
                work: hit.id.work().to_string(),
                title: hit.id.work().to_string(),
                address: hit.id.address(),
                written: None,
                era: None,
            },
            |names| AtRow::of(&names.of(&hit.id)),
        ),
        runs: shown(hit, marker, pointing, shemos),
        page,
        by,
        guessed,
        marked: marked(marker, hit),
    }
}

/// The words a hit matched, sliced out of its own text.
/// A hit's words, with the ones that answered the query in runs of their own
/// (W39).
///
/// > *"the search result is not clear (the actual hit)."*
///
/// The order matters: mark the text **as printed**, then take the nikud off. The
/// engine's ranges are byte offsets into the pointed text, so stripping first
/// would put every mark two or three letters left of the word it meant.
fn shown(
    hit: &girsa_search::index::Hit,
    marker: &girsa_search::bar::Marker,
    pointing: girsa_app::session::Pointing,
    shemos: girsa_app::shemos::Shemos,
) -> Vec<display::Run> {
    // The marks are byte ranges into `hit.text`, which is why the shemos go on
    // **after** them and the nikud comes off after that. Every substitution is
    // one letter for one letter, so a mark placed on the text as the engine
    // saw it still covers the same word once a shem has been rewritten — which
    // is the whole reason that invariant is asserted in `girsa_app::shemos`.
    display::unpointed(
        display::runs_marking(&hit.text, &marker.marks(hit))
            .into_iter()
            .map(|run| display::Run {
                text: girsa_app::shemos::written(&run.text, shemos).into_owned(),
                ..run
            })
            .collect(),
        pointing,
    )
}

pub(crate) fn marked(
    marker: &girsa_search::bar::Marker,
    hit: &girsa_search::index::Hit,
) -> Vec<String> {
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
pub(crate) struct FoundPage {
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

/// Search, and hand back everything the panel draws.
///
/// The chips are read from what was typed first (a sigil flips a chip — §9.5),
/// so the row that comes back is the row the search actually ran under.
#[tauri::command(async)]
pub(crate) fn find(
    shared: tauri::State<'_, Shared>,
    query: String,
    page: usize,
) -> Result<FoundPage, String> {
    let size = girsa_app::enough::A_PAGE;
    let paging = Paging {
        from: size * page.saturating_sub(1).min(usize::MAX / size.max(1)),
        size,
    };
    // **The lock is taken to read the chips and dropped before the search.**
    // `Bar::ask` is `&self` and the engine holds no state of ours; the only
    // reason it ever ran under the guard is that the guard was around
    // everything. Four real queries measure 8, 63, 73 and 90 ms and a regex
    // over five million segments is unbounded — and for every one of those
    // milliseconds the scroll handler beside it could not be served, in a
    // panel whose whole design is to stay open while you read.
    let (chips, bar) = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        // A sigil sets a chip, and the chip stays set — that is what makes
        // typing one a way of *finding* the chips rather than a syntax beside
        // them.
        let (chips, _) = state.chips.read(&query);
        state.chips = chips;
        let chips = state.chips.clone();
        // **An empty box is not a refusal.** The panel calls this with `""` on
        // open, deliberately, to draw the chip row without running a search —
        // and the engine answered `nothing to search for`, which the window
        // then printed in red, in English, above a row of English chips,
        // before the reader had typed anything. Opening by telling somebody
        // off.
        //
        // Nothing has been asked, so nothing is refused: the chips come back
        // and the header is empty. The engine keeps its refusal, which is the
        // right answer to a *command line* that was given no words.
        if query.trim().is_empty() {
            return Ok(FoundPage::nothing_asked(&chips));
        }
        let Some(bar) = state.bar.clone() else {
            let why = state.no_search();
            return Ok(FoundPage::refused(&chips, why));
        };
        (chips, bar)
    };

    let answer = bar.ask(
        &query,
        &chips,
        paging,
        &girsa_ref::resolve::Context::default(),
    );

    // Back under the lock only to name what came out. A page is twenty hits;
    // this is the cheap half.
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // How a place is printed, from the reader's own setting.
    let style = state.session.cite;
    // **One shelf guard for the whole of the drawing**, taken here and handed
    // to both `names` and `landing_row`. Two `state.shelf()` calls alive on one
    // thread is a read-read reentry on a non-reentrant lock.
    let shelf = state.shelf().ok();
    let names = shelf.as_ref().map(|shelf| state.names(shelf));
    Ok(match answer {
        Answer::Segments {
            results,
            offers,
            note,
            rungs: _,
            landing,
        } => {
            let pages = results.total.div_ceil(size.max(1));
            FoundPage {
                header: results.header.clone(),
                note,
                hits: results
                    .hits
                    .iter()
                    .map(|hit| hit_row(hit, &results.marker, names.as_ref(), pointing, shemos))
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
                // The place the words also name, offered above the hits and
                // taken by nothing — the resolver was the best thing in this
                // engine and it was behind a sigil nothing taught.
                landing: landing.map(|landing| landing_row(&landing, shelf.as_deref(), style)),
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
            landing: Some(landing_row(&landing, shelf.as_deref(), style)),
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

    /// The chip row, and no question asked. Not a refusal and not a zero — the
    /// panel opening, which is what `find("")` is for.
    fn nothing_asked(chips: &Chips) -> Self {
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
            refused: None,
            landing: None,
        }
    }
}

/// One landing, as the window draws it.
///
/// One function, because it is built in two places now — the Citation mode's
/// whole answer, and the offer that sits above an ordinary word search — and two
/// copies would be two ideas of what a place looks like.
fn landing_row(
    landing: &girsa_search::citation::Landing,
    shelf: Option<&Shelf>,
    style: girsa_cite::CiteStyle,
) -> LandingRow {
    // **A place, not an id.** The panel printed `girsa:bavli/shabbat/31a` three
    // times over — once as the sentence and once per candidate — while Ctrl+C
    // on the very same line produced `הועתק — שבת דף לא. שורה א'`. Two
    // formatters, and the landing got the wrong one.
    let said_of = |place: &girsa_search::citation::Place| {
        let Some(shelf) = shelf else {
            return place.reference.to_string();
        };
        let slug = place.run.first.work();
        // **With the schema's own words**, which is the difference between
        // `טור אורח חיים סימן א' סעיף א'` and `טור orach_chayim א' א'` — and
        // the second is what this printed until the real window was asked.
        shelf.work(slug).map_or_else(
            || place.reference.to_string(),
            |work| {
                girsa_app::sending::cite_of_in(
                    work,
                    Some(&shelf.sections(slug)),
                    &place.run.first,
                    style,
                )
            },
        )
    };
    let places: Vec<PlaceRow> = landing
        .places
        .iter()
        .map(|place| PlaceRow {
            said: said_of(place),
            reference: place.reference.to_string(),
            id: place.run.first.to_string(),
            work: place.run.first.work().to_string(),
        })
        .collect();
    LandingRow {
        // With one candidate the sentence *is* the place, and `Landing::describe`
        // hands back the ref's own spelling of it. With several it is a count,
        // which is a sentence about the answer and stays where it is.
        said: match places.as_slice() {
            [only] => only.said.clone(),
            _ => landing.describe(),
        },
        places,
        near: landing.near.iter().map(near_said).collect(),
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
#[tauri::command(async)]
pub(crate) fn find_rung(
    shared: tauri::State<'_, Shared>,
    query: String,
    page: usize,
    rung: String,
) -> Result<FoundPage, String> {
    let Some(rung) = girsa_search::ladder::Rung::named(&rung) else {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            format!("no such rung: {rung}"),
        ));
    };
    // Same two phases as `find`, for the same reason: a rung *widens* the
    // query, so this is the slower of the two searches, not the faster one.
    let (chips, text, bar) = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let (chips, text) = state.chips.read(&query);
        state.chips = chips.clone();
        let Some(bar) = state.bar.clone() else {
            let why = state.no_search();
            return Ok(FoundPage::refused(&chips, why));
        };
        (chips, text, bar)
    };
    let query = girsa_search::torat_emet::Query::new(text)
        .matching(chips.matching)
        .together(chips.together);
    let widened = girsa_search::ladder::Widened::new(query, [rung]);
    let paging = Paging {
        from: girsa_app::enough::A_PAGE * page.saturating_sub(1),
        size: girsa_app::enough::A_PAGE,
    };
    let found = bar
        .index()
        .search_widened_in(&widened, &chips.scope, paging)
        .map_err(|e| e.to_string())?;

    let state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
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
    let shelf = state.shelf().ok();
    let names = shelf.as_ref().map(|shelf| state.names(shelf));
    Ok(FoundPage {
        header,
        // The rung that was applied, and how to go back — **coded**, not
        // written out. A Hebrew sentence composed in the shell is the same
        // defect as an English one: an English window would have shown it in
        // Hebrew. `app/src/trouble.ts` says it in whichever language the reader
        // is in.
        note: Some(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::RungApplied,
            "a rung was applied — search again without it to go back",
        )),
        hits: found
            .hits
            .iter()
            .map(|hit| hit_row(hit, &marker, names.as_ref(), pointing, shemos))
            .collect(),
        total: found.total,
        page: page.max(1),
        pages: found.total.div_ceil(girsa_app::enough::A_PAGE),
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
#[tauri::command(async)]
pub(crate) fn find_chip(
    shared: tauri::State<'_, Shared>,
    chip: String,
    key: String,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.chips.choose(&chip, &key).map_err(|e| e.to_string())
}

/// Click a facet row: narrow to it, or rule it out (spec.md §9.8).
#[tauri::command(async)]
pub(crate) fn find_narrow(
    shared: tauri::State<'_, Shared>,
    dimension: Dimension,
    row: Row,
    exclude: bool,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
#[tauri::command(async)]
pub(crate) fn find_whole_shelf(shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.chips.scope = girsa_search::scope::Scope::everything();
    Ok(())
}

/// One thing the reader has added to or subtracted from where the search looks.
#[derive(Serialize)]
struct ScopeStep {
    /// What they clicked.
    label: String,
    /// Subtracting rather than adding.
    exclude: bool,
    /// How many seforim it names, so a row can say what it is worth.
    seforim: usize,
    /// The row it came from, so the tree can draw that row ticked.
    ///
    /// A shelf key and a sefer slug are both keys and they do not collide, so
    /// the tree matches on this alone. The label will not do: every shelf has a
    /// `ראשונים` under it.
    key: String,
}

/// Where the search is looking, as a list a panel can draw and edit.
#[derive(Serialize)]
pub(crate) struct ScopeView {
    /// The chip's own sentence, so the panel and the chip cannot word it two
    /// ways.
    said: String,
    steps: Vec<ScopeStep>,
    /// Whether this is the whole shelf — which is not the same as *no steps*
    /// once link types are in it.
    everything: bool,
    /// How many seforim the search will actually look in, and how many there
    /// are altogether.
    ///
    /// *"it should be more clear what is and is not included."* The panel listed
    /// what had been clicked and never once said what that came to, so a reader
    /// who had clicked eight times could read eight labels off the screen and
    /// still not know whether they were about to search four seforim or four
    /// thousand. Two numbers answer it in the one place the question is asked.
    seforim: usize,
    /// Every sefer on the shelf, to divide the first number by.
    shelf: usize,
}

/// What the scope is now.
///
/// The panel that draws this is the answer to *"i dont know how to add some and
/// minus some things from the search (some seforim or folders). often the tree
/// to pick from … is not even visible - it flashes, then flashes off."* The tree
/// was the **facet rail**, which is computed from a result set: it existed only
/// after a search returned hits, and it was cleared at the start of the next
/// one. So the one control for choosing where to look could only be used
/// after you had already looked, and vanished while you looked again.
///
/// This is the scope itself, which exists before any search and outlives every
/// one — asked for by the panel, not derived from an answer.
#[tauri::command(async)]
pub(crate) fn find_scope(shared: tauri::State<'_, Shared>) -> Result<ScopeView, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    Ok(scope_view(&state))
}

/// The scope as the panel draws it, out of whatever state the caller holds.
///
/// Split out because every command that edits the scope ends by handing the
/// whole of it back, and re-locking a mutex the caller still holds is a
/// deadlock on Windows, where `std::sync::Mutex` is not reentrant. `find_scope`
/// used to be called at the end of `find_scope_add` after an inner block had
/// dropped the guard — which worked, and only because the block was there.
fn scope_view(state: &State) -> ScopeView {
    let scope = &state.chips.scope;
    let shelf = state.shelf().map_or(0, |shelf| shelf.works().len());
    // What the search will actually look in. `works()` is empty when nothing
    // has been narrowed, and empty there means *every sefer* — the one place
    // that distinction has to be spelled out rather than counted.
    let seforim = if scope.is_everything() {
        shelf
    } else {
        let kept = scope.works();
        if kept.is_empty() {
            shelf.saturating_sub(scope.excluded_works().len())
        } else {
            kept.len()
        }
    };
    ScopeView {
        said: scope.describe(),
        steps: scope
            .steps()
            .iter()
            .map(|step| ScopeStep {
                label: step.label.clone(),
                exclude: step.exclude,
                seforim: step.len(),
                key: step.key.clone(),
            })
            .collect(),
        everything: scope.is_everything(),
        seforim,
        shelf,
    }
}

/// Tick or untick one row of the scope tree.
///
/// The difference from [`find_scope_add`] is that this is a **checkbox**, so it
/// is idempotent and it undoes itself: ticking a row that is already ticked
/// does nothing, and unticking one takes back the step that row put in rather
/// than adding an opposite step beside it. `find_scope_add` grew an
/// ever-lengthening list of clicks that could only be undone by index, which is
/// what a reader means by *"there should also be a clear all"*.
///
/// `on` is what the box should read after the click, not what was clicked.
#[tauri::command(async)]
pub(crate) fn find_scope_set(
    shared: tauri::State<'_, Shared>,
    dimension: Dimension,
    key: String,
    label: String,
    on: bool,
) -> Result<ScopeView, String> {
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let Some(bar) = state.bar.as_ref() else {
            return Err(state.no_search());
        };
        let asked = dimension.asked();
        let mut scope = state.chips.scope.clone();
        let was_in = scope.holds(asked, &key);
        let was_out = scope.refuses(asked, &key);
        // Whichever way this row was pointing, it is not pointing that way any
        // more. Both directions come off first, so a tick after a `−` is a tick
        // and not a `−` with a `+` stacked on it.
        scope.drop_key(asked, &key);
        let picked = scope.any_picked(asked);
        let row = Row {
            key,
            label,
            count: 0,
            depth: 0,
        };
        // Three states, not two, and this is where they are resolved. A row is
        // in scope because it was picked *or* because nothing was picked and
        // the whole shelf is in — so unticking sometimes means *drop the pick*
        // and sometimes means *rule it out*, and which one is not a property of
        // the row.
        scope = match (on, was_in, was_out, picked) {
            // Ticking a row that was only ruled out, with nothing picked: the
            // whole shelf is back in, and picking this row would narrow to it.
            (true, _, true, false) => scope,
            (true, ..) => facets::narrow(&scope, bar.catalogue(), dimension, &row),
            // Unticking one of several picks: dropping it is the whole answer.
            (false, true, ..) => scope,
            (false, ..) => facets::exclude(&scope, bar.catalogue(), dimension, &row),
        };
        state.chips.scope = scope;
    }
    find_scope(shared)
}

/// Take one step back — the `×` on a row of the scope panel.
#[tauri::command(async)]
pub(crate) fn find_scope_drop(
    shared: tauri::State<'_, Shared>,
    at: usize,
) -> Result<ScopeView, String> {
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state.chips.scope.drop_step(at);
    }
    find_scope(shared)
}

/// The lexicon `girsa-import` wrote, both halves of it, and your own seforim.
///
/// Without it linkify finds nothing — which is the right failure: a citation
/// this build cannot resolve is a citation it must not link.
///
/// Across both roots (G1). Linkify turns a mareh makom a reader **typed** into
/// a link, and a sefer they put on their own shelf is one of the things they
/// may have typed.
fn read_lexicon(
    corpus: &std::path::Path,
    personal: &std::path::Path,
) -> Option<girsa_ref::Lexicon> {
    girsa_corpus::lexicon::Titles::across(corpus, personal)
        .ok()
        .map(|titles| titles.lexicon())
}

/// Everything that comes out of a corpus, opened once.
///
/// This was fourteen lines inside `setup`, which was the right shape while
/// opening a corpus happened exactly once in the life of the process. It
/// happens twice now — at startup, and when a reader points the window at a
/// folder (finding 19) — and the second one has to produce a state
/// indistinguishable from the first. Written as two call sites it would produce
/// a window with a shelf and no timeline, or a shelf and a dead lane, and both
/// of those look like corpus bugs rather than like a missing line.
pub(crate) struct Opened {
    pub(crate) shelf: Option<Shelf>,
    pub(crate) trouble: Option<String>,
    pub(crate) timeline: Option<girsa_corpus::era::Timeline>,
    pub(crate) bar: Option<Arc<Bar>>,
    pub(crate) no_search: Option<String>,
    pub(crate) lexicon: Option<girsa_ref::Lexicon>,
    pub(crate) lane: Option<girsa_nearby::Adjacency>,
}

/// Open everything that hangs off a corpus root.
///
/// **Where the root came from is the caller's business**, and the two callers
/// answer that differently on purpose. At startup nobody has said which corpus,
/// so `roots::corpus` searches in the documented order. When a reader picks a
/// folder they *have* said which, and being told is not a search — running the
/// order there would let a `GIRSA_CORPUS` set for this launch quietly open some
/// other corpus in answer to a folder the reader chose by hand.
pub(crate) fn open_corpus(
    root: Result<PathBuf, String>,
    personal: &std::path::Path,
    showing: girsa_fix::Showing,
) -> Opened {
    let (shelf, trouble) = match root {
        Ok(root) => match Shelf::open(&root, personal) {
            // An unreadable arrangement is the shelf's own trouble to report,
            // and it is not a reason to open no shelf.
            Ok(mut shelf) => {
                let trouble = shelf.trouble().map(ToString::to_string);
                // How much of the correction layer to apply, as it was left
                // last time (W20).
                shelf.set_showing(showing);
                (Some(shelf), trouble)
            }
            Err(e) => (None, Some(e.to_string())),
        },
        Err(e) => (None, Some(e)),
    };
    // Beside the shelf, from the same catalogue file the shelf read. A window
    // that cannot date a work draws `no date`, which is honest; a window that
    // never asked drew a blank, which is not.
    let timeline = shelf
        .as_ref()
        .and_then(|shelf| girsa_corpus::era::Timeline::across(shelf.root(), personal).ok());
    let (bar, no_search) = open_bar_for(&shelf);
    let lexicon = shelf
        .as_ref()
        .and_then(|shelf| read_lexicon(shelf.root(), personal));
    // The lane, once. With it off — the default — this opens nothing and reads
    // nothing; with it on it loads the side-loaded model, which is why it
    // happens here and not on the first query.
    let lane = shelf.as_ref().map(|shelf| {
        let (lane, trouble) = girsa_nearby::Adjacency::open(shelf.root(), personal, shelf);
        for line in trouble {
            eprintln!("the semantic lane: {line}");
        }
        lane
    });
    Opened {
        shelf,
        trouble,
        timeline,
        bar,
        no_search,
        lexicon,
        lane,
    }
}

/// Open the index and put a bar over it.
///
/// The shelf knows where the corpus is; the index is beside it. A window with a
/// shelf and no index reads perfectly well and cannot search, and it says which
/// of those two it is rather than returning nothing.
fn open_bar_for(shelf: &Option<Shelf>) -> (Option<Arc<Bar>>, Option<String>) {
    let Some(shelf) = shelf.as_ref() else {
        return (None, Some(no_shelf_to_search()));
    };
    open_bar(shelf.root(), Some(shelf))
}

/// **Coded, not composed.** This crate's README says it decides nothing, and
/// this sentence reached the first screen of a Hebrew application in English —
/// twice, from two functions, because it was written out twice. The name goes
/// over the wire and `app/src/trouble.ts` says it in the reader's language, the
/// way every other refusal in this repository already worked.
fn no_shelf_to_search() -> String {
    girsa_app::trouble::refuse(
        girsa_app::trouble::Code::NoShelf,
        "there is no shelf to search",
    )
}

fn open_bar(corpus: &std::path::Path, shelf: Option<&Shelf>) -> (Option<Arc<Bar>>, Option<String>) {
    let Some(shelf) = shelf else {
        return (None, Some(no_shelf_to_search()));
    };
    // `girsa_app::find_index`, not a second one here.
    //
    // This file had its own — forty lines above a call to the shared one, in
    // this same file — with the same three candidates in the same order and a
    // *different accept predicate*: it took only `girsa-cache.json`, where the
    // shared one also takes a bare tantivy `meta.json`. A directory
    // `girsa-read` called an index, the window called *no search index*.
    //
    // The shared one now carries this error wording, which is the one thing the
    // copy had that it did not.
    let index_dir = match girsa_app::find_index(corpus) {
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
        let key = girsa_app::taxonomy::shelf_key_of(work, shelf.arrangement(), shelf.shipped());
        catalogue.filed(&work.slug, key.split('/').map(str::to_string).collect());
    }
    // Your own tags, so the tag facet has rows to count and a click has somewhere
    // to narrow to (B18). Tags were counted and shown with no code path by which
    // clicking one could narrow anything.
    let (notes, _) = girsa_note::note::Notes::open(shelf.personal());
    let catalogue = catalogue.tagged(&notes);
    // With your own layer, so citation mode resolves a sefer you put on the
    // shelf yourself by the name you gave it (G1) and then reads its segments
    // from the root that holds them.
    (
        Some(Arc::new(Bar::new(
            index,
            catalogue,
            corpus,
            Some(shelf.personal()),
        ))),
        None,
    )
}
