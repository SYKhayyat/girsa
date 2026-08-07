//! Turning a citation into the segments it names.
//!
//! W8's whole job runs through here. Sefaria's `links*.csv` address both ends
//! of every edge as a **citation** — `Sanhedrin 74b:9` — and spec.md §8.1 says
//! to resolve those onto permanent segment ids at import. `girsa-ref` turns the
//! citation into a [`Ref`]; this turns the [`Ref`] into segments.
//!
//! # A citation is usually coarser than a segment
//!
//! `Exodus 1:1-6:1` is one row in `links0.csv` and covers a parsha.
//! `Rashi on Berakhot 2a` covers a daf. So the answer is a **run** of segments
//! in reading order, and the single-segment case is the run that happens to be
//! one long — which is spec.md §4.2's rule that a ref points at a span.
//!
//! # What it will not do
//!
//! It will not fall back to "near enough". A citation into a siman that does
//! not exist resolves to nothing, and the caller counts it and says so. A
//! silent near-miss is BUILDER.md rule 6's forbidden guess in its most
//! dangerous form: the link resolves, it opens a page, and it is the wrong
//! page.

use std::collections::HashMap;
use std::path::Path;

use girsa_ref::{Address, Ref};

use crate::import;
use crate::segment::{Ordinal, SegmentId};

/// The addresses of one work's segments.
///
/// Held as `(path key, ordinal)` sorted by path key rather than as
/// [`SegmentId`]s, because a `SegmentId` carries its work slug and its path as
/// owned strings and the corpus has millions of segments — the same slug stored
/// four million times is most of a gigabyte for no information.
#[derive(Debug, Clone, Default)]
pub struct WorkSegments {
    /// `["1", "1"]` written as `1:1`, so a prefix search is a range search.
    entries: Vec<(String, Ordinal)>,
}

/// The whole corpus's addresses, one entry per work.
#[derive(Debug, Clone, Default)]
pub struct SegmentIndex {
    works: HashMap<String, WorkSegments>,
}

/// The segments a citation named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub first: SegmentId,
    /// The last segment, when the citation covered more than one.
    pub last: Option<SegmentId>,
}

impl Run {
    #[must_use]
    pub fn is_span(&self) -> bool {
        self.last.is_some()
    }
}

/// A citation's address level, written the way the importer wrote it.
///
/// The importer slugs a named section — Sefaria's node `Orach Chayim` becomes
/// `orach_chayim` — and a citation into it arrives as the words as printed.
/// Two places deriving a name from the same text and not agreeing is exactly
/// the class of failure W2 exists to prevent, so the lookup goes through the
/// same function the importer used.
///
/// Numbers and dafim pass through it unchanged: `240` is `240` and `2a` is
/// `2a`, so there is no case to special-case.
fn canonical_level(level: &girsa_ref::Level) -> String {
    crate::work::section_label_of(level.as_str())
}

/// The separator between address levels, and the character after it.
///
/// `1:1` and `1:2` both begin `1:`, and nothing else does — `10:1` begins
/// `10`. So every segment under siman 1 sits in the sorted range from `1:` up
/// to `1;`, which is a range search rather than a scan.
const LEVEL_SEP: char = ':';
const AFTER_LEVEL_SEP: char = ';';

