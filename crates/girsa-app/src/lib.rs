//! The reading workspace: what is on the shelf, what is open, and what keeps
//! two columns together.
//!
//! spec.md §5 and BUILDER.md W9. Everything here is the *model* — no window, no
//! webview, nothing that draws. That split is not tidiness: the one behaviour
//! W9 is accepted on is **where a pane lands when the pane beside it moves**,
//! and that is a fact about segment ids which a screenshot cannot establish and
//! a test can.
//!
//! # The rule the whole module turns on
//!
//! Two panes follow each other only when something in the corpus says they are
//! related:
//!
//! - the corpus **declares** it — Sefaria's schema for `Rashi on Berakhot` says
//!   `base_text_titles: [Berakhot]`, and then the addresses line up:
//!   `Rashi on Berakhot 2a:1:3` is the third comment on `Berakhot 2a:1`;
//! - or an **edge** W8 imported joins the two segments.
//!
//! Anything else and the panes are left alone. Two seforim both addressed `1:1`
//! line up by coincidence, and a column that scrolled on a coincidence would be
//! showing a reader one sefer while the header names another — BUILDER.md rule
//! 6, in the one place a reader would never think to check.

pub mod adjacent;
pub mod arrangement;
pub mod beside;
pub mod buffer;
pub mod citing;
pub mod display;
pub mod export;
pub mod fixing;
pub mod keys;
pub mod lens;
pub mod links;
pub mod markup;
pub mod mefarshim;
pub mod notes;
// A search hit, a lane result, an MCP answer and a printed line each invented
// "a segment, described for a surface", and the four disagreed about the title
// language, the address and the date.
pub mod naming;
pub mod reading;
pub mod scanning;
pub mod sending;
pub mod session;
pub mod shelf;
pub mod spans;
pub mod taxonomy;
// Three composers each said "what this answer could not see" and none of them
// could see the other two.
pub mod unseen;
pub mod workspace;

pub use adjacent::{Adjacency, Near};
pub use arrangement::Arrangement;
pub use beside::{Beside, Joined, Place, Relation};
pub use buffer::{Buffer, BufferError};
pub use citing::{linkify, who_cites, Citing, Linked};
pub use export::{export, Exported, Format};
pub use fixing::{correction, FixHere};
pub use girsa_note::since::{find_index, is_an_index, Unindexed, Written};
pub use keys::{Bound, Press, ACTIONS};
pub use lens::{Lens, Lenses};
pub use links::{touching, Link, Touching};
pub use naming::{Names, Naming};
pub use notes::{collect, note_here, yours, Marked, Wrote, Yours};
pub use reading::{gap, gap_over, readings, Gap, Scanned};
pub use scanning::{is_scan, mareh_makom, scan_of};
pub use sending::{quote, send, Selection, SendError, Sent};
pub use session::Session;
pub use shelf::{Companion, Open, Shelf, ShelfError};
pub use taxonomy::Branch;
pub use unseen::Unseen;
pub use workspace::{Axis, Layout, Pane, PaneId, Tab, Workspace};
