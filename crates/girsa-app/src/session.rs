//! What the app remembers between one evening and the next.
//!
//! Two different kinds of memory, and they are kept apart on purpose:
//!
//! - the **workspace** — which tabs are open and how they are arranged;
//! - **where you were in each sefer**, for every sefer you have ever opened,
//!   whether or not it is open now. BUILDER.md W9 asks for per-sefer position
//!   memory, and the point of it is the sefer you closed three weeks ago.
//!
//! Written as one JSON file, local, no account (spec.md §11). It is a
//! preference file, not the corpus: losing it costs a layout, and the same rule
//! applies as everywhere else here — text files are the truth and this is not
//! one of them, so nothing in it is allowed to be the only copy of anything.

use std::collections::BTreeMap;
use std::path::Path;

use girsa_corpus::segment::SegmentId;
use serde::{Deserialize, Serialize};

use crate::workspace::Workspace;

/// Everything the app remembers.
///
/// [`Default`] is written out rather than derived. Derived, `nikud` would be
/// `false` — and since a session file that will not parse falls back to the
/// default, a corrupt preferences file would silently strip the nikud out of
/// every sefer on the shelf and look like a rendering bug.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub workspace: Workspace,
    /// Sefer → the segment you were last looking at in it.
    #[serde(default)]
    pub positions: BTreeMap<String, SegmentId>,
    /// Whether nikud is shown. One setting for the window, because a reader
    /// who turns it off wants it off — not off in this pane and on in the one
    /// beside it.
    #[serde(default = "yes")]
    pub nikud: bool,
    /// Reading size, as a percentage. Hebrew with nikud at a small size is
    /// unreadable in a way Latin text at the same size is not.
    #[serde(default = "hundred")]
    pub text_size: u16,
    /// How a citation is printed when a source is sent (spec.md §10.2, W15).
    ///
    /// A preference and not a fact about the quote: what the document stores
    /// is the ref, so changing this changes how every citation *prints* and
    /// nothing about where any of them point.
    #[serde(default = "full")]
    pub cite: girsa_cite::CiteStyle,
    /// How much of your correction layer is applied to what you read (W20).
    ///
    /// Remembered like the nikud toggle, and for the same reason: a reader who
    /// turned the corrections off to check what was printed wants them off
    /// until they say otherwise.
    #[serde(default)]
    pub showing: girsa_fix::Showing,
}

const fn yes() -> bool {
    true
}

const fn hundred() -> u16 {
    100
}

/// How a sefer prints a mekor, which is what a reader expects to see.
const fn full() -> girsa_cite::CiteStyle {
    girsa_cite::CiteStyle::HebrewFull
}

impl Default for Session {
    fn default() -> Self {
        Self {
            workspace: Workspace::default(),
            positions: BTreeMap::new(),
            nikud: yes(),
            text_size: hundred(),
            cite: full(),
            showing: girsa_fix::Showing::default(),
        }
    }
}

impl Session {
    /// Read the session back, or start a fresh one.
    ///
    /// A file that will not parse gives a **fresh session rather than an
    /// error**: a preference file the app refuses to start without is a
    /// preference file that will one day stop somebody reading.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|body| serde_json::from_str(&body).ok())
            .unwrap_or_default()
    }

    /// # Errors
    ///
    /// If the directory cannot be made or the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let body = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, body)
    }

    /// Remember where a reader is in a sefer.
    pub fn remember(&mut self, at: SegmentId) {
        self.positions.insert(at.work().to_string(), at);
    }

    /// Where they were, last time.
    #[must_use]
    pub fn where_i_was(&self, slug: &str) -> Option<&SegmentId> {
        self.positions.get(slug)
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::workspace::Axis;

    fn id(work: &str, n: u32) -> SegmentId {
        format!("girsa:{work}/2a:{n}#{n}")
            .parse()
            .expect("a segment id")
    }

    #[test]
    fn a_sefer_reopens_where_it_was_closed() {
        let mut session = Session::default();
        session.remember(id("bavli/berakhot", 7));
        session.remember(id("bavli/shabbat", 2));
        // Reading further on moves the memory rather than adding a second one.
        session.remember(id("bavli/berakhot", 9));

        assert_eq!(
            session.where_i_was("bavli/berakhot"),
            Some(&id("bavli/berakhot", 9))
        );
        assert_eq!(session.where_i_was("bavli/eruvin"), None);
        assert_eq!(session.positions.len(), 2);
    }

    #[test]
    fn the_whole_session_survives_a_restart() {
        let dir = std::env::temp_dir().join("girsa-app-session-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let mut session = Session::default();
        let gemara = session.workspace.open_tab("bavli/berakhot", None);
        session
            .workspace
            .split(gemara, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        session.remember(id("bavli/berakhot", 4));
        session.nikud = false;
        session.save(&path).expect("saves");

        assert_eq!(Session::load(&path), session);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_session_file_that_will_not_parse_costs_a_layout_and_not_the_app() {
        let dir = std::env::temp_dir().join("girsa-app-session-broken");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("session.json");
        std::fs::write(&path, "{ this is not json").expect("writes");

        let session = Session::load(&path);
        assert_eq!(session, Session::default());
        assert!(session.nikud, "the default is nikud on, as printed");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
