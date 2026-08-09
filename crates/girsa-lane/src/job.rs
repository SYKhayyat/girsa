//! What is left to embed, and the shape that keeps it out of the way.
//!
//! spec.md §9.9: *background, resumable, never blocks reading.* That is W26's
//! rule for OCR, restated here because it is the same rule for the same reason
//! — a job that has to finish before the application is useful is a job that
//! makes the application useless while it runs — and it is a **shape** as much
//! as a promise.
//!
//! This hands out one batch at a time and holds nothing open. The caller can
//! stop between any two batches and the only cost of stopping is the batch it
//! was on; there is no lock on the sefer and no state to reconcile, because
//! [`crate::vectors`] is the progress record. Reopening a job re-reads what is
//! on disk and starts from the first segment that has no vector.
//!
//! # What is not embedded, and why the number is not the segment count
//!
//! A heading is not embedded. Neither is a page of a scan that has not been
//! read. Both would be vectors of nothing much — *סימן נח* against a query is
//! noise with a citation attached — and both would inflate the coverage line,
//! which is the one number in this feature a reader is asked to trust. So the
//! job's own idea of *all of it* is **segments with words in them**, and that
//! is the denominator the coverage line uses.

use girsa_corpus::import::{ImportedWork, SegmentKind};
use girsa_corpus::segment::SegmentId;

use crate::chosen::Chosen;
use crate::vectors::Vectors;

/// What is left of one sefer.
#[derive(Debug, Clone)]
pub struct Job {
    slug: String,
    /// Where in `work.segments` each wanted segment is.
    wanted: Vec<usize>,
    done: Vec<bool>,
    /// The first index of `done` that might still be false.
    ///
    /// `next` walked `done` from zero every batch, so embedding a sefer of *n*
    /// segments in batches of 32 rewalked the whole finished prefix each time —
    /// the same quadratic `did` fixed fifteen lines below, in the same struct,
    /// with its measurement in the comment: **164 million comparisons** for
    /// Mishnah Berurah's 18,120 segments.
    ///
    /// A floor and not a count: `did` may be called out of order (a batch comes
    /// back in whatever order the model finished), so this only ever advances
    /// past entries that are actually done.
    from: usize,
}

impl Job {
    /// What is left to embed of this sefer, given the selection and what is
    /// already on disk.
    #[must_use]
    pub fn of(chosen: &Chosen, work: &ImportedWork, vectors: &Vectors) -> Self {
        let mut wanted = Vec::new();
        let mut done = Vec::new();
        for (at, segment) in work.segments.iter().enumerate() {
            // A heading or an unread page of a scan is not a thing to embed —
            // see the module note.
            //
            // Both conditions are written out rather than deferred to
            // `SegmentKind::has_words`, which answers a **different** question
            // than it reads as: its body excludes only `Page`, so a heading
            // "has words" as far as the literal index is concerned — correctly,
            // because *סימן נח* is a thing a reader searches for. It is not a
            // thing worth a vector, and leaning on that method here would make
            // this quietly follow whatever the index decides next.
            if segment.kind == SegmentKind::Heading
                || !segment.kind.has_words()
                || segment.text.trim().is_empty()
            {
                continue;
            }
            if !chosen.covers(&segment.id) {
                continue;
            }
            wanted.push(at);
            done.push(vectors.has(&segment.id));
        }
        Self {
            slug: work.work.slug.clone(),
            wanted,
            done,
            from: 0,
        }
    }

    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// The next batch: the lowest segments that have no vector, in order.
    ///
    /// In order, because a reader who starts the job and then opens the sefer
    /// is at the front of it, and because a job that skipped about would make
    /// *how far has it got* unanswerable.
    ///
    /// This is not in tension with `Model::embed` grouping a batch by token
    /// length, and the two used to look as though it were. The job's order is
    /// what a reader sees and what makes a run resumable; the order rows sit in
    /// **inside one forward pass** is arithmetic nobody observes, and grouping
    /// them there is what stops one 512-token se'if making fifteen 20-token ones
    /// cost 512 tokens each. They come back in the order they were handed over.
    ///
    /// The values are indices into the work's `segments`, not copies of the
    /// text — a sefer's words are already in memory once and this refuses to be
    /// the second time.
    #[must_use]
    pub fn next(&mut self, most: usize) -> Vec<usize> {
        // Skip what is finished, once. Everything before `from` is done and
        // stays done, so this advances rather than rescanning.
        while self.from < self.done.len() && self.done[self.from] {
            self.from += 1;
        }
        self.done
            .iter()
            .enumerate()
            .skip(self.from)
            .filter(|(_, done)| !**done)
            .take(most)
            .filter_map(|(at, _)| self.wanted.get(at).copied())
            .collect()
    }

