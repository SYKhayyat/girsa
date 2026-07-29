//! The lane itself: the setting that turns it on, and the question it answers.
//!
//! # Off by default, and off means off
//!
//! [`Settings::default`] is the lane switched off with no model named, and that
//! is what a reader who never opens the setting has. **Off is not a mode that
//! returns nothing** — nothing about literal search changes, no vector is read,
//! no model is loaded, and there is no lane drawn to be partial about. spec.md
//! §16 #20, and the test that holds it is `girsa-app`'s
//! `tests/adjacent_is_never_the_answer.rs` — over there because the strongest
//! form of the claim is about the **corpus tree**, which this crate cannot see:
//! every file under `corpus/` is byte-for-byte what it was before the lane ran.
//!
//! The absence that *does* get said out loud is the other one: the lane turned
//! **on** and pointed at nothing that is a model. That is
//! [`State::Adrift`], and it goes in the search header, because a reader who
//! turned the lane on and gets no adjacent results is owed the reason rather
//! than left to conclude the corpus has nothing like their query in it.
//!
//! # It is drawn as adjacent, always
//!
//! [`Adjacent`] is not [`girsa_search::index::Hit`] and there is deliberately no
//! conversion between them anywhere in this project. Nothing merges the two
//! lists; nothing sorts them together; no caller can accidentally hand one to
//! something expecting the other. spec.md §14 — *the lane assists retrieval and
//! does not pasken* — is a UI promise that is much easier to keep when the type
//! system will not let you break it by accident.
//!
//! [`girsa_search::index::Hit`]: https://docs.rs/girsa-search

use std::path::{Path, PathBuf};
use std::sync::Arc;

use girsa_corpus::import::{self, ImportedWork};
use girsa_corpus::segment::SegmentId;
use serde::{Deserialize, Serialize};

use crate::chosen::Chosen;
use crate::job::Job;
use crate::model::{Embedded, Embedder, Model, ModelError, BATCH};
use crate::vectors::{VectorError, Vectors};

/// What every surface calls a result from this lane, wherever it is drawn.
///
/// One string, because *adjacent* is a claim about how the result was found and
/// a window, a command line and an MCP client that worded it three ways would
/// be three different claims.
pub const ADJACENT: &str = "adjacent — found by meaning rather than by these words";

/// How many adjacent results a lane returns when nobody says.
pub const MOST: usize = 10;

/// One result from the lane.
///
/// It carries no text and no title. That is not laziness: attaching them here
/// would make this look like a search hit, and the whole design of §9.9 rests
/// on it never being one. The caller that draws it fetches the text through the
/// same shelf every other reader of a segment id uses.
#[derive(Debug, Clone, PartialEq)]
pub struct Adjacent {
    pub id: SegmentId,
    /// A cosine, between −1 and 1. Both sides are unit length.
    pub nearness: f32,
}

/// The lane's setting.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Off unless somebody turned it on.
    #[serde(default)]
    pub on: bool,
    /// The model directory — a reader pointed at it, or pressed the button in
    /// [`crate::bring`] and this is where it landed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<PathBuf>,
    /// Whether Girsa may go and get a model.
    ///
    /// **False in a fresh install, and false is the whole promise.** spec.md
    /// §14 says Girsa never *needs* the network; pointing the lane at a
    /// directory you already have is the default path and always works. With
    /// this off there is no code path from anywhere in the application to
    /// [`crate::bring`] — the button is not drawn and the function refuses.
    #[serde(default)]
    pub may_fetch: bool,
}

impl Settings {
    #[must_use]
    pub fn path_in(personal: &Path) -> PathBuf {
        personal.join("lane").join("settings.json")
    }

    /// Read them. A file that will not parse leaves the lane **off**, reported
    /// — the one wrong answer here that cannot mislead.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        let path = Self::path_in(personal);
        let Ok(body) = std::fs::read_to_string(&path) else {
            return (Self::default(), Vec::new());
        };
        match serde_json::from_str(&body) {
            Ok(settings) => (settings, Vec::new()),
            Err(e) => (
                Self::default(),
                vec![format!("{} will not read: {e}", path.display())],
            ),
        }
    }

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

/// Where the lane stands, in the words the header uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Nobody turned it on. Literal search is exactly what it was, and nothing
    /// is drawn.
    Off,
    /// On, and there is no model to run. **Said out loud**, with the reason.
    Adrift(String),
    /// On, with a model.
    On {
        /// The directory's name and the model's fingerprint.
        model: String,
        dims: usize,
    },
}

impl State {
    /// The line the search header shows, or `None` when there is nothing to
    /// say because there is no lane.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        match self {
            Self::Off => None,
            Self::Adrift(why) => Some(format!("the semantic lane is on but cannot run: {why}")),
            Self::On { model, .. } => Some(format!("the semantic lane is on, using {model}")),
        }
    }

    #[must_use]
    pub const fn is_on(&self) -> bool {
        matches!(self, Self::On { .. })
    }
}

