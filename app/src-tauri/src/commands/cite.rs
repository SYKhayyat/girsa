//! Citations out of Girsa and into Ksav, and the questions about them coming back.
//!
//! Moved out of `lib.rs` whole (#68). This section never had a banner of its own,
//! which is why the file's title is its first doc comment rather than a heading:
//! `who_cites` asks which of the reader's own documents cite a place, and the rest
//! is the seam — `linkify`, `cite_open`, `ksav_presence`, `send_to_ksav` and the
//! citation style. Nothing here decides anything: what a document citation *means*
//! is `girsa_app::citing`'s, and the errand's wire is `girsa_post`'s.

use girsa_app::trouble::{refuse, Code};
use girsa_corpus::segment::SegmentId;

use crate::{clipboard, hold, post, Copied, Shared, State};

/// Which of your own documents cite this place (spec.md §10.4).
///
/// Only possible because the documents store **refs**: this is a scan, not a
/// guess, and it is why `מקור:` exists.
///
/// # What is done under the lock, and what is not
///
/// This is asked on **every click on a line**, and all of it used to run under
/// the global state guard: `Documents::open`, which parses the registry;
/// `refreshed()`, which reads the **full text of every stale `.ksav` the reader
/// has registered**, with no cap on file size or count; and the drawer scan,
/// which is a `read_dir` plus a full read of every document being written. On a
/// machine whose registry is empty that is 10 ms and on a machine that has been
/// used it is unbounded — and for every one of those milliseconds the scroll
/// handler beside it could not be served. `find` makes the same argument above,
/// at length, and this is that argument applied where it was not.
///
/// So the reading happens with no guard held, and the guard is taken twice:
/// once for the personal path, and once for the registry rows, which are
/// already in memory and cost a string parse each.
#[tauri::command(async)]
pub(crate) fn who_cites(
    shared: tauri::State<'_, Shared>,
    reference: String,
) -> Result<Vec<girsa_desk::Citing>, String> {
    let place: girsa_ref::Ref = reference.parse().map_err(|e| format!("{e}"))?;
    let personal = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let shelf = state.shelf()?;
        shelf.personal().to_path_buf()
    };
    // The registry, read and refreshed with nothing held. Fills the memo on the
    // first ask and is a no-op afterwards.
    read_documents(&shared, &personal)?;
    // The drawer, which is the file-reading half. No guard.
    let mut out = girsa_desk::in_your_drawer(&personal, &place);
    // And the registry rows, which are the cheap half.
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let documents = state.documents(&personal);
    out.extend(girsa_desk::in_your_documents(documents, &place));
    Ok(out)
}

/// Make sure the document registry has been read, **without holding the state
/// lock while reading it**.
///
/// `State::documents` fills the memo in place, which means the fill — parsing
/// the registry and reading every stale document in full — happens under
/// whatever guard the caller is holding. This does the reading outside and only
/// takes the guard to put the answer away.
///
/// A second thread that filled it first wins; this one throws its copy away.
/// That is a wasted read and not a wrong answer, and the alternative is holding
/// the lock across the read to prevent it, which is the thing being fixed.
fn read_documents(
    shared: &tauri::State<'_, Shared>,
    personal: &std::path::Path,
) -> Result<(), String> {
    {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        if state.documents.is_some() {
            return Ok(());
        }
    }
    let (mut documents, trouble) = girsa_desk::documents::Documents::open(personal);
    for line in trouble {
        eprintln!("{line}");
    }
    if let Err(e) = documents.refreshed() {
        eprintln!("the document registry will not write: {e}");
    }
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    if state.documents.is_none() {
        state.documents = Some(documents);
    }
    Ok(())
}

/// The citations in a piece of prose — **the certain ones** (spec.md §10.5).
///
/// Everything ambiguous stays plain text. See `girsa_desk::citing` for the three
/// rules and why each of them refuses more than it accepts.
#[tauri::command(async)]
pub(crate) fn linkify(
    shared: tauri::State<'_, Shared>,
    text: String,
) -> Result<Vec<girsa_desk::Linked>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let lexicon = state.lexicon.as_ref().ok_or_else(|| {
        refuse(
            Code::NoIndex,
            "there is no lexicon here — has girsa-import run?",
        )
    })?;
    Ok(girsa_desk::linkify(lexicon, &text))
}

/// Where a citation in your own writing goes (W19, spec.md §10.5).
///
/// The reader clicked words the pane drew as a link, and this turns the ref
/// those words carry into a place to open. It is the same
/// [`crate::post::landing`] a `girsa://` link from another application takes,
/// on purpose: a citation resolves to one place whether it was clicked in Ksav
/// or in a note of your own, and two resolutions would drift.
#[tauri::command(async)]
pub(crate) fn cite_open(
    shared: tauri::State<'_, Shared>,
    reference: String,
) -> Result<post::Landing, String> {
    let place: girsa_ref::Ref = reference.parse().map_err(|e| format!("{e}"))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    post::landing(&mut state, &place)
}

