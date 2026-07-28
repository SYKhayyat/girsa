//! What is on the shelf, and how a sefer is opened.
//!
//! The catalogue is 7,189 lines of JSON and loads in one read; the text of a
//! work is loaded only when a pane opens it. Five million segments do not fit
//! in a window and are not wanted in one — a reader has two or three seforim
//! open, not the library.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use girsa_corpus::import::{self, Segment};
use girsa_corpus::index::{SegmentIndex, WorkSegments};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::Work;
use girsa_ref::{Address, Ref};

/// Why the shelf, or a sefer on it, would not open.
#[derive(Debug, thiserror::Error)]
pub enum ShelfError {
    #[error("no shelf at {0} — has the import run?")]
    NoShelf(String),
    #[error("reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("no sefer here called {0}")]
    NoSuchWork(String),
    #[error("{0} is on the shelf and will not open: {1}")]
    Unreadable(String, String),
}

/// The catalogue, in memory.
#[derive(Debug)]
pub struct Shelf {
    root: PathBuf,
    works: Vec<Work>,
    by_slug: HashMap<String, usize>,
    /// Base work slug → the works that declare themselves commentaries on it.
    commentaries: HashMap<String, Vec<usize>>,
    /// Work slug → the works it shares edges with, and how many. Absent until
    /// `girsa-companions` has been run; the shelf works without it, with a
    /// shorter list of seforim to open beside what you are reading.
    linked: HashMap<String, Vec<(String, usize)>>,
}

/// A sefer offered for the column beside the one you are reading.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Companion {
    pub slug: String,
    pub he_title: String,
    pub en_title: String,
    /// Whether the corpus declares the two related, or only records edges
    /// between them. Shown, because a reader should be able to tell a stated
    /// relationship from a counted one.
    pub declared: bool,
    /// How many edges join the two, where that is what relates them.
    pub links: usize,
}

impl Shelf {
    /// Read `works/index.jsonl`, and the companions cache if it is there.
    ///
    /// # Errors
    ///
    /// If there is no work index — which means the import has not run.
    pub fn open(root: &Path) -> Result<Self, ShelfError> {
        let index = root.join("works/index.jsonl");
        let body = std::fs::read_to_string(&index)
            .map_err(|_| ShelfError::NoShelf(root.display().to_string()))?;

        let mut works = Vec::new();
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            // A work whose record will not parse is skipped rather than fatal:
            // one unreadable line should cost one sefer, not the library.
            if let Ok(work) = serde_json::from_str::<Work>(line) {
                works.push(work);
            }
        }

