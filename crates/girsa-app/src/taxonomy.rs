//! The shelf as the reader has it: the shipped taxonomy, plus their edits.
//!
//! The shipped half — *which shelf would the corpus file this sefer on* — is
//! [`girsa_corpus::taxonomy`], because it is a function of the catalogue and
//! nothing else, and because the **search facets** (spec.md §9.8) have to group
//! results by the same shelf this bookcase browses by. Two mappings would put a
//! sefer on one shelf here and another there, and nothing would say which was
//! wrong. [`Shipped`] is that answer, worked out once.
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

use girsa_corpus::taxonomy::{rank_of, shelves_of, top_rank_of};
use girsa_corpus::work::Work;
use serde::Serialize;

use crate::arrangement::{self, Arrangement};

/// Where the corpus would file each sefer, worked out once for the catalogue.
///
/// # Why this is a value and not a call
///
/// The shipped shelf used to be `shelf_of(work)`, asked afresh every time
/// anything wanted to know where a sefer stands — once per work per `tree()`,
/// once per work per `works_on()`, once per mefaresh per `folders()`. Two things
/// made that wrong rather than merely wasteful:
///
/// - one of the rules is about the sefer's **base text** and cannot be answered
///   from the work alone (`girsa_corpus::taxonomy::shelves_of`), so asking one
///   work at a time gave the wrong shelf as well as a slow one;
/// - `canonical_path` walks two tables per call, and `works_on` did it 7,189
///   times for one click on one shelf.
#[derive(Debug, Clone, Default)]
pub struct Shipped {
    /// Sefer → the shelf key the corpus files it on.
    where_it_stands: BTreeMap<String, String>,
    /// Shelf key → the Hebrew name its own seforim give it, for the shelves the
    /// term table has no word for. See `girsa_corpus::taxonomy::hebrew_names`.
    named: BTreeMap<String, String>,
}

impl Shipped {
    /// File the whole catalogue.
    #[must_use]
    pub fn of(works: &[Work]) -> Self {
        let shelves = shelves_of(works);
        Self {
            where_it_stands: works
                .iter()
                .zip(&shelves)
                .map(|(work, shelf)| (work.slug.clone(), shelf.join("/")))
                .collect(),
            named: girsa_corpus::taxonomy::hebrew_names(works, &shelves),
        }
    }

    /// Where the corpus files this sefer. Empty for one the catalogue has never
    /// seen, which is `אחר` by the same rule that puts an unfiled sefer there.
    #[must_use]
    pub fn of_slug(&self, slug: &str) -> &str {
        self.where_it_stands.get(slug).map_or("אחר", String::as_str)
    }

    /// What this shelf's own seforim call it, for a shelf the corpus named in
    /// English and the term table has no word for.
    #[must_use]
    pub fn name_of(&self, key: &str) -> Option<&str> {
        self.named.get(key).map(String::as_str)
    }
}

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
    /// Where the corpus puts this shelf among its siblings: the earliest
    /// [`Work::order`] anywhere beneath it. Not on the wire — the window draws a
    /// shelf, it does not sort one — and it is what answers complaint 1 for
    /// shelves the way `Work::order` answered it for seforim.
    #[serde(skip)]
    pub order: Option<Vec<i32>>,
    /// This shelf holds commentary on the shelf beside it. Not on the wire, for
    /// the same reason: it is a fact about where this sorts.
    #[serde(skip)]
    pub commentary: bool,
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
pub fn shelf_key_of(work: &Work, arrangement: &Arrangement, shipped: &Shipped) -> String {
    arrangement
        .works
        .get(&work.slug)
        .cloned()
        .unwrap_or_else(|| shipped.of_slug(&work.slug).to_string())
}

