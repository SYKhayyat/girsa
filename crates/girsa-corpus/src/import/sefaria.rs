//! Reading a Sefaria work: the schema says what the shape is, the merged text
//! fills it.
//!
//! spec.md §2.2 — *the schemas are the prize.* Otzaria has a line that says
//! `סימן א`. Sefaria's schema knows what a siman **is**:
//!
//! ```jsonc
//! { "nodeType": "JaggedArrayNode",
//!   "depth": 2,
//!   "addressTypes":   ["Siman", "Seif"],
//!   "heSectionNames": ["סימן", "סעיף"],
//!   "lengths": [697, 4171] }
//! ```
//!
//! So the address of every segment comes from the schema, and nothing here
//! re-derives structure from headings in the text. BUILDER.md W7: *use them; do
//! not re-derive structure from headings when a schema exists.*
//!
//! # Two node shapes
//!
//! **Jagged** — the common case, 5,493 of 6,595 schemas. `text` is arrays
//! nested `depth` deep, and the address is the index at each level.
//!
//! **Branch** — a `SchemaNode` with `nodes`, 1,101 of them. `text` is an object
//! keyed by the child's title, and the child's title becomes a level of the
//! address: `Abarbanel on Ezekiel, Introduction 3`. One child may be marked
//! `default`, is keyed by the empty string, and contributes **no** level — it
//! is the body of the work rather than a named part of it.
//!
//! # Where the address is not a number
//!
//! Of the address types in use, exactly one is not the index plus one:
//! `Talmud`. Sefaria stores a masechta as a flat array of amudim starting at
//! `1a`, so Berakhot's first two entries are empty and index 2 is daf 2a. Read
//! as an integer, every daf in Shas would be off by a page and a half.

use std::fs;

use serde_json::Value;

use super::{ImportError, RawSegment, SegmentKind};
use crate::work::{Version, Work};

/// Read one Sefaria work into segments, in reading order.
///
/// # Errors
///
/// If the text file cannot be read or does not parse. A *structural* surprise
/// — a leaf where the schema promised another level, a named node the text has
/// nothing for — is tolerated and read as best it can be, because it is common
/// across 6,000 works and refusing the sefer helps nobody.
pub fn read(work: &Work) -> Result<(Vec<RawSegment>, Option<Version>), ImportError> {
    let body = fs::read_to_string(&work.origin).map_err(ImportError::io(&work.origin))?;
    let doc: Value = serde_json::from_str(&body)
        .map_err(|e| ImportError::malformed(&work.origin, e.to_string()))?;
    let text = doc
        .get("text")
        .ok_or_else(|| ImportError::malformed(&work.origin, "no `text`"))?;

    // The authoritative schema is the one in `schemas/`; `merged.json` carries
    // a copy only for the complex works, and only sometimes.
    let node = work
        .schema
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|s| s.get("schema").cloned())
        .or_else(|| doc.get("schema").cloned());

    let node = node.as_ref().and_then(parse).unwrap_or_else(|| {
        // No schema at all. `sectionNames` in the text file still says how deep
        // the arrays go, and integer addressing is right for everything but
        // Talmud — which always has a schema.
        Node::Jagged {
            depth: doc
                .get("sectionNames")
                .and_then(Value::as_array)
                .map_or(1, Vec::len),
            address_types: Vec::new(),
        }
    });

    let mut out = Vec::new();
    let mut path = Vec::new();
    walk(&node, text, &mut path, &mut out);
    Ok((out, version_of(&doc)))
}

/// Which printed edition this text is, and where it came from.
///
/// spec.md §13: *carry each text's source and license in its metadata — costs
/// nothing now, and it is the only thing preserving the option to distribute
/// publicly later.* `merged.json` names the editions it was merged from:
///
/// ```jsonc
/// "versions": [["Maginei Eretz: Shulchan Aruch Orach Chaim, Lemberg, 1893",
///               "https://www.nli.org.il/he/books/NNL_ALEPH002084080"]]
/// ```
fn version_of(doc: &Value) -> Option<Version> {
    let editions: Vec<String> = doc
        .get("versions")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r.get(0).and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let edition = doc
        .get("versionTitle")
        .and_then(Value::as_str)
        .unwrap_or("merged");
    Some(Version {
        edition: if editions.is_empty() {
            edition.to_string()
        } else {
            editions.join(" · ")
        },
        provenance: doc
            .get("versionSource")
            .and_then(Value::as_str)
            .map(str::to_string),
        // Sefaria licenses per version and the merged file does not carry it.
        // Left absent rather than asserted: a licence this code invented would
        // be worse than none, since the only reason to record it is to rely on
        // it later.
        license: None,
    })
}

