//! Reading a Bar-Ilan / DBS export: **the header line carries its own address.**
//!
//! This runs for files in an Otzaria-shaped tree that carry no `<h1>` — the
//! OtzarLib ones under a folder its own author named
//! *ספרים שאינם מותאמים לאוצריא*, "not adapted for Otzaria". They are not
//! unstructured. They are structured a different way, and the way is legible.
//!
//! # What a header looks like
//!
//! ```text
//! שו"ת מהר"י בן לב חלק א סימן א
//! ראב"ד - תשובות ופסקים סימן א
//! ראבי"ה חלק ב - מסכת סוכה סימן תרמד
//! ר' חיים הלוי הלכות יסודי התורה פרק ה
//! @שו"ת מהרש"ם חלק א סימן א
//! ```
//!
//! Every one of those is a **complete address written out in words**: a level
//! word and the numeral it labels, as many times as the sefer is deep, with the
//! parts that are named rather than numbered spelled out in between. So the
//! address does not have to be inferred from counting, guessed from
//! indentation, or assembled from a stack of open sections. It is on the line.
//!
//! Which level words count is not decided here — [`is_section_word`] is
//! `girsa-ref`'s own list, made public for this caller. A second copy of that
//! list is a thing this project has already paid for once: when the resolver's
//! list drifted from its normalized copy, `סעיף` fell through to the numeral
//! reader, became 220, and a four-level address resolved to four wrong levels.
//!
//! # How a header is told from a line that merely mentions one
//!
//! Three signals, and they are not equal. Two are marks the exporter put there
//! on purpose and are believed on sight; the third is inferred and has to be
//! corroborated.
//!
//! - **A rule under it** — a line of `=` on the line below. Explicit.
//! - **A leading `@`.** Explicit.
//! - **An address at the head of the line that continues the run.** Inferred,
//!   and *the run is the corroboration*. `מהר"י בן לב` says `סימן ב` inside its
//!   own prose constantly; what it does not do is say it at the head of a line
//!   at the exact point a sequence of 371 simanim has reached `סימן א`. A
//!   mention cannot slot into a sequence.
//!
//!   This is also what keeps another sefer's name out. A teshuvos sefer cites
//!   somebody on nearly every line, and `שולחן ערוך אורח חיים סימן ב` parses
//!   into an address as happily as anything else — but its first level is the
//!   *name* `שולחן ערוך אורח חיים` where the run's first level is a number, and
//!   two levels that are not both numbers cannot be compared, so it is text.
//!
//! Rule 6 is why the third signal is bounded rather than trusted: a candidate
//! that goes backwards, or forwards by more than [`A_PLAUSIBLE_GAP`], is read
//! as ordinary text. Seforim skip numbers — a siman that was never printed — so
//! demanding an exact `+1` would end the run at the first gap and swallow the
//! rest of the sefer into one section.
//!
//! # What is deliberately not here
//!
//! **No stack.** [`super::otzaria`] keeps one, because an Otzaria `<h3>` says
//! only *siman א* and needs its parents to know whose siman א it is. A DBS
//! header states its full address, so a header sets the path outright. Nesting
//! it would be inventing a hierarchy the file does not claim.
//!
//! **No refusal.** A file where nothing matches is not an error: every line
//! becomes front matter, section `0`, which is what [`super::otzaria`] does with
//! a file that has no headings. A sefer addressed as one long section is worse
//! than a sefer addressed properly and far better than a sefer that did not
//! import at all.

use girsa_hebrew::normalize;
use girsa_ref::numerals::parse_hebrew;
use girsa_ref::resolve::is_section_word;

use super::{RawSegment, SegmentKind};
use crate::work::{match_key, section_label_of};

/// The label of the section holding everything above the first header.
///
/// `0`, and for the reason [`super::otzaria`] gives: the first header of most of
/// these seforim opens siman א, and front matter called `1` would be confused
/// with it constantly.
const FRONT_MATTER: &str = "0";

/// How far the deepest level of an address may jump and still be believed.
///
/// Only used for the inferred signal. Seforim skip numbers — the Rashba's
/// simanim are not a dense sequence — so the run cannot demand `+1`. Twenty is
/// wide enough for the real gaps in these files and narrow enough that a
/// numeral out of the middle of a sentence does not reach it.
const A_PLAUSIBLE_GAP: u32 = 20;

