//! The semantic lane, from the command line (spec.md §9.9, BUILDER.md W30).
//!
//! ```sh
//! cargo run --release -p girsa-app --bin girsa-lane -- corpus personal state
//! cargo run --release -p girsa-app --bin girsa-lane -- corpus personal model D:\berel
//! cargo run --release -p girsa-app --bin girsa-lane -- corpus personal add bavli/berakhot
//! cargo run --release -p girsa-app --bin girsa-lane -- corpus personal embed
//! cargo run --release -p girsa-app --bin girsa-lane -- corpus personal ask "זמן קריאת שמע"
//! ```
//!
//! It exists so the whole of W30 has an **independent reproduction** — every
//! claim the window makes about the lane can be checked here, by somebody who
//! did not write it, without a webview. That is BUILDER.md §0.3's fourth
//! condition, and it is also how the numbers in the commit message were taken.
//!
//! Every command goes through [`girsa_app::Adjacency`], which is what the window
//! calls. There is no second engine here — a tool with its own idea of what is
//! embedded would be a tool that agrees with the window right up until it
//! matters.

// A tool that prints a report. The library it calls does not print.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::{Path, PathBuf};

use girsa_app::{Adjacency, Shelf};
use girsa_lane::{bring, Chosen, Settings, BEREL};

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, personal, rest @ ..] = args.as_slice() else {
        usage();
        return std::process::ExitCode::from(2);
    };
    let (root, personal) = (PathBuf::from(root), PathBuf::from(personal));

    let shelf = match Shelf::open(&root, &personal) {
        Ok(shelf) => shelf,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let (mut lane, trouble) = Adjacency::open(&root, &personal, &shelf);
    for line in &trouble {
        eprintln!("{line}");
    }

    let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
    let done = match rest.as_slice() {
        [] | ["state"] => state(&lane),
        ["model", dir] => model(&mut lane, &shelf, Path::new(dir)),
        ["off"] => off(&mut lane, &shelf),
        ["allow-fetch"] => allow_fetch(&mut lane, &shelf),
        ["bring"] => bring_it(&mut lane, &shelf, &personal),
        ["add", "--all"] => add_all(&mut lane, &shelf),
        ["add", slug] => add(&mut lane, &shelf, slug),
        ["drop", slug] => drop(&mut lane, &shelf, slug),
        ["embed"] => embed(&mut lane, &shelf),
        ["ask", text] => ask(&lane, &shelf, text),
        _ => {
            usage();
            return std::process::ExitCode::from(2);
        }
    };
    if done {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

fn usage() {
    eprintln!(
        "usage: girsa-lane <corpus> <personal> <command>

  state                  where the lane stands, and what it covers
  model <dir>            point it at a model directory you already have, and turn it on
  off                    turn it off. Literal search is unchanged either way
  allow-fetch            let Girsa go and get a model. Off in a fresh install
  bring                  bring {} in ({:.0} MB, {}) — needs allow-fetch first
  add <slug>|--all       put a sefer, or the whole library, in the lane
  drop <slug>            take one back out
  embed                  embed what is chosen. Stop it with Ctrl-C; it resumes
  ask <text>             ask the lane. Adjacent results, and what is not covered",
        BEREL.name,
        BEREL.bytes as f64 / 1_048_576.0,
        BEREL.licence,
    );
}

fn state(lane: &Adjacency) -> bool {
    match lane.state().said() {
        None => println!("the semantic lane is off"),
        Some(said) => println!("{said}"),
    }
    println!("{}", lane.coverage().said());
    for slug in &lane.coverage().other_model {
        println!("  {slug}: vectors from another model");
    }
    true
}

fn model(lane: &mut Adjacency, shelf: &Shelf, dir: &Path) -> bool {
    let settings = Settings {
        on: true,
        model: Some(dir.to_path_buf()),
        may_fetch: lane.lane().settings().may_fetch,
    };
    if let Err(e) = lane.set(settings, shelf) {
        eprintln!("{e}");
        return false;
    }
    state(lane)
}

fn off(lane: &mut Adjacency, shelf: &Shelf) -> bool {
    let settings = Settings {
        on: false,
        ..lane.lane().settings().clone()
    };
    if let Err(e) = lane.set(settings, shelf) {
        eprintln!("{e}");
        return false;
    }
    println!("the semantic lane is off. Literal search is exactly what it was");
    true
}

fn allow_fetch(lane: &mut Adjacency, shelf: &Shelf) -> bool {
    let settings = Settings {
        may_fetch: true,
        ..lane.lane().settings().clone()
    };
    if let Err(e) = lane.set(settings, shelf) {
        eprintln!("{e}");
        return false;
    }
    println!("Girsa may now go and get a model. `bring` will fetch:");
    println!("  {} · {} · {}", BEREL.name, BEREL.by, BEREL.licence);
    println!("  {}", BEREL.what);
    println!("  {}", BEREL.about);
    true
}

fn bring_it(lane: &mut Adjacency, shelf: &Shelf, personal: &Path) -> bool {
    // The terms, before anything moves. They are not Girsa's to grant.
    println!("{} · {} · {}", BEREL.name, BEREL.by, BEREL.licence);
    println!("{}", BEREL.about);
    let may_fetch = lane.lane().settings().may_fetch;
    let mut last = 0u64;
    let dir = match bring(personal, may_fetch, &mut |progress| {
        let mb = progress.bytes / 1_048_576;
        if mb != last {
            last = mb;
            match progress.want {
                Some(want) => eprint!(
                    "\r  {} ({}/{}) · {mb} of {} MB   ",
                    progress.file,
                    progress.nth,
                    progress.of,
                    want / 1_048_576
                ),
                None => eprint!("\r  {} · {mb} MB   ", progress.file),
            }
        }
        true
    }) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("\n{e}");
            return false;
        }
    };
    eprintln!();
    println!("{}", dir.display());
    model(lane, shelf, &dir)
}

