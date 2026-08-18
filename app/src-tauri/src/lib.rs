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
//!
//! # Why every command is `#[tauri::command(async)]`
//!
//! > *"In general, it is a very unresponsive UI - almost like openoffice."*
//!
//! Every command in this file was `#[tauri::command]`, which is not a synonym
//! for *async* with the ceremony left off. `tauri-macros`' `ExecutionContext`
//! has two settings, and the default one — `Blocking` — runs the function
//! **inline in the IPC handler**, on the thread that owns the webview and the
//! window's message loop. `(async)` on a synchronous function puts it on the
//! runtime's blocking pool instead; the macro's own name for that arm is
//! `sync_threadpool`.
//!
//! So a search over 3.6 GB of index did not merely take two seconds — it took
//! two seconds during which the window could not repaint, could not scroll, and
//! could not process the click that would have cancelled it. Nothing in the
//! window was slow. One thread was doing two jobs.
//!
//! The two exceptions are `copy` and `sefer_sheet`: the clipboard and the print
//! sheet talk to the platform rather than to the shelf, they are fast, and
//! moving them buys nothing to pay for the risk.
//!
//! What this does **not** change is that the state is one `Mutex`, so two
//! commands still take their turn. It changes which thread waits.

mod clipboard;
mod post;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
    /// The catalogue and your own layer.
    ///
    /// # Why it is behind its own lock
    ///
    /// Everything in this struct used to be behind the window's one `Mutex`,
    /// so a command that only wanted to *read* the shelf waited for every
    /// command that wanted anything else. Three of the reads are not small:
    /// `Shelf::read` is a whole sefer off disk, `mefarshim::Marks::of` is
    /// 0.07 s for Berakhot, and `Joined::between` walks both works' 3.4 MB of
    /// edges. A window with three panes paid all of those one after another
    /// because the lock said so, not because the work said so.
    ///
    /// `RwLock` because the readers dominate by orders of magnitude: the
    /// writers are `add_mine`, `fix`, `write_note`, `set_showing` and the
    /// arrangement edits, every one of them something a reader *did* rather
    /// than something a frame needs. `Arc` so a command can take a handle,
    /// drop the state guard, and read the shelf with nothing else waiting on
    /// it.
    ///
    /// Take it through [`State::shelf`] and [`State::shelf_mut`] and never by
    /// hand: `shelf_mut` takes `&mut self` so that the borrow checker refuses
    /// a write guard while a read guard is alive, which is the one shape that
    /// deadlocks — `std::sync::RwLock` is not reentrant.
    pub(crate) shelf: Option<Arc<std::sync::RwLock<Shelf>>>,
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
    /// The link shards the chain has already read — see
    /// `girsa_link::chain::Cache`, which holds the whole argument.
    ///
    /// Emptied wherever a repair is written, because every work in it went
    /// through the repair layer as it stood when it was read.
    pub(crate) chains: girsa_link::chain::Cache,
    /// The search bar, if there is an index to search. Kept beside the shelf
    /// rather than inside it because an index is a rebuildable cache and a
    /// shelf is not: a window with no index still reads seforim, and says why
    /// it cannot search rather than returning nothing.
    bar: Option<Arc<Bar>>,
    /// Why there is no search, if there is none.
    no_search: Option<String>,
    /// The chip row as it stands (spec.md §9.5). Held here, not in the webview,
    /// so that what the chips say and what the engine does cannot drift.
    chips: Chips,
    /// The **find bar's** own chips, which are not the panel's.
    ///
    /// > *"the search should be the same as regular girsa search (with all the
    /// > options)."*
    ///
    /// Same engine, same modes, same match and together — and a second set of
    /// chips, because the two are two questions. A reader who has narrowed the
    /// panel to Halakhah and set it to regex has not said anything about the
    /// phrase they are about to look for in the daf in front of them, and a
    /// find bar that inherited the panel's scope would search one sefer through
    /// a filter naming a different one.
    ///
    /// The scope is replaced on every ask with *this sefer and nothing else*,
    /// so the one chip a reader cannot set here is the one that would stop it
    /// being a find.
    here_chips: Chips,
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
    open: girsa_app::held::Held<Arc<Open>>,
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
    /// The shelf, read-locked.
    ///
    /// # Errors
    ///
    /// If there is no shelf, or the lock is poisoned.
    pub(crate) fn shelf(&self) -> Result<std::sync::RwLockReadGuard<'_, Shelf>, String> {
        self.shelf
            .as_ref()
            .ok_or_else(|| self.trouble())?
            .read()
            .map_err(|_| State::poisoned())
    }

    /// The shelf, write-locked.
    ///
    /// **`&mut self` is the point.** Nothing here needs a mutable state — the
    /// lock provides the mutability — but taking one makes the borrow checker
    /// reject a `shelf_mut` call while a [`State::shelf`] guard is still
    /// alive. That pair is a deadlock and it is the only one this arrangement
    /// can produce, so it is worth a signature that cannot express it.
    ///
    /// # Errors
    ///
    /// If there is no shelf, or the lock is poisoned.
    pub(crate) fn shelf_mut(&mut self) -> Result<std::sync::RwLockWriteGuard<'_, Shelf>, String> {
        let trouble = self.trouble();
        self.shelf
            .as_ref()
            .ok_or(trouble)?
            .write()
            .map_err(|_| State::poisoned())
    }

    /// A handle on the shelf that outlives the state guard.
    ///
    /// For the commands that read the shelf for longer than the rest of the
    /// window should have to wait: take this, drop the guard, then lock the
    /// shelf.
    ///
    /// # Errors
    ///
    /// If there is no shelf.
    fn shelf_handle(&self) -> Result<Arc<std::sync::RwLock<Shelf>>, String> {
        self.shelf
            .as_ref()
            .map(Arc::clone)
            .ok_or_else(|| self.trouble())
    }

    /// Read this sefer if it is not already held. Everything below is one of
    /// three ways of asking for the result.
    fn load(&mut self, slug: &str) -> Result<(), String> {
        if self.open.has(slug) {
            return Ok(());
        }
        // The guard is dropped before `open` is touched: reading a work is
        // the long part and the map insert is not, and holding a shelf guard
        // while taking `&mut self` is a shape the borrow checker rejects
        // anyway — which is the whole reason `shelf_mut` takes `&mut self`.
        let read = {
            let shelf = self.shelf()?;
            shelf.read(slug).map_err(|e| e.to_string())?
        };
        // Whatever was dropped takes its marks table with it: a table of
        // who comments on which line of a sefer nobody has open is the
        // same megabytes with none of the use.
        if let Some(gone) = self.open.put(slug, Arc::new(read)) {
            self.marks.remove(&gone);
        }
        Ok(())
    }

    pub(crate) fn sefer(&mut self, slug: &str) -> Result<&Open, String> {
        self.load(slug)?;
        self.open
            .get(slug)
            .map(AsRef::as_ref)
            .ok_or_else(|| "not open".to_string())
    }

    /// The same sefer as a handle rather than a borrow, so the caller can drop
    /// the state guard and keep reading it.
    ///
    /// The reason `open` holds `Arc<Open>` at all. A work is tens of megabytes
    /// of text, so *clone it and drop the lock* is not a trade the export path
    /// can make — but cloning a pointer to it is free, and an `Open` is
    /// immutable for as long as anybody holds one: a correction goes through
    /// [`State::reread`], which drops the handle from the cache and reads the
    /// sefer again rather than editing the one in memory.
    pub(crate) fn held(&mut self, slug: &str) -> Result<Arc<Open>, String> {
        self.load(slug)?;
        self.open
            .get(slug)
            .cloned()
            .ok_or_else(|| "not open".to_string())
    }

    /// A sefer and the lexicon to draw it with — the pair every `Line::of` needs
    /// (W19).
    ///
    /// One method rather than two calls because `sefer` takes `&mut self` to
    /// load on demand, and the borrow it hands back rules out reading
    /// `self.lexicon` afterwards. The alternatives were both worse: cloning a
    /// 24,731-variant lexicon per pane of text, or passing `None` at the call
    /// sites and quietly not linkifying anything.
    pub(crate) fn reading(
        &mut self,
        slug: &str,
    ) -> Result<(&Open, Option<&girsa_ref::Lexicon>), String> {
        self.load(slug)?;
        let sefer = self
            .open
            .get(slug)
            .map(AsRef::as_ref)
            .ok_or_else(|| "not open".to_string())?;
        Ok((sefer, self.lexicon.as_ref()))
    }

    /// Which mefarshim speak on which line of one sefer, read once.
    fn marks(&mut self, slug: &str) -> Result<&girsa_app::mefarshim::Marks, String> {
        if !self.marks.contains_key(slug) {
            let read = {
                let shelf = self.shelf()?;
                girsa_app::mefarshim::Marks::of(&shelf, slug).map_err(|e| e.to_string())?
            };
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

    /// Put a sefer of your own into the search index, now (W11, spec.md §11).
    ///
    /// A note is a sefer and *your notes are searchable* — and until this they
    /// were searchable as of the last build, so the honest sentence in the
    /// results header was **"1 note since the index was built"** and the only
    /// way to make it stop saying that was four minutes over five million
    /// segments. A note is one segment. See `girsa_search::building`.
    ///
    /// **Nothing here fails a write.** The note is on disk before this is
    /// called; if the index will not take it the reader has still written their
    /// note, and what they lose is that it is findable until the next build —
    /// which is exactly the state everything was in before, and which the
    /// results header already knows how to say. Refusing the write because a
    /// cache would not update would be the tail wagging the dog.
    fn searchable(&mut self, slug: &str) {
        let (Some(bar), Ok(shelf)) = (self.bar.as_ref(), self.shelf()) else {
            return;
        };
        let personal = shelf.personal().to_path_buf();
        drop(shelf);
        make_searchable(bar, &personal, slug);
    }

    /// And take one out, because it is not on the shelf any more.
    ///
    /// The asymmetry is real: a work that has been deleted has no
    /// `segments.jsonl` to read back, so the delete-then-add rule never fires
    /// for it. Left alone, a note you threw away stays findable until the next
    /// full build and a hit on it opens a sefer that is not there.
    fn unsearchable(&mut self, slug: &str) {
        let Some(bar) = self.bar.as_ref() else {
            return;
        };
        if let Err(e) = bar.forget(slug) {
            eprintln!("{slug} is gone and is still in the index: {e}");
        }
    }
}

/// The body of [`State::searchable`], with the two things it needs handed to it
/// rather than borrowed off the state.
///
/// Split out because absorbing a work writes to the index — a tantivy commit —
/// and the one caller that does it in a loop is the drop path, which has no
/// business holding the state lock while it does. `Bar::absorb` is `&self` and
/// takes the personal root as an argument, so both halves were already
/// separable; only the borrow made them look joined.
/// Where the reader's own layer is, **and the guard already dropped**.
///
/// The shape finding 2c is about. A command that locks the state to read one
/// `PathBuf` and then keeps the guard for the file I/O that follows is holding
/// the whole window's turn for a reason that ended on the first line — and
/// `buffer_save` is the worst of them, because `writing.ts` calls it every
/// 900 ms for as long as the reader is typing.
///
/// # Errors
///
/// If there is no shelf, which is the one state in which there is no layer to
/// find.
fn personal_of(shared: &tauri::State<'_, Shared>) -> Result<PathBuf, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    let personal = shelf.personal().to_path_buf();
    Ok(personal)
}

/// Have this sefer in memory **before** the caller takes the state lock.
///
/// [`State::load`] can only read the shelf while the state guard is held,
/// because it is a method on the state — and that read is a whole work off
/// disk, 11 ms in the published table, on the path of every pane that opens.
/// A window with three columns paid for three of them one after another
/// because the lock said so.
///
/// This does the same read with no state lock at all and only a *read* lock on
/// the shelf, so three panes opening at once read three seforim at once. Every
/// command still works without it; it just pays under the lock, which is where
/// this started.
///
/// # Errors
///
/// If there is no shelf, the lock is poisoned, or the sefer will not read.
fn hold(shared: &tauri::State<'_, Shared>, slug: &str) -> Result<(), String> {
    let handle = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        if state.open.has(slug) {
            return Ok(());
        }
        state.shelf_handle()?
    };
    let read = {
        let shelf = handle.read().map_err(|_| State::poisoned())?;
        shelf.read(slug).map_err(|e| e.to_string())?
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // Another pane may have got there first while this one was reading. Both
    // read the same sefer, so keeping the copy already in the cache means the
    // two panes share one rather than holding one each.
    if !state.open.has(slug) {
        if let Some(gone) = state.open.put(slug, Arc::new(read)) {
            state.marks.remove(&gone);
        }
    }
    Ok(())
}

/// The same, for the table of which mefarshim speak on which line of a sefer.
///
/// `Marks::of` is the 0.07 s read for Berakhot that `State::marks` documents,
/// and `main.ts` asks for one per pane on every repaint.
///
/// # Errors
///
/// If there is no shelf, the lock is poisoned, or the table will not read.
fn hold_marks(shared: &tauri::State<'_, Shared>, slug: &str) -> Result<(), String> {
    let handle = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        if state.marks.contains_key(slug) {
            return Ok(());
        }
        state.shelf_handle()?
    };
    let read = {
        let shelf = handle.read().map_err(|_| State::poisoned())?;
        girsa_app::mefarshim::Marks::of(&shelf, slug).map_err(|e| e.to_string())?
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.marks.entry(slug.to_string()).or_insert(read);
    Ok(())
}

fn make_searchable(bar: &Bar, personal: &std::path::Path, slug: &str) {
    match bar.absorb(personal, slug) {
        Ok(done) => {
            for line in done.trouble {
                eprintln!("{line}");
            }
        }
        Err(e) => eprintln!("{slug} is written and is not in the index yet: {e}"),
    }
}

impl State {
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
    /// **The shelf is passed in and not read here**, because it is behind its
    /// own lock now and `Names` borrows it. One guard per caller, taken once
    /// and handed down: two `State::shelf` calls live at the same moment on
    /// one thread is a read-read reentry, and `std::sync::RwLock` will
    /// deadlock on that the moment a writer is queued between them.
    fn names<'a>(&'a self, shelf: &'a Shelf) -> girsa_app::Names<'a> {
        girsa_app::Names::new(
            shelf,
            self.timeline.as_ref(),
            self.session.language,
            // The reader's own citation style, so a row label and the citation
            // they copy off the same line agree.
            self.session.cite,
        )
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
            let personal = self.shelf()?.personal().to_path_buf();
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

#[tauri::command(async)]
fn state(shared: tauri::State<'_, Shared>) -> Result<girsa_app::view::Opening, String> {
    // The queue is 28,124 lines on the real corpus and this is asked on every
    // redraw, so it is read once and held. `suspects` re-reads it, which is
    // where a run of the batch job is noticed.
    //
    // Held once, but *read* with the guard down. This is the first redraw of a
    // cold window — the moment when everything else is also asking — and the
    // read is a parse of the largest file in the personal layer.
    let wanted = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        state
            .queue
            .is_none()
            .then(|| state.shelf().ok().map(|s| s.personal().to_path_buf()))
            .flatten()
    };
    if let Some(personal) = wanted {
        let read = girsa_fix::suspect::Queue::open(&personal).0;
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        // Another redraw may have filled it while this one was parsing. Theirs
        // is as good as ours and it is already in place.
        if state.queue.is_none() {
            state.queue = Some(read);
        }
    }
    let state = shared.lock().map_err(|_| State::poisoned())?;
    // One guard for both counts below. Two `map_or`s each taking their own
    // would be a read-read reentry on one thread — harmless today because each
    // closure drops its guard, and one refactor away from a hang.
    let shelf = state.shelf().ok();
    Ok(girsa_app::view::Opening {
        workspace: state.session.workspace.clone(),
        pointing: state.session.pointing,
        shemos: state.session.shemos,
        text_size: state.session.text_size,
        mefarshim_size: state.session.mefarshim_size,
        positions: state.session.positions.clone(),
        works: shelf.as_ref().map_or(0, |s| s.works().len()),
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
        fixes: shelf.as_ref().map_or(0, |s| s.fixes().count()),
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
#[tauri::command(async)]
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
    state.shelf = Some(Arc::new(std::sync::RwLock::new(shelf)));
    state.trouble = opened.trouble;
    state.timeline = opened.timeline;
    // A different corpus, so every shard read out of the old one is a lie.
    state.chains = girsa_link::chain::Cache::default();
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

#[tauri::command(async)]
fn search(shared: tauri::State<'_, Shared>, query: String) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    Ok(shelf
        .search(&query, girsa_app::enough::NAMES_OFFERED)
        .into_iter()
        .map(Card::of)
        .collect())
}

/// The seforim a reader has been in, most recent first — what the picker shows
/// before anything has been typed.
#[tauri::command(async)]
fn recent(shared: tauri::State<'_, Shared>) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let Ok(shelf) = state.shelf() else {
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
#[tauri::command(async)]
fn find(shared: tauri::State<'_, Shared>, query: String, page: usize) -> Result<FoundPage, String> {
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
fn find_rung(
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
fn find_chip(shared: tauri::State<'_, Shared>, chip: String, key: String) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.chips.choose(&chip, &key).map_err(|e| e.to_string())
}

/// Click a facet row: narrow to it, or rule it out (spec.md §9.8).
#[tauri::command(async)]
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
#[tauri::command(async)]
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
    /// The row it came from, so the tree can draw that row ticked.
    ///
    /// A shelf key and a sefer slug are both keys and they do not collide, so
    /// the tree matches on this alone. The label will not do: every shelf has a
    /// `ראשונים` under it.
    key: String,
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
fn find_scope(shared: tauri::State<'_, Shared>) -> Result<ScopeView, String> {
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
fn find_scope_set(
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
fn find_scope_drop(shared: tauri::State<'_, Shared>, at: usize) -> Result<ScopeView, String> {
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
struct Opened {
    shelf: Option<Shelf>,
    trouble: Option<String>,
    timeline: Option<girsa_corpus::era::Timeline>,
    bar: Option<Arc<Bar>>,
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

/// The shelf, as a tree — the shipped taxonomy with your arrangement on top.
///
/// Counts only; the seforim themselves come one shelf at a time from
/// [`shelf_works`]. 7,189 cards is not a browse, it is a dump.
#[tauri::command(async)]
fn shelf_tree(shared: tauri::State<'_, Shared>) -> Result<Vec<Branch>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    Ok(shelf.tree())
}

#[tauri::command(async)]
fn shelf_works(shared: tauri::State<'_, Shared>, key: String) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
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
#[tauri::command(async)]
fn titles(shared: tauri::State<'_, Shared>, slugs: Vec<String>) -> Result<Vec<Card>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    Ok(slugs
        .iter()
        .filter_map(|slug| shelf.work(slug))
        .map(Card::of)
        .collect())
}

/// Put a sefer on a shelf.
#[tauri::command(async)]
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
#[tauri::command(async)]
fn shelf_put_shelf(
    shared: tauri::State<'_, Shared>,
    key: String,
    parent: String,
) -> Result<(), String> {
    edit_shelf(&shared, move |a| a.put_shelf(&key, &parent))
}

#[tauri::command(async)]
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
#[tauri::command(async)]
fn shelf_pin(shared: tauri::State<'_, Shared>, parent: String, key: String) -> Result<(), String> {
    edit_shelf(&shared, move |a| {
        let mut order = a.order.get(&parent).cloned().unwrap_or_default();
        order.retain(|k| *k != key);
        order.insert(0, key);
        a.reorder(&parent, order);
        Ok(())
    })
}

#[tauri::command(async)]
fn shelf_make(
    shared: tauri::State<'_, Shared>,
    parent: String,
    title: String,
) -> Result<String, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
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
#[tauri::command(async)]
fn shelf_reset(shared: tauri::State<'_, Shared>) -> Result<(), String> {
    edit_shelf(&shared, |a| {
        a.reset();
        Ok(())
    })
}

/// What a drop reports itself on. See [`DropProgress`].
const DROP_EVENT: &str = "add-mine";

/// Files dropped on the window become seforim.
///
/// # Why the loop is not under the lock
///
/// It was, and it is the drag-and-drop path — so by definition the reader is
/// at the window watching, and by definition `paths.len()` is whatever they
/// selected. Copying each file in, parsing it and writing its segments back
/// out is the whole of the cost and **none** of it needs the shelf:
/// `girsa_app::shelf::read_mine` takes the personal root and a path, which is
/// why it could be split off at all. What needs the shelf is `took_mine` —
/// three in-memory writes — and that is all the lock is now held for, once per
/// file rather than once per drop.
///
/// The reader is told which file is being read as it goes, on [`DROP_EVENT`],
/// because a window that goes quiet for a large drop and then says everything
/// at once is a window that looked broken for the duration.
#[tauri::command(async)]
fn add_mine(
    app: tauri::AppHandle,
    shared: tauri::State<'_, Shared>,
    paths: Vec<String>,
) -> Result<Dropped, String> {
    use tauri::Emitter;
    // Where your layer is, and nothing else out of the state.
    let (personal, bar) = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let shelf = state.shelf()?;
        (shelf.personal().to_path_buf(), state.bar.clone())
    };

    let mut added = Vec::new();
    let mut refused = Vec::new();
    let of = paths.len() as u64;
    for (n, path) in paths.into_iter().enumerate() {
        let file = PathBuf::from(&path);
        let _ = app.emit(
            DROP_EVENT,
            girsa_app::view::DropProgress {
                doing: "read",
                what: file
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(path.as_str())
                    .to_string(),
                done: n as u64,
                of,
            },
        );
        // No lock held. This is the copy and the parse.
        let read = match girsa_app::shelf::read_mine(&personal, &file, None) {
            Ok(read) => read,
            Err(e) => {
                refused.push(Refusal {
                    file: path,
                    why: e.to_string(),
                });
                continue;
            }
        };
        let slug = {
            let mut state = shared.lock().map_err(|_| State::poisoned())?;
            let mut shelf = state.shelf_mut()?;
            let slug = shelf.took_mine(read);
            if let Some(work) = shelf.work(&slug) {
                added.push(Card::of(work));
            }
            slug
        };
        // Searchable now rather than at the next build (W11). A dropped
        // handout is a few dozen segments; a PDF is pages with no words in
        // them until it is OCR'd, and going in empty is what makes the results
        // header able to count it as *not searchable yet* rather than leaving
        // the sefer absent entirely.
        //
        // Off the lock as well: absorbing a work is a tantivy commit, and a
        // commit per dropped file is exactly the shape that must not be
        // serialised against the scroll handler.
        if let Some(bar) = bar.as_ref() {
            make_searchable(bar, &personal, &slug);
        }
    }
    let _ = app.emit(
        DROP_EVENT,
        girsa_app::view::DropProgress {
            doing: "done",
            what: String::new(),
            done: of,
            of,
        },
    );
    Ok(Dropped { added, refused })
}

fn edit_shelf(
    shared: &tauri::State<'_, Shared>,
    change: impl FnOnce(&mut girsa_app::Arrangement) -> Result<(), girsa_app::arrangement::Refused>,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
    shelf.edit(change).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn companions(shared: tauri::State<'_, Shared>, slug: String) -> Result<Vec<Companion>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
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
#[tauri::command(async)]
fn mefarshim(shared: tauri::State<'_, Shared>, slug: String) -> Result<Mefarshim, String> {
    hold_marks(&shared, &slug)?;
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
    // The list comes back in name order, in the language the window is in — see
    // `mefarshim::listed`. Read before the shelf is borrowed; `Language` is
    // `Copy`, so this costs nothing and keeps the borrow checker out of it.
    let language = state.session.language;
    // The pairs the reader made himself, read both ways round — see
    // `Session::alongside`, and the Shulchan Arukh HaRav, which is the case that
    // needs them.
    let mine = state.session.alongside_of(&slug);
    let shelf = state.shelf()?;
    Ok(Mefarshim::of(
        &shelf, &marks, &slug, &chosen, language, &mine,
    ))
}

/// Say — or unsay — that two seforim keep the same order (A6).
///
/// > *"1, plus the user can add."*
///
/// `taxonomy::Keeping` settles this from the graph and is only as good as the
/// links. The Shulchan Arukh HaRav is written on Orach Chayim's simanim and the
/// graph joins two of its 505 to their own number, so the corpus cannot say it
/// and this application will not say it for the corpus. The reader can.
///
/// Answers with the whole list, the same as `choose_mefaresh`: the window used
/// to patch its own copy after a tick and that is how a list drifts from what
/// Rust holds.
#[tauri::command(async)]
fn pair_alongside(
    shared: tauri::State<'_, Shared>,
    slug: String,
    work: String,
    on: bool,
) -> Result<Mefarshim, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.pair(&slug, &work, on);
    state.save();
    mefarshim_of(&mut state, &slug)
}

/// Tick or untick one mefaresh, and answer with the whole list as it stands
/// now.
///
/// The **whole** list, not just the marked lines: see [`mefarshim_of`].
#[tauri::command(async)]
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

/// Tick the mefarshim printed **on the page** with this sefer, in one gesture.
///
/// Rashi and Tosfos on a daf, Onkelos beside a Chumash, the Bartenura under a
/// Mishnah — see [`girsa_app::mefarshim::the_usual`], which is where the list
/// is decided and where the argument for leaving the alphabetical order alone
/// is written down.
///
/// One command and not a loop of `choose_mefaresh` in the window: each of those
/// re-weaves the whole list and hands it back, so three ticks would be three
/// round trips and three redraws for one click.
///
/// **Ticks and never unticks.** A reader who has already opened Rashi and
/// presses this wants Tosfos as well, not Rashi shut.
#[tauri::command(async)]
fn choose_the_usual(shared: tauri::State<'_, Shared>, slug: String) -> Result<Mefarshim, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let usual = mefarshim_of(&mut state, &slug)?
        .usual
        .into_iter()
        .map(|m| m.slug)
        .collect::<Vec<_>>();
    for work in usual {
        state.session.choose(&slug, &work, true);
    }
    state.save();
    mefarshim_of(&mut state, &slug)
}

/// Click a line: read the ticked mefarshim on it.
#[tauri::command(async)]
fn mefarshim_at(
    shared: tauri::State<'_, Shared>,
    slug: String,
    at: String,
) -> Result<Comments, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    hold_marks(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
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
            let (sefer, lexicon) = state.reading(&one.work)?;
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
            let mut lines: Vec<Line> = sefer.segments[first..=last]
                .iter()
                .map(|s| Line::of(sefer, s, pointing, shemos, style, lexicon))
                .collect();
            girsa_app::view::only_when_it_changes(&mut lines);
            lines
        };
        // Named out of the shelf, in a scope of its own: `state.reading` above
        // takes `&mut state`, so a guard that spanned the loop body would be a
        // borrow error — which is the borrow checker saying the same thing
        // this file says in prose, that a lock is held for the line it is
        // needed on and not for the block around it.
        let (he_title, en_title, address) = {
            let shelf = state.shelf()?;
            shelf.work(&one.work).map_or_else(
                || (one.work.clone(), one.work.clone(), one.at.address()),
                |w| {
                    (
                        w.he_title.clone(),
                        w.en_title.clone(),
                        // `רש״י על ברכות 2a:8:1` was the header on the release
                        // build. The address a mefaresh's block carries is the
                        // same kind of thing as the one in the margin, and it
                        // goes through the same formatter.
                        girsa_app::sending::printed_address(w, &one.at, style),
                    )
                },
            )
        };
        said.push(Said {
            he_title,
            en_title,
            address,
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

#[tauri::command(async)]
fn open_sefer(shared: tauri::State<'_, Shared>, slug: String) -> Result<Text, String> {
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    // Where the reader was, so the window opens around it rather than at the
    // top and then jumping. `where_i_was` is the same memory `Session` keeps for
    // every sefer ever opened (W9).
    let left = state.session.where_i_was(&slug).cloned();
    let (sefer, lexicon) = state.reading(&slug)?;
    // Whether anything in the sefer is pointed. Over the whole of it, because
    // the answer is about the sefer and not about the window a reader happens
    // to be looking at — a Chumash whose first four hundred segments are bare
    // still has nikud.
    let has_nikud = sefer.segments.iter().any(|s| display::has_marks(&s.text));
    let total = sefer.segments.len();
    // Where the reader is: where they were last time if that segment is still
    // in this sefer, else its first line. Never anything else — see `Text::at`,
    // which is what carries the answer out to the pane.
    let at = left.and_then(|id| sefer.position_of(&id)).unwrap_or(0);
    let going = sefer
        .segments
        .get(at)
        .map(|s| s.id.to_string())
        .unwrap_or_default();
    let from = at.saturating_sub(A_WINDOW / 2).min(total.saturating_sub(1));
    let to = (from + A_WINDOW).min(total);
    let mut lines: Vec<Line> = sefer.segments[from..to]
        .iter()
        .map(|s| Line::of(sefer, s, pointing, shemos, style, lexicon))
        .collect();
    girsa_app::view::only_when_it_changes(&mut lines);
    Ok(Text {
        work: Card::of(&sefer.work),
        lines,
        from,
        at: going,
        total,
        has_nikud,
    })
}

/// More of a sefer, for a pane that has scrolled to the edge of what it holds.
///
/// Clamped rather than refused: a pane asking past the end is a reader at the
/// end of the sefer, and the honest answer is the last lines rather than an
/// error.
#[tauri::command(async)]
fn sefer_lines(
    shared: tauri::State<'_, Shared>,
    slug: String,
    from: usize,
    count: usize,
) -> Result<Vec<Line>, String> {
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    let (sefer, lexicon) = state.reading(&slug)?;
    let from = from.min(sefer.segments.len());
    let to = from.saturating_add(count).min(sefer.segments.len());
    let mut lines: Vec<Line> = sefer.segments[from..to]
        .iter()
        .map(|s| Line::of(sefer, s, pointing, shemos, style, lexicon))
        .collect();
    girsa_app::view::only_when_it_changes(&mut lines);
    Ok(lines)
}

/// The table of contents of a sefer (A3).
///
/// > *"there should be a table of contents on the side for each sefer, so you
/// > can jump around."*
///
/// Built from the segments' own addresses rather than by scanning the text for
/// headings — see `girsa_app::contents`, which holds the argument. Answered per
/// sefer and per open, not per keystroke: the filter box in the window filters
/// the list it already has, the way Otzaria's does.
#[tauri::command(async)]
fn sefer_contents(
    shared: tauri::State<'_, Shared>,
    slug: String,
) -> Result<Vec<girsa_app::contents::Entry>, String> {
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let style = state.session.cite;
    let (sefer, _) = state.reading(&slug)?;
    Ok(girsa_app::contents::of(sefer, style))
}

/// Where a segment sits in its sefer, counted from the start.
///
/// What a pane asks when it is told to go to a line it has not loaded — a search
/// hit, a link, a mefaresh's place. `None` is *this sefer does not have that
/// segment*, which is a real answer and not an error: a link can point at a
/// place a re-import moved, and W23's panel is where that gets repaired.
#[tauri::command(async)]
fn sefer_index_of(
    shared: tauri::State<'_, Shared>,
    slug: String,
    at: String,
) -> Result<Option<usize>, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let sefer = state.sefer(&slug)?;
    Ok(sefer.position_of(&at))
}

/// The same question about many segments at once.
///
/// # Why there are two of these
///
/// [`sefer_index_of`] answers about one place, and its three callers in the
/// window are each a single reader action — clicking a search hit, following a
/// link — where one round trip is one round trip.
///
/// *Next highlight* is not that. `goToNextPlace` asked for every mark in the
/// sefer, one at a time, on every press of the key: a reader with 200
/// highlights in Mishnah Berurah paid 200 serialised IPC calls to answer
/// *which is the next one*, and paid them again on the next press. The marks
/// that miss the pane's own table are exactly the ones outside the window of
/// lines it is holding, which on a long sefer is most of them.
///
/// The answers come back **in the order they were asked**, one slot per
/// question, so a caller can zip them against its own list. `None` in a slot is
/// *this sefer does not have that segment*, the same real answer the single
/// version gives — and an id that will not parse is `None` too rather than a
/// refusal for the whole batch, because one bad mark should not cost the
/// reader the other 199.
///
/// `linksview.ts` already had the shape (`api.linkWords` takes an array); this
/// is the same idea one file over.
///
/// # Errors
///
/// If there is no shelf, or the sefer will not read.
#[tauri::command(async)]
fn sefer_indices_of(
    shared: tauri::State<'_, Shared>,
    slug: String,
    ats: Vec<String>,
) -> Result<Vec<Option<usize>>, String> {
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let sefer = state.sefer(&slug)?;
    Ok(ats
        .iter()
        .map(|at| {
            at.parse::<SegmentId>()
                .ok()
                .and_then(|at| sefer.position_of(&at))
        })
        .collect())
}

// ── Scans (spec.md §6.3, BUILDER.md W25) ────────────────────────────────────

/// Open a scan into a pane.
#[tauri::command(async)]
fn scan(shared: tauri::State<'_, Shared>, slug: String) -> Result<ScanView, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&slug)?;
    // Where this scan was left last time. Looked up here rather than worked out
    // in the window from the id it remembered, for the reason in `page_of_id`.
    let left = state.session.positions.get(&slug).cloned();
    let shelf = state.shelf()?;
    let sefer = state.open.peek(&slug).ok_or("not open")?;
    let scan = girsa_app::scan_of(&shelf, sefer).ok_or_else(|| format!("{slug} is not a scan"))?;
    let at = left
        .and_then(|id| girsa_app::scanning::page_of_id(sefer, &id))
        .unwrap_or(1);
    Ok(view_of(&shelf, sefer, &scan, at))
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
#[tauri::command(async)]
fn scan_reading(shared: tauri::State<'_, Shared>, slug: String) -> Result<ReadingRow, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&slug)?;
    let personal = {
        let shelf = state.shelf()?;
        shelf.personal().to_path_buf()
    };
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
#[tauri::command(async)]
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
        let _shelf = state.shelf()?;
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
#[tauri::command(async)]
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
        let shelf = state.shelf()?;
        let personal = shelf.personal().to_path_buf();
        personal
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
#[tauri::command(async)]
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
#[tauri::command(async)]
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

/// Highlight a run of words on a page, by the ink they sit on (W24, W26).
///
/// A page is one segment and carries no text, so the character span every other
/// highlight in this window is stored as means nothing here — a scan could only
/// ever be marked whole. What a page has is words with rectangles under them,
/// and the rectangle is the anchor that survives the page being read again by a
/// better engine, which is the argument `scan_fix` already makes about a
/// correction.
///
/// One rectangle per line rather than one for the run: the bounding box of a
/// highlight spanning three lines also covers the ends of the lines it passes
/// through, and redrawing from it would grow the mark by words nobody chose.
#[tauri::command(async)]
fn scan_mark(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
    from_word: usize,
    to_word: usize,
    label: Option<String>,
    colour: Option<String>,
) -> Result<Vec<girsa_app::view::ScanMarkRow>, String> {
    if from_word > to_word {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NothingChosen,
            "nothing is selected",
        ));
    }
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let at = {
            let sefer = state.sefer(&slug)?;
            girsa_app::scanning::page_id(sefer, page)
                .ok_or_else(|| format!("{slug} has no page {page}"))?
        };
        let read = state
            .words(&slug)?
            .page(page)
            .ok_or_else(|| format!("nobody has read page {page} of {slug}"))?;
        let (ink, was) = girsa_app::scanning::ink_of(&read, from_word..to_word + 1)
            .ok_or("those words are not on this page")?;
        let who = girsa_app::who();
        let mut mark = girsa_note::Mark::on_ink(at, ink, was, &who);
        mark.label = label;
        mark.colour = colour;
        state
            .shelf_mut()?
            .marks_mut()
            .add(mark)
            .map_err(|e| e.to_string())?;
    }
    scan_marks(shared, slug, page)
}

/// The highlights on a page, with the words each one covers **now**.
#[tauri::command(async)]
fn scan_marks(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<Vec<girsa_app::view::ScanMarkRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let at = {
        let sefer = state.sefer(&slug)?;
        girsa_app::scanning::page_id(sefer, page)
            .ok_or_else(|| format!("{slug} has no page {page}"))?
    };
    let read = state.words(&slug)?.page(page);
    let shelf = state.shelf()?;
    let standing = girsa_corpus::standing::Standing::just(at);
    let rows = shelf
        .marks()
        .on(&standing)
        .into_iter()
        .filter(|mark| !mark.ink.is_empty())
        .map(|mark| {
            // What is under that ink today. A page nobody has read any more —
            // the reading was thrown away and not replaced — leaves this empty,
            // which is honest: the rectangle is still on the photograph and
            // there are no words to name.
            let says = read.as_ref().map_or_else(String::new, |read| {
                girsa_app::scanning::words_under(read, &mark.ink)
                    .into_iter()
                    .filter_map(|at| read.words.get(at))
                    .map(|word| word.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            });
            girsa_app::view::ScanMarkRow {
                id: mark.id.as_str().to_string(),
                ink: mark
                    .ink
                    .iter()
                    .map(|area| girsa_app::view::WordRow::box_of(*area))
                    .collect(),
                was: mark.was.clone(),
                says,
                label: mark.label.clone(),
                colour: mark.colour.clone(),
                tags: mark.tags.clone(),
            }
        })
        .collect();
    Ok(rows)
}

/// What a search over this shelf cannot see — spec.md §9.7's results header, and
/// the two things it never used to include (B7).
///
/// The header used to be about scans alone, because `Gap` had one variant for
/// them and none for the reader's own writing. A note written this morning and a
/// typo fixed last night are equally absent from the index and were equally
/// unmentioned, which is the state a bochur is in every single day.
#[tauri::command(async)]
fn scan_gap(shared: tauri::State<'_, Shared>) -> Result<Option<GapRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    let personal = shelf.personal().to_path_buf();
    // Where the index is, if it is anywhere: two of the three gaps are *since the
    // index was built*, so a window that cannot find one has a bigger gap to
    // report, not a smaller one.
    let index = girsa_app::find_index(shelf.root()).ok();
    let gap = girsa_app::reading::gap(&shelf, &personal, index.as_deref());
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
#[tauri::command(async)]
fn scan_at(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<PageSaid, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let style = state.session.cite;
    state.sefer(&slug)?;
    let shelf = state.shelf()?;
    let sefer = state.open.peek(&slug).ok_or("not open")?;
    let scan = girsa_app::scan_of(&shelf, sefer).ok_or_else(|| format!("{slug} is not a scan"))?;

    // A scan whose sefer is not on the shelf still shows its pages; what it
    // cannot do is print a mekor naming a sefer nobody here has.
    let sent = girsa_app::scanning::naming(&shelf, &scan)
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
#[tauri::command(async)]
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
    {
        let mut shelf = state.shelf_mut()?;
        shelf
            .declare_paging(&slug, paging)
            .map_err(|e| e.to_string())?;
    }
    // What an address of this sefer means has changed, so the copy held open
    // is out of date (see `Open::paging`).
    state.reread(&slug);
    drop(state);
    scan(shared, slug)
}

/// Take a mapping back — better no mareh makom than a wrong one.
#[tauri::command(async)]
fn scan_forget(shared: tauri::State<'_, Shared>, slug: String) -> Result<ScanView, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    {
        let mut shelf = state.shelf_mut()?;
        shelf.forget_paging(&slug).map_err(|e| e.to_string())?;
    }
    state.reread(&slug);
    drop(state);
    scan(shared, slug)
}

/// The page a place is printed on — the *go to daf* box.
///
/// `None` where this scan does not carry it, and never the nearest page it
/// does: a scan opened one daf away with the header naming the daf that was
/// asked for is wrong in the way nobody checks.
#[tauri::command(async)]
fn scan_page_of(
    shared: tauri::State<'_, Shared>,
    slug: String,
    written: String,
) -> Result<Option<usize>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&slug)?;
    let shelf = state.shelf()?;
    let sefer = state.open.peek(&slug).ok_or("not open")?;
    let scan = girsa_app::scan_of(&shelf, sefer).ok_or_else(|| format!("{slug} is not a scan"))?;

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
#[tauri::command(async)]
fn scan_copy(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<Copied, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let style = state.session.cite;
    state.sefer(&slug)?;
    let shelf = state.shelf()?;
    let sefer = state.open.peek(&slug).ok_or("not open")?;
    let scan = girsa_app::scan_of(&shelf, sefer).ok_or_else(|| format!("{slug} is not a scan"))?;
    let naming = girsa_app::scanning::naming(&shelf, &scan).map_err(|e| e.to_string())?;
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
#[tauri::command(async)]
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
    let shemos = state.session.shemos;
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

    state.shelf_mut()?.fix(patch).map_err(|e| e.to_string())?;
    state.reread(&slug);

    let (sefer, lexicon) = state.reading(&slug)?;
    let position = sefer
        .position_of(&at)
        .ok_or_else(|| format!("{at} is not in this sefer"))?;
    let segment = sefer
        .segments
        .get(position)
        .ok_or_else(|| format!("{at} is not in this sefer"))?;
    Ok(Fixed {
        line: Line::of(sefer, segment, pointing, shemos, style, lexicon),
        said: format!("{was} → {now}"),
    })
}

/// Take a correction back.
#[tauri::command(async)]
fn unfix(shared: tauri::State<'_, Shared>, at: String, patch: String) -> Result<Fixed, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    let slug = at.work().to_string();

    let gone = state
        .shelf_mut()?
        .unfix(&girsa_fix::PatchId::from(patch))
        .map_err(|e| e.to_string())?;
    if !gone {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            "there is no such correction",
        ));
    }
    state.reread(&slug);

    let (sefer, lexicon) = state.reading(&slug)?;
    let position = sefer
        .position_of(&at)
        .ok_or_else(|| format!("{at} is not in this sefer"))?;
    let segment = sefer
        .segments
        .get(position)
        .ok_or_else(|| format!("{at} is not in this sefer"))?;
    Ok(Fixed {
        line: Line::of(sefer, segment, pointing, shemos, style, lexicon),
        said: "הוחזר כפי שנדפס".to_string(),
    })
}

/// *Show as printed / show corrected* (spec.md §7.1).
///
/// Three states rather than two, because a scanning error and an emendation are
/// different claims — see [`girsa_fix::Showing`]. Everything open is re-read,
/// which the window does by drawing again.
#[tauri::command(async)]
fn set_showing(shared: tauri::State<'_, Shared>, showing: String) -> Result<(), String> {
    let showing =
        girsa_fix::Showing::named(&showing).ok_or_else(|| format!("no such setting: {showing}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.showing = showing;
    state.shelf_mut()?.set_showing(showing);
    state.reread_everything();
    state.save();
    Ok(())
}

#[tauri::command(async)]
fn fixes(shared: tauri::State<'_, Shared>, slug: Option<String>) -> Result<Vec<PatchRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    // A patch row is a row about a place, like a hit and like a lane result —
    // and it was the fifth to work out a title and an address for itself.
    let shelf = state.shelf()?;
    let names = state.names(&shelf);
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
#[tauri::command(async)]
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
    let shemos = state.session.shemos;
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
    let shelf = state.shelf()?;
    let touching = girsa_app::touching(&shelf, shelf.repairs(), &standing);
    let mut links = touching.links;

    // The words each link is about, where anything says — and the far end's
    // text is only consulted for seforim that are **already open**.
    for link in &mut links {
        let far = state.open.peek(&link.work).map(AsRef::as_ref);
        link.span = girsa_app::links::span_on(link, &at, &base, &anchors, far, pointing);
    }
    if let (Some(from), Some(to)) = (from_char, to_char) {
        if from < to {
            links = girsa_app::links::touching_words(links, from..to);
        }
    }
    // The guard taken above, not a second one. Shadowing it would have kept
    // both alive to the end of the function, and a read-read reentry on one
    // thread is how `std::sync::RwLock` deadlocks the moment a writer queues
    // between them.
    let (lenses, trouble) = girsa_app::Lenses::load(shelf.personal());
    if let Some(said) = trouble {
        eprintln!("{said}");
    }
    if let Some(key) = lens.as_deref() {
        links = lenses.through(key, &shelf, links);
    }

    Ok(Links {
        links: links
            .iter()
            .map(|l| LinkRow::of(l, language, first_words(&state, l, pointing, shemos)))
            .collect(),
        incoming_unknown: touching.incoming_unknown,
        types: girsa_app::links::kinds(),
        lenses: lenses
            .lenses
            .iter()
            .map(|(key, lens)| LensRow {
                key: key.clone(),
                title: lens.title.clone(),
                types: lens.types.clone(),
                eras: lens.eras.clone(),
                at_least: lens.at_least,
                mine: lens.mine,
            })
            .collect(),
        lens,
    })
}

/// Pin a link onto the words it is about (spec.md §8.4).
#[tauri::command(async)]
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
    // Every work the chain holds went through the repair layer as it stood
    // when it was read, and this is about to change it. See `State::chains`.
    state.chains = girsa_link::chain::Cache::default();
    let mut shelf = state.shelf_mut()?;
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
    shemos: girsa_app::shemos::Shemos,
) -> Option<String> {
    let sefer = state.open.peek(&link.work)?;
    let nth = sefer.position_of(&link.other.from)?;
    let said = girsa_app::shemos::written(&sefer.segments.get(nth)?.text, shemos);
    let text = display::Shown::of(&said, pointing).text().to_string();
    Some(girsa_app::enough::first_words(&text))
}

/// Confirm, reject, retype, reanchor, or take it all back.
///
/// One command, because they are one thing: a statement about an edge, written
/// into your layer. Which statement is named rather than free-form — the window
/// may choose among what the engine offered and may not invent one.
#[tauri::command(async)]
fn link_repair(
    shared: tauri::State<'_, Shared>,
    edge: String,
    does: String,
    value: Option<String>,
) -> Result<(), String> {
    use girsa_link::repair::Verdict;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // Every work the chain holds went through the repair layer as it stood
    // when it was read, and this is about to change it. See `State::chains`.
    state.chains = girsa_link::chain::Cache::default();
    let mut shelf = state.shelf_mut()?;
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
#[tauri::command(async)]
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
    // Every work the chain holds went through the repair layer as it stood
    // when it was read, and this is about to change it. See `State::chains`.
    state.chains = girsa_link::chain::Cache::default();
    let mut shelf = state.shelf_mut()?;
    let who = girsa_app::who();
    shelf
        .repairs_mut()
        .reanchor_named(&edge, from_anchor, to_anchor, &who)
        .map_err(|e| e.to_string())
}

/// Draw a link by hand, from one place to another.
#[tauri::command(async)]
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
    // Every work the chain holds went through the repair layer as it stood
    // when it was read, and this is about to change it. See `State::chains`.
    state.chains = girsa_link::chain::Cache::default();
    let mut shelf = state.shelf_mut()?;
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
#[tauri::command(async)]
fn export_sefer(
    shared: tauri::State<'_, Shared>,
    slug: String,
    format: String,
    into: Option<String>,
) -> Result<Written, String> {
    let format =
        girsa_export::Format::named(&format).ok_or_else(|| format!("no such format: {format}"))?;
    // **Everything the write needs, gathered under the lock; the write itself
    // outside it.** A sefer is up to 17,418 segments and `session.export_into`
    // is a folder the reader chose — which may be a network share or a stick
    // they can pull out — so the duration of this file write is not something
    // this process gets to bound. Holding the one state lock across it stops
    // the daf beside it scrolling for however long that turns out to be.
    let (sefer, fixes, to, showing, pointing, shemos) = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let showing = state.session.showing;
        let pointing = state.session.pointing;
        let shemos = state.session.shemos;
        let personal = {
            let shelf = state.shelf()?;
            shelf.personal().to_path_buf()
        };
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
        //
        // One lookup, not two. This asked `state.sefer` to load it and then
        // `state.open.peek` to borrow it back, because the first borrow ruled
        // out reading the shelf in between; a handle rules out nothing.
        let sefer = state.held(&slug)?;
        let to = folder.join(girsa_export::suggested_name(&sefer, format));
        let fixes = {
            let shelf = state.shelf()?;
            shelf.fixes().clone()
        };
        (sefer, fixes, to, showing, pointing, shemos)
    };
    let done = girsa_export::export(&sefer, &fixes, format, pointing, shemos, &to)
        .map_err(|e| e.to_string())?;
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

// ── The transmission chain (spec.md §8, BUILDER.md W28) ─────────────────────

/// How far a walk goes, unless the reader says otherwise.
///
/// Clamped rather than trusted: the window sends this, and a depth of 400 is a
/// walk that reads the shelf. The library's own default is what an unclamped
/// caller gets.
const CHAIN_DEEPEST: usize = 12;

/// Which way the walk is not able to go without dates.
fn no_timeline() -> String {
    girsa_app::trouble::refuse(
        girsa_app::trouble::Code::NoSuch,
        "the catalogue could not be read, so nothing here knows when any sefer was written — \
         and which way a link points is a question about dates",
    )
}

/// How a line became halacha, or where a ruling came from (spec.md §8.1, §8.2).
///
/// The walk is `girsa-link`'s and is the same one `girsa-chain` prints on a
/// terminal. What this adds is nothing: if the panel and the tool could
/// disagree about which hops are real, they would be two answers to one
/// question, and the shape of the answer is the whole claim.
#[tauri::command(async)]
fn chain_walk(
    shared: tauri::State<'_, Shared>,
    at: String,
    direction: String,
    depth: Option<usize>,
) -> Result<girsa_app::chaining::Chain, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let direction = match direction.as_str() {
        "forward" => girsa_link::chain::Direction::Forward,
        "back" => girsa_link::chain::Direction::Back,
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such direction: {other}"),
            ))
        }
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // Taken before anything borrows the state, and put back after everything
    // that borrowed it is gone. See `State::chains`: the reader's two clicks
    // are two walks from one place, and the second one used to re-read every
    // shard the first had just finished with.
    let held = std::mem::take(&mut state.chains);
    let limits = girsa_link::chain::Limits {
        depth: depth.map_or(girsa_link::chain::Limits::default().depth, |asked| {
            asked.clamp(1, CHAIN_DEEPEST)
        }),
        ..girsa_link::chain::Limits::default()
    };
    let (chain, kept) = {
        let shelf = state.shelf()?;
        let timeline = state.timeline.as_ref().ok_or_else(no_timeline)?;
        let names = state.names(&shelf);
        let mut graph =
            girsa_link::chain::Graph::resuming(shelf.root(), timeline, shelf.repairs(), held);
        let chain = girsa_app::chaining::walk(&mut graph, &names, &at, direction, limits);
        (chain, graph.into_cache())
    };
    state.chains = kept;
    Ok(chain)
}

/// Where two rishonim read one gemara apart (spec.md §8.6).
#[tauri::command(async)]
fn chain_forks(
    shared: tauri::State<'_, Shared>,
    at: String,
) -> Result<girsa_app::chaining::Forked, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let held = std::mem::take(&mut state.chains);
    let (forked, kept) = {
        let shelf = state.shelf()?;
        let timeline = state.timeline.as_ref().ok_or_else(no_timeline)?;
        let names = state.names(&shelf);
        let mut graph =
            girsa_link::chain::Graph::resuming(shelf.root(), timeline, shelf.repairs(), held);
        let forked = girsa_app::chaining::forked(
            &mut graph,
            &names,
            &at,
            girsa_link::chain::Limits::default(),
        );
        (forked, graph.into_cache())
    };
    state.chains = kept;
    Ok(forked)
}

// ── The OCR queue (spec.md §7.3, BUILDER.md W21) ────────────────────────────

/// The next candidates to review, best first.
///
/// Re-read from disk every time: `girsa-suspects` is a batch job that runs
/// outside this window, and a queue held in memory would be the one from
/// before it ran.
#[tauri::command(async)]
fn suspects(shared: tauri::State<'_, Shared>, limit: usize) -> Result<Vec<SuspectRow>, String> {
    // The parse first, with no lock held. The queue is 28,124 lines on the
    // real corpus and it is re-read every time the drawer opens — see the note
    // above — so this is the one long read in the command and it needs the
    // personal path and nothing else out of the state.
    let personal = personal_of(&shared)?;
    let (queue, trouble) = girsa_fix::suspect::Queue::open(&personal);
    for line in trouble {
        eprintln!("{line}");
    }
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let rows: Vec<SuspectRow> = {
        let shelf = state.shelf()?;
        let names = state.names(&shelf);
        queue
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
            .collect()
    };
    state.queue = Some(queue);
    Ok(rows)
}

/// Open a candidate: where its word sits in the segment the queue named.
///
/// Two kinds of place, because the queue ranks words off both kinds of sefer.
/// `girsa-suspects` reads the index's term dictionary, and since W26 that
/// dictionary holds what an engine read off a photograph as well as what a
/// file said — so tesseract's misreads have always been ranked beside
/// Otzaria's. What they had no way to reach was **this** call: a page segment
/// carries no text of its own, so the character-span lookup below tokenized an
/// empty string and every candidate on a scan answered *that word is not in
/// that line any more*. The words of a page are in the reading, and the
/// correction is by ink, so a page takes the other branch end to end.
#[tauri::command(async)]
fn suspect_at(
    shared: tauri::State<'_, Shared>,
    id: String,
    at: String,
) -> Result<Standing, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    // The same 28,124-line read `suspects` does, and off the lock for the same
    // reason.
    let personal = personal_of(&shared)?;
    let (queue, _) = girsa_fix::suspect::Queue::open(&personal);
    let suspect = queue.get(&id).ok_or("there is no such candidate")?.clone();

    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.queue = Some(queue);
    // **No `shemos` here, and that is deliberate.** This is the correction
    // path: what it shows the reader is the word as the corpus has it, and
    // what they type goes back into their own layer as a claim about that
    // word. A box that offered `יקוק` for correction would be inviting a
    // reader to write a substitution into the corpus, which is the one place
    // this setting must not reach. `girsa_app::fixing` is silent about it for
    // the same reason.
    let pointing = state.session.pointing;

    // Which page of the file, if this is a page at all. Counted through the
    // pages rather than read off the ordinal, because a page that was split by
    // a correction mints `#47.1` and arithmetic on the ordinal would slip by
    // one from there (`girsa_app::scanning::page_of_id`).
    let page = {
        let sefer = state.sefer(at.work())?;
        let position = sefer.position_of(&at).ok_or("not in this sefer")?;
        let kind = sefer
            .segments
            .get(position)
            .ok_or("not in this sefer")?
            .kind;
        match kind {
            girsa_corpus::import::SegmentKind::Page => girsa_app::scanning::page_of_id(sefer, &at),
            _ => None,
        }
    };
    if let Some(page) = page {
        let slug = at.work().to_string();
        // The reading with the reader's own corrections already applied, which
        // is what makes a candidate they have already fixed report itself as
        // gone rather than opening a box on a word that no longer says that.
        let read = state
            .words(&slug)?
            .page(page)
            .ok_or_else(|| format!("nobody has read page {page} of {slug}"))?;
        let word = girsa_app::scanning::where_word_on_page(&read, &suspect.rare)
            .ok_or("that word is not on that page any more")?;
        let printed = read
            .words
            .get(word)
            .map(|on_page| on_page.text.clone())
            .unwrap_or_default();
        return Ok(Standing {
            at: at.to_string(),
            // A photograph has no text to take an offset into. The place is
            // the rectangle, and it is named by `page` and `word`.
            from_char: 0,
            to_char: 0,
            suggestion: suspect.suggestion(&printed),
            printed,
            page: Some(page),
            word: Some(word),
        });
    }

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
        page: None,
        word: None,
    })
}

/// Say what was done about a candidate: corrected, or not an error.
///
/// Recorded so the batch job does not ask again — never so that anything is
/// applied. The correction itself, if there is one, went through `fix`.
#[tauri::command(async)]
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
    // Take the held queue if there is one; read it off disk if there is not,
    // and do that read with nothing locked.
    let held = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state.queue.take()
    };
    let mut queue = match held {
        Some(queue) => queue,
        None => girsa_fix::suspect::Queue::open(&personal_of(&shared)?).0,
    };
    // `decide` appends to the decisions log, which is a file write.
    let known = queue.decide(&id, decision).map_err(|e| e.to_string())?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent = girsa_app::send(sefer, &selection, style, pointing, shemos, note)
        .map_err(|e| e.to_string())?;
    Ok(Copied {
        display: sent.display().to_string(),
        reference: sent.packet.reference.clone(),
        lines: sent.packet.text.lines().count(),
        put: clipboard::put(&sent),
    })
}

// ── The buffer (spec.md §10.3, BUILDER.md W17) ──────────────────────────────

#[tauri::command(async)]
fn buffers(shared: tauri::State<'_, Shared>) -> Result<Vec<String>, String> {
    let personal = personal_of(&shared)?;
    Ok(girsa_desk::Buffer::list(&personal))
}

#[tauri::command(async)]
fn buffer_open(shared: tauri::State<'_, Shared>, name: String) -> Result<Writing, String> {
    let personal = personal_of(&shared)?;
    let buffer = girsa_desk::Buffer::open(&personal, &name).map_err(|e| e.to_string())?;
    let path = girsa_desk::Buffer::path(&personal, &name).map_err(|e| e.to_string())?;
    Ok(Writing {
        name: buffer.name,
        text: buffer.text,
        path: path.display().to_string(),
    })
}

#[tauri::command(async)]
fn buffer_save(
    shared: tauri::State<'_, Shared>,
    name: String,
    text: String,
) -> Result<String, String> {
    let personal = personal_of(&shared)?;
    let mut buffer = girsa_desk::Buffer::new(name);
    buffer.text = text;
    Ok(buffer
        .save(&personal)
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
#[tauri::command(async)]
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
#[tauri::command(async)]
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
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent = girsa_app::send(sefer, &selection, style, pointing, shemos, None)
        .map_err(|e| e.to_string())?;
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
#[tauri::command(async)]
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
#[tauri::command(async)]
fn who_cites(
    shared: tauri::State<'_, Shared>,
    reference: String,
) -> Result<Vec<girsa_desk::Citing>, String> {
    let place: girsa_ref::Ref = reference.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let personal = {
        let shelf = state.shelf()?;
        shelf.personal().to_path_buf()
    };
    let documents = state.documents(&personal);
    Ok(girsa_desk::who_cites(&personal, documents, &place))
}

/// The citations in a piece of prose — **the certain ones** (spec.md §10.5).
///
/// Everything ambiguous stays plain text. See `girsa_desk::citing` for the three
/// rules and why each of them refuses more than it accepts.
#[tauri::command(async)]
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

/// Where a citation in your own writing goes (W19, spec.md §10.5).
///
/// The reader clicked words the pane drew as a link, and this turns the ref
/// those words carry into a place to open. It is the same
/// [`crate::post::landing`] a `girsa://` link from another application takes,
/// on purpose: a citation resolves to one place whether it was clicked in Ksav
/// or in a note of your own, and two resolutions would drift.
#[tauri::command(async)]
fn cite_open(shared: tauri::State<'_, Shared>, reference: String) -> Result<post::Landing, String> {
    let place: girsa_ref::Ref = reference.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    post::landing(&mut state, &place)
}

/// Whether Ksav is there (spec.md §10.6 — *presence*).
///
/// Asked of Ksav rather than assumed from a file: an endpoint left behind by a
/// crash is not presence. The window uses this to decide whether to *offer*
/// sending at all, which is the whole point — an affordance that would fail is
/// never shown.
/// # A8: the chip is telling the truth, and it is the sibling's truth
///
/// > *"it says {ksav} is registered but not answering. i have no clue if that
/// > is right."*
///
/// It is right. `girsa_post::presence` answers `Stale` when there **is** a
/// `ksav-endpoint.json` and nothing answers on the port it names, which is a
/// real state and not a guess — the endpoint is asked, over loopback, before
/// anything is said about it.
///
/// What produces it, nearly always, is the sibling leaving its registration
/// behind when it closes. **Girsa had exactly this bug and fixed it**: see the
/// note on `run()` below — `Builder::run` never returns on Windows, so
/// `Desk::drop` was never reached by any exit a reader can perform, and Ksav saw
/// every ordinary close of Girsa as *registered but not answering*. The fix is
/// the `RunEvent::Exit` callback that takes the desk out. Ksav's side of the
/// same defect is Ksav's to fix, and nothing here can reach it.
///
/// So this command stays exactly as honest as the crate is, and the **sentence**
/// changed instead (`say.ts`, `ksavStale`): the reader is told what it means and
/// what to do, rather than being told a state and left to work out whether it is
/// a crisis. `girsa_post::Endpoint` carries a `pid` for exactly this — *"so a
/// stale file can be told from a live one before anything is sent"* — and
/// `presence()` does not read it; asking whether that pid is alive needs a
/// process-table dependency on three platforms for one boolean, which is a
/// worse trade than a sentence that names the cause.
#[tauri::command(async)]
fn ksav_presence() -> girsa_post::Presence {
    girsa_post::presence(girsa_post::App::Ksav)
}

/// Send a selection straight into the open Ksav document.
///
/// The clipboard path (W15) works whether or not Ksav is running; this is the
/// one that feels like AirDrop, and it is only offered when presence says it
/// would land.
#[tauri::command(async)]
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
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent = girsa_app::send(sefer, &selection, style, pointing, shemos, note)
        .map_err(|e| e.to_string())?;
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
#[tauri::command(async)]
fn set_cite_style(shared: tauri::State<'_, Shared>, style: String) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.cite =
        girsa_cite::CiteStyle::named(&style).ok_or_else(|| format!("no such style: {style}"))?;
    state.save();
    Ok(())
}

#[tauri::command(async)]
fn open_tab(
    shared: tauri::State<'_, Shared>,
    slug: String,
    again: Option<bool>,
) -> Result<PaneId, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // A sefer reopens where it was left, which is the whole point of
    // remembering (BUILDER.md W9, per-sefer position memory).
    let at = state.session.where_i_was(&slug).cloned();
    // **Go to it if it is open**, rather than opening a second tab on one sefer
    // — `Workspace::open`, and the reason is in its doc comment. `again` is the
    // reader having asked for the other thing in so many words: see
    // `Workspace::open_again`.
    let pane = if again.unwrap_or(false) {
        state.session.workspace.open_again(&slug, at)
    } else {
        state.session.workspace.open(&slug, at)
    };
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
#[tauri::command(async)]
fn open_set(shared: tauri::State<'_, Shared>) -> Result<Vec<OpenSefer>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let language = state.session.language;
    let here = state
        .session
        .workspace
        .active_tab()
        .and_then(|tab| tab.pane(tab.focused))
        .map(|pane| pane.slug.clone());
    let shelf = state.shelf().ok();
    Ok(state
        .session
        .workspace
        .open_set()
        .into_iter()
        .map(|slug| {
            let named = shelf.as_ref().and_then(|s| s.work(&slug));
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

#[tauri::command(async)]
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

/// Move a pane into another tab (A12).
///
/// > *"make me be able to move from tab into another tab."*
///
/// `into` is a tab by index; `null` is a tab of its own. Answers whether
/// anything moved, so the window can tell *there was nowhere to go* from *it
/// went* — see `girsa_app::workspace::Workspace::move_pane`, which holds the
/// two refusals.
#[tauri::command(async)]
fn move_pane(
    shared: tauri::State<'_, Shared>,
    pane: PaneId,
    into: Option<usize>,
) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let moved = state.session.workspace.move_pane(pane, into);
    state.save();
    Ok(moved)
}

#[tauri::command(async)]
fn close_pane(shared: tauri::State<'_, Shared>, pane: PaneId) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.close(pane);
    state.save();
    Ok(())
}

/// Close a whole tab, from the tab strip, without opening it first (W40).
#[tauri::command(async)]
fn close_tab(shared: tauri::State<'_, Shared>, index: usize) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.close_tab(index);
    state.save();
    Ok(())
}

#[tauri::command(async)]
fn focus(shared: tauri::State<'_, Shared>, pane: PaneId) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.focus(pane);
    state.save();
    Ok(())
}

#[tauri::command(async)]
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

#[tauri::command(async)]
fn set_ratio(shared: tauri::State<'_, Shared>, split: usize, ratio: u16) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.set_ratio(split, ratio);
    state.save();
    Ok(())
}

/// Turn the split this pane stands in, side by side to stacked and back.
///
/// > *"Tabs should be splittable in any way and movable, like we want in
/// > ksav."*
///
/// Answers whether there was a split to turn, so the window can leave the
/// control off a pane standing alone rather than offering a gesture that does
/// nothing. See [`girsa_app::workspace::Workspace::turn_split`].
#[tauri::command(async)]
fn turn_split(shared: tauri::State<'_, Shared>, split: usize) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let turned = state.session.workspace.turn_split(split).is_some();
    state.save();
    Ok(turned)
}

/// Swap the two halves of the split this pane stands in.
#[tauri::command(async)]
fn swap_split(shared: tauri::State<'_, Shared>, split: usize) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let swapped = state.session.workspace.swap_split(split);
    state.save();
    Ok(swapped)
}

/// Move a tab along the strip — the drag half of *movable*.
#[tauri::command(async)]
fn move_tab(shared: tauri::State<'_, Shared>, from: usize, to: usize) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let moved = state.session.workspace.move_tab(from, to);
    state.save();
    Ok(moved)
}

#[tauri::command(async)]
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

/// Whether there is a newer Girsa — **asked, never volunteered**.
///
/// spec.md §14: offline is the product. A window that has not been asked makes
/// no request, keeps no timer and needs no setting to turn off, which is a
/// stronger promise than a setting that defaults to off. Otzaria checks on
/// start; this checks when a reader presses the button, which is the same
/// information one gesture later.
///
/// It does not install anything, and `girsa_app::newer` says why at length:
/// installing means verifying a signature, and an updater that ran an unsigned
/// binary off the internet would be the worst thing in the application by a
/// distance.
#[tauri::command(async)]
fn check_for_update() -> Result<girsa_app::newer::Newer, String> {
    girsa_app::newer::check(env!("CARGO_PKG_VERSION"))
        .map_err(|e| girsa_app::trouble::refuse(girsa_app::trouble::Code::Offline, e.to_string()))
}

/// Open the releases page on the machine's own browser.
///
/// No argument, on purpose: [`girsa_app::newer::open_releases`] opens one
/// address, compiled in. A command that opened whatever URL it was handed is a
/// command that opens whatever a bug hands it.
#[tauri::command(async)]
fn open_releases() -> Result<(), String> {
    girsa_app::newer::open_releases()
        .map_err(|e| girsa_app::trouble::refuse(girsa_app::trouble::Code::Offline, e.to_string()))
}

/// A named arrangement, as the panel lists it.
#[derive(Serialize)]
struct DeskRow {
    name: String,
    /// How many tabs it holds, and how many distinct seforim across them.
    tabs: usize,
    seforim: usize,
    /// Whether this is the one the reader is sitting at.
    here: bool,
}

/// Every arrangement the reader has named.
fn desk_rows(state: &State) -> Vec<DeskRow> {
    state
        .session
        .desks
        .iter()
        .map(|(name, workspace)| {
            let mut slugs: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
            for tab in &workspace.tabs {
                for pane in &tab.panes {
                    slugs.insert(pane.slug.as_str());
                }
            }
            DeskRow {
                name: name.clone(),
                tabs: workspace.tabs.len(),
                seforim: slugs.len(),
                here: state.session.desk.as_deref() == Some(name.as_str()),
            }
        })
        .collect()
}

/// The arrangements the reader has named (`Session::desks`).
#[tauri::command(async)]
fn desks(shared: tauri::State<'_, Shared>) -> Result<Vec<DeskRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    Ok(desk_rows(&state))
}

/// Keep how the seforim are laid out right now, under a name.
///
/// Overwrites a desk of the same name rather than minting `sugya (2)`: a reader
/// typing a name they already used means *this one, as it is now*, and a second
/// desk with a number after it is a thing nobody asked for and has to clean up.
#[tauri::command(async)]
fn desk_keep(shared: tauri::State<'_, Shared>, name: String) -> Result<Vec<DeskRow>, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            "a desk needs a name".to_string(),
        ));
    }
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let now = state.session.workspace.clone();
    state.session.desks.insert(name.clone(), now);
    state.session.desk = Some(name);
    state.save();
    Ok(desk_rows(&state))
}

/// Sit down at one.
///
/// **The arrangement on screen is written back first.** A switcher that threw
/// away what you had set up in order to show you something else would be a
/// switcher nobody uses twice — and the reader who is not sitting at a named
/// desk loses nothing either, because the session's own arrangement is saved on
/// every change and is what they come back to.
#[tauri::command(async)]
fn desk_open(shared: tauri::State<'_, Shared>, name: String) -> Result<Vec<DeskRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let Some(going_to) = state.session.desks.get(&name).cloned() else {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            format!("there is no desk called {name}"),
        ));
    };
    if let Some(here) = state.session.desk.clone() {
        let now = state.session.workspace.clone();
        state.session.desks.insert(here, now);
    }
    state.session.workspace = going_to;
    state.session.desk = Some(name);
    state.save();
    Ok(desk_rows(&state))
}

