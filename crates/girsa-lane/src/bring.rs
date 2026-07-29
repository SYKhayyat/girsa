//! *Bring it in for me* — the one place in this crate that touches the network.
//!
//! # Why this exists, given that §16 #20 said it would not
//!
//! It was ruled in, mid-work-order, and the ruling is narrower than *Girsa may
//! download things*:
//!
//! > **Ship the folder picker as the default path, plus a fetch button that is
//! > off until you turn it on in settings — the same shape the lane itself has.
//! > Costs one more setting; keeps the offline default true out of the box.**
//!
//! So spec.md §14 changed from *Girsa never touches the network for a model* to
//! **Girsa never needs the network**, which is the promise that was actually
//! worth keeping. A fresh install has [`crate::Settings::may_fetch`] false, and
//! with it false nothing in this module can run — there is no code path from a
//! search, a startup, an import or a job that reaches here. Turning it on
//! reveals a button. Pressing the button is a reader deciding to spend 738 MB.
//!
//! The licence line survives untouched, and that is the part that could not
//! have been traded away: **nothing is vendored**. The weights land in the
//! reader's personal layer, which is the same place their notes and their
//! corrections live and is not this repository (BUILDER.md T7).
//!
//! # Resumable *inside* one file, which the corpus fetcher is not
//!
//! `girsa_corpus::fetch` is resumable per file, because its files are a few
//! hundred kilobytes and re-fetching one costs nothing. One of these is 738 MB
//! over a domestic connection, and a fetcher that started that again from zero
//! on every dropped connection would never finish on a bad line. So this one
//! asks for `Range: bytes=N-`, appends to `x.part`, and picks up where it
//! stopped — and it verifies the length at the end, because a server that
//! ignores the header and sends the whole file from the top would otherwise
//! produce a 1.4 GB file that is two copies of half a model.

use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

/// A model somebody can be offered, with everything a reader needs to decide.
///
/// The licence is on this struct rather than in a comment because the button
/// must not be pressable without it having been shown: it is a term of use for
/// a thing that is about to land on the reader's disk, and it is not Girsa's to
/// grant on their behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// What it is called.
    pub name: &'static str,
    /// Who made it.
    pub by: &'static str,
    /// Under what terms.
    pub licence: &'static str,
    /// Where a reader can read the terms and the model card themselves.
    pub about: &'static str,
    /// What it is for, in one line.
    pub what: &'static str,
    /// Roughly how much will move.
    pub bytes: u64,
}

/// BEREL — BERT Embeddings for Rabbinic-Encoded Language.
///
/// The one offered, because it is the only encoder trained on the right
/// register: ~220M words of rabbinic Hebrew and Aramaic. spec.md §9.4 already
/// catalogued what happens when a modern-Hebrew model is pointed at a Rishon.
///
/// The licence here was **checked, not copied from the spec**: BUILDER.md W30
/// flagged that spec.md §9.4 called BEREL *unrestricted* while the README
/// warned it carries its own terms, and those are different claims. The model
/// card, its YAML frontmatter and the Hub API all say `apache-2.0`, on
/// 29 July 2026. `BEREL_2.0` is where the paper points and it redirects to 3.0.
pub const BEREL: Offer = Offer {
    name: "BEREL 3.0",
    by: "dicta-il",
    licence: "Apache-2.0",
    about: "https://huggingface.co/dicta-il/BEREL_2.0",
    what: "BERT Embeddings for Rabbinic-Encoded Language — trained on ~220M words of rabbinic \
           Hebrew and Aramaic",
    bytes: 742_923_190,
};

/// Where the files come from, and what they are called on disk.
const FROM: &str = "https://huggingface.co/dicta-il/BEREL_2.0/resolve/main";

/// The files. `vocab.txt` and `tokenizer_config.json` are not read by
/// [`crate::model`] — `tokenizer.json` carries everything it needs — and are
/// brought anyway so that the directory is a model directory by anybody's
/// reckoning and not only by this one's.
const FILES: [&str; 5] = [
    "config.json",
    "tokenizer_config.json",
    "tokenizer.json",
    "vocab.txt",
    "model.safetensors",
];

