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
mod state;

pub(crate) use state::{
    buffer_said, hold, hold_marks, lane_said, make_searchable, personal_of, searchable, State,
};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use girsa_app::trouble::{refuse, Code};
use girsa_app::{Session, Workspace};
use girsa_search::chips::Chips;
use serde::Serialize;

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
