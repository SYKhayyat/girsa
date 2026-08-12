//! Tabs, splits, and which pane follows which.
//!
//! A tab holds a **tree of splits** rather than a row of columns, because a
//! daf is not always two columns: Gemara with Rashi beside it and Tosafot under
//! the Rashi is a split inside a split, and it is the arrangement people
//! actually ask for.
//!
//! ```text
//! ┌──────────────┬──────────────┐
//! │              │   Rashi      │      Split { Vertical, 0.5,
//! │   Gemara     ├──────────────┤        Leaf(gemara),
//! │              │   Tosafot    │        Split { Horizontal, 0.5, … } }
//! └──────────────┴──────────────┘
//! ```
//!
//! Nothing here reads a sefer or knows what a segment is beyond its id. Where a
//! following pane *lands* is [`crate::beside`]'s question; this only records
//! that it follows.

use girsa_corpus::segment::SegmentId;
use serde::{Deserialize, Serialize};

/// A pane's handle. Stable for as long as the pane exists, and never reused —
/// a closed pane's id going to a new pane would silently re-point every pane
/// that followed the old one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PaneId(pub u32);

/// Which way a split divides its space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Side by side. **In an RTL window the first child is the right one** —
    /// the Gemara opens on the right and the commentary goes to its left,
    /// which is where a person looking at a daf expects it.
    Vertical,
    /// One above the other.
    Horizontal,
}

girsa_corpus::spelled!(Axis {
    Vertical => "vertical",
    Horizontal => "horizontal",
});

/// How a tab's panes divide the window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Layout {
    Leaf {
        pane: PaneId,
    },
    Split {
        axis: Axis,
        /// The first child's share, in tenths of a percent, so the layout is
        /// exactly what was saved when it is read back. A float here would
        /// make two sessions with the same layout compare unequal.
        ratio: u16,
        first: Box<Layout>,
        second: Box<Layout>,
    },
}

impl Layout {
    fn leaf(pane: PaneId) -> Self {
        Self::Leaf { pane }
    }

    /// Every pane in this layout, left to right, top to bottom.
    pub fn panes(&self) -> Vec<PaneId> {
        match self {
            Self::Leaf { pane } => vec![*pane],
            Self::Split { first, second, .. } => {
                let mut out = first.panes();
                out.extend(second.panes());
                out
            }
        }
    }

    /// Put `new` beside `at`, dividing the space `at` currently has.
    fn split(&mut self, at: PaneId, axis: Axis, new: PaneId) -> bool {
        match self {
            Self::Leaf { pane } if *pane == at => {
                *self = Self::Split {
                    axis,
                    ratio: 500,
                    first: Box::new(Self::leaf(at)),
                    second: Box::new(Self::leaf(new)),
                };
                true
            }
            Self::Leaf { .. } => false,
            Self::Split { first, second, .. } => {
                first.split(at, axis, new) || second.split(at, axis, new)
            }
        }
    }

    /// Take a pane out, collapsing the split it was half of.
    ///
    /// Returns `None` when the layout is that pane and nothing else — the
    /// caller closes the tab.
    fn without(self, pane: PaneId) -> Option<Self> {
        match self {
            Self::Leaf { pane: p } if p == pane => None,
            leaf @ Self::Leaf { .. } => Some(leaf),
            Self::Split {
                axis,
                ratio,
                first,
                second,
            } => match (first.without(pane), second.without(pane)) {
                (Some(first), Some(second)) => Some(Self::Split {
                    axis,
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            },
        }
    }

    /// Clamp every ratio in this tree — see [`SMALLEST_SHARE`].
    fn sane(&mut self) {
        if let Self::Split {
            first,
            second,
            ratio,
            ..
        } = self
        {
            *ratio = (*ratio).clamp(SMALLEST_SHARE, LARGEST_SHARE);
            first.sane();
            second.sane();
        }
    }

    fn set_ratio(&mut self, at: PaneId, ratio: u16) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split {
                first,
                second,
                ratio: r,
                ..
            } => {
                if first.panes().contains(&at) && matches!(**first, Self::Leaf { .. })
                    || second.panes().contains(&at) && matches!(**second, Self::Leaf { .. })
                {
                    *r = ratio.clamp(SMALLEST_SHARE, LARGEST_SHARE);
                    return true;
                }
                first.set_ratio(at, ratio) || second.set_ratio(at, ratio)
            }
        }
    }
}

