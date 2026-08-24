//! The lane over the shelf — and the sentence that keeps it honest.
//!
//! spec.md §9.9, BUILDER.md W30. `girsa-lane` knows how to embed a segment and
//! how to rank a vector; it deliberately does not know what is on the shelf.
//! This module is the join, and it exists for the same reason
//! [`girsa_app::reading`] does: the sentence a reader is shown about what the answer
//! could not see has to be composed **once**, so the window, the command line,
//! the MCP surface and the test cannot drift into promising different things.
//!
//! # It is drawn beside the literal results, never among them
//!
//! [`Answer`] is not [`girsa_search::bar::Results`] and there is no conversion
//! either way anywhere in this project. A caller cannot accidentally append one
//! to the other, and every [`Answer`] carries [`girsa_lane::ADJACENT`] as the
//! label it must be drawn under. spec.md §14: the lane assists retrieval and
//! does not pasken.
//!
//! # Why coverage is held rather than recomputed
//!
//! Counting what is embedded means reading every chosen sefer off the disk and
//! opening every store. Over a shelf-sized selection that is seconds, which is
//! fine once and absurd per keystroke — so [`Adjacency`] holds the answer and
//! [`Adjacency::refresh`] recomputes it. The refresh points are the ones that
//! can change it: opening the lane, changing the selection, and finishing a
//! batch of embedding.
//!
//! Nothing is cached to disk. A cache of a coverage number is a thing that can
//! be wrong about the one claim in this feature a reader is being asked to
//! trust, and the numbers here are cheap enough to earn rather than remember.
//!
//! # The one search entry point the lane is deliberately kept out of
//!
//! **Cite-on-selection** (W18, `girsa_desk::citing`). Highlight a phrase in Ksav
//! and Girsa offers the mekoros for it; when nothing fits, §10.4 drops you into
//! the search box — where the lane is, and where it belongs.
//!
//! It is not wired into the candidate list itself, and that is BUILDER.md rule 6
//! rather than an omission. A mekor is a **claim about where words came from**. A
//! nearest neighbour in an embedding space is a claim that two passages are about
//! something similar, which is a different assertion — and the measurement in
//! `girsa_lane::model` is exactly why: the lane's own top hit sits at a cosine of
//! 0.74 against a field where unrelated se'ifim sit at 0.63. That spread is
//! plenty to rank a list a reader is reading as *adjacent*, and nowhere near
//! enough to put a citation under somebody's writing. *Where is this phrase from*
//! stays literal.

use std::path::{Path, PathBuf};

use girsa_corpus::segment::SegmentId;
use girsa_lane::coverage::Covered;
use girsa_lane::{
    Chosen, Coverage, Lane, LaneError, Settings, State, ADJACENT, MEASURED, MOST, SHORTLISTED,
};

use girsa_app::naming::{Names, Naming};
use girsa_app::shelf::{self, Shelf};

/// One adjacent result, ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Near {
    /// Which place this is, what to call it and when it was written — the
    /// description four surfaces used to compose separately. This one used to
    /// carry a work slug and a Hebrew title and **no address at all**, so the
    /// window and `girsa-lane ask` each invented one and invented different
    /// ones: `58:1` in the window, the whole permanent id on the terminal.
    pub at: Naming,
    pub text: String,
    /// A cosine. Shown, because a reader deciding whether to follow one of
    /// these is entitled to know how near it actually was.
    pub nearness: f32,
}

impl Near {
    #[must_use]
    pub fn id(&self) -> &SegmentId {
        &self.at.id
    }
}

