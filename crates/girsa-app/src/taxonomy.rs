//! The shelf as the reader has it: the shipped taxonomy, plus their edits.
//!
//! The shipped half — *which shelf would the corpus file this sefer on* — is
//! [`girsa_corpus::taxonomy`], because it is a function of a work and nothing
//! else, and because the **search facets** (spec.md §9.8) have to group results
//! by the same shelf this bookcase browses by. Two mappings would put a sefer
//! on one shelf here and another there, and nothing would say which was wrong.
//!
//! What is here is everything that needs the personal layer: where the reader
//! moved a sefer to, what they renamed a shelf, what order they put things in,
//! and the tree that comes out of all of it (spec.md §5 — *the shipped taxonomy
//! is a default, not a fact*).
//!
//! Nothing here is allowed to lose a sefer. Every work is under exactly one
//! branch, and [`Branch::count`] over the roots has to come to the number of
//! works — which `every_sefer_has_a_shelf` asserts against the real corpus.

use std::collections::{BTreeMap, BTreeSet};

use girsa_corpus::taxonomy::{rank_of, shelf_of, top_rank_of};
use girsa_corpus::work::Work;
use serde::Serialize;

use crate::arrangement::{self, Arrangement};

/// One shelf, and everything under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Branch {
    /// What every edit names this shelf by. Does not change when the shelf is
    /// moved or renamed — see [`crate::arrangement`].
    pub key: String,
    /// What it is called on the page.
    pub title: String,
    /// Seforim standing on this shelf itself.
    pub here: usize,
    /// Seforim on it and on everything under it.
    pub count: usize,
    /// The reader made this shelf.
    pub mine: bool,
    /// It is not where, or not what, it shipped as. Shown, because a reader
    /// looking at a shelf that disagrees with a friend's copy should be able to
    /// see that it was them who moved it.
    pub edited: bool,
    pub children: Vec<Branch>,
    /// This is not a shelf: it is the seforim standing on its parent, gathered
    /// so that a level is all folders or all seforim (W42).
    ///
    /// > *"i dont like to have folders and files. all files should be put in an
    /// > other folder if needed."*
    ///
    /// Its `key` is its **parent's**, deliberately — so `works_on` finds exactly
    /// the loose seforim without a second idea of what a shelf key means, and so
    /// dropping a sefer on it puts that sefer where it already looks like it is.
    /// What it must not do is be renamed or moved, because it is not a thing the
    /// reader made; that is what this flag is for.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub loose: bool,
}

/// The shelf a work is on now: the shipped one, unless it was moved.
#[must_use]
pub fn shelf_key_of(work: &Work, arrangement: &Arrangement) -> String {
    arrangement
        .works
        .get(&work.slug)
        .cloned()
        .unwrap_or_else(|| shelf_of(work).join("/"))
}

/// The whole shelf, as a reader browses it.
///
/// Every work is under exactly one branch — the counts are the check on that,
/// and [`Branch::count`] over the roots has to come to the number of works.
#[must_use]
pub fn tree(works: &[Work], arrangement: &Arrangement) -> Vec<Branch> {
    let mut here: BTreeMap<String, usize> = BTreeMap::new();
    let mut keys: BTreeSet<String> = BTreeSet::new();

    for work in works {
        let key = shelf_key_of(work, arrangement);
        *here.entry(key.clone()).or_default() += 1;
        keys.insert(key);
    }
    // Every shelf anybody has named, whether or not a sefer stands on it: a
    // shelf the reader made this minute is empty and is still a shelf.
    for key in arrangement
        .made
        .iter()
        .chain(arrangement.shelves.keys())
        .chain(arrangement.titles.keys())
        .chain(arrangement.works.values())
    {
        keys.insert(key.clone());
    }

    // And every shelf that has to exist for those to hang on. `תלמוד/בבלי`
    // implies `תלמוד` without anybody writing it down.
    let mut ancestors = BTreeSet::new();
    for key in &keys {
        let mut walk = key.clone();
        while let Some(up) = arrangement.parent_of(&walk) {
            if !ancestors.insert(up.clone()) || up == walk {
                break;
            }
            walk = up;
        }
    }
    keys.extend(ancestors);

    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut roots: Vec<String> = Vec::new();
    for key in &keys {
        // A file edited by hand can say `a` hangs under `b` and `b` under `a`.
        // Neither would be reachable from any root, and the seforim on them
        // would be gone from the shelf without anything saying so — so a shelf
        // in a loop is stood at the top instead.
        match arrangement.parent_of(key) {
            Some(parent) if parent != *key && !hangs_under(arrangement, &parent, key) => {
                children.entry(parent).or_default().push(key.clone());
            }
            _ => roots.push(key.clone()),
        }
    }

    let mut out: Vec<Branch> = roots
        .iter()
        .map(|key| branch(key, arrangement, &here, &children, 0))
        .collect();
    out.sort_by(|a, b| {
        ordered(arrangement, arrangement::TOP, &a.key, &b.key)
            .then_with(|| top_rank_of(&a.key).cmp(&top_rank_of(&b.key)))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.title.cmp(&b.title))
    });
    out
}

