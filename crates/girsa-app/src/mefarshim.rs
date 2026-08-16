//! Which mefarshim speak about which line, and which of them you asked for.
//!
//! # The interaction this is for
//!
//! Girsa had one way to open a commentary: pick a sefer and it fills a column of
//! the split, kept in step with the text beside it. That is [`crate::beside`] and
//! it stays — a Gemara with Rashi down the side is the right shape for a Gemara
//! with Rashi down the side.
//!
//! It answers the wrong question for the other half of learning, which is: *I
//! have six mefarshim I care about; which of them said something about **this
//! line**, and what?* Otzaria answers that by letting you tick the commentators
//! you want, marking the lines they touch, and opening those comments when you
//! tap a marked line. That is the model this module serves.
//!
//! # No sidecar
//!
//! Otzaria needs a bitmap per commentator — one bit per line — because it is
//! reading plain text files on a phone and has nowhere else to put the answer.
//! Girsa already wrote the answer down: `corpus/links/<slug>/inbound.jsonl` is
//! one file per sefer holding every edge that lands in it. Berakhot's is 3.4 MB
//! and 21,065 rows, and turning it into *which works comment on each segment*
//! takes 0.07 s — 30 commentators over 2,749 of its segments. Once per sefer
//! opened, held in memory, is enough for the markers and the click both.
//!
//! # The marker marks what you chose
//!
//! Marking every line that has *any* commentary marks almost every line — 2,749
//! of Berakhot's segments carry one, most of them from five or more works — and a
//! marker on everything is not a marker. So [`Marks::on`] takes the chosen set,
//! and a reader who has chosen nothing sees nothing.

use std::collections::{BTreeMap, BTreeSet};

use girsa_corpus::segment::SegmentId;
use girsa_corpus::taxonomy::{rank_of, Stands};
use girsa_corpus::work::Work;
use girsa_link::{inbound, EdgeType};
use serde::Serialize;

use crate::arrangement::Arrangement;
use crate::shelf::{Companion, Related};
use crate::taxonomy::{shelf_key_of, shelf_title, Branch, Shipped};

/// Which works comment on which segment of one sefer.
#[derive(Debug, Default, Clone)]
pub struct Marks {
    by_segment: BTreeMap<String, Vec<Spoken>>,
    /// Every work that comments anywhere in this sefer, so the chooser can be
    /// drawn from the same read.
    works: BTreeSet<String>,
    /// Seforim that keep this one's order without commenting on it: the Shulchan
    /// Arukh under the Tur, the Arukh HaShulchan under the Shulchan Arukh.
    ///
    /// Its own set, and drawn as its own group, because the two claims are
    /// different and the reader is the one who knows which they wanted. Folding
    /// these into `works` would call a code a commentary; dropping them, as this
    /// module did until now, threw away a relationship without saying so.
    alongside: BTreeSet<String>,
    /// Works with commentary edges landing here that are **not** commentaries on
    /// this sefer.
    ///
    /// Kept, and not silently dropped, because it is a number worth being able
    /// to ask about: on the Tur it is thirty-six, and thirty-six was the size of
    /// the bug. A filter nobody can count is a filter nobody can check.
    refused: BTreeSet<String>,
}

/// One thing one mefaresh says about one line, and where to read it.
///
/// The address is in the **commentary**, not in the sefer: it is what a pane
/// opens and what a citation cites. A marker only needs the work's name; a click
/// needs this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Spoken {
    pub work: String,
    pub at: SegmentId,
    /// The far end, where the comment runs over more than one segment of the
    /// commentary. Kept because a Tosafot that spans four segments is one
    /// Tosafot, and reading only the first would cut it off mid-sentence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<SegmentId>,
}

/// One line's worth of answer: the chosen mefarshim that speak here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Here {
    /// The chosen works that comment on this segment, in the order they were
    /// chosen — a reader's own order is the one they can predict.
    pub works: Vec<String>,
    /// Whether anything at all comments here, chosen or not.
    ///
    /// Separate from `works` being empty, because *nobody wrote about this line*
    /// and *none of the six you picked wrote about this line* are different
    /// things to say, and saying the first when the second is true is the kind
    /// of quiet lie the rest of this codebase spent a week removing.
    pub any: bool,
}

impl Marks {
    /// Read one sefer's incoming edges and index the commentary by segment.
    ///
    /// # Errors
    ///
    /// If the inbound cache exists and cannot be read. A sefer with no inbound
    /// file has nothing commenting on it, which is not an error — but see
    /// [`inbound::built`]: *no cache at all* is a different statement, and the
    /// caller is the one that has to make it.
    pub fn of(shelf: &crate::shelf::Shelf, slug: &str) -> Result<Self, std::io::Error> {
        let mut marks = Self::default();
        let Some(base) = shelf.work(slug) else {
            // Not on this shelf. Nothing to draw a marker on, so nothing to say.
            return Ok(marks);
        };

        // One read, one pass to gather, one pass to place. The gather is what
        // lets a work be judged by **how much it says here** — which is the only
        // evidence there is for a commentary that declares no base and names no
        // section, and which costs nothing because these edges are already in
        // hand. See `Stands::AskTheEdges`.
        let edges: Vec<_> = inbound::read_back(shelf.root(), slug)?
            .into_iter()
            .filter(|edge| edge.edge_type == EdgeType::CommentsOn)
            // The far end is the commentary; this end is us. Which is only true
            // because `girsa_link::orient` made it true — before that, half of
            // these edges pointed the other way and this whole module would have
            // indexed the sefer as a commentary on its own mefarshim.
            .filter(|edge| edge.from.from.work() != slug)
            .collect();

        let mut said_here: BTreeMap<&str, usize> = BTreeMap::new();
        for edge in &edges {
            *said_here.entry(edge.from.from.work()).or_default() += 1;
        }

        // Being at the far end of a `comments-on` edge does not make a sefer a
        // mefaresh on this one. Tur has commentary edges landing in it from forty
        // works and five commentaries; the first version of this module offered
        // all forty, so Rashi on Berakhot was a mefaresh on the Tur and Shabbos
        // one on the Shulchan Arukh. That is inferring a relationship between two
        // seforim from the existence of an edge, which is BUILDER.md rule 6.
        let mut standing: BTreeMap<&str, Stands> = BTreeMap::new();
        for (work, count) in &said_here {
            // `taxonomy::settled`: the shelf's answer, with the one case it
            // refuses to guess at resolved by how much this work says here.
            // Bartenura on Torah puts 330 comments into Bereshis and none
            // anywhere in Kesuvim, and a work with a handful is passing through.
            //
            // The threshold was private to this module while `Shelf::companions`
            // and `Beside::between` answered the same question without asking
            // `stands` at all.
            let keeping = shelf.keeping(work, slug);
            let verdict = shelf.work(work).map_or(Stands::Apart, |w| {
                girsa_corpus::taxonomy::settled(w, base, *count, keeping)
            });
            standing.insert(work, verdict);
        }

        for edge in &edges {
            let from = edge.from.from.work();
            match standing.get(from).copied().unwrap_or(Stands::Apart) {
                Stands::On => marks.works.insert(from.to_string()),
                // Listed apart from the mefarshim — but indexed exactly like
                // them, because the reader who ticks the Arukh HaShulchan wants
                // the marker and the click, not a name in a list. Which group it
                // is drawn in is the only difference, and that is the point.
                Stands::Alongside => marks.alongside.insert(from.to_string()),
                Stands::Apart | Stands::AskTheEdges | Stands::AskTheAddresses => {
                    marks.refused.insert(from.to_string());
                    continue;
                }
            };
            let spoken = Spoken {
                work: from.to_string(),
                at: edge.from.from.clone(),
                to: edge.from.to.clone(),
            };
            // A span covers a run of segments and every one of them is spoken
            // about, not just the first.
            let here = edge.to.from.to_string();
            let far = edge.to.to.as_ref().map(ToString::to_string);
            marks.add(here.clone(), &spoken);
            if let Some(far) = far {
                // Unless both ends of the span landed in the same segment, in
                // which case this is one comment and not two.
                if far != here {
                    marks.add(far, &spoken);
                }
            }
        }
        Ok(marks)
    }

    fn add(&mut self, segment: String, spoken: &Spoken) {
        let said = self.by_segment.entry(segment).or_default();
        if !said.contains(spoken) {
            said.push(spoken.clone());
        }
    }

    /// Every work that comments anywhere in this sefer — the list to tick.
    #[must_use]
    pub fn commentators(&self) -> Vec<String> {
        self.works.iter().cloned().collect()
    }

    /// How many segments carry commentary from anybody. For a sentence, not a
    /// decision.
    #[must_use]
    pub fn segments_touched(&self) -> usize {
        self.by_segment.len()
    }

    /// The seforim that keep this one's order without commenting on it — its own
    /// group in the window, never mixed into [`Self::commentators`].
    #[must_use]
    pub fn alongside(&self) -> Vec<String> {
        self.alongside.iter().cloned().collect()
    }

    /// The works that link here as commentary and are neither commentaries on
    /// this sefer nor running alongside it. For a test and for a diagnostic,
    /// never for the list.
    #[must_use]
    pub fn refused(&self) -> Vec<String> {
        self.refused.iter().cloned().collect()
    }

    /// What the chosen mefarshim say about one line.
    #[must_use]
    pub fn on(&self, at: &SegmentId, chosen: &[String]) -> Here {
        let here = self.by_segment.get(&at.to_string());
        Here {
            works: chosen
                .iter()
                .filter(|w| here.is_some_and(|said| said.iter().any(|s| &s.work == *w)))
                .cloned()
                .collect(),
            any: here.is_some_and(|said| !said.is_empty()),
        }
    }

    /// Where to read the chosen mefarshim on one line.
    ///
    /// Grouped by mefaresh in the order they were ticked, and within one
    /// mefaresh in the order the corpus lists them — a Rashi with four diburim
    /// on one stretch of Gemara comes back as four, because it is four.
    #[must_use]
    pub fn said(&self, at: &SegmentId, chosen: &[String]) -> Vec<Spoken> {
        let Some(here) = self.by_segment.get(&at.to_string()) else {
            return Vec::new();
        };
        chosen
            .iter()
            .flat_map(|want| here.iter().filter(move |s| &s.work == want).cloned())
            .collect()
    }

