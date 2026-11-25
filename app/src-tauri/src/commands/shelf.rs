//! The shelf, and the cards that stand for what is on it.
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use std::path::PathBuf;

use crate::{make_searchable, Shared, State};
use girsa_app::shelf::Companion;
use girsa_app::taxonomy::Branch;
use girsa_app::view::Refusal;
use girsa_app::view::{Card, Dropped};

#[tauri::command(async)]
pub(crate) fn search(shared: tauri::State<'_, Shared>, query: String) -> Result<Vec<Card>, String> {
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
pub(crate) fn recent(shared: tauri::State<'_, Shared>) -> Result<Vec<Card>, String> {
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

/// The shelf, as a tree — the shipped taxonomy with your arrangement on top.
///
/// Counts only; the seforim themselves come one shelf at a time from
/// [`shelf_works`]. 7,189 cards is not a browse, it is a dump.

#[tauri::command(async)]
pub(crate) fn shelf_tree(shared: tauri::State<'_, Shared>) -> Result<Vec<Branch>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    Ok(shelf.tree())
}

#[tauri::command(async)]
pub(crate) fn shelf_works(
    shared: tauri::State<'_, Shared>,
    key: String,
) -> Result<Vec<Card>, String> {
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
pub(crate) fn titles(
    shared: tauri::State<'_, Shared>,
    slugs: Vec<String>,
) -> Result<Vec<Card>, String> {
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
pub(crate) fn shelf_put_work(
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
pub(crate) fn shelf_put_shelf(
    shared: tauri::State<'_, Shared>,
    key: String,
    parent: String,
) -> Result<(), String> {
    edit_shelf(&shared, move |a| a.put_shelf(&key, &parent))
}

#[tauri::command(async)]
pub(crate) fn shelf_rename(
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
pub(crate) fn shelf_pin(
    shared: tauri::State<'_, Shared>,
    parent: String,
    key: String,
) -> Result<(), String> {
    edit_shelf(&shared, move |a| {
        let mut order = a.order.get(&parent).cloned().unwrap_or_default();
        order.retain(|k| *k != key);
        order.insert(0, key);
        a.reorder(&parent, order);
        Ok(())
    })
}

#[tauri::command(async)]
pub(crate) fn shelf_make(
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
pub(crate) fn shelf_reset(shared: tauri::State<'_, Shared>) -> Result<(), String> {
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
pub(crate) fn add_mine(
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
pub(crate) fn companions(
    shared: tauri::State<'_, Shared>,
    slug: String,
) -> Result<Vec<Companion>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    Ok(shelf.companions(&slug))
}
