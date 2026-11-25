//! The links on a line, and repairing them (spec.md §8.3, W23).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use girsa_app::display;
use girsa_app::trouble::{refuse, Code};
use girsa_app::view::{LensRow, LinkRow, Links};
use girsa_corpus::segment::SegmentId;

use crate::{hold, personal_of, Shared, State};

/// The links on a line — through a lens, and against a highlight (W23, W24).
///
/// `lens` is one of yours by key, or nothing for all of them. `from_char`/
/// `to_char` are a highlight in the line, and then only the links **not known
/// to be about other words** come back (spec.md §8.4).
#[tauri::command(async)]
pub(crate) fn links(
    shared: tauri::State<'_, Shared>,
    at: String,
    lens: Option<String>,
    from_char: Option<usize>,
    to_char: Option<usize>,
) -> Result<Links, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    // **The one file read in this command, done before the guard is taken.**
    // `Lenses::load` opens `lenses.json` off disk, and this command is asked on
    // every click on a line — so that read sat on the thread the scroll handler
    // was waiting for, once per click, for a file that changes when the reader
    // edits a lens and at no other time. `find` above makes this argument at
    // length; this is it applied where it was not.
    //
    // What is still under the guard is `girsa_app::touching`, which is a walk
    // over the repair layer already in memory. That is CPU rather than I/O, it
    // needs the shelf the state guard is holding, and taking it apart would mean
    // taking the shelf lock after the state lock is dropped — the reverse of the
    // order every other command uses, which is a deadlock and not a saving.
    let personal = personal_of(&shared)?;
    let (lenses, trouble) = girsa_app::Lenses::load(&personal);
    if let Some(said) = trouble {
        eprintln!("{said}");
    }
    // The sefer this line is in, read in **before** the guard: on a cache miss
    // `State::sefer` is a whole work off disk, and `hold` is the call written
    // for exactly that (see `open_sefer`).
    hold(&shared, at.work())?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let pointing = state.session.pointing;
    let shemos = state.session.shemos;
    let lens = lens.filter(|key| !key.is_empty());

    // The line itself, as the pane drew it, because a span is in those
    // characters (W20's two coordinate systems, again) — **and** as it is on
    // disk, because the mined anchors' offsets are positions in *that* text.
    // For any segment nobody has corrected the two are the same string; for one
    // you have corrected they are not, and reading a mined offset against the
    // corrected text is a highlight on the wrong letters (W20's rule, reached
    // from the other side).
    // …and, from the same read, every name these words have carried, so an edge
    // stored under the name this place had before a corpus update still finds
    // it (see `girsa_corpus::standing`).
    let (base, printed, anchors, standing) = {
        let sefer = state.sefer(at.work())?;
        let nth = sefer
            .position_of(&at)
            .ok_or_else(|| refuse(Code::NoSuch, format!("{at} is not in this sefer")))?;
        // The anchors travel with the text. They are the segment's own statement
        // of where its commentaries attach, and reading them costs nothing that
        // reading the text did not already cost.
        let segment = sefer.segments.get(nth);
        let text = segment.map(|s| s.text.clone()).unwrap_or_default();
        let anchors = segment.map(|s| s.anchors.clone()).unwrap_or_default();
        let printed = sefer.as_printed(&at).to_string();
        (text, printed, anchors, sefer.standing(&at))
    };

    let language = state.session.language;
    let shelf = state.shelf()?;
    let touching = girsa_app::touching(&shelf, shelf.repairs(), &standing);
    let mut links = touching.links;

    // The words each link is about, where anything says — and the far end's
    // text is only consulted for seforim that are **already open**.
    for link in &mut links {
        let far = state.open.peek(&link.work).map(AsRef::as_ref);
        link.span =
            girsa_app::links::span_on(link, &standing, &base, &printed, &anchors, far, pointing);
    }
    if let (Some(from), Some(to)) = (from_char, to_char) {
        if from < to {
            links = girsa_app::links::touching_words(links, from..to);
        }
    }
    if let Some(key) = lens.as_deref() {
        links = lenses.through(key, &shelf, links);
    }

    Ok(Links {
        links: links
            .iter()
            .map(|l| LinkRow::of(l, language, first_words(&state, l, pointing, shemos)))
            .collect(),
        incoming_unknown: touching.incoming_unknown,
        incoming_damaged: touching.incoming_damaged,
        types: girsa_app::links::kinds(),
        lenses: lenses
            .lenses
            .iter()
            .map(|(key, lens)| LensRow {
                key: key.clone(),
                title: lens.title.clone(),
                types: lens.types.clone(),
                eras: lens.eras.clone(),
                at_least: lens.at_least,
                mine: lens.mine,
            })
            .collect(),
        lens,
    })
}

