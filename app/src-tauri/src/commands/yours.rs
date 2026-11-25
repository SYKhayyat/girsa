//! Your own layer: notes, highlights, bookmarks, tags and folders (spec.md §11).
//!
//! Moved out of `lib.rs` whole (#68), which is why the section comment that used
//! to introduce it reads as the file's own doc. The two sentences that matter are
//! still the ones it carried:
//!
//! **What is not here is the interesting part.** There is no `notes_on` command
//! returning your notes about a line, because your notes about a line come back
//! from `links` — a note's edge is a `girsa_link::Edge` and the panel that draws
//! what the library says about a sugya draws what you said about it in the same
//! list, sorted by the same rule. A second endpoint for it is what the whole crate
//! exists to avoid.
//!
//! What is left is the two things that are not edges — marks and folders — and the
//! writing side.

use std::collections::HashMap;

use girsa_app::display;
use girsa_app::trouble::{refuse, Code};
use girsa_app::view::{MarkRow, NoteRow, ParaRow, Yours};
use girsa_corpus::segment::SegmentId;

use crate::{hold, searchable, Shared, State};

/// What you have on the line you are standing on.
#[tauri::command(async)]
pub(crate) fn yours(shared: tauri::State<'_, Shared>, at: String) -> Result<Yours, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;

    // The line as the pane drew it — corrected, and with the nikud the reader
    // has on — because that is the string a highlight's offsets are against.
    let (base, standing) = {
        let sefer = state.sefer(at.work())?;
        let text = sefer
            .position_of(&at)
            .and_then(|nth| sefer.segments.get(nth))
            .map(|segment| segment.text.clone())
            .unwrap_or_default();
        // Your notes, highlights and folders are anchored under the name this
        // place had when you wrote them, which a corpus update may have moved.
        (text, sefer.standing(&at))
    };
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let drawn = display::Shown::of(&girsa_app::shemos::written(&base, shemos), pointing)
        .text()
        .to_string();

    let shelf = state.shelf()?;
    let found = girsa_app::yours(&shelf, &standing, &drawn);
    Ok(Yours {
        notes: found
            .notes
            .iter()
            .filter_map(|wrote| shelf.notes().get(&wrote.slug))
            .map(NoteRow::of)
            .collect(),
        marks: found.marks.iter().map(MarkRow::of).collect(),
        folders: found.folders,
    })
}

/// Every note you have, most recently touched first.
#[tauri::command(async)]
pub(crate) fn notes(shared: tauri::State<'_, Shared>) -> Result<Vec<NoteRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    let mut rows: Vec<NoteRow> = shelf.notes().all().map(NoteRow::of).collect();
    girsa_app::view::NoteRow::newest_first(&mut rows);
    Ok(rows)
}

/// Write a note about where you are standing. The three-second one.
#[tauri::command(async)]
pub(crate) fn note_write(
    shared: tauri::State<'_, Shared>,
    at: String,
    title: Option<String>,
    text: String,
) -> Result<NoteRow, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let note = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let who = girsa_app::who();
        let mut shelf = state.shelf_mut()?;
        girsa_app::note_here(&mut shelf, &at, title.as_deref(), &text, &who)
            .map_err(|e| e.to_string())?
    };
    // The guard is down; the commit below no longer holds the window's turn.
    searchable(&shared, &note.slug);
    Ok(NoteRow::of(&note))
}

/// One note, paragraph by paragraph, for editing it.
#[tauri::command(async)]
pub(crate) fn note_read(
    shared: tauri::State<'_, Shared>,
    note: String,
) -> Result<Vec<ParaRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    let held = shelf
        .notes()
        .get(&note)
        .ok_or_else(|| refuse(Code::NoSuch, format!("there is no note called {note}")))?;
    Ok(held
        .paras()
        .iter()
        .map(|para| ParaRow {
            id: para.id.to_string(),
            text: para.text.clone(),
        })
        .collect())
}