/// The whole shelf, as a reader browses it.
///
/// Every work is under exactly one branch — the counts are the check on that,
/// and [`Branch::count`] over the roots has to come to the number of works.
#[must_use]
pub fn tree(works: &[Work], arrangement: &Arrangement, shipped: &Shipped) -> Vec<Branch> {
    let mut here: BTreeMap<String, usize> = BTreeMap::new();
    let mut keys: BTreeSet<String> = BTreeSet::new();
    // The corpus's order for each shelf: the earliest `Work::order` standing on
    // it. Gathered here rather than in `Shipped` so that it follows a sefer the
    // reader has **moved** — the shelf they dragged it to is the shelf its order
    // now belongs to.
    let mut orders: BTreeMap<String, Vec<i32>> = BTreeMap::new();

    for work in works {
        let key = shelf_key_of(work, arrangement, shipped);
        *here.entry(key.clone()).or_default() += 1;
        if !work.order.is_empty() {
            let earliest = orders
                .entry(key.clone())
                .or_insert_with(|| work.order.clone());
            if work.order < *earliest {
                earliest.clone_from(&work.order);
            }
        }
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
        .map(|key| branch(key, arrangement, shipped, &here, &orders, &children, 0))
        .collect();
    // The top shelves have an order of their own — `TOP`, the sixteen a bookcase
    // has — so this is unchanged. It is the *levels below* that were sorted by
    // how much was on them.
    out.sort_by(|a, b| {
        ordered(arrangement, arrangement::TOP, &a.key, &b.key)
            .then_with(|| top_rank_of(&a.key).cmp(&top_rank_of(&b.key)))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.title.cmp(&b.title))
    });
    out
}

/// Where two shelves sit relative to each other, once the reader has not said.
///
/// # Complaint 1 came back wearing new clothes
///
/// > *"seforim sorted by name, not true order."*
///
/// Answered for **works** — `Work::order`, read from Sefaria, applied through
/// one comparator — and shelves never got it. They sorted by a rank table of
/// eight names and then by **count descending**, and the six sedarim are not
/// among the eight:
///
/// ```text
/// ראשונים 641 · אחרונים 717 · מחברי זמננו 125 · Commentary on Minor Tractates 48 ·
/// גמרא נוחה 36 · מסכתות קטנות 15 · סדר מועד 11 · סדר קדשים 9 · סדר נזיקין 8 ·
/// סדר נשים 7 · Guides 5 · סדר זרעים 1 · סדר טהרות 1
/// ```
///
/// Zeraim, where ברכות lives alone, second from the bottom under an English
/// folder called *Guides* — while inside Seder Moed the masechtos were in the
/// right order, which made the folders look even more like a mistake. Lesson 4
/// of the audit: *a fix is not finished at the site of the complaint.*
///
/// Four rules, in order, and only the last is a fallback:
///
/// 1. **The sefer before the commentaries on it.** `branch()` already applies
///    this to the loose seforim it gathers out of a level; it is the same rule
///    one level up, and it is what puts the whole of Shas above the rishonim on
///    Shas.
/// 2. **The corpus's own order**, the way the works got it: the earliest
///    `Work::order` beneath a shelf. Sefaria orders the masechtos in the
///    sequence they are learned — Berakhos `[1]`, Shabbos `[2]`, Yevamos `[14]`
///    — so this recovers זרעים-מועד-נשים-נזיקין-קדשים-טהרות without anybody
///    typing the six names anywhere. A shelf the corpus ordered comes before one
///    it did not, exactly as `Work::by_order` has it.
/// 3. **The era**, for the shelves that have one: rishonim, then acharonim,
///    then our own contemporaries. Not derivable from anything — Sefaria states
///    no order on a commentary shelf — so it stays a table.
/// 4. **Size, then the title**, so the answer is stable for everything else.
fn by_the_corpus(a: &Branch, b: &Branch) -> std::cmp::Ordering {
    a.commentary
        .cmp(&b.commentary)
        .then_with(|| match (&a.order, &b.order) {
            (Some(x), Some(y)) => x.cmp(y),
            // An unordered shelf sorts after every ordered one, because an
            // unordered shelf is one the corpus said nothing about.
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
        .then_with(|| rank_of(&a.title).cmp(&rank_of(&b.title)))
        .then_with(|| b.count.cmp(&a.count))
        .then_with(|| a.title.cmp(&b.title))
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
    shipped: &Shipped,
    here: &BTreeMap<String, usize>,
    orders: &BTreeMap<String, Vec<i32>>,
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
            .map(|child| {
                branch(
                    child,
                    arrangement,
                    shipped,
                    here,
                    orders,
                    children,
                    depth + 1,
                )
            })
            .collect()
    };
    kids.sort_by(|a, b| {
        ordered(arrangement, key, &a.key, &b.key).then_with(|| by_the_corpus(a, b))
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
                order: orders.get(key).cloned(),
                commentary: false,
                loose: true,
            },
        );
        here_count = 0;
    }

    // The earliest order anywhere beneath, so a shelf of shelves inherits the
    // sequence of what is under it — `סדר זרעים` has ברכות `[1]` standing on it
    // and `תלמוד/בבלי` has the whole of Shas beneath it.
    let order = std::iter::once(orders.get(key).cloned())
        .chain(kids.iter().map(|kid| kid.order.clone()))
        .flatten()
        .min();

    Branch {
        key: key.to_string(),
        title: shelf_title(key, arrangement, shipped),
        here: here_count,
        count: total,
        mine: arrangement.made.contains(key),
        edited: arrangement.titles.contains_key(key) || arrangement.shelves.contains_key(key),
        children: kids,
        order,
        commentary: girsa_corpus::taxonomy::is_commentary_shelf(key),
        loose: false,
    }
}

