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
use girsa_app::trouble::{refuse, Code};
use girsa_app::view::{
    AnchorRow, AtRow, Card, Comments, CoveredRow, DrawnRow, Dropped, Fixed, FolderMember,
    FolderRow, GapRow, HitRow, LandingRow, LaneAnswer, LaneProgress, LaneRow, LensRow, Line,
    LinkRow, Links, MarkRow, Mefarshim, ModelOffer, Move, NearRow, NoteRow, OfferRow, PageSaid,
    PageWordsRow, ParaRow, PatchRow, PlaceRow, QueryRow, ReadingRow, Refusal, Said, ScanView,
    ScannedRow, SettingsView, Shortcut, Standing, SuspectRow, TagRow, Text, WordRow, Writing,
    Written, Yours,
};
use girsa_app::workspace::{Axis, PaneId};
use girsa_app::{display, Beside, Session, Shelf, Workspace};
use girsa_corpus::segment::SegmentId;
use girsa_search::bar::{Answer, Bar};
use girsa_search::chips::{Chip, Chips};
use girsa_search::facets::{self, Dimension, Facets, Row};
use girsa_search::index::{Paging, SearchIndex};
use serde::Serialize;

pub(crate) struct State {
    pub(crate) shelf: Option<Shelf>,
    /// The `.ksav` files the reader has told Girsa about (spec.md §10.4).
    ///
    /// Read once and held, like the catalogue: `who_cites` is asked on a click
    /// and re-reading the registry per click would be a file read per click.
    /// The desk's `/document` clears it, because that is where a row is added.
    pub(crate) documents: Option<girsa_desk::documents::Documents>,
    /// When each work was written, read once beside the catalogue.
    ///
    /// The window had no timeline at all, so every row it drew — a search hit,
    /// a lane result, a folder member — carried no date, while `girsa-chain`
    /// and the MCP server both drew one. Which of the four composers had a
    /// `Timeline` in scope was an accident of where the code was written, and
    /// this is the field that ends it.
    pub(crate) timeline: Option<girsa_corpus::era::Timeline>,
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
    /// The reader's own layer, which is beside the session file and **not**
    /// under the corpus.
    ///
    /// Held rather than asked of the shelf, because the one moment it is needed
    /// most is the one moment there is no shelf to ask: a window that opened on
    /// nothing and is being pointed at a folder has to know where the notes go
    /// before it has anywhere to put them.
    personal: PathBuf,
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
    lane: Option<girsa_nearby::Adjacency>,
    /// Set to stop the embedding job. It is checked between batches, so
    /// stopping costs the batch in flight and nothing else.
    stop_embedding: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Slug → the sefer, read once. Cleared oldest-first.
    /// The seforim in memory, most recently read last (`girsa_app::held`).
    open: girsa_app::held::Held<Open>,
    /// Slug → which mefarshim speak on which of its lines (W43).
    ///
    /// Held because ticking a box asks again, and a reader ticking six of them
    /// should not pay six reads of a 3.4 MB file for an answer that cannot have
    /// changed. One `Marks` is the whole sefer's answer; the read that builds it
    /// is 0.07s for Berakhot.
    marks: HashMap<String, girsa_app::mefarshim::Marks>,
    /// `(leader, follower)` → how those two seforim are joined (W9).
    ///
    /// Held for the same reason `marks` is, and against a worse number.
    /// `Beside`'s own doc says it is *"built once per pair of open panes"*, and
    /// `moved` — the scroll handler — built one **per scroll event**: reading
    /// both works' whole `edges.jsonl` and expanding every anchor in them.
    /// Berakhot's shard is 3.4 MB and 21,065 rows.
    ///
    /// `girsa_app::Joined` holds no borrows and depends on nothing that changes
    /// while two panes stay on the same seforim — its inputs are the corpus
    /// shards, which move on a re-import, and the two works' segment positions,
    /// which corrections do not touch. Dropped with the seforim themselves.
    joined: HashMap<(String, String), girsa_app::Joined>,
    /// Slug → what has been read off the pages of that scan (W26).
    ///
    /// Held for the same reason `marks` is. `Words::open` parses the whole
    /// append-only log to answer about one page, and the shell called it
    /// **twice per page read** — `scan_read_page` opened it to record, then
    /// called `scan_reading`, which opened it again to count. Six call sites,
    /// six parses.
    words: HashMap<String, girsa_scan::Words>,
    /// When the session was last written, so that a scroll does not write it.
    ///
    /// `Session::save` serialises the whole workspace **plus the remembered
    /// position of every sefer ever opened**, and `moved` called it on every
    /// scroll event. `session.rs:295` records *"this one is saved on every
    /// scroll"* as a statement of fact rather than as a finding.
    ///
    /// A scroll position is the one thing in the session that is cheap to lose
    /// and cheap to re-earn: the reader is looking at the line. Every actual
    /// **decision** still saves at once — only the scroll is throttled.
    saved_at: std::time::Instant,
}

/// How long a scroll position may go unwritten.
const SAVE_SCROLL_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

impl State {
    pub(crate) fn sefer(&mut self, slug: &str) -> Result<&Open, String> {
        if !self.open.has(slug) {
            let shelf = self.shelf.as_ref().ok_or_else(|| self.trouble())?;
            let read = shelf.read(slug).map_err(|e| e.to_string())?;
            // Whatever was dropped takes its marks table with it: a table of
            // who comments on which line of a sefer nobody has open is the
            // same megabytes with none of the use.
            if let Some(gone) = self.open.put(slug, read) {
                self.marks.remove(&gone);
            }
        }
        self.open.get(slug).ok_or_else(|| "not open".to_string())
    }

    /// Which mefarshim speak on which line of one sefer, read once.
    fn marks(&mut self, slug: &str) -> Result<&girsa_app::mefarshim::Marks, String> {
        if !self.marks.contains_key(slug) {
            let trouble = self.trouble();
            let shelf = self.shelf.as_ref().ok_or(trouble)?;
            let read = girsa_app::mefarshim::Marks::of(shelf, slug).map_err(|e| e.to_string())?;
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
        self.open.forget(slug);
        // The join itself cannot have changed — a correction does not move a
        // segment — but the `Open`s it was computed against are gone, and an
        // answer held about a sefer nobody is holding is a cache outliving its
        // question.
        self.joined
            .retain(|(leader, follower), _| leader != slug && follower != slug);
    }

    /// Forget all of them, which is what *show as printed* costs.
    fn reread_everything(&mut self) {
        self.open.clear();
        self.joined.clear();
    }

    /// Why there is no shelf, named so the window need not read the prose.
    ///
    /// `girsa_app::trouble::refuse` puts a code on the front — `no-shelf: …` —
    /// because `trouble.ts` used to turn this sentence into Hebrew by matching
    /// `/no shelf at/i` against it, which made the wording load-bearing API with
    /// no test on this side of the wire.
    fn trouble(&self) -> String {
        refuse(
            Code::NoShelf,
            self.trouble.as_deref().unwrap_or("there is no shelf here"),
        )
    }

    /// The same trouble, for the window's first screen — named when there is a
    /// name to give it.
    ///
    /// This used to be the bare field, and it is the whole of finding 19: with
    /// no corpus, the top of a right-to-left Hebrew window carried four lines of
    /// Latin paths from `Looked::said` and nothing else, because `Opening`
    /// forwarded the developer's prose while every command in this file was
    /// carefully wrapping the same sentence in a code the window can read.
    ///
    /// Only the no-shelf case gets a name. A shelf that opened with a complaint
    /// about the personal layer is a different fact, it has no code yet, and
    /// giving it this one would tell a reader with 7,189 seforim on the screen
    /// that they have no seforim.
    fn said_trouble(&self) -> Option<String> {
        let prose = self.trouble.as_ref()?;
        Some(if self.shelf.is_none() {
            refuse(Code::NoShelf, prose)
        } else {
            prose.clone()
        })
    }

    fn no_search(&self) -> String {
        refuse(
            Code::NoIndex,
            self.no_search
                .as_deref()
                .unwrap_or("there is no index here"),
        )
    }

    /// The one refusal that is not about the shelf: something panicked while
    /// holding the state, so the lock is poisoned. Ninety-six commands say it.
    fn poisoned() -> String {
        refuse(Code::Poisoned, "state is poisoned")
    }

    /// The documents the reader has told Girsa about, read once.
    ///
    /// Refreshed on open — a `stat` per document, once, rather than a file read
    /// per click. A document saved while the window is up arrives through the
    /// desk's `/document`, which clears this.
    fn documents(&mut self, personal: &std::path::Path) -> &girsa_desk::documents::Documents {
        if self.documents.is_none() {
            let (mut documents, trouble) = girsa_desk::documents::Documents::open(personal);
            for line in trouble {
                eprintln!("{line}");
            }
            if let Err(e) = documents.refreshed() {
                eprintln!("the document registry will not write: {e}");
            }
            self.documents = Some(documents);
        }
        self.documents
            .as_ref()
            .unwrap_or_else(|| unreachable!("just filled"))
    }

    /// What it takes to name a place: the shelf, the dates, and the language
    /// the window is in.
    ///
    /// `None` when there is no shelf, which is the one state in which no row
    /// can be drawn anyway.
    ///
    /// Replaces `State::named`, whose doc said *"there is one rule
    /// (`Language::title_of`) and these are its two callers"* — and by the time
    /// it was read there were six rows working out a title, an address or a
    /// date, and only that one honoured the language.
    fn names(&self) -> Option<girsa_app::Names<'_>> {
        Some(girsa_app::Names::new(
            self.shelf.as_ref()?,
            self.timeline.as_ref(),
            self.session.language,
            // The reader's own citation style, so a row label and the citation
            // they copy off the same line agree.
            self.session.cite,
        ))
    }