/// Forget one.
///
/// The arrangement on screen is untouched, even when it is the desk being
/// forgotten: *stop keeping this* is not *close everything*, and a reader who
/// meant the second one can close the panes.
#[tauri::command(async)]
fn desk_forget(shared: tauri::State<'_, Shared>, name: String) -> Result<Vec<DeskRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.desks.remove(&name);
    if state.session.desk.as_deref() == Some(name.as_str()) {
        state.session.desk = None;
    }
    state.save();
    Ok(desk_rows(&state))
}

/// The words at the other end of a set of links, for one sefer.
///
/// # The finding this closes
///
/// > *"all of it — i don't know what i'm looking at."*
///
/// The links panel on one line of Yoreh De'ah draws 280 rows. **Seventy-eight
/// of them say `כף החיים על שולחן ערוך יורה דעה` and differ by one number.**
/// They are not duplicates — the Kaf HaChayim really writes ס״ק א׳ through
/// ס״ק ע״ח on that one se'if — but what repeats down the column is the sefer's
/// name, seventy-eight times, and what a reader wants is *what does it say*.
///
/// `LinkRow::preview` was the first answer and it is only filled when the other
/// sefer **is already open**, for a reason that still holds: a sidebar is not
/// entitled to read forty seforim off the disk to decorate a list. Grouping the
/// panel by sefer is what makes the reason stop biting — a reader opens one
/// group, and this reads one sefer.
///
/// So the whole line comes back as well as its opening words. Once a group is
/// open, expanding any row in it costs nothing: the words are already here, and
/// a reader walking seventy-eight ס״ק is not waiting on seventy-eight round
/// trips.
#[tauri::command(async)]
fn link_words(
    shared: tauri::State<'_, Shared>,
    work: String,
    ats: Vec<String>,
) -> Result<Vec<Words>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let sefer = state.sefer(&work)?;
    let mut out = Vec::with_capacity(ats.len());
    for at in &ats {
        let Ok(id) = at.parse::<SegmentId>() else {
            continue;
        };
        let Some(nth) = sefer.position_of(&id) else {
            // The graph points at a segment this sefer does not have. Left out
            // rather than reported as an empty quote: it is a fact about the
            // link, and W23's panel is where a bad link is repaired.
            continue;
        };
        let Some(segment) = sefer.segments.get(nth) else {
            continue;
        };
        let said = girsa_app::shemos::written(&segment.text, shemos);
        let drawn = display::Shown::of(&said, pointing).text().to_string();
        out.push(Words {
            at: at.clone(),
            opening: girsa_app::enough::first_words(&drawn),
            said: drawn,
            address: girsa_app::sending::printed_address_in(
                &sefer.work,
                Some(sefer.sections()),
                &segment.id,
                style,
            ),
        });
    }
    Ok(out)
}

