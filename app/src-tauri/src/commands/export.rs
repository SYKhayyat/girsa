//! Exporting a fixed sefer (spec.md §7.4, BUILDER.md W22).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use std::path::PathBuf;

use girsa_app::trouble::{refuse, Code};
use girsa_app::view::Written;

use crate::{Shared, State};

/// Write a sefer out with your corrections in it, into a folder you chose.
///
/// # It used to choose for you
///
/// *"send to ksav and export dont let you pick a folder."* They did not: this
/// wrote into `personal/exports/` and said the path afterwards, and the comment
/// where this one is argued that *"the file is the point and where it goes is
/// not, and a reader who wants it somewhere else has a file manager."* That is
/// a reasonable thing to believe about a debug artefact and the wrong thing to
/// believe about a sefer somebody is going to hand to a chavrusa — the whole
/// reason to export is to put the file **somewhere**, and the somewhere is the
/// reader's business.
///
/// `into` is a folder the window got from a real directory dialog. `None` is
/// *the last one you chose*, and failing that the old default — so the second
/// export does not ask again and a reader who never opens the dialog is exactly
/// where they were.
#[tauri::command(async)]
pub(crate) fn export_sefer(
    shared: tauri::State<'_, Shared>,
    slug: String,
    format: String,
    into: Option<String>,
) -> Result<Written, String> {
    let format = girsa_export::Format::named(&format)
        .ok_or_else(|| refuse(Code::NoSuch, format!("no such format: {format}")))?;
    // **Everything the write needs, gathered under the lock; the write itself
    // outside it.** A sefer is up to 17,418 segments and `session.export_into`
    // is a folder the reader chose — which may be a network share or a stick
    // they can pull out — so the duration of this file write is not something
    // this process gets to bound. Holding the one state lock across it stops
    // the daf beside it scrolling for however long that turns out to be.
    let (sefer, fixes, to, showing, pointing, shemos) = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let showing = state.session.showing;
        let pointing = state.session.pointing;
        let shemos = state.session.shemos;
        let personal = {
            let shelf = state.shelf()?;
            shelf.personal().to_path_buf()
        };
        // Remembered, so the next export opens where the last one went.
        if let Some(chosen) = into.as_ref().filter(|p| !p.trim().is_empty()) {
            state.session.export_into = Some(chosen.clone());
            state.save();
        }
        let folder = state
            .session
            .export_into
            .clone()
            .map_or_else(|| personal.join("exports"), PathBuf::from);
        // The sefer as it is being read — corrections already applied, because
        // that is what `Open` is (W20). Nothing is applied here.
        //
        // One lookup, not two. This asked `state.sefer` to load it and then
        // `state.open.peek` to borrow it back, because the first borrow ruled
        // out reading the shelf in between; a handle rules out nothing.
        let sefer = state.held(&slug)?;
        let to = folder.join(girsa_export::suggested_name(&sefer, format));
        let fixes = {
            let shelf = state.shelf()?;
            shelf.fixes().clone()
        };
        (sefer, fixes, to, showing, pointing, shemos)
    };
    let done = girsa_export::export(&sefer, &fixes, format, pointing, shemos, &to)
        .map_err(|e| e.to_string())?;
    Ok(Written {
        file: done
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        showing: match showing {
            girsa_fix::Showing::AsPrinted => "as-printed",
            girsa_fix::Showing::Fixed => "fixed",
            girsa_fix::Showing::FixedWithVariants => "fixed-with-variants",
        },
        path: done.path.display().to_string(),
        segments: done.segments,
        corrections: done.corrections,
        stale: done.stale,
        noted: done.noted,
    })
}
