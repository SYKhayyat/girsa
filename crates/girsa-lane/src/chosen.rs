//! What you chose to put in the lane.
//!
//! spec.md §9.9, ruled in §16 #20: *a shelf, a sefer, a section, your own
//! layer, or the whole 5,000,545 — any selection, added to at any time.* This
//! module is that selection, and the reason it is a selection at all rather
//! than a switch is arithmetic: 5,000,545 segments through a BERT on a laptop
//! is days. A lane that insisted on all of it before it answered anything would
//! be a feature nobody ever turned on.
//!
//! # A shelf is not a thing here, and that is deliberate
//!
//! What is stored is **seforim and sections of seforim**, because those are
//! what a segment id can be checked against without asking anything else. A
//! shelf, an era, an author and *your own layer* are properties of a work, so
//! choosing one of them is resolved to the seforim it means before it gets
//! here — the same division `girsa_search::scope` keeps, and for the same
//! reason: one rule in one place.
//!
//! The consequence is worth stating, because a reader will meet it. Choosing
//! *the Rishonim* embeds the Rishonim that are on the shelf **today**. A sefer
//! imported next month is not silently swept in — it shows up as not covered,
//! and the coverage line offers to add it. `everything` is the one standing
//! choice, and it is stored as a standing choice rather than as a list, so it
//! does keep up.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use girsa_corpus::segment::SegmentId;
use serde::{Deserialize, Serialize};

/// How much of one sefer is in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Under {
    /// All of it.
    Whole,
    /// These sections of it, each an address prefix — `["ג"]` is siman gimmel
    /// and everything under it.
    Sections(BTreeSet<Vec<String>>),
}

/// The selection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chosen {
    /// The whole library, and whatever is added to it later.
    #[serde(default)]
    everything: bool,
    #[serde(default)]
    parts: BTreeMap<String, Under>,
}

impl Chosen {
    #[must_use]
    pub fn nothing() -> Self {
        Self::default()
    }