/// One end of a link, quoted.
#[derive(Serialize)]
struct Words {
    at: String,
    /// The opening words, for the row as it stands.
    opening: String,
    /// The whole line, for the row opened out.
    said: String,
    /// Where it is, printed the reader's way.
    address: String,
}

/// A sheet of paper: what the reader is standing in, ready to print.
///
/// > *Print the daf for the shiur.* — which, before this, meant export to
/// > `.docx`, open Word, and print from there.
///
/// **The section and not the sefer.** A siman, an amud, a perek — see
/// [`girsa_app::printing`] for why that is found from the address rather than
/// by counting lines. The whole sefer on paper is what the export is for.
///
/// The lines come back as ordinary [`Line`]s, which is the point: the reader's
/// corrections are already applied, the pointing is theirs, and the shemos are
/// written the way they asked — so what prints is what is on the screen, and
/// nothing has a second idea of what the sefer says.
#[tauri::command]
fn sefer_sheet(shared: tauri::State<'_, Shared>, at: String, whole: bool) -> Result<Sheet, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let (sefer, lexicon) = state.reading(at.work())?;
    let how = if whole {
        girsa_app::printing::Sheet::Chosen
    } else {
        girsa_app::printing::Sheet::Section
    };
    let (from, to) = girsa_app::printing::run_of(sefer, &at, how)
        .ok_or_else(|| format!("{at} is not in this sefer"))?;
    let mut lines: Vec<Line> = sefer.segments[from..to]
        .iter()
        .map(|s| Line::of(sefer, s, pointing, shemos, style, lexicon))
        .collect();
    // The sheet's own from–to is read off the lines, so it has to be read
    // before the repeats are emptied — otherwise a sheet running from the
    // second mishnah of a perek is headed `משנה ב'` with no perek on it.
    let whole_address = |line: Option<&Line>| {
        line.map(|l| {
            if l.above.is_empty() {
                l.address.clone()
            } else {
                format!("{} {}", l.above, l.address)
            }
        })
        .unwrap_or_default()
    };
    let address = whole_address(lines.first());
    let to_address = whole_address(lines.last());
    girsa_app::view::only_when_it_changes(&mut lines);
    Ok(Sheet {
        title: girsa_app::printing::header(sefer),
        address,
        to_address,
        lines,
    })
}

