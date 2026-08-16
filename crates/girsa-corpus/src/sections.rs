//! What a work's schema calls the parts of an address, for the works whose
//! schema says it somewhere [`crate::work::Work::he_sections`] never looked.
//!
//! # The finding
//!
//! Every line of the Tur carried `orach_chayim א' א'` in its margin. Every line
//! of the Arukh HaShulchan carried `orach_chaim א' ט"ו`. That is a Latin slug,
//! set left-to-right, inside a right-to-left Hebrew margin, on 6,005 lines of
//! one and 25,265 of the other — and it says nothing about סימן or סעיף either,
//! so the two seforim a person reaches for beside a Shulchan Arukh were the two
//! whose addresses could not be read.
//!
//! It is the same defect the reader already reported once about the Gemara —
//! *"a Hebrew daf carried `30b:11` down its side"* — fixed then by routing the
//! margin through `girsa_cite`, and still true here because `girsa_cite` can
//! only print the words it is given.
//!
//! # Why `he_sections` is empty on exactly these works
//!
//! `Work::he_sections` is read from the schema's `heSectionNames`. A **flat**
//! schema carries them at its root and everything works. A **branch** schema —
//! a work that holds its chalakim inside itself — carries no `heSectionNames`
//! at the root at all: they sit on the leaf nodes, because the Tur's הקדמה is
//! addressed by פסקה and its body by סימן and סעיף, and the root cannot answer
//! for both.
//!
//! `girsa-cite`'s own note names the population: *"1,101 of Sefaria's 6,595 are
//! branch schemas that do not carry them."* The importer read the root, found
//! nothing, and wrote an empty list — which is honest about the root and wrong
//! about the work.
//!
//! # And the section's own name was never carried anywhere
//!
//! The second half. A named level in an address is the **slug of the schema
//! node's `title`** — `Orach Chayim` → `orach_chayim` — and the node's `heTitle`
//! beside it says `אורח חיים`. Nothing in the corpus's own files keeps that
//! pairing, so the address had the slug and no way back to the name.
//!
//! Matched on `title` and not on `key`, and that is measured rather than
//! assumed: the Tur's first node is `key: "Orach Chaim"`, `title: "Orach
//! Chayim"`, and its segments are addressed `orach_chayim`. Slugging the key
//! would miss all four chalakim of the sefer this module was written for.
//!
//! # Read here rather than repaired at import
//!
//! Re-running the importer to write these into `work.json` re-cuts segments,
//! which renames permanent ids — the same argument as
//! [`crate::taxonomy`]'s sibling in `girsa_app::display::opens_a_siman`. This is
//! one small JSON file read once per sefer opened, and the corpus does not move.

use std::collections::BTreeMap;
use std::path::Path;

/// The names a schema gives to the parts of an address.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sections {
    /// The slug of a named section → what the schema calls it in Hebrew.
    /// `orach_chayim` → `אורח חיים`.
    titles: BTreeMap<String, String>,
    /// The slug path of a section → the level names beneath it, outermost
    /// first. `["orach_chayim"]` → `["סימן", "סעיף"]`.
    ///
    /// Keyed by the whole path and not by the last level, because two chalakim
    /// of one sefer need not be addressed the same way — the Tur's הקדמה is
    /// counted in פסקאות and its body in סימנים.
    levels: BTreeMap<Vec<String>, Vec<String>>,
}

impl Sections {
    /// Read a work's schema, if it has one and it can be read.
    ///
    /// `None` for a work with no schema — every Otzaria work, and anything a
    /// reader dropped on the window — which is *the corpus does not say* and
    /// leaves the address printed by number, exactly as it is now.
    ///
    /// A schema that will not parse is also `None` rather than an error: this
    /// decorates an address, and a margin that says less is not a reason to
    /// refuse to open a sefer.
    #[must_use]
    pub fn read(schema: &Path) -> Option<Self> {
        Self::of(&std::fs::read_to_string(schema).ok()?)
    }