/// How long a level of an address may be before it stops being one.
///
/// **A header is short.** The longest real ones measured across this library are
/// under fifty characters — `אנציקלופדיה תלמודית כרך א, אב ג (עיקר) [טור יז]`,
/// `ראבי"ה מפתח הסימנים שאלות ותשובות וענינים שונים` — and prose in the same
/// files runs to hundreds.
///
/// Without this, `מוסר ודעת` had **14,019 of its 34,361 segments** filed under
/// three section labels that were whole paragraphs, one of them 9,144
/// characters long. The cause is the explicit signal being believed on sight: a
/// `====` rule sitting under an ordinary paragraph is exporter noise or a
/// divider, and reading it as a heading turns a quarter of a sefer into one
/// section named after a sentence. A permanent id is a thing somebody has to be
/// able to read out loud.
const A_LEVEL_A_READER_COULD_TYPE: usize = 120;

/// Is this line a rule — the `====` an exporter draws under a header?
///
/// Four, not three: `===` turns up inside prose often enough to matter, and a
/// rule that short is not something a typesetter drew.
fn is_rule(line: &str) -> bool {
    let line = line.trim();
    line.len() >= 4 && line.chars().all(|c| c == '=')
}

/// Read a DBS export into segments, in reading order.
///
/// `title` is the work's own name. It is used for one thing — recognising it at
/// the front of a header so the address does not carry the sefer's own name in
/// every level — and a title that does not match costs nothing but a longer
/// first level.
#[must_use]
pub fn parse(body: &str, title: &str) -> Vec<RawSegment> {
    parse_with_lines(body, title)
        .into_iter()
        .map(|(_, segment)| segment)
        .collect()
}

/// The same, keeping the 1-based line each segment came from.
///
/// [`super::otzaria::parse_with_lines`] explains why this exists and why the
/// number does not leave: Otzaria's link files address both ends by line, so
/// translating that addressing into ours needs the mapping for the length of
/// one function and then never again.
#[must_use]
pub fn parse_with_lines(body: &str, title: &str) -> Vec<(usize, RawSegment)> {
    let title: Vec<String> = words(title);
    let lines: Vec<&str> = body.lines().collect();
    // Read every line's address first, because deciding whether *this* line is
    // a header needs to know what the rest of the file looks like — see
    // [`the_spine`].
    let candidates: Vec<Option<Vec<String>>> = lines
        .iter()
        .map(|l| {
            let line = l.trim();
            let text = line.strip_prefix('@').unwrap_or(line).trim();
            (!text.is_empty() && !is_rule(text))
                .then(|| address_of(text, &title))
                .flatten()
        })
        .collect();
    let inferred = the_spine(&candidates);
    let mut out = Vec::new();

    // The section everything sits in until a header says otherwise, and the
    // count of lines already placed inside it.
    let mut current: Vec<String> = vec![FRONT_MATTER.to_string()];
    let mut placed = 0usize;

    for (n, raw) in lines.iter().enumerate() {
        let line_number = n + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if is_rule(line) {
            // Exporter furniture. A rule belongs to the header above it and is
            // not a line anybody reads.
            continue;
        }

        let marked = line.starts_with('@');
        let text = line.strip_prefix('@').unwrap_or(line).trim();
        let explicit = marked || rule_follows(&lines, n);

        if let Some(levels) = candidates[n].clone() {
            let believed = explicit || inferred.contains(&n);
            if believed {
                current = levels;
                placed = 0;
                out.push((
                    line_number,
                    RawSegment {
                        path: current.clone(),
                        kind: SegmentKind::Heading,
                        text: text.to_string(),
                    },
                ));
                continue;
            }
        }

        placed += 1;
        let mut path = current.clone();
        path.push(placed.to_string());
        out.push((
            line_number,
            RawSegment {
                path,
                kind: SegmentKind::Text,
                text: text.to_string(),
            },
        ));
    }
    out
}

/// How many times a shape has to occur before it is read as this sefer's own.
///
/// Three. Two is a coincidence a teshuvos sefer produces by citing the same
/// work twice; three in the same shape, at the head of a line, with increasing
/// numbers, is the sefer's own spine.
const MIN_HEADERS_FOR_A_RUN: usize = 3;