    fn save(&mut self) {
        self.saved_at = std::time::Instant::now();
        // A preference file that will not write is not a reason to stop
        // reading. It is a reason to say so once, on the terminal.
        if let Err(e) = self.session.save(&self.session_path) {
            eprintln!("could not save the session: {e}");
        }
    }

    /// Save, unless one was written a moment ago.
    ///
    /// For the scroll handler and nothing else. A *decision* — opening a sefer,
    /// changing a setting, closing a tab — calls [`State::save`], because losing
    /// one of those loses something the reader did.
    fn save_scroll(&mut self) {
        if self.saved_at.elapsed() >= SAVE_SCROLL_EVERY {
            self.save();
        }
    }

    /// What has been read off a scan's pages, parsed once per session.
    ///
    /// # Errors
    ///
    /// If there is no shelf to find the personal layer under.
    fn words(&mut self, slug: &str) -> Result<&mut girsa_scan::Words, String> {
        if !self.words.contains_key(slug) {
            let personal = self
                .shelf
                .as_ref()
                .ok_or("there is no shelf here")?
                .personal()
                .to_path_buf();
            let (words, trouble) = girsa_scan::Words::open(&personal, slug);
            for line in trouble {
                eprintln!("{line}");
            }
            self.words.insert(slug.to_string(), words);
        }
        self.words
            .get_mut(slug)
            .ok_or_else(|| "no words read".to_string())
    }

    /// Work out how two open seforim are joined, once.
    ///
    /// Both must already be in `open`; the caller reads them first because
    /// filling this borrows the map.
    fn join(&mut self, leader: &str, follower: &str, root: &std::path::Path) {
        let key = (leader.to_string(), follower.to_string());
        if self.joined.contains_key(&key) {
            return;
        }
        let (Some(one), Some(two)) = (self.open.peek(leader), self.open.peek(follower)) else {
            return;
        };
        let built = girsa_app::Joined::between(one, two, root);
        self.joined.insert(key, built);
    }
}

pub(crate) type Shared = Mutex<State>;

#[tauri::command]
fn state(shared: tauri::State<'_, Shared>) -> Result<girsa_app::view::Opening, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // The queue is 28,124 lines on the real corpus and this is asked on every
    // redraw, so it is read once and held. `suspects` re-reads it, which is
    // where a run of the batch job is noticed.
    if state.queue.is_none() {
        if let Some(personal) = state.shelf.as_ref().map(|s| s.personal().to_path_buf()) {
            state.queue = Some(girsa_fix::suspect::Queue::open(&personal).0);
        }
    }
    Ok(girsa_app::view::Opening {
        workspace: state.session.workspace.clone(),
        pointing: state.session.pointing,
        text_size: state.session.text_size,
        positions: state.session.positions.clone(),
        works: state.shelf.as_ref().map_or(0, |s| s.works().len()),
        trouble: state.said_trouble(),
        cite: state.session.cite,
        language: state.session.language,
        interface: state.session.interface,
        keys: girsa_app::keys::Bound::of(&state.session.keys)
            .table()
            .clone(),
        look: state.session.look.clone(),
        share_bounds: [
            girsa_app::workspace::SMALLEST_SHARE,
            girsa_app::workspace::LARGEST_SHARE,
        ],
        pairing: state.no_post.clone(),
        showing: state.session.showing,
        fixes: state.shelf.as_ref().map_or(0, |s| s.fixes().count()),
        suspects: state
            .queue
            .as_ref()
            .map_or(0, girsa_fix::suspect::Queue::waiting),
    })
}

/// Point the window at a folder of seforim (finding 19).
///
/// > *"With no corpus, the window is a wall of English file paths. No Hebrew.
/// > No button — although `tauri-plugin-dialog` is already in the build."*
///
/// The wall was the honest half of the answer: `Looked::said` names every
/// directory it tried, in order, because the usual cause is looking one
/// directory away from where the reader is standing. What it could not do is
/// end the problem, and *run girsa-fetch and girsa-import, or set
/// `GIRSA_CORPUS`* is an instruction to somebody who has a terminal open. A
/// reader who already downloaded a corpus needed to be able to say where it is.
///
/// Three decisions worth naming:
///
/// * **The folder is checked before it is remembered**, by the same one-file
///   question `roots::look` asks of every candidate — so *this folder will not
///   do* and *this folder was skipped* can never disagree. The refusal is
///   [`Code::NotACorpus`] and not `NoShelf`: telling somebody who picked their
///   Downloads folder that the import has not run sends them somewhere they do
///   not need to go.
/// * **It opens the folder that was picked**, rather than re-running the search
///   order with it added. See [`open_corpus`].
/// * **Nothing about the old corpus survives.** The seforim in memory, the
///   mefarshim marks, the joins between panes and the OCR queue are all answers
///   about a corpus that is no longer the one being read, and a stale answer
///   about a sefer that still exists in the new corpus is worse than no answer:
///   it looks right.
#[tauri::command]
fn choose_corpus(shared: tauri::State<'_, Shared>, path: String) -> Result<(), String> {
    let at = PathBuf::from(&path);
    if !girsa_corpus::roots::is_corpus(&at) {
        return Err(refuse(
            Code::NotACorpus,
            format!("{path} has no {} in it", girsa_corpus::roots::MARKER),
        ));
    }
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let personal = state.personal.clone();
    let opened = open_corpus(Ok(at.clone()), &personal, state.session.showing);
    // A directory can hold the marker file and still refuse to open — an
    // unreadable catalogue, a half-written import. Remembering that one would
    // put the reader in front of it again on every launch, with the folder they
    // meant forgotten.
    let Some(shelf) = opened.shelf else {
        return Err(refuse(Code::NotACorpus, opened.trouble.unwrap_or(path)));
    };
    state.session.corpus = Some(at);
    state.shelf = Some(shelf);
    state.trouble = opened.trouble;
    state.timeline = opened.timeline;
    state.bar = opened.bar;
    state.no_search = opened.no_search;
    state.lexicon = opened.lexicon;
    state.lane = opened.lane;
    state.open = girsa_app::held::Held::default();
    state.marks.clear();
    state.joined.clear();
    state.words.clear();
    state.documents = None;
    state.queue = None;
    state.save();
    Ok(())
}

#[tauri::command]
fn search(shared: tauri::State<'_, Shared>, query: String) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf
        .search(&query, girsa_app::enough::NAMES_OFFERED)
        .into_iter()
        .map(Card::of)
        .collect())
}

/// The seforim a reader has been in, most recent first — what the picker shows
/// before anything has been typed.
#[tauri::command]
fn recent(shared: tauri::State<'_, Shared>) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
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
        runs: shown(hit, marker, pointing),
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
) -> Vec<display::Run> {
    display::unpointed(
        display::runs_marking(&hit.text, &marker.marks(hit)),
        pointing,
    )
}

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

