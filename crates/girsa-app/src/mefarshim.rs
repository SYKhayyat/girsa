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
use std::path::Path;

use girsa_corpus::segment::SegmentId;
use girsa_link::{inbound, EdgeType};
use serde::Serialize;

/// Which works comment on which segment of one sefer.
#[derive(Debug, Default, Clone)]
pub struct Marks {
    by_segment: BTreeMap<String, BTreeSet<String>>,
    /// Every work that comments anywhere in this sefer, so the chooser can be
    /// drawn from the same read.
    works: BTreeSet<String>,
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
    pub fn of(root: &Path, slug: &str) -> Result<Self, std::io::Error> {
        let mut marks = Self::default();
        for edge in inbound::read_back(root, slug)? {
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
            marks.works.insert(from.clone());
            // A span covers a run of segments and every one of them is spoken
            // about, not just the first.
            marks
                .by_segment
                .entry(edge.to.from.to_string())
                .or_default()
                .insert(from.clone());
            if let Some(last) = &edge.to.to {
                marks
                    .by_segment
                    .entry(last.to_string())
                    .or_default()
                    .insert(from);
            }
        }
        Ok(marks)
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

    /// What the chosen mefarshim say about one line.
    #[must_use]
    pub fn on(&self, at: &SegmentId, chosen: &[String]) -> Here {
        let here = self.by_segment.get(&at.to_string());
        Here {
            works: chosen
                .iter()
                .filter(|w| here.is_some_and(|set| set.contains(*w)))
                .cloned()
                .collect(),
            any: here.is_some_and(|set| !set.is_empty()),
        }
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
            .filter(|(_, set)| set.iter().any(|w| want.contains(w.as_str())))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    const BERAKHOT: &str = "bavli/berakhot";
    const RASHI: &str = "bavli/rashi-on-berakhot";
    const TOSAFOT: &str = "bavli/tosafot-on-berakhot";

    fn corpus() -> Option<std::path::PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
        (root.join("links").is_dir() && inbound::built(&root)).then_some(root)
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
    fn made_up() -> Marks {
        let mut marks = Marks::default();
        for (seg, works) in [
            ("girsa:bavli/berakhot/2a:1#1", vec![RASHI, TOSAFOT]),
            ("girsa:bavli/berakhot/2a:2#2", vec![RASHI]),
            (
                "girsa:bavli/berakhot/2a:3#3",
                vec!["bavli/meiri-on-berakhot"],
            ),
        ] {
            // Keyed through `SegmentId` rather than by the literal, so the
            // fixture cannot disagree with `Marks::of` about whether an id
            // carries its `girsa:` scheme. The first draft of this did, and the
            // three filtering tests all failed while the corpus one passed.
            marks.by_segment.insert(
                id(seg).to_string(),
                works.iter().map(|w| (*w).to_string()).collect(),
            );
            for w in works {
                marks.works.insert(w.to_string());
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
    fn rashi_is_on_the_first_line_of_the_daf_in_the_real_corpus() {
        // The one assertion here that reads the shelf, and the point of it is
        // that it goes red if the link graph regresses the way W32 found it:
        // before `girsa_link::orient`, this edge pointed from the daf to Rashi
        // and `inbound.jsonl` filed it under Rashi, so this map would have been
        // empty for every masechta.
        let root = corpus_or_skip!();
        let marks = Marks::of(&root, BERAKHOT).expect("berakhot's inbound reads");

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

    #[test]
    fn the_sefer_is_not_a_commentary_on_its_own_mefarshim() {
        // Self-edges are dropped. Reading a sefer whose shard still held a
        // reversed edge would otherwise mark every line with the sefer's own
        // name.
        let root = corpus_or_skip!();
        let marks = Marks::of(&root, BERAKHOT).expect("reads");
        assert!(
            !marks.commentators().iter().any(|w| w == BERAKHOT),
            "Berakhot lists itself as one of its own mefarshim"
        );
    }
}
