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

/// What the lane was **measured** to do, and at what size.
///
/// # Why this is a constant and not prose in four places
///
/// It was prose in one place, and that place was a tool description for a
/// robot: `girsa_mcp::tools`, which told an agent *"measured to work on a
/// half-remembered statement and to work poorly on a question. It does not
/// pasken."* That is the most honest writing about this feature anywhere in the
/// repository and **the reader could not see any of it.** The window drew
/// [`ADJACENT`] and a coverage count and nothing about what the thing is known
/// to be bad at.
///
/// The second clause is the one nobody had written down at all. The measurement
/// in [`crate::model`] is over **240 se'ifim** of Hilchos Tefillah: top-16 of
/// 240 every time, first 8 times in 10, at a cosine spread of 0.74 against 0.63.
/// A 0.11 margin over 240 candidates is a *different claim* from a 0.11 margin
/// over five million — at 240 the tail is empty, and at 5,000,545 the tail is
/// the answer set. The crate names its other limits with real rigour (the
/// question-vs-statement asymmetry, mean-centring tried and rejected on
/// evidence, throughput) and did not name this one, while offering
/// [`crate::Chosen::everything`] as a standing choice.
pub const MEASURED: &str = "measured on a half-remembered statement, and it works poorly \
                            on a question — over 240 se'ifim, not over the whole shelf. \
                            It does not pasken.";

/// What a ranking drawn from a signature shortlist has to say for itself.
///
/// One wording, for the same reason [`ADJACENT`] and [`MEASURED`] are one
/// wording: a window, a command line and an MCP client that worded this three
/// ways would be making three different claims about the same answer.
///
/// It is drawn only when it is true — see [`crate::vectors::Ranked::whole`].
/// Under [`crate::vectors`]'s threshold, and for any store with no index, every
/// vector is read and there is nothing to disclaim.
pub const SHORTLISTED: &str = "ranked from a shortlist rather than by reading every vector \
                               — fast, and a near result the shortlist misjudged is not here";

/// What to say when the query itself is the thing the lane is bad at.
///
/// [`MEASURED`] says the lane works poorly on a question. It says it about
/// **every** answer, which is the right place to start and is not where this
/// should stop: a reader who has just typed a question is being told a general
/// caveat when the specific one applies, and ten plausible-looking rows are
/// sitting under it.
///
/// The numbers are the ones [`crate::model`] measured over 240 se'ifim of
/// Hilchos Tefillah, and they are not close. Asked as a statement you half
/// recall, the right se'if is in the top ten for **ten of ten** pairs, worst
/// case sixteenth. Asked as a question about that same se'if, **one of twelve**
/// reaches the top ten and the worst is ninety-seventh. That is not a model
/// having a bad day; BEREL is a masked-language model and not a sentence
/// encoder, and a question and its answer do not sit near each other in its
/// space.
pub const A_QUESTION: &str = "this reads as a question, and the lane is measured to be poor at                               those — one in twelve reaches the top ten, against ten in ten for                               a line you half remember. Try writing the line as you recall it.";

/// Whether a query reads as a question.
///
/// A leading interrogative, or a question mark anywhere. **Deliberately narrow**
/// — `מה` is a prefix of ordinary words and turns up mid-sentence in perfectly
/// good half-remembered lines, so only the first word is looked at. Under-
/// reporting leaves a reader exactly where they were before this existed;
/// over-reporting puts a wrong caveat over a good answer, and a caveat a reader
/// learns to ignore is worse than none.
///
/// It changes **nothing about the ranking**. The same results come back in the
/// same order, with a sentence over them — the lane does not decide it knows
/// better than the reader what they meant to type.
#[must_use]
pub fn reads_as_a_question(text: &str) -> bool {
    if text.contains('?') || text.contains('\u{061F}') {
        return true;
    }
    // The interrogatives, Hebrew and English.
    //
    // **`במה` is deliberately absent**, and it is the best argument for keeping
    // this list short and by hand rather than reaching for a pattern: `במה
    // דברים אמורים` — בד"א — is one of the commonest *statement* openers in
    // halachic literature, and a rule that caught it would put a wrong caveat
    // over exactly the kind of half-remembered line this lane is good at. `אם`
    // and `is` are out for the same reason: ordinary words far more often than
    // openings.
    //
    // `מהו` and `מהי` were missed by the first draft, which had `מה` and
    // assumed the rest followed. They do not — `מהו הדין` is the commonest way
    // anybody asks a question here, and it is a different word.
    const ASKING: [&str; 27] = [
        "מה",
        "ומה",
        "מהו",
        "מהי",
        "מי",
        "ומי",
        "מיהו",
        "מיהי",
        "מתי",
        "למה",
        "מדוע",
        "כיצד",
        "איך",
        "האם",
        "היכן",
        "מהיכן",
        "איפה",
        "לאן",
        "כמה",
        "מנין",
        "מניין",
        "מאין",
        "what",
        "who",
        "when",
        "why",
        "how",
    ];
    let first = text
        .split_whitespace()
        .next()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .unwrap_or_default();
    ASKING
        .iter()
        .any(|asking| first.eq_ignore_ascii_case(asking))
}

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

