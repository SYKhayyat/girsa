//! Reading Otzaria's `*_links.json`, for the works Sefaria does not have.
//!
//! spec.md §8.1: Sefaria's links are the same graph *before* it was converted
//! down to line numbers, so they are what W8 imports. This file exists for the
//! **978 works Sefaria has no text for**, whose links exist nowhere else.
//!
//! ```jsonc
//! { "line_index_1": 913.0,
//!   "heRef_2": "סליחות נוסח אשכנז ליטא, ליום ראשון,  ג, יא,",
//!   "path_2": "אוצריא\\סדר התפילה\\...\\סליחות נוסח אשכנז ליטא.txt",
//!   "line_index_2": 22.0,
//!   "Conection Type": "reference" }
//! ```
//!
//! # Every trap in BUILDER.md §0.2 is in those five fields
//!
//! **T1** — both ends are line numbers, which is the addressing this whole
//! project exists to leave. They are translated into segment ids here, once,
//! from a mapping recomputed out of the text file each run
//! ([`girsa_corpus::import::from_a_txt_library`], which is also what chose the
//! grammar at import) and never written down.
//!
//! **T2** — `"Conection Type"`, misspelled.
//!
//! **T3** — the indices are floats, and sometimes strings: `913.0`, `"913.0"`.
//! A plain integer parse fails on real data.
//!
//! **T4** — **`path_2` is stale.** Otzaria's folders were renamed after these
//! files were generated, so the path names a directory that is not there. The
//! *filename* is good, and every target is resolved through it.
//!
//! **T5** — 74% of the types are blank, upstream, and it is not an error.
//!
//! # Why a target is sometimes read as a citation instead
//!
//! `line_index_2` counts lines in *Otzaria's* copy of the target. When the
//! target is one of the 5,640 works both corpora have, Girsa imported Sefaria's
//! copy (decision 1) and Otzaria's line numbers describe a file that is not on
//! the shelf — they would land on whatever segment happened to be there. So for
//! those, the target comes from `heRef_2`, which is a citation and means the
//! same thing in either copy.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use girsa_corpus::import;
use girsa_corpus::index::SegmentIndex;
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_corpus::work::{match_key, Source, Work};
use serde_json::Value;

use crate::sefaria::{Resolved, Resolver, Tally};
use crate::{Anchor, Edge, EdgeType, Method};

/// Line number → the segment that line became, for one work.
///
/// **Built, used and dropped inside an import.** W6's acceptance is that no
/// line number is persisted as a durable reference; recomputing this from the
/// source file each run is what keeps that true while still being able to read
/// a corpus that is addressed that way.
#[derive(Debug, Clone, Default)]
pub struct LineMap {
    by_line: HashMap<usize, SegmentId>,
}

/// The id that begins each source line, one per line, in reading order.
///
/// # Why this is not just the segments
///
/// A segment too long to name a place is **split** at import — `#7` becomes
/// `#7.1` and `#7.2`, and the parent is deleted. So one line of the source file
/// can be several segments on the shelf, and a mapping that zips lines against
/// segments one-to-one runs out of line before it runs out of segment.
///
/// It refused rather than mis-mapping, which is right, but it refused a lot: six
/// of the ten Encyclopedia Talmudit volumes lost **every one of their footnote
/// links** to an off-by-one — `7026 lines against 7027 segments` — because a
/// single ערך in each was long enough to be cut in two. The Rambam's Mishnah
/// commentaries were losing theirs the same way and had been all along.
///
/// # Grouping by the first component is not enough
///
/// [`girsa_corpus::segment::Ordinal::child`] has **two callers that mean
/// opposite things by it**, and `Ordinal::covers` says so in as many words: a
/// cut carving `#7` into pieces, and continuity naming a line upstream inserted
/// after `#7`. Both produce `#7.1`. Only the first is `#7`'s words; the second
/// is a line of its own and has to start one.
///
/// What tells them apart is a fact about the shelf rather than about the name:
/// **a cut deletes its parent and an insertion does not.** So `#7.1` beside a
/// live `#7` is a new line, and `#7.1` where `#7` is gone is the first piece of
/// the line `#7` used to be.
///
/// Reading it the crude way cost a volume of the encyclopedia its links in
/// *both* directions on successive runs — one segment too many before a
/// re-import, one too few after it, because that run minted 17,107 ids between
/// neighbours.
fn where_each_line_starts(segments: &[girsa_corpus::import::Segment]) -> Vec<SegmentId> {
    let present: std::collections::HashSet<Ordinal> =
        segments.iter().map(|s| s.id.ordinal().clone()).collect();
    segments
        .iter()
        .filter(|segment| {
            let ordinal = segment.id.ordinal();
            // A root is always the start of its own line.
            let Some(parent) = ordinal.parent() else {
                return true;
            };
            // The parent is still here, so this was inserted beside it rather
            // than carved out of it.
            if present.contains(&parent) {
                return true;
            }
            // A piece of a split. Only the first piece present starts the line.
            let Some(k) = ordinal.at(ordinal.depth() - 1) else {
                return true;
            };
            !(1..k).any(|earlier| present.contains(&parent.child(earlier)))
        })
        .map(|segment| segment.id.clone())
        .collect()
}

