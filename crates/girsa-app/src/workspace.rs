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

impl Axis {
    /// The other one.
    ///
    /// > *"Tabs should be splittable in any way and movable."*
    ///
    /// The tree has carried both axes since it was written and every caller in
    /// the window passed [`Axis::Vertical`], so a reader could build any shape
    /// of split they liked as long as every divider in it was upright. This is
    /// the whole of *any way*: the axis is a property of a split that was
    /// decided once, at the moment of opening, by a caller with no opinion —
    /// and it is the reader's to change afterwards.
    #[must_use]
    pub fn turned(self) -> Self {
        match self {
            Self::Vertical => Self::Horizontal,
            Self::Horizontal => Self::Vertical,
        }
    }
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

    /// How many splits this tree has — how many dividers `layout.ts` draws.
    #[must_use]
    pub fn splits(&self) -> usize {
        match self {
            Self::Leaf { .. } => 0,
            Self::Split { first, second, .. } => 1 + first.splits() + second.splits(),
        }
    }

    /// Do something to one split, named by **which divider it is**.
    ///
    /// # Why a divider is not addressed by a pane
    ///
    /// It used to be. [`Self::set_ratio`] took a `PaneId` and looked for the
    /// split one of whose children *is* that leaf, and `layout.ts` handed it
    /// `firstPaneOf(layout.first)` — the leftmost leaf of the first child,
    /// which for a nested first child is not a child of this split at all. So a
    /// drag on the divider of
    ///
    /// ```text
    /// Split { Split { Gemara | Rashi } | Tosafot }
    /// ```
    ///
    /// resized the **inner** split: the pointer moved one line and a different
    /// line moved. Quietly, because there is always some split that matches and
    /// the wrong one is still a legal answer.
    ///
    /// The order is the order a pre-order walk meets them, which is the order
    /// `layout.ts` builds the dividers in, and the window sends back the number
    /// it drew. A tree that has changed under the click addresses a divider
    /// that is not the one clicked — the same exposure `close_tab(index)`
    /// carries, and a whole layout is redrawn on every answer.
    fn at_split<T>(
        &mut self,
        which: usize,
        next: &mut usize,
        act: &mut impl FnMut(&mut Self) -> T,
    ) -> Option<T> {
        if matches!(self, Self::Leaf { .. }) {
            return None;
        }
        let mine = *next;
        *next += 1;
        if mine == which {
            return Some(act(self));
        }
        let Self::Split { first, second, .. } = self else {
            return None;
        };
        match first.at_split(which, next, act) {
            Some(done) => Some(done),
            None => second.at_split(which, next, act),
        }
    }

    /// Turn one split, and answer the axis it now has.
    fn turn(&mut self, which: usize) -> Option<Axis> {
        self.at_split(which, &mut 0, &mut |split| {
            let Self::Split { axis, .. } = split else {
                unreachable!("at_split only ever hands over a split");
            };
            *axis = axis.turned();
            *axis
        })
    }

