//! Measure the lane against a real model, on a real sefer, with real questions.
//!
//! ```sh
//! cargo run --release -p girsa-lane --example measure -- corpus personal \
//!     mishneh-torah/prayer-and-the-priestly-blessing D:\berel
//! ```
//!
//! BUILDER.md W30's *done when* is **a query that shares no words with its
//! target finds it**, and that is a claim about a model rather than about code —
//! so it is measured the way W26 measured tesseract, and the numbers go in the
//! commit message and in `girsa_lane::model`'s own documentation rather than in
//! an assertion. The machinery around it is what
//! `girsa-app/tests/adjacent_is_never_the_answer.rs` holds, with a stub model,
//! so that the test suite needs no weights.
//!
//! # No shared words is checked rather than claimed
//!
//! Every pair below is a question somebody might really ask and the se'if that
//! answers it. The overlap between the two is **computed**, through
//! [`girsa_hebrew`] — nikud off, prefixes peeled, final letters folded, which is
//! the same normalizer literal search uses — and printed per pair. A pair with
//! any overlap at all is excluded from the headline number, because a pair that
//! shares a word is a pair literal search could have answered.

// A tool that prints a report.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use girsa_lane::model::{Embedder, Model};
use girsa_lane::vectors::Vectors;

/// Two kinds of query, because they are two different tasks and only one of
/// them is what spec.md §9.9 is about.
///
/// **A question** — *how late may one daven shacharis?* — is an **asymmetric**
/// retrieval task: the query and the passage are different kinds of text, and
/// answering it is what a model has to be trained for. **A half-remembered
/// statement** — *I think a Rishon says the drunk is exempt because he has no
/// kavanah* — is roughly **symmetric**: query and target are both statements of
/// the same claim in different words. That is §9.9's sentence, word for word:
/// *I remember a Rishon who says something like this but not the words.*
///
/// Both are measured, separately, because a model can be useless at the first
/// and useful at the second and reporting one number would hide it.
///
/// The address is the human one — `4:17` — resolved against the sefer's own
/// segments, so a pair does not silently start pointing at a different line if
/// the work is re-imported.
const QUESTIONS: [(&str, &str); 12] = [
    ("אישה מחויבת בתחינה", "1:2"),
    ("התערבות ישראל באומות וקלקול שפתם", "1:4"),
    ("כמה שבחים אומר בכל יום", "2:2"),
    ("כמה אומרים ביום המנוחה", "2:5"),
    ("מאיזו שעה מבקשים על מטר", "2:15"),
    ("הזמן האחרון של שמונה עשרה בבוקר", "3:1"),
    ("מי ששכח ולא עמד בבוקר, מה יעשה", "3:9"),
    ("אדם שתוי אינו רשאי לעמוד בשמונה עשרה", "4:17"),
    ("מה מברכים כשרואים נס", "2:13"),
    ("מקום שאין בו עשרה אנשים", "8:1"),
    ("נשיאת ידיים של בני אהרן בבית הבחירה", "14:14"),
    ("ברכה לאחר האוכל בשלושה", "7:9"),
];

