//! Where a sefer sits on the shelf, and what the shelf is called.
//!
//! spec.md §5: *browsable the way seforim are actually organized — Tanach /
//! Shas / Halacha / Machshava / Chassidus / Responsa / yours — **with the
//! arrangement editable.** The shipped taxonomy is a default, not a fact.*
//!
//! # The corpus does not have one taxonomy. It has two.
//!
//! Sefaria files an acharon on the Gemara under `Talmud/Bavli/Acharonim on
//! Talmud`, in English. Otzaria files one under `תלמוד בבלי/אחרונים`, in
//! Hebrew. Both are right about their own download and neither is a shelf: put
//! them side by side and a reader looking for an acharon on Shas has to know
//! which of the two corpora his sefer came from, which is the one thing the
//! union was built to stop mattering.
//!
//! So there is one shipped taxonomy, in Hebrew, and both vocabularies are
//! mapped onto it. Three rules, in order:
//!
//! 1. **A prefix table** takes the first category — sometimes the first two —
//!    onto a canonical top shelf. `["Talmud","Bavli"]` and `["תלמוד בבלי"]`
//!    both become `תלמוד/בבלי`, which is where the two corpora meet.
//! 2. **`X on Y` loses its `on Y`** when `Y` names the shelf it is already
//!    under. `Talmud/Bavli/Acharonim on Talmud` is *the acharonim*, said twice;
//!    the second saying is what keeps it off Otzaria's `אחרונים` shelf.
//! 3. **A term table** translates what is left, and **anything not in it is
//!    carried through exactly as the corpus wrote it.** A category we have no
//!    Hebrew name for is shown in the corpus's words rather than in a guess at
//!    them — and since a shelf can be renamed (`girsa_app::arrangement`), a bad
//!    default costs one drag, not a wrong label forever.
//!
//! Nothing here is allowed to lose a sefer. A category the tables have never
//! seen lands under `אחר` **carrying its original name**, so it is browsable,
//! countable and obviously unfinished, rather than quietly absent.
//!
//! # Why the shipped shelf is down here and the reader's edits are not
//!
//! This half is a function of a [`Work`] and nothing else: give it a work and
//! it says where the corpus would file it. The other half — the reader having
//! moved it, renamed the shelf, made a new one — is `girsa_app::taxonomy`,
//! because it needs the personal layer.
//!
//! Splitting them that way is what lets the **search facets** (spec.md §9.8)
//! group results by the same shelf the window browses by. A second mapping,
//! written where the facets could reach it, would put a sefer on one shelf in
//! the bookcase and on another in a result list — and a reader would have no
//! way to tell which of the two was lying.

use crate::work::Work;

/// The shipped top-level shelves, in the order a bookcase has them.
///
/// `שלי` is yours — spec.md §5's *your own material, whenever* — and it is in
/// the same list as the rest because that is what first-class means. `אחר` is
/// the catch-all, and it is deliberately visible: a shelf nobody can see is a
/// place seforim go missing.
pub const TOP: [&str; 16] = [
    "תנ״ך",
    "משנה",
    "תלמוד",
    "תוספתא",
    "מדרש",
    "הלכה",
    "שו״ת",
    "מחשבה",
    "מוסר",
    "קבלה",
    "חסידות",
    "תפילה",
    "בית שני",
    "עזר",
    "שלי",
    "אחר",
];

