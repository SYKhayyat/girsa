//! The questions a reader saved, the folders they made, the tags they used, and
//! the export of their whole layer.
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use std::path::PathBuf;

use girsa_app::trouble::{refuse, Code};
use girsa_app::view::{FolderMember, FolderRow, QueryRow, TagRow};
use girsa_search::chips::Chips;

use crate::{Shared, State};

/// Keep the question you just asked.
///
/// The chips are saved as the `chip → key` pairs the row itself sends, so
/// recalling one goes through the same `Chips::choose` a click does. The scope is
/// saved as the seforim it comes to — a scope narrowed by three clicks comes
/// back as one clause over the same seforim, which matches the same segments
/// and no longer remembers the three clicks. Said here rather than discovered.
#[tauri::command(async)]
pub(crate) fn query_keep(
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
pub(crate) fn queries(shared: tauri::State<'_, Shared>) -> Result<Vec<QueryRow>, String> {
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
pub(crate) fn query_recall(
    shared: tauri::State<'_, Shared>,
    name: String,
) -> Result<String, String> {
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
pub(crate) fn query_forget(shared: tauri::State<'_, Shared>, name: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
    Ok(shelf
        .forget_query(&name)
        .map_err(|e| e.to_string())?
        .is_some())
}

/// Your chaburah folders.
#[tauri::command(async)]
pub(crate) fn folders(shared: tauri::State<'_, Shared>) -> Result<Vec<FolderRow>, String> {
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
pub(crate) fn folder_edit(
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
pub(crate) fn folder_forget(
    shared: tauri::State<'_, Shared>,
    name: String,
) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
    shelf
        .collections_mut()
        .remove(&name)
        .map_err(|e| e.to_string())
}

/// Every tag across your whole layer.
#[tauri::command(async)]
pub(crate) fn tags(shared: tauri::State<'_, Shared>) -> Result<Vec<TagRow>, String> {
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
pub(crate) fn export_layer(
    shared: tauri::State<'_, Shared>,
    into: Option<String>,
) -> Result<String, String> {
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
