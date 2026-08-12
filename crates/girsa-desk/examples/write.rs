//! Write a buffer the way the window does, out of the real corpus (W17).
//!
//! ```sh
//! cargo run -p girsa-desk --example write -- \
//!     corpus personal "השכמת הבוקר" "שולחן ערוך, אורח חיים סימן א' סעיף א'"
//! ```
//!
//! A heading, a source taken out of the library, and a line of your own. What
//! it prints is the path of a `.ksav` file — **a Ksav document, not an export
//! of one**, which is the acceptance for this order: it opens in the real Ksav
//! with zero conversion, and Ksav's own suite compiles the file this writes.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::PathBuf;
use std::process::ExitCode;

use girsa_app::sending::{quote, Sent};
use girsa_app::session::Pointing;
use girsa_app::Shelf;
use girsa_cite::CiteStyle;
use girsa_desk::buffer::Buffer;
use girsa_ksav::CitationPlacement;
use girsa_ref::{resolve, Lexicon, Resolution};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, personal, name, citation] = args.as_slice() else {
        eprintln!("usage: write <corpus-root> <personal-root> <buffer-name> <citation>");
        return ExitCode::from(2);
    };
    let root = PathBuf::from(root);
    let personal = PathBuf::from(personal);

    let Ok(lexicon) = lexicon(&root) else {
        eprintln!("no lexicon under {}", root.display());
        return ExitCode::FAILURE;
    };
    let reference = match resolve(&lexicon, citation) {
        Resolution::Exact(r) => r,
        // BUILDER rule 6: a choice is shown as a choice.
        Resolution::Ambiguous(candidates) => {
            eprintln!("{citation:?} could be any of {} places", candidates.len());
            return ExitCode::FAILURE;
        }
        Resolution::Unresolved => {
            eprintln!("nothing on this shelf is called {citation:?}");
            return ExitCode::FAILURE;
        }
    };

    let sent: Sent =
        match Shelf::open(&root, &personal).and_then(|shelf| shelf.read(&reference.work_slug())) {
            Ok(sefer) => match quote(
                &sefer,
                &reference,
                None,
                CiteStyle::HebrewFull,
                Pointing::Plain,
            ) {
                Ok(sent) => sent,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            },
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };

    let mut buffer = Buffer::new(name);
    let mut at = buffer.insert(0, &girsa_ksav::heading(1, name));
    at = buffer.insert_source(at, &sent, CitationPlacement::Mekor);
    buffer.insert(at, "\nוצריך עיון.\n");

    match buffer.save(&personal) {
        Ok(path) => {
            println!("{}", path.display());
            print!("{}", buffer.text);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn lexicon(root: &std::path::Path) -> std::io::Result<Lexicon> {
    let mut body = std::fs::read_to_string(root.join("lexicon.tsv"))?;
    if let Ok(more) = std::fs::read_to_string(root.join("lexicon-otzaria.tsv")) {
        body.push('\n');
        body.push_str(&more);
    }
    Ok(Lexicon::from_tsv(&body))
}