/// The top shelf a work's own first categories name.
///
/// Longest match first: `Talmud/Bavli` before `Talmud`, so the Bavli and the
/// Yerushalmi are two shelves under one and Otzaria's two top-level folders
/// land on the same two.
const PREFIX: [(&[&str], &[&str]); 34] = [
    (&["Talmud", "Bavli"], &["תלמוד", "בבלי"]),
    (&["Talmud", "Yerushalmi"], &["תלמוד", "ירושלמי"]),
    (&["תלמוד בבלי"], &["תלמוד", "בבלי"]),
    (&["תלמוד ירושלמי"], &["תלמוד", "ירושלמי"]),
    (&["Talmud"], &["תלמוד"]),
    (&["Tanakh"], &["תנ״ך"]),
    (&["תנך"], &["תנ״ך"]),
    (&["Mishnah"], &["משנה"]),
    (&["משנה"], &["משנה"]),
    (&["Tosefta"], &["תוספתא"]),
    (&["תוספתא"], &["תוספתא"]),
    (&["Midrash"], &["מדרש"]),
    (&["מדרש"], &["מדרש"]),
    (&["Halakhah"], &["הלכה"]),
    (&["הלכה"], &["הלכה"]),
    (&["Responsa"], &["שו״ת"]),
    (&["שות"], &["שו״ת"]),
    (&["Jewish Thought"], &["מחשבה"]),
    (&["מחשבת ישראל"], &["מחשבה"]),
    (&["Musar"], &["מוסר"]),
    (&["ספרי מוסר"], &["מוסר"]),
    (&["Kabbalah"], &["קבלה"]),
    (&["קבלה"], &["קבלה"]),
    (&["Chasidut"], &["חסידות"]),
    (&["חסידות"], &["חסידות"]),
    (&["Liturgy"], &["תפילה"]),
    (&["סדר התפילה"], &["תפילה"]),
    // חק לישראל is a daily arrangement of learning, printed and used beside
    // the siddur. A judgment call, and the sort the arrangement exists to let
    // a reader overrule.
    (&["לימוד יומי"], &["תפילה", "לימוד יומי"]),
    (&["Second Temple"], &["בית שני"]),
    (&["Reference"], &["עזר"]),
    (&["ספרות עזר"], &["עזר"]),
    // Otzaria ships its own documentation as two works. They are not seforim,
    // and they are not thrown away either — see the module note on `אחר`.
    (&["הודעה חשובה"], &["אחר"]),
    (&["אודות התוכנה"], &["אחר"]),
    (&["שלי"], &["שלי"]),
];

/// What a category below the top is called on the shelf.
///
/// Only exact translations are in here. `Chasidut/Early Works` is *not* —
/// "early" there means the first generations of chasidus, and `ראשונים` would
/// file the Maggid of Mezritch with the Rishonim. It is carried through in
/// English instead, which is honest and which a reader can rename.
const TERM: [(&str, &str); 30] = [
    ("Rishonim", "ראשונים"),
    ("Acharonim", "אחרונים"),
    ("Commentary", "מפרשים"),
    ("Modern", "מחברי זמננו"),
    ("Modern Commentary", "מחברי זמננו"),
    ("Targum", "תרגום"),
    ("Torah", "תורה"),
    ("Prophets", "נביאים"),
    ("Writings", "כתובים"),
    ("Seder Zeraim", "סדר זרעים"),
    ("Seder Moed", "סדר מועד"),
    ("Seder Nashim", "סדר נשים"),
    ("Seder Nezikin", "סדר נזיקין"),
    ("Seder Kodashim", "סדר קדשים"),
    ("Seder Tahorot", "סדר טהרות"),
    ("Minor Tractates", "מסכתות קטנות"),
    ("Mishneh Torah", "משנה תורה"),
    ("Shulchan Arukh", "שולחן ערוך"),
    ("Tur", "טור"),
    ("Sifrei Mitzvot", "ספרי מצוות"),
    ("Aggadah", "אגדה"),
    ("Halakhah", "הלכה"),
    ("Zohar", "זהר"),
    ("Haggadah", "הגדה"),
    ("Siddur", "סידור"),
    ("Piyutim", "פיוטים"),
    ("Apocrypha", "ספרים חיצוניים"),
    ("Encyclopedic Works", "אנציקלופדיות"),
    ("Dictionary", "מילונים"),
    ("Grammar", "דקדוק"),
];

/// Shelves that have an order older than the alphabet: rishonim before
/// acharonim before our own contemporaries. Everything else sorts after these,
/// by how much is on it.
const RANK: [&str; 8] = [
    "תורה",
    "נביאים",
    "כתובים",
    "תרגום",
    "ראשונים",
    "אחרונים",
    "מחברי זמננו",
    "מפרשים",
];

/// The shelf a work sits on, as a path from the top.
///
/// Always at least one element, and its first element is always one of [`TOP`]
/// — a sefer with no shelf is a sefer a reader cannot browse to.
#[must_use]
pub fn shelf_of(work: &Work) -> Vec<String> {
    let categories: Vec<&str> = work
        .categories
        .iter()
        .map(String::as_str)
        .filter(|c| !c.trim().is_empty())
        .collect();

    let (mut shelf, consumed) = top_of(&categories);
    // What the corpus called the shelf before it was mapped. `Acharonim on
    // Talmud` sheds its `on Talmud` because `Talmud` is one of these.
    let said: Vec<&str> = categories.iter().take(consumed.max(1)).copied().collect();

    for part in categories.iter().skip(consumed) {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        shelf.push(term_of(trimmed, &said));
    }
    shelf
}