    /// The same, from the schema's text.
    ///
    /// Separated from the file read so that a test can state a schema and get
    /// an answer without a temporary directory — two of them raced over one and
    /// deleted it out from under each other, which is a test failure about
    /// nothing.
    #[must_use]
    pub fn of(body: &str) -> Option<Self> {
        let json: serde_json::Value = serde_json::from_str(body).ok()?;
        let root = json.get("schema").unwrap_or(&json);
        let mut out = Self::default();
        out.walk(root, &[]);
        (!out.titles.is_empty() || !out.levels.is_empty()).then_some(out)
    }

    /// Gather one node and everything under it.
    ///
    /// `at` is the slug path of the section this node stands in — empty at the
    /// root, because the root is the work and the work is not a level of its
    /// own address.
    fn walk(&mut self, node: &serde_json::Value, at: &[String]) {
        if let Some(names) = node.get("heSectionNames").and_then(|v| v.as_array()) {
            let names: Vec<String> = names
                .iter()
                .filter_map(|n| n.as_str().map(str::to_string))
                .collect();
            if !names.is_empty() {
                // A `default` node carries the level names of the section it
                // stands in rather than of a section of its own — that is what
                // makes it the default — so it does not deepen the path, and
                // `walk` has already been given the parent's path for it.
                self.levels.entry(at.to_vec()).or_insert(names);
            }
        }
        let Some(children) = node.get("nodes").and_then(|v| v.as_array()) else {
            return;
        };
        for child in children {
            // The **title**, slugged the way the importer slugs it. See the
            // module note: the key is a different string and misses.
            let Some(title) = child.get("title").and_then(|v| v.as_str()) else {
                continue;
            };
            let slug = crate::work::section_label_of(title);
            // A `default` node is not a place in the address. Sefaria writes it
            // for the unnamed body of a section that also has named siblings —
            // the Tur's simanim beside its הקדמה — and an address goes straight
            // from the chelek to the siman.
            let here: Vec<String> = if slug == "default" || slug.is_empty() {
                at.to_vec()
            } else {
                if let Some(said) = child.get("heTitle").and_then(|v| v.as_str()) {
                    if !said.is_empty() {
                        self.titles.entry(slug.clone()).or_insert(said.to_string());
                    }
                }
                at.iter().cloned().chain(std::iter::once(slug)).collect()
            };
            self.walk(child, &here);
        }
    }

    /// What the schema calls this level, where the level names a section.
    ///
    /// `None` for a level that is a number, and for a name the schema does not
    /// carry — both of which stay exactly as the address spells them, because
    /// a guessed name on a mekor is worse than a slug somebody can look up.
    #[must_use]
    pub fn titled(&self, level: &str) -> Option<&str> {
        self.titles.get(level).map(String::as_str)
    }

    /// The level names for an address, outermost first — `["סימן", "סעיף"]`.
    ///
    /// Given the whole address path. The named sections at the front of it are
    /// walked off one at a time and the deepest section that says anything
    /// answers, so `["orach_chayim", "1", "1"]` gets Orach Chayim's names and a
    /// path into a section the schema is silent about falls back to the work's
    /// own.
    #[must_use]
    pub fn levels(&self, path: &[String]) -> &[String] {
        let named: Vec<String> = path
            .iter()
            .take_while(|level| self.titles.contains_key(*level))
            .cloned()
            .collect();
        for depth in (0..=named.len()).rev() {
            if let Some(names) = self.levels.get(&named[..depth]) {
                return names;
            }
        }
        &[]
    }