impl LineMap {
    /// Zip the ids already on the shelf against the lines they came from.
    ///
    /// The ids are read back rather than recomputed, so this cannot invent an
    /// ordinal — if the two disagree in length the mapping is refused, because
    /// a mapping that is off by one is worse than none.
    ///
    /// # Errors
    ///
    /// If the work is not on the shelf or its source file cannot be read.
    pub fn build(root: &Path, work: &Work) -> Result<Self, import::ImportError> {
        let imported = import::read_back(root, &work.slug)?;
        let body =
            fs::read_to_string(&work.origin).map_err(import::ImportError::io(&work.origin))?;
        // The same choice of grammar the import made — see
        // [`import::from_a_txt_library`]. Asking it differently here is how a
        // mapping ends up one segment out on purpose.
        let lines = import::from_a_txt_library(&body, &work.he_title);
        let starts = where_each_line_starts(&imported.segments);

        if lines.len() != starts.len() {
            return Err(import::ImportError::malformed(
                &work.origin,
                format!(
                    "{} lines against {} segments on the shelf — the file has changed \
                     since the import, and mapping them anyway would anchor every link \
                     in this sefer one segment out",
                    lines.len(),
                    starts.len()
                ),
            ));
        }

        Ok(Self {
            by_line: lines
                .into_iter()
                .zip(starts)
                .map(|((line, _), id)| (line, id))
                .collect(),
        })
    }

    #[must_use]
    pub fn get(&self, line: usize) -> Option<&SegmentId> {
        self.by_line.get(&line)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_line.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_line.is_empty()
    }
}

/// Read a line index however Otzaria wrote it this time.
///
/// T3: `913.0`, and sometimes `"913.0"`. BUILDER.md gives the rule as
/// `substringBefore('.').toInt()`, and a naive integer parse throws on real
/// data. Done through the text of the number rather than through a float cast,
/// so nothing is rounded on the way.
#[must_use]
pub fn line_index(value: Option<&Value>) -> Option<usize> {
    let raw = match value? {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => return None,
    };
    raw.split('.').next()?.trim().parse().ok()
}

/// The work a `path_2` points at, **by filename**.
///
/// T4, and it is not a nicety: the folders in those paths were renamed after
/// the link files were generated, so following the path finds nothing while the
/// filename finds the sefer. Verified independently in `OtzariaSonim/SPEC.md`.
#[must_use]
pub fn target_title(path_2: &str) -> Option<String> {
    let filename = path_2
        .rsplit(['\\', '/'])
        .find(|part| !part.trim().is_empty())?;
    let stem = filename.strip_suffix(".txt").unwrap_or(filename);
    let stem = stem.trim();
    (!stem.is_empty()).then(|| stem.to_string())
}

