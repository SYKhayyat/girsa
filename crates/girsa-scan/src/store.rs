//! Where the mappings are kept: `personal/scans.json`, and nowhere else.
//!
//! A paging is something the reader worked out by looking at their own scan. It
//! is theirs, it is about a file only they have, and `girsa-import` rewrites
//! everything it owns on every run — so it lives under the personal root beside
//! the arrangement, the corrections and the link repairs, under the same rule
//! those three are under: **nothing here writes into `corpus/`.**
//!
//! # One file, rewritten whole
//!
//! Not a file per scan. A reader has a handful of scans, the whole thing is a
//! few hundred bytes, and one file is one thing to copy to another machine. It
//! is rewritten in full on every change rather than appended to — W8 shipped an
//! importer that opened its shards in append mode and doubled the graph on a
//! second run, and the same mistake here would put two mappings on one scan
//! with the second one silently losing.
//!
//! # A hand-edited file is read through the same door
//!
//! The format is JSON with the addresses written the way a reader writes them,
//! so it can be edited by hand — and everything read out of it goes through
//! [`Paging::declare`], which means a mapping that was edited into nonsense is
//! **refused with its slug named** rather than loaded and quietly used. One bad
//! entry costs one scan, not the library.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paging::{Anchor, Paging, Refused, Scheme};

/// Why a mapping could not be saved.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Refused(#[from] Refused),
    #[error("writing {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Malformed(String),
}

/// Every scan the reader has paged.
#[derive(Debug, Clone, Default)]
pub struct Scans {
    path: PathBuf,
    by_slug: BTreeMap<String, Paging>,
}

impl Scans {
    /// Read the mappings, and say what would not read.
    ///
    /// A scan whose entry is broken is left out and named in the returned
    /// lines — the same shape the corrections layer and the link repairs use,
    /// because the alternative is a library that will not open over one line of
    /// one file.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        let path = Self::path_in(personal);
        let mut trouble = Vec::new();
        let mut by_slug = BTreeMap::new();

        if let Ok(body) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<File>(&body) {
                Ok(file) => {
                    for (slug, written) in file.scans {
                        match written.into_paging() {
                            Ok(paging) => {
                                by_slug.insert(slug, paging);
                            }
                            Err(refused) => {
                                trouble
                                    .push(format!("the paging of {slug} will not read: {refused}"));
                            }
                        }
                    }
                }
                Err(e) => trouble.push(format!("{} will not read: {e}", path.display())),
            }
        }

        (Self { path, by_slug }, trouble)
    }

    /// An empty set that is not backed by a file, for a caller with no personal
    /// layer — a test, or a corpus tool.
    #[must_use]
    pub fn nowhere() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn path_in(personal: &Path) -> PathBuf {
        personal.join("scans.json")
    }

    /// What the reader has said about one scan.
    #[must_use]
    pub fn of(&self, slug: &str) -> Option<&Paging> {
        self.by_slug.get(slug)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_slug.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_slug.is_empty()
    }

    /// Say what a scan's pages are, and write it down.
    ///
    /// # Errors
    ///
    /// If the personal layer will not take it. The mapping itself was checked
    /// when it was declared.
    pub fn declare(&mut self, slug: &str, paging: Paging) -> Result<(), StoreError> {
        self.by_slug.insert(slug.to_string(), paging);
        self.save()
    }

    /// Take one back — the reader got the anchor wrong and would rather have no
    /// mapping than a wrong one.
    ///
    /// # Errors
    ///
    /// If the personal layer will not write.
    pub fn forget(&mut self, slug: &str) -> Result<bool, StoreError> {
        let had = self.by_slug.remove(slug).is_some();
        if had {
            self.save()?;
        }
        Ok(had)
    }

    fn save(&self) -> Result<(), StoreError> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|source| StoreError::Io {
                path: dir.display().to_string(),
                source,
            })?;
        }
        let file = File {
            scans: self
                .by_slug
                .iter()
                .map(|(slug, paging)| (slug.clone(), Written::of(paging)))
                .collect(),
        };
        let body = serde_json::to_string_pretty(&file)
            .map_err(|e| StoreError::Malformed(e.to_string()))?;
        std::fs::write(&self.path, body).map_err(|source| StoreError::Io {
            path: self.path.display().to_string(),
            source,
        })
    }
}

/// `personal/scans.json`.
#[derive(Debug, Serialize, Deserialize)]
struct File {
    scans: BTreeMap<String, Written>,
}

