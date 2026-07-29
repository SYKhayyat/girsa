//! Live facets — shelf, era, author, sefer, link type (BUILDER.md W14).
//!
//! spec.md §9.8, in full: *results carry live facets — shelf section, era,
//! author, sefer, link type — each with counts, each one click to narrow or
//! exclude. You get it right on the second try instead of being punished for
//! the first.*
//!
//! # Two of the five are columns; three are properties of a sefer
//!
//! The index knows a segment's **work** and the kinds of **link** touching it,
//! and counts both over the whole result set in one pass ([`super::index::Counts`]).
//! Shelf, era and author are not facts about a segment at all — they are facts
//! about the sefer it is in — so they are worked out here, by taking the
//! per-sefer counts and adding them up through the catalogue.
//!
//! That is why there is no era column in the index. Indexing one would mean
//! re-indexing five million segments to correct one author's dates, and the
//! catalogue is rewritten by `girsa-import --metadata-only` in seconds.
//!
//! # Counts nest, and the rows say how deep they are
//!
//! `תלמוד` and `תלמוד/בבלי` are both shelf rows and the second is inside the
//! first, so the column does not add up to the total and is not meant to.
//! [`Row::depth`] is what a reader sees as an indent. Flattening to top shelves
//! only would answer *which shelf* and never *which part of it*, which is the
//! question a reader with 300 hits in `תלמוד` actually has.
//!
//! # A zero is never guessed at
//!
//! Two rows exist that a tidier facet column would leave out, and both would
//! otherwise be silent gaps:
//!
//! - **`no era recorded`** — 2,377 of the 7,189 works on this shelf have no
//!   era in either corpus. A facet that listed only the five real eras would
//!   quietly hide a third of the library.
//! - **[`Links::NotBuilt`]** — the link column is filled from a cache
//!   (`girsa-link-types`), and an index built without it has no link types at
//!   all. *Nothing here is commented on* and *nobody worked out what is
//!   commented on* are different statements and the facet says which.

use std::collections::BTreeMap;

use girsa_corpus::taxonomy;
use girsa_corpus::work::Work;

use crate::index::Counts;
use crate::scope::Scope;

/// What a facet is counted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    Sefer,
    Shelf,
    Era,
    Author,
    Link,
}

impl Dimension {
    /// Every dimension, in the order spec.md §9.8 lists them.
    pub const ALL: [Self; 5] = [
        Self::Shelf,
        Self::Era,
        Self::Author,
        Self::Sefer,
        Self::Link,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sefer => "sefer",
            Self::Shelf => "shelf",
            Self::Era => "era",
            Self::Author => "author",
            Self::Link => "link type",
        }
    }
}

/// What the shelf knows about one sefer that the index does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub title: String,
    /// The shelf, as a path from the top.
    pub shelf: Vec<String>,
    /// Sefaria's era code, as written. `None` where neither corpus says.
    pub era: Option<String>,
    pub author: Option<String>,
}

/// Sefaria's era codes, in the words a reader uses.
///
/// A code not in this table is **carried through as written** — the same rule
/// the shelf taxonomy uses for a category nobody has translated. A guess at
/// what an unknown code means would be a label a reader cannot check.
const ERAS: [(&str, &str); 6] = [
    ("T", "תנאים"),
    ("A", "אמוראים"),
    ("GN", "גאונים"),
    ("RI", "ראשונים"),
    ("AH", "אחרונים"),
    ("CO", "מחברי זמננו"),
];

/// The key of the row for a work whose era nobody recorded.
pub const NO_ERA: &str = "";

/// Everything the facets need to know about the seforim on the shelf.
///
/// Built once from the catalogue `girsa-import` wrote, and reused for every
/// search. The shelf comes from [`girsa_corpus::taxonomy`] — the same function
/// the bookcase browses by — so a sefer is never on one shelf in the tree and
/// another in a result list.
#[derive(Debug, Clone, Default)]
pub struct Catalogue {
    rows: BTreeMap<String, Facts>,
}

impl Catalogue {
    /// Read the works as they came off the shelf.
    #[must_use]
    pub fn of(works: &[Work]) -> Self {
        let mut rows = BTreeMap::new();
        for work in works {
            rows.insert(
                work.slug.clone(),
                Facts {
                    title: work.he_title.clone(),
                    shelf: taxonomy::shelf_of(work),
                    era: work.era.clone(),
                    author: work.author.clone(),
                },
            );
        }
        Self { rows }
    }

