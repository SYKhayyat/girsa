//! The transmission chain (spec.md §8, BUILDER.md W28).
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use std::sync::Arc;

use girsa_app::Shelf;
use girsa_corpus::segment::SegmentId;

use crate::{Shared, State};

/// Which way the walk is not able to go without dates.
fn no_timeline() -> String {
    girsa_app::trouble::refuse(
        girsa_app::trouble::Code::NoSuch,
        "the catalogue could not be read, so nothing here knows when any sefer was written — \
         and which way a link points is a question about dates",
    )
}

/// What a chain walk needs out of the state, so that the walk itself can happen
/// with the state lock let go.
///
/// # Why this exists
///
/// A depth-12 walk reads `edges.jsonl` shards off disk — Berakhot's alone is
/// 3.4 MB and 21,065 rows — and it was **measured at 2,443 ms holding the
/// global state lock, during which a 3 ms scroll took 2,423 ms**. That is the
/// defect `find` was fixed for, in the command that pays the most for it.
///
/// Everything below is either an `Arc` or a clone of something small. The
/// timeline is a map of one row per work in the catalogue, which is a fraction
/// of a millisecond against the seconds it buys.
///
/// # The cache is taken last
///
/// `state.chains` leaves the state by `mem::take`, and it used to leave it
/// *before* the two fallible reads below. Any `?` between the take and the put
/// back — no shelf, no timeline, a poisoned lock — returned from the command
/// with `state.chains` left as `default()`, silently discarding every shard
/// read so far. Harmless in effect and exactly the shape that becomes a real
/// bug the first time somebody adds a fallible line to the block. Taking it
/// after everything that can fail has failed is a fix that cannot rot.
struct ForAWalk {
    shelf: Arc<std::sync::RwLock<Shelf>>,
    timeline: girsa_corpus::era::Timeline,
    language: girsa_app::session::Language,
    cite: girsa_cite::CiteStyle,
    /// Where the reader stands, resolved from the open sefer.
    ///
    /// The walk's first hop is a live segment with a shelf, so it is resolved
    /// by `Standing` exactly as the links panel resolves its own list — and a
    /// `Standing` is built out of the open sefer's live segments and redirect
    /// table, so it has to be taken while the state lock is held, with the
    /// rest of the snapshot.
    standing: girsa_corpus::standing::Standing,
    held: girsa_link::chain::Cache,
}

impl ForAWalk {
    /// Take it, and let the state lock go.
    fn taken(shared: &tauri::State<'_, Shared>, at: &SegmentId) -> Result<Self, String> {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?.clone();
        let timeline = state.timeline.clone().ok_or_else(no_timeline)?;
        let language = state.session.language;
        let cite = state.session.cite;
        let standing = state.sefer(at.work())?.standing(at);
        // Last. See the note above.
        let held = std::mem::take(&mut state.chains);
        Ok(Self {
            shelf,
            timeline,
            language,
            cite,
            standing,
            held,
        })
    }

    /// Put the cache back, whether the walk went well or not.
    ///
    /// A walk that failed still read shards, and throwing them away is the
    /// second half of the `mem::take` finding.
    fn put_back(shared: &tauri::State<'_, Shared>, kept: girsa_link::chain::Cache) {
        if let Ok(mut state) = shared.lock() {
            state.chains = kept;
        }
    }
}

/// How a line became halacha, or where a ruling came from (spec.md §8.1, §8.2).
///
/// The walk is `girsa-link`'s and is the same one `girsa-chain` prints on a
/// terminal. What this adds is nothing: if the panel and the tool could
/// disagree about which hops are real, they would be two answers to one
/// question, and the shape of the answer is the whole claim.
#[tauri::command(async)]
pub(crate) fn chain_walk(
    shared: tauri::State<'_, Shared>,
    at: String,
    direction: String,
    depth: Option<usize>,
) -> Result<girsa_app::chaining::Chain, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    let direction = match direction.as_str() {
        "forward" => girsa_link::chain::Direction::Forward,
        "back" => girsa_link::chain::Direction::Back,
        other => {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("no such direction: {other}"),
            ))
        }
    };
    // Everything the walk needs, and then the state lock is gone. The cache is
    // carried through so the reader's two clicks from one place are still one
    // reading of the shards; see `ForAWalk`.
    let taken = ForAWalk::taken(&shared, &at)?;
    // The ceiling is `chain`'s (`Limits::DEEPEST`) and not this file's — it
    // used to be a private 12 here while the terminal tool clamped to nothing.
    let limits = girsa_link::chain::Limits::with_depth(depth);
    let (chain, kept) = {
        let shelf = taken.shelf.read().map_err(|_| State::poisoned())?;
        let names =
            girsa_app::Names::new(&shelf, Some(&taken.timeline), taken.language, taken.cite);
        let mut graph = girsa_link::chain::Graph::resuming(
            shelf.root(),
            &taken.timeline,
            shelf.repairs(),
            taken.held,
        );
        let chain =
            girsa_app::chaining::walk(&mut graph, &names, &at, direction, limits, &taken.standing);
        (chain, graph.into_cache())
    };
    ForAWalk::put_back(&shared, kept);
    Ok(chain)
}

/// Where two rishonim read one gemara apart (spec.md §8.6).
#[tauri::command(async)]
pub(crate) fn chain_forks(
    shared: tauri::State<'_, Shared>,
    at: String,
) -> Result<girsa_app::chaining::Forked, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;
    // The same walk, so the same arrangement. See `ForAWalk`.
    let taken = ForAWalk::taken(&shared, &at)?;
    let (forked, kept) = {
        let shelf = taken.shelf.read().map_err(|_| State::poisoned())?;
        let names =
            girsa_app::Names::new(&shelf, Some(&taken.timeline), taken.language, taken.cite);
        let mut graph = girsa_link::chain::Graph::resuming(
            shelf.root(),
            &taken.timeline,
            shelf.repairs(),
            taken.held,
        );
        let forked = girsa_app::chaining::forked(
            &mut graph,
            &names,
            &at,
            girsa_link::chain::Limits::default(),
            &taken.standing,
        );
        (forked, graph.into_cache())
    };
    ForAWalk::put_back(&shared, kept);
    Ok(forked)
}
