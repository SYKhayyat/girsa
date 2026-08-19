//! One writer at a time, across processes.
//!
//! # What this is for, and what it is not
//!
//! [`Log`](crate::Log) is append-only, and append-only is already safe against a
//! second writer for the write it does: one `write_all` on a handle opened for
//! append lands whole, and two processes appending interleave lines rather than
//! corrupting each other.
//!
//! Compaction is the exception, and it is the one place where an append-only
//! log stops being append-only. `Log::rewrite` reads the live records, writes
//! them beside, and renames over. That is atomic against a **crash** and not
//! against a **second writer**: an append made between the read and the rename
//! lands in the file the rename is about to replace, and it is gone with no
//! error on either side.
//!
//! Girsa ships more than one process that opens the same personal layer — the
//! MCP server, whose whole point is that a program can write into your own
//! layer, and `girsa-suspects` — so this is not hypothetical, merely narrow.
//!
//! # Why a file and not a real lock
//!
//! `flock` and `LockFileEx` are foreign calls, and this workspace is `unsafe`-
//! free without a platform crate. `OpenOptions::create_new` is neither: it is
//! `O_EXCL` on POSIX and `CREATE_NEW` on Windows, both of which the filesystem
//! makes atomic, and both of which fail rather than truncate when the file is
//! already there. That is the whole of a mutex.
//!
//! What it costs is that a **crashed** holder leaves the file behind, so this
//! breaks a lock older than [`STALE_AFTER`] rather than wedging the reader's
//! layer for ever. Losing a compaction is a longer file; refusing to ever write
//! again is a lost layer, and between those two the choice is not close.
//!
//! # It is advisory
//!
//! Nothing enforces it. It works because every write in this crate goes through
//! one of two functions and both take it. A seventh store that writes the file
//! by hand is outside it, which is the usual price of an advisory lock and the
//! reason the writes are funnelled rather than merely documented.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How long a lock may be held before another process assumes its holder died.
///
/// Generous by the standard of what it guards: the longest compaction in this
/// layer is a few megabytes of JSON, which is milliseconds. Thirty seconds is
/// *the machine was swapping, or a debugger was attached*, not *the work is
/// still going*.
pub const STALE_AFTER: Duration = Duration::from_secs(30);

/// How long to keep trying before giving up and doing the write anyway.
///
/// Giving up and writing is the right end of this: the caller is holding a
/// reader's own data and the alternative to a racy write is no write. The
/// window is small enough that it never comes up and honest enough to state.
const WAIT_FOR: Duration = Duration::from_secs(2);

/// One try's pause. Short, because the thing being waited for is milliseconds
/// long.
const BETWEEN: Duration = Duration::from_millis(10);

/// A held lock. Releases when it drops, however the scope ends.
#[derive(Debug)]
pub struct Held {
    path: PathBuf,
}

impl Drop for Held {
    fn drop(&mut self) {
        // Best effort and deliberately silent. A lock file that outlives its
        // holder is broken by the next writer after `STALE_AFTER`; an error
        // reported from a destructor is noise about a file nobody asked about.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Where the lock for a file lives: beside it, named after it.
#[must_use]
pub fn lock_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".lock");
    PathBuf::from(name)
}

/// Take the lock on `path`, waiting for whoever has it.
///
/// Returns `None` if the wait ran out, which is the caller's cue to go ahead
/// anyway — see [`WAIT_FOR`]. It is never an error, because there is nothing
/// useful for a caller holding a reader's data to do with one.
#[must_use]
pub fn hold(path: &Path) -> Option<Held> {
    let lock = lock_path(path);
    let until = SystemTime::now() + WAIT_FOR;
    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
        {
            Ok(_) => return Some(Held { path: lock }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if is_stale(&lock) {
                    // The holder is gone. Removing it does not take the lock —
                    // the next turn of the loop does that, and if another
                    // process got there first this one waits for it.
                    let _ = std::fs::remove_file(&lock);
                    continue;
                }
                if SystemTime::now() >= until {
                    return None;
                }
                std::thread::sleep(BETWEEN);
            }
            // The directory is not there, or the layer is read-only. Neither is
            // a thing waiting fixes, and the caller's own write is about to
            // report it properly.
            Err(_) => return None,
        }
    }
}

/// Whether a lock file is old enough that its holder is presumed dead.
///
/// A file with no readable modified time counts as stale: it is either a
/// filesystem that does not keep one, where waiting for ever is the worse
/// failure, or a file that is being removed underneath us, where the next try
/// succeeds anyway.
fn is_stale(lock: &Path) -> bool {
    let Ok(when) = std::fs::metadata(lock).and_then(|m| m.modified()) else {
        return true;
    };
    SystemTime::now()
        .duration_since(when)
        .is_ok_and(|since| since > STALE_AFTER)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("girsa-lock-tests");
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let path = dir.join(name);
        let _ = std::fs::remove_file(lock_path(&path));
        path
    }

    #[test]
    fn one_holder_at_a_time_and_the_file_goes_when_it_drops() {
        let path = scratch("held.jsonl");
        let held = hold(&path).expect("takes it");
        assert!(lock_path(&path).exists());
        // A second try inside one process is the same contention a second
        // process makes, and it waits and then gives up rather than pretending.
        assert!(hold(&path).is_none(), "the lock is not re-entrant");
        drop(held);
        assert!(!lock_path(&path).exists(), "and it is released on drop");
        assert!(hold(&path).is_some(), "so the next writer gets it");
    }

    /// A crashed holder must not wedge the layer for ever.
    #[test]
    fn a_lock_older_than_the_limit_is_broken() {
        let path = scratch("stale.jsonl");
        let lock = lock_path(&path);
        std::fs::write(&lock, "").expect("writes a lock nobody holds");
        // `is_stale` reads the file's own modified time, so the fence is on
        // that function rather than on a test that sleeps for half a minute.
        assert!(!is_stale(&lock), "a lock taken now is not stale");
        let _ = std::fs::remove_file(&lock);
        assert!(is_stale(&lock), "and one that is not there does not block");
    }
}