    /// The whole library, and anything that joins it.
    #[must_use]
    pub fn everything() -> Self {
        Self {
            everything: true,
            parts: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn is_everything(&self) -> bool {
        self.everything
    }

    #[must_use]
    pub fn is_nothing(&self) -> bool {
        !self.everything && self.parts.is_empty()
    }

    /// Add a whole sefer. Supersedes any sections of it already chosen —
    /// a reader who adds the sefer has asked for more, not for a list.
    #[must_use]
    pub fn with_work(mut self, slug: &str) -> Self {
        self.parts.insert(slug.to_string(), Under::Whole);
        self
    }

    /// Add one section of a sefer, by the address prefix that names it.
    ///
    /// A no-op when the whole sefer is already in, rather than a narrowing:
    /// nothing in this module can take anything out of the lane, because
    /// *choosing more* is the only gesture it has.
    #[must_use]
    pub fn with_section(mut self, slug: &str, under: &[String]) -> Self {
        match self
            .parts
            .entry(slug.to_string())
            .or_insert_with(|| Under::Sections(BTreeSet::new()))
        {
            Under::Whole => {}
            Under::Sections(sections) => {
                sections.insert(under.to_vec());
            }
        }
        self
    }

    /// Add every one of these seforim — what a shelf, an era, an author or
    /// *your own layer* comes to once it has been resolved.
    #[must_use]
    pub fn with_works(mut self, slugs: impl IntoIterator<Item = String>) -> Self {
        for slug in slugs {
            self = self.with_work(&slug);
        }
        self
    }

    /// Take a sefer back out, whole. `false` if it was not in.
    ///
    /// The counterpart to [`Chosen::with_work`] and the only removal there is:
    /// a lane you cannot un-choose from is one nobody dares choose in.
    pub fn without_work(&mut self, slug: &str) -> bool {
        self.parts.remove(slug).is_some()
    }

    /// The seforim named here. Empty under [`Chosen::everything`], which names
    /// none of them and covers all of them — ask [`Chosen::is_everything`]
    /// first.
    pub fn works(&self) -> impl Iterator<Item = &str> {
        self.parts.keys().map(String::as_str)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Whether a sefer has any part of it in the lane.
    #[must_use]
    pub fn holds(&self, slug: &str) -> bool {
        self.everything || self.parts.contains_key(slug)
    }

    /// Whether this segment is in the lane's chosen corpus.
    #[must_use]
    pub fn covers(&self, id: &SegmentId) -> bool {
        if self.everything {
            return true;
        }
        match self.parts.get(id.work()) {
            None => false,
            Some(Under::Whole) => true,
            Some(Under::Sections(sections)) => sections
                .iter()
                .any(|under| id.path().starts_with(under.as_slice())),
        }
    }

    /// Where the selection is kept.
    #[must_use]
    pub fn path_in(personal: &Path) -> PathBuf {
        personal.join("lane").join("chosen.json")
    }

    /// Read it back. A file that will not parse is reported and the selection
    /// is empty — which draws as *nothing is in the lane yet*, and is the one
    /// wrong answer here that cannot mislead: it under-claims.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        let path = Self::path_in(personal);
        let Ok(body) = std::fs::read_to_string(&path) else {
            return (Self::nothing(), Vec::new());
        };
        match serde_json::from_str(&body) {
            Ok(chosen) => (chosen, Vec::new()),
            Err(e) => (
                Self::nothing(),
                vec![format!("{} will not read: {e}", path.display())],
            ),
        }
    }

    /// Write it down.
    ///
    /// # Errors
    ///
    /// If the personal layer will not take it.
    pub fn save(&self, personal: &Path) -> Result<(), std::io::Error> {
        let path = Self::path_in(personal);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let body = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let temp = path.with_extension("json.writing");
        std::fs::write(&temp, body)?;
        std::fs::rename(&temp, &path)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn id(work: &str, path: &[&str]) -> SegmentId {
        let address = path.join(":");
        format!("girsa:{work}/{address}#1").parse().expect("an id")
    }

    #[test]
    fn a_sefer_a_section_and_everything_are_three_different_answers() {
        let sefer = Chosen::nothing().with_work("mishnah-berurah");
        assert!(sefer.covers(&id("mishnah-berurah", &["ג", "א"])));
        assert!(!sefer.covers(&id("shulchan-arukh/orach-chayim", &["ג", "א"])));

        let section =
            Chosen::nothing().with_section("shulchan-arukh/orach-chayim", &["ג".to_string()]);
        assert!(section.covers(&id("shulchan-arukh/orach-chayim", &["ג", "א"])));
        assert!(!section.covers(&id("shulchan-arukh/orach-chayim", &["ד", "א"])));

        let all = Chosen::everything();
        assert!(all.covers(&id("anything-at-all", &["1"])));
        assert!(all.holds("a sefer imported tomorrow"));
    }

    #[test]
    fn adding_the_sefer_after_a_section_widens_rather_than_listing() {
        let chosen = Chosen::nothing()
            .with_section("x", &["ג".to_string()])
            .with_work("x");
        assert!(chosen.covers(&id("x", &["ד"])), "the whole sefer is in");
        // And the other way round: a section of a sefer already in whole does
        // not narrow it.
        let chosen = chosen.with_section("x", &["ה".to_string()]);
        assert!(chosen.covers(&id("x", &["ד"])));
    }

    #[test]
    fn a_selection_survives_a_restart() {
        let dir = std::env::temp_dir().join("girsa-lane-chosen");
        let _ = std::fs::remove_dir_all(&dir);
        let chosen = Chosen::nothing()
            .with_works([
                "bavli/berakhot".to_string(),
                "rashi-on-berakhot".to_string(),
            ])
            .with_section("shulchan-arukh/orach-chayim", &["נח".to_string()]);
        chosen.save(&dir).expect("saves");

        let (back, trouble) = Chosen::open(&dir);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(back, chosen);
        assert_eq!(back.len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_chosen_is_the_default_and_covers_nothing() {
        let (chosen, trouble) = Chosen::open(Path::new("a directory that is not there"));
        assert!(trouble.is_empty());
        assert!(chosen.is_nothing());
        assert!(!chosen.covers(&id("x", &["1"])));
    }
}
