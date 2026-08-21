//! How many works hold their chalakim inside themselves, and how many of them
//! can be reached by typing a mekor at them.
//!
//! The question came out of reading the window as a bachur: `טור אורח חיים סימן
//! א` landed nowhere, and so did every other branch work on the shelf. The
//! resolver was never the problem — it answers `Exact`, with `אורח חיים:1`. The
//! segments say `orach_chayim:1:1`, and nothing in between knew those were one
//! place.
//!
//! ```sh
//! cargo run -p girsa-search --example measure-branch-citations -- corpus
//! ```
//!
//! For each work whose schema names its sections, this takes the first segment
//! under each named section, writes its address the way a person writes it —
//! the sefer's Hebrew title, then each named level by the name the schema gives
//! it and each numbered level as a number — and asks whether it lands back on
//! that same segment. Nothing is inferred from the code: every row goes through
//! the same [`girsa_search::citation::Citations`] the search bar uses.
//!
//! Outcomes are counted apart, because they have different causes and
//! different owners:
//!
//! * **the sefer has no title the lexicon carries** — a gap in `lexicon.tsv`
//!   and nothing to do with sections;
//! * **landed on the segment** — the thing this measures;
//! * **did not land**, and then broken up by *why* — see [`why`]. One of those
//!   reasons is a refusal this repository makes **on purpose**, and printing
//!   one number for all of them made a working guard read as a defect.

// A measurement is a program that prints a number and stops. Panicking on a
// corpus that is not there is the honest failure — see `measure-ids`, which
// says the same.
#![allow(clippy::print_stdout, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use girsa_corpus::sections::Sections;
use girsa_search::citation::Citations;

fn main() {
    let root: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "corpus".into())
        .into();
    let body = std::fs::read_to_string(root.join("works/index.jsonl")).expect("a catalogue");
    let works: Vec<girsa_corpus::work::Work> = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let citations = Citations::open(&root, None).expect("a lexicon");
    let nowhere = girsa_ref::resolve::Context::default();

    let mut branch = 0usize;
    let mut untitled = 0usize;
    let mut asked = 0usize;
    let mut landed = 0usize;
    let mut missed: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();

    for work in &works {
        let sections = Sections::beside(&root, work.schema.as_deref());
        if sections.is_empty() {
            continue;
        }
        let samples = one_per_section(&root, &work.slug, &sections);
        if samples.is_empty() {
            continue;
        }
        branch += 1;
        // Whether the sefer's own title resolves at all, asked once and
        // separately. A title the lexicon does not carry is not a failure of
        // the chelek lookup, and counting the two together would hide
        // whichever is smaller.
        let alone = citations.look_up(&work.he_title, &nowhere);
        if alone.places.is_empty() && alone.unrefuted() == 0 {
            untitled += samples.len();
            continue;
        }
        for (typed, id, depth) in samples {
            let typed = format!("{} {typed}", work.he_title);
            asked += 1;
            let landing = citations.look_up(&typed, &nowhere);
            if landing
                .only()
                .is_some_and(|place| place.run.first.to_string() == id)
            {
                landed += 1;
            } else {
                missed
                    .entry(why(&sections, &landing, depth))
                    .or_default()
                    .push(typed);
            }
        }
    }

    let short: usize = missed.values().map(Vec::len).sum();
    println!("works whose schema names sections:  {branch}");
    println!("chalakim in a sefer with no title:  {untitled}  (a lexicon gap, not this one)");
    println!("chalakim asked for by name:         {asked}");
    println!("landed on the segment:              {landed}");
    println!("did not land:                       {short}");
    println!();
    for (why, ones) in &missed {
        println!("{:>5}  {why}", ones.len());
        for one in ones.iter().take(6) {
            println!("         {one}");
        }
        if ones.len() > 6 {
            println!("         … and {} more", ones.len() - 6);
        }
    }
}

/// Why one address did not land, in the words of the cause.
///
/// **Not every miss is a defect, and a measurement that cannot say which is
/// which reports a working guard as a bug.** The Chafetz Chaim's schema calls
/// two different sections `הקדמה`; `girsa_corpus::sections` refuses an
/// ambiguous name rather than picking one, which is BUILDER rule 6, and this
/// counted all 192 of those as failures for as long as it printed one number.
///
/// The three causes are told apart by what came back rather than by reading the
/// string:
///
/// * an address holding a name **this schema uses twice** was refused on
///   purpose;
/// * an address **shallower** than the segment's own path lost a level on the
///   way — a section named by a word the resolver knows as a level label,
///   `שער`, `חלק`, `פרשת`, where the word is swallowed and its number taken;
/// * an address that is neither is a name the schema does not carry in a form
///   this can match, which is the residue worth working on.
fn why(
    sections: &Sections,
    landing: &girsa_search::citation::Landing,
    depth: usize,
) -> &'static str {
    let Some(reference) = landing.resolution.candidates().first() else {
        return "the resolver read no reference out of it at all";
    };
    let address = sections.slugged(reference.from());
    if address
        .levels()
        .iter()
        .any(|level| sections.ambiguous(level.as_str()))
    {
        return "refused: this schema calls two sections by that name (rule 6)";
    }
    if address.depth() < depth {
        return "a word of the name was read as a level label and its number taken";
    }
    "the name is not one this schema carries in a form we can match"
}

/// One segment per named section, and the address a person would type for it.
///
/// Named levels are written by the name the schema gives them and numbered
/// levels stay numbers. A path holding a level that is neither — a daf, or a
/// section this schema is silent about — is skipped rather than typed as a
/// slug: that would be this measurement inventing a citation nobody would write
/// and then reporting the refusal as a defect.
fn one_per_section(root: &Path, slug: &str, sections: &Sections) -> Vec<(String, String, usize)> {
    let Ok(work) = girsa_corpus::import::read_back(root, slug) else {
        return Vec::new();
    };
    // The third of each row is how deep the segment's own address is, which is
    // what makes *a level went missing on the way* a thing `why` can see rather
    // than guess at.
    let mut out: BTreeMap<String, (String, String, usize)> = BTreeMap::new();
    for segment in &work.segments {
        let path = segment.id.path();
        let named = sections.named(path);
        let Some(chelek) = path.first() else { continue };
        if named == 0 || out.contains_key(chelek) {
            continue;
        }
        let mut said = Vec::with_capacity(path.len());
        let sayable = path.iter().enumerate().all(|(at, level)| {
            let word = if at < named {
                sections.titled(level).map(str::to_string)
            } else {
                level.parse::<u32>().ok().map(|n| n.to_string())
            };
            match word {
                Some(word) => {
                    said.push(word);
                    true
                }
                None => false,
            }
        });
        if sayable {
            out.insert(
                chelek.clone(),
                (said.join(" "), segment.id.to_string(), path.len()),
            );
        }
    }
    out.into_values().collect()
}