    /// How many levels at the front of this path name sections rather than
    /// places — `["orach_chayim", "1", "1"]` is one.
    ///
    /// What a formatter needs in order to print the section by name and count
    /// the rest as an address.
    #[must_use]
    pub fn named(&self, path: &[String]) -> usize {
        path.iter()
            .take_while(|level| self.titles.contains_key(*level))
            .count()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.titles.is_empty() && self.levels.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn tur() -> Sections {
        // The Tur's schema, in the shape Sefaria writes it: a branch with four
        // chalakim, each holding a named הקדמה and an unnamed body.
        let json = serde_json::json!({
            "schema": {
                "title": "Tur",
                "heTitle": "טור",
                "nodes": [
                    {
                        "key": "Orach Chaim",
                        "title": "Orach Chayim",
                        "heTitle": "אורח חיים",
                        "nodes": [
                            {
                                "title": "Introduction",
                                "heTitle": "הקדמה",
                                "heSectionNames": ["פסקה"],
                            },
                            {
                                "title": "default",
                                "heTitle": "",
                                "heSectionNames": ["סימן", "סעיף"],
                            },
                        ],
                    },
                    {
                        "key": "Yoreh Deah",
                        "title": "Yoreh De'ah",
                        "heTitle": "יורה דעה",
                        "nodes": [{
                            "title": "default",
                            "heTitle": "",
                            "heSectionNames": ["סימן", "סעיף"],
                        }],
                    },
                ],
            }
        });
        Sections::of(&json.to_string()).expect("the schema reads")
    }

    #[test]
    fn a_named_section_is_matched_on_its_title_and_not_on_its_key() {
        // Measured, and it is the whole reason this is not a one-liner: the
        // Tur's first node is `key: "Orach Chaim"` and `title: "Orach Chayim"`,
        // and its segments are addressed `orach_chayim`. Slugging the key gives
        // `orach_chaim`, which matches nothing in the sefer this was written
        // for.
        let tur = tur();
        assert_eq!(tur.titled("orach_chayim"), Some("אורח חיים"));
        assert_eq!(
            tur.titled("orach_chaim"),
            None,
            "that is the key, not the title"
        );
        assert_eq!(tur.titled("yoreh_deah"), Some("יורה דעה"));
        // Apostrophes are dropped by the slugger, which is how `Yoreh De'ah`
        // and `Yoreh Deah` arrive at one address.
        assert_eq!(tur.titled("introduction"), Some("הקדמה"));
    }

    #[test]
    fn a_branch_schema_carries_its_level_names_on_the_leaves() {
        // The defect, stated: `Work::he_sections` reads `heSectionNames` off the
        // root, and a branch schema has none there. `girsa-cite`'s own note
        // counts 1,101 of Sefaria's 6,595 schemas in that shape, and the Tur and
        // the Arukh HaShulchan are two of them.
        let tur = tur();
        let siman = ["orach_chayim".to_string(), "1".to_string(), "1".to_string()];
        assert_eq!(tur.levels(&siman), ["סימן", "סעיף"]);
        // …and a chelek is allowed to disagree with its neighbour about how it
        // is counted, which is why this is keyed by the whole path: the Tur's
        // הקדמה is in פסקאות and its body is in סימנים.
        let hakdama = [
            "orach_chayim".to_string(),
            "introduction".to_string(),
            "4".to_string(),
        ];
        assert_eq!(tur.levels(&hakdama), ["פסקה"]);
    }

    #[test]
    fn the_named_levels_at_the_front_are_counted_and_the_rest_is_the_address() {
        let tur = tur();
        assert_eq!(
            tur.named(&["orach_chayim".to_string(), "1".to_string(), "1".to_string()]),
            1
        );
        assert_eq!(
            tur.named(&[
                "orach_chayim".to_string(),
                "introduction".to_string(),
                "4".to_string()
            ]),
            2
        );
        // A flat work — the Shulchan Arukh is four separate works — names no
        // sections at all, and every level of its address is a number.
        assert_eq!(tur.named(&["1".to_string(), "1".to_string()]), 0);
    }

    #[test]
    fn a_work_with_no_schema_says_nothing_rather_than_guessing() {
        assert_eq!(Sections::read(Path::new("no/such/schema.json")), None);
        let empty = Sections::default();
        assert!(empty.is_empty());
        assert_eq!(empty.titled("orach_chayim"), None);
        assert!(empty.levels(&["1".to_string()]).is_empty());
    }
}