/// One column of reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pane {
    pub id: PaneId,
    /// The sefer this pane is showing.
    pub slug: String,
    /// Where the reader is. `None` before anything has been scrolled to,
    /// which is the top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<SegmentId>,
    /// The pane this one follows, if it is following one.
    ///
    /// Following is one-way and explicit. A pane that followed whatever moved
    /// last would swap leader mid-scroll and neither column would settle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follows: Option<PaneId>,
}

/// One tab: a layout, its panes, and which one has the cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tab {
    pub layout: Layout,
    pub panes: Vec<Pane>,
    pub focused: PaneId,
}

impl Tab {
    #[must_use]
    pub fn pane(&self, id: PaneId) -> Option<&Pane> {
        self.panes.iter().find(|p| p.id == id)
    }

    fn pane_mut(&mut self, id: PaneId) -> Option<&mut Pane> {
        self.panes.iter_mut().find(|p| p.id == id)
    }

    /// The panes that follow `leader`, in the order they are laid out.
    #[must_use]
    pub fn followers_of(&self, leader: PaneId) -> Vec<PaneId> {
        self.layout
            .panes()
            .into_iter()
            .filter(|id| self.pane(*id).and_then(|p| p.follows) == Some(leader))
            .collect()
    }
}

/// Every tab that is open, and where each of them is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub tabs: Vec<Tab>,
    pub active: usize,
    /// The next pane id to hand out. Kept so ids are never reused across a
    /// session or across a restart.
    next_pane: u32,
    /// **The open set**, most recently focused first.
    ///
    /// Borrowed from the sibling application, where the same absence produced
    /// seven separate complaints
    /// (`Ksav/ksav/app/src/opendocs.ts`, and the decision of 11 August): *which
    /// documents are open* is not the same question as *which documents exist*,
    /// and a tab strip cannot answer it once a tab is an arrangement rather than
    /// a document — a tab holding a Gemara, its Rashi and its Tosafos is one
    /// entry in the strip and three seforim that are open.
    ///
    /// Girsa parts company with Ksav on one rule and it is deliberate. Ksav says
    /// a document is never open twice, because two carets and two undo stacks
    /// over one text is how a document gets eaten. A sefer is read-only, and two
    /// panes on two places in one masechta is a thing people do all day — so the
    /// same sefer may be open more than once here, and what is a **gesture**
    /// rule instead: *open a sefer* goes to it if it is open, and a second view
    /// of it is something you ask for by splitting.
    ///
    /// Most recently focused first, because that is the order a switcher wants:
    /// the keyboard route to *the sefer I was just in* is the one thing a strip
    /// cannot express.
    #[serde(default)]
    recent: Vec<String>,
}

/// The narrowest a pane may be squeezed to, in tenths of a per cent.
///
/// # Why this is here and not in `layout.ts`
///
/// It was in `layout.ts`, only — `Math.min(85, Math.max(15, share))` — while
/// this file clamped `ratio.min(1000)`. Two clamps with two different answers,
/// and the one that decides what a reader can actually do lived in the window.
/// A ratio of 0 arriving from a hand-edited session file, or from any caller
/// that is not a pointer drag, was accepted here and drew a pane no pixels wide.
pub const SMALLEST_SHARE: u16 = 150;

/// And the widest, so the *other* pane is never squeezed to nothing either.
pub const LARGEST_SHARE: u16 = 1000 - SMALLEST_SHARE;

