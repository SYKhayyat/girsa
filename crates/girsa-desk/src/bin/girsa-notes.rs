//! Your own layer, on a terminal — so that W27 can be seen without a window
//! (BUILDER.md §0.3).
//!
//! ```sh
//! # a place, some words, done — the three-second one
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal write mishnah-berakhot 1:1 \
//!     "וצריך עיון מה שכתב הרמב\"ם כאן" --title מאימתי --tag ברכות
//!
//! # and then the claim: one call, and what I wrote is in the same list as the Rambam
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal on mishnah-berakhot 1:1
//!
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal list
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal show מאימתי
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal add מאימתי "ועוד יש לדקדק"
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal after girsa:note/מאימתי/2#2 "ובאמת"
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal anchor מאימתי bavli/berakhot 2a:1
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal forget מאימתי
//!
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal mark mishnah-berakhot 1:1 0 4
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal bookmark mishnah-berakhot 1:1 --label "להתחיל כאן"
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal keep מאימתי '"מאימתי קורין"'
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal folder thursday "חבורה יום ה" mishnah-berakhot 1:1
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal tags
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal export /tmp/my-layer
//! cargo run -p girsa-desk --bin girsa-notes -- corpus personal merge /tmp/their-layer
//! ```
//!
//! A place is given as `<slug> <address>` — the way a person says it — and the
//! permanent id is looked up rather than typed. Where a verb takes a paragraph
//! of a note instead, that is a segment id, because a paragraph of a note has
//! no other name.

// A tool that prints a report. The library it calls does not print.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::PathBuf;

use girsa_app::naming::Names;
use girsa_app::shelf::Shelf;
use girsa_app::Link;
use girsa_corpus::segment::SegmentId;
use girsa_desk::documents::Documents;
use girsa_note::mark::Placed;
use girsa_note::{Mark, Member, SavedQuery};
use girsa_plain::argv::{self, Argv, Roots};

/// The options this reads, and which of them take a value.
///
/// Naming them is what fixes `split_flags`, which made **every** `--x` swallow
/// the token after it: a switch ate a positional, and `--title=x` was stored
/// under the key `title=x` while still eating the next word.
const VALUES: &[&str] = &["--title", "--tag", "--label", "--name"];

/// The options that stand alone.
const SWITCHES: &[&str] = &["--forget"];

const USAGE: &str = "\
usage: girsa-notes [corpus] [personal] [command]

  list                                  your notes. The default
  show <note>                           one note, with its paragraph ids
  on <slug> <address>                   what you have written about a place
  write <slug> <address> <text> [--title t] [--tag t]…
  add <note> <text>                     another paragraph, at the end
  after <paragraph id> <text>           one in between, moving nothing
  tag <note> <tag>…
  anchor <note> <slug> <address>
  forget <note>
  mark <slug> <address> <from> <to> [--label l]
  bookmark <slug> <address> [--label l]
  marks
  keep <name> <query>                   save a question
  queries
  folder <name> <title> <slug> <address>
  folders
  tags
  export <directory>                    your layer, as plain files
  merge <directory>                     take somebody else's. Never overwrites yours
  documents                             the .ksav files Girsa knows about
  document <path> [--name n]            tell it about one. --forget to undo
  cites <ref>                           which of your documents cite a place

corpus and personal default to directories of those names beside you.
An option takes its value either way round: --title x and --title=x.";