/// The part of an address that does not change from one section to the next.
///
/// `שו"ת הרשב"א חלק א סימן א` and `… סימן ב` have the same shape; the depth and
/// the named levels are identical and only the numbers move. That is what makes
/// a shape worth counting.
fn shape(levels: &[String]) -> String {
    let named: Vec<&str> = levels
        .iter()
        .map(String::as_str)
        .map(|l| if as_number(l).is_some() { "#" } else { l })
        .collect();
    named.join("\u{1}")
}

/// Every line that is part of this sefer's structure.
///
/// # A sefer may state its address at more than one depth
///
/// `אחיעזר` numbers its first two chalakim `חלק א - אבן העזר סימן א` and its
/// third `חלק ג סימן א` — three levels in one half of the file and two in the
/// other. Insisting on a single shape per file meant the shallower half won on
/// count and **the first 52% of the sefer became front matter**: 1,432 segments
/// with no address, in a sefer whose addresses are all written down.
///
/// So each shape that occurs often enough gets its own run, and the runs are
/// taken longest-first. What stops that from letting a *citation* pattern in —
/// `שולחן ערוך אורח חיים סימן ב` said five times in a teshuvos sefer is a shape
/// occurring five times — is that a run is only taken when it does not lie
/// **inside** a run already taken. A sefer's parts follow one another; a work it
/// keeps quoting is scattered through them. That is the difference, and it is a
/// property of where the lines are rather than of what they say.
fn the_spine(candidates: &[Option<Vec<String>>]) -> std::collections::BTreeSet<usize> {
    let mut runs: Vec<Vec<usize>> = shapes_worth_weighing(candidates)
        .into_iter()
        .map(|shape| the_longest_run(candidates, Some(shape.as_str())))
        .filter(|run| run.len() >= MIN_HEADERS_FOR_A_RUN)
        .map(|run| run.into_iter().collect())
        .collect();
    // Longest first, and on a tie the earlier one, so the choice does not
    // depend on how a hash map happened to order its keys.
    runs.sort_by_key(|run| (std::cmp::Reverse(run.len()), run.first().copied()));

    let mut taken = std::collections::BTreeSet::new();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for run in runs {
        let (Some(&first), Some(&last)) = (run.first(), run.last()) else {
            continue;
        };
        if spans.iter().any(|&(a, b)| first <= b && a <= last) {
            continue;
        }
        spans.push((first, last));
        taken.extend(run);
    }
    taken
}

/// The shapes that occur often enough to be worth calling a spine.
fn shapes_worth_weighing(candidates: &[Option<Vec<String>>]) -> Vec<String> {
    let mut tally: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for levels in candidates.iter().flatten() {
        *tally.entry(shape(levels)).or_default() += 1;
    }
    let mut worth: Vec<(String, usize)> = tally
        .into_iter()
        .filter(|&(_, n)| n >= MIN_HEADERS_FOR_A_RUN)
        .collect();
    worth.sort_by_key(|(shape, n)| (std::cmp::Reverse(*n), shape.clone()));
    worth.into_iter().map(|(shape, _)| shape).collect()
}

