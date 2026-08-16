//! The shelf, on a terminal — so that W10 can be seen without a window.
//!
//! ```sh
//! cargo run -p girsa-app --bin girsa-shelf -- corpus personal
//! cargo run -p girsa-app --bin girsa-shelf -- corpus personal add ~/חבורה.txt
//! cargo run -p girsa-app --bin girsa-shelf -- corpus personal move bavli/berakhot שלי
//! cargo run -p girsa-app --bin girsa-shelf -- corpus personal reset
//! ```
//!
//! It is the same [`Shelf`] the window holds, called the same way, so what it
//! prints is what the panel draws. That is the point: BUILDER.md §0.3 asks for
//! *a command someone else can run to see the behaviour*, and a screenshot of a
//! tree is not one.
//!
//! Nothing here writes to the corpus. `add` and `move` write one file each,
//! both under the personal root.

// A tool that prints a report. The library it calls does not print.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::Path;

use girsa_app::taxonomy::Branch;
use girsa_app::Shelf;
use girsa_plain::argv::{self, Argv, Roots};

/// What this reads. There was none of this: the nearest thing was the error for
/// an unknown verb, which never named the `<corpus> <personal>` prefix at all —
/// so the way to find out what the first two words were was to read the source.
const USAGE: &str = "\
usage: girsa-shelf [corpus] [personal] [command]

  show                     the shelf as a tree. The default
  add <file>               put a sefer of your own on it
  move <slug> <shelf>      stand a sefer somewhere else
  reset                    forget your arrangement

corpus and personal default to directories of those names beside you.";

fn main() -> std::process::ExitCode {
    let words: Vec<String> = std::env::args().skip(1).collect();
    if Argv::wants_help(&words) {
        return argv::asked(USAGE);
    }
    let args = match Argv::of(words, &[], &[]) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return argv::refuse(USAGE);
        }
    };
    let Roots {
        corpus,
        personal,
        after,
    } = Roots::of(&args);
    let rest = args.from(after);

    let mut shelf = match Shelf::open(&corpus, &personal) {
        Ok(shelf) => shelf,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Some(trouble) = shelf.trouble() {
        eprintln!("{trouble}");
    }

    let verb = rest.first().map_or("show", String::as_str);
    let after = rest.get(1..).unwrap_or(&[]);
    let outcome = match verb {
        "show" => Ok(()),
        "add" => add(&mut shelf, after.first().map(String::as_str)),
        "move" => put(
            &mut shelf,
            after.first().map(String::as_str),
            after.get(1).map(String::as_str),
        ),
        "reset" => shelf
            .edit(|a| {
                a.reset();
                Ok(())
            })
            .map_err(|e| e.to_string()),
        // A verb nobody has is a typo, not a failure — `WRONG_INVOCATION`, so a
        // script can tell it from a corpus that will not open. This exited 1,
        // through the same path as *the shelf will not open*.
        other => {
            eprintln!("{other}: no such command");
            return argv::refuse(USAGE);
        }
    };
    if let Err(e) = outcome {
        eprintln!("{e}");
        return std::process::ExitCode::FAILURE;
    }

    let tree = shelf.tree();
    for branch in &tree {
        print(branch, 0);
    }
    // The sum is the assertion: a sefer that is on no branch is a sefer on the
    // shelf that cannot be browsed to, and this is where that would show.
    let counted: usize = tree.iter().map(|b| b.count).sum();
    println!(
        "\n{} shelves · {counted} seforim counted of {} on the shelf",
        tree.len(),
        shelf.works().len()
    );
    if counted != shelf.works().len() {
        eprintln!("a sefer is on no shelf, or on two");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

/// Two levels, which is as much as a terminal is any use for.
fn print(branch: &Branch, depth: usize) {
    if depth > 1 {
        return;
    }
    let mark = if branch.mine {
        "*"
    } else if branch.edited {
        "~"
    } else {
        " "
    };
    println!(
        "{mark}{:indent$}{:<28} {:>7}{}",
        "",
        branch.title,
        branch.count,
        if branch.here > 0 && !branch.children.is_empty() {
            format!("  ({} of them here)", branch.here)
        } else {
            String::new()
        },
        indent = depth * 2,
    );
    for child in &branch.children {
        print(child, depth + 1);
    }
}

fn add(shelf: &mut Shelf, file: Option<&str>) -> Result<(), String> {
    let file = file.ok_or("add what? give it a .txt, .docx or .pdf")?;
    let slug = shelf
        .add_mine(Path::new(file), None)
        .map_err(|e| e.to_string())?;
    let segments = shelf
        .read(&slug)
        .map(|open| open.segments.len())
        .unwrap_or_default();
    println!("{slug} — {segments} segments, each named once\n");
    Ok(())
}

fn put(shelf: &mut Shelf, slug: Option<&str>, key: Option<&str>) -> Result<(), String> {
    let (Some(slug), Some(key)) = (slug, key) else {
        return Err("move <slug> <shelf>".to_string());
    };
    if shelf.work(slug).is_none() {
        // Kept anyway by the arrangement — but say so, rather than let a typo
        // look like it worked.
        eprintln!("there is no sefer here called {slug}; the edit is kept all the same");
    }
    shelf
        .edit(|a| {
            a.put_work(slug, key);
            Ok(())
        })
        .map_err(|e| e.to_string())
}
