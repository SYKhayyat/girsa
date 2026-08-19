//! A place to write, without leaving the library (spec.md §10.3, W17).
//!
//! You are learning, you have a thought, you write it down. The library is
//! already open and the sefer is already on the screen; switching applications
//! to record one line is how the line does not get recorded.
//!
//! # Lightweight is the UI, not the format
//!
//! This is the constraint the whole order turns on. The buffer here is a plain
//! text box with a small row of buttons — and what it writes is **real Ksav
//! markup from the first keystroke**, produced by [`girsa_ksav`], the crate Ksav
//! itself compiles. A buffer that invented its own note shape would make the
//! handoff lossy, which is the drift the shared crates exist to prevent.
//!
//! So there is nothing to convert. A buffer is a `.ksav` file in your own
//! layer; opening it in the real Ksav is opening a file.
//!
//! # Where they live
//!
//! `personal/ksav/<name>.ksav` — under **your** root, never under the corpus,
//! which a re-download is entitled to replace wholesale.

use std::path::{Path, PathBuf};

use girsa_ksav::CitationPlacement;

use girsa_app::sending::Sent;

/// The extension, which is also the promise: this is a Ksav document.
const EXTENSION: &str = "ksav";

/// Something you are writing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    /// What you called it. Shown on the tab.
    pub name: String,
    /// Real Ksav markup.
    pub text: String,
}

