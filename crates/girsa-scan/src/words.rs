//! Where a scan's words are kept, and what is left to read.
//!
//! `personal/words/<slug>/pages.jsonl` — one line per page, under the personal
//! root beside the paging, the corrections and the link repairs, under the same
//! rule those three are under: **nothing here writes into `corpus/`.** A
//! reading is a machine's opinion about a file only the reader has.
//!
//! # Append, and why that is safe here when it was not in W8
//!
//! W8 shipped an importer that opened its shards in append mode and doubled the
//! graph on a second run, and W25's store was written whole every time because
//! of it. This one appends, and cannot double, because **a line is keyed by its
//! page and the last line for a page wins**. Reading a page twice leaves two
//! lines and one reading; the file gets longer and never gets wrong.
//!
//! That is worth the asymmetry, because appending is what makes the job
//! resumable without a second file to keep in step. **The work product is the
//! progress record**: the pages in the file are the pages that are done, so
//! there is no state that can survive a crash while disagreeing with what was
//! actually read.
//!
//! # A page that was read and turned out to be blank is still read
//!
//! It gets a line with no words in it. Leaving it out would put it back in the
//! queue on every run, and a scan with forty blank versos would never finish —
//! and worse, spec.md §9.7's *"4 PDFs on this shelf aren't searchable yet"*
//! would keep naming a sefer that has been read cover to cover.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use girsa_corpus::import::slug_dir;

use crate::reading::{Fix, Read};

/// Why a reading could not be written or read back.
#[derive(Debug, thiserror::Error)]
pub enum WordsError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Malformed(String),
}

impl WordsError {
    fn io(path: &Path) -> impl Fn(std::io::Error) -> Self + '_ {
        move |source| Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

/// The pages of one scan that have been read.
#[derive(Debug, Clone, Default)]
pub struct Words {
    path: Option<PathBuf>,
    fixes_path: Option<PathBuf>,
    by_page: BTreeMap<usize, Read>,
    fixes: BTreeMap<usize, Vec<Fix>>,
    /// Lines in the log, live and superseded — what tells a log that has grown
    /// from one that has not.
    written: usize,
}

impl Words {
    /// Open one scan's readings, and say what would not read.
    ///
    /// A line that will not parse is **named and skipped**, the way the
    /// corrections layer and the link repairs do it: one unreadable page may
    /// not cost the reader the other three hundred, and it may not be silent
    /// either, because a page that quietly vanished from the index is spec.md
    /// §9.7's gap with nothing to say about it.
    #[must_use]
    pub fn open(personal: &Path, slug: &str) -> (Self, Vec<String>) {
        let dir = Self::dir_in(personal, slug);
        let path = dir.join("pages.jsonl");
        let fixes_path = dir.join("fixes.json");
        let mut trouble = Vec::new();

        let mut by_page = BTreeMap::new();
        let mut lines = 0usize;
        if let Ok(body) = std::fs::read_to_string(&path) {
            for (line, text) in body.lines().enumerate() {
                if text.trim().is_empty() {
                    continue;
                }
                lines += 1;
                match serde_json::from_str::<Read>(text) {
                    // Last line for a page wins: this is a log, not a table.
                    Ok(read) => {
                        by_page.insert(read.page, read);
                    }
                    Err(e) => trouble.push(format!(
                        "{}:{} will not read: {e}",
                        path.display(),
                        line + 1
                    )),
                }
            }
        }

        let mut fixes: BTreeMap<usize, Vec<Fix>> = BTreeMap::new();
        if let Ok(body) = std::fs::read_to_string(&fixes_path) {
            match serde_json::from_str(&body) {
                Ok(read) => fixes = read,
                Err(e) => trouble.push(format!("{} will not read: {e}", fixes_path.display())),
            }
        }

        (
            Self {
                path: Some(path),
                fixes_path: Some(fixes_path),
                written: lines,
                by_page,
                fixes,
            },
            trouble,
        )
    }