/// What the lane has to say.
///
/// Every field is drawn: the label because the results are adjacent and must
/// read as adjacent, and the coverage sentence because a partial lane that
/// looks complete is the defect §9.9 exists to prevent.
#[derive(Debug, Clone)]
pub struct Answer {
    /// The label this must be drawn under. One wording, from `girsa-lane`.
    pub label: &'static str,
    /// What the lane was **measured** to do, and at what size —
    /// [`girsa_lane::MEASURED`].
    ///
    /// Drawn, like the other three. This sentence existed before this field
    /// did, in `girsa_mcp::tools`, where it told an *agent* that the lane works
    /// on a half-remembered statement and poorly on a question and does not
    /// pasken. The reader was told none of it. A limit stated to a robot and
    /// not to the person is not a stated limit.
    pub measured: &'static str,
    pub near: Vec<Near>,
    /// What the lane covers and what it does not, in words.
    pub coverage: String,
    /// Why there is nothing — the lane off, no model, a store from another
    /// model. **Never an empty list with no reason attached.**
    pub refused: Option<String>,
    /// [`girsa_lane::A_QUESTION`] when the query reads as a question, and
    /// `None` otherwise.
    ///
    /// The fifth thing this answer says about itself, and the sharpest.
    /// [`girsa_lane::MEASURED`] already says the lane is poor at questions —
    /// under *every* answer, which is the right place to start and the wrong
    /// place to stop. A reader who has just typed one is being given a general
    /// caveat where the specific one applies, with ten plausible-looking rows
    /// sitting under it. The measurement is not close: one in twelve reaches
    /// the top ten, against ten in ten for a line half remembered.
    ///
    /// It changes nothing about the ranking. The same rows in the same order,
    /// with a sentence over them — the lane does not decide it knows better
    /// than the reader what they meant to type.
    pub asking: Option<&'static str>,
    /// [`girsa_lane::SHORTLISTED`] when at least one sefer answered from a
    /// signature shortlist rather than by reading every vector it holds, and
    /// `None` when every one of them was read whole.
    ///
    /// The fourth thing this answer says about itself, and the newest. The
    /// other three — the label, the measurement, the coverage — are all
    /// answers to *what does this list not tell you*, and this is the same
    /// question about the retrieval rather than about the corpus.
    pub shortlisted: Option<&'static str>,
    /// Said when some of the lane's hits named a place this shelf could not
    /// open or resolve, and those rows were dropped: *how many, and what the
    /// list therefore is not*. The fifth admission, and the one that used to
    /// be silent — `filter_map` swallowed an unresolvable vector whole, so a
    /// lane asked for ten could answer with six and look complete. Composed
    /// here, where the count is known, like every other sentence on this
    /// struct; `None` when nothing was dropped, because a disclaimer nobody
    /// needs is noise.
    pub unresolved: Option<String>,
}

/// The lane, joined to the shelf.
pub struct Adjacency {
    lane: Lane,
    root: PathBuf,
    coverage: Coverage,
}

impl std::fmt::Debug for Adjacency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Adjacency")
            .field("state", &self.lane.state())
            .field("coverage", &self.coverage.said())
            .finish()
    }
}

impl Adjacency {
    /// Open the lane over this corpus and this shelf.
    ///
    /// Loads the model when the lane is on, which is hundreds of megabytes —
    /// see [`Lane::open`]. **The lane being off costs nothing**, which is what
    /// makes off-by-default a real default rather than a checkbox with a price.
    #[must_use]
    pub fn open(root: &Path, personal: &Path, shelf: &Shelf) -> (Self, Vec<String>) {
        let (lane, trouble) = Lane::open(personal);
        let mut adjacency = Self {
            lane,
            root: root.to_path_buf(),
            coverage: Coverage::default(),
        };
        adjacency.refresh(shelf);
        (adjacency, trouble)
    }

    /// A lane over a given embedder — the seam a test uses.
    #[must_use]
    pub fn with(root: &Path, lane: Lane, shelf: &Shelf) -> Self {
        let mut adjacency = Self {
            lane,
            root: root.to_path_buf(),
            coverage: Coverage::default(),
        };
        adjacency.refresh(shelf);
        adjacency
    }

    #[must_use]
    pub fn state(&self) -> State {
        self.lane.state()
    }

    #[must_use]
    pub fn coverage(&self) -> &Coverage {
        &self.coverage
    }

    #[must_use]
    pub fn lane(&self) -> &Lane {
        &self.lane
    }

    /// A copy of the lane for another thread.
    ///
    /// Cheap, and it **shares the model** rather than loading a second one —
    /// which is what makes spec.md §9.9's *never blocks reading* a fact about
    /// this design rather than a hope. The clone carries the selection as it was
    /// when the job started; changing it mid-job changes the next job.
    #[must_use]
    pub fn for_thread(&self) -> Lane {
        self.lane.clone()
    }

    /// Change the selection or the setting, then say what changed about
    /// coverage.
    ///
    /// # Errors
    ///
    /// If the personal layer will not take it.
    pub fn choose(&mut self, chosen: Chosen, shelf: &Shelf) -> Result<(), std::io::Error> {
        self.lane.choose(chosen)?;
        self.refresh(shelf);
        Ok(())
    }

    /// # Errors
    ///
    /// If the personal layer will not take it.
    pub fn set(&mut self, settings: Settings, shelf: &Shelf) -> Result<(), std::io::Error> {
        self.lane.set(settings)?;
        self.refresh(shelf);
        Ok(())
    }

