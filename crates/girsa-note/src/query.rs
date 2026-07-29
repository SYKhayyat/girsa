//! Saved queries — a question you want to be able to ask again.
//!
//! spec.md §11. A saved query is not a saved *result*: the corpus grows, your
//! own seforim go on the shelf, and *every place the Rambam is called upon in
//! Hilchos Tefillah* is a different list next year. What is kept is the asking.
//!
//! # What a query is, on the disk
//!
//! Exactly what the query bar holds, and nothing translated:
//!
//! - **the line you typed**, sigils and all. spec.md §9.5's rule is that typing
//!   a sigil flips a chip, so `"יתגבר כארי"` already carries *one after the
//!   other* — the text is not a lossy summary of the search, it is half of it;
//! - **the chips**, as the `chip → key` pairs the row itself sends and takes
//!   back. Not a second model of what a chip can be: this crate has no opinion
//!   about search, and a copy of the chip vocabulary here would be one more
//!   thing to keep in step with `girsa-search`;
//! - **the scope**, as the slugs it was narrowed to or away from (W14's
//!   facets).
//!
//! # What it deliberately does not keep
//!
//! Where you were when you asked. A citation query resolved against context —
//! *see siman 5* while reading Orach Chayim — means something else from
//! somewhere else, and a saved query that quietly re-resolved itself against
//! wherever you happen to be standing would be the engine changing your
//! question without telling you (spec.md §9). Saved with its context lost, a
//! partial citation is saved as the words you typed and asked again as such.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A question, kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedQuery {
    /// What you called it.
    pub name: String,
    /// The line in the box, sigils and all.
    pub typed: String,
    /// The chip row, as `chip name → the key of the option chosen`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub chips: BTreeMap<String, String>,
    /// Narrowed to these seforim.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub only: Vec<String>,
    /// And away from these.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub without: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub when: u64,
}

impl SavedQuery {
    #[must_use]
    pub fn new(name: impl Into<String>, typed: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            typed: typed.into(),
            chips: BTreeMap::new(),
            only: Vec::new(),
            without: Vec::new(),
            tags: Vec::new(),
            when: crate::now_seconds(),
        }
    }

    #[must_use]
    pub fn with_chip(mut self, chip: impl Into<String>, key: impl Into<String>) -> Self {
        self.chips.insert(chip.into(), key.into());
        self
    }

    #[must_use]
    pub fn within(mut self, only: impl IntoIterator<Item = String>) -> Self {
        self.only = only.into_iter().collect();
        self
    }

    #[must_use]
    pub fn excluding(mut self, without: impl IntoIterator<Item = String>) -> Self {
        self.without = without.into_iter().collect();
        self
    }

    #[must_use]
    pub fn tagged(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        for tag in tags {
            let tag = tag.trim().to_string();
            if !tag.is_empty() && !self.tags.iter().any(|kept| crate::same_tag(kept, &tag)) {
                self.tags.push(tag);
            }
        }
        self
    }

    /// Saved at this moment rather than now — for a test, and for reading
    /// somebody else's file.
    #[must_use]
    pub const fn saved_at(mut self, when: u64) -> Self {
        self.when = when;
        self
    }

    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|kept| crate::same_tag(kept, tag))
    }

    /// What it reads as in a list: the question, not the name.
    #[must_use]
    pub fn said(&self) -> String {
        let mut said = self.typed.clone();
        if !self.only.is_empty() {
            said.push_str(&format!(" · {}", self.only.join(", ")));
        }
        if !self.without.is_empty() {
            said.push_str(&format!(" · not {}", self.without.join(", ")));
        }
        said
    }
}

/// Why a query was not saved.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("writing {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("a saved query needs a name")]
    Unnamed,
    #[error("a saved query has to ask something")]
    Empty,
}

/// Where they live under a personal layer.
#[must_use]
pub fn path_in(personal: &Path) -> PathBuf {
    personal.join("queries.jsonl")
}

/// The questions you have kept.
#[derive(Debug, Clone)]
pub struct Queries {
    path: PathBuf,
    by_name: BTreeMap<String, SavedQuery>,
}