/// Change a note: a paragraph's words, another paragraph, one taken out, a tag,
/// an anchor.
///
/// One command, because they are one thing — an edit to a note in your own
/// layer — and which edit is **named rather than free-form**, the same rule as
/// `link_repair`.
///
/// Every one of these writes the whole note back, and none of them renumbers a
/// paragraph: a paragraph put in the middle mints a child ordinal (spec.md §3),
/// so an id the window is holding is still the id it was holding.
#[tauri::command(async)]
pub(crate) fn note_edit(
    shared: tauri::State<'_, Shared>,
    note: String,
    does: String,
    value: Option<String>,
    text: Option<String>,
) -> Result<Vec<ParaRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // Read out, edited in hand, written back. The shelf is not held open
    // across the edit: nothing between the two locks touches it.
    let mut held = {
        let shelf = state.shelf()?;
        shelf
            .notes()
            .get(&note)
            .cloned()
            .ok_or_else(|| refuse(Code::NoSuch, format!("there is no note called {note}")))?
    };

    let words = || text.clone().unwrap_or_default();
    let paragraph = |value: &Option<String>| -> Result<SegmentId, String> {
        value
            .as_deref()
            .ok_or_else(|| refuse(Code::NoSuch, "which paragraph?"))?
            .parse()
            .map_err(|e| format!("{e}"))
    };
    match does.as_str() {
        "append" => {
            held.append(words());
        }
        "after" => {
            held.insert_after(&paragraph(&value)?, words())
                .map_err(|e| e.to_string())?;
        }
        "set" => {
            if !held.set(&paragraph(&value)?, words()) {
                return Err(girsa_app::trouble::refuse(
                    girsa_app::trouble::Code::NoSuch,
                    "that paragraph is not in this note",
                ));
            }
        }
        "remove" => {
            if !held.remove(&paragraph(&value)?) {
                return Err(girsa_app::trouble::refuse(
                    girsa_app::trouble::Code::NoSuch,
                    "that paragraph is not in this note",
                ));
            }
        }
        "title" => held.title = words(),
        "tag" => {
            held.tag(&words());
        }
        "untag" => {
            held.untag(&words());
        }
        "anchor" => {
            held.anchor(paragraph(&value)?);
        }
        "unanchor" => {
            held.unanchor(&paragraph(&value)?);
        }
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such edit: {other}"),
            ))
        }
    }
    let written = {
        let mut shelf = state.shelf_mut()?;
        shelf.write_note(held).map_err(|e| e.to_string())?
    };
    let rows: Vec<ParaRow> = written
        .paras()
        .iter()
        .map(|para| ParaRow {
            id: para.id.to_string(),
            text: para.text.clone(),
        })
        .collect();
    // An edit is a rewrite of the note, so the index gets the whole note back.
    // A work is the unit of replacement, which makes *changed* and *new* the
    // same operation here. And the guard for the write above is already down:
    // this used to commit the index under the state lock.
    drop(state);
    searchable(&shared, &written.slug);
    Ok(rows)
}

/// Throw a note away — the file, the sefer, the catalogue line, and its seat
/// in any folder that held it. `false` when there was no such note.
#[tauri::command(async)]
pub(crate) fn note_forget(shared: tauri::State<'_, Shared>, note: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let (slug, gone) = {
        let mut shelf = state.shelf_mut()?;
        // A note is a sefer, and a sefer that has gone may not stay open in a
        // pane holding text nothing on the shelf accounts for.
        let slug = shelf.notes().get(&note).map(|held| held.slug.clone());
        let gone = shelf.forget_note(&note).map_err(|e| e.to_string())?;
        (slug, gone.is_some())
    };
    if let Some(slug) = slug {
        state.open.forget(&slug);
        state.unsearchable(&slug);
    }
    Ok(gone)
}

/// Highlight some words, or mark the place.
///
/// The words are read out of the line as the pane drew it and stored with the
/// mark, because an offset is not a place (`girsa_corpus::span`).
#[tauri::command(async)]
pub(crate) fn mark_here(
    shared: tauri::State<'_, Shared>,
    at: String,
    from_char: Option<usize>,
    to_char: Option<usize>,
    label: Option<String>,
    colour: Option<String>,
) -> Result<MarkRow, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let base = {
        let sefer = state.sefer(at.work())?;
        sefer
            .position_of(&at)
            .and_then(|nth| sefer.segments.get(nth))
            .map(|segment| segment.text.clone())
            .unwrap_or_default()
    };
    let drawn = display::Shown::of(&girsa_app::shemos::written(&base, shemos), pointing)
        .text()
        .to_string();

    let who = girsa_app::who();
    let mut made = match (from_char, to_char) {
        (Some(from), Some(to)) if from < to => {
            let letters: Vec<char> = drawn.chars().collect();
            let was: String = letters
                .get(from..to)
                .ok_or_else(|| refuse(Code::NoSuch, "those characters are not in the line"))?
                .iter()
                .collect();
            girsa_note::Mark::highlight(at, from..to, was, &who)
        }
        _ => girsa_note::Mark::bookmark(at, &who),
    };
    if let Some(label) = label.filter(|l| !l.trim().is_empty()) {
        made = made.called(label);
    }
    if let Some(colour) = colour.filter(|c| !c.trim().is_empty()) {
        made = made.coloured(colour);
    }

    let mut shelf = state.shelf_mut()?;
    let held = shelf
        .marks_mut()
        .add(made)
        .map_err(|e| e.to_string())?
        .clone();
    let placed = held.place(&drawn);
    Ok(MarkRow::of(&girsa_app::Marked { mark: held, placed }))
}

