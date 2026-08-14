//! Your corrections, on their way into the index (BUILDER.md W20, W11).
//!
//! The index is built from the corpus files, and the corpus files are the sefer
//! as it was scanned. Everything else a reader looks at goes through the
//! overlay: the reading pane draws it corrected, a quote copied to Ksav carries
//! the corrected words, an export writes them and says in its header that it
//! did. A search did not — so a typo fixed this morning was findable **by the
//! typo and not by the word**, which is the one surface where a correction
//! looked like it had not been made.
//!
//! Rebuilding per correction is not the answer and neither is a second index.
//! What this is instead is the overlay taught to the indexer: one read of the
//! layer per build, and a segment handed to tantivy as the reader reads it.
//! `spec.md` §4.1 is untouched — **never the text**. Nothing here writes to the
//! corpus; the base text on disk is exactly what it was, and this is a cache
//! being built over the corpus *and* what you have said about it.
//!
//! # Why it is a list of layers and not one
//!
//! A build is handed every root and is not told which of them is the personal
//! one. It cannot work it out from the work being indexed either: a correction
//! to a Sefaria sefer lives under `personal/` while the sefer itself lives
//! under `corpus/`. So every root is asked for a corrections file, and the ones
//! that have something are kept. On the ordinary two-root install exactly one
//! does.
//!
//! # Why the cost is nothing
//!
//! [`Corrections::touch`] is asked once per work, and a work nobody has
//! corrected skips the standing derivation and the apply entirely. On a real
//! shelf that is 7,189 works out of 7,189 minus a handful, so a five-million
//! segment build pays for this on the seforim somebody has actually edited.

use std::path::Path;

use girsa_corpus::standing::Standing;

/// The reader's corrections, from wherever under these roots they are kept.
#[derive(Default)]
pub struct Corrections {
    layers: Vec<girsa_fix::Layer>,
}

impl Corrections {
    /// Open every layer there is, and say what would not read.
    ///
    /// A root with no corrections file is the ordinary state of a fresh
    /// install and is not trouble; a file that will not parse is, and is
    /// reported by name the way every other store in this repository reports
    /// one.
    #[must_use]
    pub fn of<S: AsRef<Path>>(roots: &[S]) -> (Self, Vec<String>) {
        let mut layers = Vec::new();
        let mut trouble = Vec::new();
        for root in roots {
            let (layer, said) = girsa_fix::Layer::open(root.as_ref());
            trouble.extend(said);
            if layer.count() > 0 {
                layers.push(layer);
            }
        }
        (Self { layers }, trouble)
    }

    /// Nothing has been corrected anywhere.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// How many corrections there are altogether.
    #[must_use]
    pub fn count(&self) -> usize {
        self.layers.iter().map(girsa_fix::Layer::count).sum()
    }

    /// Whether anything at all has been corrected in this sefer.
    ///
    /// The cheap question, asked once per work — see the module note.
    #[must_use]
    pub fn touch(&self, slug: &str) -> bool {
        self.layers.iter().any(|layer| layer.touches(slug))
    }

    /// One segment as the reader reads it, if that is not what the file says.
    ///
    /// [`girsa_fix::Showing::Fixed`] — scanning errors repaired, girsa variants
    /// noted and not applied. The same setting the reading pane defaults to,
    /// because a search that found words the pane does not show would be a
    /// result a reader cannot see when they get there. A variant is a claim
    /// about what the text *should* say; an index is about what it does say.
    ///
    /// Asked with a [`Standing`] rather than with the id, so a correction made
    /// before upstream cut the se'if it was on still applies.
    /// `girsa_fix::Layer::on` takes exact equality and would have missed it
    /// silently — and a correction that stops applying without saying so is the
    /// failure that whole type exists to prevent.
    ///
    /// # `None` is *nothing here changes what gets indexed*
    ///
    /// Deliberately **not** `girsa_fix::Corrected::is_untouched`, which also
    /// counts corrections that were merely *noted*. A noted variant leaves the
    /// text exactly as the corpus has it, so a segment carrying nothing but
    /// variants goes down the ordinary path and the index is bit-identical to
    /// one built with no layer at all. That is the claim worth being able to
    /// make: turning a variant on changes what the reader is shown and never
    /// what a query can reach.
    #[must_use]
    pub fn text(&self, at: &Standing, base: &str) -> Option<Reading> {
        for layer in &self.layers {
            let corrected = layer.apply_at(at, base, girsa_fix::Showing::Fixed);
            if !corrected.applied.is_empty() || !corrected.stale.is_empty() {
                return Some(Reading {
                    applied: corrected.applied.len(),
                    stale: corrected.stale.len(),
                    text: corrected.text,
                });
            }
        }
        None
    }
}

/// One segment's words after the overlay, and what the overlay did to get here.
pub struct Reading {
    pub text: String,
    /// How many corrections landed.
    ///
    /// Zero is a real answer and not a no-op: a segment can carry a correction
    /// whose words are no longer there, and then the text is the corpus's and
    /// `stale` is why.
    pub applied: usize,
    /// Corrections whose words this work no longer has.
    ///
    /// Counted and reported by the build, never swallowed. It is the reader's
    /// to decide about — `girsa_fix::Corrected::stale` says the same thing to
    /// the reading pane and to an export, and a build that dropped them quietly
    /// would be the one surface where they vanished.
    pub stale: usize,
}
