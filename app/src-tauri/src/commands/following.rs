//! A pane moved. Record it, and say where the panes following it have to go.
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use girsa_app::beside::Beside;
use girsa_app::view::Move;
use girsa_app::workspace::PaneId;
use girsa_corpus::segment::SegmentId;

use crate::{hold, Shared, State};

/// A pane moved. Record it, and say where the panes following it have to go.
///
/// This is W9's acceptance in one function. The answer for each follower is a
/// [`Place`], which is three-valued on purpose: somewhere, *nowhere on this
/// line*, or *nothing relates these two seforim*. The window shows the second
/// and third rather than moving the column to something near.
///
/// The seforim are read **before** the state lock is taken for the placement —
/// [`hold`], the same doctrine as `open_sefer`, which this command is the
/// tenth instance of needing: every scroll event lands here, a cache miss is a
/// tens-of-megabytes read, and holding the guard across them queued every other
/// command in the window behind a mouse wheel.
#[tauri::command(async)]
pub(crate) fn moved(
    shared: tauri::State<'_, Shared>,
    pane: PaneId,
    at: String,
) -> Result<Vec<Move>, String> {
    let at: SegmentId = at.parse().map_err(|e| format!("{e}"))?;

    // One: under the lock — record the move and work out who cares.
    let (leader_slug, root, wanted) = {
        let mut state = shared.lock().map_err(|_| State::poisoned())?;
        state.session.remember(at.clone());
        let followers = state.session.workspace.moved(pane, at.clone());
        if followers.is_empty() {
            // `save_scroll`, not `save`: this is a scroll position, and
            // `Session::save` writes the whole workspace plus the remembered
            // place of every sefer ever opened.
            state.save_scroll();
            return Ok(Vec::new());
        }
        let leader_slug = at.work().to_string();
        let root = {
            let shelf = state.shelf()?;
            shelf.root().to_path_buf()
        };
        let wanted: Vec<(PaneId, String)> = followers
            .iter()
            .filter_map(|id| {
                state
                    .session
                    .workspace
                    .pane(*id)
                    .map(|p| (*id, p.slug.clone()))
            })
            .collect();
        (leader_slug, root, wanted)
    };

    // Two: no state lock — the leader and every follower are read into memory,
    // each paying only a read lock on the shelf.
    hold(&shared, &leader_slug)?;
    for (_, slug) in &wanted {
        hold(&shared, slug)?;
    }

    // Three: under the lock again — every `sefer` below is now a cache hit,
    // and nothing here touches the disk.
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    state.sefer(&leader_slug)?;
    for (_, slug) in &wanted {
        state.sefer(slug)?;
    }

    let mut moves = Vec::new();
    let mut alongside: Vec<(PaneId, String)> = Vec::new();
    for (id, slug) in wanted {
        let Some(follower) = state.open.peek(&slug) else {
            continue;
        };
        // A scan follows the sefer beside it by turning to the page the daf is
        // printed on — but only where the reader has said it is a scan **of**
        // that sefer (W25). Which `Place` that is, and which `Relation`, are
        // `Beside`'s to decide: this block used to work both out by hand and
        // synthesise `Relation::Declared { follower_is_commentary: false }` out
        // of nothing, with `Beside::between` reached only in the `else` below —
        // so a scan open beside a Gemara, the case W9 was accepted on, never
        // touched the tested path.
        if let Ok(shelf) = state.shelf() {
            if let Some(scan) = girsa_app::scan_of(&shelf, follower) {
                let joined = girsa_app::Joined::over_scan(scan, &leader_slug);
                let beside = Beside::over(follower, &joined);
                moves.push(Move {
                    pane: id,
                    place: beside.place(&at),
                    relation: beside.relation(),
                    page: beside.page(&at),
                });
                continue;
            }
        }
        alongside.push((id, slug));
    }

    // The joins, worked out once per pair rather than once per scroll event.
    // A second pass because filling the cache borrows `state` mutably while the
    // loop above is holding two seforim out of it.
    for (_, slug) in &alongside {
        state.join(&leader_slug, slug, &root);
    }
    for (id, slug) in alongside {
        // Two immutable borrows of two different fields, which is why the
        // cache is filled above rather than here.
        let (Some(joined), Some(follower)) = (
            state.joined.get(&(leader_slug.clone(), slug.clone())),
            state.open.peek(&slug),
        ) else {
            continue;
        };
        let beside = Beside::over(follower, joined);
        moves.push(Move {
            pane: id,
            place: beside.place(&at),
            relation: beside.relation(),
            page: None,
        });
    }
    state.save_scroll();
    Ok(moves)
}