impl Workspace {
    /// Bring every ratio back inside what a pane can be.
    ///
    /// Called on load as well as by the setter, because a clamp that only runs
    /// in a setter is a rule about one code path rather than about the value.
    pub fn sane(&mut self) {
        for tab in &mut self.tabs {
            tab.layout.sane();
        }
    }

    /// A pane already showing this sefer, if one is.
    ///
    /// The **active tab first**: a reader with Berakhos in two tabs who asks for
    /// Berakhos means the one they are looking at, not the one they opened on
    /// Tuesday.
    #[must_use]
    pub fn showing(&self, slug: &str) -> Option<PaneId> {
        let here = self
            .active_tab()
            .and_then(|tab| tab.panes.iter().find(|p| p.slug == slug));
        if let Some(pane) = here {
            return Some(pane.id);
        }
        self.tabs
            .iter()
            .flat_map(|tab| tab.panes.iter())
            .find(|p| p.slug == slug)
            .map(|p| p.id)
    }

    /// Every sefer that is open, most recently focused first.
    #[must_use]
    pub fn open_set(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for slug in &self.recent {
            if self.showing(slug).is_some() && !out.contains(slug) {
                out.push(slug.clone());
            }
        }
        // Anything open that nobody has focused since the session was read back
        // — `recent` is not persisted from before this existed, and a sefer
        // missing from the switcher is a sefer the reader cannot get back to.
        for tab in &self.tabs {
            for pane in &tab.panes {
                if !out.contains(&pane.slug) {
                    out.push(pane.slug.clone());
                }
            }
        }
        out
    }

    /// Note that this sefer is the one being read.
    fn touched(&mut self, slug: &str) {
        self.recent.retain(|s| s != slug);
        self.recent.insert(0, slug.to_string());
        // A session file is not a history. Long enough to be a switcher, short
        // enough that nobody scrolls it.
        self.recent.truncate(40);
    }

    /// Open a sefer in a new tab, and make it the tab you are looking at.
    pub fn open_tab(&mut self, slug: impl Into<String>, at: Option<SegmentId>) -> PaneId {
        let id = self.mint();
        self.tabs.push(Tab {
            layout: Layout::leaf(id),
            panes: vec![Pane {
                id,
                slug: slug.into(),
                at,
                follows: None,
            }],
            focused: id,
        });
        self.active = self.tabs.len() - 1;
        self.touched(&self.tabs[self.active].panes[0].slug.clone());
        id
    }

    /// *Open this sefer* — go to it where it already is, or open it in a tab.
    ///
    /// > *"the open sefer is confusing - it should just open a new tab."*
    ///
    /// It always opened a new tab, including for a sefer already in front of
    /// you, so asking for Berakhos twice gave two tabs called ברכות and no way
    /// to tell them apart. Going to the open one is what every application with
    /// an open set does, and a **second view** of one sefer — two places in one
    /// masechta, side by side — is still available by splitting, which is the
    /// gesture that means it.
    pub fn open(&mut self, slug: &str, at: Option<SegmentId>) -> PaneId {
        match self.showing(slug) {
            Some(pane) => {
                self.focus(pane);
                pane
            }
            None => self.open_tab(slug, at),
        }
    }

    /// Open a sefer beside an open pane, following it.
    ///
    /// Following by default is the point of the feature: nobody splits the
    /// window to put Rashi next to the Gemara and then wants to scroll it
    /// themselves. It is one field, so a reader who does want to read the
    /// Rashi on its own can turn it off.
    pub fn split(
        &mut self,
        at: PaneId,
        axis: Axis,
        slug: impl Into<String>,
        follow: bool,
    ) -> Option<PaneId> {
        let id = self.mint();
        let tab = self.tab_holding_mut(at)?;
        if !tab.layout.split(at, axis, id) {
            return None;
        }
        tab.panes.push(Pane {
            id,
            slug: slug.into(),
            at: None,
            follows: follow.then_some(at),
        });
        tab.focused = id;
        let opened = tab.panes.last().map(|p| p.slug.clone());
        if let Some(slug) = opened {
            self.touched(&slug);
        }
        Some(id)
    }

