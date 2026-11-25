//! The window's own state: the shelf, the caches beside it, the session, and the
//! seven functions every command reaches for because of it.
//!
//! Moved out of `lib.rs` whole (#68). What stayed there is what `lib.rs` is for: the
//! handler list, the plugins, and `run`. **Everything here became `pub(crate)`** —
//! the fields, the methods, and the helpers — because a state nobody can read is a
//! state nobody can use, and a shell split into files still has one state.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use girsa_app::shelf::Open;
use girsa_app::trouble::{refuse, Code};
use girsa_app::{Session, Shelf};
use girsa_search::bar::Bar;
use girsa_search::chips::Chips;

use crate::Shared;

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
    pub(crate) bar: Option<Arc<Bar>>,
    /// Why there is no search, if there is none.
    pub(crate) no_search: Option<String>,
    /// The chip row as it stands (spec.md §9.5). Held here, not in the webview,
    /// so that what the chips say and what the engine does cannot drift.
    pub(crate) chips: Chips,
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
    pub(crate) here_chips: Chips,
    /// Why the shelf is not there, if it is not. Shown in the window.
    pub(crate) trouble: Option<String>,
    /// The reader's own layer, which is beside the session file and **not**
    /// under the corpus.
    ///
    /// Held rather than asked of the shelf, because the one moment it is needed
    /// most is the one moment there is no shelf to ask: a window that opened on
    /// nothing and is being pointed at a folder has to know where the notes go
    /// before it has anywhere to put them.
    pub(crate) personal: PathBuf,
    pub(crate) session: Session,
    pub(crate) session_path: PathBuf,
    /// The loopback desk (W16). Held because dropping it withdraws the endpoint
    /// file — which is exactly how presence stops being reported the moment
    /// this application stops.
    pub(crate) desk: Option<girsa_post::desk::Desk>,
    /// Why there is no pairing, if there is none. Shown beside the presence
    /// chip: a Ksav button that quietly does nothing is worse than one that
    /// says what is wrong.
    pub(crate) no_post: Option<String>,
    /// The resolver's lexicon, for linkify (W19). Read once: it is 27,667
    /// spellings and a citation is looked up per word of prose.
    pub(crate) lexicon: Option<girsa_ref::Lexicon>,
    /// The OCR queue, once it has been looked at (W21). Written by
    /// `girsa-suspects`, which is a batch job outside this window, so it is
    /// re-read whenever the drawer is opened rather than held as truth.
    pub(crate) queue: Option<girsa_fix::suspect::Queue>,
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
    pub(crate) lane: Option<Arc<std::sync::RwLock<girsa_nearby::Adjacency>>>,
    /// Set to stop the embedding job. It is checked between batches, so
    /// stopping costs the batch in flight and nothing else.
    pub(crate) stop_embedding: std::sync::Arc<std::sync::atomic::AtomicBool>,
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
    pub(crate) working: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Slug → the sefer, read once. Cleared oldest-first.
    /// The seforim in memory, most recently read last (`girsa_app::held`).
    pub(crate) open: girsa_app::held::Held<Arc<Open>>,
    /// Slug → which mefarshim speak on which of its lines (W43).
    ///
    /// Held because ticking a box asks again, and a reader ticking six of them
    /// should not pay six reads of a 3.4 MB file for an answer that cannot have
    /// changed. One `Marks` is the whole sefer's answer; the read that builds it
    /// is 0.07s for Berakhot.
    pub(crate) marks: HashMap<String, girsa_app::mefarshim::Marks>,
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
    pub(crate) pointed: HashMap<String, bool>,
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
    pub(crate) joined: HashMap<(String, String), girsa_app::Joined>,
    /// Slug → what has been read off the pages of that scan (W26).
    ///
    /// Held for the same reason `marks` is. `Words::open` parses the whole
    /// append-only log to answer about one page, and the shell called it
    /// **twice per page read** — `scan_read_page` opened it to record, then
    /// called `scan_reading`, which opened it again to count. Six call sites,
    /// six parses.
    pub(crate) words: HashMap<String, girsa_scan::Words>,
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
    pub(crate) saved_at: std::time::Instant,
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
    pub(crate) fn shelf_handle(&self) -> Result<Arc<std::sync::RwLock<Shelf>>, String> {
        self.shelf
            .as_ref()
            .map(Arc::clone)
            .ok_or_else(|| self.trouble())
    }

    /// Read this sefer if it is not already held. Everything below is one of
    /// three ways of asking for the result.
    pub(crate) fn load(&mut self, slug: &str) -> Result<(), String> {
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
    pub(crate) fn marks(&mut self, slug: &str) -> Result<&girsa_app::mefarshim::Marks, String> {
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
    pub(crate) fn reread(&mut self, slug: &str) {
        self.open.forget(slug);
        // The join itself cannot have changed — a correction does not move a
        // segment — but the `Open`s it was computed against are gone, and an
        // answer held about a sefer nobody is holding is a cache outliving its
        // question.
        self.joined
            .retain(|(leader, follower), _| leader != slug && follower != slug);
    }

    /// Forget all of them, which is what *show as printed* costs.
    pub(crate) fn reread_everything(&mut self) {
        self.open.clear();
        self.joined.clear();
    }

    /// Why there is no shelf, named so the window need not read the prose.
    ///
    /// `girsa_app::trouble::refuse` puts a code on the front — `no-shelf: …` —
    /// because `trouble.ts` used to turn this sentence into Hebrew by matching
    /// `/no shelf at/i` against it, which made the wording load-bearing API with
    /// no test on this side of the wire.
    pub(crate) fn trouble(&self) -> String {
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
    pub(crate) fn said_trouble(&self) -> Option<String> {
        let prose = self.trouble.as_ref()?;
        Some(if self.shelf.is_none() {
            refuse(Code::NoShelf, prose)
        } else {
            prose.clone()
        })
    }

    pub(crate) fn no_search(&self) -> String {
        refuse(
            Code::NoIndex,
            self.no_search
                .as_deref()
                .unwrap_or("there is no index here"),
        )
    }

    /// The one refusal that is not about the shelf: something panicked while
    /// holding the state, so the lock is poisoned. Ninety-six commands say it.
    pub(crate) fn poisoned() -> String {
        refuse(Code::Poisoned, "state is poisoned")
    }

    /// And take one out, because it is not on the shelf any more.
    ///
    /// The asymmetry is real: a work that has been deleted has no
    /// `segments.jsonl` to read back, so the delete-then-add rule never fires
    /// for it. Left alone, a note you threw away stays findable until the next
    /// full build and a hit on it opens a sefer that is not there.
    pub(crate) fn unsearchable(&mut self, slug: &str) {
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
pub(crate) fn searchable(shared: &tauri::State<'_, Shared>, slug: &str) {
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
pub(crate) fn personal_of(shared: &tauri::State<'_, Shared>) -> Result<PathBuf, String> {
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
pub(crate) fn hold_marks(shared: &tauri::State<'_, Shared>, slug: &str) -> Result<(), String> {
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
pub(crate) fn lane_said(e: std::io::Error) -> String {
    girsa_app::trouble::refuse(Code::ReadOnly, e.to_string())
}

/// A [`girsa_desk::BufferError`], likewise: a bad name is this codebase's own
/// refusal and carries a code; an operating-system error keeps its words, which
/// is how *permission denied* becomes a sentence about permission.
pub(crate) fn buffer_said(e: girsa_desk::BufferError) -> String {
    match e {
        girsa_desk::BufferError::BadName(name) => girsa_app::trouble::refuse(
            Code::NoSuch,
            format!("`{name}` is not a name for a document"),
        ),
        other => other.to_string(),
    }
}

pub(crate) fn make_searchable(bar: &Bar, personal: &std::path::Path, slug: &str) {
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
    pub(crate) fn documents(
        &mut self,
        personal: &std::path::Path,
    ) -> &girsa_desk::documents::Documents {
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
    pub(crate) fn names<'a>(&'a self, shelf: &'a Shelf) -> girsa_app::Names<'a> {
        girsa_app::Names::new(
            shelf,
            self.timeline.as_ref(),
            self.session.language,
            // The reader's own citation style, so a row label and the citation
            // they copy off the same line agree.
            self.session.cite,
        )
    }

    pub(crate) fn save(&mut self) {
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
    pub(crate) fn save_scroll(&mut self) {
        if self.saved_at.elapsed() >= SAVE_SCROLL_EVERY {
            self.save();
        }
    }

    /// What has been read off a scan's pages, parsed once per session.
    ///
    /// # Errors
    ///
    /// If there is no shelf to find the personal layer under.
    pub(crate) fn words(&mut self, slug: &str) -> Result<&mut girsa_scan::Words, String> {
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
    pub(crate) fn join(&mut self, leader: &str, follower: &str, root: &std::path::Path) {
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