impl WorkSegments {
    /// Build from segments already in reading order.
    #[must_use]
    pub fn from_segments<'a>(segments: impl Iterator<Item = (&'a [String], &'a Ordinal)>) -> Self {
        let mut entries: Vec<(String, Ordinal)> = segments
            .map(|(path, ordinal)| (path.join(":"), ordinal.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Self { entries }
    }

    /// Read a work's `segments.jsonl` back into an index.
    ///
    /// Reads the ids and **not the text**, and now that is true of the bytes as
    /// well as of what is kept. It used to deserialize each line into an
    /// `id`-only struct, which does not retain the text and still *lexes* it —
    /// every escape of a segment that reaches 1,275,307 characters, to skip a
    /// field. Over the whole shelf that is ~3 GB read and parsed to extract five
    /// million ids, paid by `girsa-link-import` and by
    /// `girsa_search::citation` both.
    ///
    /// [`import::ordered_ids`] is the scan: `id` is the first field of every
    /// line `import::write` produces, so the answer is between byte 7 and the
    /// next quote, and a line that is not that shape falls back to the parser
    /// rather than being dropped.
    ///
    /// **1.5× over 400 works of the real corpus** — `examples/measure-ids`, which
    /// runs both readers and refuses to report a time until they agree about how
    /// many ids there are. Not more, because what is left is the file read
    /// itself: 3 GB has to come off the disk either way. Beating *that* needs a
    /// sidecar of ids written at import, which is a second file to keep in step
    /// with `segments.jsonl` and a new way to be silently stale — a worse trade
    /// than the one being made here.
    ///
    /// # Errors
    ///
    /// If the work is not on the shelf, or a record does not parse. A record
    /// that does not parse fails the load rather than being skipped: an index
    /// silently one segment short answers a citation with the wrong segment.
    pub fn load(root: &Path, slug: &str) -> Result<Self, import::ImportError> {
        let mut entries: Vec<(String, Ordinal)> = import::ordered_ids(root, slug)?
            .into_iter()
            .map(|id| (id.address(), id.ordinal().clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(Self { entries })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every segment at or under an address, in reading order.
    ///
    /// `1:1` in a two-level work is one segment. `1` is the whole siman. An
    /// address naming nothing gives nothing — never the nearest thing.
    fn at(&self, address: &Address) -> Option<(SegmentId, SegmentId)> {
        let levels: Vec<String> = address.levels().iter().map(canonical_level).collect();
        if let Some(found) = self.at_levels(&levels) {
            return Some(found);
        }

        // A named section whose last word is a number arrives as two levels.
        // Sefaria names a node `Part 2` and cites into it as
        // `Guide for the Perplexed, Part 2 12:1`, which reads as
        // `Part` · `2` · `12` · `1` — the `2` looks like an address because
        // every citation's second token usually is one.
        //
        // So each place two adjacent levels *could* be one name is tried, and
        // the result is accepted **only if exactly one of them is a real
        // address in this work**. Two hits would be a genuine ambiguity and the
        // rule for those is to take neither (BUILDER.md rule 6).
        let mut hit = None;
        for i in 0..levels.len().saturating_sub(1) {
            if levels[i].parse::<u32>().is_ok() || levels[i + 1].parse::<u32>().is_err() {
                continue;
            }
            let mut joined = levels.clone();
            let tail = joined.remove(i + 1);
            joined[i] = format!("{}_{tail}", joined[i]);
            if let Some(found) = self.at_levels(&joined) {
                if hit.is_some() {
                    return None;
                }
                hit = Some(found);
            }
        }
        hit
    }

    fn at_levels(&self, levels: &[String]) -> Option<(SegmentId, SegmentId)> {
        let key = levels.join(":");
        if key.is_empty() {
            // A ref to a whole sefer.
            let first = self.entries.iter().min_by_key(|(_, o)| o.clone())?;
            let last = self.entries.iter().max_by_key(|(_, o)| o.clone())?;
            return Some((self.id_of(first), self.id_of(last)));
        }

        let mut lower = key.clone();
        lower.push(LEVEL_SEP);
        let mut upper = key.clone();
        upper.push(AFTER_LEVEL_SEP);

        let start = self
            .entries
            .partition_point(|(k, _)| k.as_str() < key.as_str());
        let end = self
            .entries
            .partition_point(|(k, _)| k.as_str() < upper.as_str());
        // The exact address, plus everything under it. `start` lands on the
        // exact key if there is one, because `key < key:` < `key;`.
        let covered = self
            .entries
            .get(start..end)?
            .iter()
            .filter(|(k, _)| k.as_str() == key.as_str() || k.starts_with(&lower));
        let mut first: Option<&(String, Ordinal)> = None;
        let mut last: Option<&(String, Ordinal)> = None;
        for entry in covered {
            if first.is_none_or(|f| entry.1 < f.1) {
                first = Some(entry);
            }
            if last.is_none_or(|l| entry.1 > l.1) {
                last = Some(entry);
            }
        }
        Some((self.id_of(first?), self.id_of(last?)))
    }

    fn id_of(&self, entry: &(String, Ordinal)) -> SegmentId {
        SegmentId::new(
            String::new(),
            entry.0.split(':').map(str::to_string).collect(),
            entry.1.clone(),
        )
    }
}

impl SegmentIndex {
    /// Read the whole shelf back, one work at a time.
    ///
    /// # Errors
    ///
    /// If the work index is missing. A single work that will not load is
    /// counted and returned, not fatal — the alternative is no link graph at
    /// all because one sefer is corrupt.
    pub fn load(root: &Path) -> Result<(Self, Vec<String>), std::io::Error> {
        let body = std::fs::read_to_string(root.join("works/index.jsonl"))?;
        let mut works = HashMap::new();
        let mut failed = Vec::new();
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(work) = serde_json::from_str::<crate::work::Work>(line) else {
                continue;
            };
            match WorkSegments::load(root, &work.slug) {
                Ok(segments) => {
                    works.insert(work.slug, segments);
                }
                Err(_) => failed.push(work.slug),
            }
        }
        Ok((Self { works }, failed))
    }

    #[must_use]
    pub fn works(&self) -> usize {
        self.works.len()
    }

    /// Whether a work was imported at all.
    ///
    /// The resolver knows more works than the shelf holds — its lexicon comes
    /// from every Sefaria schema, and 384 of those describe a work whose Hebrew
    /// text is not in the export. A caller that cannot tell "no such sefer here"
    /// from "no such place in this sefer" reports one number for two problems.
    #[must_use]
    pub fn has_work(&self, slug: &str) -> bool {
        self.works.contains_key(slug)
    }

    #[must_use]
    pub fn segments(&self) -> usize {
        self.works.values().map(WorkSegments::len).sum()
    }

    pub fn insert(&mut self, slug: impl Into<String>, segments: WorkSegments) {
        self.works.insert(slug.into(), segments);
    }

    /// The segments a resolved citation names, or nothing.
    ///
    /// A span citation — `Exodus 1:1-6:1` — takes the first segment of its
    /// opening address and the last of its closing one, which is what the
    /// citation says and not an approximation of it.
    ///
    /// **A span's closing address is usually written short.** `Arakhin 33b:21-22`
    /// is lines 21 to 22 of one daf, not line 21 of daf 33b through daf 22 —
    /// there is no daf 22 to end on, and read literally the citation names
    /// nothing. The end is completed against the start
    /// ([`Address::completed_against`]), which is the same rule that reads
    /// "see se'if 5" while standing in a siman.
    #[must_use]
    pub fn resolve(&self, reference: &Ref) -> Option<Run> {
        let slug = reference.work_slug();
        let work = self.works.get(&slug)?;

        let completed;
        let reference = match reference.to() {
            Some(to) if to.depth() < reference.from().depth() => {
                completed = Ref::span(
                    reference.work().to_vec(),
                    reference.from().clone(),
                    to.completed_against(reference.from()),
                );
                &completed
            }
            _ => reference,
        };

        let (first, first_last) = work.at(reference.from())?;
        let last = match reference.to() {
            Some(to) => work.at(to)?.1,
            None => first_last,
        };

        let first = with_work(&slug, first);
        let last = with_work(&slug, last);
        Some(Run {
            last: (last != first).then_some(last),
            first,
        })
    }
}

fn with_work(slug: &str, id: SegmentId) -> SegmentId {
    SegmentId::new(slug, id.path().to_vec(), id.ordinal().clone())
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// Two simanim of nine and one se'if, addressed the way a schema would.
    fn shulchan_arukh() -> SegmentIndex {
        let mut paths: Vec<(Vec<String>, Ordinal)> = Vec::new();
        let mut n = 0u32;
        for siman in 1..=10u32 {
            for seif in 1..=3u32 {
                n += 1;
                paths.push((vec![siman.to_string(), seif.to_string()], Ordinal::root(n)));
            }
        }
        let mut index = SegmentIndex::default();
        index.insert(
            "shulchan-arukh/orach-chayim",
            WorkSegments::from_segments(paths.iter().map(|(p, o)| (p.as_slice(), o))),
        );
        index
    }

    fn run(index: &SegmentIndex, citation: &str) -> Option<String> {
        let reference: Ref = citation.parse().ok()?;
        let run = index.resolve(&reference)?;
        Some(match run.last {
            Some(last) => format!("{}-{}", run.first, last),
            None => run.first.to_string(),
        })
    }

    #[test]
    fn a_seif_level_citation_lands_on_one_segment() {
        let index = shulchan_arukh();
        assert_eq!(
            run(&index, "girsa:shulchan-arukh/orach-chayim/1:2").as_deref(),
            Some("girsa:shulchan-arukh/orach-chayim/1:2#2")
        );
    }

    #[test]
    fn a_siman_level_citation_covers_the_whole_siman() {
        // `Rashi on Berakhot 2a` covers a daf and `Exodus 1:1-6:1` covers a
        // parsha. A citation coarser than a segment is the common case, not an
        // edge case, and answering it with only its first segment would lose
        // most of the graph.
        let index = shulchan_arukh();
        assert_eq!(
            run(&index, "girsa:shulchan-arukh/orach-chayim/2").as_deref(),
            Some("girsa:shulchan-arukh/orach-chayim/2:1#4-girsa:shulchan-arukh/orach-chayim/2:3#6")
        );
    }

    #[test]
    fn siman_one_does_not_swallow_siman_ten() {
        // `1` and `10` share a prefix as text. A prefix search that did not
        // stop at the level separator would put every segment of siman 10
        // inside siman 1 — and the citation would still resolve.
        let index = shulchan_arukh();
        let one = run(&index, "girsa:shulchan-arukh/orach-chayim/1").unwrap_or_default();
        assert!(one.ends_with("/1:3#3"), "{one}");
    }

    #[test]
    fn a_span_citation_runs_from_the_first_to_the_last() {
        let index = shulchan_arukh();
        assert_eq!(
            run(&index, "girsa:shulchan-arukh/orach-chayim/1:2-3:1").as_deref(),
            Some("girsa:shulchan-arukh/orach-chayim/1:2#2-girsa:shulchan-arukh/orach-chayim/3:1#7")
        );
    }

    /// A work with named volumes, the way the importer slugs them, plus a node
    /// whose own name ends in a number.
    fn tur() -> SegmentIndex {
        let paths: Vec<(Vec<String>, Ordinal)> = [
            vec!["orach_chayim".to_string(), "240".into(), "1".into()],
            vec!["orach_chayim".to_string(), "240".into(), "2".into()],
            vec!["yoreh_deah".to_string(), "183".into(), "1".into()],
            vec!["part_2".to_string(), "12".into(), "1".into()],
        ]
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            #[allow(clippy::cast_possible_truncation)]
            (p, Ordinal::root(i as u32 + 1))
        })
        .collect();
        let mut index = SegmentIndex::default();
        index.insert(
            "tur",
            WorkSegments::from_segments(paths.iter().map(|(p, o)| (p.as_slice(), o))),
        );
        index
    }

    #[test]
    fn a_named_volume_is_looked_up_the_way_the_importer_wrote_it() {
        // `Tur, Orach Chayim 240:1` resolves to the level `Orach Chayim`, as
        // printed. The importer wrote the section as `orach-chayim`. Comparing
        // the two as they stand missed 1.4 million of Sefaria's five million
        // links, every one of them into a sefer with named volumes — and it
        // looked like bad data rather than a bug.
        let index = tur();
        assert_eq!(
            run(&index, "girsa:tur/Orach Chayim:240:1").as_deref(),
            Some("girsa:tur/orach_chayim:240:1#1")
        );
        assert_eq!(
            run(&index, "girsa:tur/orach_chayim:240:1").as_deref(),
            Some("girsa:tur/orach_chayim:240:1#1")
        );
    }

    #[test]
    fn a_section_whose_name_ends_in_a_number_is_still_found() {
        // Sefaria names a node `Part 2`, so `Guide for the Perplexed, Part 2
        // 12:1` reads as four levels — the `2` looks like an address, because
        // in almost every other citation it would be.
        let index = tur();
        assert_eq!(
            run(&index, "girsa:tur/Part:2:12:1").as_deref(),
            Some("girsa:tur/part_2:12:1#4")
        );
    }

    #[test]
    fn a_spans_end_is_completed_against_its_start() {
        // `Arakhin 33b:21-22` is two lines of one daf. Read literally it ends
        // on daf 22, which does not exist, and the whole citation resolves to
        // nothing — which is how a large part of Shas's link graph went
        // missing.
        let index = shulchan_arukh();
        assert_eq!(
            run(&index, "girsa:shulchan-arukh/orach-chayim/1:1-3").as_deref(),
            Some("girsa:shulchan-arukh/orach-chayim/1:1#1-girsa:shulchan-arukh/orach-chayim/1:3#3")
        );
    }

    #[test]
    fn an_address_that_is_not_there_resolves_to_nothing_rather_than_nearby() {
        // BUILDER.md rule 6 in its most dangerous form: a near-miss link
        // resolves, opens a page, and it is the wrong page.
        let index = shulchan_arukh();
        assert_eq!(run(&index, "girsa:shulchan-arukh/orach-chayim/900:1"), None);
        assert_eq!(run(&index, "girsa:shulchan-arukh/orach-chayim/1:9"), None);
        assert_eq!(run(&index, "girsa:no-such-sefer/1:1"), None);
    }

    #[test]
    fn a_citation_to_a_whole_sefer_covers_the_whole_sefer() {
        // Built rather than parsed: `girsa:shulchan-arukh/orach-chayim` read
        // back as text is the work `shulchan-arukh` at address `orach-chayim`,
        // because the grammar's rule is that the last `/` component is the
        // address. The resolver reaches this case the other way — it knows the
        // slug from the lexicon and has no address left over.
        let index = shulchan_arukh();
        let whole = index
            .resolve(&Ref::whole_work(vec![
                "shulchan-arukh".into(),
                "orach-chayim".into(),
            ]))
            .expect("a whole-sefer ref resolves");
        assert_eq!(
            whole.first.to_string(),
            "girsa:shulchan-arukh/orach-chayim/1:1#1"
        );
        assert_eq!(
            whole.last.map(|l| l.to_string()).unwrap_or_default(),
            "girsa:shulchan-arukh/orach-chayim/10:3#30"
        );
    }
}
