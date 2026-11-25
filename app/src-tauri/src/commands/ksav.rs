//! The Ksav loop (spec.md §10, BUILDER.md W15).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use girsa_corpus::segment::SegmentId;

use crate::{clipboard, hold, Copied, Shared, State};

/// Copy a selection: the quote, the citation, and the source packet.
///
/// The window sends **character offsets into the text it drew**, which is the
/// text this crate handed it — markup already off, nikud already applied. That
/// is the only way the two ends can agree about where a highlight starts
/// without the webview knowing what a mark is.
#[tauri::command]
pub(crate) fn copy(
    shared: tauri::State<'_, Shared>,
    from: String,
    to: Option<String>,
    from_char: usize,
    to_char: Option<usize>,
    note: Option<String>,
) -> Result<Copied, String> {
    let from: SegmentId = from.parse().map_err(|e| format!("{e}"))?;
    let to: SegmentId = match to {
        Some(to) => to.parse().map_err(|e| format!("{e}"))?,
        None => from.clone(),
    };
    // **Held first.** The 18 August performance audit had this command
    // deliberately not `(async)` — *the clipboard and the print sheet talk to
    // the platform rather than to the shelf, they are fast* — and told a future
    // agent not to re-fix that. It is still right about what this command *does* and it
    // was wrong about what it can *wait for*: `State::sefer` on a cache miss is
    // a whole work off disk, 11 ms in the published table and more on a cold
    // cache, **on the thread that owns the message loop**. Ctrl+C on a sefer
    // just evicted from the twelve-entry cache was a synchronous disk read in
    // the paint path.
    //
    // So the command stays synchronous, which the reasoning above is right
    // about, and the read happens first, which is what `open_sefer` already
    // does. `hold` takes and releases the guard by itself.
    hold(&shared, from.work())?;
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
    let sent = girsa_app::send(sefer, &selection, style, pointing, shemos, note)
        .map_err(|e| e.to_string())?;
    Ok(Copied {
        display: sent.display().to_string(),
        reference: sent.packet.reference.clone(),
        lines: sent.packet.text.lines().count(),
        put: clipboard::put(&sent),
    })
}