    /// A set that is not backed by a file, for a caller with no personal layer.
    #[must_use]
    pub fn nowhere() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn dir_in(personal: &Path, slug: &str) -> PathBuf {
        slug_dir(&personal.join("words"), slug)
    }

    /// A page as it was read, with this reader's corrections applied.
    ///
    /// Corrections are re-found by their ink every time rather than baked in
    /// when they are made, which is what makes re-reading the page with a
    /// better engine cost nothing — see [`crate::reading::corrected`].
    #[must_use]
    pub fn page(&self, page: usize) -> Option<Read> {
        let read = self.by_page.get(&page)?;
        match self.fixes.get(&page) {
            None => Some(read.clone()),
            Some(fixes) => Some(crate::reading::corrected(read, fixes).0),
        }
    }

    /// The page as the engine left it, corrections not applied. What the repair
    /// screen shows beside the photograph.
    #[must_use]
    pub fn as_read(&self, page: usize) -> Option<&Read> {
        self.by_page.get(&page)
    }

    /// Corrections whose ink the current reading has no word under — the
    /// reader marked something and this engine found nothing there.
    ///
    /// Reported rather than dropped. Every page, so the window can offer the
    /// list after a re-read rather than the reader discovering it a year later.
    #[must_use]
    pub fn stranded(&self) -> Vec<(usize, Fix)> {
        let mut out = Vec::new();
        for (page, fixes) in &self.fixes {
            if let Some(read) = self.by_page.get(page) {
                for fix in crate::reading::corrected(read, fixes).1 {
                    out.push((*page, fix));
                }
            }
        }
        out
    }

    /// Which pages have been read at all.
    #[must_use]
    pub fn pages_read(&self) -> usize {
        self.by_page.len()
    }

    /// Whether anything has been read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_page.is_empty()
    }

    /// Whether this page has been read — including read and found blank.
    #[must_use]
    pub fn has(&self, page: usize) -> bool {
        self.by_page.contains_key(&page)
    }

    /// Which engines have been over this scan.
    ///
    /// More than one is normal and is not a fault: a PDF can carry its own text
    /// for the pages that were typeset and none for the plates, and the two
    /// halves are read by different things. The badge is per hit, which is why
    /// this is a set and not a field.
    #[must_use]
    pub fn read_by(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .by_page
            .values()
            .map(|read| read.by.name().to_string())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Write down what a page says.
    ///
    /// # Errors
    ///
    /// If the personal layer will not take it.
    pub fn record(&mut self, read: Read) -> Result<(), WordsError> {
        if let Some(path) = self.path.clone() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(WordsError::io(dir))?;
            }
            let line =
                serde_json::to_string(&read).map_err(|e| WordsError::Malformed(e.to_string()))?;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(WordsError::io(&path))?;
            writeln!(file, "{line}").map_err(WordsError::io(&path))?;
            self.written += 1;
        }
        self.by_page.insert(read.page, read);
        self.compact_if_it_has_grown()
    }

    /// Rewrite the log when it has grown past twice what it holds.
    ///
    /// The same rule `girsa_personal::Log` uses, and for the same reason: this
    /// is an append-only log keyed by page, a later line for a page wins, and
    /// **re-reading a page appends another copy of it forever**. Reading page
    /// *k* of a 500-page masechta with a better engine parsed every superseded
    /// read of every page on the way. A page is not a small record — it is
    /// every word on it and where each one sits.
    ///
    /// Written beside and renamed over, so a machine that stops mid-compaction
    /// has either the old log or the new one.
    fn compact_if_it_has_grown(&mut self) -> Result<(), WordsError> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        if self.written <= self.by_page.len() * 2 {
            return Ok(());
        }
        let mut body = String::new();
        for read in self.by_page.values() {
            let line =
                serde_json::to_string(read).map_err(|e| WordsError::Malformed(e.to_string()))?;
            body.push_str(&line);
            body.push(0x0a as char);
        }
        let temp = path.with_extension("jsonl.writing");
        std::fs::write(&temp, body).map_err(WordsError::io(&temp))?;
        std::fs::rename(&temp, &path).map_err(WordsError::io(&path))?;
        self.written = self.by_page.len();
        Ok(())
    }

    /// Correct a word on a page.
    ///
    /// # Errors
    ///
    /// If the personal layer will not write.
    pub fn fix(&mut self, page: usize, fix: Fix) -> Result<(), WordsError> {
        self.fixes.entry(page).or_default().push(fix);
        self.save_fixes()
    }

    /// Every correction on a page, as they were made.
    #[must_use]
    pub fn fixes(&self, page: usize) -> &[Fix] {
        self.fixes.get(&page).map_or(&[], Vec::as_slice)
    }

    fn save_fixes(&self) -> Result<(), WordsError> {
        let Some(path) = &self.fixes_path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(WordsError::io(dir))?;
        }
        let body = serde_json::to_string_pretty(&self.fixes)
            .map_err(|e| WordsError::Malformed(e.to_string()))?;
        std::fs::write(path, body).map_err(WordsError::io(path))
    }
}