/// Search, and hand back everything the panel draws.
///
/// The chips are read from what was typed first (a sigil flips a chip — §9.5),
/// so the row that comes back is the row the search actually ran under.
#[tauri::command]
fn find(shared: tauri::State<'_, Shared>, query: String, page: usize) -> Result<FoundPage, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let size = girsa_app::enough::A_PAGE;
    let paging = Paging {
        from: size * page.saturating_sub(1).min(usize::MAX / size.max(1)),
        size,
    };
    // A sigil sets a chip, and the chip stays set — that is what makes typing
    // one a way of *finding* the chips rather than a syntax beside them.
    let (chips, _) = state.chips.read(&query);
    state.chips = chips;
    let pointing = state.session.pointing;
    // How a place is printed, from the reader's own setting.
    let style = state.session.cite;
    let chips = state.chips.clone();
    // What to call each place, in the window's language (W41), with its
    // address and its dates. See `girsa_app::Naming` — four surfaces used to
    // work this out separately and disagreed about all three.
    let names = state.names();
    // **An empty box is not a refusal.** The panel calls this with `""` on open,
    // deliberately, to draw the chip row without running a search — and the
    // engine answered `nothing to search for`, which the window then printed in
    // red, in English, above a row of English chips, before the reader had typed
    // anything. Opening by telling somebody off.
    //
    // Nothing has been asked, so nothing is refused: the chips come back and the
    // header is empty. The engine keeps its refusal, which is the right answer
    // to a *command line* that was given no words.
    if query.trim().is_empty() {
        return Ok(FoundPage::nothing_asked(&chips));
    }
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
            landing,
        } => {
            let pages = results.total.div_ceil(size.max(1));
            FoundPage {
                header: results.header.clone(),
                note,
                hits: results
                    .hits
                    .iter()
                    .map(|hit| hit_row(hit, &results.marker, names.as_ref(), pointing))
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
                landing: landing.map(|landing| landing_row(&landing, state.shelf.as_ref(), style)),
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
            landing: Some(landing_row(&landing, state.shelf.as_ref(), style)),
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
        shelf
            .and_then(|shelf| shelf.work(place.run.first.work()))
            .map_or_else(
                || place.reference.to_string(),
                |work| girsa_app::sending::cite_of(work, &place.run.first, style),
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
#[tauri::command]
fn find_rung(
    shared: tauri::State<'_, Shared>,
    query: String,
    page: usize,
    rung: String,
) -> Result<FoundPage, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let (chips, text) = state.chips.read(&query);
    state.chips = chips.clone();
    let pointing = state.session.pointing;
    let Some(rung) = girsa_search::ladder::Rung::named(&rung) else {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            format!("no such rung: {rung}"),
        ));
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
        from: girsa_app::enough::A_PAGE * page.saturating_sub(1),
        size: girsa_app::enough::A_PAGE,
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
    let names = state.names();
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
            .map(|hit| hit_row(hit, &marker, names.as_ref(), pointing))
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
#[tauri::command]
fn find_chip(shared: tauri::State<'_, Shared>, chip: String, key: String) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.chips.choose(&chip, &key).map_err(|e| e.to_string())
}

/// Click a facet row: narrow to it, or rule it out (spec.md §9.8).
#[tauri::command]
fn find_narrow(
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
#[tauri::command]
fn find_whole_shelf(shared: tauri::State<'_, Shared>) -> Result<(), String> {
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
}

/// Where the search is looking, as a list a panel can draw and edit.
#[derive(Serialize)]
struct ScopeView {
    /// The chip's own sentence, so the panel and the chip cannot word it two
    /// ways.
    said: String,
    steps: Vec<ScopeStep>,
    /// Whether this is the whole shelf — which is not the same as *no steps*
    /// once link types are in it.
    everything: bool,
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
#[tauri::command]
fn find_scope(shared: tauri::State<'_, Shared>) -> Result<ScopeView, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let scope = &state.chips.scope;
    Ok(ScopeView {
        said: scope.describe(),
        steps: scope
            .steps()
            .iter()
            .map(|step| ScopeStep {
                label: step.label.clone(),
                exclude: step.exclude,
                seforim: step.len(),
            })
            .collect(),
        everything: scope.is_everything(),
    })
}

/// Add a shelf or a sefer to where the search looks, or take it out.
///
/// The same two clicks the facet rows carry, reachable **before** a search:
/// `dimension` is `shelf` or `sefer`, `key` is a shelf key or a slug, and
/// `label` is what to call it on the chip. Resolved through the same
/// `facets::narrow`/`exclude` the rail uses, so a scope built from the panel and
/// one built from a result row are the same scope.
#[tauri::command]
fn find_scope_add(
    shared: tauri::State<'_, Shared>,
    dimension: Dimension,
    key: String,
    label: String,
    exclude: bool,
) -> Result<ScopeView, String> {
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let Some(bar) = state.bar.as_ref() else {
            return Err(state.no_search());
        };
        let row = Row {
            key,
            label,
            count: 0,
            depth: 0,
        };
        let scope = if exclude {
            facets::exclude(&state.chips.scope, bar.catalogue(), dimension, &row)
        } else {
            facets::narrow(&state.chips.scope, bar.catalogue(), dimension, &row)
        };
        state.chips.scope = scope;
    }
    find_scope(shared)
}

/// Take one step back — the `×` on a row of the scope panel.
#[tauri::command]
fn find_scope_drop(shared: tauri::State<'_, Shared>, at: usize) -> Result<ScopeView, String> {
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state.chips.scope.drop_step(at);
    }
    find_scope(shared)
}

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

/// Everything that comes out of a corpus, opened once.
///
/// This was fourteen lines inside `setup`, which was the right shape while
/// opening a corpus happened exactly once in the life of the process. It
/// happens twice now — at startup, and when a reader points the window at a
/// folder (finding 19) — and the second one has to produce a state
/// indistinguishable from the first. Written as two call sites it would produce
/// a window with a shelf and no timeline, or a shelf and a dead lane, and both
/// of those look like corpus bugs rather than like a missing line.
struct Opened {
    shelf: Option<Shelf>,
    trouble: Option<String>,
    timeline: Option<girsa_corpus::era::Timeline>,
    bar: Option<Bar>,
    no_search: Option<String>,
    lexicon: Option<girsa_ref::Lexicon>,
    lane: Option<girsa_nearby::Adjacency>,
}

/// Open everything that hangs off a corpus root.
///
/// **Where the root came from is the caller's business**, and the two callers
/// answer that differently on purpose. At startup nobody has said which corpus,
/// so `roots::corpus` searches in the documented order. When a reader picks a
/// folder they *have* said which, and being told is not a search — running the
/// order there would let a `GIRSA_CORPUS` set for this launch quietly open some
/// other corpus in answer to a folder the reader chose by hand.
fn open_corpus(
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
        .and_then(|shelf| girsa_corpus::era::Timeline::of(shelf.root()).ok());
    let (bar, no_search) = open_bar_for(&shelf);
    let lexicon = shelf.as_ref().and_then(|shelf| read_lexicon(shelf.root()));
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
fn open_bar_for(shelf: &Option<Shelf>) -> (Option<Bar>, Option<String>) {
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

fn open_bar(corpus: &std::path::Path, shelf: Option<&Shelf>) -> (Option<Bar>, Option<String>) {
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
    (Some(Bar::new(index, catalogue, corpus)), None)
}

/// The shelf, as a tree — the shipped taxonomy with your arrangement on top.
///
/// Counts only; the seforim themselves come one shelf at a time from
/// [`shelf_works`]. 7,189 cards is not a browse, it is a dump.
#[tauri::command]
fn shelf_tree(shared: tauri::State<'_, Shared>) -> Result<Vec<Branch>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf.tree())
}

#[tauri::command]
fn shelf_works(shared: tauri::State<'_, Shared>, key: String) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(shelf.works_on(&key).into_iter().map(Card::of).collect())
}

/// What these seforim are called, straight off the catalogue.
///
/// # The tab strip that read `bavli/tosafot-on-berakhot` every launch
///
/// A tab knew its Hebrew name only while the pane that made it was in memory.
/// `titleOf()` falls back to the slug and the window filled its name map when a
/// pane was **drawn** — so every tab but the active one was labelled with its
/// English internal id until you visited it. First thing on screen, every
/// launch:
///
/// ```text
/// genesis +2 | mishnah-berurah | שבת ×
/// ```
///
/// The catalogue knows all of them and costs nothing to ask: no segments are
/// read, no sefer is opened, and the answer is one row per slug out of a map
/// the shelf already holds.
#[tauri::command]
fn titles(shared: tauri::State<'_, Shared>, slugs: Vec<String>) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(slugs
        .iter()
        .filter_map(|slug| shelf.work(slug))
        .map(Card::of)
        .collect())
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    shelf.edit(change).map_err(|e| e.to_string())
}

#[tauri::command]
fn companions(shared: tauri::State<'_, Shared>, slug: String) -> Result<Vec<Companion>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
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

/// The mefarshim on one sefer, and what the reader has ticked.
#[tauri::command]
fn mefarshim(shared: tauri::State<'_, Shared>, slug: String) -> Result<Mefarshim, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    mefarshim_of(&mut state, &slug)
}

/// The same, for a caller that already holds the lock.
///
/// # Why ticking goes through here too
///
/// `choose_mefaresh` answered with the **marked lines only**, and the window
/// patched its own copy of the rest: it flipped `chosen` inside `works` and
/// counted that array to decide whether a click on a line means anything. But
/// the list the reader is ticking in is `listed`, which also carries the seforim
/// running alongside and the mefarshim the graph knows and the catalogue does
/// not — tick one of those and `works` never mentioned it, so the count stayed
/// zero and clicking a line did nothing at all. The reader's ninth bug:
/// *"checking off a mefarsh does not open it when its line is clicked."*
///
/// One answer, from the one place that builds it, and the window draws what it
/// is given.
fn mefarshim_of(state: &mut State, slug: &str) -> Result<Mefarshim, String> {
    let slug = slug.to_string();
    let chosen: Vec<String> = state.session.chosen_for(&slug).to_vec();
    let marks = state.marks(&slug)?.clone();
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(Mefarshim::of(shelf, &marks, &slug, &chosen))
}

/// Tick or untick one mefaresh, and answer with the whole list as it stands
/// now.
///
/// The **whole** list, not just the marked lines: see [`mefarshim_of`].
#[tauri::command]
fn choose_mefaresh(
    shared: tauri::State<'_, Shared>,
    slug: String,
    work: String,
    on: bool,
) -> Result<Mefarshim, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.choose(&slug, &work, on);
    state.save();
    mefarshim_of(&mut state, &slug)
}

/// Click a line: read the ticked mefarshim on it.
#[tauri::command]
fn mefarshim_at(
    shared: tauri::State<'_, Shared>,
    slug: String,
    at: String,
) -> Result<Comments, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
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
                .map(|s| Line::of(sefer, s, pointing, style))
                .collect()
        };
        let named = state.shelf.as_ref().and_then(|s| s.work(&one.work));
        said.push(Said {
            he_title: named.map_or_else(|| one.work.clone(), |w| w.he_title.clone()),
            en_title: named.map_or_else(|| one.work.clone(), |w| w.en_title.clone()),
            // `רש״י על ברכות 2a:8:1` was the header on the release build. The
            // address a mefaresh's block carries is the same kind of thing as
            // the one in the margin, and it goes through the same formatter.
            address: named.map_or_else(
                || one.at.address(),
                |w| girsa_app::sending::printed_address(w, &one.at, style),
            ),
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
/// How many segments a pane is handed at a time.
///
/// `pane.ts` draws a window of 400 and grows it by 300 at an edge; this is that
/// window, plus a little either side so the first scroll does not immediately
/// ask for more. It is here rather than in the window because it is what the
/// **wire** carries — see [`Text`], and `examples/measure-opening.rs` for what
/// carrying the whole sefer instead was costing.
const A_WINDOW: usize = 600;

#[tauri::command]
fn open_sefer(shared: tauri::State<'_, Shared>, slug: String) -> Result<Text, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    // Where the reader was, so the window opens around it rather than at the
    // top and then jumping. `where_i_was` is the same memory `Session` keeps for
    // every sefer ever opened (W9).
    let left = state.session.where_i_was(&slug).cloned();
    let sefer = state.sefer(&slug)?;
    // Whether anything in the sefer is pointed. Over the whole of it, because
    // the answer is about the sefer and not about the window a reader happens
    // to be looking at — a Chumash whose first four hundred segments are bare
    // still has nikud.
    let has_nikud = sefer.segments.iter().any(|s| display::has_marks(&s.text));
    let total = sefer.segments.len();
    let at = left.and_then(|id| sefer.position_of(&id)).unwrap_or(0);
    let from = at.saturating_sub(A_WINDOW / 2).min(total.saturating_sub(1));
    let to = (from + A_WINDOW).min(total);
    Ok(Text {
        work: Card::of(&sefer.work),
        lines: sefer.segments[from..to]
            .iter()
            .map(|s| Line::of(sefer, s, pointing, style))
            .collect(),
        from,
        total,
        has_nikud,
    })
}

/// More of a sefer, for a pane that has scrolled to the edge of what it holds.
///
/// Clamped rather than refused: a pane asking past the end is a reader at the
/// end of the sefer, and the honest answer is the last lines rather than an
/// error.
#[tauri::command]
fn sefer_lines(
    shared: tauri::State<'_, Shared>,
    slug: String,
    from: usize,
    count: usize,
) -> Result<Vec<Line>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    let sefer = state.sefer(&slug)?;
    let from = from.min(sefer.segments.len());
    let to = from.saturating_add(count).min(sefer.segments.len());
    Ok(sefer.segments[from..to]
        .iter()
        .map(|s| Line::of(sefer, s, pointing, style))
        .collect())
}

/// Where a segment sits in its sefer, counted from the start.
///
/// What a pane asks when it is told to go to a line it has not loaded — a search
/// hit, a link, a mefaresh's place. `None` is *this sefer does not have that
/// segment*, which is a real answer and not an error: a link can point at a
/// place a re-import moved, and W23's panel is where that gets repaired.
#[tauri::command]
fn sefer_index_of(
    shared: tauri::State<'_, Shared>,
    slug: String,
    at: String,
) -> Result<Option<usize>, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let sefer = state.sefer(&slug)?;
    Ok(sefer.position_of(&at))
}

// ── Scans (spec.md §6.3, BUILDER.md W25) ────────────────────────────────────

/// Open a scan into a pane.
#[tauri::command]
fn scan(shared: tauri::State<'_, Shared>, slug: String) -> Result<ScanView, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&slug)?;
    // Where this scan was left last time. Looked up here rather than worked out
    // in the window from the id it remembered, for the reason in `page_of_id`.
    let left = state.session.positions.get(&slug).cloned();
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let sefer = state.open.peek(&slug).ok_or("not open")?;
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

/// How far a scan has been read.
#[tauri::command]
fn scan_reading(shared: tauri::State<'_, Shared>, slug: String) -> Result<ReadingRow, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&slug)?;
    let personal = state
        .shelf
        .as_ref()
        .ok_or("there is no shelf here")?
        .personal()
        .to_path_buf();
    let sefer = state.open.peek(&slug).ok_or("not open")?;
    let pages = girsa_app::scanning::pages_of(sefer);
    // Held, not re-opened. `Words::open` parses the whole log to answer about
    // one page, and this command is called *again* by `scan_read_page` and
    // `scan_ocr_page` the moment either of them has recorded something.
    let words = state.words(&slug)?;
    let job = girsa_scan::Job::of(&slug, pages, words);
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
    // The shelf has to be there, and this is where that is refused — the
    // personal path itself is `State::words`'s to find now.
    {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        state.shelf.as_ref().ok_or("there is no shelf here")?;
    }
    if width <= 0.0 || height <= 0.0 {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoPage,
            "a page with no size on it",
        ));
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
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state
            .words(&slug)?
            .record(girsa_scan::Read::new(
                page,
                girsa_scan::Reader::Embedded,
                grouped.words,
            ))
            .map_err(|e| e.to_string())?;
    }
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
        let state = shared.lock().map_err(|_| State::poisoned())?;
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
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state
            .words(&slug)?
            .record(read)
            .map_err(|e| e.to_string())?;
    }
    scan_reading(shared, slug)
}

