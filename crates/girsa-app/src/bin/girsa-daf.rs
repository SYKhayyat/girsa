//! The page→daf mapping, on a terminal — so that W25 can be seen without a
//! window, and without a scan of your own.
//!
//! ```sh
//! cargo run -p girsa-app --bin girsa-daf -- corpus personal add ~/ברכות.pdf
//! cargo run -p girsa-app --bin girsa-daf -- corpus personal map user/ברכות amud 5=ב. --of bavli/berakhot
//! cargo run -p girsa-app --bin girsa-daf -- corpus personal show user/ברכות
//! cargo run -p girsa-app --bin girsa-daf -- corpus personal cite user/ברכות 47
//! cargo run -p girsa-app --bin girsa-daf -- corpus personal page user/ברכות "כג."
//! cargo run -p girsa-app --bin girsa-daf -- corpus personal forget user/ברכות
//! ```
//!
//! An anchor is `page=place`, and `page=-` says *from here these are not pages
//! of the sefer* — the plates, an inserted index. Several may be given:
//!
//! ```sh
//! … map user/ברכות amud 5=ב. 43=- 45=כא.
//! ```
//!
//! It is the same [`Shelf`] the window holds, so what it prints is what the
//! viewer shows. BUILDER.md §0.3 asks for a command someone else can run, and a
//! screenshot of a PDF is not one.

// A tool that prints a report. The library it calls does not print.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::Path;

use girsa_app::scanning::{self, mareh_makom, scan_of};
use girsa_app::Shelf;
use girsa_cite::CiteStyle;
use girsa_plain::argv::{self, Argv, Roots};
use girsa_ref::Address;
use girsa_scan::{Anchor, Paging, Placed, Scan, Scheme};

const USAGE: &str = "\
usage: girsa-daf [corpus] [personal] [command]

  list                                       your scans. The default
  add <file>                                 put a PDF on the shelf
  show <slug>                                what its pages are called
  map <slug> <amud|daf|numbered> <page=place>… [--of <slug>]
                                             say which page is which daf
  cite <slug> <page>                         the mekor for a page
  page <slug> <place>                        which page a place is on
  forget <slug>

corpus and personal default to directories of those names beside you.
An anchor is `5=\u{5d1}.` — page 5 is daf bet, amud alef — or `5=-` for a page
with nothing printed on it that a mekor could name.";

