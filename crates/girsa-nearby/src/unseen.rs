//! What one answer could not see — all of it, in one sentence.
//!
//! # The thing three composers each thought they were
//!
//! `girsa_lane::Coverage`, [`girsa_app::reading::Gap`] and
//! `girsa_note::since::Unindexed` each say part of *this answer did not look
//! everywhere*, and each carries a doc comment naming itself the only
//! implementation so the surfaces cannot drift. They were right about their own
//! clause and wrong about the sentence, because none of the three could see the
//! other two:
//!
//! - a `search` answer carried `did_not_search` — the layer clauses, and not the
//!   unread scans on the same shelf, and not the lane;
//! - an `adjacent` answer carried `coverage` — the lane, and nothing about the
//!   notes written since the index was built;
//! - the window's results header carried `said` — scans and layer, and nothing
//!   about the lane;
//! - and nothing anywhere composed the two.
//!
//! So a reader whose shelf holds four unread PDFs, a chaburah written this
//! morning, and a lane over eleven per cent of the library got **three
//! different subsets of one truth depending on which surface they asked**, and
//! each subset arrived wearing a sentence that said it was complete.
//!
//! # What this type is, and what it is not
//!
//! It is the joining, and nothing else. Every clause is still worded by the
//! module that owns the fact — that part of the three doc comments was correct
//! and is kept. What moved here is the decision about *which clauses belong in
//! one answer*, which is the only decision none of the three was in a position
//! to make.
//!
//! # Why the lane clause is optional and the literal one is not
//!
//! Every search reads the literal index, so there is always a literal gap to
//! report even when it is empty. The lane is off unless the reader turned it on
//! (spec.md §9.9), and *"nothing is in the semantic lane yet"* on the header of
//! a reader who has never asked for a lane is noise about a feature they have
//! not met. So: `None` means *not part of this answer*, which is a different
//! thing from `Coverage::default()`, which means *on, and covering nothing*.

use girsa_plain::said::Clauses;
use girsa_lane::Coverage;
use girsa_note::since::Unindexed;

use girsa_app::reading::Gap;

/// Everything one answer did not look at.
#[derive(Debug, Clone)]
pub struct Unseen {
    /// Unread scans, and what your own layer holds that the index has not seen.
    pub literal: Gap,
    /// What the semantic lane covers, when this answer used one.
    pub lane: Option<Coverage>,
}

impl Unseen {
    /// Both halves.
    #[must_use]
    pub fn of(literal: Gap, lane: Option<Coverage>) -> Self {
        Self { literal, lane }
    }

    /// What a caller that knows its layer but has not walked the shelf can say.
    ///
    /// The MCP server's shape: it reads [`Unindexed`] once at open — deliberately,
    /// because a program asking `search` is entitled to the header the window
    /// shows and a `stat` per note per call is not — and it has never counted
    /// pages. A gap with no scans in it is honest about that; a gap that was
    /// never asked for at all was what it had before.
    #[must_use]
    pub fn over_layer(layer: Unindexed, lane: Option<Coverage>) -> Self {
        Self {
            literal: Gap {
                scans: Vec::new(),
                pages: 0,
                layer,
            },
            lane,
        }
    }

    /// Only what the literal index could not see. The lane was not consulted.
    #[must_use]
    pub fn literal(literal: Gap) -> Self {
        Self {
            literal,
            lane: None,
        }
    }

