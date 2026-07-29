//! Lenses: which links you want to see, saved.
//!
//! spec.md §8.5, BUILDER.md W24. *Halacha / Lomdus / Peshat / Girsa / Mine are
//! **saved filters over type, era and strength — not hardcoded lists**.* The
//! difference matters: a hardcoded lens is a menu somebody else wrote, and a
//! saved filter is one you can change, add to and delete, because it is a file
//! in your own layer.
//!
//! So the five that ship are five rows of the same kind everything else is, and
//! they are the ones on the file until you edit one. What a lens can say is:
//!
//! - which **types** of link (spec.md §8.2);
//! - which **eras** the sefer at the far end belongs to — a Halacha lens is
//!   mostly *acharonim writing halacha*, and era is what the catalogue knows;
//! - how strong a claim it has to be, which after W23 means *confirmed and
//!   drawn links score 1.0, an untyped seed scores what its method scores*;
//! - and whether to show only what you said yourself.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use girsa_link::EdgeType;
use serde::{Deserialize, Serialize};

use crate::links::Link;
use crate::shelf::Shelf;

/// One saved filter over the link graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lens {
    /// What it is called on the button.
    pub title: String,
    /// The types it lets through. Empty is *any*.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<String>,
    /// The eras of the sefer at the far end, as the catalogue codes them —
    /// `RI`, `AH`, `T`. Empty is *any*.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eras: Vec<String>,
    /// How strong a claim it has to be, 0.0–1.0.
    #[serde(default)]
    pub at_least: f32,
    /// Only links you drew, confirmed, or otherwise touched.
    #[serde(default)]
    pub mine: bool,
}

impl Lens {
    /// Whether a link belongs in this lens.
    #[must_use]
    pub fn takes(&self, link: &Link, era: Option<&str>) -> bool {
        if self.mine && link.repaired.changed.is_empty() {
            return false;
        }
        if link.repaired.confidence() < self.at_least {
            return false;
        }
        if !self.types.is_empty()
            && !self
                .types
                .iter()
                .any(|name| name == link.repaired.edge.edge_type.as_str())
        {
            return false;
        }
        if !self.eras.is_empty() && !era.is_some_and(|era| self.eras.iter().any(|e| e == era)) {
            return false;
        }
        true
    }
}

/// The lenses that ship, in the order they are offered.
///
/// Five filters, not five lists. Each is a claim about what a lens *is* —
/// Peshat is what explains the words in front of you, Lomdus is where somebody
/// argues — and every one of them is editable, because whether the Tur belongs
/// under Halacha is a question about how you learn and not about this program.
#[must_use]
pub fn shipped() -> BTreeMap<String, Lens> {
    let lens = |title: &str, types: &[EdgeType], eras: &[&str], at_least: f32, mine: bool| Lens {
        title: title.to_string(),
        types: types.iter().map(|t| t.as_str().to_string()).collect(),
        eras: eras.iter().map(|e| (*e).to_string()).collect(),
        at_least,
        mine,
    };
    [
        (
            "halacha".to_string(),
            lens(
                "הלכה",
                &[EdgeType::Codifies, EdgeType::CommentsOn],
                &["RI", "AH", "CO"],
                0.0,
                false,
            ),
        ),
        (
            "lomdus".to_string(),
            lens(
                "לומדות",
                &[EdgeType::Disputes, EdgeType::Paraphrases, EdgeType::Quotes],
                &[],
                0.0,
                false,
            ),
        ),
        (
            "peshat".to_string(),
            lens(
                "פשט",
                &[EdgeType::CommentsOn, EdgeType::Translates],
                &[],
                0.0,
                false,
            ),
        ),
        (
            "girsa".to_string(),
            lens(
                "גרסה",
                &[EdgeType::Emends, EdgeType::ParallelTo],
                &[],
                0.0,
                false,
            ),
        ),
        ("mine".to_string(), lens("שלי", &[], &[], 0.0, true)),
    ]
    .into_iter()
    .collect()
}

/// Your lenses: the shipped five, with your edits over them.
///
/// One file, `personal/lenses.json`, and a lens you delete stays deleted —
/// which is the point of them being saved filters. A lens is not offered by
/// this program; it is one you keep.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lenses {
    /// Key → lens. Ordered, so the row of buttons does not shuffle itself
    /// between two openings of the panel.
    #[serde(flatten)]
    pub lenses: BTreeMap<String, Lens>,
}

impl Default for Lenses {
    fn default() -> Self {
        Self { lenses: shipped() }
    }
}

/// Where they live under a personal layer.
#[must_use]
pub fn path_in(personal: &Path) -> PathBuf {
    personal.join("lenses.json")
}

