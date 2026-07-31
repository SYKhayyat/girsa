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
use girsa_corpus::taxonomy::rank_of;
use girsa_corpus::work::Work;
use girsa_link::{inbound, EdgeType};
use serde::Serialize;

use crate::arrangement::Arrangement;
use crate::taxonomy::{shelf_key_of, Branch};

/// Which works comment on which segment of one sefer.
#[derive(Debug, Default, Clone)]
pub struct Marks {
    by_segment: BTreeMap<String, Vec<Spoken>>,
    /// Every work that comments anywhere in this sefer, so the chooser can be
    /// drawn from the same read.
    works: BTreeSet<String>,
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
        for edge in inbound::read_back(shelf.root(), slug)? {
            if edge.edge_type != EdgeType::CommentsOn {
                continue;
            }
            let from = edge.from.from.work().to_string();
            // The far end is the commentary; this end is us. Which is only true
            // because `girsa_link::orient` made it true — before that, half of
            // these edges pointed the other way and this whole module would have
            // indexed the sefer as a commentary on its own mefarshim.
            if from == slug {
                continue;
            }
            // …and being at the far end of a `comments-on` edge does not make a
            // sefer a mefaresh on this one. Tur has commentary edges landing in
            // it from forty works and four commentaries; the first version of
            // this module offered all forty, so Rashi on Berakhot was a mefaresh
            // on the Tur and Shabbos one on the Shulchan Arukh. That is inferring
            // a relationship between two seforim from the existence of an edge,
            // which is BUILDER.md rule 6, and asking `is_commentary_on` is the
            // whole of not doing it.
            let is_mefaresh = shelf
                .work(&from)
                .is_some_and(|w| girsa_corpus::taxonomy::is_commentary_on(w, base));
            if !is_mefaresh {
                marks.refused.insert(from);
                continue;
            }
            marks.works.insert(from.clone());
            let spoken = Spoken {
                work: from,
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

    /// The works that link here as commentary and are not commentaries on this
    /// sefer. For a test and for a diagnostic, never for the list.
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

    /// The segments to put a marker on: those where a chosen mefaresh speaks.
    ///
    /// Nothing chosen marks nothing. A marker on every line is not a marker.
    #[must_use]
    pub fn marked(&self, chosen: &[String]) -> BTreeSet<String> {
        if chosen.is_empty() {
            return BTreeSet::new();
        }
        let want: BTreeSet<&str> = chosen.iter().map(String::as_str).collect();
        self.by_segment
            .iter()
            .filter(|(_, said)| said.iter().any(|s| want.contains(s.work.as_str())))
            .map(|(id, _)| id.clone())
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
pub fn folders(works: &[Work], arrangement: &Arrangement) -> Folders {
    let paths: Vec<(String, Vec<String>)> = works
        .iter()
        .map(|work| {
            let key = shelf_key_of(work, arrangement);
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
    out.tree = group(&here, &[], arrangement, &mut out.of);
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
            return group(&held, &deeper, arrangement, placed);
        }
    }

    let mut out: Vec<Branch> = buckets
        .into_iter()
        .filter_map(|(name, held)| folder(name, &held, prefix, arrangement, placed))
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
    placed: &mut BTreeMap<String, String>,
) -> Option<Branch> {
    if held.len() < 2 {
        return None;
    }
    let mut path = prefix.to_vec();
    path.push(name.to_string());
    let key = path.join("/");
    let children = group(held, &path, arrangement, placed);
    // Whoever the children did not take stands in this folder itself.
    let mut mine = 0;
    for (slug, _) in held {
        if !placed.contains_key(*slug) {
            placed.insert((*slug).to_string(), key.clone());
            mine += 1;
        }
    }
    Some(Branch {
        title: arrangement.title_of(&key),
        here: mine,
        count: held.len(),
        mine: arrangement.made.contains(&key),
        edited: arrangement.titles.contains_key(&key) || arrangement.shelves.contains_key(&key),
        children,
        key,
    })
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

    fn corpus() -> Option<std::path::PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
        (root.join("links").is_dir() && inbound::built(&root)).then_some(root)
    }

    /// The real shelf, with no personal layer over it.
    fn real_shelf(root: &Path) -> crate::shelf::Shelf {
        crate::shelf::Shelf::open(root, &root.join("no-personal-layer")).expect("the shelf opens")
    }

    macro_rules! corpus_or_skip {
        () => {
            match corpus() {
                Some(root) => root,
                None => {
                    eprintln!("skipped: no imported link graph");
                    return;
                }
            }
        };
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
        assert!(marked.contains("girsa:bavli/berakhot/2a:1#1"));
        assert!(marked.contains("girsa:bavli/berakhot/2a:2#2"));
        assert!(
            !marked.contains("girsa:bavli/berakhot/2a:3#3"),
            "the Meiri's line is not Rashi's"
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
    fn rashi_is_on_the_first_line_of_the_daf_in_the_real_corpus() {
        // The one assertion here that reads the shelf, and the point of it is
        // that it goes red if the link graph regresses the way W32 found it:
        // before `girsa_link::orient`, this edge pointed from the daf to Rashi
        // and `inbound.jsonl` filed it under Rashi, so this map would have been
        // empty for every masechta.
        let root = corpus_or_skip!();
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
        let folders = folders(&works, &Arrangement::default());

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
        let folders = folders(&works, &Arrangement::default());
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
        let folders = folders(&works, &Arrangement::default());
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
        let folders = folders(&works, &Arrangement::default());
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
        let folders = folders(&works, &Arrangement::default());
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
        let onto = crate::taxonomy::shelf_key_of(&works[0], &moved);
        moved
            .works
            .insert("bavli/ben-yehoyada".to_string(), onto.clone());

        let folders = folders(&works, &moved);
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
    fn the_mefarshim_of_a_real_masechta_come_back_in_their_folders() {
        let root = corpus_or_skip!();
        let shelf = real_shelf(&root);
        let marks = Marks::of(&shelf, BERAKHOT).expect("reads");
        let on: Vec<girsa_corpus::work::Work> = marks
            .commentators()
            .iter()
            .filter_map(|slug| shelf.work(slug).cloned())
            .collect();
        let folders = folders(&on, &Arrangement::default());

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
    fn the_sefer_is_not_a_commentary_on_its_own_mefarshim() {
        // Self-edges are dropped. Reading a sefer whose shard still held a
        // reversed edge would otherwise mark every line with the sefer's own
        // name.
        let root = corpus_or_skip!();
        let marks = Marks::of(&real_shelf(&root), BERAKHOT).expect("reads");
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
        let root = corpus_or_skip!();
        let shelf = real_shelf(&root);
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
                assert!(
                    girsa_corpus::taxonomy::is_commentary_on(work, base),
                    "{slug} offers {offered} as a mefaresh and it is not a commentary on it"
                );
            }
            println!(
                "{slug}: {} mefarshim, {} works refused",
                marks.commentators().len(),
                marks.refused().len()
            );
        }
    }

    #[test]
    fn the_seforim_the_reader_named_are_gone_and_the_real_ones_are_not() {
        // The three the reader found, as a test, on the real graph. Named because
        // an assertion about *counts* would go green again the day the counts
        // change for some other reason.
        let root = corpus_or_skip!();
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
}