    /// Count what is in the lane, from the corpus and the stores.
    ///
    /// Reads every chosen sefer. Cheap when nothing is chosen, which is the
    /// default, and seconds over a shelf — see the module note on why it is not
    /// remembered between runs.
    pub fn refresh(&mut self, shelf: &Shelf) {
        if !self.lane.state().is_on() {
            self.coverage = Coverage::default();
            return;
        }
        let chosen = self.lane.chosen().clone();
        let mut covered = Vec::new();
        let mut other_model = Vec::new();
        for slug in in_the_lane(shelf, &chosen) {
            let Ok(standing) = self.lane.standing(&self.root, &slug) else {
                continue;
            };
            if let Some(_made_by) = &standing.other_model {
                other_model.push(slug.clone());
            }
            covered.push(Covered {
                slug: slug.clone(),
                title: shelf
                    .work(&slug)
                    .map_or_else(|| slug.clone(), |work| work.he_title.clone()),
                wanted: standing.wanted,
                embedded: standing.embedded,
            });
        }
        let mut coverage = Coverage::of(covered, outside(shelf, &chosen), chosen.is_everything());
        coverage.other_model = other_model;
        self.coverage = coverage;
    }

    /// Ask the lane.
    ///
    /// `scoped_to` is what the reader narrowed the search to, empty for the
    /// whole shelf. The lane looks in the **intersection** of that and what is
    /// embedded, which is the only honest reading of two narrowings: a reader
    /// who scoped to the Bavli did not ask for a Rishon, and a reader who
    /// embedded only the Rishonim has no vectors for the Bavli.
    ///
    /// Never an error. A lane that is off, adrift or empty comes back with
    /// `refused` set and `coverage` said, because *nothing, and here is why* is
    /// an answer and an empty list is not.
    #[must_use]
    pub fn ask(&self, names: &Names, text: &str, scoped_to: &[String], most: usize) -> Answer {
        let shelf = names.shelf;
        let most = if most == 0 { MOST } else { most };
        let coverage = self.coverage.said();
        // A fact about what was typed, so it is known before the lane is even
        // asked — and said on a refusal too. A reader whose lane is off and who
        // typed a question is about to turn it on and type the same thing.
        let asking = girsa_lane::lane::reads_as_a_question(text).then_some(girsa_lane::A_QUESTION);
        let refuse = |why: String| Answer {
            label: ADJACENT,
            measured: MEASURED,
            near: Vec::new(),
            coverage: coverage.clone(),
            refused: Some(why),
            asking,
            // Nothing was ranked, so there is no ranking to disclaim.
            shortlisted: None,
            unresolved: None,
        };

        match self.lane.state() {
            State::Off => return refuse("the semantic lane is off".to_string()),
            State::Adrift(why) => return refuse(why),
            State::On { .. } => {}
        }

        let chosen = self.lane.chosen();
        let mut over: Vec<String> = in_the_lane(shelf, chosen);
        if !scoped_to.is_empty() {
            over.retain(|slug| scoped_to.iter().any(|scoped| scoped == slug));
        }
        if over.is_empty() {
            return refuse(if chosen.is_nothing() {
                "nothing has been added to the semantic lane yet".to_string()
            } else {
                "nothing in the semantic lane is inside this search's scope".to_string()
            });
        }

        let asked = match self.lane.ask_reporting(text, &over, most) {
            Ok(asked) => asked,
            Err(e) => return refuse(say(&e)),
        };
        let shortlisted = (!asked.whole).then_some(SHORTLISTED);
        // Count what will not resolve *before* dropping it. The lane names
        // places by vector; a sefer that will not read, or an id with no
        // position, is a row the reader will never see — and `filter_map`
        // used to make that loss invisible, so "showing 6" looked like the
        // whole answer to a request for ten.
        let mut unresolved: usize = 0;
        let mut near = Vec::with_capacity(asked.adjacent.len());
        // One read per **sefer**, not per hit: the lane's answers cluster, and
        // ten hits out of one daf used to parse that whole work ten times.
        let mut opened: std::collections::HashMap<String, Option<shelf::Open>> =
            std::collections::HashMap::new();
        for adjacent in asked.adjacent {
            // The text comes off the shelf, through the same reader every
            // other consumer of a segment id uses — corrections applied,
            // because the lane must show what the reader can see.
            let work = adjacent.id.work().to_string();
            let open = match opened.entry(work) {
                std::collections::hash_map::Entry::Occupied(known) => known.get().clone(),
                std::collections::hash_map::Entry::Vacant(miss) => {
                    let read = shelf.read(adjacent.id.work()).ok();
                    miss.insert(read.clone());
                    read
                }
            };
            match open.and_then(|open| {
                let at = open.position_of(&adjacent.id)?;
                Some(open.segments.get(at)?.text.clone())
            }) {
                Some(text) => near.push(Near {
                    at: names.of(&adjacent.id),
                    text,
                    nearness: adjacent.nearness,
                }),
                None => unresolved += 1,
            }
        }
        let unresolved = (unresolved > 0).then(|| {
            format!(
                "{unresolved} of the lane's hits name a place this shelf could not open, and are not shown"
            )
        });

        Answer {
            label: ADJACENT,
            measured: MEASURED,
            near,
            coverage,
            refused: None,
            asking,
            shortlisted,
            unresolved,
        }
    }