impl Lenses {
    /// Read yours, or start from the five that ship.
    ///
    /// A file that will not parse gives the shipped five **and says so**: a
    /// lens list that silently resets is a reader wondering where their lens
    /// went.
    #[must_use]
    pub fn load(personal: &Path) -> (Self, Option<String>) {
        let path = path_in(personal);
        let Ok(body) = std::fs::read_to_string(&path) else {
            return (Self::default(), None);
        };
        match serde_json::from_str::<Self>(&body) {
            Ok(lenses) => (lenses, None),
            Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
        }
    }

    /// # Errors
    ///
    /// If the personal layer cannot be written.
    pub fn save(&self, personal: &Path) -> Result<(), std::io::Error> {
        let path = path_in(personal);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let body = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, body)
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Lens> {
        self.lenses.get(key)
    }

    /// Keep only the links a lens takes.
    ///
    /// The era comes from the catalogue entry of the sefer at the **far end**,
    /// which is the sefer the lens is about — a Halacha lens over a page of
    /// Gemara is asking which of the things pointing at it are halacha.
    #[must_use]
    pub fn through(&self, key: &str, shelf: &Shelf, links: Vec<Link>) -> Vec<Link> {
        let Some(lens) = self.get(key) else {
            return links;
        };
        links
            .into_iter()
            .filter(|link| {
                let era = shelf.work(&link.work).and_then(|w| w.era.as_deref());
                lens.takes(link, era)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::segment::{Ordinal, SegmentId};
    use girsa_link::repair::Repairs;
    use girsa_link::{Anchor, Edge, Method};

    fn link(edge_type: EdgeType, confirmed: bool) -> Link {
        let id = |work: &str| SegmentId::new(work, vec!["1".into()], Ordinal::root(1));
        let edge = Edge {
            from: Anchor::point(id("bavli/berakhot")),
            to: Anchor::point(id("shulchan-arukh/orach-chayim")),
            edge_type,
            method: Method::SefariaSeed,
            source_label: String::new(),
        };
        let mut repaired = Repairs::nowhere().apply(vec![edge]).remove(0);
        if confirmed {
            repaired.confirmed = true;
            repaired.changed.push("confirmed");
        }
        Link {
            repaired,
            outgoing: true,
            other: Anchor::point(id("shulchan-arukh/orach-chayim")),
            work: "shulchan-arukh/orach-chayim".into(),
            he_title: "שולחן ערוך".into(),
            address: "1".into(),
            span: None,
        }
    }

    #[test]
    fn a_lens_is_a_filter_and_not_a_list_of_seforim() {
        let lenses = Lenses::default();
        let halacha = lenses.get("halacha").expect("ships");
        assert!(halacha.takes(&link(EdgeType::Codifies, false), Some("AH")));
        // The same sefer, a different kind of claim, is a different lens.
        assert!(!halacha.takes(&link(EdgeType::Disputes, false), Some("AH")));
        // …and the same claim from an era the lens does not want.
        assert!(!halacha.takes(&link(EdgeType::Codifies, false), Some("T")));
        assert!(!halacha.takes(&link(EdgeType::Codifies, false), None));
    }

    #[test]
    fn the_mine_lens_is_what_you_have_touched_and_nothing_else() {
        let lenses = Lenses::default();
        let mine = lenses.get("mine").expect("ships");
        assert!(!mine.takes(&link(EdgeType::CommentsOn, false), Some("RI")));
        assert!(mine.takes(&link(EdgeType::CommentsOn, true), Some("RI")));
    }

    #[test]
    fn a_lens_can_ask_for_a_stronger_claim_than_a_seed() {
        let strong = Lens {
            title: "בטוח".into(),
            types: Vec::new(),
            eras: Vec::new(),
            at_least: 1.0,
            mine: false,
        };
        assert!(!strong.takes(&link(EdgeType::CommentsOn, false), None));
        assert!(strong.takes(&link(EdgeType::CommentsOn, true), None));
    }

    #[test]
    fn your_lenses_survive_a_restart_and_a_deleted_one_stays_deleted() {
        let dir = crate::shelf::tests::scratch("girsa-lenses");
        let mut lenses = Lenses::default();
        lenses.lenses.remove("lomdus");
        lenses.lenses.insert(
            "shabbos".into(),
            Lens {
                title: "שבת".into(),
                types: vec![EdgeType::Codifies.as_str().to_string()],
                eras: Vec::new(),
                at_least: 0.5,
                mine: false,
            },
        );
        lenses.save(&dir).expect("saves");

        let (back, trouble) = Lenses::load(&dir);
        assert_eq!(trouble, None);
        assert_eq!(back, lenses);
        assert!(back.get("lomdus").is_none());
        assert_eq!(
            back.get("shabbos").map(|l| l.title.clone()),
            Some("שבת".into())
        );
    }

    #[test]
    fn a_lens_file_that_will_not_read_gives_the_shipped_five_and_says_so() {
        let dir = crate::shelf::tests::scratch("girsa-lenses-broken");
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(path_in(&dir), "{ not json").expect("writes");
        let (back, trouble) = Lenses::load(&dir);
        assert_eq!(back, Lenses::default());
        assert!(trouble.is_some());
    }
}
