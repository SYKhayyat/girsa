//! The mefarshim on the line (W43).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use crate::{hold, hold_marks, Shared, State};
use girsa_app::display;
use girsa_app::trouble::{refuse, Code};
use girsa_app::view::Card;
use girsa_app::view::Comments;
use girsa_app::view::Line;
use girsa_app::view::Mefarshim;
use girsa_app::view::Said;
use girsa_app::view::Text;
use girsa_corpus::segment::SegmentId;

//
// Two ways to open a commentary, and they answer different questions. `beside`
// puts one sefer in a column and keeps it in step: *Rashi down the side of the
// Gemara*. This is the other one: *of the six mefarshim I follow, which said
// something about this line, and what?* Otzaria's model, and the reason the
// split is untouched by any of it.

/// The mefarshim on one sefer, and what the reader has ticked.
#[tauri::command(async)]
pub(crate) fn mefarshim(
    shared: tauri::State<'_, Shared>,
    slug: String,
) -> Result<Mefarshim, String> {
    hold_marks(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    mefarshim_of(&mut state, &slug)
}

/// The same, for a caller that already holds the lock.
///
/// # Why ticking goes through here too
///
/// `choose_mefaresh` answered with the **marked lines only**, and the window
/// patched its own copy of the rest: it flipped `chosen` inside `works` and
/// counted that array to decide whether a click on a line means anything. But
/// the list the reader is ticking in is `listed`, which also carries the seforim
/// running alongside and the mefarshim the graph knows and the catalogue does
/// not — tick one of those and `works` never mentioned it, so the count stayed
/// zero and clicking a line did nothing at all. The reader's ninth bug:
/// *"checking off a mefarsh does not open it when its line is clicked."*
///
/// One answer, from the one place that builds it, and the window draws what it
/// is given.
fn mefarshim_of(state: &mut State, slug: &str) -> Result<Mefarshim, String> {
    let slug = slug.to_string();
    let chosen: Vec<String> = state.session.chosen_for(&slug).to_vec();
    // The list comes back in name order, in the language the window is in — see
    // `mefarshim::listed`. Read before the shelf is borrowed; `Language` is
    // `Copy`, so this costs nothing and keeps the borrow checker out of it.
    let language = state.session.language;
    // The pairs the reader made himself, read both ways round — see
    // `Session::alongside`, and the Shulchan Arukh HaRav, which is the case that
    // needs them.
    let mine = state.session.alongside_of(&slug);
    // Filled first, and then **borrowed**. This used to be
    // `state.marks(&slug)?.clone()` — a copy of the whole marks table on every
    // call, which is the table the cache one field up exists so as not to read
    // twice. The clone was there because `marks` takes `&mut self` to fill the
    // cache and `shelf` borrows it again; filling it and then splitting the
    // fields by hand is the same arrangement the lane commands use, and it
    // costs a lookup instead of a table.
    state.marks(&slug)?;
    let no_marks = || "no mefarshim read".to_string();
    let State { marks, shelf, .. } = &mut *state;
    let marks = marks.get(&slug).ok_or_else(no_marks)?;
    let shelf = shelf
        .as_ref()
        .ok_or_else(|| refuse(Code::NoShelf, "there is no shelf here"))?
        .read()
        .map_err(|_| State::poisoned())?;
    Ok(Mefarshim::of(
        &shelf, marks, &slug, &chosen, language, &mine,
    ))
}

/// Say — or unsay — that two seforim keep the same order (A6).
///
/// > *"1, plus the user can add."*
///
/// `taxonomy::Keeping` settles this from the graph and is only as good as the
/// links. The Shulchan Arukh HaRav is written on Orach Chayim's simanim and the
/// graph joins two of its 505 to their own number, so the corpus cannot say it
/// and this application will not say it for the corpus. The reader can.
///
/// Answers with the whole list, the same as `choose_mefaresh`: the window used
/// to patch its own copy after a tick and that is how a list drifts from what
/// Rust holds.
#[tauri::command(async)]
pub(crate) fn pair_alongside(
    shared: tauri::State<'_, Shared>,
    slug: String,
    work: String,
    on: bool,
) -> Result<Mefarshim, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.pair(&slug, &work, on);
    state.save();
    mefarshim_of(&mut state, &slug)
}

/// Tick or untick one mefaresh, and answer with the whole list as it stands
/// now.
///
/// The **whole** list, not just the marked lines: see [`mefarshim_of`].
#[tauri::command(async)]
pub(crate) fn choose_mefaresh(
    shared: tauri::State<'_, Shared>,
    slug: String,
    work: String,
    on: bool,
) -> Result<Mefarshim, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.session.choose(&slug, &work, on);
    state.save();
    mefarshim_of(&mut state, &slug)
}