/// Why the lane could not answer.
#[derive(Debug, thiserror::Error)]
pub enum LaneError {
    #[error("the semantic lane is off")]
    Off,
    #[error("{0}")]
    Model(#[from] ModelError),
    #[error("{0}")]
    Vectors(#[from] VectorError),
    #[error("{0}")]
    Corpus(#[from] import::ImportError),
}

/// One sefer's standing in the lane, counted from what is actually on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub slug: String,
    /// Segments with words in them that the selection asks for.
    pub wanted: usize,
    pub embedded: usize,
    /// What made the vectors, when it was not the model configured now.
    pub other_model: Option<String>,
}

/// The lane.
///
/// **Cloning one is cheap and shares the model.** That is what makes spec.md
/// §9.9's *never blocks reading* true rather than aspirational: the window keeps
/// one and hands a clone to the thread that embeds, and there is exactly one set
/// of weights in memory. Nothing here is behind a lock, because nothing here is
/// mutated by asking — the selection and the setting are replaced wholesale by
/// [`Lane::choose`] and [`Lane::set`], and a running job holds its own clone of
/// what was true when it started.
#[derive(Clone)]
pub struct Lane {
    personal: PathBuf,
    settings: Settings,
    chosen: Chosen,
    model: Option<Arc<dyn Embedder>>,
    adrift: Option<String>,
}

impl std::fmt::Debug for Lane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lane")
            .field("state", &self.state())
            .field("chosen", &self.chosen.len())
            .finish()
    }
}

impl Lane {
    /// Open the lane over a personal layer.
    ///
    /// **Loads the model when the lane is on**, which is hundreds of megabytes
    /// and takes a moment — so a caller with a window open should do this off
    /// the drawing thread. It is eager rather than lazy on purpose: a lane that
    /// loaded its model on the first query would make the first query the slow
    /// one, and a reader would learn that the feature is slow rather than that
    /// starting it is.
    ///
    /// The lane being off costs nothing at all: nothing is opened and nothing
    /// is read.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        let (settings, mut trouble) = Settings::open(personal);
        let (chosen, more) = Chosen::open(personal);
        trouble.extend(more);

