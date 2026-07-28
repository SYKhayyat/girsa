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

use std::path::{Path, PathBuf};

use girsa_app::taxonomy::Branch;
use girsa_app::Shelf;

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let corpus = PathBuf::from(args.next().unwrap_or_else(|| "corpus".into()));
    let personal = PathBuf::from(args.next().unwrap_or_else(|| "personal".into()));

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

    let verb = args.next().unwrap_or_else(|| "show".into());
    let outcome = match verb.as_str() {
        "show" => Ok(()),
        "add" => add(&mut shelf, args.next().as_deref()),
        "move" => put(&mut shelf, args.next().as_deref(), args.next().as_deref()),
        "reset" => shelf
            .edit(|a| {
                a.reset();
                Ok(())
            })
            .map_err(|e| e.to_string()),
        other => Err(format!(
            "{other}: this reads `show`, `add <file>`, `move <slug> <shelf>` and `reset`"
        )),
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