/// What is on a page, for drawing a highlight over the photograph.
#[tauri::command]
fn scan_words(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<Option<PageWordsRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let words = state.words(&slug)?;
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
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let words = state.words(&slug)?;
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
    }
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
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let personal = shelf.personal().to_path_buf();
    // Where the index is, if it is anywhere: two of the three gaps are *since the
    // index was built*, so a window that cannot find one has a bigger gap to
    // report, not a smaller one.
    let index = girsa_app::find_index(shelf.root()).ok();
    let gap = girsa_app::reading::gap(shelf, &personal, index.as_deref());
    // Through `Unseen` rather than `Gap::said` directly: the window's header and
    // the MCP server's `did_not_search` are one sentence with one separator, and
    // the lane clause is one argument away from belonging here too.
    let unseen = girsa_nearby::Unseen::literal(gap);
    let gap = &unseen.literal;
    Ok(unseen.said().map(|said| GapRow {
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
        corrected_scans: gap.layer.scans.count(),
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
        // Coded, like every other refusal that crosses. `naming` fails one way
        // — the sefer this scan says it is a scan of is not on the shelf — and
        // the window used to print `ShelfError`'s English straight into the
        // note under the page.
        trouble: girsa_app::scanning::naming(shelf, scan)
            .err()
            .map(|e| refuse(Code::NoSefer, e)),
    }
}

/// What is printed on a page.
#[tauri::command]
fn scan_at(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<PageSaid, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let style = state.session.cite;
    state.sefer(&slug)?;
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let sefer = state.open.peek(&slug).ok_or("not open")?;
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

    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&slug)?;
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let sefer = state.open.peek(&slug).ok_or("not open")?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let style = state.session.cite;
    state.sefer(&slug)?;
    let shelf = state.shelf.as_ref().ok_or("there is no shelf here")?;
    let sefer = state.open.peek(&slug).ok_or("not open")?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    let slug = at.work().to_string();

    let patch = {
        let sefer = state.sefer(&slug)?;
        let mut patch = girsa_app::correction(
            sefer,
            &at,
            from_char..to_char,
            &now,
            kind,
            &girsa_app::who(),
            pointing,
        )
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
        line: Line::of(sefer, segment, pointing, style),
        said: format!("{was} → {now}"),
    })
}

