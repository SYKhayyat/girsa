//! The command surface, one domain per file.
//!
//! `lib.rs` held all of it — 6,914 lines and 139 commands — until #68 began
//! taking it apart. The rule for the split, and the reason it is safe to do at
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

pub(crate) mod nav;