/// Pin a link onto the words it is about (spec.md §8.4).
#[tauri::command(async)]
pub(crate) fn link_pin(
    shared: tauri::State<'_, Shared>,
    edge: String,
    at: String,
    from_char: usize,
    to_char: usize,
) -> Result<(), String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    if from_char >= to_char {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::NothingChosen,
            "nothing is selected",
        ));
    }
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // The words the selection covers, so the pin is a place and not an offset.
    //
    // **Read here, from the text the pane drew**, which is the only text these
    // offsets are a position in. A pin is the one placement in the panel a
    // reader vouched for, and storing the numbers without the letters is what
    // made it the one that could not follow the line when a correction above it
    // moved everything — see `girsa_corpus::span::Anchored`.
    let was: String = {
        let sefer = state.sefer(at.work())?;
        let nth = sefer.position_of(&at).unwrap_or(0);
        let drawn = sefer
            .segments
            .get(nth)
            .map(|s| s.text.as_str())
            .unwrap_or("");
        drawn
            .chars()
            .skip(from_char)
            .take(to_char.saturating_sub(from_char))
            .collect()
    };
    // Every work the chain holds went through the repair layer as it stood
    // when it was read, and this is about to change it. See `State::chains`.
    state.chains = girsa_link::chain::Cache::default();
    let mut shelf = state.shelf_mut()?;
    let who = girsa_app::who();
    shelf
        .repairs_mut()
        .pin_named(
            &edge,
            &girsa_corpus::span::Anchored::new(at, from_char..to_char, was),
            &who,
        )
        .map_err(|e| e.to_string())
}

/// The first words at the other end of a link, where that sefer is already read
/// (W37).
///
/// **Already read only.** Same rule as `girsa_app::links::span_on`, and the same
/// reason it gives: a sidebar is not entitled to open forty seforim to decorate
/// itself. The rows without one still name the sefer and the place, which is what
/// every row said before this.
fn first_words(
    state: &State,
    link: &girsa_app::Link,
    pointing: girsa_app::session::Pointing,
    shemos: girsa_app::shemos::Shemos,
) -> Option<String> {
    let sefer = state.open.peek(&link.work)?;
    let nth = sefer.position_of(&link.other.from)?;
    let said = girsa_app::shemos::written(&sefer.segments.get(nth)?.text, shemos);
    let text = display::Shown::of(&said, pointing).text().to_string();
    Some(girsa_app::enough::first_words(&text))
}