        let mut lane = Self {
            personal: personal.to_path_buf(),
            settings,
            chosen,
            model: None,
            adrift: None,
        };
        if lane.settings.on {
            match lane.settings.model.clone() {
                None => lane.adrift = Some(ModelError::NotConfigured.to_string()),
                Some(dir) => match Model::side_loaded(&dir) {
                    Ok(model) => lane.model = Some(Arc::new(model)),
                    Err(e) => lane.adrift = Some(e.to_string()),
                },
            }
        }
        (lane, trouble)
    }

    /// A lane over a given embedder — what a test uses, and the seam that lets
    /// everything in this crate be checked without 738 MB of weights on the
    /// machine running the checks.
    #[must_use]
    pub fn with(personal: &Path, embedder: Arc<dyn Embedder>) -> Self {
        let (chosen, _) = Chosen::open(personal);
        Self {
            personal: personal.to_path_buf(),
            settings: Settings {
                on: true,
                ..Settings::default()
            },
            chosen,
            model: Some(embedder),
            adrift: None,
        }
    }

    #[must_use]
    pub fn state(&self) -> State {
        if !self.settings.on {
            return State::Off;
        }
        match (&self.model, &self.adrift) {
            (Some(model), _) => State::On {
                model: model.named(),
                dims: model.dims(),
            },
            (None, Some(why)) => State::Adrift(why.clone()),
            (None, None) => State::Adrift(ModelError::NotConfigured.to_string()),
        }
    }

    #[must_use]
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    #[must_use]
    pub fn chosen(&self) -> &Chosen {
        &self.chosen
    }

    #[must_use]
    pub fn personal(&self) -> &Path {
        &self.personal
    }

    /// Change what is in the lane, and write it down.
    ///
    /// # Errors
    ///
    /// If the personal layer will not take it.
    pub fn choose(&mut self, chosen: Chosen) -> Result<(), std::io::Error> {
        chosen.save(&self.personal)?;
        self.chosen = chosen;
        Ok(())
    }

    /// Turn it on or off, and point it at a model.
    ///
    /// Reloads the model, so this is as slow as [`Lane::open`] when it turns
    /// the lane on.
    ///
    /// # Errors
    ///
    /// If the personal layer will not take the setting. A model that will not
    /// load is **not** an error here — it is [`State::Adrift`], which is a
    /// state the header says out loud rather than a failed click.
    pub fn set(&mut self, settings: Settings) -> Result<(), std::io::Error> {
        settings.save(&self.personal)?;
        self.settings = settings;
        self.model = None;
        self.adrift = None;
        if self.settings.on {
            match self.settings.model.clone() {
                None => self.adrift = Some(ModelError::NotConfigured.to_string()),
                Some(dir) => match Model::side_loaded(&dir) {
                    Ok(model) => self.model = Some(Arc::new(model)),
                    Err(e) => self.adrift = Some(e.to_string()),
                },
            }
        }
        Ok(())
    }

    fn embedder(&self) -> Result<&dyn Embedder, LaneError> {
        match (&self.settings.on, &self.model) {
            (true, Some(model)) => Ok(model.as_ref()),
            (true, None) => Err(LaneError::Model(ModelError::NotConfigured)),
            (false, _) => Err(LaneError::Off),
        }
    }

    /// Ask the lane, over these seforim.
    ///
    /// `over` is which seforim to look in — the selection intersected with
    /// whatever the reader scoped the search to. It is passed rather than
    /// worked out here for the reason the whole crate is arranged around: which
    /// seforim are on the shelf is `girsa-app`'s question, and a second answer
    /// to it here would be a second shelf.
    ///
    /// # Errors
    ///
    /// If the lane is off, if the model will not run, or if a store will not
    /// read. A sefer whose vectors were made by another model is **skipped, not
    /// refused** — one wrong store may not cost the other four thousand — and
    /// it shows up in [`Lane::standing`], which is what the coverage line reads.
    pub fn ask(
        &self,
        text: &str,
        over: &[String],
        most: usize,
    ) -> Result<Vec<Adjacent>, LaneError> {
        let model = self.embedder()?;
        if text.trim().is_empty() || most == 0 {
            return Ok(Vec::new());
        }
        let query: Vec<Embedded> = model.embed(&[text])?;
        let Some(query) = query.first() else {
            return Ok(Vec::new());
        };

        let mut best: Vec<Adjacent> = Vec::new();
        for slug in over {
            let (vectors, _) =
                Vectors::open(&self.personal, slug, model.fingerprint(), model.dims());
            if vectors.made_by_something_else().is_some() {
                continue;
            }
            for (id, nearness) in vectors.nearest(&query.vector, most)? {
                best.push(Adjacent { id, nearness });
            }
        }
        best.sort_by(|a, b| b.nearness.total_cmp(&a.nearness));
        best.truncate(most);
        Ok(best)
    }

    /// What is embedded of one sefer, counted from the corpus and the store.
    ///
    /// Reads the sefer, so it is not free — the caller holds the answer for the
    /// session and refreshes it when the selection or the job changes, rather
    /// than recomputing it per query.
    ///
    /// # Errors
    ///
    /// If the sefer will not read.
    pub fn standing(&self, root: &Path, slug: &str) -> Result<Standing, LaneError> {
        let model = self.embedder()?;
        let work = import::read_back(root, slug)?;
        let (vectors, _) = Vectors::open(&self.personal, slug, model.fingerprint(), model.dims());
        let job = Job::of(&self.chosen, &work, &vectors);
        Ok(Standing {
            slug: slug.to_string(),
            wanted: job.wanted(),
            embedded: job.done(),
            other_model: vectors.made_by_something_else().map(str::to_string),
        })
    }

    /// Start embedding one sefer. See [`Run`].
    ///
    /// # Errors
    ///
    /// If the lane is off, or the sefer will not read.
    pub fn run(&self, root: &Path, slug: &str) -> Result<Run<'_>, LaneError> {
        let model = self.embedder()?;
        let work = import::read_back(root, slug)?;
        let (vectors, trouble) =
            Vectors::open(&self.personal, slug, model.fingerprint(), model.dims());
        let job = Job::of(&self.chosen, &work, &vectors);
        Ok(Run {
            model,
            chosen: self.chosen.clone(),
            work,
            vectors,
            job,
            trouble,
        })
    }
}

/// One sefer being embedded, a batch at a time.
///
/// Holding this open holds a sefer's text and one store. It holds no lock and
/// nothing else waits on it: **the reader can open the sefer, page through it,
/// search it and cite from it while this runs**, because everything the window
/// needs is the text on disk and everything this needs is the batch it is on
/// (spec.md §9.9, and W26's rule for the same reason).
///
/// Stopping is dropping it. What was written is what is done.
pub struct Run<'a> {
    model: &'a dyn Embedder,
    chosen: Chosen,
    work: ImportedWork,
    vectors: Vectors,
    job: Job,
    trouble: Vec<String>,
}

impl std::fmt::Debug for Run<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Run")
            .field("slug", &self.job.slug())
            .field("done", &self.job.done())
            .field("wanted", &self.job.wanted())
            .finish()
    }
}