/// What a shelf is called, **wherever** a shelf is drawn.
///
/// Three sources in one order, and the order is the argument:
///
/// 1. what the reader renamed it to, which nothing may override;
/// 2. the Hebrew name its own seforim give it, for the shelves the term table
///    has no word for — `girsa_corpus::taxonomy::hebrew_names`, and finding 6's
///    second half: a Hebrew bookcase carrying `Chida` and `Mechokekei Yehudah`
///    among its shelves;
/// 3. the last segment of the key, which is a Sefaria category name in English
///    and is the answer of last resort.
///
/// # Why it is a function
///
/// Because it was three lines inside `branch`, and the mefarshim list drew its
/// folders with step 3 alone. So the bookcase said `רי״ף` and the chooser said
/// `Rif · 4`, between `ראשונים · 13` and `מפרשים · 3`, in the same window — two
/// places naming the same shelves and nothing making them agree, which is the
/// pattern this audit's Part 4 names.
#[must_use]
pub fn shelf_title(key: &str, arrangement: &Arrangement, shipped: &Shipped) -> String {
    arrangement
        .named_title_of(key)
        .or_else(|| shipped.name_of(key).map(str::to_string))
        .unwrap_or_else(|| arrangement.title_of(key))
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
        let tree = tree(&works, &Arrangement::default(), &Shipped::of(&works));
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
        let tree = tree(&works, &Arrangement::default(), &Shipped::of(&works));
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
        let tree = tree(&works, &Arrangement::default(), &Shipped::of(&works));
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
        let tree = tree(&works, &Arrangement::default(), &Shipped::of(&works));
        let counted: usize = tree.iter().map(|b| b.count).sum();
        assert_eq!(counted, works.len());
    }

    // ── finding 6 · complaint 1, answered for the shelves this time ─────────

    /// A work with categories **and** the corpus's order for it.
    fn ordered_work(slug: &str, categories: &[&str], order: &[i32]) -> Work {
        let mut work = work_on(slug, categories);
        work.order = order.to_vec();
        work
    }

    #[test]
    fn the_sedarim_are_in_the_order_they_are_learned_and_not_in_size_order() {
        // What a reader saw opening Shas: `ראשונים 641 · אחרונים 717 · … ·
        // סדר מועד 11 · … · סדר זרעים 1 · סדר טהרות 1`. Zeraim, where ברכות
        // lives alone, second from the bottom — because a shelf sorted by a
        // rank table of eight names and then by count descending, and the six
        // sedarim are not among the eight.
        //
        // Sefaria orders the masechtos in the sequence they are learned, so the
        // earliest order beneath each seder is the seder's own place. Nothing
        // here names a seder.
        let works = vec![
            ordered_work("bavli/berakhot", &["Talmud", "Bavli", "Seder Zeraim"], &[1]),
            ordered_work("bavli/shabbat", &["Talmud", "Bavli", "Seder Moed"], &[2]),
            ordered_work("bavli/eruvin", &["Talmud", "Bavli", "Seder Moed"], &[3]),
            ordered_work("bavli/yevamot", &["Talmud", "Bavli", "Seder Nashim"], &[14]),
            ordered_work(
                "bavli/bava-kamma",
                &["Talmud", "Bavli", "Seder Nezikin"],
                &[21],
            ),
            ordered_work(
                "bavli/zevachim",
                &["Talmud", "Bavli", "Seder Kodashim"],
                &[28],
            ),
            ordered_work("bavli/niddah", &["Talmud", "Bavli", "Seder Tahorot"], &[37]),
        ];
        let tree = tree(&works, &Arrangement::default(), &Shipped::of(&works));
        let bavli = find(&tree, "בבלי").expect("the bavli is there");
        let sedarim: Vec<&str> = bavli.children.iter().map(|b| b.title.as_str()).collect();
        assert_eq!(
            sedarim,
            vec![
                "סדר זרעים",
                "סדר מועד",
                "סדר נשים",
                "סדר נזיקין",
                "סדר קדשים",
                "סדר טהרות",
            ],
            "and Moed, with two masechtos on it, does not overtake Zeraim with one"
        );
    }

    #[test]
    fn the_gemara_comes_before_the_rishonim_on_it() {
        // Rule 1 of `by_the_corpus`, and the same rule `branch()` already
        // applies to the loose seforim it gathers: the sefer comes before the
        // commentaries on it. Without it the rishonim inherit their base's
        // order — the importer gives a commentary its base's order when it has
        // none of its own — and 641 of them sort above the one masechta they
        // are written on.
        let works = vec![
            ordered_work("bavli/berakhot", &["Talmud", "Bavli", "Seder Zeraim"], &[1]),
            ordered_work(
                "bavli/rashi-on-berakhot",
                &["Talmud", "Bavli", "Rishonim on Talmud"],
                &[1],
            ),
            ordered_work(
                "bavli/pnei-yehoshua",
                &["Talmud", "Bavli", "Acharonim on Talmud"],
                &[1],
            ),
        ];
        let tree = tree(&works, &Arrangement::default(), &Shipped::of(&works));
        let bavli = find(&tree, "בבלי").expect("the bavli is there");
        let shelves: Vec<&str> = bavli.children.iter().map(|b| b.title.as_str()).collect();
        assert_eq!(shelves, vec!["סדר זרעים", "ראשונים", "אחרונים"]);
    }

    #[test]
    fn a_shelf_the_corpus_named_in_english_is_named_by_its_own_seforim() {
        // `Bartenura` is not a word anybody could put in a translation table —
        // it is the name of a sefer, and the corpus already knows it in Hebrew,
        // sixty-three times over.
        let mut one = work_on(
            "bartenura-on-berakhot",
            &["Mishnah", "Rishonim on Mishnah", "Bartenura"],
        );
        one.he_title = "ברטנורא על ברכות".into();
        let mut two = work_on(
            "bartenura-on-shabbat",
            &["Mishnah", "Rishonim on Mishnah", "Bartenura"],
        );
        two.he_title = "ברטנורא על שבת".into();
        let works = vec![one, two];

        let tree = tree(&works, &Arrangement::default(), &Shipped::of(&works));
        assert!(
            find(&tree, "ברטנורא").is_some(),
            "named off its own seforim"
        );
        assert!(
            find(&tree, "Bartenura").is_none(),
            "and not left in English"
        );
    }

    #[test]
    fn a_shelf_of_several_of_one_mans_works_is_named_after_the_man() {
        // The Chida's shelf holds `Chomat Anakh`, `Nachal Eshkol`, `Marit
        // HaAyin` — no shared title stem, and all forty-five carry the same
        // author. A shelf named after a man takes his name.
        let mut one = work_on(
            "chomat-anakh-on-isaiah",
            &["Tanakh", "Acharonim on Tanakh", "Chida"],
        );
        one.he_title = "חומת אנך על ישעיהו".into();
        one.author = Some("חיים דוד אזולאי".into());
        let mut two = work_on(
            "nachal-eshkol-on-ruth",
            &["Tanakh", "Acharonim on Tanakh", "Chida"],
        );
        two.he_title = "נחל אשכול על רות".into();
        two.author = Some("חיים דוד אזולאי".into());
        let works = vec![one, two];

        let tree = tree(&works, &Arrangement::default(), &Shipped::of(&works));
        assert!(find(&tree, "חיים דוד אזולאי").is_some());
        assert!(find(&tree, "Chida").is_none());
    }

    #[test]
    fn a_rename_by_the_reader_beats_the_name_the_corpus_gives() {
        // spec.md §5: the shipped taxonomy is a default, not a fact. Naming a
        // shelf off its seforim must not undo a drag.
        let mut one = work_on(
            "bartenura-on-berakhot",
            &["Mishnah", "Rishonim on Mishnah", "Bartenura"],
        );
        one.he_title = "ברטנורא על ברכות".into();
        let works = vec![one];
        let mut arrangement = Arrangement::default();
        arrangement
            .titles
            .insert("משנה/ראשונים/Bartenura".to_string(), "רע״ב".to_string());

        let tree = tree(&works, &arrangement, &Shipped::of(&works));
        assert!(find(&tree, "רע״ב").is_some(), "the reader's name stands");
        assert!(find(&tree, "ברטנורא").is_none());
    }

    // ── the whole bookcase, against the shipped catalogue ───────────────────

    #[test]
    #[ignore = "needs the fetched corpus: cargo test -p girsa-app --lib -- --ignored"]
    fn every_shelf_in_the_bookcase_is_named_in_hebrew() {
        // The measurement finding 6 rests on, kept where it can go red. The
        // shipped catalogue has 533 distinct categories and 376 of them carry
        // no Hebrew letter; thirty translations against that is a Hebrew
        // bookcase with `Commentary on Minor Tractates`, `Guides`, `Rif` and
        // `Sefer Zemanim` among its shelves.
        //
        // What names them is, in order: the `X on Y` split, `TERM`, the
        // seforim's own titles, and their one author. This asserts the result
        // rather than any of the four, so a corpus that grows a category nobody
        // has named turns this red instead of quietly putting English on the
        // shelf.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
        assert!(
            root.join("works").is_dir(),
            "no corpus at {} — run girsa-fetch and girsa-import. This check is \
             #[ignore]d precisely so that its absence is never read as a pass.",
            root.display()
        );
        let shelf = crate::shelf::Shelf::open(&root, &root.join("no-personal-layer"))
            .expect("the shelf opens");
        let works = shelf.works();
        assert!(works.len() > 5_000, "only {} works read", works.len());

        let tree = tree(works, &Arrangement::default(), &Shipped::of(works));
        let mut latin: Vec<(String, usize)> = Vec::new();
        fn walk(branches: &[Branch], out: &mut Vec<(String, usize)>) {
            for b in branches {
                if !b
                    .title
                    .chars()
                    .any(|c| ('\u{0590}'..='\u{05FF}').contains(&c))
                {
                    out.push((b.title.clone(), b.count));
                }
                walk(&b.children, out);
            }
        }
        walk(&tree, &mut latin);
        latin.sort_by_key(|shelf| std::cmp::Reverse(shelf.1));
        assert!(
            latin.is_empty(),
            "{} shelves in a Hebrew bookcase have no Hebrew name: {:?}",
            latin.len(),
            &latin[..latin.len().min(20)]
        );
    }

    #[test]
    #[ignore = "needs the fetched corpus: cargo test -p girsa-app --lib -- --ignored"]
    fn shas_opens_on_zeraim_and_not_on_the_biggest_folder() {
        // The reader's own complaint, at the scale it was made at: open
        // תלמוד → בבלי and the six sedarim come first, in the order they are
        // learned, above the rishonim and acharonim written on them.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
        assert!(
            root.join("works").is_dir(),
            "no corpus at {} — run girsa-fetch and girsa-import.",
            root.display()
        );
        let shelf = crate::shelf::Shelf::open(&root, &root.join("no-personal-layer"))
            .expect("the shelf opens");
        let works = shelf.works();
        let tree = tree(works, &Arrangement::default(), &Shipped::of(works));
        let bavli = find(&tree, "בבלי").expect("the bavli is on the shelf");

        let titles: Vec<&str> = bavli.children.iter().map(|b| b.title.as_str()).collect();
        println!("תלמוד/בבלי: {}", titles.join(" · "));

        let at = |name: &str| titles.iter().position(|t| *t == name);
        let (zeraim, moed) = (at("סדר זרעים"), at("סדר מועד"));
        assert!(
            zeraim.is_some() && moed.is_some(),
            "the sedarim are {titles:?}"
        );
        assert!(zeraim < moed, "Zeraim comes before Moed");
        if let Some(rishonim) = at("ראשונים") {
            assert!(
                zeraim < Some(rishonim),
                "the Gemara comes before the rishonim on it"
            );
        }
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
        let tree = tree(&works, &Arrangement::default(), &Shipped::of(&works));
        let arukh = find(&tree, "שולחן ערוך").expect("the shelf is there");
        let gathered = &arukh.children[0];
        assert!(gathered.loose);
        assert!(!gathered.mine, "the reader did not make it");
        assert!(!gathered.edited, "and has not edited it");
    }
}