fn main() -> std::process::ExitCode {
    let typed: Vec<String> = std::env::args().skip(1).collect();
    if Argv::wants_help(&typed) {
        return argv::asked(USAGE);
    }
    let args = match Argv::of(typed, SWITCHES, VALUES) {
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
    let after_roots = args.from(prefix);

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

    let verb = after_roots.first().map_or("list", String::as_str);
    let rest: &[String] = after_roots.get(1..).unwrap_or(&[]);
    let outcome = match verb {
        "list" => list(&shelf),
        "show" => show(&shelf, rest),
        "on" => on(&shelf, rest),
        "write" => write(&mut shelf, &args, rest),
        "add" => add(&mut shelf, rest),
        "after" => after(&mut shelf, rest),
        "tag" => tag(&mut shelf, rest),
        "anchor" => anchor(&mut shelf, rest),
        "forget" => forget(&mut shelf, rest),
        "mark" => mark(&mut shelf, &args, rest),
        "bookmark" => bookmark(&mut shelf, &args, rest),
        "marks" => marks(&shelf),
        "keep" => keep(&mut shelf, rest),
        "queries" => queries(&shelf),
        "folder" => folder(&mut shelf, rest),
        "folders" => folders(&shelf),
        "tags" => tags(&shelf),
        "export" => export(&shelf, rest),
        "merge" => merge(&mut shelf, rest),
        "documents" => documents(&shelf),
        "document" => document(&shelf, &args, rest),
        "cites" => cites(&shelf, rest),
        // A typo, not a failure. The list of verbs is `USAGE`, which is
        // also what `--help` prints — this binary had no usage string at all,
        // and this arm was it.
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

/// Everything you have written.
fn list(shelf: &Shelf) -> Result<(), String> {
    if shelf.notes().is_empty() {
        println!("nothing written yet");
        return Ok(());
    }
    for note in shelf.notes().all() {
        let about = note
            .on
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "{:<24} {:<3} {} {}",
            note.name(),
            note.paras().len(),
            note.title,
            if about.is_empty() {
                "— on nothing".to_string()
            } else {
                format!("— on {about}")
            }
        );
        if !note.tags.is_empty() {
            println!("{:<24} {}", "", note.tags.join(" · "));
        }
    }
    Ok(())
}

/// One note, paragraph by paragraph, with the names a citation would use.
fn show(shelf: &Shelf, rest: &[String]) -> Result<(), String> {
    let name = rest.first().ok_or("show <note>")?;
    let note = shelf
        .notes()
        .get(name)
        .ok_or_else(|| format!("there is no note called {name}"))?;
    println!("{}   {}", note.title, note.slug);
    for at in &note.on {
        println!("  on {at}");
    }
    for tag in &note.tags {
        println!("  # {tag}");
    }
    for para in note.paras() {
        println!("\n{}\n{}", para.id, para.text);
    }
    Ok(())
}

/// The links on a line — **including what you wrote**, from the one call.
fn on(shelf: &Shelf, rest: &[String]) -> Result<(), String> {
    let asked = place(shelf, rest)?;
    let sefer = shelf.read(asked.work()).map_err(|e| e.to_string())?;
    // A name typed at a prompt may be one this sefer no longer has live — the
    // parent of a cut, or a se'if upstream folded away. Resolve it to a place
    // that exists before asking what touches it, because *what links to
    // somewhere that is not somewhere* has no honest answer.
    let nth = sefer
        .position_of(&asked)
        .ok_or_else(|| format!("{asked} names nothing in this sefer"))?;
    let at = sefer
        .segments
        .get(nth)
        .map(|segment| segment.id.clone())
        .unwrap_or(asked);
    let standing = sefer.standing(&at);
    let touching = girsa_app::touching(shelf, shelf.repairs(), &standing);
    println!("{at}");
    if touching.incoming_unknown {
        println!("  (no companions cache — the incoming half is missing)");
    }
    let mine = touching.links.iter().filter(|link| is_mine(link)).count();
    for link in &touching.links {
        println!(
            "  {:<4} {:<12} {:>3}%  {}",
            if is_mine(link) { "שלי" } else { "" },
            link.repaired.edge.edge_type.as_str(),
            (link.repaired.confidence() * 100.0).round(),
            link.said()
        );
    }
    println!("{} links, {mine} of them yours", touching.links.len());

    let text = sefer.as_printed(&at);
    let yours = girsa_app::yours(shelf, &standing, text);
    for marked in &yours.marks {
        let where_it_is = match &marked.placed {
            Placed::Whole => "the whole line".to_string(),
            Placed::At { span, moved } => format!(
                "{}..{}{}",
                span.start,
                span.end,
                if *moved { " (moved)" } else { "" }
            ),
            // Never swallowed: a highlight whose words are gone is a thing you
            // did, and the reader is the only one who can put it right.
            Placed::Stale => "its words are gone — stale".to_string(),
        };
        println!(
            "  {} {} {}",
            marked.mark.kind.as_str(),
            where_it_is,
            marked.mark.label.as_deref().unwrap_or("")
        );
    }
    for folder in &yours.folders {
        println!("  in your folder {folder}");
    }
    Ok(())
}

fn is_mine(link: &Link) -> bool {
    link.work.starts_with("note/")
}

/// Write a note about a place.
fn write(shelf: &mut Shelf, args: &Argv, rest: &[String]) -> Result<(), String> {
    let at = place(shelf, rest)?;
    let text = rest.get(2).ok_or("write <slug> <address> <text>")?;
    let title = args.value("--title");

    let mut note =
        girsa_app::note_here(shelf, &at, title, text, &whoami()).map_err(|e| e.to_string())?;
    let wanted = args.every("--tag");
    if !wanted.is_empty() {
        for value in wanted {
            note.tag(value);
        }
        shelf.write_note(note.clone()).map_err(|e| e.to_string())?;
    }
    println!("{}   {}", note.slug, note.title);
    for para in note.paras() {
        println!("{}", para.id);
    }
    Ok(())
}

/// Another paragraph, at the end.
fn add(shelf: &mut Shelf, rest: &[String]) -> Result<(), String> {
    let name = rest.first().ok_or("add <note> <text>")?;
    let text = rest.get(1).ok_or("add <note> <text>")?;
    let mut note = shelf
        .notes()
        .get(name)
        .cloned()
        .ok_or_else(|| format!("there is no note called {name}"))?;
    let id = note.append(text);
    shelf.write_note(note).map_err(|e| e.to_string())?;
    println!("{id}");
    Ok(())
}

/// A paragraph in the middle — the one that would renumber everything under a
/// design that named a paragraph by its position.
fn after(shelf: &mut Shelf, rest: &[String]) -> Result<(), String> {
    let id: SegmentId = rest
        .first()
        .ok_or("after <paragraph id> <text>")?
        .parse()
        .map_err(|e| format!("{e}"))?;
    let text = rest.get(1).ok_or("after <paragraph id> <text>")?;
    let mut note = shelf
        .notes()
        .get(id.work())
        .cloned()
        .ok_or_else(|| format!("{} is not a paragraph of a note", id))?;
    let before: Vec<String> = note.paras().iter().map(|p| p.id.to_string()).collect();
    let minted = note.insert_after(&id, text).map_err(|e| e.to_string())?;
    let after: Vec<String> = note
        .paras()
        .iter()
        .map(|p| p.id.to_string())
        .filter(|kept| kept != &minted.to_string())
        .collect();
    shelf.write_note(note).map_err(|e| e.to_string())?;
    println!("{minted}");
    println!(
        "{} paragraphs were already named, and {} of them changed",
        before.len(),
        before.iter().zip(&after).filter(|(a, b)| a != b).count()
    );
    Ok(())
}

fn tag(shelf: &mut Shelf, rest: &[String]) -> Result<(), String> {
    let name = rest.first().ok_or("tag <note> <tag>…")?;
    let mut note = shelf
        .notes()
        .get(name)
        .cloned()
        .ok_or_else(|| format!("there is no note called {name}"))?;
    for value in rest.iter().skip(1) {
        note.tag(value);
    }
    let said = note.tags.join(" · ");
    shelf.write_note(note).map_err(|e| e.to_string())?;
    println!("{said}");
    Ok(())
}

fn anchor(shelf: &mut Shelf, rest: &[String]) -> Result<(), String> {
    let name = rest.first().ok_or("anchor <note> <slug> <address>")?;
    let at = place(shelf, &rest[1..])?;
    let mut note = shelf
        .notes()
        .get(name)
        .cloned()
        .ok_or_else(|| format!("there is no note called {name}"))?;
    note.anchor(at.clone());
    shelf.write_note(note).map_err(|e| e.to_string())?;
    println!("{at}");
    Ok(())
}

fn forget(shelf: &mut Shelf, rest: &[String]) -> Result<(), String> {
    let name = rest.first().ok_or("forget <note>")?;
    if shelf.forget_note(name).map_err(|e| e.to_string())? {
        println!("{name} is gone");
    } else {
        println!("there was no note called {name}");
    }
    Ok(())
}

fn mark(shelf: &mut Shelf, args: &Argv, rest: &[String]) -> Result<(), String> {
    let at = place(shelf, rest)?;
    let from: usize = rest
        .get(2)
        .ok_or("mark <slug> <address> <from> <to>")?
        .parse()
        .map_err(|_| "the offsets are numbers of characters")?;
    let to: usize = rest
        .get(3)
        .ok_or("mark <slug> <address> <from> <to>")?
        .parse()
        .map_err(|_| "the offsets are numbers of characters")?;

    let sefer = shelf.read(at.work()).map_err(|e| e.to_string())?;
    let letters: Vec<char> = sefer.as_printed(&at).chars().collect();
    let was: String = letters
        .get(from..to)
        .ok_or("those characters are not in the line")?
        .iter()
        .collect();

    let mut made = Mark::highlight(at, from..to, &was, whoami());
    if let Some(label) = args.value("--label") {
        made = made.called(label);
    }
    let id = made.id.clone();
    shelf
        .marks_mut()
        .add(made)
        .map_err(|e| e.to_string())
        .map(|_| ())?;
    println!("{id}  {was}");
    Ok(())
}

fn bookmark(shelf: &mut Shelf, args: &Argv, rest: &[String]) -> Result<(), String> {
    let at = place(shelf, rest)?;
    let mut made = Mark::bookmark(at.clone(), whoami());
    if let Some(label) = args.value("--label") {
        made = made.called(label);
    }
    let id = made.id.clone();
    shelf
        .marks_mut()
        .add(made)
        .map_err(|e| e.to_string())
        .map(|_| ())?;
    println!("{id}  {at}");
    Ok(())
}

fn marks(shelf: &Shelf) -> Result<(), String> {
    for mark in shelf.marks().all() {
        println!(
            "{:<18} {:<10} {}  {}",
            mark.id.as_str(),
            mark.kind.as_str(),
            mark.at,
            mark.label.as_deref().unwrap_or(&mark.was)
        );
    }
    println!("{} marks", shelf.marks().count());
    Ok(())
}

fn keep(shelf: &mut Shelf, rest: &[String]) -> Result<(), String> {
    let name = rest.first().ok_or("keep <name> <query>")?;
    let typed = rest.get(1).ok_or("keep <name> <query>")?;
    shelf
        .queries_mut()
        .save(SavedQuery::new(name, typed))
        .map_err(|e| e.to_string())?;
    println!("{name}: {typed}");
    Ok(())
}

fn queries(shelf: &Shelf) -> Result<(), String> {
    for query in shelf.queries().all() {
        println!("{:<24} {}", query.name, query.said());
    }
    println!("{} saved", shelf.queries().count());
    Ok(())
}

fn folder(shelf: &mut Shelf, rest: &[String]) -> Result<(), String> {
    let name = rest
        .first()
        .ok_or("folder <name> <title> <slug> <address>")?;
    let title = rest
        .get(1)
        .ok_or("folder <name> <title> <slug> <address>")?;
    let at = place(shelf, &rest[2..])?;
    girsa_app::collect(shelf, name, title, &at).map_err(|e| e.to_string())?;
    let held = shelf.collections().get(name).map_or(0, |f| f.members.len());
    println!("{name}: {held}");
    Ok(())
}

fn folders(shelf: &Shelf) -> Result<(), String> {
    for held in shelf.collections().all() {
        println!(
            "{:<20} {:<24} {}",
            held.name,
            held.title,
            held.members.len()
        );
        for member in &held.members {
            let said = match member {
                // `Naming::said`, and not `format!("{} {}", he_title, id)` —
                // which is what this was, while the window's equivalent was
                // `format!("{} {}", he_title, id.path().join(":"))`. One
                // `Member::Place`, one folder, two different strings.
                Member::Place(id) => Names::on(shelf).of(id).said(),
                Member::Work(slug) => shelf
                    .work(slug)
                    .map_or_else(|| slug.clone(), |work| work.he_title.clone()),
                Member::Query(name) => format!("? {name}"),
            };
            println!("    {said}");
        }
    }
    Ok(())
}

fn tags(shelf: &Shelf) -> Result<(), String> {
    let tags = girsa_note::Tags::of(&shelf.layer());
    for (tag, tally) in tags.iter() {
        let carried: Vec<String> = tally
            .iter()
            .filter(|(_, count)| *count > 0)
            .map(|(kind, count)| format!("{} {count}", kind.as_str()))
            .collect();
        println!("{:<24} {:>3}   {}", tag, tally.total(), carried.join(" · "));
    }
    println!("{} tags", tags.count());
    Ok(())
}

fn export(shelf: &Shelf, rest: &[String]) -> Result<(), String> {
    let into = PathBuf::from(rest.first().ok_or("export <directory>")?);
    let written = girsa_note::export(&shelf.layer(), &into).map_err(|e| e.to_string())?;
    let said: Vec<String> = written
        .iter()
        .map(|(kind, count)| format!("{count} {}", kind.as_str()))
        .collect();
    println!("{}: {}", into.display(), said.join(" · "));
    Ok(())
}

/// Take somebody else's layer into yours (spec.md §11).
///
/// The report is per kind and not a total, because the three numbers mean
/// different things per kind and a reader deciding whether the merge did what
/// they wanted needs to see which: nine marks taken and one folder refused is a
/// good outcome, and one line saying `10` hides the only part worth reading.
///
/// The refusals are named rather than counted alone — a folder called `ברכות`
/// that was not taken is a thing to go and look at, and *1 refused* is not
/// somewhere to look.
fn merge(shelf: &mut Shelf, rest: &[String]) -> Result<(), String> {
    let from = PathBuf::from(rest.first().ok_or("merge <directory>")?);
    if !from.is_dir() {
        return Err(format!("{}: not a directory", from.display()));
    }
    let took = girsa_note::merge(&mut shelf.layer_mut(), &from).map_err(|e| e.to_string())?;
    for kind in girsa_note::Taggable::ALL {
        let one = took.of(*kind);
        if one.taken == 0 && one.already_had == 0 && one.refused == 0 {
            continue;
        }
        println!(
            "{}: {} taken · {} already had · {} refused",
            kind.as_str(),
            one.taken,
            one.already_had,
            one.refused
        );
    }
    let all = took.all();
    if all.taken == 0 && all.already_had == 0 && all.refused == 0 {
        println!("{}: nothing of that shape in there", from.display());
    }
    if all.refused > 0 {
        // The sentence that keeps this honest. A merge that refused something
        // has not merged, and the reader has to know which way round it went:
        // what is on this shelf is still theirs.
        println!("what was refused is yours, and is untouched");
    }
    Ok(())
}

/// A place, said the way a person says it: `<slug> <address>`.
///
/// The permanent id is looked up rather than typed, which is the point of an
/// address existing at all — and where the address names more than one segment
/// the first is taken and the rest are printed, so nothing is silently chosen.
fn place(shelf: &Shelf, rest: &[String]) -> Result<SegmentId, String> {
    let slug = rest.first().ok_or("a place is <slug> <address>")?;
    if let Ok(id) = slug.parse::<SegmentId>() {
        return Ok(id);
    }
    let address = rest.get(1).ok_or("a place is <slug> <address>")?;
    let sefer = shelf.read(slug).map_err(|e| e.to_string())?;
    let parsed = girsa_ref::Address::parse(address)
        .ok_or_else(|| format!("{address} is not a place in a sefer"))?;
    let found = sefer.at(&parsed);
    match found.split_first() {
        Some((first, rest)) => {
            if !rest.is_empty() {
                println!("{address} names {} segments; taking {first}", found.len());
            }
            Ok(first.clone())
        }
        None => Err(format!("{slug} has nothing at {address}")),
    }
}

/// The `.ksav` files Girsa has been told about (spec.md §10.4).
///
/// Refreshed on the way — a `stat` per document — so the refs shown are the
/// refs in the files as they are now.
fn documents(shelf: &Shelf) -> Result<(), String> {
    let (mut documents, trouble) = Documents::open(shelf.personal());
    for line in trouble {
        eprintln!("{line}");
    }
    let read = documents.refreshed().map_err(|e| e.to_string())?;
    for document in documents.all() {
        let here = if document.is_here() {
            ""
        } else {
            "  [not here]"
        };
        println!("{:<40} {} refs{here}", document.name, document.refs.len());
        println!("{:<40} {}", "", document.path);
    }
    println!("\n{} documents · {read} re-read", documents.all().count());
    Ok(())
}

/// Tell Girsa about a document, or take one off the list.
///
/// The desk's `/document` is how this normally happens — Ksav says so when it
/// saves. This is the same errand from a terminal, because a feature that can
/// only be seen by installing a second application is a feature nobody can
/// check (BUILDER.md §0.3).
fn document(shelf: &Shelf, args: &Argv, rest: &[String]) -> Result<(), String> {
    let path = PathBuf::from(rest.first().ok_or("document <path>")?);
    let (mut documents, trouble) = Documents::open(shelf.personal());
    for line in trouble {
        eprintln!("{line}");
    }
    if args.switch("--forget") {
        let had = documents.forget(&path).map_err(|e| e.to_string())?;
        println!(
            "{}",
            if had {
                "forgotten — the file is untouched"
            } else {
                "it was not on the list"
            }
        );
        return Ok(());
    }
    documents
        .remember(&path, args.value("--name"))
        .map_err(|e| e.to_string())?;
    let read = documents.refreshed().map_err(|e| e.to_string())?;
    let held = documents.get(&path).map_or(0, |d| d.refs.len());
    println!("{} · {held} refs · {read} re-read", path.display());
    Ok(())
}

/// Which of your own documents cite a place.
///
/// The toy editor's buffers **and** the registry. This used to be the buffers
/// alone, so a `.ksav` written in the real Ksav was never found.
fn cites(shelf: &Shelf, rest: &[String]) -> Result<(), String> {
    let text = rest.first().ok_or("cites <ref>")?;
    let place: girsa_ref::Ref = text.parse().map_err(|e| format!("{text}: {e}"))?;
    let (mut documents, trouble) = Documents::open(shelf.personal());
    for line in trouble {
        eprintln!("{line}");
    }
    documents.refreshed().map_err(|e| e.to_string())?;
    let found = girsa_desk::who_cites(shelf.personal(), &documents, &place);
    if found.is_empty() {
        println!("nothing of yours cites that place");
        return Ok(());
    }
    for citing in &found {
        let where_it_is = citing.path.as_deref().unwrap_or("(the buffer)");
        let away = if citing.away { "  [not here]" } else { "" };
        println!("{:<30} {where_it_is}{away}", citing.name);
        for reference in &citing.refs {
            println!("{:<30}   {reference}", "");
        }
    }
    Ok(())
}

/// Who is writing. Free text — this is a personal layer, not a registry.
fn whoami() -> String {
    std::env::var("GIRSA_WHO")
        .or_else(|_| std::env::var("USERNAME"))
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "me".to_string())
}