/// Tick the mefarshim printed **on the page** with this sefer, in one gesture.
///
/// Rashi and Tosfos on a daf, Onkelos beside a Chumash, the Bartenura under a
/// Mishnah — see [`girsa_app::mefarshim::the_usual`], which is where the list
/// is decided and where the argument for leaving the alphabetical order alone
/// is written down.
///
/// One command and not a loop of `choose_mefaresh` in the window: each of those
/// re-weaves the whole list and hands it back, so three ticks would be three
/// round trips and three redraws for one click.
///
/// **Ticks and never unticks.** A reader who has already opened Rashi and
/// presses this wants Tosfos as well, not Rashi shut.
#[tauri::command(async)]
pub(crate) fn choose_the_usual(
    shared: tauri::State<'_, Shared>,
    slug: String,
) -> Result<Mefarshim, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let usual = mefarshim_of(&mut state, &slug)?
        .usual
        .into_iter()
        .map(|m| m.slug)
        .collect::<Vec<_>>();
    for work in usual {
        state.session.choose(&slug, &work, true);
    }
    state.save();
    mefarshim_of(&mut state, &slug)
}

/// Click a line: read the ticked mefarshim on it.
#[tauri::command(async)]
pub(crate) fn mefarshim_at(
    shared: tauri::State<'_, Shared>,
    slug: String,
    at: String,
) -> Result<Comments, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    hold_marks(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    let chosen: Vec<String> = state.session.chosen_for(&slug).to_vec();

    let marks = state.marks(&slug)?;
    let spoken = marks.said(&at, &chosen);
    let mine = marks.on(&at, &chosen).works.len();
    let all = marks.on(&at, &marks.commentators()).works.len();
    let others = all > mine;

    let mut said: Vec<Said> = Vec::new();
    for one in spoken {
        // The commentary itself, read like any other sefer — the same cache, the
        // same corrections, the same nikud toggle. A comment shown by a
        // different path from the pane would be a second idea of what the text
        // says.
        let lines: Vec<Line> = {
            let (sefer, lexicon) = state.reading(&one.work)?;
            let Some(first) = sefer.position_of(&one.at) else {
                // The graph points at a segment this sefer does not have.
                // Skipped rather than reported as an empty comment: it is a fact
                // about the link, and W23's panel is where a bad link is
                // repaired.
                continue;
            };
            let last = one
                .to
                .as_ref()
                .and_then(|to| sefer.position_of(to))
                .map_or(first, |to| to.max(first));
            let mut lines: Vec<Line> = sefer.segments[first..=last]
                .iter()
                .map(|s| Line::of(sefer, s, pointing, shemos, style, lexicon))
                .collect();
            girsa_app::view::only_when_it_changes(&mut lines);
            lines
        };
        // Named out of the shelf, in a scope of its own: `state.reading` above
        // takes `&mut state`, so a guard that spanned the loop body would be a
        // borrow error — which is the borrow checker saying the same thing
        // this file says in prose, that a lock is held for the line it is
        // needed on and not for the block around it.
        let (he_title, en_title, address) = {
            let shelf = state.shelf()?;
            shelf.work(&one.work).map_or_else(
                || (one.work.clone(), one.work.clone(), one.at.address()),
                |w| {
                    (
                        w.he_title.clone(),
                        w.en_title.clone(),
                        // `רש״י על ברכות 2a:8:1` was the header on the release
                        // build. The address a mefaresh's block carries is the
                        // same kind of thing as the one in the margin, and it
                        // goes through the same formatter.
                        girsa_app::sending::printed_address(w, &one.at, style),
                    )
                },
            )
        };
        said.push(Said {
            he_title,
            en_title,
            address,
            work: one.work,
            lines,
        });
    }
    Ok(Comments { said, others })
}

/// Read a sefer for a pane.
///
/// The nikud toggle is applied **here**, by [`girsa_app::display`], rather than
/// in the window. There is one implementation of what a mark is (`girsa-hebrew`,
/// W2) and a second one written in TypeScript would drift from it — and the
/// place it would show is a reader turning nikud off and finding one word in
/// forty still pointed.
///
/// The window size is `girsa_app::view::WINDOW`, beside the `Text` it sizes —
/// it used to live here, a policy number in the bridge with no test and no
/// neighbour.

#[tauri::command(async)]
pub(crate) fn open_sefer(shared: tauri::State<'_, Shared>, slug: String) -> Result<Text, String> {
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    // Where the reader was, so the window opens around it rather than at the
    // top and then jumping. `where_i_was` is the same memory `Session` keeps for
    // every sefer ever opened (W9).
    let left = state.session.where_i_was(&slug).cloned();
    // Whether anything in the sefer is pointed. Over the whole of it, because
    // the answer is about the sefer and not about the window a reader happens
    // to be looking at — a Chumash whose first four hundred segments are bare
    // still has nikud. Asked once per read of the work rather than once per
    // open; see `State::pointed`.
    let has_nikud = match state.pointed.get(&slug) {
        Some(known) => *known,
        None => {
            let found = {
                let sefer = state.sefer(&slug)?;
                sefer.segments.iter().any(|s| display::has_marks(&s.text))
            };
            state.pointed.insert(slug.clone(), found);
            found
        }
    };
    let (sefer, lexicon) = state.reading(&slug)?;
    let total = sefer.segments.len();
    // Where the reader is: where they were last time if that segment is still
    // in this sefer, else its first line. Never anything else — see `Text::at`,
    // which is what carries the answer out to the pane.
    let at = left.and_then(|id| sefer.position_of(&id)).unwrap_or(0);
    let going = sefer
        .segments
        .get(at)
        .map(|s| s.id.to_string())
        .unwrap_or_default();
    let from = at
        .saturating_sub(girsa_app::view::WINDOW / 2)
        .min(total.saturating_sub(1));
    let to = (from + girsa_app::view::WINDOW).min(total);
    let mut lines: Vec<Line> = sefer.segments[from..to]
        .iter()
        .map(|s| Line::of(sefer, s, pointing, shemos, style, lexicon))
        .collect();
    girsa_app::view::only_when_it_changes(&mut lines);
    Ok(Text {
        work: Card::of(&sefer.work),
        lines,
        from,
        at: going,
        total,
        has_nikud,
    })
}