        let by_slug = works
            .iter()
            .enumerate()
            .map(|(i, w)| (w.slug.clone(), i))
            .collect();
        let mut commentaries: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, work) in works.iter().enumerate() {
            for base in &work.commentary_on {
                commentaries.entry(base.slug.clone()).or_default().push(i);
            }
        }

        Ok(Self {
            linked: read_companions(root),
            root: root.to_path_buf(),
            works,
            by_slug,
            commentaries,
        })
    }

    #[must_use]
    pub fn works(&self) -> &[Work] {
        &self.works
    }

    #[must_use]
    pub fn work(&self, slug: &str) -> Option<&Work> {
        self.by_slug.get(slug).and_then(|i| self.works.get(*i))
    }

    /// Open a sefer: its metadata, its segments, and an index over its
    /// addresses.
    ///
    /// # Errors
    ///
    /// If the sefer is not on the shelf, or its files will not read.
    pub fn read(&self, slug: &str) -> Result<Open, ShelfError> {
        let work = self
            .work(slug)
            .ok_or_else(|| ShelfError::NoSuchWork(slug.to_string()))?
            .clone();
        let read = import::read_back(&self.root, slug)
            .map_err(|e| ShelfError::Unreadable(slug.to_string(), e.to_string()))?;
        Ok(Open::new(work, read.segments))
    }

    /// The seforim worth opening in the column beside this one, best first.
    ///
    /// Two sources, and they are different kinds of claim. A **declaration** is
    /// the corpus saying *this is a commentary on that* — Sefaria states it on
    /// the schema of all 5,436 of them. An **edge count** is only "these two
    /// are joined 815 times", which is worth offering and is not the same
    /// thing, so the two are marked apart rather than merged into one ranking a
    /// reader cannot see into.
    #[must_use]
    pub fn companions(&self, slug: &str) -> Vec<Companion> {
        let mut out: Vec<Companion> = Vec::new();
        let mut seen: HashMap<&str, usize> = HashMap::new();

        let mut declared: Vec<&Work> = self
            .commentaries
            .get(slug)
            .into_iter()
            .flatten()
            .filter_map(|i| self.works.get(*i))
            .collect();
        // And the other direction: what you are reading may itself be the
        // commentary, in which case the sefer to put beside it is its base.
        if let Some(work) = self.work(slug) {
            declared.extend(work.commentary_on.iter().filter_map(|b| self.work(&b.slug)));
        }
        for work in declared {
            if seen.insert(work.slug.as_str(), out.len()).is_some() {
                continue;
            }
            out.push(Companion {
                slug: work.slug.clone(),
                he_title: work.he_title.clone(),
                en_title: work.en_title.clone(),
                declared: true,
                links: 0,
            });
        }

        for (other, count) in self.linked.get(slug).into_iter().flatten() {
            if let Some(at) = seen.get(other.as_str()) {
                if let Some(existing) = out.get_mut(*at) {
                    existing.links = *count;
                }
                continue;
            }
            let Some(work) = self.work(other) else {
                continue;
            };
            seen.insert(work.slug.as_str(), out.len());
            out.push(Companion {
                slug: work.slug.clone(),
                he_title: work.he_title.clone(),
                en_title: work.en_title.clone(),
                declared: false,
                links: *count,
            });
        }

        out.sort_by(|a, b| {
            b.declared
                .cmp(&a.declared)
                .then(b.links.cmp(&a.links))
                .then(a.slug.cmp(&b.slug))
        });
        out
    }

    /// Seforim whose title matches what has been typed, best first.
    ///
    /// Matched through [`girsa_hebrew::normalize`], so `שועה` finds
    /// `שולחן ערוך, אורח חיים` and a gershayim never has to be typed the way
    /// the corpus happens to spell it (W2's sibling rule: nothing here compares
    /// two Hebrew strings with `==`).
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Work> {
        let needle = girsa_hebrew::normalize(query);
        if needle.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<(u8, usize, &Work)> = Vec::new();
        for work in &self.works {
            let he = girsa_hebrew::normalize(&work.he_title);
            let en = work.en_title.to_lowercase();
            let lower = query.to_lowercase();
            // Rank by where the match is, not by how long the title is: a
            // reader typing `ברכות` wants Berakhot, not the forty seforim with
            // it somewhere in the middle of their name.
            let rank = if he == needle || en == lower {
                0
            } else if he.starts_with(&needle) || en.starts_with(&lower) {
                1
            } else if he.contains(&needle) || en.contains(&lower) {
                2
            } else {
                continue;
            };
            hits.push((rank, work.he_title.chars().count(), work));
        }
        hits.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.cmp(&b.1))
                .then(a.2.slug.cmp(&b.2.slug))
        });
        hits.into_iter().take(limit).map(|(_, _, w)| w).collect()
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// A sefer with its text, ready to be read and to be lined up against another.
#[derive(Debug, Clone)]
pub struct Open {
    pub work: Work,
    pub segments: Vec<Segment>,
    /// This work alone, addressed. Reused from the link importer rather than
    /// written again: a second implementation of "which segments does this
    /// address name" would drift from the one the graph was built with, and
    /// the panes would disagree with the links.
    index: SegmentIndex,
    position: HashMap<SegmentId, usize>,
}

impl Open {
    #[must_use]
    pub fn new(work: Work, segments: Vec<Segment>) -> Self {
        let mut index = SegmentIndex::default();
        index.insert(
            work.slug.clone(),
            WorkSegments::from_segments(segments.iter().map(|s| (s.id.path(), s.id.ordinal()))),
        );
        let position = segments
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.clone(), i))
            .collect();
        Self {
            work,
            segments,
            index,
            position,
        }
    }

    #[must_use]
    pub fn slug(&self) -> &str {
        &self.work.slug
    }

    /// Where a segment sits in reading order.
    #[must_use]
    pub fn position_of(&self, id: &SegmentId) -> Option<usize> {
        self.position.get(id).copied()
    }

    /// The segments an address names in this work, in reading order.
    ///
    /// Empty when the address names nothing here — never the nearest thing.
    #[must_use]
    pub fn at(&self, address: &Address) -> Vec<SegmentId> {
        let path: Vec<String> = self.work.slug.split('/').map(str::to_string).collect();
        let Some(run) = self.index.resolve(&Ref::point(path, address.clone())) else {
            return Vec::new();
        };
        let (Some(from), to) = (
            self.position_of(&run.first),
            run.last.as_ref().and_then(|l| self.position_of(l)),
        ) else {
            return Vec::new();
        };
        let to = to.unwrap_or(from);
        self.segments
            .get(from..=to)
            .map(|run| run.iter().map(|s| s.id.clone()).collect())
            .unwrap_or_default()
    }
}

/// The address of a segment, as an [`Address`].
///
/// A segment id's path is already canonical — the importer wrote it, and
/// [`SegmentId::is_well_formed`] holds — so this cannot fail on anything that
/// came off the shelf.
#[must_use]
pub fn address_of(id: &SegmentId) -> Address {
    Address::parse(&id.path().join(":")).unwrap_or_default()
}

