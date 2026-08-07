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
    // as when a reader splits a Gemara and puts Rashi beside it.
    let state = serde_json::json!({
        "workspace": {
            "tabs": [{
                "layout": {
                    "kind": "split", "axis": "vertical", "ratio": 550,
                    "first": {"kind": "leaf", "pane": 1},
                    "second": {"kind": "leaf", "pane": 2},
                },
                "panes": [
                    {"id": 1, "slug": LEADER},
                    {"id": 2, "slug": FOLLOWER, "follows": 1},
                ],
                "focused": 1,
            }],
            "active": 0,
        },
        "nikud": true,
        "text_size": 100,
        // What a pane may be squeezed to. Written here **by hand**, from the
        // constants in `girsa_app::workspace`, because this example cannot
        // import the shell's DTOs — which is the wire-format finding, felt: this
        // object is already missing `language`, `keys`, `look`, `cite` and
        // `pairing`, and nothing anywhere said so.
        "share_bounds": [
            girsa_app::workspace::SMALLEST_SHARE,
            girsa_app::workspace::LARGEST_SHARE,
        ],
        "positions": {},
        "works": shelf.works().len(),
        "trouble": serde_json::Value::Null,
        // Corrections are the shell's — they are written into your own layer,
        // and a page reading static files has none (W20).
        "showing": "fixed",
        "fixes": 0,
    });
    write(&out.join("state.json"), &state);

    let cards: Vec<serde_json::Value> = [LEADER, FOLLOWER, "mishnah-berakhot", "genesis"]
        .iter()
        .filter_map(|slug| shelf.work(slug))
        .map(card)
        .collect();
    write(&out.join("recent.json"), &serde_json::json!(cards));

    // The shelf: the tree with its counts, and every shelf's seforim by key.
    // Written whole rather than sampled — the point of looking at this page in
    // a second browser is to see the real thing, and a shelf of four seforim
    // would not show what 2,141 under `תלמוד` does to a scrolling list.
    write(&out.join("tree.json"), &serde_json::json!(shelf.tree()));
    let mut by_shelf: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for work in shelf.works() {
        by_shelf
            .entry(girsa_app::taxonomy::shelf_key_of(work, shelf.arrangement()))
            .or_default()
            .push(card(work));
    }
    write(&out.join("shelf.json"), &serde_json::json!(by_shelf));

    for sefer in [&leader, &follower] {
        let text = serde_json::json!({
            "work": {
                "slug": sefer.work.slug, "he_title": sefer.work.he_title,
                "en_title": sefer.work.en_title, "categories": sefer.work.categories,
                "author": sefer.work.author, "era": sefer.work.era,
            },
            "has_nikud": sefer.segments.iter().any(|s| display::has_marks(&s.text)),
            "lines": sefer.segments.iter().map(|s| serde_json::json!({
                "id": s.id.to_string(),
                "address": s.id.path().join(":"),
                "kind": s.kind.as_str(),
                "runs": display::runs(&s.text),
            })).collect::<Vec<_>>(),
        });
        write(
            &out.join(format!("text-{}.json", flatten(&sefer.work.slug))),
            &text,
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
            serde_json::json!([{
                "pane": 2, "place": place, "relation": beside.relation(),
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

/// A work as the window's `Card` — the same fields the shell's command sends,
/// because the page is the same page.
fn card(work: &girsa_corpus::work::Work) -> serde_json::Value {
    serde_json::json!({
        "slug": work.slug,
        "he_title": work.he_title,
        "en_title": work.en_title,
        "categories": work.categories,
        "author": work.author,
        "era": work.era.as_deref().map(display::era_said),
        "source": work.source.as_str(),
    })
}