/// A lane's answer, and what it took to get it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Asked {
    /// Best first, across every sefer asked.
    pub adjacent: Vec<Adjacent>,
    /// Every sefer asked was read whole. False means at least one answered from
    /// a signature shortlist — see [`crate::signature`] — and a vector the
    /// estimate misjudged is not in this list.
    pub whole: bool,
    /// Records read off disk, across every sefer. The number the 9 August report
    /// says is stated nowhere.
    pub read: usize,
    /// How many seforim were asked. A store made by another model is skipped and
    /// not counted.
    pub seforim: usize,
    /// The query reads as a question, which is what this lane is measured to be
    /// worst at. See [`reads_as_a_question`] and [`A_QUESTION`].
    pub reads_as_a_question: bool,
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
    /// The stores this lane has already opened, by slug.
    ///
    /// [`Lane::ask`] opened every sefer in the selection **per query**, and
    /// opening one walks every record in its file to build the offset map. The
    /// 9 August report counts that as one of the two full linear passes a query
    /// costs: *"`Vectors::open` reads every record's id to build an offset map
    /// and `nearest` reads the whole file again"*. The other pass is the
    /// signature index; this is the one that is simply a cache.
    ///
    /// Keyed on the vectors file's **length**, which is what makes it safe: the
    /// file is append-only, so a store that grew is a store whose length
    /// changed, and one `metadata` call per sefer per query is the whole
    /// validation. A job that embeds four hundred more segments invalidates the
    /// sefer it embedded them into and nothing else.
    ///
    /// The model's fingerprint is **in** the key. The map is shared with this
    /// lane's clones — `Lane` is `Clone` and a clone points at the same personal
    /// layer — and a clone whose model was changed under it must not be handed
    /// back a store built in another model's space, which is the one mistake
    /// this file format exists to prevent.
    stores: Arc<std::sync::Mutex<Opened>>,
}