/// Whether `key` is somewhere above `shelf` — which is what makes putting
/// `shelf` under `key` a loop.
fn hangs_under(arrangement: &Arrangement, key: &str, shelf: &str) -> bool {
    let mut walk = key.to_string();
    let mut seen = BTreeSet::new();
    while let Some(up) = arrangement.parent_of(&walk) {
        if up == shelf {
            return true;
        }
        if !seen.insert(up.clone()) {
            return true;
        }
        walk = up;
    }
    false
}

fn branch(
    key: &str,
    arrangement: &Arrangement,
    here: &BTreeMap<String, usize>,
    children: &BTreeMap<String, Vec<String>>,
    depth: usize,
) -> Branch {
    let mut kids: Vec<Branch> = if depth > 64 {
        Vec::new()
    } else {
        children
            .get(key)
            .into_iter()
            .flatten()
            .map(|child| branch(child, arrangement, here, children, depth + 1))
            .collect()
    };
    kids.sort_by(|a, b| {
        ordered(arrangement, key, &a.key, &b.key)
            .then_with(|| rank_of(&a.title).cmp(&rank_of(&b.title)))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.title.cmp(&b.title))
    });

    let here_count = here.get(key).copied().unwrap_or_default();
    // Counted before anything is gathered, and used as the total afterwards.
    // Summing the children *after* the gather would count the gathered seforim
    // once and the ones it came from not at all.
    let total = here_count + kids.iter().map(|k| k.count).sum::<usize>();

    // W42. A level with both folders and seforim on it makes a reader scan two
    // kinds of row for one thing, so the seforim are gathered into a child and
    // the level becomes all folders.
    //
    // First, ahead of the shelves: on שולחן ערוך this is the four chalakim and
    // the introduction, and the mefarshim are the sibling. The sefer comes before
    // the commentaries on it.
    let mut here_count = here_count;
    if here_count > 0 && !kids.is_empty() {
        kids.insert(
            0,
            Branch {
                key: key.to_string(),
                title: SEFORIM.to_string(),
                here: here_count,
                count: here_count,
                mine: false,
                edited: false,
                children: Vec::new(),
                loose: true,
            },
        );
        here_count = 0;
    }

    Branch {
        key: key.to_string(),
        title: arrangement.title_of(key),
        here: here_count,
        count: total,
        mine: arrangement.made.contains(key),
        edited: arrangement.titles.contains_key(key) || arrangement.shelves.contains_key(key),
        children: kids,
        loose: false,
    }
}

/// What the gathered-seforim child is called.
///
/// Not `אחר` — that is the top-level catch-all for a category nobody has mapped,
/// and a reader who saw the same word in both places would reasonably think the
/// seforim in front of them were unfiled.
const SEFORIM: &str = "ספרים";