    /// Put a sefer on the shelf the reader moved it to.
    ///
    /// The window has an arrangement and this crate does not (spec.md §5 — the
    /// shipped taxonomy is a default, not a fact). A result list that filed
    /// seforim by the shipped shelf while the bookcase beside it used the
    /// reader's would be two answers to one question.
    pub fn filed(&mut self, slug: &str, shelf: Vec<String>) {
        if let Some(facts) = self.rows.get_mut(slug) {
            facts.shelf = shelf;
        }
    }

    #[must_use]
    pub fn facts(&self, slug: &str) -> Option<&Facts> {
        self.rows.get(slug)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Every sefer that would be counted under one row of a facet.
    ///
    /// What a click needs: narrowing to `תלמוד/בבלי` is narrowing to the 1,624
    /// seforim on it, because the index knows seforim and not shelves.
    #[must_use]
    pub fn seforim_under(&self, dimension: Dimension, key: &str) -> Vec<String> {
        self.rows
            .iter()
            .filter(|(slug, facts)| match dimension {
                Dimension::Sefer => slug.as_str() == key,
                // A prefix, so `תלמוד` takes the Bavli and the Yerushalmi with
                // it — which is what clicking the shelf above them means.
                Dimension::Shelf => {
                    facts.shelf.join("/") == key
                        || facts.shelf.join("/").starts_with(&format!("{key}/"))
                }
                Dimension::Era => facts.era.as_deref().unwrap_or(NO_ERA) == key,
                Dimension::Author => facts.author.as_deref().unwrap_or_default() == key,
                // Not a property of a sefer at all — it is a column of the
                // index, and narrowing by it never goes through the catalogue.
                Dimension::Link => false,
            })
            .map(|(slug, _)| slug.clone())
            .collect()
    }
}

/// One row of a facet: a count, and what clicking it means.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Row {
    /// What a click narrows by. A slug, a shelf path, an era code, a name.
    pub key: String,
    /// What it says on the row.
    pub label: String,
    pub count: usize,
    /// How far in to indent it. Only shelves nest.
    pub depth: usize,
}

/// The link-type facet, which can be *not built* as well as empty.
///
/// Serialized as a tagged union on purpose: a window that received an empty
/// list for both cases could not tell a reader which one it was looking at,
/// which is the whole point of the distinction.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "rows")]
pub enum Links {
    Counted(Vec<Row>),
    /// The index was built before `girsa-link-types` had run. The facet cannot
    /// be shown, and showing zeros instead would be a lie a reader cannot see
    /// through.
    NotBuilt,
}

/// The five facets of spec.md §9.8, counted over a whole result set.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Facets {
    pub sefer: Vec<Row>,
    pub shelf: Vec<Row>,
    pub era: Vec<Row>,
    pub author: Vec<Row>,
    pub link: Links,
    /// Hits in seforim the catalogue has never heard of.
    ///
    /// Zero on a shelf and an index built from each other. Above zero it means
    /// the index is ahead of the catalogue, and the three derived facets are
    /// short by this many — said out loud rather than folded into an `אחר` row
    /// that would look like a real shelf.
    pub uncatalogued: usize,
    pub total: usize,
}