    /// The segments to put a marker on, and **how many** of the chosen speak on
    /// each.
    ///
    /// Nothing chosen marks nothing. A marker on every line is not a marker —
    /// and that is the sentence this function spent a year not living up to.
    ///
    /// # A bool over a set the reader chose is constant for the sets they choose
    ///
    /// > *"Ticking a targum marks every line. 1,533 of Bereishis' 1,533; Rashi
    /// > marks 356 of 400 drawn lines of Shabbos."*
    ///
    /// Taking the chosen set was the right idea and it does not go far enough,
    /// because of *which* mefarshim a reader ticks first. A targum comments on
    /// every posuk by construction; Rashi on Shabbos is close behind. So the
    /// answer to *does one of yours speak here* is `true`, 1,533 times, and the
    /// marker that was careful not to mark everything marks everything on the
    /// first sefer anybody tries it on.
    ///
    /// The information was thrown away one step earlier: the reader ticks six
    /// mefarshim and the line is asked a yes-or-no question. **How many of them
    /// speak here** varies exactly where the bool does not — a posuk with
    /// Onkelos and Rashi and the Ramban is not the posuk before it with Onkelos
    /// alone — and it costs nothing, because the works are already in hand.
    ///
    /// The window decides what to draw from that (`marking` in `mefarshim.ts`):
    /// one count repeated down the whole sefer is still a marker saying nothing,
    /// and it says it once, in words, instead of 1,533 times in the margin.
    #[must_use]
    pub fn marked(&self, chosen: &[String]) -> BTreeMap<String, usize> {
        if chosen.is_empty() {
            return BTreeMap::new();
        }
        let want: BTreeSet<&str> = chosen.iter().map(String::as_str).collect();
        self.by_segment
            .iter()
            .filter_map(|(id, said)| {
                // The chosen **works**, not the comments: a mefaresh with three
                // separate comments on one line is one mefaresh speaking there,
                // and counting the comments would make a busy Rashi look like
                // three ticked seforim.
                let here: BTreeSet<&str> = said
                    .iter()
                    .map(|s| s.work.as_str())
                    .filter(|work| want.contains(work))
                    .collect();
                (!here.is_empty()).then(|| (id.clone(), here.len()))
            })
            .collect()
    }
}

/// The mefarshim of one sefer, in their folders (W44).
#[derive(Debug, Default, Clone, Serialize)]
pub struct Folders {
    /// The folders, in the order a person wants them: rishonim before acharonim,
    /// because that is the first thing anybody wants to know about a mefaresh.
    pub tree: Vec<Branch>,
    /// Slug → the key of the folder it is drawn in. Absent means *drawn loose*,
    /// above the folders, which is where a mefaresh with nobody to stand beside
    /// goes.
    pub of: BTreeMap<String, String>,
    /// How many are drawn loose. Together with the tree's counts this comes to
    /// the whole list, which is the check that grouping lost nobody.
    pub loose: usize,
}

/// Group a sefer's mefarshim the way the shelf files them.
///
/// > *"it would be nice if meforshim remained in their folders"*
///
/// **One rule: a folder needs two seforim.** Everything else falls out of it.
///
/// - The mefarshim on a masechta are all under `Talmud/Bavli`, so that prefix
///   says nothing and is stripped. What is left — rishonim, acharonim, modern —
///   is three folders, which is what a person asked for.
/// - Under the rishonim, each of sixteen authors has exactly one sefer. Sixteen
///   folders you must open to find one row is worse than sixteen rows, so those
///   authors are not folders and their seforim sit in the rishonim folder.
/// - On Genesis, Abarbanel has four and the Chida three. Those *are* folders.
/// - The eighteen mefarshim on Shulchan Arukh, Orach Chayim share one shelf and
///   have an author apiece, so there is nothing to group: an empty tree, and the
///   window draws the list it drew before.
///
/// The shelf each sefer is on comes from [`crate::taxonomy::shelf_key_of`] and
/// never from its categories, so a shelf the reader moved or renamed is
/// respected here as everywhere else. There is a test that reads this file's own
/// source to keep it that way.
#[must_use]
pub fn folders(works: &[Work], arrangement: &Arrangement, shipped: &Shipped) -> Folders {
    let paths: Vec<(String, Vec<String>)> = works
        .iter()
        .map(|work| {
            let key = shelf_key_of(work, arrangement, shipped);
            let parts = key
                .split('/')
                .filter(|p| !p.is_empty())
                .map(ToString::to_string)
                .collect();
            (work.slug.clone(), parts)
        })
        .collect();
    let here: Vec<(&str, &[String])> = paths
        .iter()
        .map(|(slug, parts)| (slug.as_str(), parts.as_slice()))
        .collect();

    let mut out = Folders::default();
    out.tree = group(&here, &[], arrangement, shipped, &mut out.of);
    out.loose = works.len() - out.of.len();
    out
}

/// One level of folders, and which seforim end up in each.
///
/// `placed` gathers slug → folder key as it goes. A slug this returns without
/// placing is a **loose** row, drawn above the folders — which is not the same
/// as a lost one, and `Folders::loose` is what says so.
fn group(
    works: &[(&str, &[String])],
    prefix: &[String],
    arrangement: &Arrangement,
    shipped: &Shipped,
    placed: &mut BTreeMap<String, String>,
) -> Vec<Branch> {
    let mut buckets: BTreeMap<&str, Vec<(&str, &[String])>> = BTreeMap::new();
    for (slug, rest) in works {
        // Nothing left to file this one by: it belongs at this level.
        if let Some(next) = rest.first() {
            buckets
                .entry(next.as_str())
                .or_default()
                .push((slug, &rest[1..]));
        }
    }

    // One bucket holding the lot is a shelf they all share, and a shelf they all
    // share says nothing a reader did not know from the sefer they are standing
    // in. Stripped **before** anything is placed, and stripped again as deep as
    // it goes: `Talmud` then `Bavli` then — three folders, which is the answer.
    let shared = buckets.len() == 1
        && buckets
            .values()
            .next()
            .is_some_and(|held| held.len() == works.len());
    if shared {
        if let Some((name, held)) = buckets.pop_first() {
            let mut deeper = prefix.to_vec();
            deeper.push(name.to_string());
            return group(&held, &deeper, arrangement, shipped, placed);
        }
    }

    let mut out: Vec<Branch> = buckets
        .into_iter()
        .filter_map(|(name, held)| folder(name, &held, prefix, arrangement, shipped, placed))
        .collect();
    out.sort_by(|a, b| {
        rank_of(&a.title)
            .cmp(&rank_of(&b.title))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.title.cmp(&b.title))
    });
    out
}

/// One folder, if this bucket earns one.
///
/// **The one rule: a folder needs two seforim.** Sixteen rishonim on a masechta
/// are sixteen authors with one sefer apiece, and sixteen folders you must open
/// to find one row is worse than sixteen rows. Abarbanel on Genesis is four, and
/// that is a folder a person wants.
fn folder(
    name: &str,
    held: &[(&str, &[String])],
    prefix: &[String],
    arrangement: &Arrangement,
    shipped: &Shipped,
    placed: &mut BTreeMap<String, String>,
) -> Option<Branch> {
    if held.len() < 2 {
        return None;
    }
    let mut path = prefix.to_vec();
    path.push(name.to_string());
    let key = path.join("/");
    let children = group(held, &path, arrangement, shipped, placed);
    // Whoever the children did not take stands in this folder itself.
    let mut mine = 0;
    for (slug, _) in held {
        if !placed.contains_key(*slug) {
            placed.insert((*slug).to_string(), key.clone());
            mine += 1;
        }
    }
    Some(Branch {
        // The same three-step rule the bookcase uses. This was
        // `arrangement.title_of` alone — step 3 — so the chooser drew `Rif · 4`
        // between `ראשונים · 13` and `מפרשים · 3` while the bookcase four
        // inches away drew the same shelf as `רי״ף`.
        title: shelf_title(&key, arrangement, shipped),
        here: mine,
        count: held.len(),
        mine: arrangement.made.contains(&key),
        edited: arrangement.titles.contains_key(&key) || arrangement.shelves.contains_key(&key),
        children,
        // Not the bookcase's sort — this list is grouped by era and ordered by
        // `rank_of` a few lines up — but the field is a fact about the shelf and
        // is answered rather than left at a default.
        order: None,
        commentary: girsa_corpus::taxonomy::is_commentary_shelf(&key),
        key,
        // Never here: the mefarshim list already puts the ungrouped ones above the
        // folders rather than in a folder of their own, because a list of sixty
        // rows behind one heading is what W44 was for.
        loose: false,
    })
}

/// One row of the list behind `מפרשים · N`: a sefer you can open, tick, or both.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Choice {
    pub slug: String,
    pub he_title: String,
    pub en_title: String,
    /// Which corpus this sefer's text came from.
    ///
    /// On the row so the window can tell two copies apart. Sefaria and Otzaria
    /// both ship Rabbeinu Chananel on Bereshis, under two slugs and two
    /// spellings — `רבינו חננאל על בראשית` and `ר חננאל על בראשית` — and the
    /// list drew both with nothing saying why. It is **not** a merge: two
    /// catalogue entries are two seforim until something states otherwise, and
    /// this codebase does not guess at identity from a title. It is a label, so
    /// that a duplicate reads as two copies rather than as a bug.
    pub source: girsa_corpus::work::Source,
    /// How the corpus places this sefer against the one you are reading — a
    /// mefaresh on it, the sefer it is a mefaresh **on**, or its own sefer
    /// following the same order. `None` where only edges join them.
    ///
    /// See `girsa_corpus::taxonomy::settled` and [`crate::shelf::Related`]. This
    /// was `declared: bool`, and one bool over three claims is what put Bereshis
    /// in Onkelos's list labelled `פירוש`.
    pub stands: Option<Related>,
    // `said` and `why` were here — the row's Hebrew label and an English
    // sentence for the hover, both composed in Rust. That was the right move
    // for the wrong half: it is right that **what the relation is** is decided
    // beside the enum, and wrong that the *words* were. An English window drew
    // `פירוש` on every declared commentary and a Hebrew window put `the corpus
    // declares this a commentary on what you are reading` behind the hover —
    // both languages wrong, in opposite directions, on the same row.
    //
    // `stands` already crosses as a name (`on` / `base` / `alongside`), which
    // is the shape refusals and edge types settled on: the machine sends what
    // it means, `say.ts` holds both columns of what to call it.
    /// How many edges join the two, where that is all there is.
    pub links: usize,
    /// Whether ticking it could mark a line — that is, whether the link graph
    /// has it commenting somewhere in this sefer.
    ///
    /// **Not the same question as `stands`.** `Tosafot on Berakhot` declares
    /// itself a commentary; whether *this* corpus holds edges placing its
    /// comments on particular lines is a separate fact. A tick-box that can
    /// never mark anything is worse than no box.
    pub tickable: bool,
    pub chosen: bool,
    /// The folder it stands in (W44). `None` for one drawn above the folders.
    pub shelf: Option<String>,
}

