//! Scans (spec.md §6.3, BUILDER.md W25).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use crate::{clipboard, Copied, Shared, State};

use girsa_app::shelf::{Open, Shelf};
use girsa_app::trouble::{refuse, Code};
use girsa_app::view::{
    AnchorRow, Card, DrawnRow, GapRow, PageSaid, PageWordsRow, ReadingRow, ScanView, ScannedRow,
    WordRow,
};

/// Open a scan into a pane.
#[tauri::command(async)]
pub(crate) fn scan(shared: tauri::State<'_, Shared>, slug: String) -> Result<ScanView, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&slug)?;
    // Where this scan was left last time. Looked up here rather than worked out
    // in the window from the id it remembered, for the reason in `page_of_id`.
    let left = state.session.positions.get(&slug).cloned();
    let shelf = state.shelf()?;
    let sefer = state
        .open
        .peek(&slug)
        .ok_or_else(|| refuse(Code::NoSefer, "not open"))?;
    let scan = girsa_app::scan_of(&shelf, sefer)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{slug} is not a scan")))?;
    let at = left
        .and_then(|id| girsa_app::scanning::page_of_id(sefer, &id))
        .unwrap_or(1);
    Ok(view_of(&shelf, sefer, &scan, at))
}

// ---------------------------------------------------------------------------
// Reading a scan — spec.md §6.3 and §9.7, W26
//
// The division of labour is W25's, applied to words instead of pixels: the
// window is the only thing here that opens a PDF, because pdf.js is one
// renderer on all three platforms and a second PDF stack in Rust would be a
// second opinion about the same file. So the window hands over **glyphs**, or a
// **picture**, and everything after that — where the words are, what is left to
// read, which pages nobody can search yet — is decided in `girsa-scan`, where
// it can be tested without a webview.
//
// And it hands them over **one page at a time**. spec.md §6.3 asks for a job
// that is *background, resumable, never blocking reading*, and the shape of
// that promise is a call that returns after one page: the reader can turn to
// the sugya they were on, and the only cost of stopping is the page it was on.
// ---------------------------------------------------------------------------

/// How far a scan has been read.
#[tauri::command(async)]
pub(crate) fn scan_reading(
    shared: tauri::State<'_, Shared>,
    slug: String,
) -> Result<ReadingRow, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&slug)?;
    let personal = {
        let shelf = state.shelf()?;
        shelf.personal().to_path_buf()
    };
    let sefer = state
        .open
        .peek(&slug)
        .ok_or_else(|| refuse(Code::NoSefer, "not open"))?;
    let pages = girsa_app::scanning::pages_of(sefer);
    // Held, not re-opened. `Words::open` parses the whole log to answer about
    // one page, and this command is called *again* by `scan_read_page` and
    // `scan_ocr_page` the moment either of them has recorded something.
    let words = state.words(&slug)?;
    let job = girsa_scan::Job::of(&slug, pages, words);
    Ok(ReadingRow {
        slug,
        pages,
        read: job.done(),
        next: job.next(),
        by: words.read_by(),
        engine: girsa_scan::Tesseract::found(Some(&personal)).map(|t| girsa_scan::Engine::name(&t)),
        stranded: words.stranded().len(),
    })
}

/// Take the glyphs the window read off one page of a PDF and make words of them.
#[tauri::command(async)]
pub(crate) fn scan_read_page(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
    width: f32,
    height: f32,
    glyphs: Vec<DrawnRow>,
) -> Result<ReadingRow, String> {
    // The shelf has to be there, and this is where that is refused — the
    // personal path itself is `State::words`'s to find now.
    {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let _shelf = state.shelf()?;
    }
    if width <= 0.0 || height <= 0.0 {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NoPage,
            "a page with no size on it",
        ));
    }
    // Pixels in, fractions of the page out, converted here and once: a
    // rectangle in pixels is a fact about the size somebody rendered at, and a
    // highlight stored in one lands in the margin the first time a reader
    // zooms.
    let glyphs: Vec<girsa_scan::Glyph> = glyphs
        .into_iter()
        .map(|g| girsa_scan::Glyph {
            text: g.text,
            at: girsa_scan::Area::new(
                g.left / width,
                g.top / height,
                g.right / width,
                g.bottom / height,
            ),
        })
        .collect();
    let grouped = girsa_scan::group(&glyphs);
    if grouped.refused > 0 {
        eprintln!(
            "{slug} page {page}: {} words the file would not spell",
            grouped.refused
        );
    }
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state
            .words(&slug)?
            .record(girsa_scan::Read::new(
                page,
                girsa_scan::Reader::Embedded,
                grouped.words,
            ))
            .map_err(|e| e.to_string())?;
    }
    scan_reading(shared, slug)
}

