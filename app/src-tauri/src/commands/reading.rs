//! Reading a sefer in a pane: its page, its lines, and finding a phrase in it.

use girsa_app::display;
use girsa_app::trouble::{refuse, Code};
use girsa_app::view::Line;
use girsa_search::bar::Answer;
use girsa_search::chips::Chip;
use girsa_search::index::Paging;
use serde::Serialize;

use girsa_corpus::segment::SegmentId;

use crate::commands::search::marked;
use crate::{hold, Shared, State};

/// One end of a link, quoted.
#[derive(Serialize)]
pub(crate) struct Words {
    at: String,
    /// The opening words, for the row as it stands.
    opening: String,
    /// The whole line, for the row opened out.
    said: String,
    /// Where it is, printed the reader's way.
    address: String,
}

/// The words at the other end of a set of links, for one sefer.
///
/// # The finding this closes
///
/// > *"all of it — i don't know what i'm looking at."*
///
/// The links panel on one line of Yoreh De'ah draws 280 rows. **Seventy-eight
/// of them say `כף החיים על שולחן ערוך יורה דעה` and differ by one number.**
/// They are not duplicates — the Kaf HaChayim really writes ס״ק א׳ through
/// ס״ק ע״ח on that one se'if — but what repeats down the column is the sefer's
/// name, seventy-eight times, and what a reader wants is *what does it say*.
///
/// `LinkRow::preview` was the first answer and it is only filled when the other
/// sefer **is already open**, for a reason that still holds: a sidebar is not
/// entitled to read forty seforim off the disk to decorate a list. Grouping the
/// panel by sefer is what makes the reason stop biting — a reader opens one
/// group, and this reads one sefer.
///
/// So the whole line comes back as well as its opening words. Once a group is
/// open, expanding any row in it costs nothing: the words are already here, and
/// a reader walking seventy-eight ס״ק is not waiting on seventy-eight round
/// trips.
#[tauri::command(async)]
pub(crate) fn link_words(
    shared: tauri::State<'_, Shared>,
    work: String,
    ats: Vec<String>,
) -> Result<Vec<Words>, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let sefer = state.sefer(&work)?;
    let mut out = Vec::with_capacity(ats.len());
    for at in &ats {
        let Ok(id) = at.parse::<SegmentId>() else {
            continue;
        };
        let Some(nth) = sefer.position_of(&id) else {
            // The graph points at a segment this sefer does not have. Left out
            // rather than reported as an empty quote: it is a fact about the
            // link, and W23's panel is where a bad link is repaired.
            continue;
        };
        let Some(segment) = sefer.segments.get(nth) else {
            continue;
        };
        let said = girsa_app::shemos::written(&segment.text, shemos);
        let drawn = display::Shown::of(&said, pointing).text().to_string();
        out.push(Words {
            at: at.clone(),
            opening: girsa_app::enough::first_words(&drawn),
            said: drawn,
            address: girsa_app::sending::printed_address_in(
                &sefer.work,
                Some(sefer.sections()),
                &segment.id,
                style,
            ),
        });
    }
    Ok(out)
}

/// A sheet of paper: what the reader is standing in, ready to print.
///
/// > *Print the daf for the shiur.* — which, before this, meant export to
/// > `.docx`, open Word, and print from there.
///
/// **The section and not the sefer.** A siman, an amud, a perek — see
/// [`girsa_app::printing`] for why that is found from the address rather than
/// by counting lines. The whole sefer on paper is what the export is for.
///
/// The lines come back as ordinary [`Line`]s, which is the point: the reader's
/// corrections are already applied, the pointing is theirs, and the shemos are
/// written the way they asked — so what prints is what is on the screen, and
/// nothing has a second idea of what the sefer says.
#[tauri::command]
pub(crate) fn sefer_sheet(shared: tauri::State<'_, Shared>, at: String) -> Result<Sheet, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    // Held first, for the reason `copy` gives at length: this command is
    // deliberately synchronous, and a cache miss inside it is a whole work read
    // off disk on the thread that owns the message loop.
    hold(&shared, at.work())?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let (sefer, lexicon) = state.reading(at.work())?;
    // A sheet is the section, and there is no second answer to confuse it with:
    // the `whole:` flag used to name one here, but its only caller passed
    // `false` and its `true` printed *less* than `false` did.
    let (from, to) = girsa_app::printing::run_of(sefer, &at)
        .ok_or_else(|| refuse(Code::NoSuch, format!("{at} is not in this sefer")))?;
    let mut lines: Vec<Line> = sefer.segments[from..to]
        .iter()
        .map(|s| Line::of(sefer, s, pointing, shemos, style, lexicon))
        .collect();
    // The sheet's own from–to is read off the lines, so it has to be read
    // before the repeats are emptied — otherwise a sheet running from the
    // second mishnah of a perek is headed `משנה ב'` with no perek on it.
    let whole_address = |line: Option<&Line>| {
        line.map(|l| {
            if l.above.is_empty() {
                l.address.clone()
            } else {
                format!("{} {}", l.above, l.address)
            }
        })
        .unwrap_or_default()
    };
    let address = whole_address(lines.first());
    let to_address = whole_address(lines.last());
    girsa_app::view::only_when_it_changes(&mut lines);
    Ok(Sheet {
        title: girsa_app::printing::header(sefer),
        address,
        to_address,
        lines,
    })
}

