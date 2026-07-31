//! What is on the shelf, and how a sefer is opened.
//!
//! The catalogue is 7,189 lines of JSON and loads in one read; the text of a
//! work is loaded only when a pane opens it. Five million segments do not fit
//! in a window and are not wanted in one — a reader has two or three seforim
//! open, not the library.
//!
//! # Two catalogues, one shelf
//!
//! The corpus's, at `corpus/works/index.jsonl`, which `girsa-import` rewrites
//! in full every run; and yours, at `personal/works/index.jsonl`, which nothing
//! but you ever writes. They are kept in **separate files for one reason**: the
//! importer truncates the file it owns, so a sefer of yours filed in it would
//! be gone at the next corpus update and nothing would say so.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use girsa_corpus::import::{self, Segment};
use girsa_corpus::index::{SegmentIndex, WorkSegments};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::work::{Source, Work};
use girsa_fix::Showing;
use girsa_ref::{Address, Ref};

use crate::arrangement::{Arrangement, Refused};
use crate::taxonomy::{self, Branch};

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
    #[error("{0}")]
    Refused(String),
}

/// The catalogue, in memory.
#[derive(Debug)]
pub struct Shelf {
    root: PathBuf,
    /// Where your own layer lives: the arrangement, and your own seforim.
    personal: PathBuf,
    /// How you have arranged the shelf. Empty until you move something.
    arrangement: Arrangement,
    /// Your corrections (W20), and how much of them is being applied. They live
    /// in the personal layer beside the arrangement, for the same reason: the
    /// importer owns the corpus and truncates what it owns.
    fixes: girsa_fix::Layer,
    showing: Showing,
    /// What you have said about the link graph (W23). Beside the corrections,
    /// under the same rule: the importer owns the shards and replaces them.
    repairs: girsa_link::repair::Repairs,
    /// Which page of your scans is which daf (W25). In your layer for a third
    /// reason on top of the other two: it is about a file only you have.
    scans: girsa_scan::Scans,
    /// What you wrote (W27). Notes are seforim of yours, so they are in
    /// `works` as well — this is the writing side of the same thing, and it is
    /// what makes a note editable rather than only readable.
    notes: girsa_note::Notes,
    /// What you marked, what you kept asking, and what you keep together.
    marks: girsa_note::Marks,
    queries: girsa_note::Queries,
    collections: girsa_note::Collections,
    /// Something wrong with the personal layer that the reader should be told
    /// about — an arrangement file that would not read, so far.
    trouble: Option<String>,
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
    /// Read the corpus catalogue, your own catalogue, your arrangement, and
    /// the companions cache if it is there.
    ///
    /// # Errors
    ///
    /// If there is no corpus work index — which means the import has not run.
    /// A missing personal layer is not an error; it is a reader who has not
    /// added anything yet.
    pub fn open(root: &Path, personal: &Path) -> Result<Self, ShelfError> {
        let index = root.join("works/index.jsonl");
        let body = std::fs::read_to_string(&index)
            .map_err(|_| ShelfError::NoShelf(root.display().to_string()))?;

        let (mut works, mut bad_lines) = catalogue(&index, &body);
        // Yours, after the corpus's, and never in the same file — see the
        // module note.
        let mine_index = personal.join("works/index.jsonl");
        if let Ok(mine) = std::fs::read_to_string(&mine_index) {
            let (mine_works, mine_trouble) = catalogue(&mine_index, &mine);
            works.extend(mine_works);
            bad_lines.extend(mine_trouble);
        }

        let (arrangement, mut trouble) = Arrangement::load(&personal.join("shelf.json"));
        // A correction that will not read is one correction, and it is said out
        // loud — not a library that refuses to open. The catalogue above is
        // held to the same rule, and first: it is the file that decides which
        // seforim exist at all.
        let (fixes, fix_trouble) = girsa_fix::Layer::open(personal);
        bad_lines.extend(fix_trouble);
        let (repairs, bad_repairs) = girsa_link::repair::Repairs::open(personal);
        bad_lines.extend(bad_repairs);
        let (scans, bad_scans) = girsa_scan::Scans::open(personal);
        bad_lines.extend(bad_scans);
        let (notes, bad_notes) = girsa_note::Notes::open(personal);
        bad_lines.extend(bad_notes);
        let (marks, bad_marks) = girsa_note::Marks::open(personal);
        bad_lines.extend(bad_marks);
        let (queries, bad_queries) = girsa_note::Queries::open(personal);
        bad_lines.extend(bad_queries);
        let (collections, bad_folders) = girsa_note::Collections::open(personal);
        bad_lines.extend(bad_folders);
        for line in bad_lines {
            trouble = Some(match trouble {
                Some(said) => format!("{said} · {line}"),
                None => line,
            });
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
            personal: personal.to_path_buf(),
            arrangement,
            fixes,
            showing: Showing::default(),
            repairs,
            scans,
            notes,
            marks,
            queries,
            collections,
            trouble,
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
        let mut work = self
            .work(slug)
            .ok_or_else(|| ShelfError::NoSuchWork(slug.to_string()))?
            .clone();
        // Yours were written under the personal root and the corpus's under
        // the corpus root — one line, and it is the whole of what the rest of
        // the app has to know about the difference.
        let root = self.root_of(&work);
        let read = import::read_back(root, slug)
            .map_err(|e| ShelfError::Unreadable(slug.to_string(), e.to_string()))?;
        // The catalogue is a derived file; `work.json` sits beside the text and
        // is what the import actually wrote. The printed edition is read out of
        // the text and only that side ever learns it, so when the index has no
        // version, take the one next to the words (spec.md §13). `read_back`
        // has already parsed it — this is a field, not a second read.
        //
        // Belt and braces on top of the importer writing it: a corpus imported
        // by an older build is still on disk, and a quote whose provenance was
        // dropped cannot be un-dropped.
        if work.version.is_none() {
            work.version = read.work.version.clone();
        }
        let open = Open::corrected(work, read.segments, &self.fixes, self.showing);
        // A scan the reader has paged is addressed by what is printed on its
        // pages (W25). One place decides that, and this is it.
        Ok(match self.scans.of(slug) {
            Some(paging) if paging.is_declared() && crate::scanning::is_scan(&open.work) => {
                open.paged_by(paging.clone())
            }
            _ => open,
        })
    }

    /// Your corrections (W20).
    #[must_use]
    pub fn fixes(&self) -> &girsa_fix::Layer {
        &self.fixes
    }

    /// What you have said about the link graph (W23).
    #[must_use]
    pub fn repairs(&self) -> &girsa_link::repair::Repairs {
        &self.repairs
    }

    /// The same, to say something new. Every edit writes into the personal
    /// layer, and nothing here may write into `corpus/links/`.
    pub fn repairs_mut(&mut self) -> &mut girsa_link::repair::Repairs {
        &mut self.repairs
    }

    /// Which page of your scans is which daf (W25).
    #[must_use]
    pub fn scans(&self) -> &girsa_scan::Scans {
        &self.scans
    }

    /// What you wrote (W27).
    #[must_use]
    pub fn notes(&self) -> &girsa_note::Notes {
        &self.notes
    }

    /// What you marked (W27).
    #[must_use]
    pub fn marks(&self) -> &girsa_note::Marks {
        &self.marks
    }

    pub fn marks_mut(&mut self) -> &mut girsa_note::Marks {
        &mut self.marks
    }

    /// The questions you kept (W27).
    #[must_use]
    pub fn queries(&self) -> &girsa_note::Queries {
        &self.queries
    }

    pub fn queries_mut(&mut self) -> &mut girsa_note::Queries {
        &mut self.queries
    }

    /// Your chaburah folders (W27).
    #[must_use]
    pub fn collections(&self) -> &girsa_note::Collections {
        &self.collections
    }

    pub fn collections_mut(&mut self) -> &mut girsa_note::Collections {
        &mut self.collections
    }

    /// Write a note down, and put it on the shelf as a sefer.
    ///
    /// The catalogue entry goes into `works` here as well as onto the disk, so
    /// the note is openable, citable and namable in the link panel **in this
    /// session** rather than after a restart. The same thing
    /// [`Shelf::add_mine`] does for a file you dropped, and for the same
    /// reason: a sefer that exists on disk and not in the window is a sefer the
    /// reader was told about and cannot open.
    ///
    /// # Errors
    ///
    /// If the note says nothing, or your layer will not take it.
    pub fn write_note(&mut self, note: girsa_note::Note) -> Result<girsa_note::Note, ShelfError> {
        let work = note.work(&self.personal);
        let written = self
            .notes
            .write(note)
            .cloned()
            .map_err(|e| ShelfError::Refused(e.to_string()))?;
        match self.by_slug.get(&work.slug) {
            Some(i) => {
                if let Some(held) = self.works.get_mut(*i) {
                    *held = work;
                }
            }
            None => {
                self.by_slug.insert(work.slug.clone(), self.works.len());
                self.works.push(work);
            }
        }
        Ok(written)
    }

    /// Throw a note away — the file, the sefer and the catalogue line.
    ///
    /// # Errors
    ///
    /// If your layer will not write.
    pub fn forget_note(&mut self, name: &str) -> Result<bool, ShelfError> {
        let Some(slug) = self.notes.get(name).map(|note| note.slug.clone()) else {
            return Ok(false);
        };
        let gone = self
            .notes
            .remove(name)
            .map_err(|e| ShelfError::Refused(e.to_string()))?;
        if gone {
            // The catalogue in memory is a `Vec` with an index over it, so a
            // work is taken out by rebuilding the index rather than by leaving
            // a hole every later slug would point past.
            self.works.retain(|work| work.slug != slug);
            self.by_slug = self
                .works
                .iter()
                .enumerate()
                .map(|(i, w)| (w.slug.clone(), i))
                .collect();
            self.commentaries = HashMap::new();
            for (i, work) in self.works.iter().enumerate() {
                for base in &work.commentary_on {
                    self.commentaries
                        .entry(base.slug.clone())
                        .or_default()
                        .push(i);
                }
            }
        }
        Ok(gone)
    }

    /// Say what a scan's pages are, and write it down.
    ///
    /// # Errors
    ///
    /// If the personal layer will not take it. The mapping was checked when it
    /// was declared — a `Paging` that exists is one that has been checked.
    pub fn declare_paging(
        &mut self,
        slug: &str,
        paging: girsa_scan::Paging,
    ) -> Result<(), ShelfError> {
        self.scans
            .declare(slug, paging)
            .map_err(|e| ShelfError::Refused(e.to_string()))
    }

    /// Take a mapping back — better no mareh makom than a wrong one.
    ///
    /// # Errors
    ///
    /// If the personal layer will not write.
    pub fn forget_paging(&mut self, slug: &str) -> Result<bool, ShelfError> {
        self.scans
            .forget(slug)
            .map_err(|e| ShelfError::Refused(e.to_string()))
    }

    /// How much of them is being applied to what you read.
    #[must_use]
    pub fn showing(&self) -> Showing {
        self.showing
    }

    /// *Show as printed / show corrected* (spec.md §7.1). Costs a re-read of
    /// whatever is open, which is the caller's to do: this holds no text.
    pub fn set_showing(&mut self, showing: Showing) {
        self.showing = showing;
    }

    /// Take a correction and write it into your layer.
    ///
    /// # Errors
    ///
    /// If it changes nothing, claims letters another correction already claims,
    /// or the personal layer will not take it.
    pub fn fix(&mut self, patch: girsa_fix::Patch) -> Result<girsa_fix::Patch, ShelfError> {
        self.fixes
            .add(patch)
            .cloned()
            .map_err(|e| ShelfError::Refused(e.to_string()))
    }

    /// Take one back.
    ///
    /// # Errors
    ///
    /// If the personal layer will not write.
    pub fn unfix(&mut self, id: &girsa_fix::PatchId) -> Result<bool, ShelfError> {
        self.fixes
            .remove(id)
            .map_err(|e| ShelfError::Refused(e.to_string()))
    }

    /// Take somebody else's corrections (spec.md §7.1).
    ///
    /// # Errors
    ///
    /// If the file will not read, or ours will not write.
    pub fn take_fixes(&mut self, file: &Path) -> Result<girsa_fix::Merged, ShelfError> {
        self.fixes
            .merge(file)
            .map_err(|e| ShelfError::Refused(e.to_string()))
    }

    /// Which root a work's files are under.
    #[must_use]
    pub fn root_of(&self, work: &Work) -> &Path {
        match work.source {
            Source::Mine => &self.personal,
            Source::Sefaria | Source::Otzaria => &self.root,
        }
    }

    /// Put a file of yours on the shelf: a sefer, with permanent ids, in your
    /// own layer.
    ///
    /// # Errors
    ///
    /// If the file is of a kind nothing here reads, has nothing in it, or the
    /// personal layer will not take it. See [`import::mine::add`].
    pub fn add_mine(&mut self, file: &Path, title: Option<&str>) -> Result<String, ShelfError> {
        let added = import::mine::add(&self.personal, file, title)
            .map_err(|e| ShelfError::Refused(e.to_string()))?;
        let slug = added.work.slug.clone();
        self.by_slug.insert(slug.clone(), self.works.len());
        self.works.push(added.work);
        Ok(slug)
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

    #[must_use]
    pub fn personal(&self) -> &Path {
        &self.personal
    }

    /// Anything wrong with the personal layer the reader should be told.
    #[must_use]
    pub fn trouble(&self) -> Option<&str> {
        self.trouble.as_deref()
    }

    /// The shelf as it is browsed: the shipped taxonomy with your edits on top.
    #[must_use]
    pub fn tree(&self) -> Vec<Branch> {
        taxonomy::tree(&self.works, &self.arrangement)
    }

    /// The seforim standing on one shelf — not the ones on the shelves under
    /// it, which is what makes browsing a walk rather than a dump.
    #[must_use]
    pub fn works_on(&self, key: &str) -> Vec<&Work> {
        let mut on: Vec<&Work> = self
            .works
            .iter()
            .filter(|w| taxonomy::shelf_key_of(w, &self.arrangement) == key)
            .collect();
        let placed = |slug: &str| {
            self.arrangement
                .order
                .get(key)
                .and_then(|order| order.iter().position(|k| k == slug))
                .unwrap_or(usize::MAX)
        };
        on.sort_by(|a, b| {
            placed(&a.slug)
                .cmp(&placed(&b.slug))
                .then_with(|| a.he_title.cmp(&b.he_title))
        });
        on
    }

    #[must_use]
    pub fn arrangement(&self) -> &Arrangement {
        &self.arrangement
    }

    /// Where the arrangement is kept.
    #[must_use]
    pub fn arrangement_path(&self) -> PathBuf {
        self.personal.join("shelf.json")
    }

    /// Change the arrangement and write it down.
    ///
    /// Every edit goes through here so that **one** function knows the file is
    /// under `personal/` — nothing in this module is allowed to write into the
    /// corpus, and a second place that saved would be the place that one day
    /// did.
    ///
    /// # Errors
    ///
    /// If the edit is refused (a shelf inside itself), or the personal layer
    /// cannot be written. An edit that will not save is **not** applied in
    /// memory either: a shelf that rearranges itself back at the next restart
    /// is worse than one that says it could not.
    pub fn edit(
        &mut self,
        change: impl FnOnce(&mut Arrangement) -> Result<(), Refused>,
    ) -> Result<(), ShelfError> {
        let mut next = self.arrangement.clone();
        change(&mut next).map_err(|e| ShelfError::Refused(e.to_string()))?;
        next.save(&self.arrangement_path())
            .map_err(|source| ShelfError::Io {
                path: self.arrangement_path().display().to_string(),
                source,
            })?;
        self.arrangement = next;
        Ok(())
    }
}

/// One catalogue file, as works, and the lines that would not read.
///
/// A record that will not parse is skipped rather than fatal: one unreadable
/// line should cost one sefer, not the library. But it is never skipped
/// *quietly*. This file decides which seforim exist at all, so a torn write or
/// a half-finished import removing one has to be as loud as a correction that
/// will not read — the bar every other loader in [`Shelf::open`] already meets.
///
/// spec.md §5 is the promise being kept here: *"Nothing is ever missing, and
/// 'is it in there?' stops being a question you have to think about."* A
/// silently dropped line makes that question worth thinking about again.
fn catalogue(path: &Path, body: &str) -> (Vec<Work>, Vec<String>) {
    let mut works = Vec::new();
    let mut trouble = Vec::new();
    for (n, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Work>(line) {
            Ok(work) => works.push(work),
            Err(e) => trouble.push(format!(
                "{}: line {} is not a work: {e}",
                path.display(),
                n + 1
            )),
        }
    }
    (works, trouble)
}

/// A sefer with its text, ready to be read and to be lined up against another.
///
/// **The text here is corrected text** (W20). Your patches are applied when the
/// sefer is opened, so everything downstream of a pane — the words on the page,
/// a quote copied to Ksav, a citation regenerated from a ref — is the sefer as
/// you have it rather than as it was scanned. What was printed is kept beside
/// it for the segments that have a correction, because *show as printed* is
/// half of what the overlay is for.
#[derive(Debug, Clone)]
pub struct Open {
    pub work: Work,
    pub segments: Vec<Segment>,
    /// Segment → what it says on disk, for the segments a correction touched.
    /// Empty for a sefer you have never corrected, which is nearly all of them.
    printed: HashMap<SegmentId, String>,
    /// Segment → what was done to it, for marking the page.
    corrections: HashMap<SegmentId, girsa_fix::Corrected>,
    /// This work alone, addressed. Reused from the link importer rather than
    /// written again: a second implementation of "which segments does this
    /// address name" would drift from the one the graph was built with, and
    /// the panes would disagree with the links.
    index: SegmentIndex,
    position: HashMap<SegmentId, usize>,
    /// For a scan, what the reader has said its pages are called (W25).
    ///
    /// # Why an address of a scan is read through this and not through `index`
    ///
    /// A scan's segments are addressed by the **file's** page: page 47 of the
    /// PDF is `47`. What is printed on that page is a different number — the
    /// sefer's own, once the front matter is off — and for a scan numbered by
    /// page the two are both plain numbers. So `girsa:user/x/41` means
    /// *printed page 41* to the viewer and *file page 41* to the index, and on
    /// a real scan those are seven pages apart with nothing anywhere saying
    /// which was meant. Two answers for one ref is the silent wrongness this
    /// project is arranged against, and it was found by running the tool
    /// against a real PDF rather than by a test.
    ///
    /// So once a reader declares what the pages are called, **that is what an
    /// address of this sefer means**, here and everywhere. A page the mapping
    /// does not cover is then not reachable by ref, which is the honest
    /// answer — the reader has said the sefer starts on page 7, and the shaar
    /// blatt is not a place in it. It is still reachable, still noteable and
    /// still linkable by its **permanent id**, which no mapping ever moves.
    paging: Option<girsa_scan::Paging>,
}

impl Open {
    #[must_use]
    pub fn new(work: Work, segments: Vec<Segment>) -> Self {
        Self::corrected(work, segments, &girsa_fix::Layer::nowhere(), Showing::Fixed)
    }

    /// The same, with your corrections applied.
    #[must_use]
    pub fn corrected(
        work: Work,
        mut segments: Vec<Segment>,
        fixes: &girsa_fix::Layer,
        showing: Showing,
    ) -> Self {
        let mut printed = HashMap::new();
        let mut corrections = HashMap::new();
        if fixes.touches(&work.slug) {
            for segment in &mut segments {
                let corrected = fixes.apply(&segment.id, &segment.text, showing);
                if corrected.is_untouched() {
                    continue;
                }
                printed.insert(
                    segment.id.clone(),
                    std::mem::replace(&mut segment.text, corrected.text.clone()),
                );
                corrections.insert(segment.id.clone(), corrected);
            }
        }
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
            printed,
            corrections,
            index,
            position,
            paging: None,
        }
    }

    /// The same, knowing what the reader has said this scan's pages are called.
    ///
    /// Set by [`Shelf::read`] for a scan with a mapping, and by nothing else:
    /// what an address of a sefer means may be decided in one place.
    #[must_use]
    pub fn paged_by(mut self, paging: girsa_scan::Paging) -> Self {
        self.paging = Some(paging);
        self
    }

    #[must_use]
    pub fn slug(&self) -> &str {
        &self.work.slug
    }

    /// What a segment says on disk — which is what it says on the page, unless
    /// a correction changed it.
    #[must_use]
    pub fn as_printed(&self, id: &SegmentId) -> &str {
        if let Some(printed) = self.printed.get(id) {
            return printed;
        }
        self.position_of(id)
            .and_then(|at| self.segments.get(at))
            .map_or("", |s| s.text.as_str())
    }

    /// What was done to a segment, for the window to mark.
    #[must_use]
    pub fn correction(&self, id: &SegmentId) -> Option<&girsa_fix::Corrected> {
        self.corrections.get(id)
    }

    /// How many segments of this sefer a correction touched.
    #[must_use]
    pub fn corrections(&self) -> usize {
        self.corrections.len()
    }

    /// Where a segment sits in reading order.
    ///
    /// An id that was **split** since it was written down resolves to the first of
    /// its children. spec.md §3's promise is that *"editing text cannot move an
    /// anchor"* and that *"splitting a segment mints a child ID rather than shifting
    /// seventeen thousand others"* — and `Ordinal::covers` is how that promise is
    /// kept, so anything looking a segment up has to ask it. Without this, cutting
    /// a 1.2 MB segment into places (B12) would silently orphan every citation,
    /// link, mark and correction that named the parent.
    ///
    /// Exact first, because that is every lookup in a corpus nothing has split.
    #[must_use]
    pub fn position_of(&self, id: &SegmentId) -> Option<usize> {
        if let Some(at) = self.position.get(id).copied() {
            return Some(at);
        }
        self.covered_by(id).first().copied()
    }

    /// Where every segment an id covers sits, in reading order.
    ///
    /// Itself, if it is live. Its children, if it was split. Empty if it names
    /// nothing here — never the nearest thing.
    #[must_use]
    pub fn covered_by(&self, id: &SegmentId) -> Vec<usize> {
        if let Some(at) = self.position.get(id).copied() {
            return vec![at];
        }
        let mut out: Vec<usize> = self
            .position
            .iter()
            .filter(|(live, _)| id.covers(live))
            .map(|(_, at)| *at)
            .collect();
        out.sort_unstable();
        out
    }

    /// The segments an address names in this work, in reading order.
    ///
    /// Empty when the address names nothing here — never the nearest thing.
    #[must_use]
    pub fn at(&self, address: &Address) -> Vec<SegmentId> {
        // A scan whose pages the reader has named is addressed by those names
        // and not by the file's page numbers — see the field's note. What the
        // mapping does is turn one into the other; the lookup below is then the
        // same lookup every other sefer gets.
        let address = &match self.paging.as_ref() {
            Some(paging) => match paging.page_of(address, self.segments.len()) {
                Some(page) => Address::parse(&page.to_string()).unwrap_or_default(),
                None => return Vec::new(),
            },
            None => address.clone(),
        };
        let path: Vec<String> = self.work.slug.split('/').map(str::to_string).collect();
        let Some(run) = self.index.resolve(&Ref::point(path, address.clone())) else {
            return Vec::new();
        };
        // The *first* of what the start covers and the *last* of what the end
        // covers, so an address naming a segment that has since been split names
        // all of its children rather than only the first one.
        let (Some(&from), to) = (
            self.covered_by(&run.first).first(),
            run.last
                .as_ref()
                .and_then(|l| self.covered_by(l).last().copied()),
        ) else {
            return Vec::new();
        };
        let to = to.unwrap_or_else(|| self.covered_by(&run.first).last().copied().unwrap_or(from));
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
            he_sections: Vec::new(),
            commentary_on: Vec::new(),
        }
    }