/// Vector stores this lane has already opened, by fingerprint-and-slug, with the
/// file length they were read at — see the note on [`Lane::stores`].
///
/// A named type because the shape has four layers and clippy is right that a
/// signature carrying all four says nothing: `Arc<Mutex<HashMap<String, (u64,
/// Arc<Vectors>)>>>` is punctuation, and *what a lane has already opened* is
/// the fact.
type Opened = std::collections::HashMap<String, (u64, Arc<crate::Vectors>)>;

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
            stores: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
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
            stores: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
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
        // The stores held here were opened for the old model, and a store's
        // whole reason for naming its model is that vectors from two of them
        // rank against each other perfectly happily and mean nothing.
        if let Ok(mut stores) = self.stores.lock() {
            stores.clear();
        }
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
        Ok(self.ask_reporting(text, over, most)?.adjacent)
    }

    /// [`Lane::ask`], and what it took to answer.
    ///
    /// The header of [`crate::vectors::Ranked`] has the argument: a ranking over
    /// a shortlist and a ranking over a whole store look identical, so the store
    /// says which it gave. This carries that up to the caller, over the whole
    /// selection — `whole` is true only when **every** sefer asked was read
    /// whole.
    ///
    /// # Errors
    ///
    /// As [`Lane::ask`].
    pub fn ask_reporting(
        &self,
        text: &str,
        over: &[String],
        most: usize,
    ) -> Result<Asked, LaneError> {
        let model = self.embedder()?;
        // Answered even for an empty query and a `most` of zero: *this reads as
        // a question* is a fact about what was typed, not about what came back,
        // and a caller drawing a caveat wants it whether or not there are rows
        // under it.
        let asking = reads_as_a_question(text);
        if text.trim().is_empty() || most == 0 {
            return Ok(Asked {
                reads_as_a_question: asking,
                ..Asked::default()
            });
        }
        let query: Vec<Embedded> = model.embed(&[text])?;
        let Some(query) = query.first() else {
            return Ok(Asked {
                reads_as_a_question: asking,
                ..Asked::default()
            });
        };

        let mut asked = Asked {
            reads_as_a_question: asking,
            whole: true,
            ..Asked::default()
        };
        for slug in over {
            let vectors = self.store(slug, model.fingerprint(), model.dims());
            if vectors.made_by_something_else().is_some() {
                continue;
            }
            let ranked = vectors.nearest_reporting(&query.vector, most)?;
            asked.whole &= ranked.whole;
            asked.read += ranked.read;
            asked.seforim += 1;
            for (id, nearness) in ranked.best {
                asked.adjacent.push(Adjacent { id, nearness });
            }
        }
        asked
            .adjacent
            .sort_by(|a, b| b.nearness.total_cmp(&a.nearness));
        asked.adjacent.truncate(most);
        Ok(asked)
    }

    /// One sefer's store, opened once and kept.
    ///
    /// See [`Lane::stores`] for why the file's length is the whole validation.
    /// A store that will not open is not cached — the trouble it reports is
    /// dropped here because `ask` has no line to put it on, and `standing` is
    /// the surface that exists to say so.
    fn store(&self, slug: &str, fingerprint: &str, dims: usize) -> Arc<Vectors> {
        let length = std::fs::metadata(Vectors::path_in(&self.personal, slug))
            .map(|m| m.len())
            .unwrap_or(0);
        // One allocation rather than the two a `(String, String)` key would
        // cost. The separator is a control character, which is in neither a
        // fingerprint nor a slug.
        let key = format!("{fingerprint}\u{1}{slug}");
        let mut stores = match self.stores.lock() {
            Ok(held) => held,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some((was, held)) = stores.get(&key) {
            if *was == length {
                return Arc::clone(held);
            }
        }
        let (opened, _) = Vectors::open(&self.personal, slug, fingerprint, dims);
        let opened = Arc::new(opened);
        stores.insert(key, (length, Arc::clone(&opened)));
        opened
    }

    /// Forget the opened stores.
    ///
    /// Not needed for correctness — the length check catches an appended file —
    /// and here for the caller that wants the memory back after a query over the
    /// whole shelf, and for [`Vectors::restarted`], which makes a file *shorter*
    /// at a length it has held before.
    pub fn forget_stores(&self) {
        if let Ok(mut stores) = self.stores.lock() {
            stores.clear();
        }
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
    fn a_question_is_recognised_and_a_half_remembered_line_is_not() {
        // The asymmetry this guards is not subtle. Over 240 se'ifim: a
        // statement you half recall puts the right one in the top ten **ten
        // times in ten**; a question about that same se'if manages it **once in
        // twelve**, worst case ninety-seventh. BEREL is a masked-language model
        // and not a sentence encoder, so a question and its answer do not sit
        // near each other in its space.
        for asking in [
            "מה הדין בקריאת שמע של ערבית",
            // The commonest way anybody asks one, and the first draft of the
            // list missed it: it had `מה` and assumed the rest followed.
            "מהו הדין בברכת המזון",
            "מהי הברכה על הפירות",
            "מיהו החייב בתפילה",
            "מהיכן למדנו את זה",
            "לאן הולכת הברכה",
            "מתי זמן קריאת שמע",
            "האם מותר לאכול קודם התפילה",
            "למה תיקנו תפילת ערבית",
            "כיצד מברכין על הלחם",
            "why did they establish maariv",
            "how is this brocha said",
            // A question mark alone is enough, whatever it opens with.
            "זמן קריאת שמע?",
        ] {
            assert!(reads_as_a_question(asking), "{asking} reads as a question");
        }

        // And the other side, which is the one that costs something to get
        // wrong: a wrong caveat over a good answer is a caveat a reader learns
        // to ignore.
        for line in [
            "מאימתי קורין את שמע בערבין",
            "יתגבר כארי לעמוד בבוקר לעבודת בוראו",
            "לא יפסיק בין גאולה לתפילה",
            // `מה` mid-sentence, which is the false positive a looser rule
            // would produce: only the first word is looked at.
            "וכל מה שיוכל להוסיף יוסיף",
            // **The one that argues for the whole design.** `במה דברים אמורים`
            // opens like a question and is one of the commonest statement
            // openers in halachic literature — a rule built out of a pattern
            // rather than a list would catch it, and put a wrong caveat over
            // exactly the kind of line this lane is good at.
            "במה דברים אמורים בזמן שבית המקדש היה קיים",
            "the line about standing up like a lion",
        ] {
            assert!(
                !reads_as_a_question(line),
                "{line} is a line somebody half remembers, not a question"
            );
        }
    }

    #[test]
    fn what_the_lane_says_about_a_question_names_both_numbers() {
        // The sentence is the whole of what this closes: a general caveat under
        // every answer became a specific one where it applies. It has to carry
        // the comparison, because *poor at questions* without *ten in ten for a
        // line* is a limit a reader cannot act on.
        assert!(A_QUESTION.contains("one in twelve"));
        assert!(A_QUESTION.contains("ten in ten"));
        assert!(
            A_QUESTION.contains("Try writing the line"),
            "and it says what to do instead"
        );
    }

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