/// Where two shelves sit relative to each other in an order the reader set.
///
/// A shelf the reader did not place sorts after every shelf they did, in the
/// shipped order — so pinning one shelf to the front does not shuffle the rest.
fn ordered(arrangement: &Arrangement, parent: &str, a: &str, b: &str) -> std::cmp::Ordering {
    let placed = |key: &str| {
        arrangement
            .order
            .get(parent)
            .and_then(|order| order.iter().position(|k| k == key))
            .unwrap_or(usize::MAX)
    };
    placed(a).cmp(&placed(b))
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn work_on(slug: &str, categories: &[&str]) -> Work {
        let mut work = crate::shelf::tests::work(slug);
        work.categories = categories.iter().map(|c| (*c).to_string()).collect();
        work
    }

    fn find<'a>(branches: &'a [Branch], title: &str) -> Option<&'a Branch> {
        branches.iter().find_map(|b| {
            if b.title == title {
                Some(b)
            } else {
                find(&b.children, title)
            }
        })
    }

    // ── W42: a level is all folders or all seforim ───────────────────────────
    //
    // > *"i dont like to have folders and files. all files should be put in an
    // > other folder if needed."*

    #[test]
    fn a_shelf_with_both_folders_and_seforim_gathers_the_seforim_into_a_child() {
        // The Shulchan Arukh shelf, as the corpus really has it: four chalakim
        // and an introduction standing on it, and sixty-eight commentaries under
        // a folder. Two kinds of row for one kind of thing.
        let works = vec![
            work_on(
                "shulchan-arukh/orach-chayim",
                &["Halakhah", "Shulchan Arukh"],
            ),
            work_on("shulchan-arukh/yoreh-deah", &["Halakhah", "Shulchan Arukh"]),
            work_on(
                "magen-avraham",
                &["Halakhah", "Shulchan Arukh", "Commentary"],
            ),
        ];
        let tree = tree(&works, &Arrangement::default());
        let arukh = find(&tree, "שולחן ערוך").expect("the shelf is there");

        assert_eq!(arukh.here, 0, "nothing stands loose beside a folder");
        assert_eq!(arukh.count, 3, "and nothing was lost doing that");
        assert_eq!(arukh.children.len(), 2, "the seforim, and the mefarshim");

        // First, because the sefer comes before the commentaries on it.
        let gathered = &arukh.children[0];
        assert!(gathered.loose);
        assert_eq!(gathered.count, 2);
        assert_eq!(
            gathered.key, arukh.key,
            "its key is its parent's, so `works_on` finds exactly the loose ones"
        );
    }

    #[test]
    fn a_shelf_of_only_seforim_is_left_alone() {
        // Most shelves. A folder called `ספרים` holding everything, with nothing
        // beside it, is a click in front of a list.
        let works = vec![
            work_on("bavli/berakhot", &["Talmud", "Bavli", "Seder Zeraim"]),
            work_on("bavli/shabbat", &["Talmud", "Bavli", "Seder Moed"]),
        ];
        let tree = tree(&works, &Arrangement::default());
        let zeraim = find(&tree, "סדר זרעים").expect("the seder is there");
        assert_eq!(zeraim.here, 1);
        assert!(zeraim.children.is_empty());
        assert!(!zeraim.loose);
    }

    #[test]
    fn a_shelf_of_only_folders_is_left_alone() {
        let works = vec![
            work_on("bavli/berakhot", &["Talmud", "Bavli", "Seder Zeraim"]),
            work_on("bavli/shabbat", &["Talmud", "Bavli", "Seder Moed"]),
        ];
        let tree = tree(&works, &Arrangement::default());
        let bavli = find(&tree, "בבלי").expect("the bavli is there");
        assert_eq!(bavli.here, 0);
        assert_eq!(bavli.children.len(), 2, "two sedarim and no gathered child");
        assert!(!bavli.children.iter().any(|b| b.loose));
    }

    #[test]
    fn gathering_the_loose_seforim_loses_none_of_them() {
        // The rule the whole module turns on, and the one W42 could break: the
        // counts over the roots still come to the number of works.
        let works = vec![
            work_on(
                "shulchan-arukh/orach-chayim",
                &["Halakhah", "Shulchan Arukh"],
            ),
            work_on(
                "magen-avraham",
                &["Halakhah", "Shulchan Arukh", "Commentary"],
            ),
            work_on("bavli/berakhot", &["Talmud", "Bavli", "Seder Zeraim"]),
            work_on("nothing-says", &[]),
        ];
        let tree = tree(&works, &Arrangement::default());
        let counted: usize = tree.iter().map(|b| b.count).sum();
        assert_eq!(counted, works.len());
    }

    #[test]
    fn the_gathered_child_is_not_something_the_reader_made() {
        // So the window can refuse to rename or move it. A folder that renames
        // its own parent is worse than no folder.
        let works = vec![
            work_on(
                "shulchan-arukh/orach-chayim",
                &["Halakhah", "Shulchan Arukh"],
            ),
            work_on(
                "magen-avraham",
                &["Halakhah", "Shulchan Arukh", "Commentary"],
            ),
        ];
        let tree = tree(&works, &Arrangement::default());
        let arukh = find(&tree, "שולחן ערוך").expect("the shelf is there");
        let gathered = &arukh.children[0];
        assert!(gathered.loose);
        assert!(!gathered.mine, "the reader did not make it");
        assert!(!gathered.edited, "and has not edited it");
    }
}
