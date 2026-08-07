//! Your shelf, over the shipped one.
//!
//! spec.md §5: *the shipped taxonomy is a default, not a fact.* This is the
//! part that makes that true — and the whole of it is arranged so that **an
//! edit is a file of yours and never a change to the corpus.** The same rule as
//! corrections (§7.1) and link repair (§8.3), for the same reason: the corpus
//! is re-downloadable and your arrangement is not.
//!
//! # What an edit is keyed to
//!
//! Not a position. A shelf's key is the path the taxonomy derived for it —
//! `תלמוד/בבלי` — and **it keeps that key wherever the shelf is dragged to**,
//! because a key that moved with the shelf would break every other edit that
//! named it. Titles are display; keys are identity; the two are allowed to
//! disagree, which is what renaming a shelf means.
//!
//! A work is keyed by its slug, so a re-import rewrites all 7,189 catalogue
//! records and every edit still lands on the sefer it was about.
//!
//! # An edit is never thrown away
//!
//! An edit that names a sefer the shelf does not have — a work that has not
//! been imported yet, or that a corpus update has renamed — is **kept**. It
//! costs a line of JSON and it is the difference between a shelf that survives
//! a re-import and one that quietly forgets what you did to it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The top of the shelf, as a parent key.
pub const TOP: &str = "";

/// What the reader has done to the shipped shelf.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arrangement {
    /// Work slug → the shelf it was put on.
    #[serde(default)]
    pub works: BTreeMap<String, String>,
    /// Shelf key → the shelf it now hangs under. [`TOP`] for the top level.
    #[serde(default)]
    pub shelves: BTreeMap<String, String>,
    /// Shelf key → what it is called now.
    #[serde(default)]
    pub titles: BTreeMap<String, String>,
    /// Shelf key → the children put in an order, first. Anything not named
    /// here follows in the shipped order.
    #[serde(default)]
    pub order: BTreeMap<String, Vec<String>>,
    /// Shelves the reader made, which have no shipped existence to fall back
    /// on.
    #[serde(default)]
    pub made: BTreeSet<String>,
    /// Where the next made shelf's key comes from. A counter rather than a
    /// clock or a random number, so that two runs of the same edits produce
    /// the same file and a diff of your shelf is readable.
    #[serde(default)]
    pub minted: u32,
}

/// Why an edit was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    #[error("{code}: a shelf cannot be put inside itself", code = crate::trouble::Code::Cycle.as_str())]
    IntoItself,
    #[error("{code}: {0} is already inside {1}", code = crate::trouble::Code::Cycle.as_str())]
    IntoItsOwnChild(String, String),
}

impl Arrangement {
    /// Put a sefer on a shelf.
    pub fn put_work(&mut self, slug: &str, shelf: &str) {
        self.works.insert(slug.to_string(), shelf.to_string());
    }

    /// Put a shelf under another one.
    ///
    /// # Errors
    ///
    /// If that would make the shelf its own ancestor. Refused rather than
    /// repaired: the reader is holding one end of it and knows what they meant,
    /// and a shelf silently moved somewhere else is worse than one that did not
    /// move.
    pub fn put_shelf(&mut self, key: &str, parent: &str) -> Result<(), Refused> {
        if key == parent {
            return Err(Refused::IntoItself);
        }
        let mut walk = parent.to_string();
        loop {
            if walk == key {
                return Err(Refused::IntoItsOwnChild(
                    parent.to_string(),
                    key.to_string(),
                ));
            }
            match self.parent_of(&walk) {
                Some(up) => walk = up,
                None => break,
            }
        }
        self.shelves.insert(key.to_string(), parent.to_string());
        Ok(())
    }

    /// Call a shelf something else. The key does not change.
    pub fn rename(&mut self, key: &str, title: &str) {
        let title = title.trim();
        if title.is_empty() {
            self.titles.remove(key);
        } else {
            self.titles.insert(key.to_string(), title.to_string());
        }
    }

    /// Put the children of a shelf in an order.
    pub fn reorder(&mut self, parent: &str, children: Vec<String>) {
        if children.is_empty() {
            self.order.remove(parent);
        } else {
            self.order.insert(parent.to_string(), children);
        }
    }

    /// Make a shelf, and hand back its key.
    pub fn make(&mut self, parent: &str, title: &str) -> String {
        self.minted += 1;
        // No `/` in a made key: a key's `/` is what the shipped tree reads as
        // *hangs under*, and a made shelf hangs where it is put and nowhere by
        // implication.
        let key = format!("שלך-{}", self.minted);
        self.made.insert(key.clone());
        self.shelves.insert(key.clone(), parent.to_string());
        self.rename(&key, title);
        key
    }

    /// Put everything back the way it shipped.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Where a shelf hangs, if the reader has moved it.
    ///
    /// Falls back to the key's own path — `תלמוד/בבלי` hangs under `תלמוד` —
    /// which is what makes the shipped taxonomy the default rather than a
    /// separate thing that has to be recorded.
    #[must_use]
    pub fn parent_of(&self, key: &str) -> Option<String> {
        if let Some(parent) = self.shelves.get(key) {
            return (parent != TOP).then(|| parent.clone());
        }
        key.rsplit_once('/').map(|(head, _)| head.to_string())
    }