/// Every work on the shelf, findable by any spelling of its title.
///
/// Both halves of a link need this: an Otzaria filename has to find the work
/// whatever corpus supplied it, and for the 5,640 shared works that means
/// finding a Sefaria work by its Otzaria filename — which is the same match the
/// catalogue made to decide the split in the first place.
#[derive(Debug, Clone, Default)]
pub struct TitleIndex {
    by_key: HashMap<String, Vec<Work>>,
}

impl TitleIndex {
    #[must_use]
    pub fn build(works: &[Work]) -> Self {
        let mut by_key: HashMap<String, Vec<Work>> = HashMap::new();
        for work in works {
            for title in [&work.he_title, &work.en_title] {
                let entry = by_key.entry(match_key(title)).or_default();
                if !entry.iter().any(|w| w.slug == work.slug) {
                    entry.push(work.clone());
                }
            }
        }
        Self { by_key }
    }

    /// Every work a filename could name.
    ///
    /// **All of them, not the first.** This kept one work per key and let the
    /// rest fall out — a filename two seforim answer to resolved silently to
    /// whichever the work index listed first, and every link in that file
    /// pointed into the wrong sefer. Not a broken link: the wrong sefer, with
    /// no error, which is the failure mode BUILDER.md rule 6 exists to prevent
    /// and the same one the `Text N` column is read to avoid.
    #[must_use]
    pub fn get(&self, title: &str) -> &[Work] {
        self.by_key
            .get(&match_key(title))
            .map_or(&[], Vec::as_slice)
    }
}

/// What one pass over the Otzaria link files did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OtzariaTally {
    pub common: Tally,
    /// Rows whose `path_2` filename names no work on the shelf.
    pub unknown_target_file: usize,
    /// Rows whose line index is not a line that became a segment.
    pub line_not_a_segment: usize,
}

impl OtzariaTally {
    pub fn absorb(&mut self, other: Self) {
        self.common.absorb(other.common);
        self.unknown_target_file += other.unknown_target_file;
        self.line_not_a_segment += other.line_not_a_segment;
    }
}