/// A printable run of a sefer, with what has to be on the page beside it.
#[derive(Serialize)]
struct Sheet {
    /// The sefer, the edition and the terms — spec.md §13, on paper.
    title: Vec<String>,
    /// Where the sheet starts and where it ends, printed the reader's way.
    address: String,
    to_address: String,
    lines: Vec<Line>,
}

/// Set one chip on the **find bar**, which is not the panel's chip row.
///
/// Named rather than free-form, like `find_chip`: the window may choose among
/// the options the engine offered and may not invent one.
#[tauri::command(async)]
fn find_here_chip(
    shared: tauri::State<'_, Shared>,
    chip: String,
    key: String,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state
        .here_chips
        .choose(&chip, &key)
        .map_err(|e| e.to_string())
}

/// Find a phrase **inside one sefer** (`girsa_app::inside`).
///
/// The gesture every application has and this one did not. Not the search bar
/// narrowed to one work: the whole sefer is scanned, in reading order, and what
/// comes back is every place with the offsets the pane can highlight.
///
/// Scanned per keystroke rather than indexed, and that is a measured choice
/// rather than laziness: the largest sefer on the shelf is Mishnah Berurah at
/// 17,418 segments, and folding that many short strings is well under the
/// frame a reader would notice. An index would have to be invalidated by every
/// correction the reader makes, which is a second copy of a problem
/// `girsa-fix` already solved once.
#[tauri::command(async)]
fn sefer_find(
    shared: tauri::State<'_, Shared>,
    slug: String,
    query: String,
) -> Result<FoundHere, String> {
    // The find bar runs on every keystroke, so it is the *last* place that
    // should hold the state lock across an engine call. Chips out, guard
    // down, ask, then back in to draw. See `find` for the whole argument.
    let (chips, bar) = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        // The scope is this sefer, replaced on every ask. It is the one chip a
        // reader cannot set here, because setting it is what would stop this
        // being a find.
        state.here_chips.scope = girsa_search::scope::Scope::everything()
            .only([slug.clone()], girsa_app::inside::THIS_SEFER);
        let chips = state.here_chips.clone();
        if query.trim().is_empty() {
            // The same rule the panel has: an empty box is not a refusal. The
            // bar asks with `""` to draw its chip row before anything is typed.
            return Ok(FoundHere {
                places: Vec::new(),
                total: 0,
                chips: chips.row(),
                refused: None,
            });
        }
        let Some(bar) = state.bar.clone() else {
            let why = state.no_search();
            return Ok(FoundHere {
                places: Vec::new(),
                total: 0,
                chips: chips.row(),
                refused: Some(why),
            });
        };
        (chips, bar)
    };
    let answer = bar.ask(
        &query,
        &chips,
        Paging {
            from: 0,
            size: girsa_app::inside::MOST,
        },
        &girsa_ref::resolve::Context::default(),
    );
    // **The engine says which segments and which words; `inside` says where
    // those words are on the drawn page.** Two questions with two right
    // answers: the engine's marks are byte ranges into the text it indexed, and
    // the pane highlights characters of the text it drew — which differ by
    // every mark, every tag and every shem the reader asked to have rewritten.
    let (hits, refused) = match answer {
        Answer::Segments { results, .. } => (
            results
                .hits
                .iter()
                .map(|hit| (hit.id.clone(), marked(&results.marker, hit)))
                .collect::<Vec<_>>(),
            None,
        ),
        Answer::Refused(why) => (Vec::new(), Some(why)),
        // A mareh makom typed at a find bar. The bar is *inside one sefer* and
        // a citation is a jump somewhere else, so nothing is found — and the
        // search panel, which is where a citation belongs, is one key away.
        Answer::Cited(_) => (Vec::new(), None),
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let (sefer, _) = state.reading(&slug)?;
    let found = girsa_app::inside::where_marked(sefer, &hits, pointing, shemos, style);
    Ok(FoundHere {
        places: found.places,
        total: found.total,
        chips: chips.row(),
        refused,
    })
}