/// `corpus/links/companions.jsonl`, if `girsa-companions` has written it.
fn read_companions(root: &Path) -> HashMap<String, Vec<(String, usize)>> {
    #[derive(serde::Deserialize)]
    struct Row {
        work: String,
        with: Vec<Pair>,
    }
    #[derive(serde::Deserialize)]
    struct Pair {
        slug: String,
        n: usize,
    }

    let Ok(body) = std::fs::read_to_string(root.join("links/companions.jsonl")) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for line in body
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        if let Ok(row) = serde_json::from_str::<Row>(line) {
            out.insert(
                row.work,
                row.with.into_iter().map(|p| (p.slug, p.n)).collect(),
            );
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::import::SegmentKind;
    use girsa_corpus::segment::Ordinal;
    use girsa_corpus::work::{BaseText, Mapping, Source};

    pub(crate) fn work(slug: &str) -> Work {
        Work {
            slug: slug.to_string(),
            he_title: slug.to_string(),
            en_title: slug.to_string(),
            categories: Vec::new(),
            source: Source::Sefaria,
            origin: PathBuf::new(),
            schema: None,
            author: None,
            era: None,
            comp_date: None,
            version: None,
            commentary_on: Vec::new(),
        }
    }

    pub(crate) fn open(slug: &str, addresses: &[&[&str]]) -> Open {
        let segments = addresses
            .iter()
            .enumerate()
            .map(|(i, path)| {
                #[allow(clippy::cast_possible_truncation)]
                let ordinal = Ordinal::root(i as u32 + 1);
                Segment {
                    id: SegmentId::new(
                        slug,
                        path.iter().map(|p| (*p).to_string()).collect(),
                        ordinal,
                    ),
                    kind: SegmentKind::Text,
                    text: format!("{slug} {}", path.join(":")),
                }
            })
            .collect();
        Open::new(work(slug), segments)
    }

    #[test]
    fn an_address_names_the_segments_under_it_and_nothing_near_them() {
        let sefer = open("s", &[&["1", "1"], &["1", "2"], &["2", "1"], &["10", "1"]]);
        let at = |a: &str| {
            sefer
                .at(&Address::parse(a).expect("an address"))
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(at("1:1"), ["girsa:s/1:1#1"]);
        assert_eq!(at("1"), ["girsa:s/1:1#1", "girsa:s/1:2#2"]);
        // Siman 1 does not swallow siman 10, and an address nobody has is not
        // rounded to the nearest one.
        assert_eq!(at("9"), Vec::<String>::new());
        assert_eq!(at("10"), ["girsa:s/10:1#4"]);
    }

    #[test]
    fn searching_finds_a_sefer_without_its_gershayim_typed_the_corpus_way() {
        let mut works = vec![work("shulchan-arukh/orach-chayim"), work("bavli/berakhot")];
        works[0].he_title = "שולחן ערוך, אורח חיים".into();
        works[0].en_title = "Shulchan Arukh, Orach Chayim".into();
        works[1].he_title = "ברכות".into();
        works[1].en_title = "Berakhot".into();

        let shelf = Shelf {
            root: PathBuf::new(),
            by_slug: works
                .iter()
                .enumerate()
                .map(|(i, w)| (w.slug.clone(), i))
                .collect(),
            works,
            commentaries: HashMap::new(),
            linked: HashMap::new(),
        };

        let found = |q: &str| {
            shelf
                .search(q, 5)
                .first()
                .map(|w| w.slug.clone())
                .unwrap_or_default()
        };
        assert_eq!(found("ברכות"), "bavli/berakhot");
        assert_eq!(found("Berakhot"), "bavli/berakhot");
        // The comma in the corpus's title is not something a reader types.
        assert_eq!(found("שולחן ערוך אורח"), "shulchan-arukh/orach-chayim");
    }

    #[test]
    fn a_declared_commentary_is_offered_beside_its_base_and_the_base_beside_it() {
        let mut rashi = work("bavli/rashi-on-berakhot");
        rashi.commentary_on = vec![BaseText {
            slug: "bavli/berakhot".into(),
            mapping: Mapping::ManyToOne,
        }];
        let works = vec![work("bavli/berakhot"), rashi];
        let mut commentaries = HashMap::new();
        commentaries.insert("bavli/berakhot".to_string(), vec![1usize]);
        let shelf = Shelf {
            root: PathBuf::new(),
            by_slug: works
                .iter()
                .enumerate()
                .map(|(i, w)| (w.slug.clone(), i))
                .collect(),
            works,
            commentaries,
            linked: HashMap::new(),
        };

        let beside_gemara = shelf.companions("bavli/berakhot");
        assert_eq!(beside_gemara.len(), 1);
        assert_eq!(beside_gemara[0].slug, "bavli/rashi-on-berakhot");
        assert!(beside_gemara[0].declared);

        // And from the commentary, the sefer it is on.
        let beside_rashi = shelf.companions("bavli/rashi-on-berakhot");
        assert_eq!(beside_rashi.len(), 1);
        assert_eq!(beside_rashi[0].slug, "bavli/berakhot");
    }
}
