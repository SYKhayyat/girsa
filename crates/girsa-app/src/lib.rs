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

pub mod arrangement;
pub mod beside;
pub mod buffer;
pub mod citing;
pub mod display;
pub mod export;
pub mod fixing;
pub mod sending;
pub mod session;
pub mod shelf;
pub mod taxonomy;
pub mod workspace;

pub use arrangement::Arrangement;
pub use beside::{Beside, Place, Relation};
pub use buffer::{Buffer, BufferError};
pub use citing::{linkify, who_cites, Citing, Linked};
pub use export::{export, Exported, Format};
pub use fixing::{correction, FixHere};
pub use sending::{quote, send, Selection, SendError, Sent};
pub use session::Session;
pub use shelf::{Companion, Open, Shelf, ShelfError};
pub use taxonomy::Branch;
pub use workspace::{Axis, Layout, Pane, PaneId, Tab, Workspace};
