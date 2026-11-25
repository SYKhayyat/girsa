//! The buffer (spec.md §10.3, BUILDER.md W17).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use std::path::PathBuf;

use girsa_app::view::Writing;
use girsa_corpus::segment::SegmentId;

use crate::{buffer_said, personal_of, Shared, State};

#[tauri::command(async)]
pub(crate) fn buffers(shared: tauri::State<'_, Shared>) -> Result<Vec<String>, String> {
    let personal = personal_of(&shared)?;
    Ok(girsa_desk::Buffer::list(&personal))
}

#[tauri::command(async)]
pub(crate) fn buffer_open(
    shared: tauri::State<'_, Shared>,
    name: String,
) -> Result<Writing, String> {
    let personal = personal_of(&shared)?;
    let buffer = girsa_desk::Buffer::open(&personal, &name).map_err(buffer_said)?;
    let path = girsa_desk::Buffer::path(&personal, &name).map_err(buffer_said)?;
    Ok(Writing {
        name: buffer.name,
        text: buffer.text,
        path: path.display().to_string(),
    })
}

#[tauri::command(async)]
pub(crate) fn buffer_save(
    shared: tauri::State<'_, Shared>,
    name: String,
    text: String,
) -> Result<String, String> {
    let personal = personal_of(&shared)?;
    let mut buffer = girsa_desk::Buffer::new(name);
    buffer.text = text;
    Ok(buffer
        .save(&personal)
        .map_err(buffer_said)?
        .display()
        .to_string())
}

/// Save a document under a new name, refusing to land on one already in use.
///
/// # Why this is not `buffer_save` with a different name
///
/// It was, and that is the defect. `Buffer::save` truncates whatever is at the
/// name it is handed, which is right for a save — saving over the document you
/// are looking at is what saving is — and wrong for a **rename**, where the
/// name is one the reader has just typed and may already belong to something
/// they wrote last week. Renaming *ראש השנה* to a name in use destroyed the
/// document that was there, with no prompt and no mention.
///
/// So the destructive case is a refusal by default, and `over` is how the
/// window says *yes, I asked them*. The check is here rather than in the drawer
/// for the same reason [`lane_embed`]'s is: the DOM is not the only caller —
/// the loopback and the MCP server reach the layer too, and neither of them
/// runs a confirm dialog.
///
/// The old file is left where it is, which is what the drawer already promised:
/// *a rename that quietly deleted the thing you had been writing is not a
/// rename.* Both halves of that sentence are true now.
///
/// # Errors
///
/// If either name is not a name, something is already at the new one and `over`
/// was not set, or the personal layer will not take the write.
#[tauri::command(async)]
pub(crate) fn buffer_rename(
    shared: tauri::State<'_, Shared>,
    from: String,
    to: String,
    text: String,
    over: bool,
) -> Result<String, String> {
    let personal = personal_of(&shared)?;
    let named = to.trim();
    if named == from.trim() {
        // Not a rename. Ordinary save, ordinary rules.
        return buffer_save(shared, to, text);
    }
    if !over && girsa_desk::Buffer::taken(&personal, named).map_err(|e| e.to_string())? {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::AlreadyThere,
            format!("you are already writing something called {named}"),
        ));
    }
    let mut buffer = girsa_desk::Buffer::new(named);
    buffer.text = text;
    Ok(buffer
        .save(&personal)
        .map_err(buffer_said)?
        .display()
        .to_string())
}

