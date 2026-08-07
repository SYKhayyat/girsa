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

/// The category values that mark a shelf of **commentary** rather than a shelf
/// of seforim.
///
/// Prefixes, because Sefaria writes the base into the name: `Rishonim on
/// Talmud`, `Acharonim on Mishnah`, `Modern Commentary on Tanakh`, `Commentary on
/// Minor Tractates`, `Commentary of the Rosh`. Twenty-three distinct values in
/// the shipped corpus and all of them start with one of the five English ones.
///
/// The Hebrew five are Otzaria's, and leaving them out is what made a
/// Hebrew-categorized rishon invisible: `ר חננאל על בראשית` is filed
/// `["תנך","ראשונים","רבינו חננאל","תורה"]`, matched none of the English
/// prefixes, and so was not *filed as commentary at all* — which took it out of
/// the mefarshim on Bereshis without anything saying so. Both corpora write this
/// shelf and the module note says they are one shelf; this list was half of it.
const COMMENTARY: [&str; 10] = [
    "Commentary",
    "Rishonim",
    "Acharonim",
    "Modern Commentary",
    "Targum",
    "מפרשים",
    "ראשונים",
    "אחרונים",
    "מחברי זמננו",
    "תרגום",
];

/// The divisions a sefer belongs to *within* its shelf.
///
/// These are what settle the case the shelf cannot: a commentary filed
/// `["Tanakh","Rishonim on Tanakh"]` sits above the whole of Tanakh, so its
/// shelf permits Bereshis and Tehillim alike. When both it and the base name one
/// of these, they have to name the same one.
///
/// In the canonical Hebrew, because that is what both vocabularies map onto —
/// which is the whole reason [`stands`] compares canonical paths and not the
/// `categories` the corpus happened to write.
const SECTION: [&str; 9] = [
    "תורה",
    "נביאים",
    "כתובים",
    "סדר זרעים",
    "סדר מועד",
    "סדר נשים",
    "סדר נזיקין",
    "סדר קדשים",
    "סדר טהרות",
];

/// Whether a work stands on a shelf of commentary, and what is above it.
///
/// `Some(&["Halakhah", "Shulchan Arukh"])` for a work filed under `Halakhah /
/// Shulchan Arukh / Commentary / Kaf HaChayim` — the shelf its base text stands
/// on. `None` for a work that is not filed as commentary at all.
#[must_use]
pub fn commentary_shelf(work: &Work) -> Option<&[String]> {
    let at = work
        .categories
        .iter()
        .position(|c| COMMENTARY.iter().any(|m| c.starts_with(m)))?;
    Some(&work.categories[..at])
}

/// How one sefer stands to another.
///
/// A bool could say *mefaresh* or *not*, and that turned out to be the wrong
/// number of answers twice over. The Shulchan Arukh keeps the Tur's order and is
/// not a commentary on it; a Tanakh commentary with no section named sits over
/// Bereshis and Tehillim alike and the shelf cannot tell you which. Both came
/// back as a flat `false`, which read as *unrelated* and was not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stands {
    /// A mefaresh: written **about** this sefer, keyed to its words.
    On,
    /// Its own sefer, following this one's order. The Shulchan Arukh on the Tur,
    /// the Arukh HaShulchan on the Shulchan Arukh, the Mishneh Torah's hilchos
    /// beside the chelek of Yoreh De'ah that covers the same ground.
    Alongside,
    /// Nowhere near it. The Yerushalmi is not a mefaresh on the Tur, whatever
    /// the twenty-six `comments-on` edges between them say.
    Apart,
    /// The shelf permits it and cannot settle it — only where its edges land
    /// can. `תנ״ך/ראשונים` is above the whole of Tanakh, so Bartenura on Torah
    /// and Ibn Ezra on Tehillim have the same shelf-above and different answers.
    ///
    /// Returned rather than guessed, because guessing here is BUILDER.md rule 6.
    /// The caller holds the graph; this module holds the shelf.
    AskTheEdges,
}

/// How much a sefer has to say in another before saying it counts as commenting.
///
/// Only ever consulted for [`Stands::AskTheEdges`] — a commentary that declares
/// no base and whose shelf sits above a whole division, where the shelf permits
/// the answer and cannot give it.
///
/// Bartenura on Torah puts 330 comments into Bereshis, 289 into Shemos and none
/// at all anywhere in Kesuvim; the works that turn up with one or two are
/// quoting in passing. The gap between those is three orders of magnitude, so
/// the exact line matters much less than having one, and this is deliberately
/// near the bottom of it: a real mefaresh with only a dozen comments on a sefer
/// is still a mefaresh, and a stray reference does not reach a dozen.
///
/// It lived in `girsa_app::mefarshim`, private, while `Shelf::companions` and
/// `Beside::between` answered the same question without it — and without
/// [`stands`] either.
pub const SAYS_ENOUGH_TO_BE_A_MEFARESH: usize = 12;

