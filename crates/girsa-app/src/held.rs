//! How many seforim are kept in memory, and which one goes when one must.
//!
//! # The bug this module is named after
//!
//! The window kept twelve open seforim in a `HashMap` and their slugs in a
//! `Vec`, pushed on read and removed from the front when the twelfth arrived.
//! That is a **queue, not a cache**: a hit never touched the order, so the
//! sefer you have been reading all morning was evicted on its twelfth
//! neighbour because it happened to be opened first, and the commentary you
//! glanced at once outlived it.
//!
//! A masechta with its mefarshim is four or five works. Twelve is small on
//! purpose — a work is tens of megabytes of text and a reader has a handful
//! open, not a library — and *small and wrong about which one to drop* is
//! exactly the combination that makes the cost visible: the eviction re-reads
//! the biggest file the reader is using, in the middle of them using it.
//!
//! # Why it is not in the shell
//!
//! The README says `app/` is *"a window and fifty commands, and nothing that
//! decides anything"*. How long a sefer stays in memory is a decision, it was
//! being made there, and being made there is why nothing tested it. Every
//! sentence in this module note is a test below.

use std::collections::HashMap;

/// How many seforim are kept in memory at once.
///
/// A masechta with its commentaries is four or five; the number is small
/// because a work is tens of megabytes of text and a reader has a handful open,
/// not a library.
pub const KEEP_OPEN: usize = 12;

/// The most recently used `KEEP_OPEN` of something, by key.
///
/// Generic because what is held is not the interesting part — the order is.
/// `Open` seforim are what the window holds; the tests below hold numbers, so
/// that what they assert is the eviction and not the corpus.
#[derive(Debug)]
pub struct Held<T> {
    kept: HashMap<String, T>,
    /// Least recently used first. The one that goes is `order[0]`.
    order: Vec<String>,
    room: usize,
}

impl<T> Default for Held<T> {
    fn default() -> Self {
        Self::new(KEEP_OPEN)
    }
}

impl<T> Held<T> {
    /// Room for `room` of them.
    #[must_use]
    pub fn new(room: usize) -> Self {
        Self {
            kept: HashMap::new(),
            order: Vec::new(),
            room: room.max(1),
        }
    }