/// Read one `<Title>_links.json` into edges.
///
/// `source_lines` maps the owning work's line numbers onto its segment ids.
/// `target_lines` is consulted, and filled, for targets Otzaria also supplies.
///
/// # Errors
///
/// If the file cannot be read or is not a JSON array.
#[allow(clippy::too_many_arguments)]
pub fn read_file(
    path: &Path,
    source_lines: &LineMap,
    titles: &TitleIndex,
    corpus_root: &Path,
    target_lines: &mut HashMap<String, Option<LineMap>>,
    resolver: &mut Resolver<'_>,
    index: &SegmentIndex,
    mut emit: impl FnMut(Edge),
) -> Result<OtzariaTally, std::io::Error> {
    let body = fs::read_to_string(path)?;
    let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(&body) else {
        return Ok(OtzariaTally::default());
    };
    let mut tally = OtzariaTally::default();

    for row in rows {
        tally.common.rows += 1;

        // T2. Spelled the way the file spells it.
        let label = row
            .get("Conection Type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if label.trim().is_empty() {
            tally.common.untyped += 1;
        }

        let Some(from) = line_index(row.get("line_index_1")).and_then(|n| source_lines.get(n))
        else {
            tally.line_not_a_segment += 1;
            continue;
        };

        // T4. The path is stale; the filename is not.
        let Some(title) = row
            .get("path_2")
            .and_then(Value::as_str)
            .and_then(target_title)
        else {
            tally.unknown_target_file += 1;
            continue;
        };
        let target = match titles.get(&title) {
            [one] => one.clone(),
            [] => {
                tally.unknown_target_file += 1;
                continue;
            }
            // Two seforim on the shelf answer to this filename. T4 says to
            // resolve a target by its filename, and here the filename does not
            // settle it — so it is the same question the `Text N` column
            // usually answers, asked from the other direction, and it goes into
            // the same queue rather than being decided by list order.
            several => {
                resolver.record_unsettled(
                    &title,
                    several.iter().map(|w| w.slug.clone()).collect(),
                    true,
                );
                tally.common.ambiguous += 1;
                continue;
            }
        };

        let to = match target.source {
            // Otzaria supplies this one too, so its line numbers describe the
            // file on the shelf and translate exactly.
            Source::Otzaria => {
                let map = target_lines
                    .entry(target.slug.clone())
                    .or_insert_with(|| LineMap::build(corpus_root, &target).ok());
                match (line_index(row.get("line_index_2")), map.as_ref()) {
                    (Some(n), Some(map)) => map.get(n).cloned(),
                    _ => None,
                }
            }
            // One of yours. Otzaria's CSVs are about Otzaria's corpus and can
            // only ever name a work of yours by a collision of titles, so the
            // link is dropped rather than pointed at a sefer of the reader's
            // that nobody said anything about.
            Source::Mine => None,
            // Sefaria supplies it, so Otzaria's line numbers describe a file
            // that is not on the shelf. The citation means the same thing in
            // either copy; the line number does not.
            Source::Sefaria => {
                let he_ref = row
                    .get("heRef_2")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                // Grime, and it is the corpus's: these arrive with a trailing
                // comma and doubled spaces.
                let cleaned = he_ref.trim().trim_end_matches(',').trim();
                match resolver.resolve_citation(cleaned, &target.he_title, index) {
                    Resolved::Exact(r) => index.resolve(&r).map(|run| run.first),
                    Resolved::Ambiguous(_) => {
                        tally.common.ambiguous += 1;
                        continue;
                    }
                    // Every sefer it could be is here, and none of them has this
                    // address. A missing address, not a question.
                    Resolved::NoPlace => {
                        tally.common.address_not_found += 1;
                        continue;
                    }
                    Resolved::Unresolved => {
                        tally.common.unresolved_citation += 1;
                        continue;
                    }
                }
            }
        };

        let Some(to) = to else {
            tally.line_not_a_segment += 1;
            continue;
        };

        tally.common.imported += 1;
        emit(Edge {
            from: Anchor::point(from.clone()),
            to: Anchor::point(to),
            edge_type: EdgeType::from_sefaria(label),
            method: Method::OtzariaSeed,
            // Not known here. `orient::Orienting::apply` is what decides this,
            // and it runs over the edge on its way to the store — an importer
            // that guessed would be the guess this field exists to expose.
            direction: crate::Direction::NotRecorded,
            source_label: label.trim().to_string(),
        });
    }
    Ok(tally)
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// A segment as the shelf holds it, with the ordinal spelled out.
    fn seg(work: &str, path: &[&str], ordinal: &[u32]) -> girsa_corpus::import::Segment {
        let id = SegmentId::new(
            work,
            path.iter().map(|p| (*p).to_string()).collect(),
            ordinal
                .iter()
                .skip(1)
                .fold(girsa_corpus::segment::Ordinal::root(ordinal[0]), |o, k| {
                    o.child(*k)
                }),
        );
        girsa_corpus::import::Segment {
            id,
            kind: girsa_corpus::import::SegmentKind::Text,
            text: String::new(),
            anchors: Vec::new(),
        }
    }

    #[test]
    fn a_line_split_in_two_is_still_one_line() {
        // The defect: a segment too long to name a place is cut into `#7.1` and
        // `#7.2` and its parent deleted, so four source lines can be five
        // segments. Zipping them one-to-one made the counts disagree, and the
        // map was refused — which cost six of the ten Encyclopedia Talmudit
        // volumes every footnote link they had.
        let segments = vec![
            seg("s", &["0"], &[1]),
            seg("s", &["0"], &[2, 1]),
            seg("s", &["0"], &[2, 2]),
            seg("s", &["0"], &[3]),
        ];
        let starts = where_each_line_starts(&segments);
        assert_eq!(starts.len(), 3, "three lines, not four segments");
        assert_eq!(
            starts.iter().map(ToString::to_string).collect::<Vec<_>>(),
            [
                "girsa:s/0#1".to_string(),
                "girsa:s/0#2.1".to_string(),
                "girsa:s/0#3".to_string()
            ],
            "the split line is anchored at the first of its children, where its words start"
        );
    }

    #[test]
    fn a_line_inserted_beside_its_neighbour_is_a_line_of_its_own() {
        // The other caller of `Ordinal::child`, and the one that made the
        // crude reading wrong in the opposite direction. A re-import that mints
        // 17,107 ids between neighbours produces `#2.1` **beside a living
        // `#2`** — upstream added a line, and it is a line. Grouping on the
        // first component swallowed it into `#2` and the count came out one
        // short instead of one long.
        let segments = vec![
            seg("s", &["0"], &[1]),
            seg("s", &["0"], &[2]),
            seg("s", &["0"], &[2, 1]),
            seg("s", &["0"], &[3]),
        ];
        assert_eq!(
            where_each_line_starts(&segments).len(),
            4,
            "`#2` is still here, so `#2.1` was inserted rather than carved out"
        );
    }

    #[test]
    fn nothing_split_is_one_start_per_segment() {
        let segments = vec![
            seg("s", &["1"], &[1]),
            seg("s", &["1"], &[2]),
            seg("s", &["1"], &[3]),
        ];
        assert_eq!(where_each_line_starts(&segments).len(), 3);
    }

    #[test]
    fn a_line_index_reads_as_a_float_and_as_a_string() {
        // T3, and both spellings are in the real files.
        assert_eq!(line_index(Some(&serde_json::json!(913.0))), Some(913));
        assert_eq!(line_index(Some(&serde_json::json!("913.0"))), Some(913));
        assert_eq!(line_index(Some(&serde_json::json!(913))), Some(913));
        assert_eq!(line_index(Some(&serde_json::json!(" 22.0 "))), Some(22));
        assert_eq!(line_index(Some(&serde_json::json!(null))), None);
        assert_eq!(line_index(None), None);
    }

    #[test]
    fn a_target_is_found_by_filename_and_never_by_path() {
        // T4. The folders in these paths were renamed after the links were
        // generated, so the path names a directory that is not there.
        assert_eq!(
            target_title("אוצריא\\סדר התפילה\\נוסח שכבר לא קיים\\סליחות נוסח אשכנז ליטא.txt")
                .as_deref(),
            Some("סליחות נוסח אשכנז ליטא")
        );
        assert_eq!(
            target_title("אוצריא/הלכה/משנה ברורה.txt").as_deref(),
            Some("משנה ברורה")
        );
        assert_eq!(target_title(""), None);
    }

    #[test]
    fn a_filename_finds_a_work_sefaria_supplied() {
        // The 5,640 shared works are on the shelf under Sefaria's slug, and an
        // Otzaria link file names them by their Hebrew filename. This is the
        // same match the catalogue made to decide the split.
        let works = vec![work("shulchan-arukh/orach-chayim", "שולחן ערוך, אורח חיים")];
        let index = TitleIndex::build(&works);
        let found = index.get("שולחן ערוך אורח חיים");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].slug, "shulchan-arukh/orach-chayim");
        assert!(index.get("קרן אורה על נדרים").is_empty());
    }

    #[test]
    fn a_filename_two_seforim_answer_to_returns_both_rather_than_the_first() {
        // T4 resolves a target by filename, and a filename is not always
        // unique. Keeping one work per key and letting the rest fall out sent
        // every link in that file into whichever sefer the work index happened
        // to list first — the wrong sefer, with no error. The same guess
        // BUILDER.md rule 6 forbids, made by a `HashMap::or_insert_with`.
        let works = vec![
            work("otzaria/מגן-אברהם", "מגן אברהם"),
            work("magen-avraham", "מגן אברהם"),
        ];
        let index = TitleIndex::build(&works);
        let found = index.get("מגן אברהם");
        assert_eq!(found.len(), 2, "both, so the caller can decline to choose");
    }

    fn work(slug: &str, he_title: &str) -> Work {
        Work {
            slug: slug.into(),
            he_title: he_title.into(),
            en_title: slug.into(),
            categories: vec![],
            order: Vec::new(),
            source: Source::Sefaria,
            origin: std::path::PathBuf::new(),
            schema: None,
            author: None,
            era: None,
            comp_date: None,
            version: None,
            he_sections: Vec::new(),
            commentary_on: Vec::new(),
        }
    }
}