/// More of a sefer, for a pane that has scrolled to the edge of what it holds.
///
/// Clamped rather than refused: a pane asking past the end is a reader at the
/// end of the sefer, and the honest answer is the last lines rather than an
/// error.
#[tauri::command(async)]
pub(crate) fn sefer_lines(
    shared: tauri::State<'_, Shared>,
    slug: String,
    from: usize,
    count: usize,
) -> Result<Vec<Line>, String> {
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    // How a place is printed, from the reader's own setting — one formatter
    // for the margin and for the citation. See `sending::printed_address`.
    let style = state.session.cite;
    let (sefer, lexicon) = state.reading(&slug)?;
    let from = from.min(sefer.segments.len());
    let to = from.saturating_add(count).min(sefer.segments.len());
    let mut lines: Vec<Line> = sefer.segments[from..to]
        .iter()
        .map(|s| Line::of(sefer, s, pointing, shemos, style, lexicon))
        .collect();
    girsa_app::view::only_when_it_changes(&mut lines);
    Ok(lines)
}

/// The table of contents of a sefer (A3).
///
/// > *"there should be a table of contents on the side for each sefer, so you
/// > can jump around."*
///
/// Built from the segments' own addresses rather than by scanning the text for
/// headings — see `girsa_app::contents`, which holds the argument. Answered per
/// sefer and per open, not per keystroke: the filter box in the window filters
/// the list it already has, the way Otzaria's does.
#[tauri::command(async)]
pub(crate) fn sefer_contents(
    shared: tauri::State<'_, Shared>,
    slug: String,
) -> Result<Vec<girsa_app::contents::Entry>, String> {
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let style = state.session.cite;
    let (sefer, _) = state.reading(&slug)?;
    Ok(girsa_app::contents::of(sefer, style))
}

/// Where a segment sits in its sefer, counted from the start.
///
/// What a pane asks when it is told to go to a line it has not loaded — a search
/// hit, a link, a mefaresh's place. `None` is *this sefer does not have that
/// segment*, which is a real answer and not an error: a link can point at a
/// place a re-import moved, and W23's panel is where that gets repaired.
#[tauri::command(async)]
pub(crate) fn sefer_index_of(
    shared: tauri::State<'_, Shared>,
    slug: String,
    at: String,
) -> Result<Option<usize>, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let sefer = state.sefer(&slug)?;
    Ok(sefer.position_of(&at))
}

/// The same question about many segments at once.
///
/// # Why there are two of these
///
/// [`sefer_index_of`] answers about one place, and its three callers in the
/// window are each a single reader action — clicking a search hit, following a
/// link — where one round trip is one round trip.
///
/// *Next highlight* is not that. `goToNextPlace` asked for every mark in the
/// sefer, one at a time, on every press of the key: a reader with 200
/// highlights in Mishnah Berurah paid 200 serialised IPC calls to answer
/// *which is the next one*, and paid them again on the next press. The marks
/// that miss the pane's own table are exactly the ones outside the window of
/// lines it is holding, which on a long sefer is most of them.
///
/// The answers come back **in the order they were asked**, one slot per
/// question, so a caller can zip them against its own list. `None` in a slot is
/// *this sefer does not have that segment*, the same real answer the single
/// version gives — and an id that will not parse is `None` too rather than a
/// refusal for the whole batch, because one bad mark should not cost the
/// reader the other 199.
///
/// `linksview.ts` already had the shape (`api.linkWords` takes an array); this
/// is the same idea one file over.
///
/// # Errors
///
/// If there is no shelf, or the sefer will not read.
#[tauri::command(async)]
pub(crate) fn sefer_indices_of(
    shared: tauri::State<'_, Shared>,
    slug: String,
    ats: Vec<String>,
) -> Result<Vec<Option<usize>>, String> {
    hold(&shared, &slug)?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let sefer = state.sefer(&slug)?;
    Ok(ats
        .iter()
        .map(|at| {
            at.parse::<SegmentId>()
                .ok()
                .and_then(|at| sefer.position_of(&at))
        })
        .collect())
}

// The one wire type `copy`, `scan_copy` and `buffer_copy` all answer with.
