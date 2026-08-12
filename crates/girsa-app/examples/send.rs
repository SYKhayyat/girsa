//! Send a source out of the real corpus, and print the three flavours
//! (BUILDER.md W15).
//!
//! ```sh
//! cargo run -p girsa-app --example send -- \
//!     corpus "שולחן ערוך, אורח חיים סימן א' סעיף א'"
//!
//! # only part of the passage — the same thing a highlight does
//! cargo run -p girsa-app --example send -- corpus "ברכות ב." --from 0 --to 24
//! ```
//!
//! This is the independent reproduction for W15: what it prints is byte for
//! byte what one Ctrl+C puts on the clipboard, including the packet Ksav takes.
//! Pipe the last block into Ksav's `insert` and it compiles.
//!
//! A citation that names two seforim is **shown as a choice and not picked**
//! (BUILDER.md rule 6), which is why this exits non-zero rather than sending
//! the first candidate.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use girsa_app::sending::{send, Selection};
use girsa_app::session::Pointing;
use girsa_app::Shelf;
use girsa_cite::CiteStyle;
use girsa_ref::{resolve, Lexicon, Resolution};

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // Three settings now, not a flag — `girsa_app::session::Pointing`. The
    // flag stays as the shorthand for *everything the corpus has*, and
    // `--pointing nikud` is the middle one: the nikud with the trup off.
    let pointing = match args.iter().position(|a| a == "--pointing") {
        Some(at) => args
            .get(at + 1)
            .and_then(|word| Pointing::named(word))
            .unwrap_or(Pointing::Plain),
        None if args.iter().any(|a| a == "--nikud") => Pointing::Full,
        None => Pointing::Plain,
    };
    let from_char = flag(&mut args, "--from").unwrap_or(0);
    let to_char = flag(&mut args, "--to");
    let style = named_style(&mut args);
    args.retain(|a| !a.starts_with("--"));

    let mut args = args.into_iter();
    let (Some(root), Some(citation)) = (args.next(), args.next()) else {
        eprintln!("usage: send [--nikud|--pointing full|nikud|plain] [--style hebrew-full|hebrew-short|english] \\");
        eprintln!("            [--from N] [--to N] <corpus-root> <citation>");
        return ExitCode::from(2);
    };
    let root = PathBuf::from(root);

    let lexicon = match lexicon(&root) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("no lexicon under {}: {e}", root.display());
            return ExitCode::FAILURE;
        }
    };
    let reference = match resolve(&lexicon, &citation) {
        Resolution::Exact(r) => r,
        Resolution::Ambiguous(candidates) => {
            eprintln!("{citation:?} could be any of these, so nothing was sent:");
            for candidate in candidates {
                eprintln!("  {candidate}");
            }
            return ExitCode::FAILURE;
        }
        Resolution::Unresolved => {
            eprintln!("nothing on this shelf is called {citation:?}");
            return ExitCode::FAILURE;
        }
    };

    let personal =
        std::env::var("GIRSA_PERSONAL").map_or_else(|_| root.join("../personal"), PathBuf::from);
    let shelf = match Shelf::open(&root, &personal) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let sefer = match shelf.read(&reference.work_slug()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let at = sefer.at(reference.from());
    let (Some(first), Some(last)) = (at.first(), at.last()) else {
        eprintln!(
            "{} is on the shelf and has no {}",
            sefer.work.he_title,
            reference.from()
        );
        return ExitCode::FAILURE;
    };
    let selection = Selection {
        from: first.clone(),
        to: last.clone(),
        from_char,
        to_char,
    };

    let sent = match send(&sefer, &selection, style, pointing, None) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    println!("── the ref the document stores ──────────────────────────────");
    println!("{}", sent.packet.reference);
    println!("\n── text/plain — WhatsApp, a terminal, anything ──────────────");
    println!("{}", sent.plain);
    println!("\n── text/html — Word, an email, a browser ────────────────────");
    println!("{}", sent.html);
    println!("\n── application/x-girsa-source+json — Ksav ───────────────────");
    match sent.packet.to_json() {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("the packet would not serialize: {e}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// The lexicon `girsa-import` wrote, both halves of it.
fn lexicon(root: &Path) -> std::io::Result<Lexicon> {
    let mut body = std::fs::read_to_string(root.join("lexicon.tsv"))?;
    if let Ok(more) = std::fs::read_to_string(root.join("lexicon-otzaria.tsv")) {
        body.push('\n');
        body.push_str(&more);
    }
    Ok(Lexicon::from_tsv(&body))
}

fn flag(args: &mut Vec<String>, name: &str) -> Option<usize> {
    let at = args.iter().position(|a| a == name)?;
    let value = args.get(at + 1)?.parse().ok();
    args.remove(at + 1);
    value
}

fn named_style(args: &mut Vec<String>) -> CiteStyle {
    let Some(at) = args.iter().position(|a| a == "--style") else {
        return CiteStyle::HebrewFull;
    };
    let style = args
        .get(at + 1)
        .and_then(|name| CiteStyle::named(name))
        .unwrap_or(CiteStyle::HebrewFull);
    args.remove(at + 1);
    style
}
