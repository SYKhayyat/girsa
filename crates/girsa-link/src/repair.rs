//! Saying that the graph is wrong, without editing the graph.
//!
//! spec.md §8.3, BUILDER.md W23. The link data really is wrong: 40% of it
//! carries no type at all, and it originates upstream (T5), so a re-import does
//! not fix it. The four things a reader can do about it are **reanchor**,
//! **retype**, **reject or confirm**, and **draw one by hand** — and every one
//! of them is stored as an override in your own layer, never as an edit to what
//! shipped.
//!
//! That is the same rule as corrections (§7.1), for the same three reasons: the
//! importer replaces every shard it owns on every run, your judgement and the
//! corpus's have to stay distinguishable, and a thing you said should be
//! undoable.
//!
//! # Everything shows its work
//!
//! §8.3 asks for it, and it is the difference between a repair tool and a
//! rumour. A [`Repaired`] carries **what it was** as well as what it is, which
//! of the four actions changed it, who said so, and the method and confidence
//! it came in with. And [`Repaired::is_curated`] is deliberately narrow: an
//! untyped seed is never a curated fact, whatever it is drawn beside, until
//! somebody looks at it and says so.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Anchor, Edge, EdgeType, Method};

/// What somebody said about a link when they looked at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Looked at, and right. This is what turns an untyped seed into something
    /// a reader may be shown as a fact.
    Confirmed,
    /// Looked at, and wrong. Not a deletion: the edge is still there and the
    /// rejection is yours to take back.
    Rejected,
}

/// One thing you said about one edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "does", rename_all = "lowercase")]
pub enum Repair {
    Judged {
        verdict: Verdict,
    },
    Retyped {
        #[serde(rename = "type")]
        edge_type: String,
    },
    Reanchored {
        from: String,
        to: String,
    },
    /// An edge that is not in the corpus at all, because you drew it.
    Drawn {
        from: String,
        to: String,
        #[serde(rename = "type")]
        edge_type: String,
    },
}

impl Repair {
    /// The word the UI puts beside a changed edge.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Judged {
                verdict: Verdict::Confirmed,
            } => "confirmed",
            Self::Judged {
                verdict: Verdict::Rejected,
            } => "rejected",
            Self::Retyped { .. } => "retyped",
            Self::Reanchored { .. } => "reanchored",
            Self::Drawn { .. } => "drawn",
        }
    }

    /// Whether two repairs are the same kind of statement, and so replace each
    /// other rather than piling up.
    const fn same_kind(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Judged { .. }, Self::Judged { .. })
                | (Self::Retyped { .. }, Self::Retyped { .. })
                | (Self::Reanchored { .. }, Self::Reanchored { .. })
                | (Self::Drawn { .. }, Self::Drawn { .. })
        )
    }
}

/// One record in your layer: which edge, what you said, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// The edge this is about, named by the **anchors it shipped with** — so a
    /// reanchored edge is still found by the record that moved it.
    pub edge: String,
    #[serde(flatten)]
    pub repair: Repair,
    pub who: String,
    pub when: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// What names an edge in your layer.
///
/// Its two anchors, as text, in the direction it is stored (spec.md §8.2: the
/// inverse is derived and never stored twice). Not a hash: a repair file is
/// meant to be greppable, and `girsa:bavli/berakhot/2a:1#1` in it should be
/// findable by searching for the segment.
#[must_use]
pub fn name_of(edge: &Edge) -> String {
    format!("{} → {}", edge.from, edge.to)
}

/// Why a repair could not be recorded.
#[derive(Debug, thiserror::Error)]
pub enum RepairError {
    #[error("writing {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("a link has to join two different places")]
    ToItself,
}

/// Your repairs, all of them: `personal/links.jsonl`.
#[derive(Debug, Clone)]
pub struct Repairs {
    path: PathBuf,
    by_edge: BTreeMap<String, Vec<Record>>,
}

/// Where they live under a personal layer.
#[must_use]
pub fn path_in(personal: &Path) -> PathBuf {
    personal.join("links.jsonl")
}

