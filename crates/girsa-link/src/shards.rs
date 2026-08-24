//! Holding lines by work, and writing them out so that running twice is the
//! same as running once.
//!
//! # Why this is a module and not a paragraph in two files
//!
//! [`crate::store::Writer`] and [`crate::inbound::Writer`] are two different
//! things — one files an edge under the work it comes **from** and the other
//! under the work it lands **on**, and only the second one writes a marker at
//! the end. What they were not two of is this: a `BTreeMap<String, String>` of
//! pending lines, a count, a set of shards this run has already opened, a
//! `buffered_bytes`, and a `flush` whose whole subtlety is one flag.
//!
//! `inbound.rs`'s header said so out loud — *"the same discipline as
//! [`crate::store::Writer`] and for the same reason"* — which is an accurate
//! description of a copy and is not a reason for one. The 9 August duplication
//! report filed it under *the copies that admit themselves in a comment*, and
//! the admission is the tell: somebody looked at both, understood they were the
//! same, wrote that down, and left two.
//!
//! # The flag, which is the only interesting line in here
//!
//! A run is many flushes, because four million edges do not fit in memory. So a
//! shard has to be **appended** to within a run. But appending to what the
//! *last* run left would silently double every link in the corpus — and the
//! import is a command somebody else is told to run, so running it twice has to
//! mean what running it once means.
//!
//! Hence `opened`: the first flush that touches a work truncates its shard, and
//! every flush after it in the same run appends. One bool, in one place, for
//! both trees.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Lines waiting to be written, by the work they belong to.
#[derive(Debug, Default)]
pub(crate) struct Shards {
    by_work: BTreeMap<String, String>,
    written: usize,
    /// Shards this writer has already opened, and so must not truncate again.
    opened: BTreeSet<String>,
}

impl Shards {
    /// Hold one line against `slug`. The trailing newline is this function's.
    ///
    /// The lookup comes before [`std::collections::BTreeMap::entry`] on
    /// purpose: `entry` needs an owned key, which would allocate the slug on
    /// every push — millions per import against thousands of works. A hit is
    /// the common case by three orders of magnitude, so the owned key is paid
    /// once per work and never per edge.
    pub(crate) fn add(&mut self, slug: &str, line: &str) {
        if let Some(body) = self.by_work.get_mut(slug) {
            body.push_str(line.trim_end());
            body.push('\n');
            self.written += 1;
            return;
        }
        let body = self.by_work.entry(slug.to_owned()).or_default();
        body.push_str(line.trim_end());
        body.push('\n');
        self.written += 1;
    }

    pub(crate) const fn len(&self) -> usize {
        self.written
    }

    /// How many bytes are being held, so a caller can flush before memory
    /// becomes the reason the run did not finish.
    pub(crate) fn buffered_bytes(&self) -> usize {
        self.by_work.values().map(String::len).sum()
    }

    /// Write everything held to the path `path_of` names for each work, and
    /// forget it.
    ///
    /// # Errors
    ///
    /// If a shard cannot be created or written to.
    pub(crate) fn flush(
        &mut self,
        root: &Path,
        path_of: impl Fn(&Path, &str) -> PathBuf,
    ) -> Result<(), std::io::Error> {
        for (slug, body) in std::mem::take(&mut self.by_work) {
            let path = path_of(root, &slug);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let first_touch = self.opened.insert(slug);
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(!first_touch)
                .write(first_touch)
                .truncate(first_touch)
                .open(&path)?;
            file.write_all(body.as_bytes())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn path_of(root: &Path, slug: &str) -> PathBuf {
        root.join(format!("{slug}.jsonl"))
    }

    #[test]
    fn running_it_twice_is_the_same_as_running_it_once() {
        // The property both writers had a test for, in two files, with two
        // names. It is one property and it belongs to `opened`.
        let dir = std::env::temp_dir().join("girsa-shards-twice");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch");

        let mut once = Shards::default();
        once.add("berakhot", "{\"a\":1}");
        once.flush(&dir, path_of).expect("flushes");
        let after_one = fs::read_to_string(dir.join("berakhot.jsonl")).expect("reads");

        let mut again = Shards::default();
        again.add("berakhot", "{\"a\":1}");
        again.flush(&dir, path_of).expect("flushes");
        assert_eq!(
            fs::read_to_string(dir.join("berakhot.jsonl")).expect("reads"),
            after_one,
            "a second run replaces the shard rather than doubling it"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_flush_in_one_run_appends() {
        // And the other half, which is the half that makes the flag necessary:
        // four million edges are many flushes, and the second one must not
        // throw the first one away.
        let dir = std::env::temp_dir().join("girsa-shards-appends");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch");

        let mut shards = Shards::default();
        shards.add("berakhot", "one");
        shards.flush(&dir, path_of).expect("flushes");
        shards.add("berakhot", "two");
        shards.flush(&dir, path_of).expect("flushes");

        assert_eq!(
            fs::read_to_string(dir.join("berakhot.jsonl")).expect("reads"),
            "one\ntwo\n"
        );
        assert_eq!(shards.len(), 2);
        assert_eq!(shards.buffered_bytes(), 0, "flushing forgets what it wrote");

        let _ = fs::remove_dir_all(&dir);
    }
}
