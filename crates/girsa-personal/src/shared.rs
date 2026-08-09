//! The three things every store in the personal layer needed, in the crate they
//! all already depend on.
//!
//! # Why here, and why this is not a junk drawer
//!
//! The 9 August report's §4 measured what the `girsa-fix` / `girsa-note` wall
//! costs. The wall itself is right — they are siblings, neither may name the
//! other, and `since.rs:311` says so — but a wall between two crates does not
//! stop them needing the same primitive, it only stops them sharing one:
//!
//! > The wall costs a duplicated FNV-1a hash (`fix/lib.rs:141` / `note/mark.rs:77`,
//! > byte-identical down to a non-standard `0xff` separator), a duplicated
//! > `"corrections.jsonl"` literal, four `now_seconds()`, five identical
//! > `From<LogError>`, three identical `to_text()`, and one function invented
//! > purely to route around the wall.
//!
//! The route around the wall was already found once, and its own doc comment
//! records the rule that made it work — `girsa_note::since`, on why it counts
//! records rather than naming `Patch`:
//!
//! > *The answer was never to name `Patch`. **Counting records in a log is a
//! > fact about the log format**, and the format is `girsa-personal`'s, which
//! > both crates already depend on — the same argument that already put
//! > `is_tombstone` there.*
//!
//! Everything in this module passes that test and nothing else may be added
//! that does not. An id derived from a record's fields, the second a record was
//! written, and the name of the file corrections go in are all facts about *the
//! personal layer's format*, which is this crate's subject. A helper that is
//! merely wanted by two crates is not — it goes where its subject is, or it
//! stays duplicated, which is the cheaper mistake.
//!
//! `From<LogError>` and `to_text` are the same finding and are answered
//! elsewhere in this crate: the first by [`crate::store::io_from_log_error`],
//! the second by [`crate::Store::to_text`], because both are about a store
//! rather than about a value.

/// The file your corrections are written to, under a personal root.
///
/// One string. `girsa-fix` owns corrections and `girsa-note::since` counts what
/// is newer than the index, and the wall between them meant the *name of a file*
/// was written out twice — so a rename would have left one crate counting a file
/// nobody writes and reporting zero, which is the reading this whole `since`
/// mechanism exists to prevent.
pub const CORRECTIONS: &str = "corrections.jsonl";

/// FNV-1a, 64-bit, over each part with a `0xff` between them.
///
/// Small, dependency-free and deterministic across machines, which is the only
/// property an id derived this way needs — a collision would have to be two
/// different records about the same place.
///
/// # The separator is not standard, and that is why this is one function
///
/// `hash ^= 0xff` between parts is not part of FNV-1a. It is here so that
/// `["ab", "c"]` and `["a", "bc"]` hash differently, which they must, because
/// the parts are a segment id and a span and concatenating them would make two
/// different corrections collide.
///
/// It existed twice — `girsa_fix::fingerprint` and `girsa_note::mark::fingerprint`
/// — byte-identical *including* that non-standard step, which is the tell: two
/// copies of an algorithm agree until one of them is improved. These ids are on
/// disk in readers' personal layers, so an improvement to either would silently
/// stop matching every record already written.
#[must_use]
pub fn fingerprint(parts: &[&str]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("{hash:016x}")
}

/// Now, in seconds since the epoch.
///
/// Zero when the clock is before 1970, which is a machine nobody can reason
/// about and a record that sorts first — the safe direction for a `when` field
/// whose only job is ordering.
///
/// Four copies, in `girsa-desk`, `girsa-fix`, `girsa-link` and `girsa-note`, all
/// stamping records in the same log format. The stamp is part of the format.
#[must_use]
pub fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_separator_is_what_keeps_two_corrections_apart() {
        // The reason this is not plain FNV-1a. Without the `0xff` these two
        // would be the same hash, and two corrections to different spans of one
        // segment would be one patch id.
        assert_ne!(fingerprint(&["ab", "c"]), fingerprint(&["a", "bc"]));
    }

    #[test]
    fn an_id_is_the_same_id_on_every_machine() {
        // Pinned to the literal digest, because these are on disk in readers'
        // personal layers: a change here does not produce different ids, it
        // produces ids that stop matching records already written.
        assert_eq!(
            fingerprint(&["girsa:bavli/berakhot/2a#1", "0", "7"]),
            fingerprint(&["girsa:bavli/berakhot/2a#1", "0", "7"]),
        );
        assert_eq!(fingerprint(&[]), "cbf29ce484222325");
        // One empty part is `0xff` and nothing else, which is *not* FNV-1a's
        // digest of the empty string — the separator is applied per part.
        assert_eq!(fingerprint(&[""]), "af64724c8602eb6e");
    }

    #[test]
    fn the_clock_going_backwards_is_not_a_panic() {
        // It cannot be tested by moving the clock, but the shape can: the
        // function has no `unwrap`, and a stamp of zero sorts first rather than
        // taking the reader's window with it.
        assert!(now_seconds() > 1_700_000_000, "after November 2023");
    }
}
