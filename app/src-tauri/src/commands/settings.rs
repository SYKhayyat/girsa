//! The settings panel — the pointing, the keys, the language and the sizes.
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use girsa_app::view::{SettingsView, Shortcut};

use crate::{Shared, State};

#[tauri::command(async)]
pub(crate) fn settings(shared: tauri::State<'_, Shared>) -> Result<SettingsView, String> {
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
pub(crate) fn set_look(
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
pub(crate) fn bind_key(
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
pub(crate) fn what_key(
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
pub(crate) fn set_language(
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
pub(crate) fn set_interface(
    shared: tauri::State<'_, Shared>,
    language: girsa_app::session::Language,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.interface = language;
    state.save();
    Ok(())
}

#[tauri::command(async)]
pub(crate) fn set_text_size(shared: tauri::State<'_, Shared>, percent: u16) -> Result<(), String> {
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
pub(crate) fn set_mefarshim_size(
    shared: tauri::State<'_, Shared>,
    percent: u16,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.mefarshim_size = percent;
    state.session.sane();
    state.save();
    Ok(())
}