/// What a find inside one sefer found, with the row of options it ran under.
#[derive(Serialize)]
struct FoundHere {
    places: Vec<girsa_app::inside::Found>,
    total: usize,
    /// The chip row, so the bar draws what the engine will do rather than what
    /// it last drew.
    chips: Vec<Chip>,
    /// A refusal, in the engine's own words — a regex that will not compile,
    /// an index that is not there.
    refused: Option<String>,
}

/// What day it is, and what is being learned on it (`girsa_app::luach`).
///
/// **The window says which day**, and that is the whole of why this takes three
/// numbers instead of reading a clock. `std::time` knows the number of seconds
/// since 1970 and nothing at all about the reader's timezone; a Rust-side
/// `today` would be UTC, which is the previous evening in New York and three in
/// the morning in Yerushalayim. The webview has the machine's own calendar, so
/// it is asked.
///
/// Each limud is marked for whether its sefer is actually on this shelf, so the
/// window can offer the daf without promising a sefer nobody imported.
#[tauri::command(async)]
fn luach(
    shared: tauri::State<'_, Shared>,
    year: i32,
    month: u32,
    day: u32,
    hour: u8,
) -> Result<girsa_app::luach::Luach, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    // The hour comes from the same clock the date does, and the turnover is a
    // setting rather than a calculation — `girsa_app::luach::at` argues why.
    let mut luach = girsa_app::luach::at(
        girsa_app::luach::Civil { year, month, day },
        hour,
        state.session.day_turns_at,
    );
    if let Ok(shelf) = state.shelf() {
        for limud in luach.today.iter_mut().chain(luach.tomorrow.iter_mut()) {
            limud.here = shelf.work(&limud.slug).is_some();
        }
    }
    Ok(luach)
}