/// Take a correction back.
#[tauri::command]
fn unfix(shared: tauri::State<'_, Shared>, at: String, patch: String) -> Result<Fixed, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    let slug = at.work().to_string();

    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let gone = shelf
        .unfix(&girsa_fix::PatchId::from(patch))
        .map_err(|e| e.to_string())?;
    if !gone {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            "there is no such correction",
        ));
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
        line: Line::of(sefer, segment, pointing, style),
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.showing = showing;
    let trouble = state.trouble();
    state.shelf.as_mut().ok_or(trouble)?.set_showing(showing);
    state.reread_everything();
    state.save();
    Ok(())
}

#[tauri::command]
fn fixes(shared: tauri::State<'_, Shared>, slug: Option<String>) -> Result<Vec<PatchRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    // A patch row is a row about a place, like a hit and like a lane result —
    // and it was the fifth to work out a title and an address for itself.
    let names = state.names().ok_or_else(|| state.trouble())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let mut rows: Vec<PatchRow> = shelf
        .fixes()
        .all()
        .filter(|p| slug.as_ref().is_none_or(|s| p.segment.work() == s))
        .map(|p| PatchRow {
            id: p.id.to_string(),
            segment: p.segment.to_string(),
            work: names.of(&p.segment).work,
            title: names.of(&p.segment).title,
            address: p.segment.address(),
            kind: p.kind.as_str(),
            was: p.was.clone(),
            now: p.now.clone(),
            who: p.who.clone(),
            when: p.when,
            note: p.note.clone(),
            source: p.source.clone(),
        })
        .collect();
    girsa_app::view::PatchRow::newest_first(&mut rows);
    Ok(rows)
}

// ── The links on a line, and repairing them (spec.md §8.3, W23) ─────────────

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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let lens = lens.filter(|key| !key.is_empty());

    // The line itself, as the pane drew it, because a span is in those
    // characters (W20's two coordinate systems, again).
    // …and, from the same read, every name these words have carried, so an edge
    // stored under the name this place had before a corpus update still finds
    // it (see `girsa_corpus::standing`).
    let (base, anchors, standing) = {
        let sefer = state.sefer(at.work())?;
        let nth = sefer
            .position_of(&at)
            .ok_or_else(|| format!("{at} is not in this sefer"))?;
        // The anchors travel with the text. They are the segment's own statement
        // of where its commentaries attach, and reading them costs nothing that
        // reading the text did not already cost.
        let segment = sefer.segments.get(nth);
        let text = segment.map(|s| s.text.clone()).unwrap_or_default();
        let anchors = segment.map(|s| s.anchors.clone()).unwrap_or_default();
        (text, anchors, sefer.standing(&at))
    };

    let language = state.session.language;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let touching = girsa_app::touching(shelf, shelf.repairs(), &standing);
    let mut links = touching.links;

    // The words each link is about, where anything says — and the far end's
    // text is only consulted for seforim that are **already open**.
    for link in &mut links {
        let far = state.open.peek(&link.work);
        link.span = girsa_app::links::span_on(link, &at, &base, &anchors, far, pointing);
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
        links: links
            .iter()
            .map(|l| LinkRow::of(l, language, first_words(&state, l, pointing)))
            .collect(),
        incoming_unknown: touching.incoming_unknown,
        types: girsa_app::links::kinds(),
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
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NothingChosen,
            "nothing is selected",
        ));
    }
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = girsa_app::who();
    shelf
        .repairs_mut()
        .pin_named(&edge, &at, from_char..to_char, &who)
        .map_err(|e| e.to_string())
}

/// The first words at the other end of a link, where that sefer is already read
/// (W37).
///
/// **Already read only.** Same rule as `girsa_app::links::span_on`, and the same
/// reason it gives: a sidebar is not entitled to open forty seforim to decorate
/// itself. The rows without one still name the sefer and the place, which is what
/// every row said before this.
fn first_words(
    state: &State,
    link: &girsa_app::Link,
    pointing: girsa_app::session::Pointing,
) -> Option<String> {
    let sefer = state.open.peek(&link.work)?;
    let nth = sefer.position_of(&link.other.from)?;
    let text = display::Shown::of(&sefer.segments.get(nth)?.text, pointing)
        .text()
        .to_string();
    Some(girsa_app::enough::first_words(&text))
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = girsa_app::who();
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
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such repair: {other}"),
            ))
        }
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
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such end: {other}"),
            ))
        }
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = girsa_app::who();
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
    use girsa_app::trouble::{refuse, Code};
    let from: SegmentId = from
        .parse()
        .map_err(|e| refuse(Code::NoSuch, format_args!("{e}")))?;
    let to: SegmentId = to
        .parse()
        .map_err(|e| refuse(Code::NoSuch, format_args!("{e}")))?;
    let edge_type = girsa_link::touching::type_named(&kind)
        .ok_or_else(|| refuse(Code::NoSuch, format_args!("no such link type: {kind}")))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = girsa_app::who();
    shelf
        .repairs_mut()
        .draw(
            girsa_link::Anchor::point(from),
            girsa_link::Anchor::point(to),
            edge_type,
            &who,
        )
        .map_err(|e| refuse(Code::ReadOnly, e))
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

/// Write a sefer out with your corrections in it, into a folder you chose.
///
/// # It used to choose for you
///
/// *"send to ksav and export dont let you pick a folder."* They did not: this
/// wrote into `personal/exports/` and said the path afterwards, and the comment
/// where this one is argued that *"the file is the point and where it goes is
/// not, and a reader who wants it somewhere else has a file manager."* That is
/// a reasonable thing to believe about a debug artefact and the wrong thing to
/// believe about a sefer somebody is going to hand to a chavrusa — the whole
/// reason to export is to put the file **somewhere**, and the somewhere is the
/// reader's business.
///
/// `into` is a folder the window got from a real directory dialog. `None` is
/// *the last one you chose*, and failing that the old default — so the second
/// export does not ask again and a reader who never opens the dialog is exactly
/// where they were.
#[tauri::command]
fn export_sefer(
    shared: tauri::State<'_, Shared>,
    slug: String,
    format: String,
    into: Option<String>,
) -> Result<Written, String> {
    let format =
        girsa_export::Format::named(&format).ok_or_else(|| format!("no such format: {format}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let showing = state.session.showing;
    let personal = state
        .shelf
        .as_ref()
        .ok_or_else(|| state.trouble())?
        .personal()
        .to_path_buf();
    // Remembered, so the next export opens where the last one went.
    if let Some(chosen) = into.as_ref().filter(|p| !p.trim().is_empty()) {
        state.session.export_into = Some(chosen.clone());
        state.save();
    }
    let folder = state
        .session
        .export_into
        .clone()
        .map_or_else(|| personal.join("exports"), PathBuf::from);

    // The sefer as it is being read — corrections already applied, because
    // that is what `Open` is (W20). Nothing is applied here.
    let sefer = state.sefer(&slug)?;
    let to = folder.join(girsa_export::suggested_name(sefer, format));
    let fixes = state
        .shelf
        .as_ref()
        .ok_or("there is no shelf here")?
        .fixes();
    let sefer = state.open.peek(&slug).ok_or("not open")?;
    let done =
        girsa_export::export(sefer, fixes, format, pointing, &to).map_err(|e| e.to_string())?;
    Ok(Written {
        said: format!(
            "{} · {} · {}",
            done.path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            girsa_export::showing_said(showing),
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

/// The next candidates to review, best first.
///
/// Re-read from disk every time: `girsa-suspects` is a batch job that runs
/// outside this window, and a queue held in memory would be the one from
/// before it ran.
#[tauri::command]
fn suspects(shared: tauri::State<'_, Shared>, limit: usize) -> Result<Vec<SuspectRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let names = state.names().ok_or_else(|| state.trouble())?;
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
                // The sixth. `Names::of` falls back to the slug rather than to
                // `None`, so a suspect in a sefer the catalogue has not caught
                // up with now draws a row with a name on it.
                title: at.map(|id| names.of(id).title),
                address: at.map(girsa_corpus::segment::SegmentId::address),
            }
        })
        .collect();
    state.queue = Some(queue);
    Ok(rows)
}

/// Open a candidate: where its word sits in the segment the queue named.
#[tauri::command]
fn suspect_at(
    shared: tauri::State<'_, Shared>,
    id: String,
    at: String,
) -> Result<Standing, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let (queue, _) = girsa_fix::suspect::Queue::open(shelf.personal());
    let suspect = queue.get(&id).ok_or("there is no such candidate")?.clone();
    state.queue = Some(queue);

    let sefer = state.sefer(at.work())?;
    let span = girsa_app::fixing::where_word(sefer, &at, &suspect.rare, pointing)
        .ok_or("that word is not in that line any more")?;
    let drawn = girsa_app::display::Shown::of(
        &sefer
            .segments
            .get(sefer.position_of(&at).ok_or("not in this sefer")?)
            .ok_or("not in this sefer")?
            .text,
        pointing,
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
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such decision: {other}"),
            ))
        }
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
        Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            "there is no such candidate",
        ))
    }
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent =
        girsa_app::send(sefer, &selection, style, pointing, note).map_err(|e| e.to_string())?;
    Ok(Copied {
        display: sent.display().to_string(),
        reference: sent.packet.reference.clone(),
        lines: sent.packet.text.lines().count(),
        put: clipboard::put(&sent),
    })
}