    /// Close a pane. Closes its tab if it was the last one in it.
    pub fn close(&mut self, pane: PaneId) {
        let Some(index) = self.tabs.iter().position(|t| t.pane(pane).is_some()) else {
            return;
        };
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        match std::mem::replace(&mut tab.layout, Layout::leaf(pane)).without(pane) {
            Some(layout) => {
                tab.layout = layout;
                tab.panes.retain(|p| p.id != pane);
                // A pane that was following the closed one stops following
                // rather than following a pane that no longer exists.
                for p in &mut tab.panes {
                    if p.follows == Some(pane) {
                        p.follows = None;
                    }
                }
                if tab.focused == pane {
                    tab.focused = tab.layout.panes().first().copied().unwrap_or(pane);
                }
            }
            None => {
                self.tabs.remove(index);
                self.active = self.active.min(self.tabs.len().saturating_sub(1));
            }
        }
    }

    /// Close a whole tab, without the reader having to be in it (W40).
    ///
    /// > *"needs a way to close tab without going in."*
    ///
    /// The only way to shut a tab used to be to activate it and close its panes
    /// one at a time, which meant reading every sefer in it and moving every pane
    /// that follows another — work, and a change of place, to throw something
    /// away.
    ///
    /// **Where the reader ends up** is the whole of the decision here, and it is
    /// three cases: a tab before theirs closing shifts their index down so they
    /// stay where they were; a tab after theirs changes nothing; and closing the
    /// one they are in lands them on whatever took its place, because that is what
    /// is under the cursor. An index past the end is not one of the cases — an
    /// empty strip is `active == 0` and no active tab.
    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            // The window and the model can disagree for a frame; an impatient
            // second click is not a reason to take the window down.
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active = 0;
        } else if index < self.active {
            self.active -= 1;
        } else {
            self.active = self.active.min(self.tabs.len() - 1);
        }
    }

    /// Record where a pane is, and answer with the panes that have to move
    /// because of it.
    ///
    /// The *where they land* part is [`crate::beside`]'s; this says who is
    /// affected, which is a fact about the layout and testable without a
    /// corpus.
    pub fn moved(&mut self, pane: PaneId, to: SegmentId) -> Vec<PaneId> {
        let Some(tab) = self.tab_holding_mut(pane) else {
            return Vec::new();
        };
        if let Some(p) = tab.pane_mut(pane) {
            p.at = Some(to);
        }
        tab.followers_of(pane)
    }

    pub fn focus(&mut self, pane: PaneId) {
        if let Some(tab) = self.tab_holding_mut(pane) {
            tab.focused = pane;
        }
        if let Some(i) = self.tabs.iter().position(|t| t.pane(pane).is_some()) {
            self.active = i;
        }
        if let Some(slug) = self.pane(pane).map(|p| p.slug.clone()) {
            self.touched(&slug);
        }
    }

    /// Set whether a pane follows another, or nothing.
    pub fn set_follows(&mut self, pane: PaneId, leader: Option<PaneId>) {
        // A pane cannot follow itself, and two panes cannot follow each other:
        // either would be a loop that moves the window forever.
        if leader == Some(pane) {
            return;
        }
        let Some(tab) = self.tab_holding_mut(pane) else {
            return;
        };
        if let Some(leader) = leader {
            if tab.pane(leader).and_then(|p| p.follows) == Some(pane) {
                return;
            }
            if tab.pane(leader).is_none() {
                return;
            }
        }
        if let Some(p) = tab.pane_mut(pane) {
            p.follows = leader;
        }
    }

    pub fn set_ratio(&mut self, pane: PaneId, ratio: u16) {
        if let Some(tab) = self.tab_holding_mut(pane) {
            tab.layout.set_ratio(pane, ratio);
        }
    }

    #[must_use]
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    #[must_use]
    pub fn pane(&self, id: PaneId) -> Option<&Pane> {
        self.tabs.iter().find_map(|t| t.pane(id))
    }

    fn tab_holding_mut(&mut self, pane: PaneId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.pane(pane).is_some())
    }

    fn mint(&mut self) -> PaneId {
        self.next_pane += 1;
        PaneId(self.next_pane)
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn id(n: u32) -> SegmentId {
        format!("girsa:bavli/berakhot/2a:{n}#{n}")
            .parse()
            .expect("a segment id")
    }

    // ── W40: closing a tab without going into it ─────────────────────────────
    //
    // > *"needs a way to close tab without going in."*
    //
    // There was no way to close a tab at all. `close` takes a pane, so shutting a
    // tab meant activating it — reading it, redrawing it, moving the panes that
    // follow — and then closing its panes one at a time.

    #[test]
    fn a_tab_closes_without_being_opened_first() {
        let mut w = Workspace::default();
        w.open_tab("bavli/berakhot", None);
        w.open_tab("bavli/shabbat", None);
        w.open_tab("genesis", None);
        assert_eq!(w.active, 2);

        w.close_tab(0);
        assert_eq!(w.tabs.len(), 2);
        assert_eq!(w.tabs[0].panes[0].slug, "bavli/shabbat");
        // And the reader is still looking at what they were looking at, which is
        // the whole point of *without going in*.
        assert_eq!(w.active, 1);
        assert_eq!(
            w.active_tab().expect("a tab").panes[0].slug,
            "genesis",
            "closing another tab moved the reader"
        );
    }

    #[test]
    fn closing_a_tab_after_the_active_one_leaves_the_reader_alone() {
        let mut w = Workspace::default();
        w.open_tab("bavli/berakhot", None);
        w.open_tab("bavli/shabbat", None);
        w.open_tab("genesis", None);
        w.active = 0;

        w.close_tab(2);
        assert_eq!(w.active, 0);
        assert_eq!(
            w.active_tab().expect("a tab").panes[0].slug,
            "bavli/berakhot"
        );
    }

    #[test]
    fn closing_the_tab_you_are_in_lands_on_a_neighbour() {
        let mut w = Workspace::default();
        w.open_tab("bavli/berakhot", None);
        w.open_tab("bavli/shabbat", None);
        w.open_tab("genesis", None);
        w.active = 1;

        w.close_tab(1);
        assert_eq!(w.tabs.len(), 2);
        // The one that took its place, not the far end of the strip: a reader
        // closing what they were reading looks at what is now under the cursor.
        assert_eq!(w.active, 1);
        assert_eq!(w.active_tab().expect("a tab").panes[0].slug, "genesis");
    }

    #[test]
    fn closing_the_last_tab_leaves_nothing_open_rather_than_an_index_to_nowhere() {
        let mut w = Workspace::default();
        w.open_tab("bavli/berakhot", None);
        w.close_tab(0);
        assert!(w.tabs.is_empty());
        assert_eq!(w.active, 0, "and not an index past the end");
        assert!(w.active_tab().is_none());
    }

    #[test]
    fn closing_a_tab_that_is_not_there_does_nothing() {
        // The window and the model can disagree for one frame — a tab closed
        // twice by an impatient click. A panic there would take the reader's
        // window with it.
        let mut w = Workspace::default();
        w.open_tab("bavli/berakhot", None);
        w.close_tab(7);
        assert_eq!(w.tabs.len(), 1);
    }

    #[test]
    fn closing_a_tab_closes_every_pane_in_it() {
        let mut w = Workspace::default();
        let gemara = w.open_tab("bavli/berakhot", None);
        w.split(gemara, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        let open = |w: &Workspace| w.tabs.iter().map(|t| t.panes.len()).sum::<usize>();
        assert_eq!(open(&w), 2);
        w.close_tab(0);
        assert_eq!(open(&w), 0, "a pane outlived its tab");
    }

    #[test]
    fn a_split_puts_the_new_pane_beside_the_one_it_came_from() {
        let mut w = Workspace::default();
        let gemara = w.open_tab("bavli/berakhot", None);
        let rashi = w
            .split(gemara, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        assert_eq!(
            w.active_tab().expect("a tab").layout.panes(),
            [gemara, rashi]
        );
        assert_eq!(w.pane(rashi).and_then(|p| p.follows), Some(gemara));
    }

    #[test]
    fn a_split_inside_a_split_is_a_daf() {
        // Gemara on the right, Rashi beside it, Tosafot under the Rashi.
        let mut w = Workspace::default();
        let gemara = w.open_tab("bavli/berakhot", None);
        let rashi = w
            .split(gemara, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        let tosafot = w
            .split(rashi, Axis::Horizontal, "bavli/tosafot-on-berakhot", true)
            .expect("splits again");

        assert_eq!(
            w.active_tab().expect("a tab").layout.panes(),
            [gemara, rashi, tosafot]
        );
        // Both commentaries follow the Gemara? No — Tosafot was split from the
        // Rashi pane, so it follows that. Following is explicit, and this is
        // the case where guessing would be wrong half the time.
        assert_eq!(w.pane(tosafot).and_then(|p| p.follows), Some(rashi));
        w.set_follows(tosafot, Some(gemara));
        assert_eq!(w.pane(tosafot).and_then(|p| p.follows), Some(gemara));
    }

    #[test]
    fn moving_a_pane_names_the_panes_that_have_to_move_with_it() {
        let mut w = Workspace::default();
        let gemara = w.open_tab("bavli/berakhot", None);
        let rashi = w
            .split(gemara, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        let tosafot = w
            .split(gemara, Axis::Horizontal, "bavli/tosafot-on-berakhot", true)
            .expect("splits");

        // In layout order, which is what the window shows: Tosafot was split
        // off the Gemara pane so it sits under it, and the Rashi column is to
        // the side of both.
        assert_eq!(w.moved(gemara, id(1)), [tosafot, rashi]);
        assert_eq!(w.pane(gemara).and_then(|p| p.at.clone()), Some(id(1)));
        // And a commentary moving on its own drags nothing.
        assert!(w.moved(rashi, id(2)).is_empty());
    }

    #[test]
    fn closing_a_pane_leaves_the_ones_that_followed_it_alone_rather_than_lost() {
        let mut w = Workspace::default();
        let gemara = w.open_tab("bavli/berakhot", None);
        let rashi = w
            .split(gemara, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        w.close(gemara);

        assert_eq!(w.tabs.len(), 1, "the tab still holds the Rashi pane");
        assert_eq!(w.active_tab().expect("a tab").layout.panes(), [rashi]);
        assert_eq!(
            w.pane(rashi).and_then(|p| p.follows),
            None,
            "it was following a pane that no longer exists"
        );
        assert_eq!(w.active_tab().expect("a tab").focused, rashi);
    }

    #[test]
    fn closing_the_last_pane_closes_the_tab() {
        let mut w = Workspace::default();
        let only = w.open_tab("bavli/berakhot", None);
        w.close(only);
        assert!(w.tabs.is_empty());
        assert_eq!(w.active, 0);
    }

    // ── The open set, borrowed from Ksav ─────────────────────────────────────

    #[test]
    fn opening_a_sefer_that_is_already_open_goes_to_it() {
        // > *"the open sefer is confusing - it should just open a new tab."*
        //
        // It always opened a new one, so asking for Berakhos twice gave two tabs
        // called ברכות and nothing to tell them apart.
        let mut w = Workspace::default();
        let first = w.open("bavli/berakhot", None);
        w.open_tab("bavli/shabbat", None);
        assert_eq!(w.tabs.len(), 2);

        let again = w.open("bavli/berakhot", None);
        assert_eq!(again, first, "the pane that was already showing it");
        assert_eq!(w.tabs.len(), 2, "and no second tab for one sefer");
        assert_eq!(w.active, 0, "and the reader is looking at it");
    }

    #[test]
    fn a_second_view_of_one_sefer_is_still_something_you_can_ask_for() {
        // Where Girsa parts company with Ksav, and on purpose: a sefer is
        // read-only, and two places in one masechta side by side is a thing
        // people do all day. The **gesture** decides — *open* goes to it, a
        // split makes another view.
        let mut w = Workspace::default();
        let gemara = w.open("bavli/berakhot", None);
        let beside = w
            .split(gemara, Axis::Vertical, "bavli/berakhot", false)
            .expect("splits");
        assert_ne!(beside, gemara);
        assert_eq!(w.active_tab().expect("a tab").panes.len(), 2);
    }

    #[test]
    fn the_open_set_is_most_recently_read_first() {
        // What a switcher wants, and the one thing a strip of tabs cannot say
        // once a tab is an arrangement: a tab holding a Gemara, its Rashi and
        // its Tosafos is one entry in the strip and three seforim that are open.
        let mut w = Workspace::default();
        let gemara = w.open_tab("bavli/berakhot", None);
        w.split(gemara, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        w.open_tab("genesis", None);
        assert_eq!(
            w.open_set(),
            ["genesis", "bavli/rashi-on-berakhot", "bavli/berakhot"]
        );

        // Reading the Gemara again puts it back on top.
        w.focus(gemara);
        assert_eq!(
            w.open_set(),
            ["bavli/berakhot", "genesis", "bavli/rashi-on-berakhot"]
        );
    }

    #[test]
    fn a_sefer_that_is_closed_leaves_the_open_set() {
        let mut w = Workspace::default();
        w.open_tab("bavli/berakhot", None);
        w.open_tab("genesis", None);
        w.close_tab(1);
        assert_eq!(w.open_set(), ["bavli/berakhot"]);
    }

    #[test]
    fn a_session_written_before_the_open_set_still_lists_what_is_open() {
        // `recent` is `serde(default)`, so a session from before this existed
        // reads back empty — and a sefer missing from the switcher is a sefer
        // the reader cannot get back to.
        let mut w = Workspace::default();
        w.open_tab("bavli/berakhot", None);
        w.open_tab("genesis", None);
        let text = serde_json::to_string(&w).expect("writes");
        let older: Workspace =
            serde_json::from_str(&text.replace("\"recent\"", "\"was_recent\"")).expect("reads");
        assert_eq!(older.open_set().len(), 2);
    }

    #[test]
    fn a_pane_id_is_never_handed_out_twice() {
        // A reused id would silently re-point every pane that followed the old
        // one — at a sefer that is not the one it was reading.
        let mut w = Workspace::default();
        let first = w.open_tab("a", None);
        w.close(first);
        let second = w.open_tab("b", None);
        assert_ne!(first, second);
    }

    #[test]
    fn two_panes_cannot_be_made_to_follow_each_other() {
        let mut w = Workspace::default();
        let a = w.open_tab("bavli/berakhot", None);
        let b = w
            .split(a, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        // b follows a already. Making a follow b would be a loop and the
        // window would never settle.
        w.set_follows(a, Some(b));
        assert_eq!(w.pane(a).and_then(|p| p.follows), None);
        w.set_follows(a, Some(a));
        assert_eq!(w.pane(a).and_then(|p| p.follows), None);
    }

    #[test]
    fn a_layout_survives_being_written_down_and_read_back() {
        // The workspace is restored on the next launch, so this is the
        // difference between reopening where you were and reopening a blank
        // window.
        let mut w = Workspace::default();
        let gemara = w.open_tab("bavli/berakhot", Some(id(3)));
        w.split(gemara, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        w.set_ratio(gemara, 620);

        let text = serde_json::to_string(&w).expect("writes");
        let back: Workspace = serde_json::from_str(&text).expect("reads");
        assert_eq!(back, w);
    }
}
