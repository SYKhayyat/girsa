//! Chaburah folders — *just named collections* (spec.md §11).
//!
//! The shiur you are giving on Thursday is four places in Shas, two of your own
//! notes, the sefer somebody lent you, and the search you keep re-typing. That
//! is a list, and the design work is to make sure it stays a list: a folder
//! here holds **members, not copies**, and a member is one of the three things
//! this library already has names for.
//!
//! | member | written as |
//! |---|---|
//! | a place in a sefer | `girsa:bavli/berakhot/2a:1#1` |
//! | a sefer, yours or the corpus's — a note is one | `work:note/מאימתי-קורין` |
//! | a saved query | `query:מאימתי` |
//!
//! One string each, so a collections file is greppable: searching it for a
//! segment id finds the chaburos that line is in, which is the same property
//! `personal/links.jsonl` is written for.
//!
//! There is deliberately no *note* member. A note **is** a sefer on your shelf
//! (see [`crate::note`]), and giving it a second kind of membership would be
//! the first crack in the claim W27 exists to make.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use girsa_corpus::segment::SegmentId;
use girsa_corpus::standing::Standing;
use girsa_personal::Log;
use serde::{Deserialize, Serialize};

/// One thing in a folder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Member {
    /// A place in a sefer — a segment, permanently named (spec.md §3).
    Place(SegmentId),
    /// A whole sefer, by slug. A note of yours is one of these.
    Work(String),
    /// A saved query, by name.
    Query(String),
}

impl std::fmt::Display for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Place(id) => write!(f, "{id}"),
            Self::Work(slug) => write!(f, "work:{slug}"),
            Self::Query(name) => write!(f, "query:{name}"),
        }
    }
}

impl std::str::FromStr for Member {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if let Some(slug) = s.strip_prefix("work:") {
            return Ok(Self::Work(slug.to_string()));
        }
        if let Some(name) = s.strip_prefix("query:") {
            return Ok(Self::Query(name.to_string()));
        }
        s.parse::<SegmentId>()
            .map(Self::Place)
            .map_err(|e| format!("`{s}` is not a place, a sefer or a query: {e}"))
    }
}

impl Serialize for Member {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Member {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// A named list of things you are holding together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    /// What you ask for it by.
    pub name: String,
    /// What it says on the folder — Hebrew, usually, and free.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// In the order you put them in, which is the order a shiur goes in.
    /// **Not sorted**: the sequence is the content of a chaburah.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<Member>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub when: u64,
}

impl Collection {
    #[must_use]
    pub fn new(name: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            members: Vec::new(),
            tags: Vec::new(),
            when: girsa_personal::now_seconds(),
        }
    }

    /// Made at this moment rather than now — for a test, and for reading
    /// somebody else's file.
    #[must_use]
    pub const fn made_at(mut self, when: u64) -> Self {
        self.when = when;
        self
    }

    /// Put something in. `false` if it was already there — a folder is a list
    /// and not a tally.
    pub fn put(&mut self, member: Member) -> bool {
        if self.members.contains(&member) {
            return false;
        }
        self.members.push(member);
        true
    }

    /// Take something out.
    pub fn take_out(&mut self, member: &Member) -> bool {
        let before = self.members.len();
        self.members.retain(|kept| kept != member);
        self.members.len() != before
    }

    /// Move a member to a new position in the list. `false` if it is not in it.
    ///
    /// The order of a chaburah is the chaburah, so this is not decoration.
    pub fn move_to(&mut self, member: &Member, to: usize) -> bool {
        let Some(from) = self.members.iter().position(|kept| kept == member) else {
            return false;
        };
        let held = self.members.remove(from);
        let to = to.min(self.members.len());
        self.members.insert(to, held);
        true
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

    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|kept| crate::same_tag(kept, tag))
    }

    /// Whether a place is in this folder — either named outright, or by way of
    /// the sefer it is in.
    ///
    /// A place put in before a cut carved the line up is still in the folder
    /// after it (spec.md §3); a se'if upstream inserted beside it never was.
    /// See [`Standing`].
    #[must_use]
    pub fn holds(&self, at: &Standing) -> bool {
        self.members.iter().any(|member| match member {
            Member::Place(id) => at.named_by(id),
            Member::Work(slug) => slug == at.at().work(),
            Member::Query(_) => false,
        })
    }
}

/// Why a folder was not written.
#[derive(Debug, thiserror::Error)]
pub enum CollectionError {
    #[error("writing {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("a folder needs a name")]
    Unnamed,
}

/// Where they live under a personal layer.
#[must_use]
pub fn path_in(personal: &Path) -> PathBuf {
    personal.join("collections.jsonl")
}

/// Your folders.
///
/// # One line written per folder
///
/// The file is a [`Log`]: writing one down appends it, throwing one away appends
/// a tombstone, and the file is rewritten only when it has grown past twice what
/// it holds. Editing a folder — which is what adding a member is — appends the
/// folder as it now stands, so a chaburah list built up over a term costs one
/// line an edit instead of the whole file.
#[derive(Debug, Clone)]
pub struct Collections {
    log: Log,
    by_name: BTreeMap<String, Collection>,
}

girsa_personal::io_from_log_error!(CollectionError);

/// The replay, the index and the compaction — `girsa_personal::Store`.
impl girsa_personal::Store for Collections {
    type Record = Collection;
    const WHAT: &'static str = "a folder";