impl Run<'_> {
    #[must_use]
    pub const fn job(&self) -> &Job {
        &self.job
    }

    #[must_use]
    pub fn work(&self) -> &ImportedWork {
        &self.work
    }

    /// What would not read on the way in. Named, never silent.
    #[must_use]
    pub fn trouble(&self) -> &[String] {
        &self.trouble
    }

    /// Whether this sefer's vectors were made by another model — in which case
    /// nothing can be added to them until [`Run::restart`].
    #[must_use]
    pub fn made_by_something_else(&self) -> Option<&str> {
        self.vectors.made_by_something_else()
    }

    /// Throw away vectors made by another model and begin again.
    ///
    /// # Errors
    ///
    /// If the personal layer will not write.
    pub fn restart(&mut self) -> Result<(), LaneError> {
        self.vectors.restarted()?;
        // The same selection, over an empty store: restarting throws away
        // vectors, never the reader's choice of what to embed.
        self.job = Job::of(&self.chosen, &self.work, &self.vectors);
        Ok(())
    }

    /// Embed one batch. Returns how many segments were written down.
    ///
    /// Zero means finished. The vectors are recorded **as they come back**, so
    /// a process that dies here loses the batch it was on and nothing else.
    ///
    /// # Errors
    ///
    /// If the model will not run, or the store will not take a vector.
    pub fn step(&mut self) -> Result<usize, LaneError> {
        let at = self.job.next(BATCH);
        if at.is_empty() {
            return Ok(0);
        }
        let texts: Vec<&str> = at
            .iter()
            .filter_map(|n| self.work.segments.get(*n))
            .map(|segment| segment.text.as_str())
            .collect();
        let vectors = self.model.embed(&texts)?;
        let mut wrote = 0;
        for (n, embedded) in at.iter().zip(vectors) {
            let Some(segment) = self.work.segments.get(*n) else {
                continue;
            };
            self.vectors.record(&segment.id, &embedded.vector)?;
            self.job.did(*n);
            wrote += 1;
        }
        Ok(wrote)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_lane_is_off_until_somebody_turns_it_on() {
        let settings = Settings::default();
        assert!(!settings.on);
        assert!(settings.model.is_none());

        let dir = std::env::temp_dir().join("girsa-lane-settings-default");
        let _ = std::fs::remove_dir_all(&dir);
        let (lane, trouble) = Lane::open(&dir);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(lane.state(), State::Off);
        assert_eq!(lane.state().said(), None, "off draws nothing at all");
        assert!(matches!(
            lane.ask("anything", &["x".to_string()], 5),
            Err(LaneError::Off)
        ));
    }

    #[test]
    fn on_with_no_model_is_said_out_loud_rather_than_returning_nothing() {
        let dir = std::env::temp_dir().join("girsa-lane-adrift");
        let _ = std::fs::remove_dir_all(&dir);
        let mut lane = Lane::open(&dir).0;
        lane.set(Settings {
            on: true,
            ..Settings::default()
        })
        .expect("saves");
        let said = lane.state().said().expect("a sentence");
        assert!(said.contains("the semantic lane is on"), "{said}");
        assert!(said.contains("downloads nothing"), "{said}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn on_and_pointed_at_something_that_is_not_a_model_names_what_is_missing() {
        let dir = std::env::temp_dir().join("girsa-lane-not-a-model");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("empty")).expect("a directory");
        let mut lane = Lane::open(&dir).0;
        lane.set(Settings {
            on: true,
            model: Some(dir.join("empty")),
            ..Settings::default()
        })
        .expect("saves");
        let said = lane.state().said().expect("a sentence");
        assert!(said.contains("config.json"), "{said}");
        assert!(said.contains("model.safetensors"), "{said}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_setting_survives_a_restart() {
        let dir = std::env::temp_dir().join("girsa-lane-settings-round-trip");
        let _ = std::fs::remove_dir_all(&dir);
        Settings {
            on: true,
            model: Some(PathBuf::from("/models/berel")),
            ..Settings::default()
        }
        .save(&dir)
        .expect("saves");
        let (back, trouble) = Settings::open(&dir);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert!(back.on);
        assert_eq!(back.model.as_deref(), Some(Path::new("/models/berel")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_settings_file_that_will_not_read_leaves_the_lane_off_and_says_so() {
        let dir = std::env::temp_dir().join("girsa-lane-settings-nonsense");
        let _ = std::fs::remove_dir_all(&dir);
        let path = Settings::path_in(&dir);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        std::fs::write(&path, "not json").expect("writes");
        let (settings, trouble) = Settings::open(&dir);
        assert!(!settings.on, "the safe answer is off");
        assert_eq!(trouble.len(), 1, "{trouble:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adjacent_is_worded_once() {
        assert!(ADJACENT.contains("rather than"));
        assert!(ADJACENT.starts_with("adjacent"));
    }
}