impl Repairs {
    /// Read them. A line that will not parse costs that repair and is reported.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        let path = path_in(personal);
        let mut repairs = Self {
            path,
            by_edge: BTreeMap::new(),
        };
        let mut trouble = Vec::new();
        let Ok(body) = std::fs::read_to_string(&repairs.path) else {
            return (repairs, trouble);
        };
        for (n, line) in body.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Record>(line) {
                Ok(record) => repairs.hold(record),
                Err(e) => trouble.push(format!(
                    "{}: line {} is not a repair: {e}",
                    repairs.path.display(),
                    n + 1
                )),
            }
        }
        (repairs, trouble)
    }

    /// A layer that is never written, for a caller that only wants to apply
    /// what it already has.
    #[must_use]
    pub fn nowhere() -> Self {
        Self {
            path: PathBuf::new(),
            by_edge: BTreeMap::new(),
        }
    }

    fn hold(&mut self, record: Record) {
        let held = self.by_edge.entry(record.edge.clone()).or_default();
        // One statement of each kind per edge: retyping twice is one retype and
        // the last word stands, and confirming after rejecting replaces the
        // verdict rather than leaving both on the file.
        held.retain(|kept| !kept.repair.same_kind(&record.repair));
        held.push(record);
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.by_edge.values().map(Vec::len).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_edge.is_empty()
    }

    /// What you have said about one edge.
    #[must_use]
    pub fn about(&self, edge: &Edge) -> &[Record] {
        self.by_edge.get(&name_of(edge)).map_or(&[], Vec::as_slice)
    }

    /// Confirm or reject a link.
    ///
    /// # Errors
    ///
    /// If your layer cannot be written.
    pub fn judge(&mut self, edge: &Edge, verdict: Verdict, who: &str) -> Result<(), RepairError> {
        self.judge_named(&name_of(edge), verdict, who)
    }

    /// The same, for a caller holding the edge's name rather than the edge —
    /// the window, which was handed rows and hands one back.
    ///
    /// # Errors
    ///
    /// If your layer cannot be written.
    pub fn judge_named(
        &mut self,
        edge: &str,
        verdict: Verdict,
        who: &str,
    ) -> Result<(), RepairError> {
        self.record(edge.to_string(), Repair::Judged { verdict }, who)
    }

    /// Give a link a type — the blank three quarters, mostly.
    ///
    /// # Errors
    ///
    /// If your layer cannot be written.
    pub fn retype(
        &mut self,
        edge: &Edge,
        edge_type: EdgeType,
        who: &str,
    ) -> Result<(), RepairError> {
        self.retype_named(&name_of(edge), edge_type, who)
    }

    /// # Errors
    ///
    /// If your layer cannot be written.
    pub fn retype_named(
        &mut self,
        edge: &str,
        edge_type: EdgeType,
        who: &str,
    ) -> Result<(), RepairError> {
        self.record(
            edge.to_string(),
            Repair::Retyped {
                edge_type: edge_type.as_str().to_string(),
            },
            who,
        )
    }

    /// Move a link onto the segments it belongs on.
    ///
    /// # Errors
    ///
    /// If both ends would be the same place, or your layer cannot be written.
    pub fn reanchor(
        &mut self,
        edge: &Edge,
        from: Anchor,
        to: Anchor,
        who: &str,
    ) -> Result<(), RepairError> {
        self.reanchor_named(&name_of(edge), from, to, who)
    }

    /// # Errors
    ///
    /// If both ends would be the same place, or your layer cannot be written.
    pub fn reanchor_named(
        &mut self,
        edge: &str,
        from: Anchor,
        to: Anchor,
        who: &str,
    ) -> Result<(), RepairError> {
        if from == to {
            return Err(RepairError::ToItself);
        }
        self.record(
            edge.to_string(),
            Repair::Reanchored {
                from: from.to_string(),
                to: to.to_string(),
            },
            who,
        )
    }

    /// Draw a link the corpus does not have.
    ///
    /// # Errors
    ///
    /// If both ends are the same place, or your layer cannot be written.
    pub fn draw(
        &mut self,
        from: Anchor,
        to: Anchor,
        edge_type: EdgeType,
        who: &str,
    ) -> Result<(), RepairError> {
        if from == to {
            return Err(RepairError::ToItself);
        }
        let drawn = Repair::Drawn {
            from: from.to_string(),
            to: to.to_string(),
            edge_type: edge_type.as_str().to_string(),
        };
        self.record(format!("{from} → {to}"), drawn, who)
    }

    fn record(&mut self, edge: String, repair: Repair, who: &str) -> Result<(), RepairError> {
        self.hold(Record {
            edge,
            repair,
            who: who.to_string(),
            when: now_seconds(),
            note: None,
        });
        self.save()
    }

    /// Take back everything you said about one edge. `false` if you had said
    /// nothing.
    ///
    /// # Errors
    ///
    /// If your layer cannot be written.
    pub fn undo(&mut self, edge: &Edge) -> Result<bool, RepairError> {
        self.undo_named(&name_of(edge))
    }

    /// # Errors
    ///
    /// If your layer cannot be written.
    pub fn undo_named(&mut self, edge: &str) -> Result<bool, RepairError> {
        let gone = self.by_edge.remove(edge).is_some();
        self.save()?;
        Ok(gone)
    }

    fn save(&self) -> Result<(), RepairError> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }
        let io = |source: std::io::Error| RepairError::Io {
            path: self.path.display().to_string(),
            source,
        };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let mut body = String::new();
        for record in self.by_edge.values().flatten() {
            let line = serde_json::to_string(record)
                .map_err(|e| io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
            body.push_str(&line);
            body.push('\n');
        }
        let temp = self.path.with_extension("jsonl.writing");
        std::fs::write(&temp, body).map_err(io)?;
        std::fs::rename(&temp, &self.path).map_err(io)?;
        Ok(())
    }

    /// The shipped edges with your layer over them.
    ///
    /// Rejected edges come back **flagged rather than dropped**: a caller that
    /// draws links filters them out, and the repair UI shows them so that a
    /// rejection can be taken back.
    #[must_use]
    pub fn apply(&self, edges: Vec<Edge>) -> Vec<Repaired> {
        edges.into_iter().map(|edge| self.over(edge)).collect()
    }

    /// The links you drew that touch a segment, in either direction.
    ///
    /// Both directions, because a link you drew *to* this place is as much a
    /// link on this place as one you drew from it — spec.md §8.2 stores an edge
    /// once and derives the inverse, and this is where it is derived.
    #[must_use]
    pub fn drawn_touching(&self, at: &girsa_corpus::segment::SegmentId) -> Vec<Repaired> {
        self.drawn()
            .filter(|link| link.edge.from.covers(at) || link.edge.to.covers(at))
            .collect()
    }

    fn over(&self, shipped: Edge) -> Repaired {
        let records = self.about(&shipped);
        let mut repaired = Repaired::of(shipped.clone());
        for record in records {
            match &record.repair {
                Repair::Judged { verdict } => match verdict {
                    Verdict::Confirmed => {
                        repaired.confirmed = true;
                        repaired.rejected = false;
                    }
                    Verdict::Rejected => {
                        repaired.rejected = true;
                        repaired.confirmed = false;
                    }
                },
                Repair::Retyped { edge_type } => {
                    let Some(edge_type) = crate::touching::type_named(edge_type) else {
                        continue;
                    };
                    repaired.edge.edge_type = edge_type;
                }
                Repair::Reanchored { from, to } => {
                    let (Some(from), Some(to)) = (anchor(from), anchor(to)) else {
                        continue;
                    };
                    repaired.edge.from = from;
                    repaired.edge.to = to;
                }
                // A drawn edge is not an override of a shipped one.
                Repair::Drawn { .. } => continue,
            }
            repaired.changed.push(record.repair.as_str());
            repaired.who = Some(record.who.clone());
            repaired.when = Some(record.when);
        }
        if !repaired.changed.is_empty() {
            repaired.shipped = Some(shipped);
        }
        repaired
    }

    /// The links you drew that start in a sefer.
    #[must_use]
    pub fn drawn_in(&self, slug: &str) -> Vec<Repaired> {
        self.drawn()
            .filter(|link| link.edge.from.from.work() == slug)
            .collect()
    }

    /// Every link you drew.
    fn drawn(&self) -> impl Iterator<Item = Repaired> + '_ {
        self.by_edge.values().flatten().filter_map(|record| {
            let Repair::Drawn {
                from,
                to,
                edge_type,
            } = &record.repair
            else {
                return None;
            };
            let (from, to) = (anchor(from)?, anchor(to)?);
            let edge = Edge {
                from,
                to,
                edge_type: crate::touching::type_named(edge_type).unwrap_or(EdgeType::References),
                method: Method::ByHand,
                source_label: String::new(),
            };
            let mut repaired = Repaired::of(edge);
            repaired.mine = true;
            repaired.changed.push("drawn");
            repaired.who = Some(record.who.clone());
            repaired.when = Some(record.when);
            Some(repaired)
        })
    }

    /// Repairs about edges that are not in the graph handed in.
    ///
    /// Upstream dropped an edge, or re-segmented one of its ends. Kept rather
    /// than cleaned up — it is a thing you said — and reported, because a repair
    /// that quietly applies to nothing is a repair you think you made.
    #[must_use]
    pub fn orphans(&self, edges: &[Edge]) -> Vec<&Record> {
        let live: std::collections::HashSet<String> = edges.iter().map(name_of).collect();
        self.by_edge
            .iter()
            .filter(|(name, _)| !live.contains(*name))
            .flat_map(|(_, records)| records)
            .filter(|record| !matches!(record.repair, Repair::Drawn { .. }))
            .collect()
    }
}

