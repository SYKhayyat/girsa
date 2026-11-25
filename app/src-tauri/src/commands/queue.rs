//! The OCR queue (spec.md §7.3, BUILDER.md W21).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use girsa_app::trouble::{refuse, Code};
use girsa_app::view::{Standing, SuspectRow};
use girsa_corpus::segment::SegmentId;

use crate::{personal_of, Shared, State};

/// The next candidates to review, best first.
///
/// Re-read from disk every time: `girsa-suspects` is a batch job that runs
/// outside this window, and a queue held in memory would be the one from
/// before it ran.
#[tauri::command(async)]
pub(crate) fn suspects(
    shared: tauri::State<'_, Shared>,
    limit: usize,
) -> Result<Vec<SuspectRow>, String> {
    // The parse first, with no lock held. The queue is 28,124 lines on the
    // real corpus and it is re-read every time the drawer opens — see the note
    // above — so this is the one long read in the command and it needs the
    // personal path and nothing else out of the state.
    let personal = personal_of(&shared)?;
    let (queue, trouble) = girsa_fix::suspect::Queue::open(&personal);
    for line in trouble {
        eprintln!("{line}");
    }
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let rows: Vec<SuspectRow> = {
        let shelf = state.shelf()?;
        let names = state.names(&shelf);
        queue
            .page(limit)
            .into_iter()
            .map(|suspect| {
                let at = suspect.places.first();
                SuspectRow {
                    id: suspect.id.clone(),
                    rare: suspect.rare.clone(),
                    common: suspect.common.clone(),
                    rare_count: suspect.rare_count,
                    common_count: suspect.common_count,
                    confusion: suspect.confusion.clone(),
                    how: suspect.how.as_str(),
                    at: at.map(ToString::to_string),
                    work: at.map(|id| id.work().to_string()),
                    // The sixth. `Names::of` falls back to the slug rather than to
                    // `None`, so a suspect in a sefer the catalogue has not caught
                    // up with now draws a row with a name on it.
                    title: at.map(|id| names.of(id).title),
                    address: at.map(girsa_corpus::segment::SegmentId::address),
                }
            })
            .collect()
    };
    state.queue = Some(queue);
    Ok(rows)
}

/// Open a candidate: where its word sits in the segment the queue named.
///
/// Two kinds of place, because the queue ranks words off both kinds of sefer.
/// `girsa-suspects` reads the index's term dictionary, and since W26 that
/// dictionary holds what an engine read off a photograph as well as what a
/// file said — so tesseract's misreads have always been ranked beside
/// Otzaria's. What they had no way to reach was **this** call: a page segment
/// carries no text of its own, so the character-span lookup below tokenized an
/// empty string and every candidate on a scan answered *that word is not in
/// that line any more*. The words of a page are in the reading, and the
/// correction is by ink, so a page takes the other branch end to end.
#[tauri::command(async)]
pub(crate) fn suspect_at(
    shared: tauri::State<'_, Shared>,
    id: String,
    at: String,
) -> Result<Standing, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    // The same 28,124-line read `suspects` does, and off the lock for the same
    // reason.
    let personal = personal_of(&shared)?;
    let (queue, _) = girsa_fix::suspect::Queue::open(&personal);
    let suspect = queue
        .get(&id)
        .ok_or_else(|| refuse(Code::NoSuch, "there is no such candidate"))?
        .clone();

    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.queue = Some(queue);
    // **No `shemos` here, and that is deliberate.** This is the correction
    // path: what it shows the reader is the word as the corpus has it, and
    // what they type goes back into their own layer as a claim about that
    // word. A box that offered `יקוק` for correction would be inviting a
    // reader to write a substitution into the corpus, which is the one place
    // this setting must not reach. `girsa_app::fixing` is silent about it for
    // the same reason.
    let pointing = state.session.pointing;

    // Which page of the file, if this is a page at all. Counted through the
    // pages rather than read off the ordinal, because a page that was split by
    // a correction mints `#47.1` and arithmetic on the ordinal would slip by
    // one from there (`girsa_app::scanning::page_of_id`).
    let page = {
        let sefer = state.sefer(at.work())?;
        let position = sefer
            .position_of(&at)
            .ok_or_else(|| refuse(Code::NoSuch, "not in this sefer"))?;
        let kind = sefer
            .segments
            .get(position)
            .ok_or_else(|| refuse(Code::NoSuch, "not in this sefer"))?
            .kind;
        match kind {
            girsa_corpus::import::SegmentKind::Page => girsa_app::scanning::page_of_id(sefer, &at),
            _ => None,
        }
    };
    if let Some(page) = page {
        let slug = at.work().to_string();
        // The reading with the reader's own corrections already applied, which
        // is what makes a candidate they have already fixed report itself as
        // gone rather than opening a box on a word that no longer says that.
        let read = state.words(&slug)?.page(page).ok_or_else(|| {
            refuse(
                Code::NoPage,
                format!("nobody has read page {page} of {slug}"),
            )
        })?;
        let word = girsa_app::scanning::where_word_on_page(&read, &suspect.rare)
            .ok_or_else(|| refuse(Code::NoPage, "that word is not on that page any more"))?;
        let printed = read
            .words
            .get(word)
            .map(|on_page| on_page.text.clone())
            .unwrap_or_default();
        return Ok(Standing {
            at: at.to_string(),
            // A photograph has no text to take an offset into. The place is
            // the rectangle, and it is named by `page` and `word`.
            from_char: 0,
            to_char: 0,
            suggestion: suspect.suggestion(&printed),
            printed,
            page: Some(page),
            word: Some(word),
        });
    }

    let sefer = state.sefer(at.work())?;
    let span = girsa_app::fixing::where_word(sefer, &at, &suspect.rare, pointing)
        .ok_or_else(|| refuse(Code::NoSuch, "that word is not in that line any more"))?;
    let drawn = girsa_app::display::Shown::of(
        &sefer
            .segments
            .get(
                sefer
                    .position_of(&at)
                    .ok_or_else(|| refuse(Code::NoSuch, "not in this sefer"))?,
            )
            .ok_or_else(|| refuse(Code::NoSuch, "not in this sefer"))?
            .text,
        pointing,
    );
    let printed: String = drawn
        .text()
        .chars()
        .skip(span.start)
        .take(span.len())
        .collect();
    Ok(Standing {
        at: at.to_string(),
        from_char: span.start,
        to_char: span.end,
        suggestion: suspect.suggestion(&printed),
        printed,
        page: None,
        word: None,
    })
}

/// Say what was done about a candidate: corrected, or not an error.
///
/// Recorded so the batch job does not ask again — never so that anything is
/// applied. The correction itself, if there is one, went through `fix`.
#[tauri::command(async)]
pub(crate) fn suspect_decide(
    shared: tauri::State<'_, Shared>,
    id: String,
    decision: String,
) -> Result<(), String> {
    let decision = match decision.as_str() {
        "dismissed" => girsa_fix::suspect::Decision::Dismissed,
        "fixed" => girsa_fix::suspect::Decision::Fixed,
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such decision: {other}"),
            ))
        }
    };
    // Take the held queue if there is one; read it off disk if there is not,
    // and do that read with nothing locked.
    let held = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state.queue.take()
    };
    let mut queue = match held {
        Some(queue) => queue,
        None => girsa_fix::suspect::Queue::open(&personal_of(&shared)?).0,
    };
    // `decide` appends to the decisions log, which is a file write.
    let known = queue.decide(&id, decision).map_err(|e| e.to_string())?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.queue = Some(queue);
    if known {
        Ok(())
    } else {
        Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoSuch,
            "there is no such candidate",
        ))
    }
}
