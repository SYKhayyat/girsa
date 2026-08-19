//! One whole-file write, done the only way a whole-file write may be done.
//!
//! # The argument, which this layer had already made three times
//!
//! `std::fs::write` opens with `O_TRUNC`. From the moment of the open until the
//! write returns, the file on disk is **empty** — not the old contents, not the
//! new ones, nothing. A crash, a power loss, a full disk or a killed process
//! inside that window leaves a truncated or empty file where the reader's work
//! was, and nothing anywhere says it happened.
//!
//! The personal layer knows this. [`Log`](crate::Log) is append-only precisely
//! so that it is never exposed to it, and the one whole-file write it does —
//! compaction — is written beside and renamed over, with the reason beside it.
//! `Session::save` was given the same treatment and the same argument in
//! writing: *"what this call still owes is the atomicity, which is why it is a
//! rename and not a write."*
//!
//! Three files never got it, and they are not minor ones:
//!
//! * the document the reader is typing, which is saved 900 ms after every
//!   keystroke and holds the one thing in the layer that cannot be re-derived
//!   from anything else;
//! * the shelf arrangement — every shelf they made, every sefer they moved,
//!   every rename — whose torn-write failure mode the arrangement module had
//!   *already written out in prose* as *"your shelf arrangement would not read
//!   … and the shipped shelf is being shown"*;
//! * the copy `buffer_write_to` puts in a folder the reader chose, which may be
//!   a network share or a removable disk, and which is the copy they
//!   deliberately made to keep.
//!
//! So the argument lives here once, and the callers call it. Copying four lines
//! three more times would have worked and would have left the reason in five
//! places, which is how the fifth caller comes to be written without it.
//!
//! # What this does and does not promise
//!
//! It promises that the file at `path` is either the old contents or the new
//! ones, never a prefix of either. That is what `rename` gives on both
//! platforms this ships to: POSIX `rename(2)` is atomic, and Windows'
//! `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING` — which is what `fs::rename`
//! calls — replaces in one step.
//!
//! It does **not** promise durability against a power loss: neither the data
//! nor the directory entry is fsynced, so a machine that loses power may come
//! back to the old contents. That is the right trade for a file written every
//! 900 ms — an fsync per keystroke-pause is a stutter the reader would feel —
//! and it is worth stating rather than implying, because *atomic* and *durable*
//! are two words and this is one of them. The old contents are a save the
//! reader remembers making. An empty file is not.
//!
//! It is also not a lock. Two processes writing the same path still race, and
//! the loser's write disappears whole rather than half — see
//! [`Log::rewrite`](crate::Log::rewrite), which has the same exposure and says
//! so.

use std::path::{Path, PathBuf};

/// Where the half-written file lives while it is being written.
///
/// Beside the real one, in the same directory, because `rename` across a
/// filesystem boundary is a copy and a delete rather than one step — and the
/// system temp directory is routinely a different filesystem from the reader's
/// documents folder or a network share.
///
/// The extension is appended rather than replacing what is there, so
/// `סוגיא.typ` becomes `סוגיא.typ.writing` and not `סוגיא.writing`. Two
/// documents in one folder whose names differ only by extension would otherwise
/// share a temp file and overwrite each other's saves.
#[must_use]
pub fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".writing");
    PathBuf::from(name)
}

/// Write `body` to `path` so that the file is never seen half-written.
///
/// Creates the parent directory if it is not there. See the module note for
/// what this promises and what it does not.
///
/// # Errors
///
/// If the directory cannot be made, the file beside cannot be written, or the
/// rename over the real one fails. The half-written file is removed on the way
/// out of a failed rename, so a failure does not leave litter next to the
/// reader's documents.
pub fn write(path: &Path, body: impl AsRef<[u8]>) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let temp = temp_path(path);
    std::fs::write(&temp, body)?;
    if let Err(e) = std::fs::rename(&temp, path) {
        // Best effort, and deliberately not reported: the error worth returning
        // is the one that says the save did not happen, not a second one about
        // the cleanup of the file that proves it.
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("girsa-beside-tests");
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir.join(name)
    }

    #[test]
    fn the_file_is_the_new_contents_and_the_temp_file_is_gone() {
        let path = scratch("plain.txt");
        write(&path, "ראשון").expect("writes");
        write(&path, "שני").expect("writes again");
        assert_eq!(std::fs::read_to_string(&path).expect("reads"), "שני");
        assert!(
            !temp_path(&path).exists(),
            "the file beside is not left behind"
        );
    }

    #[test]
    fn a_missing_directory_is_made() {
        let path = scratch("made/up/deep.txt");
        let _ = std::fs::remove_dir_all(path.parent().expect("has a parent"));
        write(&path, "דבר").expect("writes");
        assert_eq!(std::fs::read_to_string(&path).expect("reads"), "דבר");
    }

    /// The temp name is derived from the whole file name and not from its stem.
    ///
    /// Two documents in one folder differing only by extension — which is
    /// exactly what a `.typ` and its exported `.ksav` are — must not share the
    /// file beside, or each save is racing the other one's.
    #[test]
    fn two_files_that_differ_only_by_extension_do_not_share_a_temp_file() {
        let one = scratch("סוגיא.typ");
        let two = scratch("סוגיא.ksav");
        assert_ne!(temp_path(&one), temp_path(&two));
    }
}