fn anchor(text: &str) -> Option<Anchor> {
    match text.split_once("-girsa:") {
        Some((from, to)) => Some(Anchor::span(
            from.parse().ok()?,
            format!("girsa:{to}").parse().ok()?,
        )),
        None => Some(Anchor::point(text.parse().ok()?)),
    }
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// One edge as it stands after your layer, and what it was before.
#[derive(Debug, Clone, PartialEq)]
pub struct Repaired {
    pub edge: Edge,
    /// As the corpus shipped it, where your layer changed something.
    pub shipped: Option<Edge>,
    pub confirmed: bool,
    pub rejected: bool,
    /// You drew this one; it is in no shard.
    pub mine: bool,
    /// Which of §8.3's actions were applied, in the order they were made.
    pub changed: Vec<&'static str>,
    pub who: Option<String>,
    pub when: Option<u64>,
}

impl Repaired {
    fn of(edge: Edge) -> Self {
        Self {
            edge,
            shipped: None,
            confirmed: false,
            rejected: false,
            mine: false,
            changed: Vec::new(),
            who: None,
            when: None,
        }
    }

    /// How much to believe it, with your layer taken into account.
    ///
    /// A confirmed link, or one you drew, is certain — you are the authority on
    /// your own layer. Everything else keeps the confidence of the method it
    /// came in by (citation-addressed above line-indexed), and a rejected one is
    /// nothing at all.
    #[must_use]
    pub fn confidence(&self) -> f32 {
        if self.rejected {
            return 0.0;
        }
        if self.confirmed || self.mine {
            return 1.0;
        }
        self.edge.confidence()
    }

    /// Whether this may be shown to a reader as a statement about the texts.
    ///
    /// spec.md §8.3: **a blank-typed link is never presented as curated fact.**
    /// So: it says something (a type that is an assertion), or a person said it
    /// does — and a rejected one never is.
    #[must_use]
    pub fn is_curated(&self) -> bool {
        !self.rejected && (self.confirmed || self.mine || self.edge.edge_type.is_asserted())
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::segment::{Ordinal, SegmentId};

    fn id(work: &str, n: u32) -> SegmentId {
        SegmentId::new(work, vec!["1".into()], Ordinal::root(n))
    }

    fn edge() -> Edge {
        Edge {
            from: Anchor::point(id("a", 1)),
            to: Anchor::point(id("b", 2)),
            edge_type: EdgeType::References,
            method: Method::SefariaSeed,
            source_label: String::new(),
        }
    }

    #[test]
    fn an_edge_is_named_by_its_two_ends_so_the_file_is_greppable() {
        assert_eq!(name_of(&edge()), "girsa:a/1#1 → girsa:b/1#2");
    }

    #[test]
    fn retyping_twice_is_one_retype_and_the_last_word_stands() {
        let mut repairs = Repairs::nowhere();
        repairs
            .retype(&edge(), EdgeType::Quotes, "me")
            .expect("takes");
        repairs
            .retype(&edge(), EdgeType::CommentsOn, "me")
            .expect("takes");
        assert_eq!(repairs.count(), 1);
        let seen = repairs.apply(vec![edge()]);
        assert_eq!(seen[0].edge.edge_type, EdgeType::CommentsOn);
    }

    #[test]
    fn confirming_after_rejecting_replaces_the_verdict() {
        let mut repairs = Repairs::nowhere();
        repairs
            .judge(&edge(), Verdict::Rejected, "me")
            .expect("takes");
        repairs
            .judge(&edge(), Verdict::Confirmed, "me")
            .expect("takes");
        let seen = repairs.apply(vec![edge()]);
        assert!(seen[0].confirmed);
        assert!(!seen[0].rejected, "and not both at once");
    }

    #[test]
    fn a_link_to_itself_is_refused() {
        let mut repairs = Repairs::nowhere();
        let same = Anchor::point(id("a", 1));
        assert!(repairs
            .draw(same.clone(), same.clone(), EdgeType::Quotes, "me")
            .is_err());
        assert!(repairs.reanchor(&edge(), same.clone(), same, "me").is_err());
    }

    #[test]
    fn an_untouched_edge_carries_the_confidence_of_the_method_it_came_by() {
        let repairs = Repairs::nowhere();
        let seen = repairs.apply(vec![edge()]);
        assert!((seen[0].confidence() - Method::SefariaSeed.confidence()).abs() < f32::EPSILON);
        assert!(
            seen[0].shipped.is_none(),
            "nothing was changed, so nothing is shown as changed"
        );
    }
}
