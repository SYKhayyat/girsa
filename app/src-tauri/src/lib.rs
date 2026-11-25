//! The window, and the commands behind it.
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
mod commands;
mod post;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use girsa_app::shelf::Open;
use girsa_app::trouble::{refuse, Code};
use girsa_app::view::{
    AtRow, CoveredRow, FolderMember, FolderRow, LaneAnswer, LaneProgress, LaneRow, Line,
    ModelOffer, Move, NearRow, QueryRow, SettingsView, Shortcut, TagRow,
};
use girsa_app::workspace::PaneId;
use girsa_app::{display, Beside, Session, Shelf, Workspace};
use girsa_corpus::segment::SegmentId;
use girsa_search::bar::{Answer, Bar};
use girsa_search::chips::{Chip, Chips};
use girsa_search::index::Paging;
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
    /// The desk's `/document-saved` clears it, because that is where a row is added.
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
    /// The resolver's lexicon, for linkify (W19). Read once: it is 27,667
    /// spellings and a citation is looked up per word of prose.
    pub(crate) lexicon: Option<girsa_ref::Lexicon>,
    /// The OCR queue, once it has been looked at (W21). Written by
    /// `girsa-suspects`, which is a batch job outside this window, so it is
    /// re-read whenever the drawer is opened rather than held as truth.
    queue: Option<girsa_fix::suspect::Queue>,
    /// The semantic lane (spec.md §9.9, W30). Held open like the index, because
    /// turning it on loads a model — and **off costs nothing**, which is what
    /// makes off-by-default a real default rather than a checkbox with a price.
    /// The lane, **behind its own lock**, for the reason the shelf is.
    ///
    /// `Adjacency::ask` embeds the query with a neural model and searches every
    /// vector store in the lane. It is the heaviest single operation in the
    /// application and it used to run with the global state guard held for all
    /// of it — so a reader who asked the lane a question could not scroll until
    /// it answered. Behind its own lock, `lane_ask` takes the state guard to
    /// pick up two handles, lets it go, and asks.
    ///
    /// # The order is shelf, then lane
    ///
    /// Both are taken together by the settings commands, which read the shelf
    /// and write the lane, so there is an order and it has to be one order.
    /// **Shelf first, lane second, always.** The state guard comes before both
    /// or not at all.
    lane: Option<Arc<std::sync::RwLock<girsa_nearby::Adjacency>>>,
    /// Set to stop the embedding job. It is checked between batches, so
    /// stopping costs the batch in flight and nothing else.
    stop_embedding: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Whether a long job — embedding, or bringing a model in — is already
    /// running.
    ///
    /// # Why this is in Rust and not on the button
    ///
    /// The window's guard was `go.disabled = this.working`, evaluated **when
    /// the row is drawn**. The click handler set `working = true` and never
    /// disabled the button it was on, and the row is not redrawn until the
    /// first progress event arrives — so two clicks inside that gap started two
    /// threads, each opening its own `Vectors` store for the same slug and
    /// appending to the same files, re-embedding every segment the other was
    /// already embedding, for a job measured in hours.
    ///
    /// And it **defeated Stop**: every run begins `stop.store(false)` on the one
    /// shared flag, so a run starting after the reader pressed Stop cleared the
    /// flag for the run that was stopping.
    ///
    /// Disabling the button in the click handler is worth doing too and is not
    /// sufficient on its own. The loopback and a second window are not bound by
    /// the DOM, and neither is the MCP server. A `swap` is: the second caller
    /// loses the race and is told so, by name.
    working: std::sync::Arc<std::sync::atomic::AtomicBool>,
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
    /// Slug → whether anything in the sefer is pointed.
    ///
    /// A property of the **work**, and it was being recomputed on every open:
    /// `segments.iter().any(display::has_marks)` short-circuits on a pointed
    /// sefer and reads all 7.7 MB of an unpointed one, once per pane, for an
    /// answer that cannot change while the sefer is in memory.
    ///
    /// It belongs on the catalogue row, computed at import, and that is a
    /// corpus-format change that would leave every already-imported shelf
    /// without it. Held here instead: same answer, computed once per read of
    /// the work rather than once per open, and dropped beside `marks` when the
    /// sefer is evicted or re-read — because both are facts about the text that
    /// was just thrown away.
    pointed: HashMap<String, bool>,
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
            self.pointed.remove(&gone);
        }
        Ok(())
    }

    pub(crate) fn sefer(&mut self, slug: &str) -> Result<&Open, String> {
        self.load(slug)?;
        self.open
            .get(slug)
            .map(AsRef::as_ref)
            .ok_or_else(|| refuse(Code::NoSefer, "not open"))
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
            .ok_or_else(|| refuse(Code::NoSefer, "not open"))
    }

    /// A sefer and the lexicon to draw it with — the pair every `Line::of` needs
    /// (W19).
    ///
    /// One method rather than two calls because `sefer` takes `&mut self` to
    /// load on demand, and the borrow it hands back rules out reading
    /// `self.lexicon` afterwards. The alternatives were both worse: cloning a
    /// 27,667-variant lexicon per pane of text, or passing `None` at the call
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
            .ok_or_else(|| refuse(Code::NoSefer, "not open"))?;
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
            .ok_or_else(|| refuse(Code::NoSefer, "no mefarshim read"))
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