    /// A shelf of these works, with a personal layer at `personal` — pass a
    /// scratch directory to test an edit, or nothing to test what is on it.
    pub(crate) fn shelf_of(works: Vec<Work>, personal: &Path) -> Shelf {
        let mut commentaries: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, work) in works.iter().enumerate() {
            for base in &work.commentary_on {
                commentaries.entry(base.slug.clone()).or_default().push(i);
            }
        }
        let (arrangement, trouble) = Arrangement::load(&personal.join("shelf.json"));
        let (fixes, _) = girsa_fix::Layer::open(personal);
        let (repairs, _) = girsa_link::repair::Repairs::open(personal);
        let (scans, _) = girsa_scan::Scans::open(personal);
        let (notes, _) = girsa_note::Notes::open(personal);
        let (marks, _) = girsa_note::Marks::open(personal);
        let (queries, _) = girsa_note::Queries::open(personal);
        let (collections, _) = girsa_note::Collections::open(personal);
        Shelf {
            root: PathBuf::new(),
            personal: personal.to_path_buf(),
            arrangement,
            fixes,
            showing: Showing::default(),
            repairs,
            scans,
            notes,
            marks,
            queries,
            collections,
            trouble,
            by_slug: works
                .iter()
                .enumerate()
                .map(|(i, w)| (w.slug.clone(), i))
                .collect(),
            works,
            commentaries,
            linked: HashMap::new(),
        }
    }

    /// A scratch personal layer, emptied first.
    pub(crate) fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        dir
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

        let shelf = shelf_of(works, &scratch("girsa-shelf-search"));

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
        let shelf = shelf_of(
            vec![work("bavli/berakhot"), rashi],
            &scratch("girsa-shelf-companions"),
        );

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