fn main() -> std::process::ExitCode {
    let typed: Vec<String> = std::env::args().skip(1).collect();
    if Argv::wants_help(&typed) {
        return argv::asked(USAGE);
    }
    let args = match Argv::of(typed, &[], &["--of"]) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return argv::refuse(USAGE);
        }
    };
    let Roots { corpus, personal } = Roots::of(&args);
    let after = args.from(Roots::AFTER);

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

    let verb = after.first().map_or("list", String::as_str);
    let rest: &[String] = after.get(1..).unwrap_or(&[]);
    let outcome = match verb {
        "list" => list(&shelf),
        "add" => add(&mut shelf, rest.first().map(String::as_str)),
        "show" => show(&shelf, rest.first().map(String::as_str)),
        "map" => map(&mut shelf, args.value("--of"), rest),
        "cite" => cite(&shelf, rest),
        "page" => page(&shelf, rest),
        "forget" => forget(&mut shelf, rest.first().map(String::as_str)),
        other => {
            eprintln!("{other}: no such command");
            return argv::refuse(USAGE);
        }
    };
    if let Err(e) = outcome {
        eprintln!("{e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

/// Every scan on the shelf, and whether the chore has been done.
fn list(shelf: &Shelf) -> Result<(), String> {
    let mut found = 0usize;
    for work in shelf.works().iter().filter(|w| scanning::is_scan(w)) {
        found += 1;
        let paged = shelf
            .scans()
            .of(&work.slug)
            .is_some_and(girsa_scan::Paging::is_declared);
        println!(
            "{:<40} {}",
            work.slug,
            if paged {
                "paged"
            } else {
                "not paged yet — `map` it"
            }
        );
    }
    if found == 0 {
        println!("no scans on this shelf — `add` a PDF");
    }
    Ok(())
}

fn add(shelf: &mut Shelf, file: Option<&str>) -> Result<(), String> {
    let file = file.ok_or("add what? give it a .pdf")?;
    let slug = shelf
        .add_mine(Path::new(file), None)
        .map_err(|e| e.to_string())?;
    let pages = shelf
        .read(&slug)
        .map(|open| scanning::pages_of(&open))
        .unwrap_or_default();
    println!("{slug} — {pages} pages, each named once and never again");
    Ok(())
}

/// The scan, or a message saying which of the two reasons there is not one.
fn scan(shelf: &Shelf, slug: Option<&str>) -> Result<(Scan, String), String> {
    let slug = slug.ok_or("which scan?")?;
    let sefer = shelf.read(slug).map_err(|e| e.to_string())?;
    let scan = scan_of(shelf, &sefer).ok_or_else(|| format!("{slug} is not a scan"))?;
    let title = sefer.work.he_title.clone();
    Ok((scan, title))
}

fn show(shelf: &Shelf, slug: Option<&str>) -> Result<(), String> {
    let (scan, title) = scan(shelf, slug)?;
    println!("{title} — {} pages", scan.pages());
    match scan.paging().of() {
        Some(of) => println!("a scan of {of}"),
        None => println!("standing on its own; `--of <slug>` says what it is a scan of"),
    }
    if !scan.is_paged() {
        println!("\nnot paged yet. Nothing here is citable until it is:");
        println!("  girsa-daf … map {} amud 5=ב.", scan.slug());
        return Ok(());
    }

    println!("scheme: {}", scan.paging().scheme().name());
    for anchor in scan.paging().anchors() {
        match &anchor.at {
            Some(at) => println!("  page {:<5} is {at}", anchor.page),
            None => println!("  page {:<5} and on: not pages of the sefer", anchor.page),
        }
    }

    // The first few pages of every run, which is where an off-by-one shows.
    println!("\nwhat the pages carry:");
    let mut shown = 0usize;
    for page in 1..=scan.pages() {
        let interesting = page <= 3
            || scan
                .paging()
                .anchors()
                .iter()
                .any(|a| page >= a.page && page < a.page + 3);
        if !interesting || shown > 30 {
            continue;
        }
        shown += 1;
        println!("  page {:<5} {}", page, said(&scan.at(page)));
    }
    Ok(())
}

fn said(placed: &Placed) -> String {
    match placed {
        Placed::At { from, to: None } => from.to_string(),
        Placed::At { from, to: Some(to) } => format!("{from}–{to}"),
        Placed::Unpaged => "— nothing printed on it that a mekor could name".to_string(),
    }
}

fn map(shelf: &mut Shelf, of: Option<&str>, words: &[String]) -> Result<(), String> {
    let mut rest = words.iter();
    let slug = rest.next().ok_or("map which scan?")?.clone();
    let scheme_name = rest
        .next()
        .ok_or("which scheme? `amud` — one side of a leaf a page — or `daf` or `numbered`")?;
    let scheme = Scheme::named(scheme_name)
        .ok_or_else(|| format!("{scheme_name}: this reads `amud`, `daf` or `numbered`"))?;

    // `--of` was pulled out of the anchors here by a hand-rolled loop with
    // `Vec::remove(0)` in it — the fifth flag mechanism in the repository, in a
    // binary whose *positionals* are already `key=value`.
    let mut anchors = Vec::new();
    for arg in rest {
        let (page, at) = arg
            .split_once('=')
            .ok_or_else(|| format!("{arg}: an anchor is `page=place`, or `page=-`"))?;
        let page: usize = page
            .trim()
            .parse()
            .map_err(|_| format!("{page}: a page is a number"))?;
        anchors.push(if at.trim() == "-" {
            Anchor::unpaged(page)
        } else {
            Anchor::written(page, at).map_err(|e| e.to_string())?
        });
    }
    if anchors.is_empty() {
        return Err("no anchors. `5=ב.` says page 5 is daf ב, amud alef".to_string());
    }
    if let Some(of) = of {
        if shelf.work(of).is_none() {
            return Err(format!(
                "there is no sefer here called {of}, so nothing could print its name"
            ));
        }
    }

    let paging =
        Paging::declare(of.map(ToString::to_string), scheme, anchors).map_err(|e| e.to_string())?;
    shelf
        .declare_paging(&slug, paging)
        .map_err(|e| e.to_string())?;
    show(shelf, Some(&slug))
}

fn cite(shelf: &Shelf, args: &[String]) -> Result<(), String> {
    let (scan, _) = scan(shelf, args.first().map(String::as_str))?;
    let page: usize = args
        .get(1)
        .ok_or("cite which page?")?
        .parse()
        .map_err(|_| "a page is a number".to_string())?;
    let sefer = shelf.read(scan.slug()).map_err(|e| e.to_string())?;
    let naming = scanning::naming(shelf, &scan).map_err(|e| e.to_string())?;

    let Some(sent) = mareh_makom(&scan, page, &naming, &sefer.work, CiteStyle::HebrewShort) else {
        println!("page {page} of the file — nothing printed on it that a mekor could name");
        return Ok(());
    };
    println!("{}", sent.plain);
    println!("{}", sent.packet.reference);
    if let Some(id) = scanning::page_id(&sefer, page) {
        // The anchor a note would hang on, which the mapping never moves.
        println!("{id}");
    }

    // And the ref, followed back through the shelf the way anything else that
    // has one would follow it. The two have to be the same page: a mekor that
    // opens somewhere other than where it was copied from is the whole class of
    // defect this project is arranged against.
    if let Ok(reference) = sent.packet.reference.parse::<girsa_ref::Ref>() {
        let landed = sefer.at(reference.from());
        let same = scanning::page_id(&sefer, page).is_some_and(|id| landed.contains(&id));
        println!(
            "the ref opens {}",
            if same {
                format!("page {page} — the page it was copied from")
            } else {
                format!("somewhere else: {landed:?}")
            }
        );
    }
    Ok(())
}

fn page(shelf: &Shelf, args: &[String]) -> Result<(), String> {
    let (scan, _) = scan(shelf, args.first().map(String::as_str))?;
    let written = args.get(1).ok_or("which place?")?;
    let address =
        Address::parse(written).ok_or_else(|| format!("{written} is not a place in a sefer"))?;
    match scan.page_of(&address) {
        Some(page) => println!("{address} is on page {page}"),
        // Never the nearest page it does have.
        None => println!("{address} is not in this scan"),
    }
    Ok(())
}

fn forget(shelf: &mut Shelf, slug: Option<&str>) -> Result<(), String> {
    let slug = slug.ok_or("forget which scan's mapping?")?;
    let had = shelf.forget_paging(slug).map_err(|e| e.to_string())?;
    println!(
        "{}",
        if had {
            "forgotten — better no mareh makom than a wrong one"
        } else {
            "there was no mapping for it"
        }
    );
    Ok(())
}