    /// Whether this one is already here — **without** counting as a use.
    ///
    /// For asking *would this be free?*. Reading it is [`Held::get`], and that
    /// one does count.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.kept.contains_key(key)
    }

    /// How many are held.
    #[must_use]
    pub fn count(&self) -> usize {
        self.kept.len()
    }

    /// The one held under this key, if it is here, **and it is now the most
    /// recently used**.
    ///
    /// That last clause is the whole module. Without it this is a queue.
    pub fn get(&mut self, key: &str) -> Option<&T> {
        if !self.kept.contains_key(key) {
            return None;
        }
        self.touch(key);
        self.kept.get(key)
    }

    /// The one held under this key **without** counting as a use.
    ///
    /// For decoration: the sidebar quotes the first words of the far end of a
    /// link where that sefer happens to be open already, and a row drawn
    /// beside what you are reading is not you reading it. Promoting it would
    /// let a list of forty links reorder the whole cache without anybody
    /// opening anything.
    #[must_use]
    pub fn peek(&self, key: &str) -> Option<&T> {
        self.kept.get(key)
    }

    /// Put one in, evicting the least recently used if there is no room.
    ///
    /// Returns the key of whatever was dropped, so a caller holding anything
    /// derived from it can drop that too — the window holds a marks table per
    /// sefer, and a marks table for a sefer nobody has open is the same
    /// megabytes with none of the use.
    pub fn put(&mut self, key: &str, held: T) -> Option<String> {
        if self.kept.insert(key.to_string(), held).is_some() {
            self.touch(key);
            return None;
        }
        self.order.push(key.to_string());
        if self.kept.len() <= self.room {
            return None;
        }
        let gone = self.order.first().cloned()?;
        self.order.remove(0);
        self.kept.remove(&gone);
        Some(gone)
    }

    /// Forget one, whether or not it is here.
    pub fn forget(&mut self, key: &str) {
        self.kept.remove(key);
        self.order.retain(|k| k != key);
    }

    /// Forget all of them — a new shelf is a new set of seforim.
    pub fn clear(&mut self) {
        self.kept.clear();
        self.order.clear();
    }

    /// Everything held, in no particular order.
    pub fn all(&self) -> impl Iterator<Item = (&String, &T)> {
        self.kept.iter()
    }

    /// Move a key to the most-recently-used end.
    fn touch(&mut self, key: &str) {
        if let Some(at) = self.order.iter().position(|k| k == key) {
            // A `Vec` of twelve. A linked list would be the textbook answer and
            // would cost an allocation per node to beat eleven pointer moves.
            let key = self.order.remove(at);
            self.order.push(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> Held<usize> {
        let mut held = Held::new(3);
        for (at, key) in ["a", "b", "c"].iter().enumerate() {
            held.put(key, at);
        }
        held
    }

    #[test]
    fn the_one_that_goes_is_the_one_nobody_has_looked_at() {
        let mut held = full();
        // Read `a`, which is the oldest by arrival and the newest by use.
        assert_eq!(held.get("a"), Some(&0));
        let gone = held.put("d", 3);
        assert_eq!(gone.as_deref(), Some("b"), "b was the least recently used");
        assert!(held.has("a"), "the one being read survived");
    }

    #[test]
    fn a_queue_would_have_dropped_the_one_being_read() {
        // The shell's version, exactly: push on read, remove the front. This
        // is what it did, and it is why the sefer you had open all morning was
        // the one that went.
        let mut held = full();
        held.get("a");
        let gone = held.put("d", 3);
        assert_ne!(gone.as_deref(), Some("a"));
    }

    #[test]
    fn putting_the_same_one_twice_does_not_use_up_room() {
        let mut held = full();
        assert_eq!(held.put("a", 99), None, "nothing was evicted");
        assert_eq!(held.count(), 3);
        assert_eq!(held.get("a"), Some(&99), "and it is the new one");
    }

    #[test]
    fn asking_whether_it_is_here_is_not_reading_it() {
        let mut held = full();
        assert!(held.has("a"));
        // `has` did not touch the order, so `a` is still the oldest.
        assert_eq!(held.put("d", 3).as_deref(), Some("a"));
    }

    #[test]
    fn looking_at_a_row_about_it_is_not_reading_it() {
        // Forty link rows quoting forty far ends would otherwise reorder the
        // whole cache without the reader opening a thing.
        let mut held = full();
        assert_eq!(held.peek("a"), Some(&0));
        assert_eq!(held.put("d", 3).as_deref(), Some("a"));
    }

    #[test]
    fn what_was_dropped_is_named_so_the_rest_of_it_can_be_dropped_too() {
        // The window holds a marks table per open sefer. Evicting the sefer
        // and keeping its marks is the same megabytes with none of the use.
        let mut held = full();
        assert_eq!(held.put("d", 3).as_deref(), Some("a"));
        assert_eq!(held.count(), 3, "and it did not grow");
    }

    #[test]
    fn room_for_none_is_room_for_one() {
        // A cache of zero would re-read the sefer on every question about it,
        // which is not a smaller cache but a different program.
        let mut held: Held<usize> = Held::new(0);
        held.put("a", 1);
        assert_eq!(held.get("a"), Some(&1));
    }

    #[test]
    fn forgetting_one_that_is_not_here_is_not_an_event() {
        let mut held = full();
        held.forget("z");
        assert_eq!(held.count(), 3);
        held.forget("b");
        assert_eq!(held.count(), 2);
        assert!(!held.has("b"));
    }

    #[test]
    fn twelve_is_what_the_window_keeps() {
        // Written down here rather than in the shell, because the README says
        // the shell decides nothing and this is a decision.
        assert_eq!(KEEP_OPEN, 12);
        let held: Held<usize> = Held::default();
        assert_eq!(held.room, KEEP_OPEN);
    }
}