    /// Swap the two halves of one split.
    ///
    /// **And invert the ratio**, because `ratio` is the *first* child's share.
    /// Swapping without it moves the panes and leaves the widths where they
    /// were, so a Gemara at 70% and a Rashi at 30% become a Gemara at 30% —
    /// which is a resize the reader did not ask for wearing the clothes of a
    /// move.
    fn swap(&mut self, which: usize) -> bool {
        self.at_split(which, &mut 0, &mut |split| {
            let Self::Split {
                ratio,
                first,
                second,
                ..
            } = split
            else {
                unreachable!("at_split only ever hands over a split");
            };
            std::mem::swap(first, second);
            *ratio = 1000_u16.saturating_sub(*ratio);
        })
        .is_some()
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

    /// Where one divider sits, as the first child's share. See [`Self::at_split`]
    /// for why this is *which divider* and not *which pane*.
    fn set_ratio(&mut self, which: usize, ratio: u16) -> bool {
        self.at_split(which, &mut 0, &mut |split| {
            let Self::Split { ratio: r, .. } = split else {
                unreachable!("at_split only ever hands over a split");
            };
            *r = ratio.clamp(SMALLEST_SHARE, LARGEST_SHARE);
        })
        .is_some()
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

    /// *Open this sefer **again*** — a second view of it, whatever is open.
    ///
    /// # The gesture that could not be made
    ///
    /// > *"There is no way to open one sefer in two tabs via search, at
    /// > least."*
    ///
    /// [`Workspace::open`]'s ruling above is right and is not in question: a
    /// reader who asks for Berakhos while Berakhos is open in front of them
    /// means *go there*, and two identically-named tabs with no way to tell
    /// them apart is what the old behaviour gave them. But it left **no**
    /// gesture for the other thing — two places in one masechta, side by side —
    /// except splitting, and splitting puts the second view inside the first
    /// one's tab, which is not what *two tabs* means.
    ///
    /// So this is the other verb, reached from the gestures that carry the
    /// intent: the `＋` on the tab strip, which is named after making a tab, and
    /// Shift on a search result. Nothing calls it by accident, because nothing
    /// calls it except a control that says so.
    pub fn open_again(&mut self, slug: &str, at: Option<SegmentId>) -> PaneId {
        self.open_tab(slug, at)
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

    /// Move a pane into another tab (A12).
    ///
    /// > *"make me be able to move from tab into another tab."*
    ///
    /// A tab in Girsa is an arrangement of panes, and until now a pane was born
    /// into one and could only be closed out of it. So a reader who opened the
    /// Shulchan Arukh in its own tab and then wanted it beside the Tur had to
    /// close it and open it again — losing the place he was at in it, which is
    /// the one thing this application promises to keep.
    ///
    /// `into` is which tab, by index; `None` is a tab of its own. Answers
    /// whether anything moved, so a caller can tell *there was nowhere to go*
    /// from *it went*.
    ///
    /// # What travels with it
    ///
    /// The sefer, and **the place the reader was at in it** — the pane's own
    /// `at`, which is what a pane is for. What does not travel is `follows`:
    /// following is an arrangement between two panes standing beside each other,
    /// and a pane that has left the tab is not beside anything it used to
    /// follow. The panes it *led* stop following it for the same reason, exactly
    /// as they do when it closes.
    ///
    /// # Where it lands
    ///
    /// Split off the target tab's focused pane, which is where the reader is
    /// looking in that tab — the same landing `split` gives every other way of
    /// putting a sefer beside another, so a moved pane and an opened one arrive
    /// in the same shape.
    pub fn move_pane(&mut self, pane: PaneId, into: Option<usize>) -> bool {
        let Some(from) = self.tabs.iter().position(|t| t.pane(pane).is_some()) else {
            return false;
        };
        // Into the tab it is already in is not a move. Nor is moving the only
        // pane of a tab into a tab of its own, which would close one tab and
        // open an identical one.
        if into == Some(from) {
            return false;
        }
        if into.is_none() && self.tabs.get(from).is_some_and(|t| t.panes.len() == 1) {
            return false;
        }
        let Some(target) = into.and_then(|i| self.tabs.get(i)).map(|t| t.focused) else {
            let Some(carried) = self.take(pane, from) else {
                return false;
            };
            let id = self.open_tab(carried.slug.clone(), carried.at.clone());
            self.touched(&carried.slug);
            let _ = id;
            return true;
        };
        let Some(carried) = self.take(pane, from) else {
            return false;
        };
        // Split, then put the reader's place back on the new pane: `split`
        // opens at the head of the sefer because that is right for a sefer
        // being opened, and this one is not being opened.
        let Some(landed) = self.split(target, Axis::Vertical, carried.slug.clone(), false) else {
            // The layout refused. The pane is already out of its old tab, so
            // putting it somewhere is better than losing it — and a tab of its
            // own is where it came from being nowhere.
            self.open_tab(carried.slug.clone(), carried.at.clone());
            return true;
        };
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.pane(landed).is_some()) {
            if let Some(p) = tab.pane_mut(landed) {
                p.at = carried.at.clone();
            }
        }
        self.active = self
            .tabs
            .iter()
            .position(|t| t.pane(landed).is_some())
            .unwrap_or(self.active);
        self.touched(&carried.slug);
        true
    }

    /// Lift a pane out of its tab, closing the tab if it was the last one.
    ///
    /// The removal half of [`Self::move_pane`], and it is [`Self::close`]'s
    /// behaviour with the pane handed back rather than dropped — which is why
    /// it is the same call: two ways to take a pane out of a layout would be two
    /// ways to leave a layout naming a pane that is not there.
    fn take(&mut self, pane: PaneId, from: usize) -> Option<Pane> {
        let carried = self.tabs.get(from)?.pane(pane)?.clone();
        self.close(pane);
        Some(carried)
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

    /// Where one divider of the tab being read sits.
    ///
    /// **The active tab**, because a divider is a thing on the screen and the
    /// only dividers on the screen belong to the tab in front of the reader.
    /// The same goes for the two below it.
    pub fn set_ratio(&mut self, split: usize, ratio: u16) {
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.layout.set_ratio(split, ratio);
        }
    }

    /// Turn one split — side by side becomes one above the other, and back.
    /// Answers the axis it now has, or `None` where there is no such divider.
    pub fn turn_split(&mut self, split: usize) -> Option<Axis> {
        self.tabs.get_mut(self.active)?.layout.turn(split)
    }

    /// Swap the two halves of one split.
    ///
    /// The *movable* half of the finding, within a tab. [`Self::move_pane`]
    /// already moves a pane **between** tabs; inside one there was no way to
    /// put the Rashi on the right and the Gemara on the left short of closing
    /// both and opening them the other way round.
    pub fn swap_split(&mut self, split: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return false;
        };
        let swapped = tab.layout.swap(split);
        if swapped {
            tab.layout.sane();
        }
        swapped
    }

    /// Move a tab along the strip.
    ///
    /// `to` is where it should end up **after** it has been taken out, which is
    /// what a drop target on a strip means: dropping the third tab onto the
    /// first makes it the first. The reader keeps looking at the tab they were
    /// looking at, whichever one that is — the strip reordering under a person
    /// and taking them somewhere else is two surprises for one gesture.
    pub fn move_tab(&mut self, from: usize, to: usize) -> bool {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return false;
        }
        // By the pane the reader is in, not by index: an index is exactly the
        // thing this call changes.
        let watching = self.tabs.get(self.active).map(|t| t.focused);
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        if let Some(watching) = watching {
            if let Some(now) = self.tabs.iter().position(|t| t.focused == watching) {
                self.active = now;
            }
        }
        true
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
        w.set_ratio(0, 620);

        let text = serde_json::to_string(&w).expect("writes");
        let back: Workspace = serde_json::from_str(&text).expect("reads");
        assert_eq!(back, w);
    }
    #[test]
    fn a_pane_moves_into_another_tab_and_keeps_its_place() {
        // A12. *"make me be able to move from tab into another tab."*
        //
        // A pane was born into a tab and could only be closed out of it, so a
        // reader who opened the Shulchan Arukh in its own tab and then wanted it
        // beside the Tur had to close it and open it again — losing the place he
        // was at in it, which is the one thing this application promises to keep.
        let mut space = Workspace::default();
        let tur = space.open_tab("tur", None);
        let arukh = space.open_tab("shulchan-arukh/yoreh-deah", Some(id(7)));
        assert_eq!(space.tabs.len(), 2);

        assert!(space.move_pane(arukh, Some(0)), "into the first tab");
        assert_eq!(space.tabs.len(), 1, "and the tab it left is gone");
        let tab = space.tabs.first().expect("a tab");
        assert_eq!(
            tab.panes.len(),
            2,
            "the Tur and the Shulchan Arukh, side by side"
        );
        let landed = tab
            .panes
            .iter()
            .find(|p| p.slug == "shulchan-arukh/yoreh-deah")
            .expect("the moved pane");
        assert_eq!(
            landed.at.as_ref(),
            Some(&id(7)),
            "the place the reader was at travels with it"
        );
        assert_ne!(landed.id, arukh, "and it is a pane of the tab it landed in");
        assert!(tab.pane(tur).is_some(), "the Tur is still there");

        // Out again, into a tab of its own.
        assert!(space.move_pane(landed.id, None));
        assert_eq!(space.tabs.len(), 2);

        // The two refusals, and both are things a reader will try. Moving a
        // pane into the tab it is already in is not a move; nor is moving the
        // only pane of a tab into a tab of its own, which would close one tab
        // and open an identical one.
        let here = space.tabs[space.active].focused;
        assert!(!space.move_pane(here, Some(space.active)));
        assert!(!space.move_pane(here, None));
        assert_eq!(space.tabs.len(), 2, "and nothing happened");
    }