/// Whether Ksav is there (spec.md §10.6 — *presence*).
///
/// Asked of Ksav rather than assumed from a file: an endpoint left behind by a
/// crash is not presence. The window uses this to decide whether to *offer*
/// sending at all, which is the whole point — an affordance that would fail is
/// never shown.
/// # A8: the chip is telling the truth, and it is the sibling's truth
///
/// > *"it says {ksav} is registered but not answering. i have no clue if that
/// > is right."*
///
/// It is right. `girsa_post::presence` answers `Stale` when there **is** a
/// `ksav-endpoint.json` and nothing answers on the port it names, which is a
/// real state and not a guess — the endpoint is asked, over loopback, before
/// anything is said about it.
///
/// What produces it, nearly always, is the sibling leaving its registration
/// behind when it closes. **Girsa had exactly this bug and fixed it**: see the
/// note on `run()` below — `Builder::run` never returns on Windows, so
/// `Desk::drop` was never reached by any exit a reader can perform, and Ksav saw
/// every ordinary close of Girsa as *registered but not answering*. The fix is
/// the `RunEvent::Exit` callback that takes the desk out. Ksav's side of the
/// same defect is Ksav's to fix, and nothing here can reach it.
///
/// So this command stays exactly as honest as the crate is, and the **sentence**
/// changed instead (`say.ts`, `ksavStale`): the reader is told what it means and
/// what to do, rather than being told a state and left to work out whether it is
/// a crisis. `girsa_post::Endpoint` carries a `pid`, which used to claim it was
/// this check — *"so a stale file can be told from a live one before anything is
/// sent"* — and never was one: asking whether a pid is alive is a foreign call,
/// and the shared workspace forbids `unsafe` rather than merely denying it. The
/// field says what it is now, which is the pid for a person reading the file,
/// and staleness stays here where it works.
#[tauri::command(async)]
pub(crate) fn ksav_presence() -> girsa_post::Presence {
    girsa_post::presence(girsa_post::App::Ksav)
}

/// Send a selection straight into the open Ksav document.
///
/// The clipboard path (W15) works whether or not Ksav is running; this is the
/// one that feels like AirDrop, and it is only offered when presence says it
/// would land.
///
/// # The guard is dropped before the other process is spoken to
///
/// This used to hold the global state lock across `girsa_post::send`, which is
/// a **blocking round trip to another application**. `girsa-post` allows 400 ms
/// per read and five seconds for the whole exchange, so a Ksav that is slow,
/// paging in, or sitting on a modal dialog held Girsa's entire state — the
/// scroll handler included — for up to five seconds.
///
/// Worse, it was a cycle. Girsa's desk serves Ksav's errands on **one thread**
/// (`desk.rs` — `for request in server.incoming_requests()`), and every handler
/// takes this same lock. If Ksav, while handling `/insert`, asked Girsa
/// anything at all — a `/cite` to print the citation it had just been handed is
/// the obvious one — that request blocked on the lock this command was holding,
/// and this command blocked on the reply. Nothing deadlocked permanently
/// because the five-second deadline broke it; the reader got a five-second
/// freeze and an error for an operation that had worked.
///
/// The packet is built under the guard, which is what the guard is for, and the
/// guard is gone before the socket is opened.
#[tauri::command(async)]
pub(crate) fn send_to_ksav(
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
    // On a cache miss this is a whole work off disk, and it is the same 11 ms
    // `open_sefer` refuses to pay under the guard.
    hold(&shared, from.work())?;
    let (sent, packet) = {
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
        let packet = sent.packet.to_json().map_err(|e| e.to_string())?;
        (sent, packet)
    };
    girsa_post::send(girsa_post::App::Ksav, "/insert", Some(&packet)).map_err(|e| e.to_string())?;
    Ok(Copied {
        display: sent.display().to_string(),
        reference: sent.packet.reference.clone(),
        lines: sent.packet.text.lines().count(),
        // Nothing was put on the clipboard: this went the other way.
        put: clipboard::Put::default(),
    })
}

/// How citations print. A preference, and it moves nothing: the document
/// stores the ref.
#[tauri::command(async)]
pub(crate) fn set_cite_style(
    shared: tauri::State<'_, Shared>,
    style: String,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.cite = girsa_cite::CiteStyle::named(&style)
        .ok_or_else(|| refuse(Code::NoSuch, format!("no such style: {style}")))?;
    state.save();
    Ok(())
}