/// A heading, or a sefer — the list behind the door, in reading order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Listed {
    Folder {
        title: String,
        depth: usize,
        count: usize,
    },
    Sefer {
        choice: Box<Choice>,
    },
}

/// The three headings this list draws that are not a shelf's own name.
///
/// In Rust because they are **what the sections mean**, not how they look, and
/// because a heading invented in the window is a heading no Rust test can hold.
/// `girsa_app::links::kinds` made the same move for edge types.
mod heading {
    /// The sefer this one was written **about** — the other direction, which a
    /// bool could not hold. Onkelos declares Bereshis; Bereshis is the sefer to
    /// put beside Onkelos and is not a peirush on it.
    pub const BASE: &str = "הספר שעליו נכתב";
    /// Seforim that keep this one's order without commenting on it.
    pub const ALONGSIDE: &str = "על סדר הספר";
    /// Declared commentaries whose comments the graph cannot place on a line.
    pub const NO_PLACE: &str = "פירושים בלי מקום בשורה";
    /// Merely joined by edges — the Beit Yosef cites Berakhot 815 times.
    pub const LINKED: &str = "ספרים מקושרים";
}

/// The whole list behind the door, woven once.
///
/// # Why this is here and not in the window
///
/// It was 277 lines of TypeScript beside a module with twenty-five Rust tests
/// about this same list, and the giveaway was the shape it had to be given:
/// `Mefarshim` arrived as **four parallel arrays** — `works`, `alongside`,
/// `folders`, `marked` — that only `listed()` in `mefarshim.ts` knew how to
/// weave together. Four sections, three Hebrew headings, and an ordering rule,
/// all decided in the one place nothing could test them against the corpus.
///
/// # The four sections, and why they are four
///
/// They are four different claims:
///
/// 1. the mefarshim the corpus places on this sefer's lines, in their folders —
///    rishonim together, acharonim together, because that is the first thing
///    anybody wants to know about a mefaresh;
/// 2. **`על סדר הספר`** — seforim that keep this one's order without commenting
///    on it: the Shulchan Arukh under the Tur, the Arukh HaShulchan under the
///    Shulchan Arukh. They tick and mark like a mefaresh and they are not one,
///    and saying so is the whole of this section;
/// 3. the commentaries the corpus **declares** but whose comments it cannot
///    place on a line, so they can be opened beside but not ticked;
/// 4. the seforim that merely share links, under a heading that says so.
///
/// A sefer appears **once**. A list that shows the same sefer under two
/// headings is a list that has stopped meaning anything.
#[must_use]
pub fn listed(
    companions: &[Companion],
    can_mark: &[String],
    alongside: &[String],
    folders: &Folders,
    chosen: &[String],
    shelf: &crate::shelf::Shelf,
    language: crate::session::Language,
) -> Vec<Listed> {
    let placeable: BTreeSet<&str> = can_mark.iter().map(String::as_str).collect();
    let named = |slug: &str, stands: Option<Related>, links: usize, tickable: bool| {
        let work = shelf.work(slug);
        Choice {
            he_title: work.map_or_else(|| slug.to_string(), |w| w.he_title.clone()),
            en_title: work.map_or_else(|| slug.to_string(), |w| w.en_title.clone()),
            source: work.map_or(girsa_corpus::work::Source::Mine, |w| w.source),
            stands,
            links,
            tickable,
            chosen: chosen.iter().any(|c| c == slug),
            shelf: folders.of.get(slug).cloned(),
            slug: slug.to_string(),
        }
    };
    // Where each sefer stands in the printed sequence, so a mefaresh's five
    // volumes come back in the order they are printed in rather than in the
    // order their slugs sort. Read once per row, not once per comparison.
    let order_of = |slug: &str| -> Vec<i32> {
        shelf
            .work(slug)
            .map(|w| w.order.clone())
            .unwrap_or_default()
    };
    let in_order = |a: &Choice, b: &Choice| {
        let (mine, theirs) = (order_of(&a.slug), order_of(&b.slug));
        match (mine.is_empty(), theirs.is_empty()) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => mine.cmp(&theirs),
        }
        .then_with(|| a.he_title.cmp(&b.he_title))
        .then_with(|| a.slug.cmp(&b.slug))
    };
    // **By name, in the language the list is being read in.**
    //
    // The rule the reader states generally: *aleph-beis order in Hebrew,
    // English order in English*, for every list of names in the application.
    // This list was ordered by **how many edges each mefaresh has** — a number
    // no reader can see, so the list read as unordered, and looking for the
    // Taz meant reading every row. A bachur looking for a mefaresh knows its
    // name and nothing about its edge count.
    //
    // Hebrew needs no collation table for this: א through ת are U+05D0..U+05EA
    // in aleph-beis sequence, so the ordinary string compare *is* aleph-beis
    // order. English is the same argument in the other alphabet, which is why
    // the language has to reach this function at all — `Choice` has carried
    // both titles all along and nothing ever chose between them.
    //
    // `in_order` stays as the tiebreak so that two volumes printed under one
    // title still come back in the order they are printed in.
    let by_name = |a: &Choice, b: &Choice| {
        let (mine, theirs) = match language {
            crate::session::Language::Hebrew => (&a.he_title, &b.he_title),
            crate::session::Language::English => (&a.en_title, &b.en_title),
        };
        mine.cmp(theirs).then_with(|| in_order(a, b))
    };

    // The companions, in the order a reader learns: placed first, then by how
    // much joins them, then in the order the seforim are printed in so the same
    // daf opened twice is the same list. Sorted rather than taken as it arrived,
    // because `companions()` builds in two passes.
    let mut rows: Vec<Choice> = companions
        .iter()
        .map(|c| {
            named(
                &c.slug,
                c.stands,
                c.links,
                placeable.contains(c.slug.as_str()),
            )
        })
        .collect();
    rows.sort_by(|a, b| {
        b.stands
            .is_some()
            .cmp(&a.stands.is_some())
            .then_with(|| by_name(a, b))
    });
    // Mefarshim the graph knows and the catalogue does not — the Ben Yehoyada on
    // Berakhot, most of Otzaria's shelf — follow rather than being dropped.
    let offered: BTreeSet<String> = rows.iter().map(|r| r.slug.clone()).collect();
    let mut rest: Vec<Choice> = can_mark
        .iter()
        .filter(|slug| !offered.contains(*slug))
        .map(|slug| named(slug, None, 0, true))
        .collect();
    rest.sort_by(&by_name);
    rows.extend(rest);

    let mut out: Vec<Listed> = Vec::new();
    let mut shown: BTreeSet<String> = BTreeSet::new();
    let mut say = |out: &mut Vec<Listed>, shown: &mut BTreeSet<String>, choice: &Choice| {
        shown.insert(choice.slug.clone());
        out.push(Listed::Sefer {
            choice: Box::new(choice.clone()),
        });
    };

    // The sefer this one is a commentary **on**, at the top, under its own
    // heading. Reading Onkelos, Bereshis is the first thing you want beside it
    // — and it is not a peirush on Onkelos, which is what the list said before
    // the relation had a direction.
    let base: Vec<Choice> = rows
        .iter()
        .filter(|r| r.stands == Some(Related::Base))
        .cloned()
        .collect();
    section(heading::BASE, &base, &mut out, &mut shown, &mut say);

    // The mefarshim with no folder go next, above the headings, so a heading
    // always has its own seforim under it and never somebody else's.
    let loose: Vec<Choice> = rows
        .iter()
        .filter(|r| r.tickable && r.shelf.is_none() && !shown.contains(&r.slug))
        .cloned()
        .collect();
    for row in &loose {
        say(&mut out, &mut shown, row);
    }
    walk(&folders.tree, 0, &rows, &mut out, &mut shown, &mut say);

    // Directly under the mefarshim, because they tick and mark exactly like one
    // and are the second thing a person reaching for a mefaresh wants — not down
    // with the merely-linked, which is where a bool had quietly filed them.
    //
    // **Two sources, and only one of them was being read.** `alongside` is built
    // from `inbound.jsonl` — the seforim whose edges land *on* this one — so
    // standing on Yoreh De'ah it held the Arukh HaShulchan and the Mishneh
    // Torah, and not the Tur. The Tur's edges point the other way (spec.md §8.2
    // stores an edge once, in the direction it was written, and the Shulchan
    // Arukh is the one that points at the Tur), so the sefer that keeps this
    // one's order more exactly than anything else in the corpus was never a
    // candidate for the heading that means exactly that.
    //
    // It was not missing from the list, which is worse: `companions` had it,
    // with `Related::Alongside` on it, and the partition below files anything
    // with a relation and no place under *פירושים בלי מקום בשורה*. So the Tur
    // was offered as a commentary on the Shulchan Arukh — the wrong claim, and
    // backwards by a century.
    let also: Vec<&str> = rows
        .iter()
        .filter(|r| r.stands == Some(Related::Alongside))
        .map(|r| r.slug.as_str())
        .collect();
    let mut following: Vec<Choice> = alongside
        .iter()
        .map(String::as_str)
        .chain(also)
        .filter(|slug| !shown.contains(*slug))
        .collect::<BTreeSet<&str>>()
        .into_iter()
        .map(|slug| {
            // A row `companions` already built keeps its edge count; one that
            // only the inbound cache knows about has none to keep.
            let links = rows.iter().find(|r| r.slug == slug).map_or(0, |r| r.links);
            named(slug, Some(Related::Alongside), links, true)
        })
        .collect();
    following.sort_by(&by_name);
    section(
        heading::ALONGSIDE,
        &following,
        &mut out,
        &mut shown,
        &mut say,
    );

    let left: Vec<Choice> = rows
        .iter()
        .filter(|r| !shown.contains(&r.slug))
        .cloned()
        .collect();
    let (declared, linked): (Vec<Choice>, Vec<Choice>) =
        left.into_iter().partition(|r| r.stands.is_some());
    section(heading::NO_PLACE, &declared, &mut out, &mut shown, &mut say);
    section(heading::LINKED, &linked, &mut out, &mut shown, &mut say);
    out
}