/// A schema node, reduced to the two things the importer needs from it.
#[derive(Debug, Clone)]
enum Node {
    Jagged {
        depth: usize,
        address_types: Vec<String>,
    },
    Branch {
        children: Vec<Child>,
    },
}

#[derive(Debug, Clone)]
struct Child {
    /// The key this child's text sits under in the `text` object.
    text_key: String,
    /// The level it contributes to the address, or nothing if it is the
    /// default node — the body of the work rather than a named part of it.
    label: Option<String>,
    node: Node,
}

fn parse(v: &Value) -> Option<Node> {
    if let Some(nodes) = v.get("nodes").and_then(Value::as_array) {
        let children = nodes
            .iter()
            .filter_map(|child| {
                let node = parse(child)?;
                let title = child
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let is_default = child
                    .get("default")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    || title.is_empty();
                Some(Child {
                    text_key: title.to_string(),
                    label: (!is_default).then(|| section_label(title)),
                    node,
                })
            })
            .collect();
        return Some(Node::Branch { children });
    }

    let depth = v.get("depth").and_then(Value::as_u64)? as usize;
    Some(Node::Jagged {
        depth,
        address_types: v
            .get("addressTypes")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn walk(node: &Node, text: &Value, path: &mut Vec<String>, out: &mut Vec<RawSegment>) {
    match node {
        Node::Jagged {
            depth,
            address_types,
        } => walk_jagged(text, *depth, address_types, 0, path, out),
        Node::Branch { children } => {
            let Some(map) = text.as_object() else {
                // A branch whose text is an array: the work was re-shaped
                // upstream and the schema has not caught up. Read it as a
                // jagged array rather than dropping the sefer.
                walk_jagged(text, 1, &[], 0, path, out);
                return;
            };
            for child in children {
                let Some(child_text) = map.get(&child.text_key) else {
                    continue;
                };
                match &child.label {
                    Some(label) => {
                        path.push(label.clone());
                        walk(&child.node, child_text, path, out);
                        path.pop();
                    }
                    None => walk(&child.node, child_text, path, out),
                }
            }
        }
    }
}

fn walk_jagged(
    text: &Value,
    depth: usize,
    address_types: &[String],
    level: usize,
    path: &mut Vec<String>,
    out: &mut Vec<RawSegment>,
) {
    match text {
        // A leaf, whatever the schema said the depth was. Some works are one
        // level shallower than their schema in places, and the words are still
        // the words.
        Value::String(s) => push(s, path, out),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                path.push(level_label(address_types.get(level), i));
                // Depth is advisory below the first level: an array where the
                // schema promised a string is descended into with integer
                // addressing rather than being stringified into one segment.
                walk_jagged(
                    item,
                    depth.saturating_sub(1),
                    address_types,
                    level + 1,
                    path,
                    out,
                );
                path.pop();
            }
        }
        // `null` is a gap Sefaria left, not a segment.
        _ => {}
    }
}

/// Mint a segment, unless there is nothing there.
///
/// An empty slot in a jagged array is a se'if Sefaria does not have, and
/// minting an id for it would put a blank line on the shelf and an anchor
/// pointing at nothing into every index built over it.
fn push(text: &str, path: &[String], out: &mut Vec<RawSegment>) {
    // T8: leading and trailing whitespace is grime, and it reaches the search
    // index and the clipboard if it is not taken off here.
    let text = text.trim();
    if text.is_empty() || path.is_empty() {
        return;
    }
    out.push(RawSegment {
        path: path.to_vec(),
        kind: SegmentKind::Text,
        text: text.to_string(),
    });
}

/// The address of the `i`th item at a level of the given type.
///
/// Every address type in the corpus is one-based integers — `Siman`, `Seif`,
/// `Perek`, `Halakhah`, `Mishnah`, `Pasuk`, `Integer` — except `Talmud`.
fn level_label(address_type: Option<&String>, i: usize) -> String {
    if address_type.is_some_and(|t| t == "Talmud") {
        return daf(i);
    }
    (i + 1).to_string()
}

/// Sefaria stores a masechta as a flat array of amudim from `1a`, so index 2 is
/// daf 2a — where every masechta actually begins.
fn daf(i: usize) -> String {
    let amud = if i % 2 == 0 { 'a' } else { 'b' };
    format!("{}{amud}", i / 2 + 1)
}

/// A section title as an address level.
///
/// `Introduction` is cited as a section of the work, so it has to be a level
/// that survives the grammar — no `/`, no `:`, no `#`. Hebrew names keep their
/// letters; everything else is a separator.
fn section_label(title: &str) -> String {
    let slug = crate::work::hebrew_slug_of(title);
    if slug.is_empty() {
        "1".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn segments(schema: &str, text: &str) -> Vec<(String, String)> {
        let schema: Value = serde_json::from_str(schema).unwrap_or(Value::Null);
        let text: Value = serde_json::from_str(text).unwrap_or(Value::Null);
        let node = parse(&schema).unwrap_or(Node::Jagged {
            depth: 1,
            address_types: Vec::new(),
        });
        let mut out = Vec::new();
        walk(&node, &text, &mut Vec::new(), &mut out);
        out.into_iter()
            .map(|s| (s.path.join(":"), s.text))
            .collect()
    }

    #[test]
    fn a_siman_and_seif_come_out_addressed_the_way_they_are_cited() {
        let got = segments(
            r#"{"nodeType":"JaggedArrayNode","depth":2,"addressTypes":["Siman","Seif"]}"#,
            r#"[["יתגבר כארי","ולא יתבייש"],["המשכים"]]"#,
        );
        assert_eq!(
            got,
            [
                ("1:1".to_string(), "יתגבר כארי".to_string()),
                ("1:2".to_string(), "ולא יתבייש".to_string()),
                ("2:1".to_string(), "המשכים".to_string()),
            ]
        );
    }

    #[test]
    fn a_masechta_starts_at_daf_2a_and_not_at_daf_1() {
        // Sefaria's array starts at 1a, which does not exist, so the first two
        // entries are empty and index 2 is where Berakhot begins. Read as an
        // integer this lands a page and a half early — on every daf in Shas.
        let got = segments(
            r#"{"nodeType":"JaggedArrayNode","depth":2,"addressTypes":["Talmud","Integer"]}"#,
            r#"[[],[],["מאימתי קורין"],["דילמא ביאת אורו"]]"#,
        );
        assert_eq!(
            got,
            [
                ("2a:1".to_string(), "מאימתי קורין".to_string()),
                ("2b:1".to_string(), "דילמא ביאת אורו".to_string()),
            ]
        );
    }

    #[test]
    fn a_named_node_becomes_a_level_and_the_default_node_does_not() {
        // `Abarbanel on Ezekiel` is an Introduction plus the commentary
        // proper. The introduction is cited by name; the commentary is not
        // cited as "the default part of Abarbanel".
        let got = segments(
            r#"{"nodes":[
                 {"title":"Introduction","depth":1,"addressTypes":["Integer"]},
                 {"title":"","default":true,"depth":2,"addressTypes":["Integer","Integer"]}]}"#,
            r#"{"Introduction":["פתיחה"],"":[["על פסוק א"]]}"#,
        );
        assert_eq!(
            got,
            [
                ("introduction:1".to_string(), "פתיחה".to_string()),
                ("1:1".to_string(), "על פסוק א".to_string()),
            ]
        );
    }

    #[test]
    fn a_gap_in_the_text_does_not_become_a_segment() {
        // Sefaria leaves `""` where it does not have a se'if. An id minted for
        // one would put a blank line on the shelf and an anchor pointing at
        // nothing into every index built over it.
        let got = segments(
            r#"{"depth":2,"addressTypes":["Integer","Integer"]}"#,
            r#"[["א","","  ","ד"]]"#,
        );
        assert_eq!(
            got,
            [
                ("1:1".to_string(), "א".to_string()),
                ("1:4".to_string(), "ד".to_string()),
            ],
            "a gap must be skipped without shifting the address of what follows"
        );
    }

    #[test]
    fn a_leaf_deeper_than_the_schema_promised_is_still_read() {
        // Real across thousands of works: the schema says depth 2 and one
        // place has three. Stringifying the array into one segment would put
        // JSON on the reader's page.
        let got = segments(
            r#"{"depth":2,"addressTypes":["Integer","Integer"]}"#,
            r#"[[["א","ב"]]]"#,
        );
        assert_eq!(
            got,
            [
                ("1:1:1".to_string(), "א".to_string()),
                ("1:1:2".to_string(), "ב".to_string()),
            ]
        );
    }
}