/// The lines whose addresses are this sefer's spine: **the longest run there
/// is**, not the first run that fits.
///
/// # Why this is not a greedy walk, which is what it was
///
/// `מאמרי המשגיח` has 441 maamarim numbered א through the end, and at line 136
/// — between maamar א and maamar ב — it has a stray `טז.` that belongs to
/// nothing. A walk that takes the first candidate that fits took it: 16 follows
/// 1 by less than [`A_PLAUSIBLE_GAP`], so the run jumped to 16, and then every
/// real maamar from ב to טו went *backwards* and was read as ordinary text.
/// Fourteen maamarim were filed under the wrong address — silently, and with a
/// heading count high enough to look healthy.
///
/// The bug was in the shape of the question. *Does this line continue the run
/// so far* has no good answer at line 136, because the run so far is one line
/// long and both readings are consistent with it. *Which set of lines is the
/// spine* has a good answer, and it is the one that explains the most of the
/// file: keeping `טז.` costs fourteen maamarim, so it is not kept.
///
/// So this is a longest-increasing-run over the candidates of the shape the
/// file keeps, where an edge is legal when [`follows`] allows it. Quadratic,
/// which is why it is bounded: past [`TOO_MANY_TO_WEIGH`] candidates it falls
/// back to the greedy walk, and the greedy walk is right whenever the file is
/// clean — which, at that size, every file measured is.
fn the_longest_run(
    candidates: &[Option<Vec<String>>],
    kept: Option<&str>,
) -> std::collections::BTreeSet<usize> {
    let Some(kept) = kept else {
        return std::collections::BTreeSet::new();
    };
    let of_the_shape: Vec<(usize, &Vec<String>)> = candidates
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|l| (i, l)))
        .filter(|(_, l)| shape(l) == kept)
        .collect();
    if of_the_shape.len() > TOO_MANY_TO_WEIGH {
        let mut taken = std::collections::BTreeSet::new();
        let mut run: Option<&Vec<String>> = None;
        for (i, levels) in of_the_shape {
            if follows(run.map(Vec::as_slice), levels) {
                taken.insert(i);
                run = Some(levels);
            }
        }
        return taken;
    }

    // `best[k]` is the length of the longest run ending at k, and `from[k]` the
    // candidate before it.
    let n = of_the_shape.len();
    let mut best = vec![1usize; n];
    let mut from = vec![usize::MAX; n];
    for k in 0..n {
        if !follows(None, of_the_shape[k].1) {
            // Cannot start a run — the numbering does not begin where numbering
            // begins — so it may only continue one.
            best[k] = 0;
        }
        for j in 0..k {
            if best[j] > 0
                && best[j] + 1 > best[k]
                && follows(Some(of_the_shape[j].1), of_the_shape[k].1)
            {
                best[k] = best[j] + 1;
                from[k] = j;
            }
        }
    }
    let Some(mut at) = (0..n).max_by_key(|&k| (best[k], std::cmp::Reverse(k))) else {
        return std::collections::BTreeSet::new();
    };
    if best[at] < MIN_HEADERS_FOR_A_RUN {
        return std::collections::BTreeSet::new();
    }
    let mut taken = std::collections::BTreeSet::new();
    loop {
        taken.insert(of_the_shape[at].0);
        if from[at] == usize::MAX {
            break;
        }
        at = from[at];
    }
    taken
}

/// Past this many candidates the run is walked rather than weighed.
///
/// The weighing is quadratic and the walking is linear; at ten thousand
/// candidates that is a hundred million comparisons against ten thousand, for a
/// difference that only shows up in a file with a stray line in it. Every file
/// measured above this size is an exporter's clean output with no strays.
const TOO_MANY_TO_WEIGH: usize = 10_000;

/// Is the next line with anything on it a rule?
///
/// Looked forward rather than remembered backward, because the decision is
/// *this* line's: a header is a header when something under it says so, and a
/// parser that finds out one line too late has already emitted it as text.
fn rule_follows(lines: &[&str], from: usize) -> bool {
    lines
        .iter()
        .skip(from + 1)
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .is_some_and(is_rule)
}

/// The words of a string, normalized, with the ones that are only punctuation
/// dropped.
///
/// `ראבי"ה חלק ב - מסכת סוכה` has a bare `-` in it, and a hyphen is not part of
/// anybody's address — it is also, per [`crate::work::section_label_of`], how
/// `girsa-ref` writes a span, so leaving one inside a level would make an
/// address that reads as two.
fn words(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(match_key)
        .filter(|w| !w.is_empty())
        .collect()
}