/// The top shelf, and how many of the work's own categories it used up.
fn top_of(categories: &[&str]) -> (Vec<String>, usize) {
    for (from, to) in PREFIX {
        if from.len() <= categories.len() && categories.iter().zip(from).all(|(c, f)| c == f) {
            return (to.iter().map(|s| (*s).to_string()).collect(), from.len());
        }
    }
    // Unmapped. Under `אחר`, keeping the name the corpus gave it: browsable,
    // countable, and obviously something nobody has filed yet.
    match categories.first() {
        Some(first) => (vec!["אחר".to_string(), (*first).to_string()], 1),
        None => (vec!["אחר".to_string()], 0),
    }
}

/// One category below the top, as the shelf says it.
fn term_of(part: &str, said: &[&str]) -> String {
    // `Rishonim on Tanakh` under the Tanakh shelf is *the rishonim*: the `on
    // Tanakh` is the corpus repeating where the reader already is, and it is
    // the whole reason Sefaria's rishonim and Otzaria's ראשונים would sit on
    // two shelves.
    let bare = match part.split_once(" on ") {
        Some((head, base)) if said.iter().any(|s| s.eq_ignore_ascii_case(base)) => head,
        _ => part,
    };
    TERM.iter()
        .find(|(en, _)| *en == bare)
        .map_or_else(|| bare.to_string(), |(_, he)| (*he).to_string())
}

/// Where a shelf sorts among its siblings: era first, then size.
#[must_use]
pub fn rank_of(title: &str) -> usize {
    RANK.iter()
        .position(|r| *r == title)
        .unwrap_or(RANK.len() + 1)
}

/// Where a top-level shelf sorts.
#[must_use]
pub fn top_rank_of(title: &str) -> usize {
    TOP.iter().position(|t| *t == title).unwrap_or(TOP.len())
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::work::Source;

    fn shelf(categories: &[&str]) -> String {
        let work = Work {
            slug: "x".into(),
            he_title: "x".into(),
            en_title: "x".into(),
            categories: categories.iter().map(|c| (*c).to_string()).collect(),
            source: Source::Sefaria,
            origin: std::path::PathBuf::new(),
            schema: None,
            author: None,
            era: None,
            comp_date: None,
            version: None,
            he_sections: Vec::new(),
            commentary_on: Vec::new(),
        };
        shelf_of(&work).join("/")
    }

    #[test]
    fn the_two_corpora_write_one_shelf_two_ways_and_it_is_one_shelf() {
        assert_eq!(
            shelf(&["Talmud", "Bavli", "Acharonim on Talmud"]),
            "תלמוד/בבלי/אחרונים"
        );
        assert_eq!(shelf(&["תלמוד בבלי", "אחרונים"]), "תלמוד/בבלי/אחרונים");

        assert_eq!(
            shelf(&["Talmud", "Yerushalmi", "Commentary"]),
            "תלמוד/ירושלמי/מפרשים"
        );
        assert_eq!(shelf(&["תלמוד ירושלמי", "מפרשים"]), "תלמוד/ירושלמי/מפרשים");

        assert_eq!(shelf(&["Responsa", "Acharonim"]), "שו״ת/אחרונים");
        assert_eq!(shelf(&["שות", "אחרונים"]), "שו״ת/אחרונים");
    }

    #[test]
    fn a_category_nobody_has_translated_is_carried_through_not_guessed_at() {
        // Real: Sefaria's chasidus shelf. `Early Works` is not `ראשונים` and
        // is not going to be renamed into one by this code.
        assert_eq!(shelf(&["Chasidut", "Early Works"]), "חסידות/Early Works");
        assert_eq!(shelf(&["Kabbalah", "Baal HaSulam"]), "קבלה/Baal HaSulam");
    }

    #[test]
    fn a_shelf_nobody_has_mapped_keeps_its_name_and_stays_countable() {
        assert_eq!(shelf(&["Something New"]), "אחר/Something New");
        assert_eq!(shelf(&[]), "אחר");
        // Otzaria ships its own about-box as a work. It is not a sefer; it is
        // also not silently dropped.
        assert_eq!(shelf(&["אודות התוכנה"]), "אחר");
    }

    #[test]
    fn on_y_is_only_shed_when_y_is_where_the_reader_already_is() {
        // The base named is the shelf: shed it.
        assert_eq!(
            shelf(&["Mishnah", "Modern Commentary on Mishnah"]),
            "משנה/מחברי זמננו"
        );
        // The base named is something else — a work, not this shelf. Kept,
        // because dropping it would file the commentary as though it were the
        // thing it comments on.
        assert_eq!(
            shelf(&["Talmud", "Bavli", "Commentary on Minor Tractates"]),
            "תלמוד/בבלי/Commentary on Minor Tractates"
        );
    }
}