/// Write the document out into a folder the reader chose.
///
/// The other half of *"send to ksav and export dont let you pick a folder"*.
/// The working buffer stays where it lives — `personal/ksav/` is what
/// `buffers()` lists, and a buffer that wandered off would be a document the
/// drawer could no longer find — so this writes a **copy** where it was asked
/// to, which is what *save a copy* means everywhere else.
///
/// # Errors
///
/// If the folder will not take the file, or the name is not one.
#[tauri::command(async)]
pub(crate) fn buffer_write_to(
    shared: tauri::State<'_, Shared>,
    name: String,
    text: String,
    into: String,
) -> Result<String, String> {
    let folder = PathBuf::from(into.trim());
    if folder.as_os_str().is_empty() {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NothingChosen,
            "no folder was chosen",
        ));
    }
    let named = name.trim();
    if named.is_empty() || named.contains(['/', '\\']) {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            format!("`{name}` is not a name for a document"),
        ));
    }
    // The lock is not held across the write, and the write is not a truncating
    // one. Two separate reasons, one call:
    //
    // The folder is one the reader chose, which may be a network share or a
    // removable disk, so the write is bounded by somebody else's machine — and
    // for every millisecond of it the scroll handler beside it could not be
    // served (`find`, above, makes the same argument at length).
    //
    // And it is the copy the reader deliberately made to keep, on a disk that
    // may be slower and less reliable than their own, which is the last place a
    // truncate-then-write belongs. `beside::write` makes the folder too.
    let path = folder.join(format!("{named}.ksav"));
    girsa_personal::beside::write(&path, text).map_err(|e| e.to_string())?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.export_into = Some(folder.display().to_string());
    state.save();
    Ok(path.display().to_string())
}

/// The markup for a selection, ready to go into the buffer.
///
/// **The window does not build this string.** It is `girsa-ksav`'s, the writer
/// Ksav itself compiles — a second one written in TypeScript is precisely the
/// drift spec.md §10.3 forbids, and it would show up as documents that differ
/// depending on which end wrote them.
#[tauri::command(async)]
pub(crate) fn source_markup(
    shared: tauri::State<'_, Shared>,
    from: String,
    to: Option<String>,
    from_char: usize,
    to_char: Option<usize>,
) -> Result<String, String> {
    let from: SegmentId = from.parse().map_err(|e| format!("{e}"))?;
    let to: SegmentId = match to {
        Some(to) => to.parse().map_err(|e| format!("{e}"))?,
        None => from.clone(),
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let sefer = state.sefer(from.work())?;
    let selection = girsa_app::Selection {
        from,
        to,
        from_char,
        to_char,
    };
    let sent = girsa_app::send(sefer, &selection, style, pointing, shemos, None)
        .map_err(|e| e.to_string())?;
    Ok(girsa_ksav::to_ksav(
        &sent.packet,
        girsa_ksav::CitationPlacement::Mekor,
    ))
}

/// Hand the whole buffer to the real Ksav (spec.md §10.3 — *open the real Ksav
/// editor here*).
///
/// It is saved first, so what Ksav is given and what is on disk are the same
/// words, and only offered when presence says Ksav would take it.
#[tauri::command(async)]
pub(crate) fn buffer_to_ksav(
    shared: tauri::State<'_, Shared>,
    name: String,
    text: String,
) -> Result<(), String> {
    buffer_save(shared, name.clone(), text.clone())?;
    let errand = serde_json::json!({ "name": name, "text": text }).to_string();
    send_to_ksav_or_legacy(TAKE_DOCUMENT, LEGACY_DOCUMENT, &errand)
}

use girsa_post::routes::ksav::{LEGACY_DOCUMENT, TAKE_DOCUMENT};

/// One errand, addressed to the name Ksav answers to.
///
/// `/document` used to mean two unrelated things depending on which way it was
/// travelling, which is why it is `/take-document` now. The rename looked like
/// it needed both applications in one commit — two repositories, so never. It
/// does not: a path only collides *across* the seam, so each side can answer to
/// both names and each sender can try the new one and fall back. Old Girsa with
/// new Ksav, and new Girsa with old Ksav, both work, and the two releases are
/// independent.
///
/// **404 only.** A refusal for any other reason is Ksav having heard the errand
/// and declined it, and asking again under an older name would be asking a
/// second time for something already answered.
fn send_to_ksav_or_legacy(path: &str, legacy: &str, errand: &str) -> Result<(), String> {
    let app = girsa_post::App::Ksav;
    match girsa_post::send(app, path, Some(errand)) {
        Ok(_) => Ok(()),
        Err(girsa_post::PostError::Refused { status: 404, .. }) => {
            girsa_post::send(app, legacy, Some(errand))
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}