    /// What a shelf is called: the reader's name for it, else the last part of
    /// its key.
    #[must_use]
    pub fn title_of(&self, key: &str) -> String {
        if let Some(title) = self.titles.get(key) {
            return title.clone();
        }
        key.rsplit_once('/')
            .map_or_else(|| key.to_string(), |(_, tail)| tail.to_string())
    }

    /// Read the arrangement from the personal layer.
    ///
    /// A file that will not parse is **moved aside rather than overwritten** —
    /// [`Self::save`] would otherwise write an empty shelf over an evening of
    /// somebody's filing at the next drag. The returned message is shown in the
    /// window.
    #[must_use]
    pub fn load(path: &Path) -> (Self, Option<String>) {
        let Ok(body) = std::fs::read_to_string(path) else {
            return (Self::default(), None);
        };
        match serde_json::from_str(&body) {
            Ok(arrangement) => (arrangement, None),
            Err(e) => {
                let aside = path.with_extension("json.unreadable");
                let moved = std::fs::rename(path, &aside).is_ok();
                let where_it_went = if moved {
                    format!("it is at {}", aside.display())
                } else {
                    "it could not be moved aside either".to_string()
                };
                (
                    Self::default(),
                    Some(format!(
                        "your shelf arrangement would not read ({e}) — {where_it_went}, \
                         and the shipped shelf is being shown"
                    )),
                )
            }
        }
    }

    /// Write it back.
    ///
    /// # Errors
    ///
    /// If the personal directory cannot be made or the file cannot be written.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let body = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, body)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_shelf_cannot_be_put_inside_itself_or_inside_its_own_child() {
        let mut a = Arrangement::default();
        assert_eq!(a.put_shelf("תלמוד", "תלמוד"), Err(Refused::IntoItself));
        // `תלמוד/בבלי` hangs under `תלמוד` without anybody recording it, so
        // this is the shipped tree being walked, not just the edits.
        assert!(matches!(
            a.put_shelf("תלמוד", "תלמוד/בבלי"),
            Err(Refused::IntoItsOwnChild(_, _))
        ));
        // And the refusal left nothing behind.
        assert_eq!(a, Arrangement::default());

        // A move that is not a cycle is fine, and then the cycle it creates is
        // caught through the moved parentage rather than the shipped one.
        a.put_shelf("הלכה", "תלמוד/בבלי").unwrap();
        assert!(matches!(
            a.put_shelf("תלמוד", "הלכה"),
            Err(Refused::IntoItsOwnChild(_, _))
        ));
    }

    #[test]
    fn a_shelf_keeps_its_key_when_it_is_moved_and_when_it_is_renamed() {
        let mut a = Arrangement::default();
        a.put_shelf("תלמוד/בבלי", "הלכה").unwrap();
        a.rename("תלמוד/בבלי", "הש״ס שלי");
        // The key is what every other edit names it by, so it does not move
        // when the shelf does.
        assert_eq!(a.parent_of("תלמוד/בבלי").as_deref(), Some("הלכה"));
        assert_eq!(a.title_of("תלמוד/בבלי"), "הש״ס שלי");
        assert_eq!(a.title_of("תלמוד/ירושלמי"), "ירושלמי");
    }

    #[test]
    fn a_made_shelf_gets_a_key_that_two_runs_agree_on() {
        let mut one = Arrangement::default();
        let mut two = Arrangement::default();
        let a = one.make(TOP, "חבורה");
        let b = two.make(TOP, "חבורה");
        assert_eq!(a, b);
        assert_ne!(a, one.make(TOP, "עוד אחת"));
        assert!(one.made.contains(&a));
        assert_eq!(one.parent_of(&a), None, "made at the top, so no parent");
    }

    #[test]
    fn an_arrangement_that_will_not_parse_is_moved_aside_rather_than_overwritten() {
        let dir = std::env::temp_dir().join("girsa-arrangement-broken");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shelf.json");
        let aside = path.with_extension("json.unreadable");
        let _ = std::fs::remove_file(&aside);
        std::fs::write(&path, "{ this is not json").unwrap();

        let (arrangement, trouble) = Arrangement::load(&path);
        assert_eq!(arrangement, Arrangement::default());
        assert!(trouble.is_some(), "the reader is told");
        assert!(aside.is_file(), "and the file itself is still here");
        assert!(std::fs::read_to_string(&aside)
            .unwrap()
            .contains("not json"));
    }

    #[test]
    fn the_whole_arrangement_survives_being_written_and_read() {
        let dir = std::env::temp_dir().join("girsa-arrangement-roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shelf.json");

        let mut a = Arrangement::default();
        let made = a.make(TOP, "חבורה");
        a.put_work("bavli/berakhot", &made);
        a.put_shelf("תלמוד/ירושלמי", &made).unwrap();
        a.rename("הלכה", "פסק");
        a.reorder(TOP, vec!["הלכה".into(), "תלמוד".into()]);
        // A sefer that is not on this shelf at all. Kept: it may be a work
        // that has not been imported yet, and forgetting it here is how an
        // evening of filing disappears at the next corpus update.
        a.put_work("not/here", &made);
        a.save(&path).unwrap();

        let (back, trouble) = Arrangement::load(&path);
        assert_eq!(trouble, None);
        assert_eq!(back, a);
        assert_eq!(back.works.get("not/here").map(String::as_str), Some(&*made));
    }
}