/// A printable run of a sefer, with what has to be on the page beside it.
#[derive(Serialize)]
pub(crate) struct Sheet {
    /// The sefer, the edition and the terms — spec.md §13, on paper.
    title: Vec<String>,
    /// Where the sheet starts and where it ends, printed the reader's way.
    address: String,
    to_address: String,
    lines: Vec<Line>,
}

/// Set one chip on the **find bar**, which is not the panel's chip row.
///
/// Named rather than free-form, like `find_chip`: the window may choose among
/// the options the engine offered and may not invent one.
#[tauri::command(async)]
pub(crate) fn find_here_chip(
    shared: tauri::State<'_, Shared>,
    chip: String,
    key: String,
) -> Result<(), String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state
        .here_chips
        .choose(&chip, &key)
        .map_err(|e| e.to_string())
}

/// Find a phrase **inside one sefer** (`girsa_app::inside`).
///
/// The gesture every application has and this one did not. Not the search bar
/// narrowed to one work: the whole sefer is scanned, in reading order, and what
/// comes back is every place with the offsets the pane can highlight.
///
/// Scanned per keystroke rather than indexed, and that is a measured choice
/// rather than laziness: the largest sefer on the shelf is Mishnah Berurah at
/// 17,418 segments, and folding that many short strings is well under the
/// frame a reader would notice. An index would have to be invalidated by every
/// correction the reader makes, which is a second copy of a problem
/// `girsa-fix` already solved once.
#[tauri::command(async)]
pub(crate) fn sefer_find(
    shared: tauri::State<'_, Shared>,
    slug: String,
    query: String,
) -> Result<FoundHere, String> {
    // The find bar runs on every keystroke, so it is the *last* place that
    // should hold the state lock across an engine call. Chips out, guard
    // down, ask, then back in to draw. See `find` for the whole argument.
    let (chips, bar) = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        // The scope is this sefer, replaced on every ask. It is the one chip a
        // reader cannot set here, because setting it is what would stop this
        // being a find.
        state.here_chips.scope = girsa_search::scope::Scope::everything()
            .only([slug.clone()], girsa_app::inside::THIS_SEFER);
        let chips = state.here_chips.clone();
        if query.trim().is_empty() {
            // The same rule the panel has: an empty box is not a refusal. The
            // bar asks with `""` to draw its chip row before anything is typed.
            return Ok(FoundHere {
                places: Vec::new(),
                total: 0,
                chips: chips.row(),
                refused: None,
            });
        }
        let Some(bar) = state.bar.clone() else {
            let why = state.no_search();
            return Ok(FoundHere {
                places: Vec::new(),
                total: 0,
                chips: chips.row(),
                refused: Some(why),
            });
        };
        (chips, bar)
    };
    let answer = bar.ask(
        &query,
        &chips,
        Paging {
            from: 0,
            size: girsa_app::inside::MOST,
        },
        &girsa_ref::resolve::Context::default(),
    );
    // **The engine says which segments and which words; `inside` says where
    // those words are on the drawn page.** Two questions with two right
    // answers: the engine's marks are byte ranges into the text it indexed, and
    // the pane highlights characters of the text it drew — which differ by
    // every mark, every tag and every shem the reader asked to have rewritten.
    let (hits, refused) = match answer {
        Answer::Segments { results, .. } => (
            results
                .hits
                .iter()
                .map(|hit| (hit.id.clone(), marked(&results.marker, hit)))
                .collect::<Vec<_>>(),
            None,
        ),
        Answer::Refused(why) => (Vec::new(), Some(why)),
        // A mareh makom typed at a find bar. The bar is *inside one sefer* and
        // a citation is a jump somewhere else, so nothing is found — and the
        // search panel, which is where a citation belongs, is one key away.
        Answer::Cited(_) => (Vec::new(), None),
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let style = state.session.cite;
    let (sefer, _) = state.reading(&slug)?;
    let found = girsa_app::inside::where_marked(sefer, &hits, pointing, shemos, style);
    Ok(FoundHere {
        places: found.places,
        total: found.total,
        chips: chips.row(),
        refused,
    })
}

/// What a find inside one sefer found, with the row of options it ran under.
#[derive(Serialize)]
pub(crate) struct FoundHere {
    places: Vec<girsa_app::inside::Found>,
    total: usize,
    /// The chip row, so the bar draws what the engine will do rather than what
    /// it last drew.
    chips: Vec<Chip>,
    /// A refusal, in the engine's own words — a regex that will not compile,
    /// an index that is not there.
    refused: Option<String>,
}
