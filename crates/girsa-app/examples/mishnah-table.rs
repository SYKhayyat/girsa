//! Emit `luach::MISHNAYOS` from the corpus, so nobody types 525 numbers.
//!
//! ```sh
//! cargo run --release -p girsa-app --example mishnah-table -- corpus
//! ```
//!
//! # What this exists to stop
//!
//! Mishnah Yomis needs to know how many mishnayos each perek holds, or it
//! cannot say where a day lands. That is roughly 525 numbers across 63
//! masechtos, and a single wrong one is a wrong limud for a day with nothing
//! to catch it — the same hazard as a wrong epoch, one level down. So they are
//! counted rather than recalled, and the output is pasted into `luach.rs`
//! where a test asserts the total.
//!
//! # Three traps, all of them met
//!
//! - **`categories[0] == "Mishnah"` finds 948 works**, because every
//!   commentary on Mishnayos is filed under it too — Rishonim, Acharonim and
//!   modern. The filter also requires `categories[1]` to be one of the six
//!   sedarim, which finds 63.
//! - **A `mishnah-*` glob finds 62 and is wrong twice.** It misses
//!   `pirkei-avot`, which Sefaria does not name that way, and it sweeps in
//!   `mishnah-berurah` and its 17,418 segments.
//! - **The order is not alphabetical and is not the Bavli's.** It is the seder,
//!   then Sefaria's own `order` field, which is what this reads.
//!
//! The count comes to **4,192**, which is the number Mishnah Yomis is built on
//! and, separately, the number of days between two published cycle starts.
//! Getting a different number means the corpus is not what this assumed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The sedarim, in order. The one piece of sequence not taken from the corpus,
/// because `categories[1]` is a name and not an index.
const SEDARIM: [&str; 6] = [
    "Seder Zeraim",
    "Seder Moed",
    "Seder Nashim",
    "Seder Nezikin",
    "Seder Kodashim",
    "Seder Tahorot",
];

struct Found {
    slug: String,
    said: String,
    seder: usize,
    order: i64,
    perakim: Vec<u32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let corpus: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "corpus".into())
        .into();
    let works = corpus.join("works");

    let mut found: Vec<Found> = Vec::new();
    for entry in std::fs::read_dir(&works)? {
        let dir = entry?.path();
        let Ok(text) = std::fs::read_to_string(dir.join("work.json")) else {
            continue;
        };
        let work: serde_json::Value = serde_json::from_str(&text)?;
        let categories: Vec<&str> = work["categories"]
            .as_array()
            .map(|a| a.iter().filter_map(serde_json::Value::as_str).collect())
            .unwrap_or_default();
        if categories.first() != Some(&"Mishnah") {
            continue;
        }
        let Some(seder) = categories
            .get(1)
            .and_then(|name| SEDARIM.iter().position(|s| s == name))
        else {
            continue;
        };
        let slug = work["slug"].as_str().unwrap_or_default().to_string();
        let said = work["he_title"]
            .as_str()
            .unwrap_or_default()
            .trim_start_matches("משנה ")
            .to_string();
        let order = work["order"][0].as_i64().unwrap_or(0);
        found.push(Found {
            perakim: perakim_of(&dir)?,
            slug,
            said,
            seder,
            order,
        });
    }
    found.sort_by_key(|f| (f.seder, f.order));

    let total: u32 = found.iter().flat_map(|f| f.perakim.iter()).sum();
    eprintln!("masechtos: {}", found.len());
    eprintln!("mishnayos: {total}");
    if found.len() != 63 || total != 4_192 {
        eprintln!("!! expected 63 masechtos and 4,192 mishnayos — do not paste this");
    }

    println!("const MISHNAYOS: &[Maseches] = &[");
    for f in &found {
        let counts: Vec<String> = f.perakim.iter().map(u32::to_string).collect();
        println!(
            "    ms(\"{}\", \"{}\", &[{}]),",
            f.slug,
            f.said,
            counts.join(", ")
        );
    }
    println!("];");
    Ok(())
}

/// How many mishnayos each perek of one maseches holds.
///
/// Counted from the segment ids — `girsa:mishnah-berakhot/1:1#1` — rather than
/// from the schema, because the ids are what the reader will actually open.
fn perakim_of(dir: &Path) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(dir.join("segments.jsonl"))?;
    let mut counts: BTreeMap<u32, u32> = BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let segment: serde_json::Value = serde_json::from_str(line)?;
        let Some(id) = segment["id"].as_str() else {
            continue;
        };
        let Some((_, address)) = id.split_once('/') else {
            continue;
        };
        let address = address.split('#').next().unwrap_or_default();
        let mut levels = address.split(':');
        let (Some(perek), Some(_mishnah), None) = (levels.next(), levels.next(), levels.next())
        else {
            continue;
        };
        if let Ok(perek) = perek.parse::<u32>() {
            *counts.entry(perek).or_default() += 1;
        }
    }
    Ok(counts.into_values().collect())
}
