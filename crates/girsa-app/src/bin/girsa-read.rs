//! Read the words off a scan, on a terminal — so that W26 can be seen without a
//! window (BUILDER.md §0.3).
//!
//! ```sh
//! # what the file itself says, which is exact where a PDF was typeset
//! node app/tools/glyphs.mjs personal/files/user-berachos-combined.pdf 7 \
//!   | cargo run -q -p girsa-app --bin girsa-read -- corpus personal words user/berachos-combined
//!
//! # and OCR, for the pages that carry no text of their own
//! cargo run -q -p girsa-app --bin girsa-read -- corpus personal ocr user/berachos-combined /tmp/pages
//!
//! cargo run -q -p girsa-app --bin girsa-read -- corpus personal show user/berachos-combined 7
//! cargo run -q -p girsa-app --bin girsa-read -- corpus personal status
//!
//! # and a correction, which is anchored to the ink and not to the words
//! cargo run -q -p girsa-app --bin girsa-read -- corpus personal fix user/berachos-combined 151 12 שמע
//! ```
//!
//! `status` with no slug is spec.md §9.7's sentence — *4 PDFs on this shelf
//! aren't searchable yet* — printed rather than drawn, so that the count in the
//! results header can be checked against something.

// A tool that prints a report. The library it calls does not print.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::io::Read as _;
use std::path::Path;

use girsa_app::find_index;
use girsa_app::reading::gap;
use girsa_app::scanning::{is_scan, pages_of};
use girsa_app::Shelf;
use girsa_plain::argv::{self, Argv, Roots};
use girsa_scan::engine::{Engine, Image, Tesseract};
use girsa_scan::reading::{group, unmapped, Area, Glyph, Read, Reader};
use girsa_scan::words::{Job, Words};
use serde::Deserialize;

/// A page of glyphs as `app/tools/glyphs.mjs` prints it, in pixels of the page
/// at scale 1.
#[derive(Debug, Deserialize)]
struct PageGlyphs {
    page: usize,
    width: f32,
    height: f32,
    glyphs: Vec<Drawn>,
}

#[derive(Debug, Deserialize)]
struct Drawn {
    text: String,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl PageGlyphs {
    /// The glyphs as fractions of the page.
    ///
    /// Converted here and once. A rectangle in pixels is a fact about the size
    /// somebody rendered at, and a highlight stored in one lands in the margin
    /// the first time the reader zooms.
    fn as_fractions(&self) -> Vec<Glyph> {
        if self.width <= 0.0 || self.height <= 0.0 {
            return Vec::new();
        }
        self.glyphs
            .iter()
            .map(|g| Glyph {
                text: g.text.clone(),
                at: Area::new(
                    g.left / self.width,
                    g.top / self.height,
                    g.right / self.width,
                    g.bottom / self.height,
                ),
            })
            .collect()
    }
}

/// What this reads.
const USAGE: &str = "\
usage: girsa-read [corpus] [personal] [command]

