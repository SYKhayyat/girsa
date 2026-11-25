//! What day it is, and what is being learned on it.
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use crate::{Shared, State};

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
pub(crate) fn luach(
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
pub(crate) fn set_shemos(shared: tauri::State<'_, Shared>, shemos: String) -> Result<(), String> {
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
pub(crate) fn set_day_turns_at(shared: tauri::State<'_, Shared>, hour: u8) -> Result<(), String> {
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