/// Why a buffer would not open or save.
#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    /// A name that is not a name — empty, or one that would put the file
    /// somewhere other than your own layer. **Refused rather than repaired**:
    /// silently rewriting `../../etc/passwd` into something safe teaches
    /// nobody anything, and the reader typed something they meant.
    #[error("`{0}` is not a name for a buffer")]
    BadName(String),
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl Buffer {
    /// A new, empty one.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            text: String::new(),
        }
    }

    /// Where the buffers are.
    #[must_use]
    pub fn dir(personal: &Path) -> PathBuf {
        personal.join(EXTENSION)
    }

    /// The file a name maps to.
    ///
    /// # Errors
    ///
    /// If the name would land anywhere but in that directory.
    pub fn path(personal: &Path, name: &str) -> Result<PathBuf, BufferError> {
        let trimmed = name.trim();
        // Every one of these is a way out of the directory, and the list is
        // written as a property rather than a blocklist: a name is a *file
        // name*, so anything that is not one is refused.
        let is_a_name = !trimmed.is_empty()
            && trimmed != "."
            && trimmed != ".."
            && !trimmed.contains(['/', '\\', ':', '\0'])
            && !trimmed.starts_with('.');
        if !is_a_name {
            return Err(BufferError::BadName(name.to_string()));
        }
        Ok(Self::dir(personal).join(format!("{trimmed}.{EXTENSION}")))
    }

    /// Read one back, or start it.
    ///
    /// A name with no file yet is an **empty buffer, not an error** — that is
    /// what starting to write is.
    ///
    /// # Errors
    ///
    /// If the name is not a name, or the file is there and will not read.
    pub fn open(personal: &Path, name: &str) -> Result<Self, BufferError> {
        let path = Self::path(personal, name)?;
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(source) => {
                return Err(BufferError::Io {
                    path: path.display().to_string(),
                    source,
                })
            }
        };
        Ok(Self {
            name: name.trim().to_string(),
            text,
        })
    }

    /// Write it down.
    ///
    /// Beside and renamed over. This is the **most frequently written file in
    /// the application** — the drawer schedules a save 900 ms after every
    /// keystroke — and it holds the one thing in the personal layer that cannot
    /// be re-derived from anything else. A truncating write leaves it empty for
    /// the length of the write, so a machine that stops in that window leaves
    /// an empty document where the reader's writing was, and says nothing.
    ///
    /// # Errors
    ///
    /// If the name is not a name, or the personal layer will not take it.
    pub fn save(&self, personal: &Path) -> Result<PathBuf, BufferError> {
        let path = Self::path(personal, &self.name)?;
        girsa_personal::beside::write(&path, &self.text).map_err(|source| BufferError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Ok(path)
    }

    /// Whether a document of this name is already on the shelf.
    ///
    /// Asked before a **rename**, which is the one write in this drawer that
    /// can destroy a document the reader did not have open. `save` truncates
    /// whatever is at the name it is given, and the name it is given on a
    /// rename is a new one — so renaming *ראש השנה* to a name already in use
    /// replaced the document that was there, with no prompt and no mention.
    ///
    /// Not asked before an ordinary save: saving over the document you are
    /// looking at is what saving is.
    ///
    /// # Errors
    ///
    /// If the name is not a name.
    pub fn taken(personal: &Path, name: &str) -> Result<bool, BufferError> {
        Ok(Self::path(personal, name)?.exists())
    }

    /// Everything you have been writing, most recently touched first.
    #[must_use]
    pub fn list(personal: &Path) -> Vec<String> {
        let mut found: Vec<(std::time::SystemTime, String)> =
            std::fs::read_dir(Self::dir(personal))
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.extension().is_none_or(|e| e != EXTENSION) {
                        return None;
                    }
                    let name = path.file_stem()?.to_str()?.to_string();
                    let when = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    Some((when, name))
                })
                .collect();
        found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        found.into_iter().map(|(_, name)| name).collect()
    }

    /// Put text in at a character offset, and say where the cursor now is.
    ///
    /// Characters, not bytes, for the reason they are characters everywhere in
    /// this crate: Hebrew is two bytes a letter and an offset that came from a
    /// text box lands mid-character about half the time.
    pub fn insert(&mut self, at: usize, markup: &str) -> usize {
        let at = at.min(self.text.chars().count());
        let byte = self
            .text
            .char_indices()
            .nth(at)
            .map_or(self.text.len(), |(i, _)| i);
        self.text.insert_str(byte, markup);
        at + markup.chars().count()
    }

    /// Put a source in: the quote, and its mekor.
    ///
    /// The markup is [`girsa_ksav`]'s, byte for byte — the same function Ksav
    /// renders an arriving packet with. There is deliberately no second
    /// implementation here, because the moment there is one, the buffer and the
    /// pen start producing documents that differ.
    pub fn insert_source(&mut self, at: usize, sent: &Sent, placement: CitationPlacement) -> usize {
        self.insert(at, &girsa_ksav::to_ksav(&sent.packet, placement))
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_app::sending::{send, Selection};
    use girsa_app::session::Pointing;
    use girsa_cite::CiteStyle;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_buffer_is_a_ksav_file_in_your_own_layer() {
        let personal = scratch("girsa-buffer-file");
        let mut buffer = Buffer::new("חבורה");
        buffer.text = "#כותרת1[סוגיית מאימתי]\n".into();
        let path = buffer.save(&personal).expect("saves");

        assert!(path.starts_with(&personal), "{}", path.display());
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("ksav"));
        assert_eq!(Buffer::open(&personal, "חבורה").expect("reads"), buffer);
        assert_eq!(Buffer::list(&personal), ["חבורה"]);
        let _ = std::fs::remove_dir_all(&personal);
    }

    #[test]
    fn a_name_that_would_leave_the_personal_layer_is_refused() {
        let personal = scratch("girsa-buffer-escape");
        for name in [
            "../secret",
            "..\\secret",
            "",
            "   ",
            ".",
            "..",
            ".hidden",
            "C:/x",
        ] {
            assert!(
                Buffer::path(&personal, name).is_err(),
                "{name:?} was accepted"
            );
        }
    }

    #[test]
    fn a_name_with_no_file_yet_is_an_empty_buffer_and_not_an_error() {
        let personal = scratch("girsa-buffer-new");
        let buffer = Buffer::open(&personal, "חדש").expect("starts");
        assert!(buffer.text.is_empty());
    }

    #[test]
    fn text_goes_in_where_the_cursor_is_and_the_cursor_moves_past_it() {
        let mut buffer = Buffer::new("x");
        buffer.text = "אבג".into();
        assert_eq!(buffer.insert(1, "—"), 2);
        assert_eq!(buffer.text, "א—בג");
        // Past the end is the end, rather than a panic: a text box that has
        // scrolled can hand over an offset from a moment ago.
        assert_eq!(buffer.insert(999, "!"), 5);
        assert_eq!(buffer.text, "א—בג!");
    }

    #[test]
    fn a_source_written_into_a_buffer_is_real_ksav_and_nothing_of_ours() {
        // The constraint the whole order turns on (spec.md §10.3). The
        // assertion is deliberately an equality against `girsa_ksav`, not a
        // `contains`: a second renderer here would pass a `contains` for years
        // and produce documents that differ.
        let sefer = girsa_app::pretend::shulchan_arukh();
        let sent = send(
            &sefer,
            &Selection::whole(sefer.segments[0].id.clone()),
            CiteStyle::HebrewFull,
            Pointing::Plain,
            girsa_app::shemos::Shemos::AsWritten,
            None,
        )
        .expect("sends");

        let mut buffer = Buffer::new("חבורה");
        buffer.insert_source(0, &sent, CitationPlacement::Mekor);
        assert_eq!(
            buffer.text,
            girsa_ksav::to_ksav(&sent.packet, CitationPlacement::Mekor)
        );
        assert!(buffer.text.contains("#ציטוט["));
        // With the ref in it (W19): a document that keeps only the printed
        // string cannot answer *where did I use this*.
        assert!(buffer.text.contains("#מראה_מקום(מקור: \"girsa:"));
        assert!(buffer.text.contains("יתגבר כארי"));
    }

    #[test]
    fn what_you_wrote_and_what_you_pasted_end_up_in_one_document() {
        let sefer = girsa_app::pretend::shulchan_arukh();
        let sent = send(
            &sefer,
            &Selection::whole(sefer.segments[0].id.clone()),
            CiteStyle::HebrewShort,
            Pointing::Plain,
            girsa_app::shemos::Shemos::AsWritten,
            None,
        )
        .expect("sends");

        let personal = scratch("girsa-buffer-mixed");
        let mut buffer = Buffer::new("חבורה");
        let at = buffer.insert(0, "#כותרת1[השכמת הבוקר]\n");
        buffer.insert_source(at, &sent, CitationPlacement::Mekor);
        buffer.insert(
            buffer.text.chars().count(),
            "\nוצריך עיון מה שכתב הרמ\"א.\n",
        );
        buffer.save(&personal).expect("saves");

        let back = Buffer::open(&personal, "חבורה").expect("reads");
        assert!(back.text.starts_with("#כותרת1[השכמת הבוקר]"));
        assert!(back.text.contains("#ציטוט["));
        assert!(back.text.ends_with("הרמ\"א.\n"));
        let _ = std::fs::remove_dir_all(&personal);
    }
}