/// How the shemos are written (`girsa_app::shemos`).
///
/// > *"i would like if you could add a feature that every יהוה or אל or אלהים
/// > or anything like that could optionally not be written as a shem hashem."*
///
/// A page with a shem on it may not be thrown away, and neither may a printout
/// of one. Every sefer solves it by changing a letter, and so does this — on
/// the page, in the search results, in a quote and in an export, which is three
/// surfaces more than Otzaria's own setting covers.
#[tauri::command(async)]
fn set_shemos(shared: tauri::State<'_, Shared>, shemos: String) -> Result<(), String> {
    let Some(shemos) = girsa_app::shemos::Shemos::named(&shemos) else {
        // Refused by name rather than defaulted, for the reason `set_pointing`
        // gives: a window sending a spelling this project does not write has a
        // wiring bug, and quietly drawing the shemos as they stand is how it
        // would never be found.
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            format!("no such setting for the shemos: {shemos}"),
        ));
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.shemos = shemos;
    state.save();
    Ok(())
}

/// The hour the daf turns over, where the reader is standing.
///
/// Refused outside 0–23 rather than clamped, for the reason `set_shemos` gives:
/// a window sending 25 has a wiring bug, and a silently clamped 23 is how it
/// would never be found.
#[tauri::command(async)]
fn set_day_turns_at(shared: tauri::State<'_, Shared>, hour: u8) -> Result<(), String> {
    if hour > 23 {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            format!("an hour is 0 to 23, not {hour}"),
        ));
    }
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.day_turns_at = hour;
    state.save();
    Ok(())
}

