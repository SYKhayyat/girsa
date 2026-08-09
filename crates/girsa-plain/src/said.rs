//! One sentence about what an answer could not see.
//!
//! # Why a crate this low holds a string joiner
//!
//! Three modules compose the sentence *here is what this answer did not look
//! at*, and each of them carries a doc comment saying it is the only one:
//!
//! | | says | claims to serve |
//! |---|---|---|
//! | `girsa_lane::coverage::Coverage::said` | what the semantic lane covers | "the window, the command line, the MCP surface and the test" |
//! | `girsa_app::reading::Gap::said` | unread scans, plus the layer clause | "the window's line, the CLI's line, the MCP server's line" |
//! | `girsa_note::since::Unindexed::said` | notes and corrections newer than the index | "the window's header, `girsa-read`'s line, `girsa-index find`'s footer and the MCP server's field" |
//!
//! Each was written to stop *its own* surfaces drifting, and none of them
//! could see the other two. What drifted was everything between them:
//! `Coverage` joins clauses with `; ` and formats `12904` as `12,904`; the
//! other two join with `" · "` and print the bare integer. `Gap::said` joins
//! its own clause list with `" · "` and then joins an already-joined string
//! from `Unindexed::said` into it, so two levels of the same sentence use one
//! separator by coincidence rather than by construction.
//!
//! So the three keep their clauses — they are the only ones who know how to
//! word them — and lose the joining. A [`Clauses`] is a flat list, and
//! [`Clauses::and`] flattens rather than nests, which is the whole of the fix
//! for the case above.
//!
//! # Why this is not a `Display` impl
//!
//! A clause list that has nothing to say is [`None`], not `""`. The difference
//! is the header disappearing versus the header rendering as an empty strip
//! with padding, and it is the reason `Gap::said` returns an `Option` and the
//! reason to keep returning one.

/// What goes between two clauses of one sentence.
///
/// A middle dot rather than a semicolon or a comma: the clauses are
/// independent facts about one answer, not a list and not a sequence, and in a
/// line that mixes Hebrew titles with English counts a `·` does not fight the
/// bidi algorithm the way a `;` does.
pub const BETWEEN: &str = " · ";

/// The clauses of one *what this answer could not see* sentence.
///
/// Flat by construction. See the module note.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Clauses {
    parts: Vec<String>,
}

impl Clauses {
    #[must_use]
    pub const fn new() -> Self {
        Self { parts: Vec::new() }
    }

    /// Add a clause.
    pub fn say(&mut self, clause: impl Into<String>) -> &mut Self {
        self.parts.push(clause.into());
        self
    }

    /// Add a clause only when there is something to count.
    ///
    /// The `n > 0` guard written once. Every one of the three composers had it
    /// spelled by hand around every clause, and `Coverage`'s copy spelled it as
    /// `!self.outside.is_empty()` — the same test, in a shape that cannot be
    /// searched for.
    pub fn count(&mut self, n: usize, clause: impl FnOnce(usize) -> String) -> &mut Self {
        if n > 0 {
            self.parts.push(clause(n));
        }
        self
    }

    /// Take another list's clauses, **flat**.
    pub fn and(&mut self, other: Self) -> &mut Self {
        self.parts.extend(other.parts);
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    #[must_use]
    pub fn parts(&self) -> &[String] {
        &self.parts
    }

    /// The sentence, or `None` when there is nothing to say.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        if self.parts.is_empty() {
            None
        } else {
            Some(self.parts.join(BETWEEN))
        }
    }

    /// The sentence, for a composer that always has one.
    ///
    /// Empty string when there are no clauses, which is a state
    /// [`girsa_lane::coverage::Coverage`] does not have and the other two do.
    #[must_use]
    pub fn joined(&self) -> String {
        self.parts.join(BETWEEN)
    }
}

impl FromIterator<String> for Clauses {
    fn from_iter<I: IntoIterator<Item = String>>(parts: I) -> Self {
        Self {
            parts: parts.into_iter().collect(),
        }
    }
}

/// `12904` as `12,904`.
///
/// A five-figure number with no separator in a sentence about how much of a
/// library is covered is a number nobody reads — which the lane's copy of this
/// function argued and the other two composers, printing `{n}` into the same
/// header, did not know had been argued.
#[must_use]
pub fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// `one` when `n` is 1, `many` otherwise.
///
/// Trivial, and worth a name: the three composers between them wrote this
/// ternary **eleven** times, and one of the eleven is why *"words you corrected
/// on 1 scan are"* and *"words you corrected on 2 scans are"* had to be spelled
/// as two whole phrases rather than a noun and a verb.
#[must_use]
pub fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 {
        one
    } else {
        many
    }
}

/// `1 PDF`, `4 PDFs` — a count and its noun, with the separator and the
/// agreement both taken care of.
#[must_use]
pub fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{} {}", thousands(n), plural(n, one, many))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_list_with_nothing_in_it_is_silence_and_not_an_empty_header() {
        assert_eq!(Clauses::new().said(), None);
        assert_eq!(Clauses::new().joined(), "");
    }

    #[test]
    fn taking_another_list_flattens_it() {
        // The bug this type exists for. `Gap::said` joined its own clauses with
        // `" · "` and pushed `Unindexed::said`'s *already joined* string in as
        // one clause, so a sentence with four clauses in it had a nesting
        // nobody could see because both levels happened to use one separator.
        let mut layer = Clauses::new();
        layer.say("two notes").say("one correction");
        let mut gap = Clauses::new();
        gap.say("4 PDFs").and(layer);
        assert_eq!(gap.parts().len(), 3, "{:?}", gap.parts());
        assert_eq!(gap.said().unwrap(), "4 PDFs · two notes · one correction");
    }

    #[test]
    fn a_count_of_nothing_says_nothing() {
        let mut clauses = Clauses::new();
        clauses
            .count(0, |n| format!("{n} notes"))
            .count(2, |n| format!("{n} corrections"));
        assert_eq!(clauses.said().unwrap(), "2 corrections");
    }

    #[test]
    fn thousands_are_grouped_and_small_numbers_are_left_alone() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(12_904), "12,904");
        assert_eq!(thousands(5_000_545), "5,000,545");
    }

    #[test]
    fn one_is_singular_and_nothing_is_plural() {
        // Zero takes the plural — *0 seforim*, not *0 sefer* — which is the one
        // case a hand-written `if n == 1` gets right by accident and a hand
        // written `if n > 1` gets wrong.
        assert_eq!(counted(0, "sefer", "seforim"), "0 seforim");
        assert_eq!(counted(1, "sefer", "seforim"), "1 sefer");
        assert_eq!(counted(4_182, "sefer", "seforim"), "4,182 seforim");
    }
}
