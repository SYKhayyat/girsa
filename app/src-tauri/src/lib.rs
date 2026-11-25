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
use girsa_app::{Session, Shelf, Workspace};
use girsa_search::bar::Bar;
use girsa_search::chips::Chips;
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

use commands::search::{open_corpus, Opened};
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
            commands::lane::lane_state,
            commands::lane::lane_ask,
            commands::lane::lane_set,
            commands::lane::lane_allow_fetch,
            commands::lane::lane_bring,
            commands::lane::lane_choose,
            commands::lane::lane_embed,
            commands::lane::lane_stop,
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
            commands::luach::set_shemos,
            commands::luach::set_day_turns_at,
            commands::luach::luach,
            commands::reading::sefer_find,
            commands::reading::find_here_chip,
            commands::reading::sefer_sheet,
            commands::reading::link_words,
            commands::desks::desks,
            commands::desks::desk_keep,
            commands::desks::desk_open,
            commands::desks::desk_forget,
            commands::updates::check_for_update,
            commands::updates::open_releases,
            commands::settings::set_language,
            commands::settings::set_interface,
            commands::settings::settings,
            commands::settings::set_look,
            commands::settings::bind_key,
            commands::settings::what_key,
            commands::settings::set_text_size,
            commands::settings::set_mefarshim_size,
            commands::following::moved,
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
            commands::questions::query_keep,
            commands::questions::queries,
            commands::questions::query_recall,
            commands::questions::query_forget,
            commands::questions::folders,
            commands::questions::folder_edit,
            commands::questions::folder_forget,
            commands::questions::tags,
            commands::questions::export_layer,
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

/// Kept honest: the workspace type the window draws is the one the tests are
/// written against, not a second copy of it living in TypeScript.
#[allow(dead_code)]
fn _assert_workspace_is_the_tested_one(w: &Workspace) -> usize {
    w.tabs.len()
}
