//! The library's side of the loopback (spec.md §10.6, BUILDER.md W16).
//!
//! Three errands, and each of them is Ksav asking the library a question only
//! the library can answer:
//!
//! | | |
//! |---|---|
//! | `POST /open` | *show me this place* — the window opens the sefer and scrolls to it |
//! | `POST /cite` | *print this ref in that style* — the citation, re-printed |
//! | `POST /quote` | *give me this source again* — the words, from the corpus as it stands now |
//!
//! # Why `/cite` and `/quote` exist at all
//!
//! Because a Ksav document stores the **ref**, not the printed string, spec.md
//! §10.2 promises two things that would otherwise be impossible: switching a
//! whole sefer from abbreviated to full-form citations, and regenerating every
//! quote against a corrected edition (§7). Both need the library — the title,
//! the words the schema uses for a level, and the text itself all live here.
//!
//! Ksav could have carried a copy of them. That is exactly the drift the shared
//! crates exist to prevent: two catalogues, and the one in the pen is the one
//! nobody updates. So the formatter is shared code (`girsa-cite`) and the
//! *facts* stay in the library, one loopback call away.

use girsa_app::sending::about;
use girsa_app::Shelf;
use girsa_cite::{cite, CiteStyle};
use girsa_post::desk::{Desk, Reply};
use girsa_post::{App, Errand};
use girsa_ref::Ref;
use serde::Deserialize;
use tauri::{Emitter, Manager};

use crate::Shared;

/// The event the window listens for when something asks Girsa to show a place.
pub const OPEN_EVENT: &str = "girsa://open";

/// What every errand carries.
#[derive(Deserialize)]
struct Asked {
    #[serde(rename = "ref")]
    reference: String,
    /// `hebrew-full`, `hebrew-short`, `english`. Absent means the reader's own
    /// setting, which is the sensible default for a window that is open.
    #[serde(default)]
    style: Option<String>,
}

/// Open the desk and start answering.
///
/// Returns the desk, which has to be kept alive: dropping it withdraws the
/// endpoint file, which is how presence stops being reported the moment the
/// application stops.
///
/// A failure here is **not** a reason to refuse to start. Girsa is a library
/// first; if the pairing cannot be opened the window still reads seforim, and
/// the presence chip says why.
pub fn open(handle: &tauri::AppHandle) -> Result<Desk, std::io::Error> {
    let desk = Desk::open(App::Girsa, env!("CARGO_PKG_VERSION"))?;
    let handle = handle.clone();
    desk.serve(move |path, body| answer(&handle, path, body));
    Ok(desk)
}

fn answer(handle: &tauri::AppHandle, path: &str, body: &str) -> Reply {
    let asked: Asked = match serde_json::from_str(body) {
        Ok(asked) => asked,
        Err(e) => return Reply::refused(400, format!("that is not an errand: {e}")),
    };
    let Ok(reference) = asked.reference.parse::<Ref>() else {
        return Reply::refused(400, format!("`{}` is not a girsa ref", asked.reference));
    };

    match path {
        "/open" => show(handle, &reference),
        "/cite" => quote(handle, &reference, asked.style.as_deref(), false),
        "/quote" => quote(handle, &reference, asked.style.as_deref(), true),
        other => Reply::refused(404, format!("no such errand: {other}")),
    }
}

/// *Show me this place.* The window does the opening; this only tells it.
///
/// The ref is turned into a **segment id** here rather than in the window,
/// because that is a question about the corpus: which segments an address
/// names is decided by the same index the link graph was built with, and a
/// second answer computed in TypeScript would disagree with it eventually.
fn show(handle: &tauri::AppHandle, reference: &Ref) -> Reply {
    let shared = handle.state::<Shared>();
    let Ok(mut state) = shared.lock() else {
        return Reply::refused(500, "the library is busy");
    };
    let slug = reference.work_slug();
    let sefer = match state.sefer(&slug) {
        Ok(sefer) => sefer,
        Err(e) => return Reply::refused(404, e),
    };
    let at = sefer.at(reference.from());
    let Some(first) = at.first() else {
        return Reply::refused(
            404,
            format!("{slug} is on the shelf and has no {}", reference.from()),
        );
    };
    let landing = serde_json::json!({
        "ref": reference.to_string(),
        "slug": slug,
        "id": first.to_string(),
    });
    // Raised rather than acted on here: which pane a sefer opens in, and what
    // happens to the one beside it, is the window's business and is already
    // wired.
    drop(state);
    if let Err(e) = handle.emit(OPEN_EVENT, &landing) {
        return Reply::refused(500, format!("the window did not take it: {e}"));
    }
    // And bring it to the front — an app that quietly changed pages behind
    // three other windows has not shown anybody anything.
    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    Reply::ok(landing.to_string())
}

/// The citation, and — for `/quote` — the words it names, as the corpus stands
/// now.
fn quote(handle: &tauri::AppHandle, reference: &Ref, style: Option<&str>, text: bool) -> Reply {
    let shared = handle.state::<Shared>();
    let Ok(mut state) = shared.lock() else {
        return Reply::refused(500, "the library is busy");
    };
    let style = style
        .and_then(CiteStyle::named)
        .unwrap_or(state.session.cite);

    let slug = reference.work_slug();
    let Some(work) = state
        .shelf
        .as_ref()
        .and_then(|s: &Shelf| s.work(&slug))
        .cloned()
    else {
        return Reply::refused(404, format!("{slug} is not on this shelf"));
    };
    let display = cite(&about(&work), reference, style);

    if !text {
        return Reply::ok(
            serde_json::json!({
                "ref": reference.to_string(),
                "display": display,
                "he_title": work.he_title,
                "en_title": work.en_title,
            })
            .to_string(),
        );
    }

    // The words, re-read from the corpus rather than from whatever the document
    // remembers: that is the whole point of storing a ref (spec.md §7). Which
    // segments the address names is `girsa_app`'s to answer, and is tested
    // there.
    let nikud = state.session.nikud;
    let sefer = match state.sefer(&slug) {
        Ok(sefer) => sefer,
        Err(e) => return Reply::refused(404, e),
    };
    match girsa_app::quote(sefer, reference, style, nikud) {
        Ok(sent) => match sent.packet.to_json() {
            Ok(json) => Reply::ok(json),
            Err(e) => Reply::refused(500, e.to_string()),
        },
        Err(e) => Reply::refused(404, e.to_string()),
    }
}

/// A `girsa://` URL the operating system handed us.
///
/// The same errand as `/open`, arriving the other way: from a citation clicked
/// in a Word document, a compiled PDF, or a chat message. Anything that is not
/// an errand is dropped without comment — a URL handler is reachable by any web
/// page on the machine.
pub fn opened_url(handle: &tauri::AppHandle, url: &str) {
    let Some(Errand::Open { reference }) = girsa_post::deep_link(App::Girsa, url) else {
        return;
    };
    if let Ok(reference) = reference.parse::<Ref>() {
        show(handle, &reference);
    }
}