/// Why the model did not come in.
#[derive(Debug, thiserror::Error)]
pub enum BringError {
    /// The setting is off. **A state, not a failure** — and the only way to
    /// reach this module at all is with it on.
    #[error(
        "bringing a model in is switched off — Girsa never needs the network, so this is a \
         setting you turn on rather than one you turn off"
    )]
    NotAllowed,
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("http {status} for {url}")]
    Http { status: u16, url: String },
    #[error("{url}: {source}")]
    Transport {
        url: String,
        #[source]
        source: Box<ureq::Error>,
    },
    /// The one a resuming fetcher has to check for. See the module note.
    #[error("{name} came to {got} bytes and should be {want} — it was not brought in")]
    WrongSize { name: String, got: u64, want: u64 },
    /// The reader closed the panel. Not an error in any real sense; it is
    /// reported so that a caller does not report success.
    #[error("stopped — {done} of {all} files were brought in and will resume where they stopped")]
    Stopped { done: usize, all: usize },
}

impl BringError {
    fn io(path: &Path) -> impl Fn(std::io::Error) -> Self + '_ {
        move |source| Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

/// How far along it is. Handed to the caller often enough to draw a bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// Which file, by name.
    pub file: String,
    /// Which of them, one-based.
    pub nth: usize,
    pub of: usize,
    /// Bytes of this file on disk, including what a previous run left.
    pub bytes: u64,
    /// What this file should come to, where the server said.
    pub want: Option<u64>,
}

/// Where a brought-in model goes: the reader's own layer, never the corpus and
/// never this repository.
#[must_use]
pub fn into(personal: &Path) -> PathBuf {
    personal.join("lane").join("models").join("berel")
}

/// Bring BEREL in.
///
/// `watch` is called as it goes and returns whether to keep going — so the
/// panel can draw a bar and the reader can stop, and stopping costs only the
/// chunk in flight. Resuming is calling this again.
///
/// # Errors
///
/// If the setting is off, if the network or the disk will not cooperate, or if
/// what arrived is not the length it should be. A run the reader stopped comes
/// back as [`BringError::Stopped`], which is not success and is not a fault.
pub fn bring(
    personal: &Path,
    may_fetch: bool,
    watch: &mut dyn FnMut(&Progress) -> bool,
) -> Result<PathBuf, BringError> {
    if !may_fetch {
        return Err(BringError::NotAllowed);
    }
    let dir = into(personal);
    std::fs::create_dir_all(&dir).map_err(BringError::io(&dir))?;

    for (nth, name) in FILES.iter().enumerate() {
        let path = dir.join(name);
        let mut progress = Progress {
            file: (*name).to_string(),
            nth: nth + 1,
            of: FILES.len(),
            bytes: 0,
            want: None,
        };
        if !one(&format!("{FROM}/{name}"), &path, &mut progress, watch)? {
            return Err(BringError::Stopped {
                done: nth,
                all: FILES.len(),
            });
        }
    }
    Ok(dir)
}

/// How much of a chunk is read before the caller is asked whether to carry on.
///
/// Small enough that stopping feels immediate and large enough that the check
/// is not the expensive part of the loop.
const CHUNK: usize = 1 << 20;