/// Look at the picture instead, for a page that carries no text of its own.
///
/// `png` arrives **base64**. As `Vec<u8>` it was a JSON array of numbers: a
/// multi-MB page render became millions of boxed values inside the exact
/// serialization this job promised never to make the reader wait for.
#[tauri::command(async)]
pub(crate) fn scan_ocr_page(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
    width: u32,
    height: u32,
    png: String,
) -> Result<ReadingRow, String> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    let png = B64
        .decode(png.as_bytes())
        .map_err(|e| format!("the picture of page {page} did not decode: {e}"))?;
    let personal = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let shelf = state.shelf()?;
        let personal = shelf.personal().to_path_buf();
        personal
    };
    let engine = girsa_scan::Tesseract::found(Some(&personal)).ok_or_else(|| {
        girsa_app::trouble::refuse(
            Code::NoEngine,
            girsa_scan::EngineError::NoEngine.to_string(),
        )
    })?;
    let read = girsa_scan::Engine::read(&engine, page, &girsa_scan::Image { png, width, height })
        .map_err(|e| e.to_string())?;
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state
            .words(&slug)?
            .record(read)
            .map_err(|e| e.to_string())?;
    }
    scan_reading(shared, slug)
}

/// What is on a page, for drawing a highlight over the photograph.
#[tauri::command(async)]
pub(crate) fn scan_words(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<Option<PageWordsRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let words = state.words(&slug)?;
    Ok(words.page(page).map(|read| PageWordsRow {
        page: read.page,
        by: Some(read.by.name().to_string()),
        guessed: read.by.is_ocr(),
        words: read.words.iter().map(WordRow::of).collect(),
    }))
}

/// Correct a word on a page, by its ink rather than by where it is in the text.
///
/// The whole of W26 in one call: what is written down is the rectangle, so the
/// correction is still on the same word after the page has been read again by
/// something better (`girsa-scan/tests/the_image_is_ground_truth.rs`).
#[tauri::command(async)]
pub(crate) fn scan_fix(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
    word: usize,
    says: String,
) -> Result<Option<PageWordsRow>, String> {
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let words = state.words(&slug)?;
        let was = words
            .as_read(page)
            .ok_or_else(|| {
                refuse(
                    Code::NoPage,
                    format!("nobody has read page {page} of {slug}"),
                )
            })?
            .words
            .get(word)
            .ok_or_else(|| {
                refuse(
                    Code::NoPage,
                    format!("page {page} has no word {word} on it"),
                )
            })?
            .clone();
        words
            .fix(
                page,
                girsa_scan::Fix {
                    at: was.at,
                    was: was.text,
                    says,
                },
            )
            .map_err(|e| e.to_string())?;
    }
    scan_words(shared, slug, page)
}