/// Put a work of your own into the search index, **with no state lock held**.
///
/// The two things it needs — the index handle and where the reader's layer is —
/// are taken under a lock held for two clones, and the tantivy commit itself
/// runs with the guard down. [`State::searchable`] used to be a method called
/// under the caller's existing guard by `note_write` and `note_edit`, which are
/// the whole of its callers: every saved keystroke-batch in a note stalled the
/// entire window for a commit, which is precisely what `make_searchable`'s
/// split exists to prevent and what `add_mine` already obeyed.
///
/// **Nothing here fails a write.** The note is on disk before this is called;
/// if the index will not take it the reader has still written their note, and
/// what they lose is that it is findable until the next build — which is
/// exactly the state everything was in before, and which the results header
/// already knows how to say.
fn searchable(shared: &tauri::State<'_, Shared>, slug: &str) {
    let (bar, personal) = {
        let Ok(state) = shared.lock() else {
            return;
        };
        let (Some(bar), Ok(shelf)) = (state.bar.clone(), state.shelf()) else {
            return;
        };
        (bar, shelf.personal().to_path_buf())
    };
    make_searchable(&bar, &personal, slug);
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

/// Have this sefer in memory **before** the caller takes the state lock.///
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
pub(crate) fn hold(shared: &tauri::State<'_, Shared>, slug: &str) -> Result<(), String> {
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
            state.pointed.remove(&gone);
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

/// A lane-settings write that failed, named for the window.
///
/// [`girsa_nearby::Adjacency::set`] and `::choose` save into the personal
/// layer, so their one failure is a refused write — which used to arrive as
/// the operating system's English sentence in a Hebrew window.
fn lane_said(e: std::io::Error) -> String {
    girsa_app::trouble::refuse(Code::ReadOnly, e.to_string())
}

/// A [`girsa_desk::BufferError`], likewise: a bad name is this codebase's own
/// refusal and carries a code; an operating-system error keeps its words, which
/// is how *permission denied* becomes a sentence about permission.
fn buffer_said(e: girsa_desk::BufferError) -> String {
    match e {
        girsa_desk::BufferError::BadName(name) => girsa_app::trouble::refuse(
            Code::NoSuch,
            format!("`{name}` is not a name for a document"),
        ),
        other => other.to_string(),
    }
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
    /// desk's `/document-saved`, which clears this.
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
            .ok_or_else(|| refuse(Code::NoSefer, "no words read"))
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

use commands::search::{marked, open_corpus, Opened};
use commands::shelf::{
    add_mine, companions, recent, search, shelf_make, shelf_pin, shelf_put_shelf, shelf_put_work,
    shelf_rename, shelf_reset, shelf_tree, shelf_works, titles,
};

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
    // The two facts the open needs, taken and released; the open itself runs
    // with **no state lock held**. It reads the catalogue, opens the index,
    // opens the notes, builds the lexicon and — when the lane is on — loads
    // the neural model, every one of which is thread-worthy work that used to
    // sit under the one lock every command in this file waits on. Switching
    // corpora froze the window for the whole pile.
    let (personal, showing) = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        (state.personal.clone(), state.session.showing)
    };
    let opened = open_corpus(Ok(at.clone()), &personal, showing);
    // A directory can hold the marker file and still refuse to open — an
    // unreadable catalogue, a half-written import. Remembering that one would
    // put the reader in front of it again on every launch, with the folder they
    // meant forgotten.
    let Some(shelf) = opened.shelf else {
        return Err(refuse(Code::NotACorpus, opened.trouble.unwrap_or(path)));
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.corpus = Some(at);
    state.shelf = Some(Arc::new(std::sync::RwLock::new(shelf)));
    state.trouble = opened.trouble;
    state.timeline = opened.timeline;
    // A different corpus, so every shard read out of the old one is a lie.
    state.chains = girsa_link::chain::Cache::default();
    state.bar = opened.bar;
    state.no_search = opened.no_search;
    state.lexicon = opened.lexicon;
    state.lane = opened
        .lane
        .map(|lane| Arc::new(std::sync::RwLock::new(lane)));
    state.open = girsa_app::held::Held::default();
    // **The chips too, and this was the gap.** Everything else about the old
    // corpus is cleared above with the comment that *a stale answer about a
    // sefer that still exists in the new corpus is worse than no answer: it
    // looks right* — and the scope inside the chip rows survived, holding work
    // slugs and shelf keys from the corpus that was just replaced. So the first
    // search after pointing the window at a new folder ran against a filter
    // naming seforim that are not there.
    state.chips = Chips::default();
    state.here_chips = Chips::default();
    state.marks.clear();
    state.pointed.clear();
    state.joined.clear();
    state.words.clear();
    state.documents = None;
    state.queue = None;
    state.save();
    Ok(())
}

/// A named arrangement, as the panel lists it.
#[derive(Serialize)]
pub(crate) struct DeskRow {
    name: String,
    /// How many tabs it holds, and how many distinct seforim across them.
    tabs: usize,
    seforim: usize,
    /// Whether this is the one the reader is sitting at.
    here: bool,
}

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
fn sefer_sheet(shared: tauri::State<'_, Shared>, at: String) -> Result<Sheet, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    // Held first, for the reason `copy` gives at length: this command is
    // deliberately synchronous, and a cache miss inside it is a whole work read
    // off disk on the thread that owns the message loop.
    hold(&shared, at.work())?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let (sefer, lexicon) = state.reading(at.work())?;
    // A sheet is the section, and there is no second answer to confuse it with:
    // the `whole:` flag used to name one here, but its only caller passed
    // `false` and its `true` printed *less* than `false` did.
    let (from, to) = girsa_app::printing::run_of(sefer, &at)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{at} is not in this sefer")))?;
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

/// A claim on the one long job the lane may be running.
///
/// Held for the life of the thread and released when it drops — however it
/// ends, including a panic. See [`State::working`].
struct Working(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for Working {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Claim the long job, or refuse because one is already running.
///
/// `swap` and not a read-then-write: two callers arriving together must not
/// both read `false`, which is the whole failure this replaces.
fn claim_the_job(shared: &tauri::State<'_, Shared>) -> Result<Working, String> {
    let flag = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        state.working.clone()
    };
    if flag.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::AlreadyWorking,
            "the lane is already working — stop it first, or wait for it",
        ));
    }
    Ok(Working(flag))
}

const BRING_EVENT: &str = "lane-bring";
const EMBED_EVENT: &str = "lane-embed";

fn lane_row(state: &State) -> Result<LaneRow, String> {
    let lane = state
        .lane
        .as_ref()
        .ok_or_else(|| state.trouble())?
        .read()
        .map_err(|_| State::poisoned())?;
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
    // **The lock is taken to read the handles and dropped before the ask.**
    // `Adjacency::ask` embeds the query with a neural model and searches every
    // vector store in the lane; it is the heaviest single operation in the
    // application, and it ran under the global guard for all of it. `ask` is
    // `&self` and the lane holds nothing of ours, so the only reason it was
    // ever under the guard is that the guard was around everything — which is
    // what `find` says above, in the same words, about a cheaper command.
    let (shelf, lane, timeline, scoped, language, cite) = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?.clone();
        let lane = state.lane.as_ref().ok_or_else(|| state.trouble())?.clone();
        (
            shelf,
            lane,
            state.timeline.clone(),
            // Scoped by the same chip the literal search is scoped by, so *the
            // whole shelf* and *this sefer* mean the same thing in both columns.
            state
                .chips
                .scope
                .works()
                .into_iter()
                .collect::<Vec<String>>(),
            state.session.language,
            state.session.cite,
        )
    };
    // Shelf, then lane. See the note on `State::lane`: the settings commands
    // take the pair in that order and one order is the whole of not deadlocking.
    let shelf = shelf.read().map_err(|_| State::poisoned())?;
    let lane = lane.read().map_err(|_| State::poisoned())?;
    // The same `Names` the search column uses, so the two columns beside each
    // other cannot call one sefer by two names.
    let names = girsa_app::Names::new(&shelf, timeline.as_ref(), language, cite);
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
        unresolved: answer.unresolved,
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
    // Shelf above, lane here: the one order. See `State::lane`.
    let mut lane = lane
        .as_ref()
        .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
        .write()
        .map_err(|_| State::poisoned())?;
    let was = lane.lane().settings().clone();
    let settings = girsa_lane::Settings {
        on,
        model: model.map(PathBuf::from).or(was.model),
        may_fetch: was.may_fetch,
    };
    let done = lane.set(settings, &shelf).map_err(lane_said);
    // Both guards, in the reverse of the order they were taken. `lane_row`
    // reads the state and would otherwise be borrowing it while these are alive.
    drop(lane);
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
    // Shelf above, lane here: the one order. See `State::lane`.
    let mut lane = lane
        .as_ref()
        .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
        .write()
        .map_err(|_| State::poisoned())?;
    let settings = girsa_lane::Settings {
        may_fetch: allow,
        ..lane.lane().settings().clone()
    };
    let done = lane.set(settings, &shelf).map_err(lane_said);
    // Both guards, in the reverse of the order they were taken. `lane_row`
    // reads the state and would otherwise be borrowing it while these are alive.
    drop(lane);
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
        let lane = state
            .lane
            .as_ref()
            .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
            .read()
            .map_err(|_| State::poisoned())?;
        (
            lane.lane().personal().to_path_buf(),
            lane.lane().settings().may_fetch,
        )
    };
    if !may_fetch {
        return Err(girsa_lane::BringError::NotAllowed.to_string());
    }
    // The same claim, and the same reason: two clicks used to start two
    // downloads of a multi-hundred-megabyte model into one directory.
    let working = claim_the_job(&shared)?;
    std::thread::spawn(move || {
        let _working = working;
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
                    // Shelf, then lane. The one order — see `State::lane`.
                    let shelf = shelf.as_ref().and_then(|s| s.read().ok());
                    let lane = lane.as_ref().and_then(|l| l.write().ok());
                    if let (Some(shelf), Some(mut lane)) = (shelf, lane) {
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
    // Shelf above, lane here: the one order. See `State::lane`.
    let mut lane = lane
        .as_ref()
        .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
        .write()
        .map_err(|_| State::poisoned())?;
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
    let done = lane.choose(chosen, &shelf).map_err(lane_said);
    drop(lane);
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
        let held = state
            .lane
            .as_ref()
            .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
            .read()
            .map_err(|_| State::poisoned())?;
        if !held.state().is_on() {
            return Err(girsa_app::trouble::refuse(
                Code::NoLane,
                "the semantic lane is off",
            ));
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
    // **One job at a time, claimed here.** See `State::working` for what a
    // second one did. The claim comes before `stop.store(false)` and not after:
    // clearing the stop flag is the thing a second run must never get to do.
    let working = claim_the_job(&shared)?;
    stop.store(false, std::sync::atomic::Ordering::Relaxed);

    std::thread::spawn(move || {
        // Released however this thread ends — the loop finishing, a `break` on
        // Stop, or a panic inside the model. A flag that is only cleared on the
        // happy path is a lane that can never be embedded again without a
        // restart, which is a worse failure than the one it prevents.
        let _working = working;
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
            // Shelf, then lane. The one order — see `State::lane`.
            let shelf = shelf.as_ref().and_then(|s| s.read().ok());
            let held = lane.as_ref().and_then(|l| l.write().ok());
            if let (Some(shelf), Some(mut held)) = (shelf, held) {
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
///
/// The seforim are read **before** the state lock is taken for the placement —
/// [`hold`], the same doctrine as `open_sefer`, which this command is the
/// tenth instance of needing: every scroll event lands here, a cache miss is a
/// tens-of-megabytes read, and holding the guard across them queued every other
/// command in the window behind a mouse wheel.
#[tauri::command(async)]
fn moved(shared: tauri::State<'_, Shared>, pane: PaneId, at: String) -> Result<Vec<Move>, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;

    // One: under the lock — record the move and work out who cares.
    let (leader_slug, root, wanted) = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state.session.remember(at.clone());
        let followers = state.session.workspace.moved(pane, at.clone());
        if followers.is_empty() {
            // `save_scroll`, not `save`: this is a scroll position, and
            // `Session::save` writes the whole workspace plus the remembered
            // place of every sefer ever opened.
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
        (leader_slug, root, wanted)
    };

    // Two: no state lock — the leader and every follower are read into memory,
    // each paying only a read lock on the shelf.
    hold(&shared, &leader_slug)?;
    for (_, slug) in &wanted {
        hold(&shared, slug)?;
    }

    // Three: under the lock again — every `sefer` below is now a cache hit,
    // and nothing here touches the disk.
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
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
    // WebKitGTK on NixOS/Wayland reports a negative devicePixelRatio, which
    // collapses the whole UI (ksav issue #103). Force X11 there — but never
    // over an explicit choice: `GDK_BACKEND` set by the reader or the session
    // is their decision, and `GDK_BACKEND=wayland` stays the escape hatch for
    // a machine where XWayland is absent or native Wayland renders correctly.
    if std::env::var_os("GDK_BACKEND").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        std::env::set_var("GDK_BACKEND", "x11");
    }
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
            // A session file that would not read is **kept aside and said out
            // loud** (#54). The sentence goes in `trouble` below, which is what
            // the top of the window shows; `opened.trouble` wins when the shelf
            // is missing too, because that is the fact the reader needs first.
            let (session, session_noted) = Session::load(&session_path);
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
                trouble: shelf_trouble,
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
                    // One slot for *what the window has to say before you ask
                    // it anything*, and the shelf's absence comes first when
                    // there are two things to say: a reader with no seforim
                    // needs that sentence, not the one about their tabs.
                    trouble: shelf_trouble.or(session_noted),
                    personal,
                    session,
                    session_path,
                    desk: None,
                    no_post: None,
                    lexicon,
                    queue: None,
                    lane: lane.map(|lane| Arc::new(std::sync::RwLock::new(lane))),
                    stop_embedding: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    working: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    open: girsa_app::held::Held::default(),
                    marks: HashMap::new(),
                    pointed: HashMap::new(),
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
            commands::nav::nav_ask,
            commands::nav::sheet_open,
            choose_corpus,
            search,
            recent,
            companions,
            commands::mefarshim::mefarshim,
            commands::mefarshim::choose_mefaresh,
            commands::mefarshim::choose_the_usual,
            commands::mefarshim::pair_alongside,
            commands::mefarshim::mefarshim_at,
            commands::mefarshim::open_sefer,
            commands::mefarshim::sefer_lines,
            commands::mefarshim::sefer_index_of,
            commands::mefarshim::sefer_indices_of,
            commands::panes::open_tab,
            commands::mefarshim::sefer_contents,
            commands::panes::open_set,
            commands::scans::scan,
            commands::scans::scan_at,
            commands::scans::scan_map,
            commands::scans::scan_reading,
            commands::scans::scan_read_page,
            commands::scans::scan_ocr_page,
            commands::scans::scan_words,
            commands::scans::scan_fix,
            commands::scans::scan_mark,
            commands::scans::scan_marks,
            commands::scans::scan_gap,
            lane_state,
            lane_ask,
            lane_set,
            lane_allow_fetch,
            lane_bring,
            lane_choose,
            lane_embed,
            lane_stop,
            commands::scans::scan_forget,
            commands::scans::scan_page_of,
            commands::scans::scan_copy,
            commands::panes::split,
            commands::panes::close_pane,
            commands::panes::move_pane,
            commands::panes::close_tab,
            commands::panes::focus,
            commands::panes::set_follows,
            commands::panes::set_ratio,
            commands::panes::turn_split,
            commands::panes::swap_split,
            commands::panes::move_tab,
            commands::panes::set_pointing,
            set_shemos,
            set_day_turns_at,
            luach,
            sefer_find,
            find_here_chip,
            sefer_sheet,
            link_words,
            commands::desks::desks,
            commands::desks::desk_keep,
            commands::desks::desk_open,
            commands::desks::desk_forget,
            commands::updates::check_for_update,
            commands::updates::open_releases,
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
            commands::search::find,
            commands::search::find_chip,
            commands::search::find_rung,
            commands::search::find_narrow,
            commands::search::find_whole_shelf,
            commands::search::find_scope,
            commands::search::find_scope_set,
            commands::search::find_scope_drop,
            commands::ksav::copy,
            commands::cite::set_cite_style,
            commands::cite::ksav_presence,
            commands::cite::send_to_ksav,
            commands::buffer::buffers,
            commands::buffer::buffer_open,
            commands::buffer::buffer_save,
            commands::buffer::buffer_rename,
            commands::buffer::buffer_write_to,
            commands::buffer::source_markup,
            commands::buffer::buffer_to_ksav,
            commands::cite::who_cites,
            commands::cite::linkify,
            commands::cite::cite_open,
            commands::corrections::fix,
            commands::corrections::unfix,
            commands::corrections::set_showing,
            commands::corrections::fixes,
            commands::chain::chain_walk,
            commands::chain::chain_forks,
            commands::queue::suspects,
            commands::queue::suspect_at,
            commands::queue::suspect_decide,
            commands::export::export_sefer,
            commands::links::links,
            commands::links::link_repair,
            commands::links::link_reanchor,
            commands::links::link_draw,
            commands::links::link_pin,
            commands::yours::yours,
            commands::yours::notes,
            commands::yours::note_write,
            commands::yours::note_read,
            commands::yours::note_edit,
            commands::yours::note_forget,
            commands::yours::mark_here,
            commands::yours::mark_forget,
            commands::yours::marks_in,
            commands::yours::bookmarks,
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
            .ok_or_else(|| {
                refuse(
                    Code::NoSuch,
                    format!("there is no saved query called {name}"),
                )
            })?
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

/// Forget a saved query — and its seat in any folder that held it.
#[tauri::command(async)]
fn query_forget(shared: tauri::State<'_, Shared>, name: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
    Ok(shelf
        .forget_query(&name)
        .map_err(|e| e.to_string())?
        .is_some())
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
    shelf.collections_mut().save(folder).map_err(|e| {
        // A write the personal layer refused, named: this used to forward
        // the crate's English Display to a Hebrew window.
        girsa_app::trouble::refuse(Code::ReadOnly, e.to_string())
    })?;
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
    // The state guard is taken for the handle and dropped; what is held across
    // the write is the shelf's own read lock, which is the one the export
    // genuinely needs. Writing the **entire personal layer** to disk — every
    // mark, every correction, every saved question — is bounded by the disk it
    // is going to, and none of that time belongs to the scroll handler.
    let shelf = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        state.shelf.as_ref().ok_or_else(|| state.trouble())?.clone()
    };
    let shelf = shelf.read().map_err(|_| State::poisoned())?;
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
