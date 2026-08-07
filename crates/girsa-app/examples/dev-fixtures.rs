//! Write the app's JSON to static files, so the page can be opened in a
//! browser that is not the shell.
//!
//! ```sh
//! cargo run -p girsa-app --example dev-fixtures -- corpus app/public/dev
//! npm --prefix app run dev     # then open http://localhost:5174
//! ```
//!
//! BUILDER.md W9 has a trap on it: *Tauri uses Edge's engine on Windows and
//! Safari's on macOS. Test Hebrew-with-nikud rendering on both — a screenshot
//! from one OS is not evidence.* Building an installer for each is the real
//! answer and needs both machines. This is the cheap half of it: the same page,
//! the same CSS, the same menukad Gemara, in any browser on hand — which at
//! least catches the difference between two engines' idea of where a nikud
//! point sits.
//!
//! It writes **real text off the shelf**, not invented Hebrew. Made-up sample
//! data would look fine in exactly the cases the corpus does not.

// A tool that writes files and says what it wrote.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use girsa_app::view::{Card, Line, Move, Opening, Text};
use girsa_app::workspace::{Axis, Workspace};
use girsa_app::{display, Beside, Shelf};

/// Berakhot with Rashi beside it — W9's acceptance, as a page to look at.
const LEADER: &str = "bavli/berakhot";
const FOLLOWER: &str = "bavli/rashi-on-berakhot";

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().unwrap_or_else(|| "corpus".into()));
    let out = PathBuf::from(args.next().unwrap_or_else(|| "app/public/dev".into()));
    // The reader's own layer, wherever the app would keep it. A fixture run is
    // read-only and takes it as it finds it — including the seforim of yours
    // that are on the shelf, so the page shows the shelf the app shows.
    let personal =
        PathBuf::from(std::env::var("GIRSA_PERSONAL").unwrap_or_else(|_| "personal".to_string()));

    let shelf = match Shelf::open(&root, &personal) {
        Ok(shelf) => shelf,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("could not make {}: {e}", out.display());
        return std::process::ExitCode::FAILURE;
    }

    let (Ok(leader), Ok(follower)) = (shelf.read(LEADER), shelf.read(FOLLOWER)) else {
        eprintln!("{LEADER} and {FOLLOWER} both have to be on the shelf");
        return std::process::ExitCode::FAILURE;
    };

    // Two panes, the second following the first, which is what the app opens
    // as when a reader splits a Gemara and puts Rashi beside it. Built by
    // `Workspace` itself, so the layout tree in the fixture is the tree the
    // window makes rather than a hand-typed picture of one.
    let mut workspace = Workspace::default();
    let leader_pane = workspace.open_tab(LEADER, None);
    let follower_pane = workspace
        .split(leader_pane, Axis::Vertical, FOLLOWER, true)
        .unwrap_or(leader_pane);
    workspace.set_ratio(leader_pane, 550);

    // `girsa_app::view::Opening`, the real type.
    //
    // This was fifteen keys built with `serde_json::json!` in the shell, and
    // **nine** of them hand-typed here — and the comment that used to sit on
    // this block named five of the six that were missing, so the comment
    // documenting the drift had itself drifted. A field added to `Opening` now
    // fails to compile until it is answered here, which is the whole of the
    // argument for the DTOs living in this crate.
    let session = girsa_app::Session::default();
    let state = Opening {
        workspace,
        nikud: true,
        text_size: 100,
        positions: session.positions.clone(),
        works: shelf.works().len(),
        trouble: None,
        cite: session.cite,
        language: session.language,
        keys: girsa_app::keys::Bound::of(&session.keys).table().clone(),
        look: session.look.clone(),
        share_bounds: [
            girsa_app::workspace::SMALLEST_SHARE,
            girsa_app::workspace::LARGEST_SHARE,
        ],
        // The desk is the shell's loopback, and there is not one out here.
        pairing: Some("הכתיבה פועלת בחלון בלבד".to_string()),
        // Corrections are the shell's — they are written into your own layer,
        // and a page reading static files has none (W20).
        showing: session.showing,
        fixes: 0,
        suspects: 0,
    };
    write(&out.join("state.json"), &serde_json::json!(state));

    let cards: Vec<Card> = [LEADER, FOLLOWER, "mishnah-berakhot", "genesis"]
        .iter()
        .filter_map(|slug| shelf.work(slug))
        .map(Card::of)
        .collect();
    write(&out.join("recent.json"), &serde_json::json!(cards));

    // The shelf: the tree with its counts, and every shelf's seforim by key.
    // Written whole rather than sampled — the point of looking at this page in
    // a second browser is to see the real thing, and a shelf of four seforim
    // would not show what 2,141 under `תלמוד` does to a scrolling list.
    write(&out.join("tree.json"), &serde_json::json!(shelf.tree()));
    let mut by_shelf: BTreeMap<String, Vec<Card>> = BTreeMap::new();
    for work in shelf.works() {
        by_shelf
            .entry(girsa_app::taxonomy::shelf_key_of(work, shelf.arrangement()))
            .or_default()
            .push(Card::of(work));
    }
    write(&out.join("shelf.json"), &serde_json::json!(by_shelf));

    for sefer in [&leader, &follower] {
        // `Text`, `Card` and `Line`, the real types.
        //
        // This block used to build a **second** inline copy of a card — missing
        // `source` and `scan`, and emitting `"era": work.era`, the raw code,
        // where `card()` seventy lines below emitted `display::era_said(code)`.
        // Two hand-written copies of one shape inside one 202-line file,
        // disagreeing with each other about the value under a key they both
        // spelled the same way.
        let text = Text {
            work: Card::of(&sefer.work),
            has_nikud: sefer.segments.iter().any(|s| display::has_marks(&s.text)),
            lines: sefer
                .segments
                .iter()
                .map(|s| Line::of(sefer, s, true))
                .collect(),
        };
        write(
            &out.join(format!("text-{}.json", flatten(&sefer.work.slug))),
            &serde_json::json!(text),
        );

        let companions = shelf.companions(&sefer.work.slug);
        write(
            &out.join(format!("companions-{}.json", flatten(&sefer.work.slug))),
            &serde_json::json!(companions),
        );
    }

    // Every place the Rashi column goes, for every line of the Gemara —
    // precomputed, because a static file cannot call `Beside::place`.
    let beside = Beside::between(&leader, &follower, &root);
    let mut moves: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut somewhere = 0usize;
    for segment in &leader.segments {
        let place = beside.place(&segment.id);
        if place.is_somewhere() {
            somewhere += 1;
        }
        moves.insert(
            segment.id.to_string(),
            serde_json::json!([Move {
                // The follower's pane, which `Workspace::split` handed out
                // above — and the fixture used to type `2` because a hand-built
                // JSON object cannot be told it is wrong.
                pane: follower_pane,
                place,
                relation: beside.relation(),
                // Which page of a scan. The follower here is Rashi, not a scan.
                page: None,
            }]),
        );
    }
    write(&out.join("moves.json"), &serde_json::json!(moves));

    println!(
        "{} · {} lines of {}, {} of them with a Rashi",
        out.display(),
        leader.segments.len(),
        leader.work.he_title,
        somewhere,
    );
    std::process::ExitCode::SUCCESS
}

fn write(path: &Path, value: &serde_json::Value) {
    match serde_json::to_vec(value) {
        Ok(body) => {
            if let Err(e) = std::fs::write(path, body) {
                eprintln!("could not write {}: {e}", path.display());
            }
        }
        Err(e) => eprintln!("could not encode {}: {e}", path.display()),
    }
}

fn flatten(slug: &str) -> String {
    slug.replace('/', "_")
}
