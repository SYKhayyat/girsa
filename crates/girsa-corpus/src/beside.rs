//! One whole-file write, done the only way a whole-file write may be done.
//!
//! `std::fs::write` opens with `O_TRUNC`. From the moment of the open until the
//! write returns, the file on disk is **empty**. A crash, a power loss, a full
//! disk or a killed process inside that window leaves a truncated file where
//! the reader's shelf was — and here the file that was being rewritten is
//! `segments.jsonl`, the very thing that says which permanent name belongs to
//! which text. A torn one reads back as nothing, [`crate::import::Previous`]
//! believes it, and the next import mints ordinals from enumeration position
//! again: every correction, mark and link anchored to the old names now points
//! at shifted text. That is T1, through a tidy-up.
//!
//! The argument in full — what is promised, what is not, and why rename rather
//! than anything cleverer — lives in `girsa_personal::beside`, which had it
//! before this crate did. This crate cannot depend on that one (this is the
//! leaf), so the four lines live here too; the name is the same because the
//! doctrine is.
//!
//! # What this does not promise
//!
//! Atomic against a crash: yes — the file at `path` is always either the old
//! contents or the new ones, never a prefix of either. Durable against power
//! loss: no — neither the bytes nor the directory entry are fsynced, which for
//! an import run of hours is the right trade by a wide margin.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Where the half-written file lives while it is being written.
///
/// Beside the real one, in the same directory, because `rename` across a
/// filesystem boundary is a copy and a delete rather than one step. The
/// extension is **appended** rather than replaced, so `segments.jsonl` becomes
/// `segments.jsonl.part` and not `segments.part`: two files whose names differ
/// only by extension would otherwise share a temp file and race each other's
/// writes — which is exactly the shape a work's `work.json` and a hypothetical
/// sibling do not have but its `.ksav` exports do.
#[must_use]
pub fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".part");
    PathBuf::from(name)
}

/// Write `body` to `path` so that the file is never seen half-written.
///
/// Creates the parent directory if it is not there.
///
/// # Errors
///
/// If the directory cannot be made, the file beside cannot be written, or the
/// rename over the real one fails. The half-written file is removed on the way
/// out of a failed rename, so a failure does not leave litter beside the shelf.
pub fn write(path: &Path, body: impl AsRef<[u8]>) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let temp = temp_path(path);
    {
        let mut f = std::fs::File::create(&temp)?;
        f.write_all(body.as_ref())?;
        f.flush()?;
    }
    if let Err(e) = std::fs::rename(&temp, path) {
        // Best effort, and deliberately not reported: the error worth returning
        // is the one that says the write did not happen, not a second one about
        // cleaning up the file that proves it.
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
        let dir = std::env::temp_dir().join("girsa-corpus-beside");
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir.join(name)
    }

    #[test]
    fn the_file_is_the_new_contents_and_the_temp_file_is_gone() {
        let path = scratch("plain.jsonl");
        write(&path, "ראשון").expect("writes");
        write(&path, "שני").expect("writes again");
        assert_eq!(std::fs::read_to_string(&path).expect("reads"), "שני");
        assert!(!temp_path(&path).exists(), "no litter beside the shelf");
    }

    #[test]
    fn two_files_that_differ_only_by_extension_do_not_share_a_temp_file() {
        let one = scratch("berakhot.json");
        let two = scratch("berakhot.jsonl");
        assert_ne!(temp_path(&one), temp_path(&two));
    }

    /// The whole point: a reader of the final path never sees a prefix.
    #[test]
    fn a_missing_directory_is_made_and_the_rename_is_one_step() {
        let path = scratch("made/up/deep/segments.jsonl");
        let _ = std::fs::remove_dir_all(path.parent().expect("has a parent"));
        write(&path, "{\"id\":\"a\"}\n").expect("writes");
        assert_eq!(
            std::fs::read_to_string(&path).expect("reads"),
            "{\"id\":\"a\"}\n"
        );
    }
}