/// Highlight a run of words on a page, by the ink they sit on (W24, W26).
///
/// A page is one segment and carries no text, so the character span every other
/// highlight in this window is stored as means nothing here — a scan could only
/// ever be marked whole. What a page has is words with rectangles under them,
/// and the rectangle is the anchor that survives the page being read again by a
/// better engine, which is the argument `scan_fix` already makes about a
/// correction.
///
/// One rectangle per line rather than one for the run: the bounding box of a
/// highlight spanning three lines also covers the ends of the lines it passes
/// through, and redrawing from it would grow the mark by words nobody chose.
#[tauri::command(async)]
pub(crate) fn scan_mark(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
    from_word: usize,
    to_word: usize,
    label: Option<String>,
    colour: Option<String>,
) -> Result<Vec<girsa_app::view::ScanMarkRow>, String> {
    if from_word > to_word {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NothingChosen,
            "nothing is selected",
        ));
    }
    {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let at = {
            let sefer = state.sefer(&slug)?;
            girsa_app::scanning::page_id(sefer, page)
                .ok_or_else(|| refuse(Code::NoPage, format!("{slug} has no page {page}")))?
        };
        let read = state.words(&slug)?.page(page).ok_or_else(|| {
            refuse(
                Code::NoPage,
                format!("nobody has read page {page} of {slug}"),
            )
        })?;
        let (ink, was) = girsa_app::scanning::ink_of(&read, from_word..to_word + 1)
            .ok_or_else(|| refuse(Code::NoPage, "those words are not on this page"))?;
        let who = girsa_app::who();
        let mut mark = girsa_note::Mark::on_ink(at, ink, was, &who);
        mark.label = label;
        mark.colour = colour;
        state
            .shelf_mut()?
            .marks_mut()
            .add(mark)
            .map_err(|e| e.to_string())?;
    }
    scan_marks(shared, slug, page)
}