    fn key_of(c: &Collection) -> String {
        c.name.clone()
    }
    fn log(&self) -> &Log {
        &self.log
    }
    fn hold(&mut self, c: Collection) {
        self.by_name.insert(c.name.clone(), c);
    }
    fn count(&self) -> usize {
        self.by_name.len()
    }
    fn records(&self) -> Vec<&Collection> {
        self.by_name.values().collect()
    }
}

impl Collections {
    /// Read them. A line that will not parse costs that folder and is reported.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        girsa_personal::open(Self {
            log: Log::at(path_in(personal)),
            by_name: BTreeMap::new(),
        })
    }

    /// A layer that is never written.
    #[must_use]
    pub fn nowhere() -> Self {
        Self {
            log: Log::nowhere(),
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

    pub fn all(&self) -> impl Iterator<Item = &Collection> {
        self.by_name.values()
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Collection> {
        self.by_name.get(name)
    }

    /// The folders a place is in — what the reading pane asks so it can say
    /// *this line is in your Thursday chaburah*.
    #[must_use]
    pub fn holding(&self, at: &Standing) -> Vec<&Collection> {
        self.all().filter(|folder| folder.holds(at)).collect()
    }

    #[must_use]
    pub fn by_tag(&self, tag: &str) -> Vec<&Collection> {
        self.all().filter(|folder| folder.has_tag(tag)).collect()
    }

    /// Write one down. Saving over a name replaces it.
    ///
    /// # Errors
    ///
    /// If it has no name, or your layer will not write.
    pub fn save(&mut self, collection: Collection) -> Result<&Collection, CollectionError> {
        if collection.name.trim().is_empty() {
            return Err(CollectionError::Unnamed);
        }
        let name = collection.name.clone();
        // Written down before it is held, so a folder that will not save is not
        // one the shelf says you have.
        self.log.append(&collection)?;
        self.by_name.insert(name.clone(), collection);
        self.by_name.get(&name).ok_or(CollectionError::Unnamed)
    }

    /// Throw a folder away. The things in it are untouched — it held members,
    /// not copies.
    ///
    /// # Errors
    ///
    /// If your layer will not write.
    pub fn remove(&mut self, name: &str) -> Result<bool, CollectionError> {
        if !self.by_name.contains_key(name) {
            return Ok(false);
        }
        self.log.took(&[name])?;
        Ok(self.by_name.remove(name).is_some())
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::segment::Ordinal;

    fn place(n: u32) -> SegmentId {
        SegmentId::new(
            "bavli/berakhot",
            vec!["2a".to_string(), n.to_string()],
            Ordinal::root(n),
        )
    }

    #[test]
    fn a_member_is_one_string_so_the_file_is_greppable() {
        for member in [
            Member::Place(place(1)),
            Member::Work("note/מאימתי".to_string()),
            Member::Query("מאימתי".to_string()),
        ] {
            let written = serde_json::to_string(&member).expect("writes");
            let back: Member = serde_json::from_str(&written).expect("reads");
            assert_eq!(back, member);
        }
        assert_eq!(
            serde_json::to_string(&Member::Place(place(1))).expect("writes"),
            "\"girsa:bavli/berakhot/2a:1#1\""
        );
    }

    #[test]
    fn the_order_of_a_chaburah_is_the_chaburah() {
        let mut folder = Collection::new("thursday", "חבורה יום ה");
        assert!(folder.put(Member::Place(place(1))));
        assert!(folder.put(Member::Place(place(2))));
        assert!(folder.put(Member::Work("note/מאימתי".to_string())));
        assert!(!folder.put(Member::Place(place(1))), "a list, not a tally");

        assert!(folder.move_to(&Member::Work("note/מאימתי".to_string()), 0));
        assert_eq!(folder.members[0], Member::Work("note/מאימתי".to_string()));
        assert_eq!(folder.members[1], Member::Place(place(1)));

        assert!(folder.take_out(&Member::Place(place(1))));
        assert_eq!(folder.members.len(), 2);
    }

    #[test]
    fn a_folder_holds_a_place_named_outright_and_one_named_by_its_sefer() {
        let mut folder = Collection::new("thursday", "חבורה");
        folder.put(Member::Place(place(1)));
        assert!(folder.holds(&Standing::just(place(1))));
        assert!(!folder.holds(&Standing::just(place(2))));
        // And after a cut carves up the line it was put in on: the piece
        // inherits the name because the cut took the parent off the shelf.
        let piece = place(1).split(2).remove(1);
        assert!(folder.holds(&Standing::of(piece.clone(), [place(1)])));
        // But a se'if merely *named* below it — what upstream inserting one
        // after `#1` is spelled like — was never put in any folder.
        assert!(!folder.holds(&Standing::just(piece)));

        folder.put(Member::Work("bavli/berakhot".to_string()));
        assert!(
            folder.holds(&Standing::just(place(2))),
            "the whole sefer is in the folder"
        );
    }

    #[test]
    fn folders_survive_a_restart() {
        let dir = crate::note::tests::scratch("collections");
        let (mut collections, _) = Collections::open(&dir);
        let mut folder = Collection::new("thursday", "חבורה יום ה").made_at(1);
        folder.put(Member::Place(place(1)));
        folder.put(Member::Query("מאימתי".to_string()));
        collections.save(folder).expect("saves");

        let (back, trouble) = Collections::open(&dir);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(back.count(), 1);
        assert_eq!(back.holding(&Standing::just(place(1))).len(), 1);
        assert_eq!(
            back.get("thursday").map(|f| f.members.len()),
            Some(2),
            "and the query is in it beside the place"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