// ── The buffer (spec.md §10.3, BUILDER.md W17) ──────────────────────────────

#[tauri::command]
fn buffers(shared: tauri::State<'_, Shared>) -> Result<Vec<String>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    Ok(girsa_desk::Buffer::list(shelf.personal()))
}

#[tauri::command]
fn buffer_open(shared: tauri::State<'_, Shared>, name: String) -> Result<Writing, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let buffer = girsa_desk::Buffer::open(shelf.personal(), &name).map_err(|e| e.to_string())?;
    let path = girsa_desk::Buffer::path(shelf.personal(), &name).map_err(|e| e.to_string())?;
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
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let mut buffer = girsa_desk::Buffer::new(name);
    buffer.text = text;
    Ok(buffer
        .save(shelf.personal())
        .map_err(|e| e.to_string())?
        .display()
        .to_string())
}

/// Write the document out into a folder the reader chose.
///
/// The other half of *"send to ksav and export dont let you pick a folder"*.
/// The working buffer stays where it lives — `personal/ksav/` is what
/// `buffers()` lists, and a buffer that wandered off would be a document the
/// drawer could no longer find — so this writes a **copy** where it was asked
/// to, which is what *save a copy* means everywhere else.
///
/// # Errors
///
/// If the folder will not take the file, or the name is not one.
#[tauri::command]
fn buffer_write_to(
    shared: tauri::State<'_, Shared>,
    name: String,
    text: String,
    into: String,
) -> Result<String, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let folder = PathBuf::from(into.trim());
    if folder.as_os_str().is_empty() {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NothingChosen,
            "no folder was chosen",
        ));
    }
    let named = name.trim();
    if named.is_empty() || named.contains(['/', '\\']) {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            format!("`{name}` is not a name for a document"),
        ));
    }
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join(format!("{named}.ksav"));
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    state.session.export_into = Some(folder.display().to_string());
    state.save();
    Ok(path.display().to_string())
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent =
        girsa_app::send(sefer, &selection, style, pointing, None).map_err(|e| e.to_string())?;
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
) -> Result<Vec<girsa_desk::Citing>, String> {
    let place: girsa_ref::Ref = reference.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let personal = state
        .shelf
        .as_ref()
        .ok_or_else(|| state.trouble())?
        .personal()
        .to_path_buf();
    let documents = state.documents(&personal);
    Ok(girsa_desk::who_cites(&personal, documents, &place))
}

/// The citations in a piece of prose — **the certain ones** (spec.md §10.5).
///
/// Everything ambiguous stays plain text. See `girsa_desk::citing` for the three
/// rules and why each of them refuses more than it accepts.
#[tauri::command]
fn linkify(
    shared: tauri::State<'_, Shared>,
    text: String,
) -> Result<Vec<girsa_desk::Linked>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let lexicon = state
        .lexicon
        .as_ref()
        .ok_or("there is no lexicon here — has girsa-import run?")?;
    Ok(girsa_desk::linkify(lexicon, &text))
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent =
        girsa_app::send(sefer, &selection, style, pointing, note).map_err(|e| e.to_string())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.cite =
        girsa_cite::CiteStyle::named(&style).ok_or_else(|| format!("no such style: {style}"))?;
    state.save();
    Ok(())
}

#[tauri::command]
fn open_tab(shared: tauri::State<'_, Shared>, slug: String) -> Result<PaneId, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // A sefer reopens where it was left, which is the whole point of
    // remembering (BUILDER.md W9, per-sefer position memory).
    let at = state.session.where_i_was(&slug).cloned();
    // **Go to it if it is open**, rather than opening a second tab on one sefer
    // — `Workspace::open`, and the reason is in its doc comment.
    let pane = state.session.workspace.open(&slug, at);
    state.save();
    Ok(pane)
}

/// One sefer that is open, for the switcher.
#[derive(Serialize)]
struct OpenSefer {
    slug: String,
    /// What to call it, in the window's language (W41).
    title: String,
    /// Whether it is the one being read right now.
    here: bool,
}

/// Every sefer that is open, most recently read first.
///
/// The open set is not the tab strip — see `girsa_app::workspace::Workspace`.
/// A tab holding a Gemara, its Rashi and its Tosafos is one entry in the strip
/// and three seforim that are open, and the strip cannot say so.
#[tauri::command]
fn open_set(shared: tauri::State<'_, Shared>) -> Result<Vec<OpenSefer>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let language = state.session.language;
    let here = state
        .session
        .workspace
        .active_tab()
        .and_then(|tab| tab.pane(tab.focused))
        .map(|pane| pane.slug.clone());
    let shelf = state.shelf.as_ref();
    Ok(state
        .session
        .workspace
        .open_set()
        .into_iter()
        .map(|slug| {
            let named = shelf.and_then(|s| s.work(&slug));
            OpenSefer {
                title: named.map_or_else(
                    || slug.clone(),
                    |w| language.title_of(&w.he_title, &w.en_title).to_string(),
                ),
                here: here.as_deref() == Some(slug.as_str()),
                slug,
            }
        })
        .collect())
}

#[tauri::command]
fn split(
    shared: tauri::State<'_, Shared>,
    pane: PaneId,
    axis: String,
    slug: String,
    follow: bool,
) -> Result<Option<PaneId>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.close(pane);
    state.save();
    Ok(())
}

/// Close a whole tab, from the tab strip, without opening it first (W40).
#[tauri::command]
fn close_tab(shared: tauri::State<'_, Shared>, index: usize) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.close_tab(index);
    state.save();
    Ok(())
}

#[tauri::command]
fn focus(shared: tauri::State<'_, Shared>, pane: PaneId) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.set_follows(pane, leader);
    state.save();
    Ok(())
}

#[tauri::command]
fn set_ratio(shared: tauri::State<'_, Shared>, pane: PaneId, ratio: u16) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.set_ratio(pane, ratio);
    state.save();
    Ok(())
}

#[tauri::command]
fn set_pointing(shared: tauri::State<'_, Shared>, pointing: String) -> Result<(), String> {
    let Some(pointing) = girsa_app::session::Pointing::named(&pointing) else {
        // Refused by name rather than defaulted. A window that sent a spelling
        // this project does not write has a wiring bug, and silently drawing
        // the sefer with everything on is how it would never be found —
        // `girsa_search::chips::choose` says the same thing at more length.
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            format!("no such pointing: {pointing}"),
        ));
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.pointing = pointing;
    state.save();
    Ok(())
}

#[tauri::command]
fn settings(shared: tauri::State<'_, Shared>) -> Result<SettingsView, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let session = &state.session;
    let bound = girsa_app::keys::Bound::of(&session.keys);
    Ok(SettingsView {
        pointing: session.pointing,
        text_size: session.text_size,
        language: session.language,
        interface: session.interface,
        cite: session.cite,
        showing: session.showing,
        theme: session.look.theme.as_str(),
        hebrew_font: session.look.hebrew_font.clone(),
        latin_font: session.look.latin_font.clone(),
        line_height: session.look.line_height,
        column_ch: session.look.column_ch,
        share_bounds: [
            girsa_app::workspace::SMALLEST_SHARE,
            girsa_app::workspace::LARGEST_SHARE,
        ],
        shortcuts: girsa_app::keys::ACTIONS
            .iter()
            .map(|action| Shortcut {
                id: action.id,
                he: action.he,
                en: action.en,
                bound: bound.on(action.id),
                shipped: action.default,
            })
            .collect(),
        fonts: girsa_app::session::FONTS
            .iter()
            .map(|f| (*f).to_string())
            .collect(),
    })
}

/// How the reading looks (B13).
#[tauri::command]
fn set_look(
    shared: tauri::State<'_, Shared>,
    look: girsa_app::session::Look,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // Clamped in `girsa_app`, once — and by the same call `load` makes, so a
    // hand-edited session file is held to the same bounds a setter is. A window
    // that clamped and a command that clamped again is two readers of one rule,
    // which is what B27 is about.
    state.session.look = look;
    state.session.sane();
    state.save();
    Ok(())
}