/// Read the address out of a header line, or decide there is not one.
///
/// Walks the words after the sefer's own name. A level word with a numeral
/// after it is a numbered level; anything else piles up and is flushed as a
/// **named** level, which is how `הלכות יסודי התורה` survives as the thing it
/// is rather than being thrown away for the `פרק ה` that follows it.
///
/// A named level goes through [`crate::work::section_label_of`], which is the
/// same sanitizer [`super::otzaria`] puts its headings through and not a second
/// one. It is not cosmetic: a level may not contain `/`, `:`, `#` or `-`, and
/// `ראבי"ה חלק ב - מסכת סוכה סימן תרמד` has a hyphen in the middle of it. The
/// first run of this reader over the real library minted **9,914 ids that were
/// not well formed** — a hyphen inside a level is how `girsa-ref` writes a
/// span, so `א-ב` reads back as *from א to ב* and the id names a range instead
/// of a place.
fn address_of(line: &str, title: &[String]) -> Option<Vec<String>> {
    let raw: Vec<&str> = line.split_whitespace().collect();
    let normalized: Vec<String> = raw.iter().map(|w| normalize(w)).collect();
    let keyed: Vec<String> = raw.iter().map(|w| match_key(w)).collect();
    let start = after_the_title(&keyed, title);

    let mut levels: Vec<String> = Vec::new();
    let mut named: Vec<&str> = Vec::new();
    let mut i = start;
    while i < raw.len() {
        let numbered = is_section_word(&normalized[i])
            .then(|| normalized.get(i + 1).and_then(|w| parse_hebrew(w)))
            .flatten();
        if let Some(n) = numbered {
            if !named.is_empty() {
                levels.push(section_label_of(&named.join(" ")));
                named.clear();
            }
            levels.push(n.to_string());
            i += 2;
            continue;
        }
        if !normalized[i].is_empty() {
            named.push(raw[i]);
        }
        i += 1;
    }
    if !named.is_empty() {
        levels.push(section_label_of(&named.join(" ")));
    }
    // A bare numeral on its own line — `א` opening a maamar — reaches here as
    // one named level that happens to be a number, which is not what it is.
    if levels.len() == 1 {
        if let Some(n) = parse_hebrew(&normalize(&levels[0])) {
            levels[0] = n.to_string();
        }
    }
    // A level nobody could read out is not an address, whatever marked it —
    // see [`A_LEVEL_A_READER_COULD_TYPE`]. Checked here rather than at the
    // marker, so it holds for all three signals at once.
    if levels
        .iter()
        .any(|l| l.chars().count() > A_LEVEL_A_READER_COULD_TYPE)
    {
        return None;
    }
    (!levels.is_empty()).then_some(levels)
}

/// Where the address starts: after the sefer's own name, if the line opens with
/// it.
///
/// Allows words in front of it — a header written `שו"ת <name> …` is the norm,
/// and the genre word is not part of the title the file is called by. Bounded,
/// so a sentence that happens to mention the sefer halfway through does not
/// have its second half read as an address.
fn after_the_title(line: &[String], title: &[String]) -> usize {
    if title.is_empty() {
        return 0;
    }
    let reach = line.len().min(title.len() + WORDS_BEFORE_A_TITLE);
    (0..reach)
        .find(|&at| line[at..].starts_with(title))
        .map_or(0, |at| at + title.len())
}

/// How many words may stand between the start of a header and the sefer's name.
///
/// `שו"ת` is one. Two allows `ספר שו"ת`, and stops well short of a sentence.
const WORDS_BEFORE_A_TITLE: usize = 2;

/// The number a level is, if it is one.
///
/// Numbered levels are written decimal by [`address_of`] — `1`, not `א` — so
/// this is a decimal parse and nothing else. That it fails on a named level is
/// the point: it is what tells `הלכות יסודי התורה` from `5`.
fn as_number(level: &str) -> Option<u32> {
    level.parse().ok()
}