/// Confirm, reject, retype, reanchor, or take it all back.
///
/// One command, because they are one thing: a statement about an edge, written
/// into your layer. Which statement is named rather than free-form — the window
/// may choose among what the engine offered and may not invent one.
#[tauri::command(async)]
pub(crate) fn link_repair(
    shared: tauri::State<'_, Shared>,
    edge: String,
    does: String,
    value: Option<String>,
) -> Result<(), String> {
    use girsa_link::repair::Verdict;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // Every work the chain holds went through the repair layer as it stood
    // when it was read, and this is about to change it. See `State::chains`.
    state.chains = girsa_link::chain::Cache::default();
    let mut shelf = state.shelf_mut()?;
    let who = girsa_app::who();
    let repairs = shelf.repairs_mut();
    match does.as_str() {
        "confirm" => repairs.judge_named(&edge, Verdict::Confirmed, &who),
        "reject" => repairs.judge_named(&edge, Verdict::Rejected, &who),
        "retype" => {
            let name = value.ok_or_else(|| refuse(Code::NoSuch, "which type?"))?;
            let edge_type = girsa_link::touching::type_named(&name)
                .ok_or_else(|| refuse(Code::NoSuch, format!("no such link type: {name}")))?;
            repairs.retype_named(&edge, edge_type, &who)
        }
        "undo" => {
            return repairs
                .undo_named(&edge)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such repair: {other}"),
            ))
        }
    }
    .map_err(|e| e.to_string())
}

/// Move a link onto the segment you are standing on.
///
/// The end being moved is named, because a link has two and moving the wrong
/// one silently is exactly the class of guess rule 6 forbids.
#[tauri::command(async)]
pub(crate) fn link_reanchor(
    shared: tauri::State<'_, Shared>,
    edge: String,
    end: String,
    to: String,
) -> Result<(), String> {
    let to: SegmentId = to.parse().map_err(|e| format!("{e}"))?;
    let (from_text, to_text) = edge
        .split_once(" → ")
        .ok_or_else(|| refuse(Code::NoSuch, "that is not an edge"))?;
    let (from_anchor, to_anchor) = (parse_anchor(from_text)?, parse_anchor(to_text)?);
    let (from_anchor, to_anchor) = match end.as_str() {
        "from" => (girsa_link::Anchor::point(to), to_anchor),
        "to" => (from_anchor, girsa_link::Anchor::point(to)),
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such end: {other}"),
            ))
        }
    };
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // Every work the chain holds went through the repair layer as it stood
    // when it was read, and this is about to change it. See `State::chains`.
    state.chains = girsa_link::chain::Cache::default();
    let mut shelf = state.shelf_mut()?;
    let who = girsa_app::who();
    shelf
        .repairs_mut()
        .reanchor_named(&edge, from_anchor, to_anchor, &who)
        .map_err(|e| e.to_string())
}

/// Draw a link by hand, from one place to another.
#[tauri::command(async)]
pub(crate) fn link_draw(
    shared: tauri::State<'_, Shared>,
    from: String,
    to: String,
    kind: String,
) -> Result<(), String> {
    use girsa_app::trouble::{refuse, Code};
    let from: SegmentId = from
        .parse()
        .map_err(|e| refuse(Code::NoSuch, format_args!("{e}")))?;
    let to: SegmentId = to
        .parse()
        .map_err(|e| refuse(Code::NoSuch, format_args!("{e}")))?;
    let edge_type = girsa_link::touching::type_named(&kind)
        .ok_or_else(|| refuse(Code::NoSuch, format_args!("no such link type: {kind}")))?;
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    // Every work the chain holds went through the repair layer as it stood
    // when it was read, and this is about to change it. See `State::chains`.
    state.chains = girsa_link::chain::Cache::default();
    let mut shelf = state.shelf_mut()?;
    let who = girsa_app::who();
    shelf
        .repairs_mut()
        .draw(
            girsa_link::Anchor::point(from),
            girsa_link::Anchor::point(to),
            edge_type,
            &who,
        )
        .map_err(|e| refuse(Code::ReadOnly, e))
}

fn parse_anchor(text: &str) -> Result<girsa_link::Anchor, String> {
    match text.split_once("-girsa:") {
        Some((from, to)) => Ok(girsa_link::Anchor::span(
            from.parse().map_err(|e| format!("{e}"))?,
            format!("girsa:{to}").parse().map_err(|e| format!("{e}"))?,
        )),
        None => Ok(girsa_link::Anchor::point(
            text.parse().map_err(|e| format!("{e}"))?,
        )),
    }
}