/// Rebind one shortcut, or put it back (B13).
///
/// An empty `to` **removes the reader's binding** rather than binding nothing, so
/// the action goes back to whatever the table ships with. That is what a reset
/// button needs and it is one code path rather than two.
#[tauri::command]
fn bind_key(
    shared: tauri::State<'_, Shared>,
    action: String,
    to: String,
) -> Result<Vec<Shortcut>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    if to.trim().is_empty() {
        state.session.keys.remove(&action);
    } else if let Some(press) = girsa_app::keys::Press::parse(&to) {
        // Stored in the one spelling, so a session file cannot hold two names for
        // one combination.
        state.session.keys.insert(action, press.said());
    } else {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            format!("{to} is not a key combination"),
        ));
    }
    state.save();
    let bound = girsa_app::keys::Bound::of(&state.session.keys);
    Ok(girsa_app::keys::ACTIONS
        .iter()
        .map(|action| Shortcut {
            id: action.id,
            he: action.he,
            en: action.en,
            bound: bound.on(action.id),
            shipped: action.default,
        })
        .collect())
}

/// What a key press means (B13). The window asks; the table answers.
#[tauri::command]
fn what_key(
    shared: tauri::State<'_, Shared>,
    press: girsa_app::keys::Press,
) -> Result<Option<String>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    Ok(girsa_app::keys::Bound::of(&state.session.keys)
        .what(&press)
        .map(ToString::to_string))
}

/// Which language the window is in (W41).
///
/// Every sefer name in the window follows it, so the whole reason it is one
/// setting and not a per-row choice is that a shelf half in each is unreadable.
#[tauri::command]
fn set_language(
    shared: tauri::State<'_, Shared>,
    language: girsa_app::session::Language,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.language = language;
    state.save();
    Ok(())
}

/// What language the **window** speaks — as against what the seforim are called.
///
/// > *"there is no way to change UI into english - only seforim names. there
/// > should be 2 seperate commands."*
///
/// There is now, and they are two because they are two questions. A reader who
/// learns in Hebrew and wants the buttons in English is ordinary; so is the
/// reverse; and one setting could serve neither.
#[tauri::command]
fn set_interface(
    shared: tauri::State<'_, Shared>,
    language: girsa_app::session::Language,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.interface = language;
    state.save();
    Ok(())
}