fn add(lane: &mut Adjacency, shelf: &Shelf, slug: &str) -> bool {
    if shelf.work(slug).is_none() {
        eprintln!("{slug} is not on this shelf");
        return false;
    }
    let chosen = lane.lane().chosen().clone().with_work(slug);
    if let Err(e) = lane.choose(chosen, shelf) {
        eprintln!("{e}");
        return false;
    }
    state(lane)
}

fn add_all(lane: &mut Adjacency, shelf: &Shelf) -> bool {
    if let Err(e) = lane.choose(Chosen::everything(), shelf) {
        eprintln!("{e}");
        return false;
    }
    state(lane)
}

fn drop(lane: &mut Adjacency, shelf: &Shelf, slug: &str) -> bool {
    let mut chosen = lane.lane().chosen().clone();
    if chosen.is_everything() {
        eprintln!(
            "the whole library is in the lane — `add` a sefer to choose seforim instead, then drop"
        );
        return false;
    }
    if !chosen.without_work(slug) {
        eprintln!("{slug} was not in the lane");
        return false;
    }
    if let Err(e) = lane.choose(chosen, shelf) {
        eprintln!("{e}");
        return false;
    }
    state(lane)
}

fn embed(lane: &mut Adjacency, shelf: &Shelf) -> bool {
    let started = std::time::Instant::now();
    let mut last = String::new();
    let result = lane.embed(shelf, &mut |slug, done, wanted| {
        if slug != last {
            last = slug.to_string();
            eprintln!();
        }
        eprint!("\r  {slug} · {done}/{wanted}          ");
        true
    });
    eprintln!();
    match result {
        Ok((wrote, trouble)) => {
            for line in &trouble {
                eprintln!("{line}");
            }
            let seconds = started.elapsed().as_secs_f64();
            #[allow(clippy::cast_precision_loss)]
            let rate = if seconds > 0.0 {
                wrote as f64 / seconds
            } else {
                0.0
            };
            println!("{wrote} segments embedded in {seconds:.1}s ({rate:.1}/s)");
            println!("{}", lane.coverage().said());
            true
        }
        Err(e) => {
            eprintln!("{e}");
            false
        }
    }
}

fn ask(lane: &Adjacency, shelf: &Shelf, text: &str) -> bool {
    let answer = lane.ask(shelf, text, &[], girsa_lane::MOST);
    // The label first, every time. These are adjacent results and they are
    // never to be read as the places the words appear (spec.md §14).
    println!("{}", answer.label);
    println!("{}", answer.coverage);
    if let Some(why) = &answer.refused {
        println!("nothing: {why}");
        return true;
    }
    if answer.near.is_empty() {
        println!("nothing near it in what is covered");
        return true;
    }
    for near in &answer.near {
        println!();
        println!("  {:.4}  {} {}", near.nearness, near.title, near.id);
        println!("          {}", one_line(&near.text, 140));
    }
    true
}

/// A segment on one line, cut at a character boundary and **marked** where it
/// was cut. A quotation that silently stops reads as the whole line.
fn one_line(text: &str, most: usize) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= most {
        return flat;
    }
    let cut: String = flat.chars().take(most).collect();
    format!("{cut}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_that_was_cut_says_it_was_cut() {
        assert_eq!(one_line("  שתי   מילים  ", 40), "שתי מילים");
        let long: String = std::iter::repeat_n('א', 50).collect();
        let cut = one_line(&long, 10);
        assert_eq!(cut.chars().count(), 11);
        assert!(cut.ends_with('…'));
    }
}