    #[test]
    fn a_moved_pane_stops_following_and_stops_being_followed() {
        // Following is an arrangement between two panes standing beside each
        // other. A pane that has left the tab is not beside anything, and a
        // `follows` pointing into another tab is a pane that scrolls when a
        // sefer the reader cannot see moves.
        let mut space = Workspace::default();
        let gemara = space.open_tab("bavli/berakhot", None);
        let rashi = space
            .split(gemara, Axis::Vertical, "rashi-on-berakhot", true)
            .expect("a split");
        space.open_tab("tur", None);

        assert!(
            space.move_pane(rashi, Some(1)),
            "the Rashi into the other tab"
        );
        let moved = space.tabs[1]
            .panes
            .iter()
            .find(|p| p.slug == "rashi-on-berakhot")
            .expect("the Rashi");
        assert_eq!(moved.follows, None, "it follows nothing in its new tab");

        // And the other direction: the leader loses its follower rather than
        // keeping a `follows` from a pane in another tab.
        let mut space = Workspace::default();
        let gemara = space.open_tab("bavli/berakhot", None);
        let rashi = space
            .split(gemara, Axis::Vertical, "rashi-on-berakhot", true)
            .expect("a split");
        space.open_tab("tur", None);
        assert!(space.move_pane(gemara, Some(1)));
        assert_eq!(
            space.tabs[0].pane(rashi).and_then(|p| p.follows),
            None,
            "the Rashi is not following a pane that has left"
        );
    }