impl Facets {
    /// Work the facets out from the counts and the catalogue.
    #[must_use]
    pub fn of(counts: &Counts, catalogue: &Catalogue) -> Self {
        let mut sefer: Vec<Row> = Vec::new();
        let mut shelf: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        let mut era: BTreeMap<String, usize> = BTreeMap::new();
        let mut author: BTreeMap<String, usize> = BTreeMap::new();
        let mut uncatalogued = 0usize;

        for (slug, count) in &counts.by_work {
            let Some(facts) = catalogue.facts(slug) else {
                uncatalogued += count;
                sefer.push(Row {
                    key: slug.clone(),
                    label: slug.clone(),
                    count: *count,
                    depth: 0,
                });
                continue;
            };
            sefer.push(Row {
                key: slug.clone(),
                label: facts.title.clone(),
                count: *count,
                depth: 0,
            });
            // Every prefix of the shelf path, so a reader can narrow to
            // `תלמוד` or to `תלמוד/בבלי/אחרונים` and both rows are real.
            for depth in 1..=facts.shelf.len() {
                let key = facts.shelf[..depth].join("/");
                let row = shelf.entry(key).or_insert((0, depth - 1));
                row.0 += count;
            }
            *era.entry(facts.era.clone().unwrap_or_else(|| NO_ERA.to_string()))
                .or_default() += count;
            if let Some(name) = &facts.author {
                *author.entry(name.clone()).or_default() += count;
            }
        }

        sefer.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));
        Self {
            sefer,
            shelf: ordered(
                shelf
                    .into_iter()
                    .map(|(key, (count, depth))| Row {
                        label: key.rsplit('/').next().unwrap_or(&key).to_string(),
                        key,
                        count,
                        depth,
                    })
                    .collect(),
            ),
            era: ordered(
                era.into_iter()
                    .map(|(key, count)| Row {
                        label: era_label(&key),
                        key,
                        count,
                        depth: 0,
                    })
                    .collect(),
            ),
            author: ordered(
                author
                    .into_iter()
                    .map(|(key, count)| Row {
                        label: key.clone(),
                        key,
                        count,
                        depth: 0,
                    })
                    .collect(),
            ),
            link: if counts.link_types_built {
                Links::Counted(ordered(
                    counts
                        .by_link
                        .iter()
                        .map(|(kind, count)| Row {
                            key: kind.as_str().to_string(),
                            label: kind.as_str().to_string(),
                            count: *count,
                            depth: 0,
                        })
                        .collect(),
                ))
            } else {
                Links::NotBuilt
            },
            uncatalogued,
            total: counts.total,
        }
    }

    /// The rows of one dimension.
    #[must_use]
    pub fn rows(&self, dimension: Dimension) -> &[Row] {
        match dimension {
            Dimension::Sefer => &self.sefer,
            Dimension::Shelf => &self.shelf,
            Dimension::Era => &self.era,
            Dimension::Author => &self.author,
            Dimension::Link => match &self.link {
                Links::Counted(rows) => rows,
                Links::NotBuilt => &[],
            },
        }
    }
}

/// Narrow a scope to one facet row — the click (spec.md §9.8).
#[must_use]
pub fn narrow(scope: &Scope, catalogue: &Catalogue, dimension: Dimension, row: &Row) -> Scope {
    match dimension {
        Dimension::Link => match girsa_link::touching::type_named(&row.key) {
            Some(kind) => scope.clone().linked(kind),
            None => scope.clone(),
        },
        _ => scope
            .clone()
            .only(catalogue.seforim_under(dimension, &row.key), &row.label),
    }
}

/// Rule one facet row out — the other click.
#[must_use]
pub fn exclude(scope: &Scope, catalogue: &Catalogue, dimension: Dimension, row: &Row) -> Scope {
    match dimension {
        Dimension::Link => match girsa_link::touching::type_named(&row.key) {
            Some(kind) => scope.clone().unlinked(kind),
            None => scope.clone(),
        },
        _ => scope
            .clone()
            .without(catalogue.seforim_under(dimension, &row.key), &row.label),
    }
}

/// An era code in the words a reader uses, or as written when nobody knows it.
#[must_use]
pub fn era_label(code: &str) -> String {
    if code.is_empty() {
        return "no era recorded".to_string();
    }
    ERAS.iter()
        .find(|(en, _)| *en == code)
        .map_or_else(|| code.to_string(), |(_, he)| (*he).to_string())
}