/// The highlights on a page, with the words each one covers **now**.
#[tauri::command(async)]
pub(crate) fn scan_marks(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<Vec<girsa_app::view::ScanMarkRow>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let at = {
        let sefer = state.sefer(&slug)?;
        girsa_app::scanning::page_id(sefer, page)
            .ok_or_else(|| refuse(Code::NoPage, format!("{slug} has no page {page}")))?
    };
    let read = state.words(&slug)?.page(page);
    let shelf = state.shelf()?;
    let standing = girsa_corpus::standing::Standing::just(at);
    let rows = shelf
        .marks()
        .on(&standing)
        .into_iter()
        .filter(|mark| !mark.ink.is_empty())
        .map(|mark| {
            // What is under that ink today. A page nobody has read any more —
            // the reading was thrown away and not replaced — leaves this empty,
            // which is honest: the rectangle is still on the photograph and
            // there are no words to name.
            let says = read.as_ref().map_or_else(String::new, |read| {
                girsa_app::scanning::words_under(read, &mark.ink)
                    .into_iter()
                    .filter_map(|at| read.words.get(at))
                    .map(|word| word.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            });
            girsa_app::view::ScanMarkRow {
                id: mark.id.as_str().to_string(),
                ink: mark
                    .ink
                    .iter()
                    .map(|area| girsa_app::view::WordRow::box_of(*area))
                    .collect(),
                was: mark.was.clone(),
                says,
                label: mark.label.clone(),
                colour: mark.colour.clone(),
                tags: mark.tags.clone(),
            }
        })
        .collect();
    Ok(rows)
}

/// What a search over this shelf cannot see — spec.md §9.7's results header, and
/// the two things it never used to include (B7).
///
/// The header used to be about scans alone, because `Gap` had one variant for
/// them and none for the reader's own writing. A note written this morning and a
/// typo fixed last night are equally absent from the index and were equally
/// unmentioned, which is the state a bochur is in every single day.
#[tauri::command(async)]
pub(crate) fn scan_gap(shared: tauri::State<'_, Shared>) -> Result<Option<GapRow>, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    let shelf = state.shelf()?;
    let personal = shelf.personal().to_path_buf();
    // Where the index is, if it is anywhere: two of the three gaps are *since the
    // index was built*, so a window that cannot find one has a bigger gap to
    // report, not a smaller one.
    let index = girsa_app::find_index(shelf.root()).ok();
    let gap = girsa_app::reading::gap(&shelf, &personal, index.as_deref());
    // Through `Unseen` rather than `Gap::said` directly: the window's header and
    // the MCP server's `did_not_search` are one sentence with one separator, and
    // the lane clause is one argument away from belonging here too.
    let unseen = girsa_nearby::Unseen::literal(gap);
    let gap = &unseen.literal;
    Ok(unseen.said().map(|said| GapRow {
        said,
        pages: gap.pages,
        scans: gap
            .scans
            .iter()
            .map(|scan| ScannedRow {
                slug: scan.slug.clone(),
                title: scan.title.clone(),
                pages: scan.pages,
                read: scan.read,
            })
            .collect(),
        notes: gap.layer.notes.count(),
        fixes: gap.layer.fixes.count(),
        corrected_scans: gap.layer.scans.count(),
    }))
}

fn view_of(shelf: &Shelf, sefer: &Open, scan: &girsa_scan::Scan, at: usize) -> ScanView {
    ScanView {
        work: Card::of(&sefer.work),
        pages: scan.pages(),
        at,
        file: sefer.work.origin.display().to_string(),
        paged: scan.is_paged(),
        of: scan.paging().of().map(ToString::to_string),
        scheme: scan.paging().scheme().name(),
        anchors: scan
            .paging()
            .anchors()
            .iter()
            .map(|a| AnchorRow {
                page: a.page,
                at: a.at.as_ref().map(ToString::to_string),
            })
            .collect(),
        // Coded, like every other refusal that crosses. `naming` fails one way
        // — the sefer this scan says it is a scan of is not on the shelf — and
        // the window used to print `ShelfError`'s English straight into the
        // note under the page.
        trouble: girsa_app::scanning::naming(shelf, scan)
            .err()
            .map(|e| refuse(Code::NoSefer, e)),
    }
}

/// What is printed on a page.
#[tauri::command(async)]
pub(crate) fn scan_at(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<PageSaid, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let style = state.session.cite;
    state.sefer(&slug)?;
    let shelf = state.shelf()?;
    let sefer = state
        .open
        .peek(&slug)
        .ok_or_else(|| refuse(Code::NoSefer, "not open"))?;
    let scan = girsa_app::scan_of(&shelf, sefer)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{slug} is not a scan")))?;

    // A scan whose sefer is not on the shelf still shows its pages; what it
    // cannot do is print a mekor naming a sefer nobody here has.
    let sent = girsa_app::scanning::naming(&shelf, &scan)
        .ok()
        .and_then(|naming| girsa_app::mareh_makom(&scan, page, &naming, &sefer.work, style));
    Ok(PageSaid {
        page,
        display: sent.as_ref().map(|s| s.display().to_string()),
        reference: sent.as_ref().map(|s| s.packet.reference.clone()),
        id: girsa_app::scanning::page_id(sefer, page).map(|id| id.to_string()),
    })
}

/// Say which page is which daf — the once-per-sefer chore (spec.md §6.3).
#[tauri::command(async)]
pub(crate) fn scan_map(
    shared: tauri::State<'_, Shared>,
    slug: String,
    scheme: String,
    anchors: Vec<AnchorRow>,
    of: Option<String>,
) -> Result<ScanView, String> {
    let scheme = girsa_scan::Scheme::named(&scheme).ok_or_else(|| {
        refuse(
            Code::NoSuch,
            format!("{scheme}: this reads `amud`, `daf` or `numbered`"),
        )
    })?;
    let anchors: Result<Vec<girsa_scan::Anchor>, String> = anchors
        .into_iter()
        .map(|row| match row.at {
            Some(at) => girsa_scan::Anchor::written(row.page, &at).map_err(|e| e.to_string()),
            None => Ok(girsa_scan::Anchor::unpaged(row.page)),
        })
        .collect();
    let of = of.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    // What the named sefer actually carries, counted by the same arithmetic
    // the scheme uses — so a daf the masechta ends before is refused here,
    // with the anchor named, instead of shipping as a mekor that cites
    // cleanly and resolves nowhere. Unchecked when nothing is named or the
    // work is not on this shelf: there is nothing to ask.
    let units = of.as_ref().and_then(|named| {
        let state = shared.lock().ok()?;
        let shelf = state.shelf().ok()?;
        let open = shelf.read(named).ok()?;
        let scheme_units: std::collections::HashSet<u32> = open
            .segments
            .iter()
            .filter_map(|segment| segment.id.path().first())
            .filter_map(|leaf| scheme.unit_of_written(leaf))
            .collect();
        Some(scheme_units)
    });

    let paging = match &units {
        Some(units) => {
            let ask = |address: &girsa_ref::Address| {
                address
                    .levels()
                    .first()
                    .and_then(|level| scheme.unit_of_written(&level.to_string()))
                    .is_some_and(|unit| units.contains(&unit))
            };
            girsa_scan::Paging::declare_checked(of.clone(), scheme, anchors?, &ask)
        }
        None => girsa_scan::Paging::declare(of, scheme, anchors?),
    }
    .map_err(|e| e.to_string())?;

    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    {
        let mut shelf = state.shelf_mut()?;
        shelf
            .declare_paging(&slug, paging)
            .map_err(|e| e.to_string())?;
    }
    // What an address of this sefer means has changed, so the copy held open
    // is out of date (see `Open::paging`).
    state.reread(&slug);
    drop(state);
    scan(shared, slug)
}

/// Take a mapping back — better no mareh makom than a wrong one.
#[tauri::command(async)]
pub(crate) fn scan_forget(
    shared: tauri::State<'_, Shared>,
    slug: String,
) -> Result<ScanView, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    {
        let mut shelf = state.shelf_mut()?;
        shelf.forget_paging(&slug).map_err(|e| e.to_string())?;
    }
    state.reread(&slug);
    drop(state);
    scan(shared, slug)
}

