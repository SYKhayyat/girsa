//! Panes and tabs — what is open, and how the window is arranged.
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use girsa_app::workspace::{Axis, PaneId};
use serde::Serialize;

use crate::{Shared, State};

#[tauri::command(async)]
pub(crate) fn open_tab(
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
pub(crate) struct OpenSefer {
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
pub(crate) fn open_set(shared: tauri::State<'_, Shared>) -> Result<Vec<OpenSefer>, String> {
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
pub(crate) fn split(
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
pub(crate) fn move_pane(
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
pub(crate) fn close_pane(shared: tauri::State<'_, Shared>, pane: PaneId) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.close(pane);
    state.save();
    Ok(())
}

/// Close a whole tab, from the tab strip, without opening it first (W40).
#[tauri::command(async)]
pub(crate) fn close_tab(shared: tauri::State<'_, Shared>, index: usize) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.close_tab(index);
    state.save();
    Ok(())
}

#[tauri::command(async)]
pub(crate) fn focus(shared: tauri::State<'_, Shared>, pane: PaneId) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.workspace.focus(pane);
    state.save();
    Ok(())
}

#[tauri::command(async)]
pub(crate) fn set_follows(
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
pub(crate) fn set_ratio(
    shared: tauri::State<'_, Shared>,
    split: usize,
    ratio: u16,
) -> Result<(), String> {
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
pub(crate) fn turn_split(shared: tauri::State<'_, Shared>, split: usize) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let turned = state.session.workspace.turn_split(split).is_some();
    state.save();
    Ok(turned)
}

/// Swap the two halves of the split this pane stands in.
#[tauri::command(async)]
pub(crate) fn swap_split(shared: tauri::State<'_, Shared>, split: usize) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let swapped = state.session.workspace.swap_split(split);
    state.save();
    Ok(swapped)
}

/// Move a tab along the strip — the drag half of *movable*.
#[tauri::command(async)]
pub(crate) fn move_tab(
    shared: tauri::State<'_, Shared>,
    from: usize,
    to: usize,
) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let moved = state.session.workspace.move_tab(from, to);
    state.save();
    Ok(moved)
}

#[tauri::command(async)]
pub(crate) fn set_pointing(
    shared: tauri::State<'_, Shared>,
    pointing: String,
) -> Result<(), String> {
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