#[tauri::command]
fn set_text_size(shared: tauri::State<'_, Shared>, percent: u16) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // The clamp is `Session::sane`, which is also what `load` runs — this line
    // used to be `percent.clamp(60, 250)`, sixty-eight lines below a doc comment
    // saying the clamping happens *"in one place, here, rather than in the
    // window and again in the command."*
    state.session.text_size = percent;
    state.session.sane();
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
    let state = shared.lock().map_err(|_| State::poisoned())?;
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
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let lane = state.lane.as_ref().ok_or_else(|| state.trouble())?;
    // Scoped by the same chip the literal search is scoped by, so *the whole
    // shelf* and *this sefer* mean the same thing in both columns.
    let scoped: Vec<String> = state.chips.scope.works().into_iter().collect();
    // The same `Names` the search column uses, so the two columns beside each
    // other cannot call one sefer by two names.
    let names = girsa_app::Names::new(
        shelf,
        state.timeline.as_ref(),
        state.session.language,
        state.session.cite,
    );
    let answer = lane.ask(&names, &text, &scoped, limit.unwrap_or(girsa_lane::MOST));
    Ok(LaneAnswer {
        label: answer.label,
        measured: answer.measured,
        near: answer
            .near
            .iter()
            .map(|near| NearRow {
                at: AtRow::of(&near.at),
                text: near.text.clone(),
                nearness: near.nearness,
            })
            .collect(),
        coverage: answer.coverage,
        refused: answer.refused,
        shortlisted: answer.shortlisted,
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
        let state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("{slug} was not in the lane"),
            ));
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
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
        let held = state.lane.as_ref().ok_or("there is no lane here")?;
        if !held.state().is_on() {
            return Err(girsa_lane::LaneError::Off.to_string());
        }
        let slugs = girsa_nearby::adjacent::in_the_lane(shelf, held.lane().chosen());
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
    let state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;

    state.session.remember(at.clone());
    let followers = state.session.workspace.moved(pane, at.clone());
    if followers.is_empty() {
        // `save_scroll`, not `save`: this is a scroll position, and
        // `Session::save` writes the whole workspace plus the remembered place
        // of every sefer ever opened.
        state.save_scroll();
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
    let mut alongside: Vec<(PaneId, String)> = Vec::new();
    for (id, slug) in wanted {
        let Some(follower) = state.open.peek(&slug) else {
            continue;
        };
        // A scan follows the sefer beside it by turning to the page the daf is
        // printed on — but only where the reader has said it is a scan **of**
        // that sefer (W25). Which `Place` that is, and which `Relation`, are
        // `Beside`'s to decide: this block used to work both out by hand and
        // synthesise `Relation::Declared { follower_is_commentary: false }` out
        // of nothing, with `Beside::between` reached only in the `else` below —
        // so a scan open beside a Gemara, the case W9 was accepted on, never
        // touched the tested path.
        if let Some(shelf) = state.shelf.as_ref() {
            if let Some(scan) = girsa_app::scan_of(shelf, follower) {
                let joined = girsa_app::Joined::over_scan(scan, &leader_slug);
                let beside = Beside::over(follower, &joined);
                moves.push(Move {
                    pane: id,
                    place: beside.place(&at),
                    relation: beside.relation(),
                    page: beside.page(&at),
                });
                continue;
            }
        }
        alongside.push((id, slug));
    }

    // The joins, worked out once per pair rather than once per scroll event.
    // A second pass because filling the cache borrows `state` mutably while the
    // loop above is holding two seforim out of it.
    for (_, slug) in &alongside {
        state.join(&leader_slug, slug, &root);
    }
    for (id, slug) in alongside {
        // Two immutable borrows of two different fields, which is why the
        // cache is filled above rather than here.
        let (Some(joined), Some(follower)) = (
            state.joined.get(&(leader_slug.clone(), slug.clone())),
            state.open.peek(&slug),
        ) else {
            continue;
        };
        let beside = Beside::over(follower, joined);
        moves.push(Move {
            pane: id,
            place: beside.place(&at),
            relation: beside.relation(),
            page: None,
        });
    }
    state.save_scroll();
    Ok(moves)
}

/// Open the window.
///
/// If it cannot be opened at all there is nothing to carry on into, so this
/// says so and exits non-zero. It used to `expect`, which is the same outcome
/// wearing a backtrace — and it was the only `unwrap`/`expect` in this file,
/// which is now denied here rather than merely avoided (see `Cargo.toml`).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let built = tauri::Builder::default();

    // **First**, before every other plugin, because it decides whether this
    // process is going to be an application at all.
    //
    // Measured on the release build with Girsa already open: firing
    // `girsa:bavli/shabbat/12b:3#242` opened a *second copy* and left the
    // running window exactly where it was. The forward half of the Ksav loop
    // worked and the return half — click a citation in your document, land on
    // the daf — delivered the reader a duplicate application with its own
    // workspace, both halves then writing one `session.json`.
    //
    // The deep-link listener below was never wrong; on Windows and Linux it
    // only ever hears a *cold* start, and handing a URL to a process that is
    // already running is this plugin's job. With the `deep-link` feature it
    // forwards the argv into that same listener, so there is nothing here to
    // keep in step with it.
    #[cfg(any(windows, target_os = "linux"))]
    let built = built.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        // The URL routes itself. What is left is the part a reader notices: the
        // window they already had is behind their document, and a citation that
        // silently scrolled a hidden window would look exactly like a citation
        // that did nothing.
        use tauri::Manager as _;
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }));

    let built = built
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
            let personal = girsa_corpus::roots::personal(&data);

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

            let opened = open_corpus(
                girsa_corpus::roots::corpus(session.corpus.as_deref()),
                &personal,
                session.showing,
            );
            let Opened {
                shelf,
                trouble,
                timeline,
                bar,
                no_search,
                lexicon,
                lane,
            } = opened;
            tauri::Manager::manage(
                app,
                Mutex::new(State {
                    shelf,
                    documents: None,
                    timeline,
                    bar,
                    no_search,
                    chips: Chips::default(),
                    trouble,
                    personal,
                    session,
                    session_path,
                    desk: None,
                    no_post: None,
                    lexicon,
                    queue: None,
                    lane,
                    stop_embedding: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    open: girsa_app::held::Held::default(),
                    marks: HashMap::new(),
                    joined: HashMap::new(),
                    words: HashMap::new(),
                    saved_at: std::time::Instant::now(),
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
            choose_corpus,
            search,
            recent,
            companions,
            mefarshim,
            choose_mefaresh,
            mefarshim_at,
            open_sefer,
            sefer_lines,
            sefer_index_of,
            open_tab,
            open_set,
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
            close_tab,
            focus,
            set_follows,
            set_ratio,
            set_pointing,
            set_language,
            set_interface,
            settings,
            set_look,
            bind_key,
            what_key,
            set_text_size,
            moved,
            shelf_tree,
            shelf_works,
            titles,
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
            find_scope,
            find_scope_add,
            find_scope_drop,
            copy,
            set_cite_style,
            ksav_presence,
            send_to_ksav,
            buffers,
            buffer_open,
            buffer_save,
            buffer_write_to,
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
        // `build` and then `run(callback)`, where it used to be `run(context)`.
        //
        // The difference is the only place this application can be told it is
        // about to stop. `Builder::run` takes no callback and, on Windows, never
        // returns — the event loop calls `exit()` — so nothing managed is ever
        // dropped. Which made the note on `State::desk` false: it says *"dropping
        // it withdraws the endpoint file — which is exactly how presence stops
        // being reported the moment this application stops"*, and `Desk::drop`
        // was never reached by any exit a reader can perform.
        //
        // Measured: close the window, and `girsa-endpoint.json` is still there
        // naming a dead pid. So Ksav, which reads that file to find us, saw
        // every ordinary close as `Presence::Stale` — *registered but not
        // answering, it may have closed badly*. That state exists for the crash
        // case and had quietly become the normal one.
        .build(tauri::generate_context!());
    match built {
        Ok(app) => app.run(|handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                // Taking the desk out drops it, and `Desk::drop` unblocks the
                // listener and withdraws the endpoint — both halves, in that
                // order, which is the crate's own rule about which of the two
                // may outlive the other.
                use tauri::Manager as _;
                if let Some(shared) = handle.try_state::<Shared>() {
                    if let Ok(mut state) = shared.lock() {
                        state.desk = None;
                    }
                }
            }
        }),
        Err(e) => {
            // A sentence a reader can act on, not a panic message. The rest of
            // this shell refuses legibly; the one path that can only stop should
            // too.
            eprintln!("Girsa could not open its window: {e}");
            std::process::exit(1);
        }
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

/// What you have on the line you are standing on.
#[tauri::command]
fn yours(shared: tauri::State<'_, Shared>, at: String) -> Result<Yours, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;

    // The line as the pane drew it — corrected, and with the nikud the reader
    // has on — because that is the string a highlight's offsets are against.
    let (base, standing) = {
        let sefer = state.sefer(at.work())?;
        let text = sefer
            .position_of(&at)
            .and_then(|nth| sefer.segments.get(nth))
            .map(|segment| segment.text.clone())
            .unwrap_or_default();
        // Your notes, highlights and folders are anchored under the name this
        // place had when you wrote them, which a corpus update may have moved.
        (text, sefer.standing(&at))
    };
    let pointing = state.session.pointing;
    let drawn = display::Shown::of(&base, pointing).text().to_string();

    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let found = girsa_app::yours(shelf, &standing, &drawn);
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
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let mut rows: Vec<NoteRow> = shelf.notes().all().map(NoteRow::of).collect();
    girsa_app::view::NoteRow::newest_first(&mut rows);
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    let who = girsa_app::who();
    let note = girsa_app::note_here(shelf, &at, title.as_deref(), &text, &who)
        .map_err(|e| e.to_string())?;
    Ok(NoteRow::of(&note))
}

/// One note, paragraph by paragraph, for editing it.
#[tauri::command]
fn note_read(shared: tauri::State<'_, Shared>, note: String) -> Result<Vec<ParaRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
                return Err(girsa_app::trouble::refuse(
                    girsa_app::trouble::Code::NoSuch,
                    "that paragraph is not in this note",
                ));
            }
        }
        "remove" => {
            if !held.remove(&paragraph(&value)?) {
                return Err(girsa_app::trouble::refuse(
                    girsa_app::trouble::Code::NoSuch,
                    "that paragraph is not in this note",
                ));
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
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such edit: {other}"),
            ))
        }
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    // A note is a sefer, and a sefer that has gone may not stay open in a pane
    // holding text nothing on the shelf accounts for.
    let slug = shelf.notes().get(&note).map(|held| held.slug.clone());
    let gone = shelf.forget_note(&note).map_err(|e| e.to_string())?;
    if let Some(slug) = slug {
        state.open.forget(&slug);
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let base = {
        let sefer = state.sefer(at.work())?;
        sefer
            .position_of(&at)
            .and_then(|nth| sefer.segments.get(nth))
            .map(|segment| segment.text.clone())
            .unwrap_or_default()
    };
    let drawn = display::Shown::of(&base, pointing).text().to_string();

    let who = girsa_app::who();
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let drawn: HashMap<String, String> = {
        let sefer = state.sefer(&slug)?;
        sefer
            .segments
            .iter()
            .map(|segment| {
                (
                    segment.id.to_string(),
                    display::Shown::of(&segment.text, pointing)
                        .text()
                        .to_string(),
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
    let state = shared.lock().map_err(|_| State::poisoned())?;
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

/// Keep the question you just asked.
///
/// The chips are saved as the `chip → key` pairs the row itself sends, so
/// recalling one goes through the same `Chips::choose` a click does. The scope is
/// saved as the seforim it comes to — a scope narrowed by three clicks comes
/// back as one clause over the same seforim, which matches the same segments
/// and no longer remembers the three clicks. Said here rather than discovered.
#[tauri::command]
fn query_keep(
    shared: tauri::State<'_, Shared>,
    name: String,
    typed: String,
) -> Result<QueryRow, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut kept = girsa_note::SavedQuery::new(name, typed);
    for chip in state.chips.row() {
        if let Some(chosen) = chip.choices.iter().find(|choice| choice.chosen) {
            // The scope chip's key is not an option among others — it is the
            // whole scope — so it is saved as the slugs below instead.
            if chip.key != girsa_search::chips::DOORWAY {
                kept = kept.with_chip(chip.key, chosen.key.clone());
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
    let state = shared.lock().map_err(|_| State::poisoned())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let held = shelf
        .queries()
        .get(&name)
        .ok_or_else(|| format!("there is no saved query called {name}"))?
        .clone();

    state.chips = Chips::default();
    for (chip, key) in &held.chips {
        state.chips.choose(chip, key).map_err(|e| e.to_string())?;
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    shelf.queries_mut().remove(&name).map_err(|e| e.to_string())
}

/// Your chaburah folders.
#[tauri::command]
fn folders(shared: tauri::State<'_, Shared>) -> Result<Vec<FolderRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let names = state.names().ok_or_else(|| state.trouble())?;
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
                        // `Naming::said`. This and `girsa-notes`'s copy printed
                        // one `Member::Place` two ways — the whole permanent id
                        // there, the address here — and neither honoured the
                        // language the window is in.
                        said: names.of(id).said(),
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such edit: {other}"),
            ))
        }
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
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let trouble = state.trouble();
    let shelf = state.shelf.as_mut().ok_or(trouble)?;
    shelf
        .collections_mut()
        .remove(&name)
        .map_err(|e| e.to_string())
}

/// Every tag across your whole layer.
#[tauri::command]
fn tags(shared: tauri::State<'_, Shared>) -> Result<Vec<TagRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let counted = girsa_note::Tags::of(&shelf.layer());
    Ok(counted
        .iter()
        .map(|(tag, tally)| TagRow::of(tag, tally))
        .collect())
}

/// Write your whole layer out somewhere, as plain files.
///
/// Into `personal/exports/` by default, the way a corrected sefer goes out
/// (W22): the files are the point and where they land is not.
#[tauri::command]
fn export_layer(shared: tauri::State<'_, Shared>, into: Option<String>) -> Result<String, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?;
    let into = into.map_or_else(
        || shelf.personal().join("exports").join("my-layer"),
        PathBuf::from,
    );
    let written = girsa_note::export(&shelf.layer(), &into).map_err(|e| e.to_string())?;
    // Composed from the list rather than typed out, so a fifth store appears
    // in this sentence without anybody remembering to add it.
    let said: Vec<String> = written
        .iter()
        .map(|(kind, count)| format!("{count} {}", kind.said()))
        .collect();
    Ok(format!("{} · {}", into.display(), said.join(" · ")))
}

/// Kept honest: the workspace type the window draws is the one the tests are
/// written against, not a second copy of it living in TypeScript.
#[allow(dead_code)]
fn _assert_workspace_is_the_tested_one(w: &Workspace) -> usize {
    w.tabs.len()
}
