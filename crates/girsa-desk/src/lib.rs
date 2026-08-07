//! The desk: what you are writing, and what you have already written
//! (spec.md §10).
//!
//! Three modules — the buffer you type into, the rules for turning a
//! highlighted phrase into a mekor, and the registry of `.ksav` files on your
//! machine — and one thing they have in common that the reading workspace does
//! not: they all speak **Ksav**.
//!
//! # Why it is a crate
//!
//! `girsa-app` is *"the shelf, tabs and splits, and what keeps two columns
//! together"*, and its manifest had `girsa-ksav` and `girsa-cite` in it because
//! of these three files. Nothing in the reading pane compiles a document
//! format. Below this line is a sefer, an address and a correction; above it is
//! what you write with them.
//!
//! The dependency runs one way and only one way. [`girsa_app::sending`] hands
//! over a [`girsa_source::SourcePacket`] — the words, the place, and which
//! characters of it — and this crate decides what that looks like on a page.
//! The workspace has never needed to know.

pub mod buffer;
pub mod citing;
pub mod documents;
// The one errand a clipboard cannot express: a document's worth of quotes,
// re-read against the corpus as it stands now (spec.md §10.2).
pub mod refreshing;

pub use buffer::{Buffer, BufferError};
pub use citing::{linkify, who_cites, Citing, Linked};
pub use documents::Documents;
pub use refreshing::{refreshed, Refreshed, Wanted};