    /// Embed what is chosen, a batch at a time, until `keep_going` says stop.
    ///
    /// Returns how many segments were written down. Stopping is free and losing
    /// nothing: the vectors on disk are the progress record (spec.md §9.9's
    /// *background, resumable, never blocks reading*).
    ///
    /// # Errors
    ///
    /// If the lane is off, or a sefer or a store will not read. A single sefer
    /// that will not read is **named in the returned trouble and skipped**, not
    /// fatal — one bad sefer may not cost the reader the other four thousand.
    pub fn embed(
        &mut self,
        shelf: &Shelf,
        keep_going: &mut dyn FnMut(&str, usize, usize) -> bool,
    ) -> Result<(usize, Vec<String>), LaneError> {
        if !self.lane.state().is_on() {
            return Err(LaneError::Off);
        }
        let chosen = self.lane.chosen().clone();
        let mut wrote = 0;
        let mut trouble = Vec::new();
        for slug in in_the_lane(shelf, &chosen) {
            let mut run = match self.lane.run(&self.root, &slug) {
                Ok(run) => run,
                Err(e) => {
                    trouble.push(format!("{slug}: {}", say(&e)));
                    continue;
                }
            };
            trouble.extend(run.trouble().iter().cloned());
            if let Some(made_by) = run.made_by_something_else() {
                // Not restarted on the reader's behalf: that destroys work.
                trouble.push(format!(
                    "{slug}: its vectors were made by {made_by} and are not being added to"
                ));
                continue;
            }
            loop {
                match run.step() {
                    Ok(0) => break,
                    Ok(n) => {
                        wrote += n;
                        if !keep_going(&slug, run.job().done(), run.job().wanted()) {
                            self.refresh(shelf);
                            return Ok((wrote, trouble));
                        }
                    }
                    Err(e) => {
                        trouble.push(format!("{slug}: {}", say(&e)));
                        break;
                    }
                }
            }
        }
        self.refresh(shelf);
        Ok((wrote, trouble))
    }
}

/// Which seforim on this shelf the lane holds any part of.
///
/// Resolved against the shelf rather than trusted from the selection, so a
/// sefer a reader chose and then removed from the shelf does not go on being
/// counted as covered.
#[must_use]
pub fn in_the_lane(shelf: &Shelf, chosen: &Chosen) -> Vec<String> {
    if chosen.is_everything() {
        return shelf.works().iter().map(|work| work.slug.clone()).collect();
    }
    shelf
        .works()
        .iter()
        .filter(|work| chosen.holds(&work.slug))
        .map(|work| work.slug.clone())
        .collect()
}

/// Which seforim on this shelf the lane holds no part of. The other half of the
/// coverage sentence, and the half a reader would otherwise never be told.
#[must_use]
pub fn outside(shelf: &Shelf, chosen: &Chosen) -> Vec<String> {
    if chosen.is_everything() {
        return Vec::new();
    }
    shelf
        .works()
        .iter()
        .filter(|work| !chosen.holds(&work.slug))
        .map(|work| work.slug.clone())
        .collect()
}

fn say(error: &impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn an_answer_is_never_an_empty_list_with_no_reason() {
        // The whole shape of this module in one assertion: there is no way to
        // construct an `Answer` that says nothing about why it says nothing.
        let answer = Answer {
            label: ADJACENT,
            measured: MEASURED,
            near: Vec::new(),
            coverage: Coverage::default().said(),
            refused: Some("the semantic lane is off".to_string()),
            asking: None,
            shortlisted: None,
            unresolved: None,
        };
        assert!(answer.near.is_empty());
        assert!(answer.refused.is_some());
        assert!(!answer.coverage.is_empty());
        assert_eq!(answer.label, ADJACENT);
    }
}