    /// > *"Tabs should be splittable in any way and movable."*
    ///
    /// The tree held both axes and nothing could reach the second one.
    #[test]
    fn a_split_can_be_turned_and_turned_back() {
        let mut space = Workspace::default();
        let gemara = space.open_tab("bavli/berakhot", None);
        let rashi = space
            .split(gemara, Axis::Vertical, "rashi-on-berakhot", true)
            .expect("a split");

        assert_eq!(space.turn_split(0), Some(Axis::Horizontal));
        assert!(
            matches!(
                space.tabs[0].layout,
                Layout::Split {
                    axis: Axis::Horizontal,
                    ..
                }
            ),
            "the split is stacked"
        );
        assert_eq!(space.turn_split(0), Some(Axis::Vertical), "and back");
        assert_eq!(
            space.tabs[0].layout.panes(),
            vec![gemara, rashi],
            "turning moves nothing"
        );
    }

    /// A pane alone in its tab stands in no split, and there is nothing to turn.
    #[test]
    fn one_pane_has_no_split_to_turn() {
        let mut space = Workspace::default();
        space.open_tab("bavli/berakhot", None);
        assert_eq!(space.tabs[0].layout.splits(), 0, "no dividers to name");
        assert_eq!(space.turn_split(0), None);
        assert!(!space.swap_split(0));
    }