#[tauri::command(async)]
fn settings(shared: tauri::State<'_, Shared>) -> Result<SettingsView, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let session = &state.session;
    let bound = girsa_app::keys::Bound::of(&session.keys);
    Ok(SettingsView {
        pointing: session.pointing,
        shemos: session.shemos,
        day_turns_at: session.day_turns_at,
        text_size: session.text_size,
        mefarshim_size: session.mefarshim_size,
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
#[tauri::command(async)]
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
#[tauri::command(async)]
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
#[tauri::command(async)]
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
#[tauri::command(async)]
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
#[tauri::command(async)]
fn set_interface(
    shared: tauri::State<'_, Shared>,
    language: girsa_app::session::Language,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.interface = language;
    state.save();
    Ok(())
}

#[tauri::command(async)]
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

/// The same, for the mefarshim beside the daf.
///
/// > *"I think it is a good idea to have a separate control for mefarshim and
/// > top level."*
///
/// A second command rather than a second argument to the first: `A+` in the
/// toolbar and `Ctrl+=` mean *the sefer*, and a size row that could silently be
/// either would be one control with two meanings. The clamp is
/// `Session::sane`'s, the same one, for the reason its own comment gives.
#[tauri::command(async)]
fn set_mefarshim_size(shared: tauri::State<'_, Shared>, percent: u16) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.mefarshim_size = percent;
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

#[tauri::command(async)]
fn lane_state(shared: tauri::State<'_, Shared>) -> Result<LaneRow, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    lane_row(&state)
}

/// Ask the lane. Never an error — a lane that is off, adrift or empty comes
/// back with `refused` set and the coverage sentence said.
#[tauri::command(async)]
fn lane_ask(
    shared: tauri::State<'_, Shared>,
    text: String,
    limit: Option<usize>,
) -> Result<LaneAnswer, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    let lane = state.lane.as_ref().ok_or_else(|| state.trouble())?;
    // Scoped by the same chip the literal search is scoped by, so *the whole
    // shelf* and *this sefer* mean the same thing in both columns.
    let scoped: Vec<String> = state.chips.scope.works().into_iter().collect();
    // The same `Names` the search column uses, so the two columns beside each
    // other cannot call one sefer by two names.
    let names = girsa_app::Names::new(
        &shelf,
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
        asking: answer.asking,
        shortlisted: answer.shortlisted,
    })
}

/// Turn the lane on or off, and point it at a model.
///
/// Turning it on loads the model, which is hundreds of megabytes — so this can
/// take a moment, and a model that will not load is **not** an error here. It is
/// [`girsa_lane::State::Adrift`], which the header says out loud rather than a
/// click that failed silently.
#[tauri::command(async)]
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
    let shelf = shelf
        .as_ref()
        .ok_or(no_shelf)?
        .read()
        .map_err(|_| State::poisoned())?;
    let lane = lane.as_mut().ok_or("there is no lane here")?;
    let was = lane.lane().settings().clone();
    let settings = girsa_lane::Settings {
        on,
        model: model.map(PathBuf::from).or(was.model),
        may_fetch: was.may_fetch,
    };
    let done = lane.set(settings, &shelf).map_err(|e| e.to_string());
    drop(shelf);
    done?;
    lane_row(&state)
}

/// Let Girsa go and get a model, or stop it being able to.
///
/// Its own command rather than a field on `lane_set`, because it is its own
/// decision: spec.md §14 says Girsa never *needs* the network, and this is the
/// switch that makes that sentence true in a fresh install.
#[tauri::command(async)]
fn lane_allow_fetch(shared: tauri::State<'_, Shared>, allow: bool) -> Result<LaneRow, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let no_shelf = state.trouble();
    // Two disjoint fields: the shelf read, the lane written. Taken apart by
    // hand because a method on `State` would borrow the whole of it.
    let State { shelf, lane, .. } = &mut *state;
    let shelf = shelf
        .as_ref()
        .ok_or(no_shelf)?
        .read()
        .map_err(|_| State::poisoned())?;
    let lane = lane.as_mut().ok_or("there is no lane here")?;
    let settings = girsa_lane::Settings {
        may_fetch: allow,
        ..lane.lane().settings().clone()
    };
    let done = lane.set(settings, &shelf).map_err(|e| e.to_string());
    drop(shelf);
    done?;
    lane_row(&state)
}

/// Bring a model in. Needs `lane_allow_fetch` first.
///
/// Runs on its own thread and emits [`BRING_EVENT`], so the panel draws a bar
/// and the reader can carry on learning. Stopping is closing the panel: the
/// `.part` file stays and the next press resumes where it left off.
#[tauri::command(async)]
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
                    let shelf = shelf.as_ref().and_then(|s| s.read().ok());
                    if let (Some(shelf), Some(lane)) = (shelf, lane.as_mut()) {
                        let settings = girsa_lane::Settings {
                            on: true,
                            model: Some(dir.clone()),
                            may_fetch: true,
                        };
                        if let Err(e) = lane.set(settings, &shelf) {
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
#[tauri::command(async)]
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
    let shelf = shelf
        .as_ref()
        .ok_or(no_shelf)?
        .read()
        .map_err(|_| State::poisoned())?;
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
    let done = lane.choose(chosen, &shelf).map_err(|e| e.to_string());
    drop(shelf);
    done?;
    lane_row(&state)
}

/// Embed what is chosen, on its own thread, emitting [`EMBED_EVENT`].
///
/// The lane is **cloned** for the thread, which shares the one loaded model
/// rather than loading a second — see `girsa_lane::Lane`. The state lock is held
/// only long enough to take that clone, so nothing about reading a sefer waits
/// on this.
#[tauri::command(async)]
fn lane_embed(app: tauri::AppHandle, shared: tauri::State<'_, Shared>) -> Result<(), String> {
    use tauri::Emitter;
    let (lane, root, slugs, titles, stop) = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let shelf = state.shelf()?;
        let held = state.lane.as_ref().ok_or("there is no lane here")?;
        if !held.state().is_on() {
            return Err(girsa_lane::LaneError::Off.to_string());
        }
        let slugs = girsa_nearby::adjacent::in_the_lane(&shelf, held.lane().chosen());
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
            let shelf = shelf.as_ref().and_then(|s| s.read().ok());
            if let (Some(shelf), Some(held)) = (shelf, lane.as_mut()) {
                held.refresh(&shelf);
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
#[tauri::command(async)]
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
#[tauri::command(async)]
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
    let root = {
        let shelf = state.shelf()?;
        shelf.root().to_path_buf()
    };

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
        if let Ok(shelf) = state.shelf() {
            if let Some(scan) = girsa_app::scan_of(&shelf, follower) {
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
                    shelf: shelf.map(|shelf| Arc::new(std::sync::RwLock::new(shelf))),
                    documents: None,
                    timeline,
                    chains: girsa_link::chain::Cache::default(),
                    bar,
                    no_search,
                    chips: Chips::default(),
                    here_chips: Chips::default(),
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
            choose_the_usual,
            pair_alongside,
            mefarshim_at,
            open_sefer,
            sefer_lines,
            sefer_index_of,
            sefer_indices_of,
            open_tab,
            sefer_contents,
            open_set,
            scan,
            scan_at,
            scan_map,
            scan_reading,
            scan_read_page,
            scan_ocr_page,
            scan_words,
            scan_fix,
            scan_mark,
            scan_marks,
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
            move_pane,
            close_tab,
            focus,
            set_follows,
            set_ratio,
            turn_split,
            swap_split,
            move_tab,
            set_pointing,
            set_shemos,
            set_day_turns_at,
            luach,
            sefer_find,
            find_here_chip,
            sefer_sheet,
            link_words,
            desks,
            desk_keep,
            desk_open,
            desk_forget,
            check_for_update,
            open_releases,
            set_language,
            set_interface,
            settings,
            set_look,
            bind_key,
            what_key,
            set_text_size,
            set_mefarshim_size,
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
            find_scope_set,
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
            cite_open,
            fix,
            unfix,
            set_showing,
            fixes,
            chain_walk,
            chain_forks,
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
#[tauri::command(async)]
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
    let shemos = state.session.shemos;
    let drawn = display::Shown::of(&girsa_app::shemos::written(&base, shemos), pointing)
        .text()
        .to_string();

    let shelf = state.shelf()?;
    let found = girsa_app::yours(&shelf, &standing, &drawn);
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
#[tauri::command(async)]
fn notes(shared: tauri::State<'_, Shared>) -> Result<Vec<NoteRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    let mut rows: Vec<NoteRow> = shelf.notes().all().map(NoteRow::of).collect();
    girsa_app::view::NoteRow::newest_first(&mut rows);
    Ok(rows)
}

/// Write a note about where you are standing. The three-second one.
#[tauri::command(async)]
fn note_write(
    shared: tauri::State<'_, Shared>,
    at: String,
    title: Option<String>,
    text: String,
) -> Result<NoteRow, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let who = girsa_app::who();
    let note = {
        let mut shelf = state.shelf_mut()?;
        girsa_app::note_here(&mut shelf, &at, title.as_deref(), &text, &who)
            .map_err(|e| e.to_string())?
    };
    let slug = note.slug.clone();
    state.searchable(&slug);
    Ok(NoteRow::of(&note))
}

/// One note, paragraph by paragraph, for editing it.
#[tauri::command(async)]
fn note_read(shared: tauri::State<'_, Shared>, note: String) -> Result<Vec<ParaRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
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
#[tauri::command(async)]
fn note_edit(
    shared: tauri::State<'_, Shared>,
    note: String,
    does: String,
    value: Option<String>,
    text: Option<String>,
) -> Result<Vec<ParaRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // Read out, edited in hand, written back. The shelf is not held open
    // across the edit: nothing between the two locks touches it.
    let mut held = {
        let shelf = state.shelf()?;
        shelf
            .notes()
            .get(&note)
            .cloned()
            .ok_or_else(|| format!("there is no note called {note}"))?
    };

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
    let written = {
        let mut shelf = state.shelf_mut()?;
        shelf.write_note(held).map_err(|e| e.to_string())?
    };
    let rows: Vec<ParaRow> = written
        .paras()
        .iter()
        .map(|para| ParaRow {
            id: para.id.to_string(),
            text: para.text.clone(),
        })
        .collect();
    let slug = written.slug.clone();
    // An edit is a rewrite of the note, so the index gets the whole note back.
    // A work is the unit of replacement, which makes *changed* and *new* the
    // same operation here.
    state.searchable(&slug);
    Ok(rows)
}

/// Throw a note away — the file, the sefer and the catalogue line.
#[tauri::command(async)]
fn note_forget(shared: tauri::State<'_, Shared>, note: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let (slug, gone) = {
        let mut shelf = state.shelf_mut()?;
        // A note is a sefer, and a sefer that has gone may not stay open in a
        // pane holding text nothing on the shelf accounts for.
        let slug = shelf.notes().get(&note).map(|held| held.slug.clone());
        let gone = shelf.forget_note(&note).map_err(|e| e.to_string())?;
        (slug, gone)
    };
    if let Some(slug) = slug {
        state.open.forget(&slug);
        state.unsearchable(&slug);
    }
    Ok(gone)
}

/// Highlight some words, or mark the place.
///
/// The words are read out of the line as the pane drew it and stored with the
/// mark, because an offset is not a place (`girsa_corpus::span`).
#[tauri::command(async)]
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
    let shemos = state.session.shemos;
    let base = {
        let sefer = state.sefer(at.work())?;
        sefer
            .position_of(&at)
            .and_then(|nth| sefer.segments.get(nth))
            .map(|segment| segment.text.clone())
            .unwrap_or_default()
    };
    let drawn = display::Shown::of(&girsa_app::shemos::written(&base, shemos), pointing)
        .text()
        .to_string();

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

    let mut shelf = state.shelf_mut()?;
    let held = shelf
        .marks_mut()
        .add(made)
        .map_err(|e| e.to_string())?
        .clone();
    let placed = held.place(&drawn);
    Ok(MarkRow::of(&girsa_app::Marked { mark: held, placed }))
}

/// Take a mark back.
#[tauri::command(async)]
fn mark_forget(shared: tauri::State<'_, Shared>, mark: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
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
#[tauri::command(async)]
fn marks_in(shared: tauri::State<'_, Shared>, slug: String) -> Result<Vec<MarkRow>, String> {
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let drawn: HashMap<String, String> = {
        let sefer = state.sefer(&slug)?;
        sefer
            .segments
            .iter()
            .map(|segment| {
                (
                    segment.id.to_string(),
                    display::Shown::of(
                        &girsa_app::shemos::written(&segment.text, shemos),
                        pointing,
                    )
                    .text()
                    .to_string(),
                )
            })
            .collect()
    };
    let shelf = state.shelf()?;
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
#[tauri::command(async)]
fn bookmarks(shared: tauri::State<'_, Shared>) -> Result<Vec<MarkRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
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
#[tauri::command(async)]
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

    let mut shelf = state.shelf_mut()?;
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
#[tauri::command(async)]
fn queries(shared: tauri::State<'_, Shared>) -> Result<Vec<QueryRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
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
#[tauri::command(async)]
fn query_recall(shared: tauri::State<'_, Shared>, name: String) -> Result<String, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let held = {
        let shelf = state.shelf()?;
        shelf
            .queries()
            .get(&name)
            .ok_or_else(|| format!("there is no saved query called {name}"))?
            .clone()
    };

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
#[tauri::command(async)]
fn query_forget(shared: tauri::State<'_, Shared>, name: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
    shelf.queries_mut().remove(&name).map_err(|e| e.to_string())
}

/// Your chaburah folders.
#[tauri::command(async)]
fn folders(shared: tauri::State<'_, Shared>) -> Result<Vec<FolderRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    let names = state.names(&shelf);
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
#[tauri::command(async)]
fn folder_edit(
    shared: tauri::State<'_, Shared>,
    name: String,
    title: Option<String>,
    does: String,
    member: String,
) -> Result<usize, String> {
    let member: girsa_note::Member = member.parse()?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
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
#[tauri::command(async)]
fn folder_forget(shared: tauri::State<'_, Shared>, name: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
    shelf
        .collections_mut()
        .remove(&name)
        .map_err(|e| e.to_string())
}

/// Every tag across your whole layer.
#[tauri::command(async)]
fn tags(shared: tauri::State<'_, Shared>) -> Result<Vec<TagRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
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
#[tauri::command(async)]
fn export_layer(shared: tauri::State<'_, Shared>, into: Option<String>) -> Result<String, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
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