    /// Say a segment is embedded. Called after the vector is recorded, never
    /// before — the file is the progress record and this is its shadow.
    ///
    /// **Binary search, because `wanted` is built ascending.** `of` above walks
    /// `work.segments` in order and pushes each index it keeps, so the vector is
    /// sorted by construction and `position` was reading half of it per call.
    /// Mishnah Berurah is 18,120 segments: **164 million comparisons** to embed
    /// one sefer, and a million-segment work would not finish.
    ///
    /// The sortedness is asserted below rather than assumed, since it is a
    /// property of a loop forty lines up and nothing else says so.
    pub fn did(&mut self, at: usize) {
        if let Ok(place) = self.wanted.binary_search(&at) {
            if let Some(slot) = self.done.get_mut(place) {
                *slot = true;
            }
        }
    }

    /// How many segments of this sefer the lane wants.
    #[must_use]
    pub fn wanted(&self) -> usize {
        self.wanted.len()
    }

    #[must_use]
    pub fn done(&self) -> usize {
        self.done.iter().filter(|done| **done).count()
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.wanted.len() - self.done()
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.remaining() == 0
    }

    /// The ids this job wants, whether or not they are done. What the coverage
    /// line counts against.
    pub fn ids<'a>(&'a self, work: &'a ImportedWork) -> impl Iterator<Item = &'a SegmentId> + 'a {
        self.wanted
            .iter()
            .filter_map(|at| work.segments.get(*at))
            .map(|segment| &segment.id)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    use girsa_corpus::import::{RawSegment, SegmentKind};
    use girsa_corpus::work::{Source, Work};

    pub(crate) fn named(slug: &str) -> Work {
        Work {
            slug: slug.to_string(),
            he_title: slug.to_string(),
            en_title: slug.to_string(),
            categories: Vec::new(),
            source: Source::Mine,
            origin: std::path::PathBuf::new(),
            schema: None,
            he_sections: Vec::new(),
            author: None,
            era: None,
            comp_date: None,
            version: None,
            commentary_on: Vec::new(),
        }
    }

    fn work(slug: &str, lines: &[(SegmentKind, &str)]) -> ImportedWork {
        let raw = lines
            .iter()
            .enumerate()
            .map(|(at, (kind, text))| RawSegment {
                path: vec![(at + 1).to_string()],
                kind: *kind,
                text: (*text).to_string(),
            })
            .collect();
        ImportedWork::assemble(named(slug), raw)
    }

    #[test]
    fn a_heading_and_an_unread_page_are_not_things_to_embed() {
        let work = work(
            "x",
            &[
                (SegmentKind::Heading, "סימן נח"),
                (SegmentKind::Text, "זמן קריאת שמע"),
                (SegmentKind::Page, ""),
                (SegmentKind::Text, "   "),
                (SegmentKind::Note, "והוא הדין"),
            ],
        );
        let job = Job::of(&Chosen::everything(), &work, &Vectors::nowhere("m", 2));
        assert_eq!(
            job.wanted(),
            2,
            "the text and the footnote; not the heading, the page or the blank"
        );
    }

