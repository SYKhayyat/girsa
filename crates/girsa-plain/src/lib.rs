//! The two things every binary needs and no corpus explains.
//!
//! [`argv`] reads a command line the way all sixteen tools read it. [`said`]
//! joins the clauses of *here is what this answer did not look at* the way all
//! three surfaces join them.
//!
//! # Why this crate exists at all
//!
//! Both of them used to live in `girsa-corpus`, and the 9 August
//! three-repository report said why:
//!
//! > `girsa-corpus` has become the workspace basement: 886 lines of
//! > `argv`/`said`/`roots`/`csv` live in the ingest crate because the ingest
//! > crate is the one everything can `use`, so every UI-string helper is shipped
//! > to `girsa-scan` and compiled at `opt-level = 2` as ingest code.
//!
//! *"The one everything can `use`"* is the whole diagnosis. Neither module has a
//! single line about a corpus — between them they name a `PathBuf`, an
//! `ExitCode` and a `Display` derive, and nothing else. They landed there
//! because there was nowhere lower, and a crate that everything depends on will
//! accumulate whatever has nowhere else to go until somebody makes it a place on
//! purpose.
//!
//! This is that place, on purpose, and its dependency list is one derive macro
//! for `Display`. It knows about no corpus, no shelf, no ref and no personal
//! layer, and nothing may be added here that does — which is what keeps it a
//! leaf rather than the next basement.
//!
//! # Two of the four, and the argument for the other two
//!
//! The report names four modules. Two of them moved and two did not, because
//! two of them are about a corpus and the report is wrong about that:
//!
//! - **`girsa_corpus::roots`** answers *what makes a directory a corpus*, and
//!   the answer is `works/index.jsonl`. That is a fact about the corpus format,
//!   and its own header already argues the case: the rule used to live in the
//!   Tauri shell, under a README saying the shell decides nothing, and it was
//!   moved **into** `girsa-corpus` for exactly this reason. Moving it out again
//!   to satisfy a line count would undo a fix.
//! - **`girsa_corpus::csv`** reads Sefaria's link CSVs, and exists because the
//!   fields contain the commas. That is ingest, in the ingest crate.
//!
//! Which leaves 612 lines of the 886, and those are the ones that had no
//! business being compiled as ingest code.

pub mod argv;
pub mod said;

pub use said::Clauses;