/// The same seforim, asked the way §9.9 says a reader asks: a claim they half
/// remember, in their own words.
const STATEMENTS: [(&str, &str); 10] = [
    (
        "קבעו עמידה נוספת בשעות החשכה, כנגד אימורי הקרבן הנשרפים עד אור הבוקר",
        "1:6",
    ),
    ("מנין הבקשות שתקנו הוא כמנין הזבחים שהיו מקריבים", "1:5"),
    (
        "קהל אינו מוסיף עמידה שאינה חובה, לפי שאין הרבים מביאים זבח רשות",
        "1:10",
    ),
    ("כשהתרבו הכופרים בימי הנשיא, הוסיפו ברכה אחת כנגדם", "2:1"),
    ("בעונת הקור מזכיר את המטר, ובעונת החום את האגל", "2:15"),
    (
        "המוקדמת מחצי היום ומחצה, והמאוחרת מתשע ומחצה עד הערב",
        "3:4",
    ),
    ("מי שהזיד ולא עמד בזמנו, שוב אין לו השלמה", "3:8"),
    ("מי שנשתכר ביין לא יעמוד, לפי שאין דעתו מכוונת", "4:17"),
    (
        "בבית הבחירה נשאו בני אהרן את כפיהם פעם אחת אחר זבח הבוקר, על מדרגות ההיכל",
        "14:14",
    ),
    (
        "כשמסיימים בני אהרן את שלושת הפסוקים, פותח החזן בברכה שהיא שים שלום",
        "14:4",
    ),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, personal, slug, model] = args.as_slice() else {
        eprintln!("usage: measure <corpus> <personal> <slug> <model-dir>");
        std::process::exit(2);
    };
    let (root, personal) = (PathBuf::from(root), PathBuf::from(personal));

    let model = match Model::side_loaded(Path::new(model)) {
        Ok(model) => model,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let work = match girsa_corpus::import::read_back(&root, slug) {
        Ok(work) => work,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let (vectors, trouble) = Vectors::open(&personal, slug, model.fingerprint(), model.dims());
    for line in &trouble {
        eprintln!("{line}");
    }
    if vectors.is_empty() {
        eprintln!("nothing is embedded of {slug} — run `girsa-lane {root:?} {personal:?} embed`");
        std::process::exit(1);
    }
    println!(
        "{} · {} segments embedded of {slug}\n",
        model.named(),
        vectors.len()
    );

    // Every vector, in memory, so the same pairs can be ranked two ways in one
    // run: as the store ranks them, and with the mean of what is embedded
    // subtracted from both sides. The second is the standard repair for the one
    // thing an un-finetuned BERT does badly — every sentence sits in a narrow
    // cone, so the dominant direction of the space carries no information and
    // dominates the cosine anyway. Measured before it is built.
    let all: Vec<(girsa_corpus::segment::SegmentId, Vec<f32>)> = match vectors.all() {
        Ok(iter) => iter
            .filter_map(Result::ok)
            .map(|v| (v.id, v.vector))
            .collect(),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let mut mean = vec![0.0f32; model.dims()];
    for (_, vector) in &all {
        for (at, value) in vector.iter().enumerate() {
            mean[at] += value;
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let n = all.len() as f32;
    for value in &mut mean {
        *value /= n;
    }
    let centred: Vec<(girsa_corpus::segment::SegmentId, Vec<f32>)> = all
        .iter()
        .map(|(id, vector)| (id.clone(), centre(vector, &mean)))
        .collect();

    for (what, pairs) in [
        ("A question about the se'if", QUESTIONS.as_slice()),
        ("The se'if, half remembered", STATEMENTS.as_slice()),
    ] {
        println!("\n== {what} ==");
        run(&model, &vectors, &work, &all, &centred, &mean, pairs);
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    model: &Model,
    vectors: &Vectors,
    work: &girsa_corpus::import::ImportedWork,
    all: &[(girsa_corpus::segment::SegmentId, Vec<f32>)],
    centred: &[(girsa_corpus::segment::SegmentId, Vec<f32>)],
    mean: &[f32],
    pairs: &[(&str, &str)],
) {
    let _ = all;
    let most = vectors.len();
    let mut clean = 0;
    let mut at_1 = 0;
    let mut at_5 = 0;
    let mut at_10 = 0;
    let mut centred_at_1 = 0;
    let mut centred_at_5 = 0;
    let mut centred_at_10 = 0;
    println!(
        "{:<44} {:>5} {:>6} {:>7} {:>8}  target",
        "asked", "over", "rank", "cosine", "centred"
    );
    println!("{}", "-".repeat(92));

    for (asked, address) in pairs.iter().copied() {
        let Some(target) = work
            .segments
            .iter()
            .find(|segment| segment.id.path().join(":") == address)
        else {
            println!("{asked:<44} {address} — no such se'if");
            continue;
        };
        let overlap = shared(asked, &target.text);
        let query = match model.embed(&[asked]) {
            Ok(query) => query,
            Err(e) => {
                eprintln!("{asked}: {e}");
                continue;
            }
        };
        let ranked = match vectors.nearest(&query[0].vector, most) {
            Ok(ranked) => ranked,
            Err(e) => {
                eprintln!("{asked}: {e}");
                continue;
            }
        };
        let (rank, cosine) = match ranked.iter().position(|(id, _)| *id == target.id) {
            Some(at) => (at + 1, ranked[at].1),
            None => (0, 0.0),
        };
        let by_centre = rank_in(centred, &centre(&query[0].vector, mean), &target.id);

        println!(
            "{asked:<44} {:>5} {rank:>6} {cosine:>7.4} {by_centre:>8}  {address}",
            overlap.len()
        );
        if overlap.is_empty() {
            clean += 1;
            at_1 += usize::from(rank == 1);
            at_5 += usize::from((1..=5).contains(&rank));
            at_10 += usize::from((1..=10).contains(&rank));
            centred_at_1 += usize::from(by_centre == 1);
            centred_at_5 += usize::from((1..=5).contains(&by_centre));
            centred_at_10 += usize::from((1..=10).contains(&by_centre));
        } else {
            println!(
                "     shares: {}",
                overlap.iter().cloned().collect::<Vec<_>>().join(" ")
            );
        }
    }

    println!(
        "\n{clean} of {} pairs share no word with their target.",
        pairs.len()
    );
    if clean > 0 {
        println!(
            "  of those, out of {} embedded segments:\n    \
             as stored: rank 1 for {at_1}, top 5 for {at_5}, top 10 for {at_10}\n    \
             centred:   rank 1 for {centred_at_1}, top 5 for {centred_at_5}, \
             top 10 for {centred_at_10}",
            vectors.len()
        );
    }
}

/// A vector with the mean of the space taken out of it, re-normalized.
fn centre(vector: &[f32], mean: &[f32]) -> Vec<f32> {
    let mut out: Vec<f32> = vector
        .iter()
        .zip(mean)
        .map(|(value, mean)| value - mean)
        .collect();
    let norm = out.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
    for value in &mut out {
        *value /= norm;
    }
    out
}

/// Where a target lands when everything is ranked against a query. `0` for not
/// found at all.
fn rank_in(
    all: &[(girsa_corpus::segment::SegmentId, Vec<f32>)],
    query: &[f32],
    target: &girsa_corpus::segment::SegmentId,
) -> usize {
    let mut scored: Vec<(&girsa_corpus::segment::SegmentId, f32)> = all
        .iter()
        .map(|(id, vector)| {
            (
                id,
                query.iter().zip(vector).map(|(a, b)| a * b).sum::<f32>(),
            )
        })
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored
        .iter()
        .position(|(id, _)| *id == target)
        .map_or(0, |at| at + 1)
}

/// The words two lines have in common, under the normalizer literal search uses.
///
/// Through `girsa-hebrew` rather than by splitting on spaces: `וּבַשַּׁבָּת` and `שבת`
/// are the same word and a measurement that called them different would be
/// crediting the lane for something the literal index does perfectly well.
///
/// **Nikud off and nothing else**, because that is exactly what Torat Emet does
/// (spec.md §9.1, §9.3) and Torat Emet is what the lane is being measured
/// against. Peeling prefixes as well was tried and thrown out: it made
/// `שְׁתוּיִ` share `תוי` with a word that has nothing to do with it, and a
/// measurement whose *shares a word* column is full of two-letter fragments is a
/// measurement that excludes its own best pairs for no reason.
fn shared(a: &str, b: &str) -> BTreeSet<String> {
    let words = |text: &str| -> BTreeSet<String> {
        girsa_hebrew::tokenize(text)
            .into_iter()
            .map(|token| girsa_hebrew::normalize(&token.text))
            .filter(|word| word.chars().count() > 1)
            .collect()
    };
    words(a).intersection(&words(b)).cloned().collect()
}
