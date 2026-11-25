//! Every arrangement the reader has named.
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use crate::{DeskRow, Shared, State};

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
pub(crate) fn desks(shared: tauri::State<'_, Shared>) -> Result<Vec<DeskRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    Ok(desk_rows(&state))
}

/// Keep how the seforim are laid out right now, under a name.
///
/// Overwrites a desk of the same name rather than minting `sugya (2)`: a reader
/// typing a name they already used means *this one, as it is now*, and a second
/// desk with a number after it is a thing nobody asked for and has to clean up.
#[tauri::command(async)]
pub(crate) fn desk_keep(
    shared: tauri::State<'_, Shared>,
    name: String,
    over: bool,
) -> Result<Vec<DeskRow>, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            "a desk needs a name".to_string(),
        ));
    }
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // A desk of this name already exists, and keeping over it destroys the
    // arrangement it held — without a word, until now. `over` is the window
    // saying it asked. Same shape as `buffer_rename`, same reason: the DOM is
    // not the only caller.
    if !over && state.session.desks.contains_key(&name) {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::AlreadyThere,
            format!("you already have a desk called {name}"),
        ));
    }
    let now = state.session.workspace.clone();
    state.session.desks.insert(name.clone(), now);
    state.session.desk = Some(name);
    state.save();
    Ok(desk_rows(&state))
}

/// Sit down at one.
///
/// **The arrangement on screen is written back first** — when there is
/// somewhere to write it back to. A switcher that threw away what you had set
/// up in order to show you something else would be a switcher nobody uses
/// twice.
///
/// # The reader who has never named one
///
/// The sentence here used to go on: *"and the reader who is not sitting at a
/// named desk loses nothing either, because the session's own arrangement is
/// saved on every change and is what they come back to."* That is not what
/// happens. `session.workspace` **is** the arrangement on screen, and the line
/// below overwrites it — so a reader who had never named a desk lost every tab
/// and pane they had, with no prompt and no mention, the first time they tried
/// one out.
///
/// `discard` is the window saying it asked. Without it, an unnamed arrangement
/// is a refusal ([`Code::NotKept`]) rather than a loss, and the reader gets the
/// obvious offer: name this one first.
#[tauri::command(async)]
pub(crate) fn desk_open(
    shared: tauri::State<'_, Shared>,
    name: String,
    discard: bool,
) -> Result<Vec<DeskRow>, String> {
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
    } else if !discard && !state.session.workspace.tabs.is_empty() {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NotKept,
            "the arrangement on screen has no name, so there is nowhere to put it back",
        ));
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
pub(crate) fn desk_forget(
    shared: tauri::State<'_, Shared>,
    name: String,
) -> Result<Vec<DeskRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.desks.remove(&name);
    if state.session.desk.as_deref() == Some(name.as_str()) {
        state.session.desk = None;
    }
    state.save();
    Ok(desk_rows(&state))
}