    /// The divider a control sits on is the reader's **own** divider, not the
    /// outermost one — three panes, and turning the innermost split leaves the
    /// one it hangs off alone.
    #[test]
    fn turning_a_split_turns_the_one_the_pane_stands_in() {
        let mut space = Workspace::default();
        let gemara = space.open_tab("bavli/berakhot", None);
        let rashi = space
            .split(gemara, Axis::Vertical, "rashi-on-berakhot", true)
            .expect("a split");
        space
            .split(rashi, Axis::Vertical, "tosafot-on-berakhot", true)
            .expect("a second split");
        assert_eq!(space.tabs[0].layout.splits(), 2, "two dividers");

        // Divider 1, not divider 0: pre-order meets the outer split first.
        assert_eq!(space.turn_split(1), Some(Axis::Horizontal));
        let Layout::Split { axis, second, .. } = &space.tabs[0].layout else {
            panic!("the outer split");
        };
        assert_eq!(*axis, Axis::Vertical, "the outer one did not move");
        assert!(
            matches!(
                **second,
                Layout::Split {
                    axis: Axis::Horizontal,
                    ..
                }
            ),
            "the inner one did"
        );
    }

    /// Swapping moves the panes **and** the widths with them.
    #[test]
    fn swapping_the_halves_carries_the_widths_across() {
        let mut space = Workspace::default();
        let gemara = space.open_tab("bavli/berakhot", None);
        let rashi = space
            .split(gemara, Axis::Vertical, "rashi-on-berakhot", true)
            .expect("a split");
        space.set_ratio(0, 700);

        assert!(space.swap_split(0));
        assert_eq!(
            space.tabs[0].layout.panes(),
            vec![rashi, gemara],
            "the Rashi is first now"
        );
        let Layout::Split { ratio, .. } = &space.tabs[0].layout else {
            panic!("a split");
        };
        assert_eq!(
            *ratio, 300,
            "the Rashi keeps the 30% it had — the ratio is the first child's share"
        );
    }

    /// **The divider that was dragged is the divider that moves.**
    ///
    /// `set_ratio` took a `PaneId`, matched the split one of whose children *is*
    /// that leaf, and `layout.ts` handed it `firstPaneOf(layout.first)` — the
    /// leftmost leaf of the first child. On a nested first child that leaf is a
    /// grandchild, the outer split does not match it, and the recursion found
    /// the **inner** split instead. So a drag on the outer divider resized the
    /// inner one: the pointer moved one line and a different line moved.
    #[test]
    fn dragging_a_divider_moves_that_divider_and_not_the_one_inside_it() {
        let mut space = Workspace::default();
        let gemara = space.open_tab("bavli/berakhot", None);
        space
            .split(gemara, Axis::Vertical, "rashi-on-berakhot", true)
            .expect("a split");
        // Split the Gemara again, so the outer split's first child is a split
        // and its leftmost leaf is a grandchild rather than a child.
        space
            .split(gemara, Axis::Horizontal, "tosafot-on-berakhot", true)
            .expect("a second split");
        assert_eq!(space.tabs[0].layout.splits(), 2);

        space.set_ratio(0, 700);

        let Layout::Split { ratio, first, .. } = &space.tabs[0].layout else {
            panic!("the outer split");
        };
        assert_eq!(*ratio, 700, "the divider that was dragged");
        let Layout::Split { ratio: inner, .. } = &**first else {
            panic!("the inner split");
        };
        assert_eq!(*inner, 500, "and not the one inside it");
    }

    /// The strip reorders and the reader stays where they were looking.
    #[test]
    fn a_tab_moves_along_the_strip_without_taking_the_reader_with_it() {
        let mut space = Workspace::default();
        space.open_tab("bavli/berakhot", None);
        space.open_tab("bavli/shabbat", None);
        let third = space.open_tab("tur", None);
        assert_eq!(space.active, 2);

        assert!(space.move_tab(0, 2), "the first tab to the end");
        assert_eq!(
            space
                .tabs
                .iter()
                .map(|t| t.panes[0].slug.as_str())
                .collect::<Vec<_>>(),
            vec!["bavli/shabbat", "tur", "bavli/berakhot"]
        );
        assert_eq!(
            space.active_tab().and_then(|t| t.pane(third)).map(|p| p.id),
            Some(third),
            "the reader is still in the tab they were in"
        );

        assert!(!space.move_tab(1, 1), "nowhere is not a move");
        assert!(!space.move_tab(0, 9), "and neither is off the end");
    }
}