/// The page a place is printed on — the *go to daf* box.
///
/// `None` where this scan does not carry it, and never the nearest page it
/// does: a scan opened one daf away with the header naming the daf that was
/// asked for is wrong in the way nobody checks.
#[tauri::command(async)]
pub(crate) fn scan_page_of(
    shared: tauri::State<'_, Shared>,
    slug: String,
    written: String,
) -> Result<Option<usize>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&slug)?;
    let shelf = state.shelf()?;
    let sefer = state
        .open
        .peek(&slug)
        .ok_or_else(|| refuse(Code::NoSefer, "not open"))?;
    let scan = girsa_app::scan_of(&shelf, sefer)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{slug} is not a scan")))?;

    // A ref pasted in, or a place typed the way a reader writes one. Both are
    // read by `girsa-ref`, which is the one thing in this system that knows
    // what `ב ע"ב` is.
    if let Ok(reference) = written.parse::<girsa_ref::Ref>() {
        return Ok(scan.page_of_ref(&reference));
    }
    Ok(girsa_ref::Address::parse(&written).and_then(|address| scan.page_of(&address)))
}

/// Ctrl+C on a page of a scan: the mareh makom, in the three flavours.
///
/// There is nothing to quote — the importer will not invent Hebrew it cannot
/// read — so what goes down is the citation and the ref. `girsa-ksav` writes
/// that as a mareh makom rather than as an empty quote block.
#[tauri::command(async)]
pub(crate) fn scan_copy(
    shared: tauri::State<'_, Shared>,
    slug: String,
    page: usize,
) -> Result<Copied, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let style = state.session.cite;
    state.sefer(&slug)?;
    let shelf = state.shelf()?;
    let sefer = state
        .open
        .peek(&slug)
        .ok_or_else(|| refuse(Code::NoSefer, "not open"))?;
    let scan = girsa_app::scan_of(&shelf, sefer)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{slug} is not a scan")))?;
    let naming = girsa_app::scanning::naming(&shelf, &scan).map_err(|e| e.to_string())?;
    let sent =
        girsa_app::mareh_makom(&scan, page, &naming, &sefer.work, style).ok_or_else(|| {
            format!("there is nothing printed on page {page} that a mekor could name")
        })?;
    Ok(Copied {
        display: sent.display().to_string(),
        reference: sent.packet.reference.clone(),
        lines: 0,
        put: clipboard::put(&sent),
    })
}