/// Biggest first, and ties broken by name so two runs agree.
fn ordered(mut rows: Vec<Row>) -> Vec<Row> {
    rows.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.depth.cmp(&b.depth))
            .then_with(|| a.key.cmp(&b.key))
    });
    rows
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::work::Source;
    use girsa_link::EdgeType;

    fn work(slug: &str, categories: &[&str], era: Option<&str>, author: Option<&str>) -> Work {
        Work {
            slug: slug.to_string(),
            he_title: slug.to_string(),
            en_title: slug.to_string(),
            categories: categories.iter().map(|c| (*c).to_string()).collect(),
            source: Source::Sefaria,
            origin: std::path::PathBuf::new(),
            schema: None,
            author: author.map(str::to_string),
            era: era.map(str::to_string),
            comp_date: None,
            version: None,
            he_sections: Vec::new(),
            commentary_on: Vec::new(),
        }
    }

    fn shelf() -> Catalogue {
        Catalogue::of(&[
            work("bavli/berakhot", &["Talmud", "Bavli"], Some("A"), None),
            work(
                "bavli/rashi-on-berakhot",
                &["Talmud", "Bavli", "Rishonim on Talmud"],
                Some("RI"),
                Some("רש״י"),
            ),
            work(
                "mishnah-berurah",
                &["Halakhah"],
                Some("AH"),
                Some("החפץ חיים"),
            ),
            work("unnamed", &["Halakhah"], None, None),
        ])
    }

    fn counts(pairs: &[(&str, usize)], links: &[(EdgeType, usize)], built: bool) -> Counts {
        Counts {
            by_work: pairs
                .iter()
                .map(|(slug, n)| ((*slug).to_string(), *n))
                .collect(),
            by_link: links.iter().copied().collect(),
            total: pairs.iter().map(|(_, n)| n).sum(),
            link_types_built: built,
        }
    }

    #[test]
    fn a_shelf_is_counted_at_every_depth_it_has() {
        // A reader with 30 hits in `תלמוד` wants to know which part of it. A
        // facet flattened to top shelves can never answer that; one flattened
        // to leaves can never answer "how much of this is Shas".
        let facets = Facets::of(
            &counts(
                &[("bavli/berakhot", 20), ("bavli/rashi-on-berakhot", 10)],
                &[],
                false,
            ),
            &shelf(),
        );
        let rows: Vec<(&str, usize, usize)> = facets
            .shelf
            .iter()
            .map(|r| (r.key.as_str(), r.count, r.depth))
            .collect();
        assert!(rows.contains(&("תלמוד", 30, 0)), "{rows:?}");
        assert!(rows.contains(&("תלמוד/בבלי", 30, 1)), "{rows:?}");
        assert!(rows.contains(&("תלמוד/בבלי/ראשונים", 10, 2)), "{rows:?}");
    }

    #[test]
    fn a_work_with_no_era_is_a_row_and_not_a_silence() {
        // 2,377 of the 7,189 works on the real shelf have no era. Dropping them
        // from the column would hide a third of the library behind a facet that
        // looked complete.
        let facets = Facets::of(&counts(&[("unnamed", 5)], &[], false), &shelf());
        assert_eq!(facets.era.len(), 1);
        assert_eq!(facets.era[0].key, NO_ERA);
        assert_eq!(facets.era[0].label, "no era recorded");
        assert_eq!(facets.era[0].count, 5);
    }

    #[test]
    fn an_index_built_without_the_link_cache_says_so_rather_than_showing_zero() {
        let without = Facets::of(&counts(&[("bavli/berakhot", 3)], &[], false), &shelf());
        assert_eq!(without.link, Links::NotBuilt);

        let with = Facets::of(
            &counts(&[("bavli/berakhot", 3)], &[(EdgeType::CommentsOn, 2)], true),
            &shelf(),
        );
        assert_eq!(
            with.link,
            Links::Counted(vec![Row {
                key: "comments-on".into(),
                label: "comments-on".into(),
                count: 2,
                depth: 0,
            }])
        );
    }

    #[test]
    fn an_index_ahead_of_the_catalogue_is_counted_out_loud() {
        // The three derived facets can only speak for seforim the catalogue
        // knows. A hit in one it does not would otherwise be in the total and
        // in no shelf row, and the column would simply not add up.
        let facets = Facets::of(&counts(&[("who-is-this", 4)], &[], false), &shelf());
        assert_eq!(facets.uncatalogued, 4);
        assert!(facets.shelf.is_empty());
        assert_eq!(facets.sefer.len(), 1, "it is still a sefer row");
    }

    #[test]
    fn clicking_a_shelf_narrows_to_the_seforim_on_it_and_under_it() {
        let catalogue = shelf();
        let facets = Facets::of(
            &counts(
                &[
                    ("bavli/berakhot", 20),
                    ("bavli/rashi-on-berakhot", 10),
                    ("mishnah-berurah", 5),
                ],
                &[],
                false,
            ),
            &catalogue,
        );
        let row = facets
            .shelf
            .iter()
            .find(|r| r.key == "תלמוד")
            .expect("a תלמוד row");
        let scope = narrow(&Scope::everything(), &catalogue, Dimension::Shelf, row);
        assert_eq!(
            scope.works().iter().cloned().collect::<Vec<_>>(),
            ["bavli/berakhot", "bavli/rashi-on-berakhot"],
            "the shelf above them takes both"
        );
        assert_eq!(scope.describe(), "תלמוד");
    }

    #[test]
    fn excluding_a_sefer_rules_it_out_rather_than_narrowing_to_it() {
        let catalogue = shelf();
        let row = Row {
            key: "mishnah-berurah".into(),
            label: "משנה ברורה".into(),
            count: 5,
            depth: 0,
        };
        let scope = exclude(&Scope::everything(), &catalogue, Dimension::Sefer, &row);
        assert!(scope.works().is_empty());
        assert_eq!(
            scope.excluded_works().iter().cloned().collect::<Vec<_>>(),
            ["mishnah-berurah"]
        );
    }
}
