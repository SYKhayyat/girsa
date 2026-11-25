//! The command surface, one domain per file.
//!
//! `lib.rs` held all of it until #68 began taking it apart. The rule for the split, and the reason it is safe to do at
//! all: **one domain per commit, pure movement, and `node tools/commands.mjs`
//! green after every one.** That tool reads every `.rs` file under
//! `app/src-tauri/src/`, so a command that moves is still counted, and a command
//! that is renamed in one place and not another fails the gate rather than the
//! button.
//!
//! Nothing here decides anything. Each module adapts a question the window asked
//! into a call on [`girsa_app`] and hands the answer back as JSON — see the note
//! at the top of `lib.rs` for why that rule is not negotiable: none of it can be
//! tested in a webview and all of it can be tested one directory up.

pub(crate) mod buffer;
pub(crate) mod chain;
pub(crate) mod cite;
pub(crate) mod corrections;
pub(crate) mod desks;
pub(crate) mod export;
pub(crate) mod ksav;
pub(crate) mod lane;
pub(crate) mod links;
pub(crate) mod luach;
pub(crate) mod mefarshim;
pub(crate) mod nav;
pub(crate) mod panes;
pub(crate) mod questions;
pub(crate) mod queue;
pub(crate) mod reading;
pub(crate) mod scans;
pub(crate) mod search;
pub(crate) mod settings;
pub(crate) mod shelf;
pub(crate) mod updates;
pub(crate) mod yours;
