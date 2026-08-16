//! What opening a sefer costs the window, measured.
//!
//! The reader's last sentence was *"also ensure it is blazing fast"*, and the
//! honest way to answer that is a number rather than an adjective. `three_seconds`
//! already measures the **correction** path in Rust and finds it at 80 ms on the
//! largest sefer on the shelf. This measures the other half — the one that runs
//! on every pane a reader opens, and the one that crosses the IPC boundary:
//!
//! 1. reading the sefer off disk with your corrections applied (`Shelf::read`);
//! 2. drawing every line the way the pane draws it (`view::Line::of`);
//! 3. **serializing all of them to JSON**, which is what actually goes over the
//!    wire to the webview.
//!
//! Step 3 is the one nobody had a number for. It is worth having because it is
//! the one that scales with the sefer rather than with what the reader is
//! looking at: a pane draws a window of 400 lines and is handed all 18,120.
//!
//! ```sh
//! cargo run --release -p girsa-app --example measure-opening -- corpus personal
//! ```

#![allow(clippy::print_stdout, clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::Instant;

use girsa_app::session::Pointing;
use girsa_app::shelf::Shelf;
use girsa_app::view::Line;

fn main() {
    let mut args = std::env::args().skip(1);
    let corpus = PathBuf::from(args.next().unwrap_or_else(|| "corpus".to_string()));
    let personal = PathBuf::from(args.next().unwrap_or_else(|| "personal".to_string()));

    let began = Instant::now();
    let shelf = match Shelf::open(&corpus, &personal) {
        Ok(shelf) => shelf,
        Err(e) => {
            eprintln!("no shelf at {}: {e}", corpus.display());
            std::process::exit(1);
        }
    };
    println!(
        "shelf: {} works in {} ms",
        shelf.works().len(),
        began.elapsed().as_millis()
    );

    // The bookcase, which is what a reader meets first.
    let began = Instant::now();
    let tree = shelf.tree();
    let counted: usize = tree.iter().map(|b| b.count).sum();
    println!(
        "tree: {} top shelves over {counted} seforim in {} ms",
        tree.len(),
        began.elapsed().as_millis()
    );

    // One shelf's worth of seforim, in the order they are printed in. This is a
    // click on a shelf, and it used to work out every work's shipped shelf from
    // its categories on every call.
    let began = Instant::now();
    let mut clicked = 0usize;
    for branch in &tree {
        clicked += shelf.works_on(&branch.key).len();
    }
    println!(
        "works_on: every top shelf ({clicked} seforim) in {} ms",
        began.elapsed().as_millis()
    );

    // And opening a sefer, all the way to the bytes the webview receives.
    for slug in ["mishnah-berurah", "bavli/berakhot", "genesis"] {
        let began = Instant::now();
        let Ok(sefer) = shelf.read(slug) else {
            println!("{slug}: not on this shelf");
            continue;
        };
        let read = began.elapsed();

        let began = Instant::now();
        let lines: Vec<Line> = sefer
            .segments
            .iter()
            .map(|s| {
                Line::of(
                    &sefer,
                    s,
                    Pointing::Full,
                    girsa_app::shemos::Shemos::AsWritten,
                    girsa_cite::CiteStyle::HebrewShort,
                    None,
                )
            })
            .collect();
        let drawn = began.elapsed();

        let began = Instant::now();
        let json = serde_json::to_string(&lines).expect("lines serialize");
        let wire = began.elapsed();

        // What `open_sefer` actually sends: a window, not the sefer. The
        // whole-sefer numbers are kept beside it because they are what this
        // measurement was built to argue about — see `view::Text`.
        let window = 600.min(sefer.segments.len());
        let began = Instant::now();
        let only: Vec<Line> = sefer.segments[..window]
            .iter()
            .map(|s| {
                Line::of(
                    &sefer,
                    s,
                    Pointing::Full,
                    girsa_app::shemos::Shemos::AsWritten,
                    girsa_cite::CiteStyle::HebrewShort,
                    None,
                )
            })
            .collect();
        let sent = serde_json::to_string(&only).expect("a window serializes");
        let windowed = began.elapsed();

        println!(
            "{slug}: {} segments · read {} ms
    whole:  draw {} ms · serialize {} ms · {} KB
    window: draw+serialize {} ms · {} KB  ← what a pane is handed",
            lines.len(),
            read.as_millis(),
            drawn.as_millis(),
            wire.as_millis(),
            json.len() / 1024,
            windowed.as_millis(),
            sent.len() / 1024,
        );
    }
}