/// Take a mark back.
#[tauri::command(async)]
pub(crate) fn mark_forget(shared: tauri::State<'_, Shared>, mark: String) -> Result<bool, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let mut shelf = state.shelf_mut()?;
    shelf
        .marks_mut()
        .remove(&girsa_note::MarkId::from(mark))
        .map_err(|e| e.to_string())
}

/// Every mark in one sefer, placed against the lines as the pane draws them.
///
/// One call for a whole sefer rather than one per line: a highlight has to be
/// **painted where it is**, and asking line by line would make that a hundred
/// round trips a page. Where each one lands is still decided here — the window
/// is handed offsets into the text it was already sent, and works nothing out.
///
/// # The marks are asked for first, and only what carries one is rendered
///
/// This used to render the **whole sefer** before it asked the layer anything:
/// a `HashMap` of every segment, each one through `shemos::written` and
/// `display::Shown`, tens of megabytes of string work for Berakhot — and only
/// then iterate `marks().in_work(&slug)`, which is usually empty. It is called
/// on every pane open and again after every change to the layer, so a reader
/// with no highlights paid for all of it, repeatedly, to be handed nothing:
/// measured at 15 ms for Berakhot and **70 ms for Shulchan Arukh Orach
/// Chayim**, both times to return an empty list.
///
/// The order is the fix. Ask which places carry a mark, and render those. A
/// reader has a handful of highlights in a sefer, not one per line, so the work
/// is now proportional to the answer rather than to the sefer.
#[tauri::command(async)]
pub(crate) fn marks_in(
    shared: tauri::State<'_, Shared>,
    slug: String,
) -> Result<Vec<MarkRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // The layer first. Cloned out so the shelf guard is gone before `sefer`
    // needs `&mut state` — and so the common answer, *none*, costs a lookup.
    let marks: Vec<girsa_note::Mark> = {
        let shelf = state.shelf()?;
        shelf.marks().in_work(&slug).cloned().collect()
    };
    if marks.is_empty() {
        return Ok(Vec::new());
    }
    // Only now is the sefer worth having, and only the places a mark names are
    // worth drawing.
    let wanted: std::collections::HashSet<String> =
        marks.iter().map(|mark| mark.at.to_string()).collect();
    drop(state);
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let drawn: HashMap<String, String> = {
        let sefer = state.sefer(&slug)?;
        sefer
            .segments
            .iter()
            .filter(|segment| wanted.contains(&segment.id.to_string()))
            .map(|segment| {
                (
                    segment.id.to_string(),
                    display::Shown::of(
                        &girsa_app::shemos::written(&segment.text, shemos),
                        pointing,
                    )
                    .text()
                    .to_string(),
                )
            })
            .collect()
    };
    Ok(marks
        .iter()
        .map(|mark| {
            let text = drawn.get(&mark.at.to_string()).map_or("", String::as_str);
            MarkRow::of(&girsa_app::Marked {
                mark: mark.clone(),
                placed: mark.place(text),
            })
        })
        .collect())
}

/// Every bookmark, most recent first — the *take me back* list.
#[tauri::command(async)]
pub(crate) fn bookmarks(shared: tauri::State<'_, Shared>) -> Result<Vec<MarkRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    Ok(shelf
        .marks()
        .bookmarks()
        .into_iter()
        .map(|mark| {
            MarkRow::of(&girsa_app::Marked {
                mark: mark.clone(),
                placed: girsa_note::mark::Placed::Whole,
            })
        })
        .collect())
}