    /// Whether there is nothing to say.
    ///
    /// A lane that was consulted always has something to say about its own
    /// coverage — see `Coverage::said` — so an answer with a lane in it is never
    /// silent.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.literal.is_none() && self.lane.is_none()
    }

    /// The clauses, in the order a reader wants them.
    ///
    /// Literal first: *this is not searchable yet* is a thing the reader can go
    /// and fix, and *the lane covers this much* is a standing fact about a
    /// feature. A line that leads with the standing fact buries the actionable
    /// one.
    #[must_use]
    pub fn clauses(&self) -> Clauses {
        let mut clauses = self.literal.clauses();
        if let Some(lane) = &self.lane {
            clauses.and(lane.clauses());
        }
        clauses
    }

    /// The whole sentence, or `None` when there is nothing to say.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        self.clauses().said()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use girsa_lane::coverage::Covered;
    use girsa_note::since::Written;

    use super::*;
    use girsa_app::reading::Scanned;

    fn scanned() -> Scanned {
        Scanned {
            slug: "user/shas".to_string(),
            title: "ש\"ס".to_string(),
            pages: 302,
            read: 40,
        }
    }

    fn layer(notes: usize, fixes: usize) -> Unindexed {
        Unindexed {
            notes: Written::Since(notes),
            fixes: Written::Since(fixes),
            scans: Written::Since(0),
        }
    }

    fn lane() -> Coverage {
        Coverage::of(
            [Covered {
                slug: "berakhot".to_string(),
                title: "ברכות".to_string(),
                wanted: 12_904,
                embedded: 12_904,
            }],
            ["מגילה".to_string()],
            false,
        )
    }

    #[test]
    fn one_answer_says_all_three_things_at_once() {
        // The state a real reader is in, and the state no surface could report:
        // unread scans, a chaburah written this morning, and a lane over one
        // sefer of a shelf holding two.
        let unseen = Unseen::of(
            Gap {
                scans: vec![scanned()],
                pages: 262,
                layer: layer(1, 0),
            },
            Some(lane()),
        );
        let said = unseen.said().expect("three things to say");
        assert!(said.contains("1 PDF"), "{said}");
        assert!(said.contains("1 note"), "{said}");
        assert!(said.contains("this lane covers ברכות"), "{said}");
        assert_eq!(
            unseen.clauses().parts().len(),
            4,
            "flat, not nested: {:?}",
            unseen.clauses().parts()
        );
    }

    #[test]
    fn every_clause_uses_one_separator() {
        // The drift this type exists to end. `Coverage` joined with `; `, the
        // other two with `" · "`, and a sentence built from all three would have
        // read as two sentences with different punctuation rules.
        let unseen = Unseen::of(
            Gap {
                scans: vec![scanned()],
                pages: 262,
                layer: layer(1, 2),
            },
            Some(lane()),
        );
        let said = unseen.said().expect("something to say");
        assert!(!said.contains(';'), "a semicolon survived: {said}");
        assert_eq!(
            said.matches(girsa_plain::said::BETWEEN).count(),
            unseen.clauses().parts().len() - 1,
            "{said}"
        );
    }

    #[test]
    fn every_number_in_the_sentence_is_grouped() {
        // `Coverage` knew a five-figure number wants a comma. The other two
        // printed `{n}` into the same header, so one line could carry both
        // `12,904` and `12904`.
        let unseen = Unseen::of(
            Gap {
                scans: vec![scanned()],
                pages: 1_234,
                layer: layer(0, 0),
            },
            Some(lane()),
        );
        let said = unseen.said().expect("something to say");
        assert!(said.contains("1,234 pages"), "{said}");
        assert!(said.contains("12,904 segments"), "{said}");
    }

    #[test]
    fn a_lane_nobody_turned_on_is_not_mentioned() {
        // `None` is *not part of this answer*; `Coverage::default()` is *on and
        // covering nothing*. Only the second one has a sentence.
        let quiet = Unseen::literal(Gap::none());
        assert_eq!(quiet.said(), None);
        assert!(quiet.is_none());

        let on = Unseen::of(Gap::none(), Some(Coverage::default()));
        assert_eq!(
            on.said().as_deref(),
            Some(girsa_lane::coverage::NOTHING_YET)
        );
        assert!(!on.is_none());
    }

    #[test]
    fn a_server_that_only_knows_its_layer_still_says_the_layer() {
        // What `girsa-mcp` had — and it had it under a field name only it used.
        let unseen = Unseen::over_layer(layer(3, 0), None);
        let said = unseen.said().expect("three notes");
        assert!(said.contains("3 notes"), "{said}");
        assert!(!said.contains("PDF"), "it never counted pages: {said}");
    }
}
