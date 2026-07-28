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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use girsa_app::shelf::{Companion, Open};
use girsa_app::workspace::{Axis, PaneId};
use girsa_app::{display, Beside, Place, Session, Shelf, Workspace};
use girsa_corpus::segment::SegmentId;
use serde::Serialize;

/// How many seforim are kept in memory at once.
///
/// A masechta with its commentaries is four or five; the number is small
/// because a work is tens of megabytes of text and a reader has a handful open,
/// not a library.
const KEEP_OPEN: usize = 12;

struct State {
    shelf: Option<Shelf>,
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
}

impl Card {
    fn of(work: &girsa_corpus::work::Work) -> Self {
        Self {
            slug: work.slug.clone(),
            he_title: work.he_title.clone(),
            en_title: work.en_title.clone(),
            categories: work.categories.clone(),
            author: work.author.clone(),
            era: work.era.clone(),
        }
    }
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
                kind: match s.kind {
                    girsa_corpus::import::SegmentKind::Heading => "heading",
                    girsa_corpus::import::SegmentKind::Text => "text",
                },
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

/// # Panics
///
/// If the window cannot be created at all, which is not a condition the app can
/// carry on from.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (shelf, trouble) = match find_corpus() {
        Ok(root) => match Shelf::open(&root) {
            Ok(shelf) => (Some(shelf), None),
            Err(e) => (None, Some(e.to_string())),
        },
        Err(e) => (None, Some(e)),
    };

    tauri::Builder::default()
        .setup(move |app| {
            let session_path = tauri::Manager::path(app)
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("session.json");
            let session = Session::load(&session_path);
            tauri::Manager::manage(
                app,
                Mutex::new(State {
                    shelf,
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