/// One mapping, as it is written down.
///
/// The address is a **string**, in the notation a reader types — `ב.` and not
/// `2a`. It is canonicalised on the way in by the same reader that reads every
/// citation in this system, and keeping the typed form in the file is what
/// makes it editable by somebody looking at their scan rather than at this
/// crate.
#[derive(Debug, Serialize, Deserialize)]
struct Written {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    of: Option<String>,
    scheme: Scheme,
    anchors: Vec<WrittenAnchor>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WrittenAnchor {
    page: usize,
    /// Absent where the anchor says *these pages are not pages of the sefer*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    at: Option<String>,
}

impl Written {
    fn of(paging: &Paging) -> Self {
        Self {
            of: paging.of().map(ToString::to_string),
            scheme: paging.scheme(),
            anchors: paging
                .anchors()
                .iter()
                .map(|anchor| WrittenAnchor {
                    page: anchor.page,
                    at: anchor.at.as_ref().map(ToString::to_string),
                })
                .collect(),
        }
    }

    fn into_paging(self) -> Result<Paging, Refused> {
        let anchors: Result<Vec<Anchor>, Refused> = self
            .anchors
            .into_iter()
            .map(|anchor| match anchor.at {
                Some(at) => Anchor::written(anchor.page, &at),
                None => Ok(Anchor::unpaged(anchor.page)),
            })
            .collect();
        Paging::declare(self.of, self.scheme, anchors?)
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::Placed;
    use girsa_ref::Address;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-scans-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_mapping_survives_being_written_down_and_read_back() {
        let dir = scratch("round-trip");
        let paging = Paging::declare(
            Some("bavli/berakhot".to_string()),
            Scheme::Amud,
            vec![
                Anchor::written(5, "ב.").expect("an anchor"),
                Anchor::unpaged(43),
                Anchor::written(45, "כא.").expect("an anchor"),
            ],
        )
        .expect("a mapping");

        let (mut scans, trouble) = Scans::open(&dir);
        assert!(trouble.is_empty(), "{trouble:?}");
        scans
            .declare("user/berakhot-vilna", paging.clone())
            .expect("saves");

        let (again, trouble) = Scans::open(&dir);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(again.of("user/berakhot-vilna"), Some(&paging));
        assert_eq!(again.len(), 1);
    }

    #[test]
    fn the_file_is_rewritten_whole_so_a_second_declaration_replaces_the_first() {
        let dir = scratch("replace");
        let (mut scans, _) = Scans::open(&dir);
        for page in [5, 7] {
            let paging = Paging::declare(
                None,
                Scheme::Amud,
                vec![Anchor::written(page, "ב.").expect("an anchor")],
            )
            .expect("a mapping");
            scans.declare("user/x", paging).expect("saves");
        }
        let (again, _) = Scans::open(&dir);
        assert_eq!(again.len(), 1);
        assert_eq!(
            again.of("user/x").map(|p| p.at(7)),
            Some(Placed::At {
                from: Address::parse("2a").expect("an address"),
                to: None
            })
        );
    }

    #[test]
    fn a_mapping_edited_into_nonsense_costs_one_scan_and_names_it() {
        // The file is hand-editable on purpose, so it can be hand-edited wrong.
        // One bad entry may not take the library with it, and it may not load
        // silently either — a mapping nobody can see is refused is a mapping
        // whose citations are quietly missing.
        let dir = scratch("nonsense");
        std::fs::create_dir_all(&dir).expect("a directory");
        std::fs::write(
            Scans::path_in(&dir),
            r#"{"scans":{
                "user/good":{"scheme":"amud","anchors":[{"page":5,"at":"ב."}]},
                "user/bad": {"scheme":"amud","anchors":[{"page":5,"at":"17"}]}
            }}"#,
        )
        .expect("writes");

        let (scans, trouble) = Scans::open(&dir);
        assert_eq!(scans.len(), 1);
        assert!(scans.of("user/good").is_some());
        assert!(scans.of("user/bad").is_none());
        assert_eq!(trouble.len(), 1);
        assert!(trouble[0].contains("user/bad"), "{trouble:?}");
    }

    #[test]
    fn forgetting_a_mapping_leaves_the_scan_unpaged_rather_than_wrongly_paged() {
        let dir = scratch("forget");
        let (mut scans, _) = Scans::open(&dir);
        scans
            .declare(
                "user/x",
                Paging::declare(
                    None,
                    Scheme::Amud,
                    vec![Anchor::written(5, "ב.").expect("an anchor")],
                )
                .expect("a mapping"),
            )
            .expect("saves");
        assert!(scans.forget("user/x").expect("writes"));
        assert!(!scans.forget("user/x").expect("writes"));
        let (again, _) = Scans::open(&dir);
        assert!(again.is_empty());
    }
}