/// Fetch one file, resuming a `.part` if there is one. `false` if the reader
/// stopped.
fn one(
    url: &str,
    path: &Path,
    progress: &mut Progress,
    watch: &mut dyn FnMut(&Progress) -> bool,
) -> Result<bool, BringError> {
    // Already whole. Nothing here re-fetches what is on disk — the second run
    // of a stopped job should cost the tail and not the file.
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > 0 {
            progress.bytes = meta.len();
            progress.want = Some(meta.len());
            return Ok(watch(progress));
        }
    }

    let part = path.with_extension("part");
    let have = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);

    let mut request = ureq::get(url);
    if have > 0 {
        request = request.header("Range", &format!("bytes={have}-"));
    }
    let response = request.call().map_err(|source| match &source {
        ureq::Error::StatusCode(status) => BringError::Http {
            status: *status,
            url: url.to_string(),
        },
        _ => BringError::Transport {
            url: url.to_string(),
            source: Box::new(source),
        },
    })?;

    // 206 means the server honoured the range; 200 means it sent the whole file
    // and whatever is in `.part` has to go, or the two would be concatenated
    // into a file that is the right kind of nonsense to be hard to notice.
    let resumed = response.status().as_u16() == 206;
    let length = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let want = length.map(|len| if resumed { have + len } else { len });
    progress.want = want;

    let mut file = if resumed && have > 0 {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&part)
            .map_err(BringError::io(&part))?;
        file.seek(std::io::SeekFrom::End(0))
            .map_err(BringError::io(&part))?;
        progress.bytes = have;
        file
    } else {
        progress.bytes = 0;
        std::fs::File::create(&part).map_err(BringError::io(&part))?
    };

    let mut body = response.into_body().into_reader();
    let mut chunk = vec![0u8; CHUNK];
    loop {
        let read = body.read(&mut chunk).map_err(BringError::io(&part))?;
        if read == 0 {
            break;
        }
        file.write_all(&chunk[..read])
            .map_err(BringError::io(&part))?;
        progress.bytes += read as u64;
        if !watch(progress) {
            file.flush().map_err(BringError::io(&part))?;
            return Ok(false);
        }
    }
    file.flush().map_err(BringError::io(&part))?;
    drop(file);

    let got = std::fs::metadata(&part)
        .map_err(BringError::io(&part))?
        .len();
    if let Some(want) = want {
        if got != want {
            return Err(BringError::WrongSize {
                name: path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                got,
                want,
            });
        }
    }
    // The rename is the only moment the file exists under its real name, so
    // `Model::side_loaded` can never be handed a half-written safetensors —
    // the same rule `girsa_corpus::fetch` keeps, for the same reason.
    std::fs::rename(&part, path).map_err(BringError::io(path))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn nothing_can_be_fetched_until_the_setting_says_so() {
        // The whole of what was traded away and what was not. Off is the
        // default, and off is enforced here rather than only in the window —
        // a button is a drawing, and this is the door.
        let dir = std::env::temp_dir().join("girsa-lane-bring-off");
        let said = bring(&dir, false, &mut |_| true)
            .expect_err("refused")
            .to_string();
        assert!(said.contains("switched off"), "{said}");
        assert!(said.contains("never needs the network"), "{said}");
        assert!(!dir.join("lane").exists(), "and nothing was created");
    }

    #[test]
    fn the_offer_carries_its_licence_and_where_to_read_it() {
        // BUILDER.md W30: verify the licence before writing a line. This is
        // the line the reader sees, and it is checked so that a change to the
        // constant cannot quietly drop the terms out of the panel.
        assert_eq!(BEREL.licence, "Apache-2.0");
        assert!(BEREL.about.starts_with("https://huggingface.co/"));
        assert!(BEREL.by == "dicta-il");
        // The number a reader is agreeing to spend. Asserted so that shrinking
        // it to a guess, or to zero, does not silently take the size out of the
        // sentence the panel shows before the button does anything.
        const { assert!(BEREL.bytes > 700_000_000) }
    }

    #[test]
    fn a_model_lands_in_the_personal_layer_and_nowhere_else() {
        let dir = into(Path::new("/somewhere/personal"));
        assert!(
            dir.ends_with("lane/models/berel") || dir.ends_with(r"lane\models\berel"),
            "{dir:?}"
        );
        assert!(dir.starts_with("/somewhere/personal"));
    }

    #[test]
    fn the_files_brought_are_the_files_a_model_directory_needs() {
        for wanted in crate::model::WANTED {
            assert!(FILES.contains(&wanted), "{wanted} is not brought in");
        }
    }
}