/// Does this address continue the run, or is it a number in a sentence?
///
/// Compared level by level. The first level where they differ decides, and it
/// decides only if **both sides are numbers** — a named level against a number
/// is two different kinds of thing and the answer is no, which is what keeps
/// another sefer's citation out of the sequence.
fn follows(previous: Option<&[String]>, candidate: &[String]) -> bool {
    let Some(previous) = previous else {
        // Nothing to continue yet, so this would be starting one. A run starts
        // at its beginning — `א`, `ב` — and a sefer whose first header is
        // siman ק is a sefer where something has already gone wrong. The shape
        // has already been agreed by the time this is asked, so the named
        // levels need no checking here; only that the numbering starts where
        // numbering starts.
        return candidate
            .last()
            .and_then(|l| as_number(l))
            .is_some_and(|n| n <= A_PLAUSIBLE_GAP);
    };
    if previous.len() != candidate.len() {
        // A different depth is a different shape of address, and an inferred
        // signal is not enough to believe one. An explicit marker still can.
        return false;
    }
    for (was, now) in previous.iter().zip(candidate) {
        if was == now {
            continue;
        }
        let (Some(was), Some(now)) = (as_number(was), as_number(now)) else {
            return false;
        };
        return now > was && now - was <= A_PLAUSIBLE_GAP;
    }
    // Identical to the one before it. A sefer does not open the same siman
    // twice, so this is the address being quoted inside its own section.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every address the parser produced, in order, with its text.
    fn addressed(body: &str, title: &str) -> Vec<(String, String)> {
        parse(body, title)
            .into_iter()
            .map(|s| (s.path.join("/"), s.text))
            .collect()
    }

    fn just_addresses(body: &str, title: &str) -> Vec<String> {
        addressed(body, title).into_iter().map(|(a, _)| a).collect()
    }

    #[test]
    fn a_header_is_an_address_written_out_in_words() {
        // מהר"י בן לב, its first simanim, in the shape the file has them:
        // a header, then its lines, with no blank-line ceremony to lean on.
        let got = just_addresses(
            "שו\"ת מהר\"י בן לב חלק א סימן א\nכלל ראשון מדיני עיגונא\nוהשיבו\n\
             שו\"ת מהר\"י בן לב חלק א סימן ב\nשאלה אחרת\n\
             שו\"ת מהר\"י בן לב חלק א סימן ג\nושאלה שלישית\n",
            "מהרי בן לב",
        );
        assert_eq!(
            got,
            ["1/1", "1/1/1", "1/1/2", "1/2", "1/2/1", "1/3", "1/3/1"],
            "chelek א siman א, its two lines, then simanim ב and ג"
        );
    }

    #[test]
    fn a_rule_under_a_line_makes_it_a_header_whatever_else_it_says() {
        // ר' חיים הלוי: the header names the Rambam's halachos, which are not
        // this sefer's own levels at all, and the `====` is what says it is a
        // header. The name is kept as a level rather than dropped for the
        // number after it.
        let got = addressed(
            "ר' חיים הלוי הלכות יסודי התורה פרק ה\n\
             ====================================\n\
             כשיעמוד נכרי ויאנס\n",
            "רבינו חיים הלוי",
        );
        assert_eq!(got.len(), 2, "the rule is consumed, not kept as a line");
        assert!(
            got[0].0.contains("יסודי") && got[0].0.ends_with('5'),
            "the halachos named and the perek numbered: {:?}",
            got[0].0
        );
    }

    #[test]
    fn an_at_sign_marks_a_header_the_way_the_exporter_meant_it_to() {
        let got = addressed("@שו\"ת מהרש\"ם חלק א סימן א\nלהרב הגאון המפורסם\n", "מהרשם");
        assert_eq!(got[0].0, "1/1");
        assert!(
            !got[0].1.starts_with('@'),
            "the marker is not part of what anybody reads: {:?}",
            got[0].1
        );
    }

    #[test]
    fn a_numeral_alone_on_a_line_opens_a_section_when_it_continues_the_run() {
        // מאמרי המשגיח: 441 of these, running א ב ג in order, each opening a
        // maamar whose title is the line under it.
        assert_eq!(
            just_addresses(
                "משגיח סט\nא\nמאמר ראשון\nב\nמאמר שני\nג\nמאמר שלישי\n",
                "מאמרי המשגיח"
            ),
            ["0/1", "1", "1/1", "2", "2/1", "3", "3/1"],
            "the title line is front matter; each numeral opens its maamar"
        );
    }

    #[test]
    fn a_numeral_that_does_not_continue_the_run_is_ordinary_text() {
        // Rule 6. The second `ב` goes backwards, so it is a numeral in a
        // sentence and not a section — and reading it as one would put
        // everything after it under an address that is simply wrong.
        assert_eq!(
            just_addresses("א\nראשון\nב\nשני\nג\nשלישי\nב\nלא כותרת\nד\nרביעי\n", "ספר"),
            ["1", "1/1", "2", "2/1", "3", "3/1", "3/2", "3/3", "4", "4/1"],
            "א ב ג ד is the spine; the second ב is text inside siman ג"
        );
    }

    #[test]
    fn a_stray_number_ahead_of_the_run_does_not_capture_it() {
        // The one that was actually wrong, and it was wrong silently.
        // `מאמרי המשגיח` has a loose `טז.` sitting between maamar א and maamar
        // ב. Taking the first candidate that fits took it — 16 is inside the
        // gap from 1 — and then maamarim ב through טו all went backwards and
        // were read as prose. Fourteen maamarim under the wrong address, with a
        // heading count high enough to look healthy.
        //
        // Weighing the whole file instead of walking it forward answers this
        // without needing to know anything about the stray: keeping `טז` costs
        // three maamarim and buys one.
        assert_eq!(
            just_addresses("א\nראשון\nטז.\nמשוטט\nב\nשני\nג\nשלישי\nד\nרביעי\n", "ספר"),
            ["1", "1/1", "1/2", "1/3", "2", "2/1", "3", "3/1", "4", "4/1"],
            "the spine is א ב ג ד; טז explains less and is left as text"
        );
    }

    #[test]
    fn a_gap_in_the_numbering_does_not_end_the_run() {
        // Seforim skip simanim. An exact +1 would read everything after the
        // first gap as one enormous section.
        assert_eq!(
            just_addresses("א\nראשון\nה\nחמישי\nו\nששי\n", "ספר"),
            ["1", "1/1", "5", "5/1", "6", "6/1"]
        );
        // And a jump too far to be a gap is not believed.
        assert_eq!(
            just_addresses("א\nראשון\nב\nשני\nק\nמאה\nג\nשלישי\n", "ספר"),
            ["1", "1/1", "2", "2/1", "2/2", "2/3", "3", "3/1"],
            "ק is 100, which is past the gap, so it is a numeral in the text"
        );
    }

    #[test]
    fn a_sefer_may_state_its_address_at_more_than_one_depth() {
        // `אחיעזר` numbers its first chalakim `חלק א - אבן העזר סימן א` and its
        // third `חלק ג סימן א`. Taking only the commonest shape gave the
        // shallower one, and 52% of the sefer — 1,432 segments whose addresses
        // are written on the page — came out as front matter.
        let got = just_addresses(
            "שו\"ת אחיעזר חלק א - אבן העזר סימן א\nראשונה\n\
             שו\"ת אחיעזר חלק א - אבן העזר סימן ב\nשניה\n\
             שו\"ת אחיעזר חלק א - אבן העזר סימן ג\nשלישית\n\
             שו\"ת אחיעזר חלק ג סימן א\nרביעית\n\
             שו\"ת אחיעזר חלק ג סימן ב\nחמישית\n\
             שו\"ת אחיעזר חלק ג סימן ג\nששית\n",
            "אחיעזר",
        );
        assert_eq!(
            got,
            [
                "1/אבן_העזר/1",
                "1/אבן_העזר/1/1",
                "1/אבן_העזר/2",
                "1/אבן_העזר/2/1",
                "1/אבן_העזר/3",
                "1/אבן_העזר/3/1",
                "3/1",
                "3/1/1",
                "3/2",
                "3/2/1",
                "3/3",
                "3/3/1",
            ],
            "both halves keep their own addresses; neither is front matter"
        );
    }

    #[test]
    fn a_work_quoted_over_and_over_does_not_become_a_section_of_this_one() {
        // The cost of allowing a second shape. `שולחן ערוך` is cited three
        // times here — enough occurrences to be a shape — but scattered
        // *inside* the sefer's own run rather than following it, and a sefer's
        // parts follow one another.
        let got = just_addresses(
            "שו\"ת פלוני סימן א\nשאלה\nשולחן ערוך אורח חיים סימן א\nועיין\n\
             שו\"ת פלוני סימן ב\nשאלה\nשולחן ערוך אורח חיים סימן ב\nועיין\n\
             שו\"ת פלוני סימן ג\nשאלה\nשולחן ערוך אורח חיים סימן ג\nועיין\n\
             שו\"ת פלוני סימן ד\nשאלה\n",
            "פלוני",
        );
        assert_eq!(
            got,
            [
                "1", "1/1", "1/2", "1/3", "2", "2/1", "2/2", "2/3", "3", "3/1", "3/2", "3/3", "4",
                "4/1",
            ],
            "the שו\"ע lines are lines inside each siman, not simanim"
        );
    }

    #[test]
    fn a_line_naming_a_different_sefer_is_never_a_header() {
        // The one that matters in a teshuvos sefer, where nearly every line
        // cites somebody. It parses into an address perfectly well; what it
        // cannot do is continue a run whose first level is a number.
        assert_eq!(
            just_addresses(
                "שו\"ת הרשב\"א חלק א סימן א\nשאלת\nשולחן ערוך אורח חיים סימן ב\nועיין שם\n\
                 שו\"ת הרשב\"א חלק א סימן ב\nשאלה אחרת\n\
                 שו\"ת הרשב\"א חלק א סימן ג\nועוד אחת\n",
                "הרשבא",
            ),
            ["1/1", "1/1/1", "1/1/2", "1/1/3", "1/2", "1/2/1", "1/3", "1/3/1"],
            "the שו\"ע line is a line of siman א, not a section of the Rashba"
        );
    }

    #[test]
    fn a_file_with_nothing_to_recognise_still_imports() {
        // No refusal: front matter, the same as an Otzaria file with no
        // headings in it.
        assert_eq!(
            just_addresses("סתם שורה\nעוד שורה\n", "ספר"),
            ["0/1", "0/2"]
        );
    }

    #[test]
    fn the_line_numbers_are_the_files_own() {
        // Blank lines are skipped, so the nth segment is not line n — which is
        // exactly why the mapping is returned rather than assumed.
        let got = parse_with_lines("א\n\nראשון\n", "ספר");
        assert_eq!(
            got.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            [1, 3],
            "the blank line is skipped and does not shift the count"
        );
    }

    #[test]
    fn the_sefers_own_name_is_not_repeated_in_every_level() {
        // With the title recognised the address is `1/1`; without it the name
        // would be the first level of all 371 simanim.
        let body = "שו\"ת פלוני חלק א סימן א\nאחת\nשו\"ת פלוני חלק א סימן ב\nשתים\n\
                    שו\"ת פלוני חלק א סימן ג\nשלש\n";
        assert_eq!(
            just_addresses(body, "פלוני"),
            ["1/1", "1/1/1", "1/2", "1/2/1", "1/3", "1/3/1"]
        );
        // A title that does not match costs a longer first level, not a failure
        // to import: the shape is *counted*, so the sefer's own name simply
        // becomes the outermost level of every address in it — through
        // `section_label_of`, because a level of a permanent id may not carry a
        // space or a gershayim any more than it may carry a hyphen.
        let missed = just_addresses(body, "אלמוני");
        assert_eq!(missed.len(), 6);
        assert_eq!(missed[0], "שות_פלוני/1/1");
    }

    #[test]
    fn a_named_level_can_be_written_into_a_permanent_id() {
        // The first run of this reader over the real library minted **9,914
        // ids that were not well formed**, and the importer's own check caught
        // it. A level may not contain `/`, `:`, `#` or `-`, and a hyphen is the
        // one that bites: it is how `girsa-ref` writes a span, so a level
        // `חלק-ב` reads back as *from חלק to ב* and the id names a range
        // instead of a place.
        let got = just_addresses(
            "ראבי\"ה חלק ב - מסכת סוכה סימן א\nאחת\n\
             ראבי\"ה חלק ב - מסכת סוכה סימן ב\nשתים\n\
             ראבי\"ה חלק ב - מסכת סוכה סימן ג\nשלש\n",
            "ראביה",
        );
        for address in &got {
            for level in address.split('/') {
                assert!(
                    !level.contains(['-', ':', '#']) && !level.is_empty(),
                    "a level that cannot be written into an id: {level:?} in {address:?}"
                );
            }
        }
        assert_eq!(
            got[0], "2/מסכת_סוכה/1",
            "the hyphen is gone, the address is not"
        );
    }

    #[test]
    fn a_paragraph_with_a_rule_under_it_is_still_a_paragraph() {
        // `מוסר ודעת` has `====` rules sitting under ordinary prose, and
        // believing the marker on sight put **14,019 of its 34,361 segments**
        // under three section labels that were whole paragraphs — one of them
        // 9,144 characters long. A permanent id has to be something a person
        // can read out.
        let prose = "וזהו המובן במה שאמר הכתוב דברים ד ד ואתם הדבקים בד אלוקיכם חיים \
                     כולכם היום וכל הענין הזה טעון ביאור ארוך ורחב שאין כאן מקומו";
        let body = format!("{prose}\n=========\nשורה ראשונה\nשורה שניה\n");
        let got = just_addresses(&body, "מוסר ודעת");
        assert!(
            got.iter().all(|a| a.starts_with('0')),
            "the paragraph opens nothing; everything is front matter: {got:?}"
        );
        // And a real header with a rule under it still opens a section.
        let short = just_addresses("סימן א\n=========\nשורה\n", "ספר");
        assert_eq!(short, ["1", "1/1"]);
    }

    #[test]
    fn a_rule_is_a_rule_and_a_line_of_dashes_is_not() {
        assert!(is_rule("===="));
        assert!(is_rule("  ==========  "));
        assert!(!is_rule("==="), "three turns up inside prose");
        assert!(!is_rule("----"));
        assert!(!is_rule("=a=="));
    }

    #[test]
    fn the_level_words_come_from_the_resolver() {
        // Not a second list. `is_section_word` is `girsa-ref`'s own, made
        // public for exactly this caller.
        assert!(is_section_word("סימן"));
        assert!(!is_section_word("הרשבא"));
    }
}