/// What is left to read of one scan.
///
/// # Never blocking reading
///
/// spec.md §6.3: OCR *runs in the background, resumable, never blocking
/// reading*. That is a shape as much as a promise — this hands out **one page
/// at a time** and holds nothing open, so the caller can stop between any two
/// pages and the only cost of stopping is the page it was on. There is no
/// batch to finish and no lock on the scan: the reader can open the sefer, page
/// through it and cite from it while it runs, because everything the window
/// needs is the PDF and everything this needs is one page of it.
#[derive(Debug, Clone)]
pub struct Job {
    slug: String,
    pages: usize,
    done: Vec<bool>,
}

impl Job {
    /// What is left of a scan, given what has been read.
    #[must_use]
    pub fn of(slug: &str, pages: usize, words: &Words) -> Self {
        Self {
            slug: slug.to_string(),
            pages,
            done: (1..=pages).map(|page| words.has(page)).collect(),
        }
    }

    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// The next page to read: the lowest one that has not been.
    ///
    /// In order, because a reader who starts the job and then opens the sefer
    /// is at the front of it, and because a job that skipped about would make
    /// *"how far has it got"* unanswerable.
    #[must_use]
    pub fn next(&self) -> Option<usize> {
        self.done.iter().position(|done| !done).map(|at| at + 1)
    }

    /// Say a page is done. Called after the reading is recorded, never before —
    /// the file is the progress record and this is only its shadow in memory.
    pub fn did(&mut self, page: usize) {
        if let Some(slot) = page.checked_sub(1).and_then(|at| self.done.get_mut(at)) {
            *slot = true;
        }
    }

    #[must_use]
    pub fn pages(&self) -> usize {
        self.pages
    }

    /// Whether one page has been read.
    #[must_use]
    pub fn is_done(&self, page: usize) -> bool {
        page.checked_sub(1)
            .and_then(|at| self.done.get(at))
            .copied()
            .unwrap_or(false)
    }