  status [slug]            what is searchable, and what honestly is not. The default
  words <slug>             read a page from glyphs on stdin
  ocr <slug> <dir> [page…] read pages with the OCR engine, one at a time
  show <slug> <page>       the words on a page, as they were read
  fix <slug> <page> <word> <says>
                           correct one word of one page

corpus and personal default to directories of those names beside you.";

fn main() -> std::process::ExitCode {
    // Named `typed` rather than `words`, which is one of this binary's verbs.
    let typed: Vec<String> = std::env::args().skip(1).collect();
    if Argv::wants_help(&typed) {
        return argv::asked(USAGE);
    }
    let args = match Argv::of(typed, &[], &[]) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return argv::refuse(USAGE);
        }
    };
    let Roots {
        corpus,
        personal,
        after: prefix,
    } = Roots::of(&args);
    let after = args.from(prefix);

    let shelf = match Shelf::open(&corpus, &personal) {
        Ok(shelf) => shelf,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Some(trouble) = shelf.trouble() {
        eprintln!("{trouble}");
    }

    let verb = after.first().map_or("status", String::as_str);
    let rest: &[String] = after.get(1..).unwrap_or(&[]);
    let outcome = match verb {
        "status" => status(&shelf, &personal, rest.first().map(String::as_str)),
        "words" => words(&shelf, &personal, rest.first().map(String::as_str)),
        "ocr" => ocr(&shelf, &personal, rest),
        "show" => show(&personal, rest),
        "fix" => fix(&personal, rest),
        // A typo, not a failure. This used to come back through the same
        // `Err(String)` path as *the shelf will not open* and exit 1 with it.
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

/// What is searchable and what is not — spec.md §9.7's sentence, printed.
fn status(shelf: &Shelf, personal: &Path, slug: Option<&str>) -> Result<(), String> {
    if let Some(slug) = slug {
        let sefer = shelf.read(slug).map_err(|e| e.to_string())?;
        let pages = pages_of(&sefer);
        let (words, trouble) = Words::open(personal, slug);
        for line in trouble {
            eprintln!("{line}");
        }
        let job = Job::of(slug, pages, &words);
        println!("{slug}: {} of {pages} pages read", job.done());
        if !words.read_by().is_empty() {
            println!("read by {}", words.read_by().join(", "));
        }
        for (page, fix) in words.stranded() {
            println!(
                "page {page}: the correction to {:?} has nothing under it now",
                fix.was
            );
        }
        return Ok(());
    }

    // The sentence itself comes from the library, so this, the results header
    // and the test cannot drift into disagreeing about a count.
    let index = find_index(shelf.root()).ok();
    let gap = gap(shelf, personal, index.as_deref());
    match gap.said() {
        Some(said) => {
            println!("{said}");
            for scan in &gap.scans {
                println!(
                    "  {} — {} of {} pages read",
                    scan.slug, scan.read, scan.pages
                );
            }
        }
        None => println!("every scan on this shelf has been read, and your own layer is indexed"),
    }
    Ok(())
}

/// Take the glyphs a PDF hands over and make words of them.
fn words(shelf: &Shelf, personal: &Path, slug: Option<&str>) -> Result<(), String> {
    let slug = slug.ok_or("which scan? give its slug")?;
    let sefer = shelf.read(slug).map_err(|e| e.to_string())?;
    if !is_scan(&sefer.work) {
        return Err(format!("{slug} is not a scan"));
    }

    let mut body = String::new();
    std::io::stdin()
        .read_to_string(&mut body)
        .map_err(|e| format!("reading the glyphs: {e}"))?;

    let (mut store, trouble) = Words::open(personal, slug);
    for line in trouble {
        eprintln!("{line}");
    }

    let mut pages = 0;
    let mut found = 0;
    let mut lost = 0;
    let mut refused = 0;
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let page: PageGlyphs =
            serde_json::from_str(line).map_err(|e| format!("that is not a page of glyphs: {e}"))?;
        let glyphs = page.as_fractions();
        lost += unmapped(&glyphs);
        let grouped = group(&glyphs);
        refused += grouped.refused;
        let read = Read::new(page.page, Reader::Embedded, grouped.words);
        found += read.words.len();
        pages += 1;
        store.record(read).map_err(|e| e.to_string())?;
    }

    println!("{pages} pages, {found} words");
    if lost > 0 {
        // Expected in a small way and worth saying: most of them are the nikud,
        // which the index strips in every mode anyway (spec.md §9.1). The
        // second number is the one to look at — those are words this file
        // cannot spell, left out rather than indexed a letter short.
        println!("{lost} code points the file would not name; {refused} words left out for it");
    }
    let job = Job::of(slug, pages_of(&sefer), &store);
    println!("{} of {} pages read", job.done(), job.pages());
    Ok(())
}

/// Look at the pictures instead, for the pages that have no text of their own.
fn ocr(shelf: &Shelf, personal: &Path, rest: &[String]) -> Result<(), String> {
    let slug = rest.first().ok_or("which scan? give its slug")?;
    let from = rest.get(1).ok_or("where are the page images?")?;
    let sefer = shelf.read(slug).map_err(|e| e.to_string())?;

    let engine = Tesseract::found(Some(personal))
        .ok_or_else(|| girsa_scan::EngineError::NoEngine.to_string())?;
    eprintln!("reading with {}", engine.name());

    let (mut store, trouble) = Words::open(personal, slug);
    for line in trouble {
        eprintln!("{line}");
    }
    let mut job = Job::of(slug, pages_of(&sefer), &store);
    // Named pages, or whatever is left. Naming them is how one page gets read
    // again with a better engine without the other three hundred being redone.
    let asked: Vec<usize> = rest[2..].iter().filter_map(|n| n.parse().ok()).collect();

    let mut again: Vec<usize> = Vec::new();
    while let Some(page) = next_of(&asked, &job, &again) {
        let path = Path::new(from).join(format!("page{page}.png"));
        let Ok(png) = std::fs::read(&path) else {
            // Not an error: this drives one page at a time on purpose, and a
            // caller who rendered five pages of a sefer of three hundred is
            // exercising five pages of it.
            break;
        };
        let (width, height) =
            size_of(&png).ok_or_else(|| format!("{} is not a PNG", path.display()))?;
        let read = engine
            .read(page, &Image { png, width, height })
            .map_err(|e| e.to_string())?;
        println!("page {page}: {} words", read.words.len());
        store.record(read).map_err(|e| e.to_string())?;
        job.did(page);
        again.push(page);
    }
    println!("{} of {} pages read", job.done(), job.pages());
    Ok(())
}

/// The next page to read.
///
/// Whatever the job has not got to — or, where the caller named pages, exactly
/// those, **whether or not they have been read before**. Naming a page is how a
/// reader says *read this one again with the better engine*, and refusing
/// because it has already been read would make the one thing W26 is built to
/// make safe the one thing that cannot be done.
fn next_of(asked: &[usize], job: &Job, done: &[usize]) -> Option<usize> {
    if asked.is_empty() {
        return job.next();
    }
    asked.iter().copied().find(|page| !done.contains(page))
}

/// A PNG's size, off its header.
///
/// Eight bytes of signature, then an `IHDR` chunk whose first two fields are the
/// width and the height, big-endian. Reading them is cheaper and far less
/// dependency than decoding an image this program never looks at: the engine
/// opens the file itself, and all that is wanted here is what the boxes coming
/// back are a fraction of.
fn size_of(png: &[u8]) -> Option<(u32, u32)> {
    let header = png.get(..24)?;
    if &header[..8] != b"\x89PNG\r\n\x1a\n" || &header[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(header[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(header[20..24].try_into().ok()?);
    Some((width, height))
}

/// What one page says, and where on the page it says it.
fn show(personal: &Path, rest: &[String]) -> Result<(), String> {
    let slug = rest.first().ok_or("which scan? give its slug")?;
    let page: usize = rest
        .get(1)
        .ok_or("which page?")?
        .parse()
        .map_err(|_| "a page is a number")?;

    let (store, trouble) = Words::open(personal, slug);
    for line in trouble {
        eprintln!("{line}");
    }
    let read = store
        .page(page)
        .ok_or_else(|| format!("nobody has read page {page} of {slug}"))?;
    println!("page {page}, read by {}", read.by.name());
    println!("{}", read.text());
    Ok(())
}

/// Correct a word on a page, by its ink.
///
/// The word is named by its place in the current reading, which is how a reader
/// points at one; what is **written down** is the rectangle it sits on. So the
/// same correction still lands after the page has been read again by something
/// else, which is the whole of W26 and is what
/// `girsa-scan/tests/the_image_is_ground_truth.rs` asserts.
fn fix(personal: &Path, rest: &[String]) -> Result<(), String> {
    let slug = rest.first().ok_or("which scan? give its slug")?;
    let page: usize = rest
        .get(1)
        .ok_or("which page?")?
        .parse()
        .map_err(|_| "a page is a number")?;
    let word: usize = rest
        .get(2)
        .ok_or("which word? give its place in the reading, counting from 1")?
        .parse()
        .map_err(|_| "a word is a number")?;
    let says = rest.get(3).ok_or("what does it say?")?.clone();

    let (mut store, trouble) = Words::open(personal, slug);
    for line in trouble {
        eprintln!("{line}");
    }
    let was = store
        .as_read(page)
        .ok_or_else(|| format!("nobody has read page {page} of {slug}"))?
        .words
        .get(word.checked_sub(1).ok_or("the first word is 1")?)
        .ok_or_else(|| format!("page {page} has no word {word} on it"))?
        .clone();

    store
        .fix(
            page,
            girsa_scan::Fix {
                at: was.at,
                was: was.text.clone(),
                says: says.clone(),
            },
        )
        .map_err(|e| e.to_string())?;
    println!("page {page}, word {word}: {} → {says}", was.text);
    println!(
        "anchored to the ink at {:.3},{:.3}–{:.3},{:.3} of the page",
        was.at.left, was.at.top, was.at.right, was.at.bottom
    );
    Ok(())
}
