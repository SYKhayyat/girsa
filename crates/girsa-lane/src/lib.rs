//! The semantic lane — *I remember a Rishon who says something like this but
//! not the words*.
//!
//! spec.md §9.9, ruled in §16 #20, built as BUILDER.md W30. It is the one thing
//! in the spec that was held back for a ruling rather than for work, and the
//! ruling is what shapes every module here:
//!
//! | The ruling | Where it lives |
//! |---|---|
//! | You side-load the model; the fetch button is off until you turn it on | [`model`], [`bring`] |
//! | Off by default, and off means literal search is unchanged | [`lane::Settings`] |
//! | You choose what is embedded, at any granularity | [`chosen`] |
//! | Background, resumable, never blocks reading | [`job`], [`lane::Run`] |
//! | The lane always states its own coverage | [`coverage`] |
//! | An answer says whether it read the whole store or a shortlist | [`signature`], [`vectors::Ranked`] |
//! | It is drawn as adjacent, always | [`lane::Adjacent`], [`lane::ADJACENT`] |
//!
//! # What it is not
//!
//! It is **not a rung on the relaxation ladder** (spec.md §9.6). A zero-result
//! literal query offers other spellings, other forms, a root, an abbreviation
//! and a wider proximity — and it does not, ever, quietly widen into
//! embeddings. Those rungs are countable before the click; an embedding lane is
//! not the same kind of answer and cannot be offered as though it were. The
//! test that holds this lives in `girsa-search`, where the ladder is, because a
//! guarantee tested in the crate that would have to break it is worth more than
//! one tested here.
//!
//! It is also **not a paskener** (spec.md §14). It assists retrieval. Every
//! surface that draws a result from it says so, in the one wording
//! [`lane::ADJACENT`] holds.

pub mod bring;
pub mod chosen;
pub mod coverage;
pub mod job;
pub mod lane;
pub mod model;
pub mod signature;
pub mod vectors;

pub use bring::{bring, BringError, Offer, BEREL};
pub use chosen::Chosen;
pub use coverage::{Coverage, Covered};
pub use job::Job;
pub use lane::{
    reads_as_a_question, Adjacent, Asked, Lane, LaneError, Run, Settings, Standing, State,
    ADJACENT, A_QUESTION, MEASURED, MOST, SHORTLISTED,
};
pub use model::{how_long, Embedded, Embedder, Model, ModelError, SEGMENTS_A_SECOND};
pub use signature::Signature;
pub use vectors::{Ranked, VectorError, Vectors};