/// One heading and the seforim under it, or nothing at all.
fn section(
    title: &str,
    rows: &[Choice],
    out: &mut Vec<Listed>,
    shown: &mut BTreeSet<String>,
    say: &mut impl FnMut(&mut Vec<Listed>, &mut BTreeSet<String>, &Choice),
) {
    if rows.is_empty() {
        return;
    }
    out.push(Listed::Folder {
        title: title.to_string(),
        depth: 0,
        count: rows.len(),
    });
    for row in rows {
        say(out, shown, row);
    }
}

/// The folder tree, depth-first, with each folder's own seforim under it.
fn walk(
    branches: &[Branch],
    depth: usize,
    rows: &[Choice],
    out: &mut Vec<Listed>,
    shown: &mut BTreeSet<String>,
    say: &mut impl FnMut(&mut Vec<Listed>, &mut BTreeSet<String>, &Choice),
) {
    for branch in branches {
        let held: Vec<&Choice> = rows
            .iter()
            .filter(|r| r.shelf.as_deref() == Some(branch.key.as_str()))
            .collect();
        if held.is_empty() && branch.children.is_empty() {
            continue;
        }
        out.push(Listed::Folder {
            title: branch.title.clone(),
            depth,
            count: branch.count,
        });
        for row in held {
            say(out, shown, row);
        }
        walk(&branch.children, depth + 1, rows, out, shown, say);
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use std::path::Path;

    use super::*;

    const BERAKHOT: &str = "bavli/berakhot";
    const RASHI: &str = "bavli/rashi-on-berakhot";
    const TOSAFOT: &str = "bavli/tosafot-on-berakhot";

    /// The shelf these run against: works, an imported graph, an inbound cache.
    ///
    /// Seven of the tests below gated on the 3.4 GB download and `return`ed when
    /// it was absent, which is every fresh clone and every CI run — so what they
    /// printed was `ok` in 0.00s having looked at no mefarshim at all. The
    /// fixture has Berakhot with its Rashi, its Tosafos and a Penei Yehoshua on
    /// it, built by the real importer from a `merged.json` and a `links0.csv`.
    fn corpus() -> &'static Path {
        girsa_fixture::linked().root()
    }

    /// The real download, for the five checks below that are about *it* and not
    /// about this code.
    ///
    /// Named seforim from a Sefaria release — `bartenura-on-torah`,
    /// `arukh-hashulchan`, the Beis Yosef on the Tur — and thresholds that need
    /// a masechta's worth of commentary. No fixture can stand in for those and
    /// this one does not pretend to: they are `#[ignore]`d, so a run without the
    /// corpus prints `5 ignored` rather than five green ticks.
    ///
    /// ```sh
    /// cargo test -p girsa-app --lib -- --ignored
    /// ```
    fn real_corpus() -> std::path::PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
        assert!(
            root.join("links").is_dir() && inbound::built(&root),
            "no link graph with an inbound cache at {} — run girsa-link-import \
             then girsa-link-types. This check is #[ignore]d precisely so that \
             its absence is never read as a pass.",
            root.display()
        );
        root
    }

    /// The real shelf, with no personal layer over it.
    fn real_shelf(root: &Path) -> crate::shelf::Shelf {
        crate::shelf::Shelf::open(root, &root.join("no-personal-layer")).expect("the shelf opens")
    }

    /// Hand-built, so the filtering rules are tested without the corpus.
    ///
    /// Two Rashis on `2a:2`, because that is what a daf looks like.
    fn made_up() -> Marks {
        let mut marks = Marks::default();
        for (seg, said) in [
            (
                "girsa:bavli/berakhot/2a:1#1",
                vec![
                    (RASHI, "girsa:bavli/rashi-on-berakhot/2a:1:1#1"),
                    (TOSAFOT, "girsa:bavli/tosafot-on-berakhot/2a:1:1#1"),
                ],
            ),
            (
                "girsa:bavli/berakhot/2a:2#2",
                vec![
                    (RASHI, "girsa:bavli/rashi-on-berakhot/2a:2:1#2"),
                    (RASHI, "girsa:bavli/rashi-on-berakhot/2a:2:2#3"),
                ],
            ),
            (
                "girsa:bavli/berakhot/2a:3#3",
                vec![(
                    "bavli/meiri-on-berakhot",
                    "girsa:bavli/meiri-on-berakhot/2a:3:1#4",
                )],
            ),
        ] {
            for (work, at) in said {
                // Keyed through `SegmentId` rather than by the literal, so the
                // fixture cannot disagree with `Marks::of` about whether an id
                // carries its `girsa:` scheme. The first draft of this did, and
                // the three filtering tests all failed while the corpus one
                // passed.
                marks.add(
                    id(seg).to_string(),
                    &Spoken {
                        work: work.to_string(),
                        at: id(at),
                        to: None,
                    },
                );
                marks.works.insert(work.to_string());
            }
        }
        marks
    }

    fn id(s: &str) -> SegmentId {
        s.parse().expect("a segment id parses")
    }

    #[test]
    fn nothing_chosen_marks_nothing() {
        // The whole reason `on` takes the chosen set. Berakhot has commentary on
        // 3,183 segments; marking all of them is the same as marking none, only
        // noisier.
        assert!(made_up().marked(&[]).is_empty());
    }

    #[test]
    fn the_marked_lines_are_exactly_the_ones_the_chosen_mefaresh_speaks_on() {
        let marks = made_up();
        let marked = marks.marked(&[RASHI.to_string()]);
        assert_eq!(marked.len(), 2, "{marked:?}");
        assert!(marked.contains_key("girsa:bavli/berakhot/2a:1#1"));
        assert!(marked.contains_key("girsa:bavli/berakhot/2a:2#2"));
        assert!(
            !marked.contains_key("girsa:bavli/berakhot/2a:3#3"),
            "the Meiri's line is not Rashi's"
        );
    }

    #[test]
    fn the_marker_counts_the_chosen_who_speak_there() {
        // The half a bool threw away. A reader with two mefarshim ticked wants
        // to see the line where **both** of them are, and a `true` cannot carry
        // it — which is why ticking a targum, who speaks on every posuk,
        // produced a marker on every posuk and told nobody anything.
        let marks = made_up();
        let both = marks.marked(&[RASHI.to_string(), TOSAFOT.to_string()]);
        assert_eq!(
            both.get("girsa:bavli/berakhot/2a:1#1"),
            Some(&2),
            "Rashi and Tosfos are both here: {both:?}"
        );
        assert_eq!(
            both.get("girsa:bavli/berakhot/2a:2#2"),
            Some(&1),
            "Rashi alone"
        );
        assert!(
            !both.contains_key("girsa:bavli/berakhot/2a:3#3"),
            "the Meiri is not ticked, so his line is not marked: {both:?}"
        );
    }

    #[test]
    fn one_mefaresh_with_two_comments_on_a_line_is_one_mefaresh() {
        // The count is of ticked **seforim** speaking here, not of comments.
        // `2a:2` carries two Rashis, because that is what a daf looks like; a
        // Rashi with three dibburim on one posuk must not read as three of the
        // reader's mefarshim, or the busiest line in the sefer looks like the
        // one where everybody turned up.
        let marks = made_up();
        let marked = marks.marked(&[RASHI.to_string()]);
        assert_eq!(
            marked.get("girsa:bavli/berakhot/2a:2#2"),
            Some(&1),
            "two comments, one mefaresh: {marked:?}"
        );
    }

    #[test]
    fn a_line_nobody_wrote_about_and_a_line_none_of_yours_wrote_about_are_different() {
        let marks = made_up();
        // The Meiri wrote here; Rashi did not.
        let theirs = marks.on(&id("girsa:bavli/berakhot/2a:3#3"), &[RASHI.to_string()]);
        assert!(theirs.works.is_empty());
        assert!(theirs.any, "somebody wrote about this line");

        // Nobody wrote here at all.
        let nobody = marks.on(&id("girsa:bavli/berakhot/99z:9#999"), &[RASHI.to_string()]);
        assert!(nobody.works.is_empty());
        assert!(!nobody.any);
    }

    #[test]
    fn a_chosen_mefaresh_who_says_nothing_here_is_not_an_error() {
        let marks = made_up();
        let here = marks.on(
            &id("girsa:bavli/berakhot/2a:2#2"),
            &["bavli/rosh-on-berakhot".to_string(), RASHI.to_string()],
        );
        assert_eq!(here.works, vec![RASHI.to_string()]);
    }

    #[test]
    fn the_answer_comes_back_in_the_order_you_chose_them() {
        // Not alphabetical, and not the corpus's order. A reader who put the
        // Rosh above Rashi meant it.
        let marks = made_up();
        let chosen = vec![TOSAFOT.to_string(), RASHI.to_string()];
        let here = marks.on(&id("girsa:bavli/berakhot/2a:1#1"), &chosen);
        assert_eq!(here.works, chosen);
    }

    #[test]
    fn the_answer_says_where_in_the_commentary_to_read_not_only_who_wrote() {
        // The click needs an address. `Here` says *Rashi speaks here*, which is
        // enough to draw a marker and not enough to show a word of Rashi.
        let marks = made_up();
        let said = marks.said(&id("girsa:bavli/berakhot/2a:1#1"), &[RASHI.to_string()]);
        assert_eq!(said.len(), 1);
        assert_eq!(said[0].work, RASHI);
        assert_eq!(said[0].at.work(), RASHI, "the address is in the commentary");
    }

    #[test]
    fn one_mefaresh_can_say_two_things_about_one_line() {
        // Rashi on a single stretch of Gemara is routinely three or four
        // separate diburim, each its own segment. Returning one and calling it
        // Rashi would silently drop the rest.
        let marks = made_up();
        let said = marks.said(&id("girsa:bavli/berakhot/2a:2#2"), &[RASHI.to_string()]);
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said.iter().all(|s| s.work == RASHI));
    }

    #[test]
    fn what_you_did_not_tick_is_not_read() {
        let marks = made_up();
        let said = marks.said(&id("girsa:bavli/berakhot/2a:1#1"), &[TOSAFOT.to_string()]);
        assert_eq!(said.len(), 1);
        assert_eq!(said[0].work, TOSAFOT);
        assert!(marks
            .said(&id("girsa:bavli/berakhot/2a:1#1"), &[])
            .is_empty());
    }

    #[test]
    fn rashi_is_on_the_first_line_of_the_daf() {
        // The regression this is really about, checked without the download.
        //
        // Before `girsa_link::orient`, the commentary edge pointed from the daf
        // to Rashi and `inbound.jsonl` filed it under *Rashi* — so this map came
        // back empty for every masechta and the panel offered a reader two
        // aggadic works out of forty. The fixture writes its `comments-on` rows
        // in **both** column orders, exactly as Sefaria's export does, so the
        // orientation code is what has to put them right and this goes red if it
        // stops doing so.
        let root = corpus();
        let shelf = real_shelf(root);
        let marks = Marks::of(&shelf, BERAKHOT).expect("berakhot's inbound reads");

        assert!(
            marks.commentators().iter().any(|w| w == RASHI),
            "Rashi does not comment anywhere in Berakhot: {:?}",
            marks.commentators()
        );
        assert!(
            marks.commentators().iter().any(|w| w == TOSAFOT),
            "Tosafos does not comment anywhere in Berakhot: {:?}",
            marks.commentators()
        );

        // And on the line itself. `2a:3` is one of the pairs the fixture writes
        // base-first, so this is the assertion that would have been empty.
        let at = shelf
            .read(BERAKHOT)
            .expect("Berakhot opens")
            .segments
            .iter()
            .find(|s| s.id.path() == ["2a", "3"])
            .map(|s| s.id.clone())
            .expect("Berakhot 2a:3");
        let here = marks.on(&at, &[RASHI.to_string()]);
        assert_eq!(
            here.works,
            vec![RASHI.to_string()],
            "Rashi is not on Berakhot 2a:3"
        );
        println!(
            "{} commentators over {} segments of Berakhot",
            marks.commentators().len(),
            marks.segments_touched()
        );
    }

    #[test]
    #[ignore = "needs the fetched corpus: cargo test -p girsa-app --lib -- --ignored"]
    fn rashi_is_on_the_first_line_of_the_daf_in_the_real_corpus() {
        // The same claim at the scale that makes the threshold meaningful. A
        // masechta's worth of commentary is a fact about a Sefaria release.
        let root = real_corpus();
        let marks = Marks::of(&real_shelf(&root), BERAKHOT).expect("berakhot's inbound reads");

        assert!(
            marks.segments_touched() > 1000,
            "only {} segments of Berakhot carry commentary",
            marks.segments_touched()
        );
        assert!(
            marks.commentators().iter().any(|w| w == RASHI),
            "Rashi does not comment anywhere in Berakhot"
        );

        let here = marks.on(&id("girsa:bavli/berakhot/10a:1#418"), &[RASHI.to_string()]);
        assert_eq!(
            here.works,
            vec![RASHI.to_string()],
            "Rashi is not on Berakhot 10a:1"
        );
        println!(
            "{} commentators over {} segments of Berakhot",
            marks.commentators().len(),
            marks.segments_touched()
        );
    }

    // ── W44: the mefarshim keep their folders ───────────────────────────────

    /// Works with the categories Sefaria files them under, for the folder tests.
    ///
    /// Built through `shelf::tests::work` so there is one place that knows what
    /// a `Work` has in it — a fixture with its own copy of the struct is a
    /// fixture that stops compiling for no reason a reader can see.
    fn works_named(names: &[(&str, &[&str])]) -> Vec<girsa_corpus::work::Work> {
        names
            .iter()
            .map(|(slug, categories)| {
                let mut work = crate::shelf::tests::work(slug);
                work.categories = categories.iter().map(|c| (*c).to_string()).collect();
                work
            })
            .collect()
    }

    #[test]
    fn the_rishonim_are_together_and_the_acharonim_are_together() {
        // The shape of the mefarshim on a masechta, as Sefaria files them. Two
        // rishonim, two acharonim, one modern.
        let works = works_named(&[
            (
                "bavli/rashi-on-berakhot",
                &["Talmud", "Bavli", "Rishonim on Talmud", "Rashi"],
            ),
            (
                "bavli/tosafot-on-berakhot",
                &["Talmud", "Bavli", "Rishonim on Talmud", "Tosafot"],
            ),
            (
                "bavli/ben-yehoyada",
                &["Talmud", "Bavli", "Acharonim on Talmud", "Ben Yehoyada"],
            ),
            (
                "bavli/pnei-yehoshua",
                &["Talmud", "Bavli", "Acharonim on Talmud", "Pnei Yehoshua"],
            ),
            (
                "bavli/steinsaltz-on-berakhot",
                &[
                    "Talmud",
                    "Bavli",
                    "Modern Commentary on Talmud",
                    "Steinsaltz",
                ],
            ),
            (
                "bavli/reshimot-shiurim-on-berakhot",
                &[
                    "Talmud",
                    "Bavli",
                    "Modern Commentary on Talmud",
                    "Reshimot Shiurim",
                ],
            ),
        ]);
        let folders = folders(&works, &Arrangement::default(), &Shipped::of(&works));

        assert_eq!(folders.tree.len(), 3, "{:?}", titles(&folders.tree));
        // Rishonim first: that order is a fact about the seforim and it is
        // `girsa_corpus::taxonomy::rank_of`'s, not this module's.
        assert_eq!(folders.tree[0].count, 2);
        assert_eq!(
            folders.of.get("bavli/rashi-on-berakhot"),
            folders.of.get("bavli/tosafot-on-berakhot"),
            "Rashi and Tosafot are both rishonim and belong in one folder"
        );
        assert_ne!(
            folders.of.get("bavli/rashi-on-berakhot"),
            folders.of.get("bavli/ben-yehoyada"),
        );
    }

    #[test]
    fn grouping_the_mefarshim_loses_none_of_them() {
        // The rule the whole shelf turns on, applied here: every sefer is under
        // exactly one branch, and a folder view that quietly drops one is worse
        // than no folder view.
        let works = works_named(&[
            (
                "bavli/rashi-on-berakhot",
                &["Talmud", "Bavli", "Rishonim on Talmud", "Rashi"],
            ),
            (
                "bavli/tosafot-on-berakhot",
                &["Talmud", "Bavli", "Rishonim on Talmud", "Tosafot"],
            ),
            (
                "bavli/ben-yehoyada",
                &["Talmud", "Bavli", "Acharonim on Talmud", "Ben Yehoyada"],
            ),
            (
                "bavli/pnei-yehoshua",
                &["Talmud", "Bavli", "Acharonim on Talmud", "Pnei Yehoshua"],
            ),
            ("nothing-says", &[]),
        ]);
        let folders = folders(&works, &Arrangement::default(), &Shipped::of(&works));
        // Every sefer is in a folder or loose, and nothing is both or neither.
        // The one with no categories at all is loose, which is the honest answer:
        // it is on the `אחר` shelf with nobody to stand beside.
        let counted: usize = folders.tree.iter().map(|b| b.count).sum();
        assert_eq!(
            counted,
            folders.of.len(),
            "the folders hold what was placed"
        );
        assert_eq!(
            counted + folders.loose,
            works.len(),
            "the folders and the loose rows come to the whole list"
        );
        assert_eq!(folders.loose, 1, "only the one nothing says anything about");
    }

    #[test]
    fn a_folder_holding_every_mefaresh_is_not_a_folder() {
        // The mefarshim on Shulchan Arukh, Orach Chayim: eighteen works, all of
        // them under `Halakhah/Shulchan Arukh/Commentary`, one author apiece. A
        // tree one level deep containing everything is a list with an extra
        // click in front of it.
        let works = works_named(&[
            (
                "magen-avraham",
                &["Halakhah", "Shulchan Arukh", "Commentary", "Magen Avraham"],
            ),
            (
                "mishnah-berurah",
                &[
                    "Halakhah",
                    "Shulchan Arukh",
                    "Commentary",
                    "Mishnah Berurah",
                ],
            ),
        ]);
        let folders = folders(&works, &Arrangement::default(), &Shipped::of(&works));
        assert!(folders.tree.is_empty(), "{:?}", titles(&folders.tree));
        assert_eq!(folders.loose, 2, "both are drawn, ungrouped");
    }

    #[test]
    fn one_sefer_is_not_given_a_folder_of_its_own() {
        // Sixteen rishonim on a masechta are sixteen authors with one sefer
        // each. Folders named after them would be sixteen folders you have to
        // open to find one row.
        let works = works_named(&[
            (
                "bavli/rashi-on-berakhot",
                &["Talmud", "Bavli", "Rishonim on Talmud", "Rashi", "Zeraim"],
            ),
            (
                "bavli/tosafot-on-berakhot",
                &["Talmud", "Bavli", "Rishonim on Talmud", "Tosafot", "Zeraim"],
            ),
        ]);
        let folders = folders(&works, &Arrangement::default(), &Shipped::of(&works));
        // Both are rishonim; the common shelf is stripped, and what is left —
        // one author each — is not worth a folder. So: no folders, two rows.
        assert!(folders.tree.is_empty(), "{:?}", titles(&folders.tree));
    }

    #[test]
    fn an_author_with_several_seforim_does_get_a_folder() {
        // The other side of the same rule, and the case W44 was filed for:
        // Abarbanel on Genesis is four seforim, the Chida another three. Those
        // are folders a person wants.
        let works = works_named(&[
            (
                "abarbanel-a",
                &["Tanakh", "Rishonim on Tanakh", "Abarbanel"],
            ),
            (
                "abarbanel-b",
                &["Tanakh", "Rishonim on Tanakh", "Abarbanel"],
            ),
            (
                "ramban-on-genesis",
                &["Tanakh", "Rishonim on Tanakh", "Ramban"],
            ),
            ("chida-a", &["Tanakh", "Acharonim on Tanakh", "Chida"]),
            ("chida-b", &["Tanakh", "Acharonim on Tanakh", "Chida"]),
        ]);
        let folders = folders(&works, &Arrangement::default(), &Shipped::of(&works));
        assert_eq!(folders.tree.len(), 2, "{:?}", titles(&folders.tree));

        let rishonim = folders
            .tree
            .iter()
            .find(|b| b.count == 3)
            .expect("the rishonim folder holds three");
        assert_eq!(
            rishonim.children.len(),
            1,
            "only Abarbanel earns a subfolder"
        );
        assert_eq!(rishonim.children[0].count, 2);
        assert_eq!(rishonim.here, 1, "the Ramban sits in the folder itself");
        assert_eq!(
            folders.of.get("abarbanel-a"),
            folders.of.get("abarbanel-b"),
            "one author, one folder"
        );
        assert_ne!(
            folders.of.get("ramban-on-genesis"),
            folders.of.get("abarbanel-a"),
        );
    }

    #[test]
    fn a_shelf_the_reader_moved_is_respected_here_too() {
        // Through `taxonomy::shelf_key_of`, which is the whole reason this asks
        // that function rather than reading `categories`: a reader who moved the
        // Ben Yehoyada onto the rishonim shelf sees it there in every list,
        // including this one.
        let works = works_named(&[
            (
                "bavli/rashi-on-berakhot",
                &["Talmud", "Bavli", "Rishonim on Talmud", "Rashi"],
            ),
            (
                "bavli/ben-yehoyada",
                &["Talmud", "Bavli", "Acharonim on Talmud", "Ben Yehoyada"],
            ),
        ]);
        let mut moved = Arrangement::default();
        let onto = crate::taxonomy::shelf_key_of(&works[0], &moved, &Shipped::of(&works));
        moved
            .works
            .insert("bavli/ben-yehoyada".to_string(), onto.clone());

        let folders = folders(&works, &moved, &Shipped::of(&works));
        assert_eq!(
            folders.of.get("bavli/ben-yehoyada"),
            folders.of.get("bavli/rashi-on-berakhot"),
            "the reader put them on the same shelf, so they are in the same place"
        );
    }

    #[test]
    fn this_module_does_not_read_categories() {
        // The guard behind the test above. `categories` is the shipped default
        // and `shelf_key_of` is the answer; a second reader of the first would
        // put a sefer in one folder here and another on the shelf, with nothing
        // saying which was wrong. Written split so this line is not its own
        // counter-example.
        //
        // Only the module, not the tests below it: a fixture that *sets*
        // categories to stand a work on a shelf is the input to the rule, not a
        // second reader of it.
        let source = include_str!("mefarshim.rs");
        let module = source
            .split_once("#[cfg(test)]")
            .map_or(source, |(before, _)| before);
        let reads = module.matches(concat!(".categor", "ies")).count();
        assert_eq!(
            reads, 0,
            "the folder grouping reads a work's categories directly"
        );
    }

    fn titles(branches: &[crate::taxonomy::Branch]) -> Vec<String> {
        branches.iter().map(|b| b.title.clone()).collect()
    }

    #[test]
    #[ignore = "needs the fetched corpus: cargo test -p girsa-app --lib -- --ignored"]
    fn the_mefarshim_of_a_real_masechta_come_back_in_their_folders() {
        // Corpus-scale by construction: `folders.of` counts the seforim that
        // landed *in* a folder, and a mefaresh with no shelf-mates is drawn
        // loose above them rather than lost. Whether every mefaresh has
        // shelf-mates is a fact about how many mefarshim a masechta has, which
        // is a fact about the download.
        let root = real_corpus();
        let shelf = real_shelf(&root);
        let marks = Marks::of(&shelf, BERAKHOT).expect("reads");
        let on: Vec<girsa_corpus::work::Work> = marks
            .commentators()
            .iter()
            .filter_map(|slug| shelf.work(slug).cloned())
            .collect();
        let folders = folders(&on, &Arrangement::default(), &Shipped::of(&on));

        assert_eq!(folders.of.len(), on.len(), "every mefaresh is placed");
        assert!(
            folders.tree.len() >= 2,
            "the mefarshim on Berakhot fall into one bucket: {:?}",
            titles(&folders.tree)
        );
        println!(
            "{} mefarshim on Berakhot in {} folders: {:?}",
            on.len(),
            folders.tree.len(),
            titles(&folders.tree)
        );
    }

    #[test]
    #[ignore = "needs the fetched corpus: cargo test -p girsa-app --lib -- --ignored"]
    fn the_number_on_the_door_in_the_onboarding_is_the_number_the_door_shows() {
        // `docs/start-here.md` step 2 opens *"the button that says **מפרשים ·
        // 30**"*. The button says 34, and had said 34 in the screenshot twelve
        // lines below it the whole time — the page carried both numbers for the
        // same button on the same masechta and nothing compared them.
        //
        // It is the second sentence of the second step of the walkthrough the
        // README calls the whole product, so a reader meets it before anything
        // else, with the window open beside the page.
        //
        // The number is read **out of the page** rather than written here.
        // Written here it would be a third copy, free to agree with a stale doc;
        // read out, the test fails when either side moves and says which.
        let root = real_corpus();
        let shelf = real_shelf(&root);
        let counted = shelf
            .companions(BERAKHOT)
            .iter()
            .filter(|c| c.stands == Some(crate::shelf::Related::On))
            .count();

        let page =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/start-here.md");
        let body = std::fs::read_to_string(&page)
            .unwrap_or_else(|e| panic!("{} reads: {e}", page.display()));
        // `מפרשים · N`, however it is emphasised around the digits.
        let printed: Vec<usize> = body
            .split("מפרשים · ")
            .skip(1)
            .filter_map(|after| {
                let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
                digits.parse().ok()
            })
            .collect();
        assert!(
            !printed.is_empty(),
            "{} no longer prints `מפרשים · N`, so this check is asserting nothing",
            page.display()
        );
        for said in &printed {
            assert_eq!(
                *said, counted,
                "start-here.md says the button reads {said} on {BERAKHOT} and it reads {counted}"
            );
        }
    }

    #[test]
    #[ignore = "needs the fetched corpus: cargo test -p girsa-app --lib -- --ignored"]
    fn every_folder_in_the_chooser_is_named_in_hebrew() {
        // Finding 17. The chooser drew `Rif · 4` between `ראשונים · 13` and
        // `מפרשים · 3` — an untranslated Sefaria category name in a Hebrew
        // list, four inches from a bookcase that drew the same shelf as
        // `רי״ף`. Two places naming the same shelves, and nothing making them
        // agree: `folder` here reached for `arrangement.title_of` alone, which
        // is the last of `taxonomy::shelf_title`'s three steps and the one
        // that hands back the key.
        //
        // Over **many** masechtos rather than over Berakhos, because the bug
        // was on a shelf Berakhos does not have. That is what makes this a
        // corpus test: the failing case is somewhere in the download, and
        // asserting the class is the only way to reach it.
        let root = real_corpus();
        let shelf = real_shelf(&root);
        let all: Vec<girsa_corpus::work::Work> = shelf.works().to_vec();
        let shipped = Shipped::of(&all);

        fn latin(branches: &[crate::taxonomy::Branch], out: &mut Vec<String>) {
            for branch in branches {
                if !branch
                    .title
                    .chars()
                    .any(|c| ('\u{0590}'..='\u{05FF}').contains(&c))
                {
                    out.push(format!("{} · {}", branch.title, branch.count));
                }
                latin(&branch.children, out);
            }
        }

        let mut unnamed = Vec::new();
        let mut checked = 0;
        for work in &all {
            let Ok(marks) = Marks::of(&shelf, &work.slug) else {
                continue;
            };
            let on: Vec<girsa_corpus::work::Work> = marks
                .commentators()
                .iter()
                .filter_map(|slug| shelf.work(slug).cloned())
                .collect();
            if on.len() < 2 {
                continue;
            }
            checked += 1;
            latin(
                &folders(&on, &Arrangement::default(), &shipped).tree,
                &mut unnamed,
            );
            // Enough to cross every shelf the mefarshim of Shas and Tanach
            // stand on, and not the whole 7,188-work catalogue: this reads one
            // marks shard per sefer.
            if checked >= 200 {
                break;
            }
        }
        unnamed.sort();
        unnamed.dedup();
        println!("{checked} seforim' mefarshim folders checked");
        assert!(
            unnamed.is_empty(),
            "{} folders in the mefarshim chooser have no Hebrew name: {:?}",
            unnamed.len(),
            &unnamed[..unnamed.len().min(20)]
        );
    }

    #[test]
    fn the_sefer_is_not_a_commentary_on_its_own_mefarshim() {
        // Self-edges are dropped. Reading a sefer whose shard still held a
        // reversed edge would otherwise mark every line with the sefer's own
        // name.
        let root = corpus();
        let marks = Marks::of(&real_shelf(root), BERAKHOT).expect("reads");
        assert!(
            !marks.commentators().iter().any(|w| w == BERAKHOT),
            "Berakhot lists itself as one of its own mefarshim"
        );
    }

    #[test]
    fn nothing_is_offered_as_a_mefaresh_that_is_not_one_anywhere_on_the_shelf() {
        // The test W43 needed and did not have, and the reason it did not: the
        // one corpus test read Berakhot, where all 30 works with commentary edges
        // landing in it are declared commentaries on it. One clean masechta is
        // not the corpus. Tur is 4 of 40.
        //
        // Five different parts of the shelf, so a rule that works for the Talmud
        // and not for halakhah fails here rather than in front of a reader.
        let root = corpus();
        let shelf = real_shelf(root);
        let sample = [
            BERAKHOT,
            "tur",
            "shulchan-arukh/orach-chayim",
            "shulchan-arukh/yoreh-deah",
            "genesis",
            "mishnah-berakhot",
            "psalms",
        ];
        for slug in sample {
            let Some(base) = shelf.work(slug) else {
                continue;
            };
            let marks = Marks::of(&shelf, slug).expect("inbound reads");
            for offered in marks.commentators() {
                let Some(work) = shelf.work(&offered) else {
                    panic!("{slug} offers {offered}, which is not on the shelf");
                };
                let verdict = girsa_corpus::taxonomy::stands(work, base);
                assert!(
                    matches!(verdict, Stands::On | Stands::AskTheEdges),
                    "{slug} offers {offered} as a mefaresh and it stands {verdict:?} to it"
                );
            }
            // And nothing is in two places at once: a sefer is a mefaresh or it
            // runs alongside, never drawn in both groups.
            for beside in marks.alongside() {
                assert!(
                    !marks.commentators().contains(&beside),
                    "{slug} draws {beside} as both a mefaresh and alongside"
                );
            }
            println!(
                "{slug}: {} mefarshim, {} alongside, {} refused",
                marks.commentators().len(),
                marks.alongside().len(),
                marks.refused().len()
            );
        }
    }

    #[test]
    #[ignore = "needs the fetched corpus: cargo test -p girsa-app --lib -- --ignored"]
    fn the_mefarshim_a_person_would_look_for_are_actually_offered() {
        // Named seforim out of a Sefaria release, deliberately: this file's own
        // note says a count would go green again the day it changed for an
        // unrelated reason. Names are what make it a real check, and names from
        // the download are what make it need the download.
        // The half the suite did not have, and the reason a filter bug could sit
        // in the corpus for a release with every test green: everything checked
        // that nothing *wrong* was offered, and nothing checked that anything
        // *right* was. W43 tightened the rule from 40 mefarshim on the Tur to 5;
        // an over-tightening would have looked exactly as clean.
        //
        // Named seforim, not counts. A count goes green again the day it changes
        // for an unrelated reason.
        let root = real_corpus();
        let shelf = real_shelf(&root);

        let genesis = Marks::of(&shelf, "genesis").expect("reads");
        for wanted in [
            // Filed `["Tanakh","Rishonim on Tanakh"]` with no division named, so
            // the shelf sits over the whole of Tanakh and only the graph settles
            // it. Every one of these was refused before.
            "bartenura-on-torah",
            "beit-halevi-on-torah",
            "chatam-sofer-on-torah",
            "chanukat-hatorah",
            "em-lamikra",
            // Otzaria's, filed in Hebrew. Refused because the comparison ran over
            // `categories` as written and his are not in English.
            "ר-חננאל-על-בראשית",
        ] {
            assert!(
                genesis.commentators().iter().any(|w| w == wanted),
                "{wanted} is not offered as a mefaresh on Bereshis: {:?}",
                genesis.refused()
            );
        }

        // And the guard on all of the above: a commentary on the Chumash does not
        // thereby become one on Tehillim. This is what the division test and the
        // edge threshold are for, and it is the failure this fix could plausibly
        // have introduced.
        let psalms = Marks::of(&shelf, "psalms").expect("reads");
        for not_here in ["bartenura-on-torah", "chatam-sofer-on-torah", "em-lamikra"] {
            assert!(
                !psalms.commentators().iter().any(|w| w == not_here),
                "{not_here} is offered as a mefaresh on Tehillim"
            );
        }
    }

    #[test]
    #[ignore = "needs the fetched corpus: cargo test -p girsa-app --lib -- --ignored"]
    fn a_code_that_keeps_the_order_is_listed_apart_from_the_mefarshim() {
        // The Tur, the Shulchan Arukh and the Arukh HaShulchan, by name and by
        // the categories the download files them under.
        // W44b's whole point, and the thing a bool could not say. The Shulchan
        // Arukh keeps the Tur's order and is not a commentary on it; the Arukh
        // HaShulchan does the same to the Shulchan Arukh. Both were thrown away
        // before — not by a decision, but by the shape of their categories.
        let root = real_corpus();
        let shelf = real_shelf(&root);

        let tur = Marks::of(&shelf, "tur").expect("reads");
        assert!(
            tur.alongside()
                .iter()
                .any(|w| w.starts_with("shulchan-arukh/")),
            "the Shulchan Arukh does not run alongside the Tur: {:?}",
            tur.alongside()
        );
        assert!(
            !tur.commentators()
                .iter()
                .any(|w| w.starts_with("shulchan-arukh/")),
            "and it is not listed among the Tur's mefarshim"
        );

        let arukh = Marks::of(&shelf, "shulchan-arukh/orach-chayim").expect("reads");
        assert!(
            arukh.alongside().iter().any(|w| w == "arukh-hashulchan"),
            "the Arukh HaShulchan does not run alongside Orach Chayim: {:?}",
            arukh.alongside()
        );
    }

    #[test]
    #[ignore = "needs the fetched corpus: cargo test -p girsa-app --lib -- --ignored"]
    fn the_seforim_the_reader_named_are_gone_and_the_real_ones_are_not() {
        // The three the reader found, on the real graph. There is nothing else
        // they could be checked against.
        // The three the reader found, as a test, on the real graph. Named because
        // an assertion about *counts* would go green again the day the counts
        // change for some other reason.
        let root = real_corpus();
        let shelf = real_shelf(&root);

        let tur = Marks::of(&shelf, "tur").expect("reads");
        assert!(
            !tur.commentators().iter().any(|w| w == RASHI),
            "Rashi on Berakhot is still a mefaresh on the Tur"
        );
        assert!(
            !tur.commentators()
                .iter()
                .any(|w| w.starts_with("shulchan-arukh/")),
            "the Shulchan Arukh is still a mefaresh on the Tur"
        );
        // And the Beit Yosef, which declares nothing, is still there — the fix
        // must not be *keep only the declared*, which would empty the Tur.
        assert!(
            tur.commentators().iter().any(|w| w == "beit-yosef"),
            "the Beit Yosef is not a mefaresh on the Tur: {:?}",
            tur.commentators()
        );

        let arukh = Marks::of(&shelf, "shulchan-arukh/orach-chayim").expect("reads");
        assert!(
            !arukh.commentators().iter().any(|w| w == "bavli/shabbat"),
            "Shabbos is still a mefaresh on the Shulchan Arukh"
        );
        assert!(
            arukh
                .commentators()
                .iter()
                .any(|w| w.starts_with("kaf-hachayim-on-")),
            "the Kaf HaChayim is not a mefaresh on Orach Chayim: {:?}",
            arukh.commentators()
        );
        assert!(
            arukh.commentators().iter().any(|w| w == "mishnah-berurah"),
            "the Mishnah Berurah is not a mefaresh on Orach Chayim"
        );
    }

    // ── the list behind the door (W43, W44) ─────────────────────────────────
    //
    // These were `app/test/mefarshim.test.mjs`, asserting against hand-written
    // TypeScript constants. They are here now, against the same types the
    // window is sent, in the module that already carried twenty-five tests
    // about this list and could not see the weave.

    fn shelf_of(works: &[(&str, &str)]) -> crate::shelf::Shelf {
        let works: Vec<Work> = works
            .iter()
            .map(|(slug, title)| {
                let mut work = crate::shelf::tests::work(slug);
                work.he_title = (*title).to_string();
                work
            })
            .collect();
        crate::shelf::tests::shelf_of(works, std::path::Path::new("no-personal-layer"))
    }

    fn companion(slug: &str, declared: bool, links: usize) -> Companion {
        Companion {
            slug: slug.to_string(),
            he_title: slug.to_string(),
            en_title: slug.to_string(),
            links,
            stands: declared.then_some(crate::shelf::Related::On),
        }
    }

    /// The list as a shape a test can read: `# heading` or a slug.
    fn shape(rows: &[Listed]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Listed::Folder { title, .. } => format!("# {title}"),
                Listed::Sefer { choice } => choice.slug.clone(),
            })
            .collect()
    }

    fn folder(key: &str, title: &str, count: usize) -> Branch {
        Branch {
            key: key.to_string(),
            title: title.to_string(),
            here: count,
            count,
            mine: false,
            edited: false,
            children: Vec::new(),
            order: None,
            commentary: false,
            loose: false,
        }
    }

    #[test]
    fn a_mefaresh_the_graph_can_place_is_tickable_and_says_whether_it_is_ticked() {
        // One list, two jobs: click a row to open that sefer in the column
        // beside you — the split, which the reader asked to keep — or tick it
        // to have its comments marked on the daf. Every row does the first.
        // Only a row whose comments the graph can actually place does the
        // second: a checkbox that will never mark a line teaches the reader
        // that the ticks do nothing.
        let shelf = shelf_of(&[
            ("rashi", "רש״י"),
            ("tosafot", "תוספות"),
            ("ben-yehoyada", "בן יהוידע"),
        ]);
        let companions = [
            companion("rashi", true, 3139),
            companion("tosafot", true, 812),
        ];
        let can_mark = ["rashi".to_string(), "ben-yehoyada".to_string()];
        let rows = listed(
            &companions,
            &can_mark,
            &[],
            &Folders::default(),
            &["rashi".to_string()],
            &shelf,
            crate::session::Language::Hebrew,
        );
        let ticks: Vec<(String, bool)> = rows
            .iter()
            .filter_map(|row| match row {
                Listed::Sefer { choice } if choice.tickable => {
                    Some((choice.slug.clone(), choice.chosen))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            ticks,
            [
                ("rashi".to_string(), true),
                ("ben-yehoyada".to_string(), false)
            ]
        );
    }

    #[test]
    fn a_sefer_this_one_follows_is_listed_as_following_it_and_not_as_a_peirush() {
        // A10, the half of it that is in this function. Standing on Yoreh
        // De'ah, *על סדר הספר* held the Arukh HaShulchan and the Mishneh Torah
        // and not the **Tur** — and the Tur was in the list all along, under
        // *פירושים בלי מקום בשורה*, offered as a commentary on the Shulchan
        // Arukh.
        //
        // Two headings, two different claims, and the row went to the wrong one
        // because this function read `alongside` and nothing else. That list is
        // built from `inbound.jsonl` — the edges that land *on* this sefer —
        // and the Shulchan Arukh is the one that points at the Tur, so the Tur
        // could never appear in it however exactly it keeps the order.
        let shelf = shelf_of(&[("tur", "טור"), ("arukh-hashulchan", "ערוך השולחן")]);
        let companions = [Companion {
            slug: "tur".to_string(),
            he_title: "טור".to_string(),
            en_title: "טור".to_string(),
            links: 410,
            stands: Some(crate::shelf::Related::Alongside),
        }];
        let rows = listed(
            &companions,
            &[],
            &["arukh-hashulchan".to_string()],
            &Folders::default(),
            &[],
            &shelf,
            crate::session::Language::Hebrew,
        );
        assert_eq!(
            shape(&rows),
            [
                format!("# {}", heading::ALONGSIDE),
                "tur".to_string(),
                "arukh-hashulchan".to_string(),
            ],
            "the Tur belongs under {} with the sefer that reached it the other way",
            heading::ALONGSIDE
        );
        // And it keeps the edge count `companions` gave it. The inbound-only
        // rows have none, and taking the whole group from that side would have
        // thrown this away.
        let tur = rows.iter().find_map(|row| match row {
            Listed::Sefer { choice } if choice.slug == "tur" => Some(choice),
            _ => None,
        });
        assert_eq!(tur.map(|c| c.links), Some(410));
    }

    #[test]
    fn the_mefarshim_come_back_in_aleph_beis_order() {
        // The reader's rule, stated generally: *aleph-beis order in Hebrew,
        // English order in English*, for every list of names in the window.
        //
        // The list used to be ordered by **edge count** — a number that appears
        // nowhere on screen — so finding the Taz meant reading every row. The
        // three titles here are deliberately in the *opposite* order to their
        // counts: on the old sort this comes back שפתי כהן, טורי זהב, באר הגולה
        // and the assertion below fails.
        //
        // No collation table is needed for the Hebrew: א..ת are
        // U+05D0..U+05EA in aleph-beis sequence, so an ordinary string compare
        // is the right one.
        let shelf = shelf_of(&[
            ("siftei-kohen", "שפתי כהן"),
            ("turei-zahav", "טורי זהב"),
            ("beer-hagolah", "באר הגולה"),
        ]);
        let companions = [
            companion("siftei-kohen", true, 900),
            companion("turei-zahav", true, 400),
            companion("beer-hagolah", true, 100),
        ];
        let rows = listed(
            &companions,
            &[
                "siftei-kohen".to_string(),
                "turei-zahav".to_string(),
                "beer-hagolah".to_string(),
            ],
            &[],
            &Folders::default(),
            &[],
            &shelf,
            crate::session::Language::Hebrew,
        );
        let titles: Vec<String> = rows
            .iter()
            .filter_map(|row| match row {
                Listed::Sefer { choice } => Some(choice.he_title.clone()),
                Listed::Folder { .. } => None,
            })
            .collect();
        assert_eq!(titles, ["באר הגולה", "טורי זהב", "שפתי כהן"]);
    }

    #[test]
    fn a_declared_commentary_with_no_edges_is_offered_and_not_tickable() {
        // Tosafos declares itself a commentary and this graph has no edges from
        // it. It is still offered — the split opens it fine — but ticking it
        // would mark nothing, so it gets no tick-box rather than a dead one.
        let shelf = shelf_of(&[("rashi", "רש״י"), ("tosafot", "תוספות")]);
        let rows = listed(
            &[
                companion("rashi", true, 3139),
                companion("tosafot", true, 812),
            ],
            &["rashi".to_string()],
            &[],
            &Folders::default(),
            &[],
            &shelf,
            crate::session::Language::Hebrew,
        );
        let tosafot = rows.iter().find_map(|row| match row {
            Listed::Sefer { choice } if choice.slug == "tosafot" => Some(choice.clone()),
            _ => None,
        });
        let tosafot = tosafot.expect("Tosafos is offered");
        assert!(!tosafot.tickable, "a dead tick-box");
        assert_eq!(
            tosafot.stands,
            Some(Related::On),
            "and it is still a declared commentary"
        );
    }

    #[test]
    fn a_mefaresh_the_graph_knows_and_the_catalogue_does_not_is_still_offered() {
        // The Ben Yehoyada is not in Sefaria's declared list for Berakhot and
        // comments on it all the same. Dropping it would hide a real mefaresh
        // behind a metadata gap.
        let shelf = shelf_of(&[("rashi", "רש״י"), ("ben-yehoyada", "בן יהוידע")]);
        let rows = listed(
            &[companion("rashi", true, 3139)],
            &["rashi".to_string(), "ben-yehoyada".to_string()],
            &[],
            &Folders::default(),
            &[],
            &shelf,
            crate::session::Language::Hebrew,
        );
        assert_eq!(shape(&rows), ["rashi", "ben-yehoyada"]);
    }

    #[test]
    fn no_sefer_is_listed_twice_however_many_sections_could_claim_it() {
        // A list that shows the same sefer under two headings is a list that
        // has stopped meaning anything.
        let shelf = shelf_of(&[("rashi", "רש״י"), ("shulchan-arukh", "שולחן ערוך")]);
        let rows = listed(
            &[
                companion("rashi", true, 3139),
                companion("shulchan-arukh", true, 40),
            ],
            &["rashi".to_string(), "shulchan-arukh".to_string()],
            &["shulchan-arukh".to_string()],
            &Folders::default(),
            &[],
            &shelf,
            crate::session::Language::Hebrew,
        );
        let seforim: Vec<String> = shape(&rows)
            .into_iter()
            .filter(|row| !row.starts_with("# "))
            .collect();
        let mut once = seforim.clone();
        once.sort();
        once.dedup();
        assert_eq!(seforim.len(), once.len(), "{seforim:?}");
    }

    #[test]
    fn the_mefarshim_stand_in_their_folders_and_the_loose_one_above_them() {
        // Four sections and four different claims, in reading order. A heading
        // always has its own seforim under it and never somebody else's, which
        // is why the folderless mefarshim come first.
        let shelf = shelf_of(&[
            ("rashi", "רש״י"),
            ("tosafot", "תוספות"),
            ("pnei", "פני יהושע"),
            ("loose", "בודד"),
            ("beit-yosef", "בית יוסף"),
            ("declared-nowhere", "מוצהר"),
        ]);
        let mut folders = Folders {
            tree: vec![
                folder("ראשונים", "ראשונים", 2),
                folder("אחרונים", "אחרונים", 1),
            ],
            ..Folders::default()
        };
        for (slug, shelf_key) in [
            ("rashi", "ראשונים"),
            ("tosafot", "ראשונים"),
            ("pnei", "אחרונים"),
        ] {
            folders.of.insert(slug.to_string(), shelf_key.to_string());
        }
        let rows = listed(
            &[
                companion("rashi", true, 3139),
                companion("tosafot", true, 812),
                companion("pnei", true, 40),
                companion("loose", true, 5),
                companion("beit-yosef", false, 815),
                companion("declared-nowhere", true, 0),
            ],
            &[
                "rashi".to_string(),
                "tosafot".to_string(),
                "pnei".to_string(),
                "loose".to_string(),
            ],
            &[],
            &folders,
            &[],
            &shelf,
            crate::session::Language::Hebrew,
        );
        assert_eq!(
            shape(&rows),
            [
                "loose",
                "# ראשונים",
                "rashi",
                "tosafot",
                "# אחרונים",
                "pnei",
                // A declared commentary the graph cannot place on any line:
                // offered, under a heading saying why it has no tick-box.
                "# פירושים בלי מקום בשורה",
                "declared-nowhere",
                // And a sefer that merely shares links, said as that.
                "# ספרים מקושרים",
                "beit-yosef",
            ]
        );
        assert_eq!(
            shape(&rows).iter().filter(|r| !r.starts_with("# ")).count(),
            6,
            "grouping the list lost somebody"
        );
    }

    #[test]
    fn a_folder_with_nothing_in_it_is_not_drawn() {
        // The heading is the claim, and the claim has to be true of what is
        // under it.
        let shelf = shelf_of(&[("rashi", "רש״י")]);
        let rows = listed(
            &[companion("rashi", true, 1)],
            &[],
            &[],
            &Folders {
                tree: vec![folder("ריק", "ריק", 0)],
                ..Folders::default()
            },
            &[],
            &shelf,
            crate::session::Language::Hebrew,
        );
        assert_eq!(shape(&rows), ["# פירושים בלי מקום בשורה", "rashi"]);
    }

    #[test]
    fn the_seforim_that_follow_this_ones_order_are_their_own_section() {
        // `על סדר הספר` — directly under the mefarshim, because they tick and
        // mark exactly like one and are the second thing a person reaching for
        // a mefaresh wants. Not down with the merely-linked, which is where a
        // bool had quietly filed them.
        let shelf = shelf_of(&[("rashi", "רש״י"), ("arukh", "ערוך השולחן")]);
        let rows = listed(
            &[companion("rashi", true, 3139)],
            &["rashi".to_string()],
            &["arukh".to_string()],
            &Folders::default(),
            &[],
            &shelf,
            crate::session::Language::Hebrew,
        );
        assert_eq!(shape(&rows), ["rashi", "# על סדר הספר", "arukh"]);
    }

    #[test]
    fn with_nothing_read_from_the_graph_no_row_is_tickable() {
        let shelf = shelf_of(&[("rashi", "רש״י"), ("tosafot", "תוספות")]);
        let rows = listed(
            &[
                companion("rashi", true, 3139),
                companion("tosafot", true, 812),
            ],
            &[],
            &[],
            &Folders::default(),
            &[],
            &shelf,
            crate::session::Language::Hebrew,
        );
        assert!(
            !rows.iter().any(|row| matches!(
                row,
                Listed::Sefer { choice } if choice.tickable
            )),
            "a tick-box with no graph behind it"
        );
    }
}
