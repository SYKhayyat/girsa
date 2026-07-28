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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    /// Side by side. **In an RTL window the first child is the right one** —
    /// the Gemara opens on the right and the commentary goes to its left,
    /// which is where a person looking at a daf expects it.
    Vertical,
    /// One above the other.
    Horizontal,
}

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
                    *r = ratio.min(1000);
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
}

impl Workspace {
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
        id
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