/// [`stands`], with the one case it refuses to guess at settled by the count.
///
/// # Why the two are separate functions
///
/// [`stands`] holds the shelf and will not infer a relationship between two
/// seforim from the existence of an edge — BUILDER.md rule 6. The caller holds
/// the graph. This is the seam between them, and it is a function rather than a
/// convention because there were **three** callers asking *which seforim relate
/// to this one* and only one of them asked `stands` at all:
///
/// | | asked | answered from |
/// |---|---|---|
/// | `girsa_app::mefarshim::Marks::of` | `stands`, then its own private threshold | `inbound.jsonl`, `comments-on` only |
/// | `girsa_app::Shelf::companions` | nothing — `commentary_on` in either direction | `companions.jsonl`, every edge type |
/// | `girsa_app::beside::Joined::between` | nothing — `commentary_on` in either direction | both works' shards, every edge type |
///
/// So the Beit Yosef, which declares no base and is a mefaresh on the Tur by its
/// shelf, was a mefaresh in the tick-list and **not** a companion in the picker
/// — and the window's button counted the declared ones and said *5* over a list
/// of forty.
#[must_use]
pub fn settled(commentary: &Work, base: &Work, edges: usize) -> Stands {
    match stands(commentary, base) {
        Stands::AskTheEdges => {
            if edges >= SAYS_ENOUGH_TO_BE_A_MEFARESH {
                Stands::On
            } else {
                Stands::Apart
            }
        }
        settled => settled,
    }
}

/// How one work stands to another — mefaresh, alongside, or neither.
///
/// This is the question W43's tick-list, and anything else that says *these are
/// the mefarshim on this sefer*, has to ask. It is not the same question as
/// *does an edge join them*: `comments-on` is a type on one of 4.18M edges and
/// Sefaria's link data is not careful enough for that to be a claim about two
/// seforim. Tur has commentary edges landing in it from forty works and is
/// commented on by five.
///
/// Two ways to be a mefaresh, and both of them are the corpus's own statement:
///
/// - it **declares** the base — `commentary_on`, which is Sefaria's
///   `base_text_titles`;
/// - or it stands on a **commentary shelf** whose shelf-above is at or over the
///   shelf the sefer itself stands on, agreeing on the section where both name
///   one. The Kaf HaChayim declares nothing and is the largest commentary on
///   Orach Chayim; the Beit Yosef declares nothing and is *the* commentary on
///   the Tur. Keeping only the declared ones would drop both.
///
/// Compared over **canonical** paths, not over `categories`. The corpus has two
/// vocabularies (see this module's note) and a raw string compare quietly means
/// *English only*.
///
/// Never from a slug or a title — BUILDER.md rule 6, which is what the first
/// version of this got wrong by not asking the question at all.
#[must_use]
pub fn stands(commentary: &Work, base: &Work) -> Stands {
    if commentary.slug == base.slug {
        return Stands::Apart;
    }
    if commentary
        .commentary_on
        .iter()
        .any(|declared| declared.slug == base.slug)
    {
        return Stands::On;
    }
    // A declaration that names some *other* sefer is a statement about what this
    // is a commentary on, and it did not name this one. The shelf is only asked
    // of a work that has said nothing.
    if !commentary.commentary_on.is_empty() {
        return Stands::Apart;
    }

    // A sefer with no shelf at all is not alongside anything.
    //
    // `canonical_path` answers an unfiled work with a **default top** rather
    // than with nothing, so two seforim a reader dropped on the window — neither
    // filed, both with an empty `categories` — matched each other on that
    // default and came back `Alongside`. Which is a claim that they keep the
    // same order, and would line them up by address: two unrelated files that
    // both happen to be addressed `1:1` moving each other.
    if commentary.categories.is_empty() || base.categories.is_empty() {
        return Stands::Apart;
    }
    let theirs = canonical_path(&base.categories);
    let Some(above) = commentary_shelf(commentary) else {
        // Not filed as commentary at all. It is not a mefaresh — but a sefer on
        // the same top shelf, with commentary edges into this one, is running
        // *alongside* it: the Shulchan Arukh keeps the Tur's order, the Arukh
        // HaShulchan keeps the Shulchan Arukh's. Saying `Apart` there threw away
        // a relationship the reader wants; saying `On` would call a code a
        // commentary. So it is neither, and it says so.
        let mine = canonical_path(&commentary.categories);
        return match (mine.first(), theirs.first()) {
            (Some(mine), Some(theirs)) if mine == theirs => Stands::Alongside,
            _ => Stands::Apart,
        };
    };

    let above = canonical_path(above);
    // The shelf directly above the commentary *is* the base's shelf: the Kaf
    // HaChayim under `הלכה/שולחן ערוך/מפרשים`, Orach Chayim on `הלכה/שולחן ערוך`.
    if above == theirs {
        return Stands::On;
    }
    // Not above it at all — being *a* commentary is not being a commentary on
    // *this*.
    if !theirs.starts_with(&above) {
        return Stands::Apart;
    }
    // Above it, but higher up than its own shelf: `תנ״ך/ראשונים` sits over the
    // whole of Tanakh. Whether it reaches *this* sefer is what the section
    // settles, and where neither names one, only the graph can.
    match (
        section_of(&canonical_path(&commentary.categories)),
        section_of(&theirs),
    ) {
        (Some(mine), Some(theirs)) if mine == theirs => Stands::On,
        (Some(_), Some(_)) => Stands::Apart,
        _ => Stands::AskTheEdges,
    }
}