    #[must_use]
    pub fn done(&self) -> usize {
        self.done.iter().filter(|done| **done).count()
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.pages - self.done()
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.remaining() == 0
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::reading::{Area, Reader, Word};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-words-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn page(n: usize, text: &str) -> Read {
        Read::new(
            n,
            Reader::Embedded,
            text.split(' ')
                .enumerate()
                .map(|(at, word)| Word {
                    text: word.to_string(),
                    #[allow(clippy::cast_precision_loss)]
                    at: Area::new(0.8 - at as f32 * 0.06, 0.2, 0.85 - at as f32 * 0.06, 0.22),
                    confidence: 1.0,
                })
                .collect(),
        )
    }

    #[test]
    fn a_reading_survives_being_written_down_and_read_back() {
        let dir = scratch("round-trip");
        let (mut words, trouble) = Words::open(&dir, "user/berachos");
        assert!(trouble.is_empty(), "{trouble:?}");
        words
            .record(page(7, "מאימתי קורין את שמע"))
            .expect("writes");

        let (again, trouble) = Words::open(&dir, "user/berachos");
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(
            again.page(7).map(|p| p.text()).as_deref(),
            Some("מאימתי קורין את שמע")
        );
        assert_eq!(again.pages_read(), 1);
    }

    #[test]
    fn reading_a_page_twice_leaves_one_reading() {
        // W8's importer doubled its graph on a second run. A log keyed by page
        // cannot: the file grows and the answer does not change.
        let dir = scratch("twice");
        let (mut words, _) = Words::open(&dir, "user/x");
        words.record(page(3, "קורין")).expect("writes");
        words.record(page(3, "קוראין")).expect("writes");

        let (again, _) = Words::open(&dir, "user/x");
        assert_eq!(again.pages_read(), 1);
        assert_eq!(again.page(3).map(|p| p.text()).as_deref(), Some("קוראין"));
    }

    #[test]
    fn a_page_read_and_found_blank_does_not_come_back_round_the_queue() {
        let dir = scratch("blank");
        let (mut words, _) = Words::open(&dir, "user/x");
        words
            .record(Read::new(2, Reader::Embedded, Vec::new()))
            .expect("writes");

        assert!(words.has(2));
        let job = Job::of("user/x", 4, &words);
        assert_eq!(job.next(), Some(1));
        assert_eq!(job.done(), 1);
    }

    #[test]
    fn the_queue_is_what_is_on_disk_so_it_resumes_where_it_stopped() {
        let dir = scratch("resume");
        let (mut words, _) = Words::open(&dir, "user/x");
        let mut job = Job::of("user/x", 302, &words);
        // Read forty pages and stop, the way a reader closing the window does.
        while let Some(page) = job.next() {
            if page > 40 {
                break;
            }
            words.record(page_blank(page)).expect("writes");
            job.did(page);
        }
        assert_eq!(job.done(), 40);

        // Nothing else was written down. The file is the progress record.
        let (again, trouble) = Words::open(&dir, "user/x");
        assert!(trouble.is_empty(), "{trouble:?}");
        let resumed = Job::of("user/x", 302, &again);
        assert_eq!(resumed.next(), Some(41));
        assert_eq!(resumed.done(), 40);
        assert_eq!(resumed.remaining(), 262);
        assert!(!resumed.is_finished());
    }

    fn page_blank(n: usize) -> Read {
        Read::new(n, Reader::Embedded, Vec::new())
    }

    #[test]
    fn a_line_that_will_not_parse_costs_one_page_and_names_it() {
        let dir = scratch("nonsense");
        let path = Words::dir_in(&dir, "user/x");
        std::fs::create_dir_all(&path).expect("a directory");
        std::fs::write(
            path.join("pages.jsonl"),
            "{\"page\":1,\"by\":\"embedded\",\"words\":[]}\nnot json at all\n",
        )
        .expect("writes");

        let (words, trouble) = Words::open(&dir, "user/x");
        assert_eq!(words.pages_read(), 1);
        assert_eq!(trouble.len(), 1);
        assert!(trouble[0].contains(":2"), "{trouble:?}");
    }

    #[test]
    fn a_correction_is_applied_on_the_way_out_and_never_written_into_the_reading() {
        let dir = scratch("fix");
        let (mut words, _) = Words::open(&dir, "user/x");
        words
            .record(page(7, "מאימתי קודין את שמע"))
            .expect("writes");
        let ink = words.as_read(7).expect("a reading").words[1].at;
        words
            .fix(
                7,
                Fix {
                    at: ink,
                    was: "קודין".into(),
                    says: "קורין".into(),
                },
            )
            .expect("writes");

        let (again, trouble) = Words::open(&dir, "user/x");
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(
            again.page(7).map(|p| p.text()).as_deref(),
            Some("מאימתי קורין את שמע")
        );
        // The engine's own reading is untouched, which is what makes the repair
        // screen able to show what was corrected.
        assert_eq!(
            again.as_read(7).map(Read::text).as_deref(),
            Some("מאימתי קודין את שמע")
        );
        assert!(again.stranded().is_empty());
    }
}
