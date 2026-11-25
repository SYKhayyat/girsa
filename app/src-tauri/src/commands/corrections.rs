//! Corrections (spec.md §7, BUILDER.md W20).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use girsa_app::trouble::{refuse, Code};
use girsa_app::view::{Fixed, Line, PatchRow};
use girsa_corpus::segment::SegmentId;

use crate::{Shared, State};

/// Correct a typo from where you are reading (spec.md §7.5).
///
/// The offsets are the ones the pane reports for a highlight — the same call
/// Ctrl+C makes — so there is nothing for the reader to look up and nothing to
/// navigate to. What comes back is the one line, redrawn, so the window does
/// not rebuild the sefer around them while they are reading it.
#[tauri::command(async)]
pub(crate) fn fix(
    shared: tauri::State<'_, Shared>,
    at: String,
    from_char: usize,
    to_char: usize,
    now: String,
    kind: String,
    note: Option<String>,
) -> Result<Fixed, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let kind = girsa_fix::Kind::named(&kind)
        .ok_or_else(|| refuse(Code::NoSuch, format!("no such kind: {kind}")))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    let slug = at.work().to_string();

    let patch = {
        let sefer = state.sefer(&slug)?;
        let mut patch = girsa_app::correction(
            sefer,
            &at,
            from_char..to_char,
            &now,
            kind,
            &girsa_app::who(),
            pointing,
        )
        .map_err(|e| e.to_string())?;
        if let Some(note) = note.filter(|n| !n.trim().is_empty()) {
            patch = patch.with_note(note);
        }
        patch
    };
    let was = patch.was.clone();

    state.shelf_mut()?.fix(patch).map_err(|e| e.to_string())?;
    state.reread(&slug);

    let (sefer, lexicon) = state.reading(&slug)?;
    let position = sefer
        .position_of(&at)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{at} is not in this sefer")))?;
    let segment = sefer
        .segments
        .get(position)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{at} is not in this sefer")))?;
    Ok(Fixed {
        line: Line::of(sefer, segment, pointing, shemos, style, lexicon),
        said: Some(format!("{was} → {now}")),
    })
}

/// Take a correction back.
#[tauri::command(async)]
pub(crate) fn unfix(
    shared: tauri::State<'_, Shared>,
    at: String,
    patch: String,
) -> Result<Fixed, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    let slug = at.work().to_string();

    let gone = state
        .shelf_mut()?
        .unfix(&girsa_fix::PatchId::from(patch))
        .map_err(|e| e.to_string())?;
    if !gone {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            "there is no such correction",
        ));
    }
    state.reread(&slug);

    let (sefer, lexicon) = state.reading(&slug)?;
    let position = sefer
        .position_of(&at)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{at} is not in this sefer")))?;
    let segment = sefer
        .segments
        .get(position)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{at} is not in this sefer")))?;
    Ok(Fixed {
        line: Line::of(sefer, segment, pointing, shemos, style, lexicon),
        // Nothing to name: this took a correction back, and the window says so.
        said: None,
    })
}

/// *Show as printed / show corrected* (spec.md §7.1).
///
/// Three states rather than two, because a scanning error and an emendation are
/// different claims — see [`girsa_fix::Showing`]. Everything open is re-read,
/// which the window does by drawing again.
#[tauri::command(async)]
pub(crate) fn set_showing(shared: tauri::State<'_, Shared>, showing: String) -> Result<(), String> {
    let showing = girsa_fix::Showing::named(&showing)
        .ok_or_else(|| refuse(Code::NoSuch, format!("no such setting: {showing}")))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.showing = showing;
    state.shelf_mut()?.set_showing(showing);
    state.reread_everything();
    state.save();
    Ok(())
}

#[tauri::command(async)]
pub(crate) fn fixes(
    shared: tauri::State<'_, Shared>,
    slug: Option<String>,
) -> Result<Vec<PatchRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    // A patch row is a row about a place, like a hit and like a lane result —
    // and it was the fifth to work out a title and an address for itself.
    let shelf = state.shelf()?;
    let names = state.names(&shelf);
    let mut rows: Vec<PatchRow> = shelf
        .fixes()
        .all()
        .filter(|p| slug.as_ref().is_none_or(|s| p.segment.work() == s))
        .map(|p| PatchRow {
            id: p.id.to_string(),
            segment: p.segment.to_string(),
            work: names.of(&p.segment).work,
            title: names.of(&p.segment).title,
            address: p.segment.address(),
            kind: p.kind.as_str(),
            was: p.was.clone(),
            now: p.now.clone(),
            who: p.who.clone(),
            when: p.when,
            note: p.note.clone(),
            source: p.source.clone(),
        })
        .collect();
    girsa_app::view::PatchRow::newest_first(&mut rows);
    Ok(rows)
}