/// The section a canonical shelf path names, if it names one.
fn section_of(path: &[String]) -> Option<&str> {
    path.iter()
        .map(String::as_str)
        .find(|part| SECTION.contains(part))
}

/// One work's categories, in the canonical vocabulary — the shelf without the
/// W46 commentary step, which is about where a sefer is *drawn* and not about
/// what it is.
fn canonical_path(categories: &[String]) -> Vec<String> {
    let categories: Vec<&str> = categories
        .iter()
        .map(String::as_str)
        .filter(|c| !c.trim().is_empty())
        .collect();
    let (mut shelf, consumed) = top_of(&categories);
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

/// The shelf a work sits on, as a path from the top.
///
/// Always at least one element, and its first element is always one of [`TOP`]
/// — a sefer with no shelf is a sefer a reader cannot browse to.
#[must_use]
pub fn shelf_of(work: &Work) -> Vec<String> {
    // The same mapping [`stands`] compares over. One implementation, because two
    // would put a sefer on one shelf in the bookcase and judge it by another.
    let mut shelf = canonical_path(&work.categories);

    // W46. A sefer that **declares** itself a commentary and whose categories put
    // it on its own base's shelf is filed one level down, with the commentaries.
    //
    // Sefaria's own data disagrees with itself about this, in one line:
    //
    //     peri-megadim-on-orach-chayim  ['Halakhah','Shulchan Arukh']
    //     peri-megadim-on-yoreh-deah    ['Halakhah','Shulchan Arukh','Commentary','Pri Megadim']
    //
    // Same author, same sefer, two chalakim, two filings — so one of them stands
    // beside the four chalakim as though it were a fifth. The declaration is the
    // stronger statement and it wins, which is the precedence the rest of this
    // codebase already uses everywhere else.
    if !work.commentary_on.is_empty() && commentary_shelf(work).is_none() {
        // What the corpus called the shelf before it was mapped, which is what
        // `term_of` strips an `on Y` against.
        let categories: Vec<&str> = work
            .categories
            .iter()
            .map(String::as_str)
            .filter(|c| !c.trim().is_empty())
            .collect();
        let (_, consumed) = top_of(&categories);
        let said: Vec<&str> = categories.iter().take(consumed.max(1)).copied().collect();
        shelf.push(term_of("Commentary", &said));
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

    /// A work with categories, and the bases it declares.
    fn work_of(slug: &str, categories: &[&str], on: &[&str]) -> Work {
        Work {
            slug: slug.into(),
            he_title: slug.into(),
            en_title: slug.into(),
            categories: categories.iter().map(|c| (*c).to_string()).collect(),
            source: Source::Sefaria,
            origin: std::path::PathBuf::new(),
            schema: None,
            author: None,
            era: None,
            comp_date: None,
            version: None,
            he_sections: Vec::new(),
            commentary_on: on
                .iter()
                .map(|slug| crate::work::BaseText {
                    slug: (*slug).to_string(),
                    mapping: crate::work::Mapping::Unstated,
                })
                .collect(),
        }
    }

    // ── W45: is this a commentary on *that* sefer ────────────────────────────
    //
    // The corpus holds 4.18M edges and `comments-on` is a type on an edge, not a
    // claim about two seforim. Tur has commentary edges landing in it from forty
    // works and is commented on by four. Reading the edge as the claim is how
    // Rashi on Berakhot became a mefaresh on the Tur.

    // ── W46: a sefer that says it is a commentary is filed as one ────────────

    #[test]
    fn the_pri_megadim_is_filed_with_the_mefarshim_and_not_beside_the_shulchan_arukh() {
        // Upstream, in one line, and this is the reader's *"pri megadim is lumped
        // with it"*:
        //
        //   peri-megadim-on-orach-chayim  ['Halakhah','Shulchan Arukh']
        //   peri-megadim-on-yoreh-deah    ['Halakhah','Shulchan Arukh','Commentary','Pri Megadim']
        //
        // Same author, same sefer, two chalakim, two different filings. So one of
        // them stands on the Shulchan Arukh's own shelf as though it were a fifth
        // chelek. It is not a guess to move it: it **declares**
        // `commentary_on: shulchan-arukh/orach-chayim`.
        let pri_megadim = work_of(
            "peri-megadim-on-orach-chayim",
            &["Halakhah", "Shulchan Arukh"],
            &["shulchan-arukh/orach-chayim"],
        );
        assert_eq!(shelf_of(&pri_megadim).join("/"), "הלכה/שולחן ערוך/מפרשים");
    }

    #[test]
    fn the_shulchan_arukh_itself_does_not_move() {
        // The base text declares nothing, so nothing moves it. This is the test
        // that keeps the rule above from filing the whole shelf under its own
        // commentaries.
        let arukh = work_of(
            "shulchan-arukh/orach-chayim",
            &["Halakhah", "Shulchan Arukh"],
            &[],
        );
        assert_eq!(shelf_of(&arukh).join("/"), "הלכה/שולחן ערוך");

        // And an introduction to a sefer is part of the sefer.
        let intro = work_of(
            "shulchan-arukh/introduction",
            &["Halakhah", "Shulchan Arukh"],
            &[],
        );
        assert_eq!(shelf_of(&intro).join("/"), "הלכה/שולחן ערוך");
    }

    #[test]
    fn a_commentary_already_filed_as_one_is_left_where_it_is() {
        // Rashi declares Berakhot and is already under the rishonim. Moving it
        // again would put it under `תלמוד/בבלי/ראשונים/מפרשים`, which is a folder
        // nobody asked for.
        let rashi = work_of(
            "bavli/rashi-on-berakhot",
            &["Talmud", "Bavli", "Rishonim on Talmud", "Rashi"],
            &["bavli/berakhot"],
        );
        assert_eq!(
            shelf_of(&rashi).join("/"),
            "תלמוד/בבלי/ראשונים/Rashi",
            "a commentary on a commentary shelf stays on it"
        );
    }

    #[test]
    fn a_commentary_that_declares_its_base_is_a_commentary_on_it() {
        let rashi = work_of(
            "bavli/rashi-on-berakhot",
            &["Talmud", "Bavli", "Rishonim on Talmud", "Rashi"],
            &["bavli/berakhot"],
        );
        let berakhot = work_of("bavli/berakhot", &["Talmud", "Bavli", "Seder Zeraim"], &[]);
        assert_eq!(stands(&rashi, &berakhot), Stands::On);
    }

    #[test]
    fn rashi_on_berakhot_is_not_a_mefaresh_on_the_tur() {
        // The reader's words: *"rashi on berachos is put as a mefaresh on tur.
        // this is crazy."* It was, and the graph really does hold `comments-on`
        // edges between them — Sefaria's link types are not this careful.
        let rashi = work_of(
            "bavli/rashi-on-berakhot",
            &["Talmud", "Bavli", "Rishonim on Talmud", "Rashi"],
            &["bavli/berakhot"],
        );
        let tur = work_of("tur", &["Halakhah", "Tur"], &[]);
        assert_eq!(stands(&rashi, &tur), Stands::Apart);
    }

    #[test]
    fn a_masechta_is_not_a_mefaresh_on_the_shulchan_arukh() {
        // *"shabbos is put as a mefaresh on shulchan aruch which is absurd."*
        let shabbat = work_of("bavli/shabbat", &["Talmud", "Bavli", "Seder Moed"], &[]);
        let arukh = work_of(
            "shulchan-arukh/orach-chayim",
            &["Halakhah", "Shulchan Arukh"],
            &[],
        );
        assert_eq!(stands(&shabbat, &arukh), Stands::Apart);
        // And the Shulchan Arukh is not a *commentary* on the Tur, though it has
        // 697 commentary edges into it. It is not unrelated to it either: it
        // keeps the Tur's order, siman for siman, which is the whole reason the
        // edges exist. A bool had to call that `false` and mean two things.
        let tur = work_of("tur", &["Halakhah", "Tur"], &[]);
        assert_eq!(stands(&arukh, &tur), Stands::Alongside);
    }

    #[test]
    fn a_commentary_that_declares_nothing_is_known_by_the_shelf_it_stands_on() {
        // The Kaf HaChayim is the largest mefaresh on Orach Chayim — 29,956
        // edges — and Sefaria declares no base text for it. So *keep only the
        // declared ones* is not the fix: it would throw away the biggest one.
        //
        // Sefaria does say where it stands, though, and that is enough: a
        // commentary shelf, with Orach Chayim's own shelf directly above it.
        let kaf = work_of(
            "kaf-hachayim-on-shulchan-arukh/orach-chayim",
            &["Halakhah", "Shulchan Arukh", "Commentary", "Kaf HaChayim"],
            &[],
        );
        let arukh = work_of(
            "shulchan-arukh/orach-chayim",
            &["Halakhah", "Shulchan Arukh"],
            &[],
        );
        assert_eq!(stands(&kaf, &arukh), Stands::On);

        // The Beit Yosef on the Tur, same shape — and this one the old
        // declared-only reading would have dropped from the Tur entirely,
        // 18,353 edges of the sefer that is *the* commentary on it.
        let beit_yosef = work_of("beit-yosef", &["Halakhah", "Tur", "Commentary"], &[]);
        let tur = work_of("tur", &["Halakhah", "Tur"], &[]);
        assert_eq!(stands(&beit_yosef, &tur), Stands::On);
    }

    #[test]
    fn a_commentary_on_one_shelf_is_not_a_commentary_on_a_sefer_on_another() {
        // The negative half of the rule above. Being *a* commentary is not being
        // a commentary on *this*.
        let kaf = work_of(
            "kaf-hachayim-on-shulchan-arukh/orach-chayim",
            &["Halakhah", "Shulchan Arukh", "Commentary", "Kaf HaChayim"],
            &[],
        );
        let tur = work_of("tur", &["Halakhah", "Tur"], &[]);
        assert_eq!(stands(&kaf, &tur), Stands::Apart);
    }

    #[test]
    fn a_rishon_filed_in_hebrew_reaches_the_sefer_he_wrote_about() {
        // F2. `ר חננאל על בראשית` comes from Otzaria and is filed in Otzaria's
        // vocabulary. The old rule compared `categories` as written, so a work
        // whose shelf was in Hebrew could never equal a base whose shelf was in
        // English — Rabbeinu Chananel was not a mefaresh on Bereshis, and
        // nothing said why.
        //
        // Two things fix it and both are in this module already: `ראשונים` joins
        // the commentary prefixes, and the comparison runs over canonical paths.
        let rabbeinu_chananel = work_of(
            "ר-חננאל-על-בראשית",
            &["תנך", "ראשונים", "רבינו חננאל", "תורה"],
            &[],
        );
        let genesis = work_of("genesis", &["Tanakh", "Torah"], &[]);
        assert_eq!(stands(&rabbeinu_chananel, &genesis), Stands::On);

        // And he does not thereby become a mefaresh on Tehillim: he names תורה
        // and Tehillim names כתובים.
        let psalms = work_of("psalms", &["Tanakh", "Writings"], &[]);
        assert_eq!(stands(&rabbeinu_chananel, &psalms), Stands::Apart);
    }

    #[test]
    fn a_commentary_over_a_whole_division_is_referred_to_the_graph() {
        // F1. Bartenura on Torah declares no base and its shelf is
        // `תנ״ך/ראשונים` — above the *whole* of Tanakh. So the shelf permits
        // Bereshis and Tehillim alike and can choose neither, and the old rule's
        // equality test answered `false` to both, which read as *unrelated* and
        // silently emptied the Chumash of its acharonim.
        //
        // The honest answer is that this module does not know. It says so, and
        // the caller — which holds the graph — settles it: 330 comments into
        // Bereshis, none anywhere in Kesuvim.
        let bartenura = work_of("bartenura-on-torah", &["Tanakh", "Rishonim on Tanakh"], &[]);
        let genesis = work_of("genesis", &["Tanakh", "Torah"], &[]);
        assert_eq!(stands(&bartenura, &genesis), Stands::AskTheEdges);

        // Not a licence to roam: a different top shelf is still `Apart` without
        // anybody consulting anything.
        let berakhot = work_of("bavli/berakhot", &["Talmud", "Bavli", "Seder Zeraim"], &[]);
        assert_eq!(stands(&bartenura, &berakhot), Stands::Apart);
    }

    #[test]
    fn a_commentary_that_names_its_division_is_settled_without_the_graph() {
        // The other half of the rule above, and the reason `AskTheEdges` is rare
        // rather than the usual answer: where both name a division, the shelf is
        // enough and the graph is never asked.
        let ibn_ezra = work_of(
            "ibn-ezra-on-psalms",
            &["Tanakh", "Rishonim on Tanakh", "Ibn Ezra", "Writings"],
            &[],
        );
        let psalms = work_of("psalms", &["Tanakh", "Writings"], &[]);
        let genesis = work_of("genesis", &["Tanakh", "Torah"], &[]);
        assert_eq!(stands(&ibn_ezra, &psalms), Stands::On);
        assert_eq!(stands(&ibn_ezra, &genesis), Stands::Apart);
    }

    #[test]
    fn a_code_that_keeps_another_codes_order_stands_alongside_it() {
        // F4. The Arukh HaShulchan follows the Shulchan Arukh siman for siman
        // and is not a commentary on it; the Shulchan Arukh HaRav likewise. Both
        // were refused on the *shape* of their categories rather than by any
        // decision, which is the sort of accident this whole module exists to
        // stop.
        let orach_chayim = work_of(
            "shulchan-arukh/orach-chayim",
            &["Halakhah", "Shulchan Arukh"],
            &[],
        );
        for slug_and_shelf in [
            ("arukh-hashulchan", &["Halakhah"][..]),
            (
                "shulchan-arukh-harav",
                &["Halakhah", "Shulchan Arukh HaRav"][..],
            ),
        ] {
            let (slug, categories) = slug_and_shelf;
            let code = work_of(slug, categories, &[]);
            assert_eq!(
                stands(&code, &orach_chayim),
                Stands::Alongside,
                "{slug} should run alongside Orach Chayim"
            );
        }
    }

    #[test]
    fn running_alongside_does_not_reach_across_the_bookcase() {
        // The guard on `Alongside`, which is a looser claim than `On` and so
        // needs one. A masechta is not *alongside* the Shulchan Arukh and
        // Vayikra is not alongside Yoreh De'ah, however many `comments-on` edges
        // the graph holds between them — they are not even on the same shelf.
        let yoreh_deah = work_of(
            "shulchan-arukh/yoreh-deah",
            &["Halakhah", "Shulchan Arukh"],
            &[],
        );
        let shabbat = work_of("bavli/shabbat", &["Talmud", "Bavli", "Seder Moed"], &[]);
        let leviticus = work_of("leviticus", &["Tanakh", "Torah"], &[]);
        assert_eq!(stands(&shabbat, &yoreh_deah), Stands::Apart);
        assert_eq!(stands(&leviticus, &yoreh_deah), Stands::Apart);

        // But the Mishneh Torah's hilchos on the same subject *are* alongside:
        // same shelf, its own sefer, covering the ground in its own order.
        let rambam = work_of(
            "mishneh-torah/forbidden-foods",
            &["Halakhah", "Mishneh Torah", "Sefer Kedushah"],
            &[],
        );
        assert_eq!(stands(&rambam, &yoreh_deah), Stands::Alongside);
    }

    #[test]
    fn a_sefer_is_not_a_commentary_on_itself() {
        let arukh = work_of(
            "shulchan-arukh/orach-chayim",
            &["Halakhah", "Shulchan Arukh"],
            &[],
        );
        assert_eq!(stands(&arukh, &arukh), Stands::Apart);
    }

    #[test]
    fn the_rule_asks_the_shelf_and_not_the_slug() {
        // BUILDER.md rule 6. `X on Y` in a slug is not evidence: the two works
        // here are named for each other and stand nowhere near each other, and
        // the answer is no.
        let looks_right = work_of(
            "something-on-berakhot",
            &["Musar", "Acharonim"],
            &["berakhot-a-different-sefer"],
        );
        let berakhot = work_of("bavli/berakhot", &["Talmud", "Bavli", "Seder Zeraim"], &[]);
        assert_eq!(stands(&looks_right, &berakhot), Stands::Apart);
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