impl Queries {
    /// Read them. A line that will not parse costs that query and is reported.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        let path = path_in(personal);
        let mut queries = Self {
            path,
            by_name: BTreeMap::new(),
        };
        let mut trouble = Vec::new();
        let Ok(body) = std::fs::read_to_string(&queries.path) else {
            return (queries, trouble);
        };
        for (n, line) in body.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SavedQuery>(line) {
                Ok(query) => {
                    queries.by_name.insert(query.name.clone(), query);
                }
                Err(e) => trouble.push(format!(
                    "{}: line {} is not a saved query: {e}",
                    queries.path.display(),
                    n + 1
                )),
            }
        }
        (queries, trouble)
    }

    /// A layer that is never written.
    #[must_use]
    pub fn nowhere() -> Self {
        Self {
            path: PathBuf::new(),
            by_name: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.by_name.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = &SavedQuery> {
        self.by_name.values()
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&SavedQuery> {
        self.by_name.get(name)
    }

    #[must_use]
    pub fn by_tag(&self, tag: &str) -> Vec<&SavedQuery> {
        self.all().filter(|query| query.has_tag(tag)).collect()
    }

    /// Keep one. Saving over a name replaces it — the name is what you asked
    /// for it by.
    ///
    /// # Errors
    ///
    /// If it has no name, asks nothing, or your layer will not write.
    pub fn save(&mut self, query: SavedQuery) -> Result<&SavedQuery, QueryError> {
        if query.name.trim().is_empty() {
            return Err(QueryError::Unnamed);
        }
        if query.typed.trim().is_empty() {
            return Err(QueryError::Empty);
        }
        let name = query.name.clone();
        let replaced = self.by_name.insert(name.clone(), query);
        if let Err(e) = self.write() {
            match replaced {
                Some(old) => {
                    self.by_name.insert(name, old);
                }
                None => {
                    self.by_name.remove(&name);
                }
            }
            return Err(e);
        }
        self.by_name.get(&name).ok_or(QueryError::Unnamed)
    }

    /// Forget one. `false` if there was no such query.
    ///
    /// # Errors
    ///
    /// If your layer will not write.
    pub fn remove(&mut self, name: &str) -> Result<bool, QueryError> {
        let gone = self.by_name.remove(name).is_some();
        if gone {
            self.write()?;
        }
        Ok(gone)
    }

    /// The file, as it goes to disk.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut body = String::new();
        for query in self.all() {
            if let Ok(line) = serde_json::to_string(query) {
                body.push_str(&line);
                body.push('\n');
            }
        }
        body
    }

    fn write(&self) -> Result<(), QueryError> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }
        let io = |source: std::io::Error| QueryError::Io {
            path: self.path.display().to_string(),
            source,
        };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let temp = self.path.with_extension("jsonl.writing");
        std::fs::write(&temp, self.to_text()).map_err(io)?;
        std::fs::rename(&temp, &self.path).map_err(io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_saved_query_keeps_the_line_and_the_chips_and_nothing_translated() {
        let query = SavedQuery::new("מאימתי", "\"יתגבר כארי\"")
            .with_chip("mode", "ToratEmet")
            .with_chip("together", "Near5")
            .within(["bavli/berakhot".to_string()])
            .excluding(["chasidut".to_string()])
            .tagged(["השכמת הבוקר".to_string()]);
        assert_eq!(query.typed, "\"יתגבר כארי\"");
        assert_eq!(
            query.chips.get("together").map(String::as_str),
            Some("Near5")
        );
        assert!(query.said().contains("bavli/berakhot"));
        assert!(query.said().contains("not chasidut"));
        assert!(query.has_tag("השכמת הבוקר"));
    }

    #[test]
    fn a_query_with_no_name_or_nothing_to_ask_is_refused() {
        let mut queries = Queries::nowhere();
        assert!(queries.save(SavedQuery::new("  ", "שבת")).is_err());
        assert!(queries.save(SavedQuery::new("שבת", "   ")).is_err());
    }

    #[test]
    fn saved_queries_survive_a_restart_and_saving_over_a_name_replaces_it() {
        let dir = crate::note::tests::scratch("queries");
        let (mut queries, _) = Queries::open(&dir);
        queries
            .save(SavedQuery::new("מאימתי", "מאימתי קורין").saved_at(1))
            .expect("saves");
        queries
            .save(SavedQuery::new("מאימתי", "\"מאימתי קורין\"").saved_at(2))
            .expect("saves");

        let (back, trouble) = Queries::open(&dir);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(back.count(), 1);
        assert_eq!(
            back.get("מאימתי").map(|q| q.typed.clone()),
            Some("\"מאימתי קורין\"".to_string())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