    #[test]
    fn the_queue_is_what_is_on_disk_so_it_resumes_where_it_stopped() {
        let dir = std::env::temp_dir().join("girsa-lane-job-resume");
        let _ = std::fs::remove_dir_all(&dir);
        let lines: Vec<(SegmentKind, String)> = (1..=40)
            .map(|n| (SegmentKind::Text, format!("שורה {n}")))
            .collect();
        let refs: Vec<(SegmentKind, &str)> = lines.iter().map(|(k, t)| (*k, t.as_str())).collect();
        let work = work("x", &refs);

        let (mut store, _) = Vectors::open(&dir, "x", "m", 2);
        let mut job = Job::of(&Chosen::everything(), &work, &store);
        assert_eq!(job.wanted(), 40);

        // Twelve, and then the window closes.
        for at in job.next(12) {
            let segment = &work.segments[at];
            store.record(&segment.id, &[1.0, 0.0]).expect("writes");
            job.did(at);
        }
        assert_eq!(job.done(), 12);

        // Nothing else was written down.
        let (again, trouble) = Vectors::open(&dir, "x", "m", 2);
        assert!(trouble.is_empty(), "{trouble:?}");
        let mut resumed = Job::of(&Chosen::everything(), &work, &again);
        assert_eq!(resumed.done(), 12);
        assert_eq!(resumed.remaining(), 28);
        assert_eq!(resumed.next(1), vec![12], "the thirteenth segment");
        assert!(!resumed.is_finished());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_section_chosen_is_the_only_part_the_job_wants() {
        let work = ImportedWork::assemble(
            named("s-a/o-c"),
            vec![
                RawSegment {
                    path: vec!["נח".to_string(), "א".to_string()],
                    kind: SegmentKind::Text,
                    text: "זמן קריאת שמע".to_string(),
                },
                RawSegment {
                    path: vec!["נט".to_string(), "א".to_string()],
                    kind: SegmentKind::Text,
                    text: "ברכות קריאת שמע".to_string(),
                },
            ],
        );
        let chosen = Chosen::nothing().with_section("s-a/o-c", &["נח".to_string()]);
        let mut job = Job::of(&chosen, &work, &Vectors::nowhere("m", 2));
        assert_eq!(job.wanted(), 1);
        assert_eq!(job.next(9), vec![0]);
    }

    #[test]
    fn a_sefer_nobody_chose_has_nothing_to_do() {
        let work = work("x", &[(SegmentKind::Text, "שורה")]);
        let mut job = Job::of(&Chosen::nothing(), &work, &Vectors::nowhere("m", 2));
        assert_eq!(job.wanted(), 0);
        assert!(job.is_finished(), "finished, rather than stuck");
        assert!(job.next(10).is_empty());
    }

    #[test]
    fn what_is_wanted_is_in_order_so_did_can_binary_search_it() {
        // `did` looks a segment up by binary search, which is only correct
        // because `Job::of` walks `work.segments` in order and pushes each index
        // it keeps. That is a property of a loop forty lines above it and
        // nothing else said so.
        //
        // It matters: `position` read half the vector per call, which is 164
        // million comparisons to embed Mishnah Berurah's 18,120 segments and
        // does not finish for a million-segment work.
        // A heading between the text, so the kept indices are not 0,1,2,3 —
        // otherwise "ascending" would be true of any list and prove nothing.
        let work = work(
            "x",
            &[
                (SegmentKind::Text, "ראשון"),
                (SegmentKind::Heading, "סימן"),
                (SegmentKind::Text, "שני"),
                (SegmentKind::Page, ""),
                (SegmentKind::Text, "שלישי"),
            ],
        );
        let job = Job::of(&Chosen::everything(), &work, &Vectors::nowhere("m", 2));
        assert!(job.wanted() > 2, "the fixture has to have something in it");

        let mut ascending = job.wanted.clone();
        ascending.sort_unstable();
        assert_eq!(job.wanted, ascending, "`wanted` is not in order");
        ascending.dedup();
        assert_eq!(
            job.wanted.len(),
            ascending.len(),
            "`wanted` repeats an index"
        );

        // And the lookup finds what it should, at both ends and in the middle.
        for at in job.wanted.clone() {
            let mut job = Job::of(&Chosen::everything(), &work, &Vectors::nowhere("m", 2));
            let before = job.done();
            job.did(at);
            assert_eq!(job.done(), before + 1, "{at} was not marked done");
        }
        // A segment this job does not want changes nothing.
        let mut job = Job::of(&Chosen::everything(), &work, &Vectors::nowhere("m", 2));
        job.did(usize::MAX);
        assert_eq!(job.done(), 0);
    }
}
