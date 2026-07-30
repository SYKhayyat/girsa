//! Print the Source Packet that Ksav's `from_girsa` test asserts against.
//!
//! ```sh
//! cargo run -q -p girsa-app --example fixture-packet
//! ```
//!
//! # Why this exists
//!
//! `Ksav/ksav/engine/tests/from_girsa.rs` is the only test anywhere that checks
//! the **producer against the consumer** — that what this library puts on the
//! clipboard is what the pen actually reads. Its own module note explains why
//! the fixture beside it is real output rather than a hand-written shape, and
//! then warns:
//!
//! > *a fixture nobody can reproduce is a fixture that will be wrong quietly.*
//!
//! It was right, and it happened. The fixture was captured by hand from a run
//! against the 2.2 GB corpus, the import path then started dropping the printed
//! edition (grade finding N-1), and for as long as that lasted the one test
//! that existed to catch it was comparing Ksav against a Girsa that no longer
//! existed — passing all the while.
//!
//! The reason it rotted is that regenerating it needed a corpus no gate has. So
//! the corpus is the thing that had to go. This example builds the one work the
//! fixture quotes, in a temp directory, out of constants below — the same
//! `Shelf::open` → `Shelf::read` → `send` path a Ctrl+C takes, with nothing
//! stubbed — and prints the packet. No corpus, no network, no fixture files:
//! `cargo run` and a diff, which is a thing CI can do on every push.
//!
//! `tools/check-ksav-fixture.sh` is that diff.
//!
//! # It is the same bytes as the real corpus
//!
//! Verified when this was written, and the reason the constants below are
//! copied out of the corpus rather than tidied:
//!
//! ```text
//! $ cargo run -q -p girsa-app --example send -- corpus \
//!       "שולחן ערוך, אורח חיים סימן א' סעיף ג'" | tail -1  >  a
//! $ cargo run -q -p girsa-app --example fixture-packet     >  b
//! $ diff a b   # no output
//! ```
//!
//! So `SEIF` keeps its commentator markup exactly as `merged.json` carries it.
//! The packet holds plain text, which means the stripping is part of what is
//! being checked, and a tidied constant would quietly stop checking it.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::PathBuf;
use std::process::ExitCode;

use girsa_app::sending::{send, Selection};
use girsa_app::Shelf;
use girsa_cite::CiteStyle;
use girsa_corpus::import::{self, ImportedWork, RawSegment, SegmentKind};
use girsa_corpus::work::{Source, Version, Work};

/// The sefer the fixture quotes, field for field out of
/// `corpus/works/shulchan-arukh/orach-chayim/work.json`.
const SLUG: &str = "shulchan-arukh/orach-chayim";

/// Orach Chayim 1:3, verbatim out of `segments.jsonl` — commentary anchors and
/// all. See the module note on why the markup stays.
const SEIF: &str = "<i data-commentator=\"Mishnah Berurah\" data-label=\"ט\"></i>\
<i data-commentator=\"Ateret Zekenim\" data-label=\"♦\" data-order=\"3\"></i>\
<i data-commentator=\"Be'er HaGolah\" data-label=\"ד\" data-order=\"4\"></i>\
ראוי לכל ירא שמים <i data-commentator=\"Ba'er Hetev\" data-order=\"7\"></i>\
<i data-commentator=\"Mishnah Berurah\" data-label=\"י\"></i>שיהא מיצר ודואג על \
<i data-commentator=\"Magen Avraham\" data-order=\"5\"></i>\
<i data-commentator=\"Mishnah Berurah\" data-label=\"יא\"></i>חורבן בית המקדש:";

fn main() -> ExitCode {
    let root = std::env::temp_dir().join("girsa-fixture-packet");
    let _ = std::fs::remove_dir_all(&root);

    let work = Work {
        slug: SLUG.to_string(),
        he_title: "שולחן ערוך, אורח חיים".into(),
        en_title: "Shulchan Arukh, Orach Chayim".into(),
        categories: vec!["Halakhah".into(), "Shulchan Arukh".into()],
        source: Source::Sefaria,
        // Where this came from on the machine it was imported on, which is not
        // the same on two machines and is not in the packet. Left empty so the
        // output is identical everywhere.
        origin: PathBuf::new(),
        schema: None,
        author: Some("יוסף קארו".into()),
        era: Some("AH".into()),
        comp_date: Some("1563 CE".into()),
        // The whole point of finding N-1. Read out of `merged.json` at import
        // and carried to the pen, because a sefer typeset from a quote whose
        // provenance was dropped cannot be un-dropped (spec.md §13).
        version: Some(Version {
            edition: "Maginei Eretz: Shulchan Aruch Orach Chaim, Lemberg, 1893".into(),
            license: None,
            provenance: Some("https://www.sefaria.org/Shulchan_Arukh,_Orach_Chayim".into()),
        }),
        he_sections: vec!["סימן".into(), "סעיף".into()],
        commentary_on: Vec::new(),
    };

    let raw = vec![RawSegment {
        path: vec!["1".into(), "3".into()],
        kind: SegmentKind::Text,
        text: SEIF.into(),
    }];

    // Written through the importer and read back through the shelf, rather than
    // handed straight to `send`. The trip through disk is where N-1 lived.
    let imported = ImportedWork::assemble(work, raw);
    if let Err(e) = import::write(&root, &imported) {
        eprintln!("could not write the fixture corpus: {e}");
        return ExitCode::FAILURE;
    }
    let line = match serde_json::to_string(&imported.work) {
        Ok(line) => line,
        Err(e) => {
            eprintln!("could not catalogue the fixture work: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::create_dir_all(root.join("works"))
        .and_then(|()| std::fs::write(root.join("works/index.jsonl"), format!("{line}\n")))
    {
        eprintln!("could not write the fixture catalogue: {e}");
        return ExitCode::FAILURE;
    }

    let personal = root.join("personal");
    let shelf = match Shelf::open(&root, &personal) {
        Ok(shelf) => shelf,
        Err(e) => {
            eprintln!("the fixture shelf will not open: {e}");
            return ExitCode::FAILURE;
        }
    };
    let sefer = match shelf.read(SLUG) {
        Ok(sefer) => sefer,
        Err(e) => {
            eprintln!("the fixture sefer will not open: {e}");
            return ExitCode::FAILURE;
        }
    };

    let Some(address) = girsa_ref::Address::parse("1:3") else {
        eprintln!("1:3 is not an address");
        return ExitCode::FAILURE;
    };
    let at = sefer.at(&address);
    let Some(first) = at.first().cloned() else {
        eprintln!("the fixture corpus has no se'if at 1:3");
        return ExitCode::FAILURE;
    };
    let selection = Selection {
        from: first.clone(),
        to: first,
        from_char: 0,
        to_char: None,
    };

    let sent = match send(&sefer, &selection, CiteStyle::HebrewFull, false, None) {
        Ok(sent) => sent,
        Err(e) => {
            eprintln!("nothing was sent: {e}");
            return ExitCode::FAILURE;
        }
    };
    match sent.packet.to_json() {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("the packet will not serialize: {e}");
            ExitCode::FAILURE
        }
    }
}
