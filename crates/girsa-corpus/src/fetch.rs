//! Fetching the Sefaria export.
//!
//! Hebrew `merged.json`, every schema, and the link CSVs from
//! `gs://sefaria-export`. English and both `cltk-*` formats are skipped — they
//! are most of the bucket and none of the product (spec.md §2.3b).
//!
//! # Measured size, which is not the size the spec states
//!
//! spec.md §2.3b budgets **~2.2 GB across ~12,700 files**, from a 40-title
//! sample. Listing the bucket gives **12,826 files and 3.3 GB** — the file
//! count is nearly exact, so the shape of the estimate was right and the mean
//! file size was under-sampled by about half.
//!
//! It is reported rather than coded around. The consequence is a longer first
//! run than §5 promises, and §2.3b's "one evening, not a project" is still
//! true; but the number in the spec is stale and the number here is what the
//! bucket says today.
//!
//! # Three properties, and none of them are polish
//!
//! **Resumable.** 3.3 GB over a domestic connection will be interrupted. A
//! restart re-lists nothing and re-downloads nothing: the plan is cached, and a
//! file already on disk at its stated size is skipped.
//!
//! **Atomic per file.** Every download lands at `x.part` and is renamed into
//! place only once it is complete and the right size. There is therefore no
//! such thing as a half-written sefer on disk — a file either is not there or
//! is whole.
//!
//! **Readable while it lands.** That last property is what makes spec.md §5's
//! promise work: you can start learning from the shelves that have arrived
//! while the rest is still coming, because nothing on disk is ever in an
//! in-between state.

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// The public bucket. No auth, no account, no key.
const BUCKET: &str = "sefaria-export";
/// `books.json` is not in the bucket; it lives in the git repository.
const BOOKS_JSON: &str =
    "https://raw.githubusercontent.com/Sefaria/Sefaria-Export/master/books.json";

/// Where a resumed run picks the plan back up, so a restart does not re-list
/// 85,000 objects to discover it already knew what to do.
const PLAN_FILE: &str = ".girsa-fetch-plan.json";

/// One file to fetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// Where to get it.
    pub url: String,
    /// Where it goes, relative to the corpus root.
    pub rel_path: String,
    /// Size the bucket reports. A file on disk of a different size is
    /// incomplete and gets fetched again — the check that makes "already there"
    /// mean something.
    pub size: Option<u64>,
}

/// Everything a full fetch consists of.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Plan {
    pub targets: Vec<Target>,
}

impl Plan {
    /// Total bytes the plan will move, where the bucket told us.
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.targets.iter().filter_map(|t| t.size).sum()
    }
}

/// What went wrong.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http {status} for {url}")]
    Http { status: u16, url: String },
    #[error("transport for {url}: {source}")]
    Transport {
        url: String,
        #[source]
        source: Box<ureq::Error>,
    },
    #[error("{url} gave {got} bytes, the bucket says {want}")]
    ShortRead { url: String, got: u64, want: u64 },
    #[error("a worker thread died")]
    WorkerLost,
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

/// Build the plan, listing the bucket. Cached to the corpus root.
///
/// Ordered deliberately: schemas first, then links, then texts. The schemas are
/// 32 MB and carry the title variants the whole resolver is seeded from, so
/// they are worth having ten minutes in rather than two hours in. Texts come
/// last because they are the part you can begin reading before it finishes.
pub fn plan(root: &Path) -> Result<Plan, FetchError> {
    let cached = root.join(PLAN_FILE);
    if let Ok(bytes) = fs::read(&cached) {
        if let Ok(plan) = serde_json::from_slice::<Plan>(&bytes) {
            if !plan.targets.is_empty() {
                eprintln!("resuming a cached plan of {} files", plan.targets.len());
                return Ok(plan);
            }
        }
    }

    let mut targets = Vec::new();

    targets.push(Target {
        url: BOOKS_JSON.to_string(),
        rel_path: "books.json".to_string(),
        size: None,
    });

    eprintln!("listing schemas/ …");
    targets.extend(list_prefix("schemas/")?.into_iter().map(target_from));

    eprintln!("listing links/ …");
    targets.extend(
        list_prefix("links/")?
            .into_iter()
            .filter(|o| o.name.ends_with(".csv"))
            .map(target_from),
    );

    eprintln!("listing json/ … (this one is long)");
    targets.extend(
        list_prefix("json/")?
            .into_iter()
            .filter(|o| is_wanted_text(&o.name))
            .map(target_from),
    );

    let plan = Plan { targets };
    fs::create_dir_all(root)?;
    fs::write(&cached, serde_json::to_vec_pretty(&plan)?)?;
    Ok(plan)
}

/// Hebrew `merged.json` only.
///
/// English is roughly half the bucket and Girsa ships what exists rather than
/// translating (spec.md §14). Both `cltk-*` formats are research artifacts of a
/// different project. `merged.json` is the version-merged text, which is the one
/// to read; the per-edition files under the same directory are for someone
/// comparing editions, which is not this.
fn is_wanted_text(name: &str) -> bool {
    name.contains("/Hebrew/") && name.ends_with("/merged.json") && !name.contains("/cltk")
}

fn target_from(o: Object) -> Target {
    Target {
        url: format!(
            "https://storage.googleapis.com/{BUCKET}/{}",
            urlencode_path(&o.name)
        ),
        rel_path: o.name,
        size: o.size.and_then(|s| s.parse().ok()),
    }
}

/// Percent-encode the characters that are legal in an object name and not in a
/// URL path. Sefaria's names carry commas, apostrophes and spaces
/// (`Shulchan_Arukh,_Orach_Chayim.json`, `Rashi_on_Genesis`), and getting this
/// wrong fails on exactly the seforim with the most interesting titles.
fn urlencode_path(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for b in name.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char);
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Turn a bucket object name into a path Windows will actually accept.
///
/// Sefaria has books whose titles contain `?` and `"`:
///
/// ```text
/// schemas/Will_We_Have_Jewish_Grandchildren?_Jewish_Continuity_and_How_to_Achieve_It.json
/// schemas/Conversion_"According_to_Halakhah";_What_Is_It.json
/// ```
///
/// Those are legal object names, legal on Linux and macOS, and rejected
/// outright by Windows — `os error 123`, before a byte is written. Three
/// seforim, which is not many until you notice that the same three would go
/// missing from every Windows install with nothing in the corpus to show for
/// it. The characters are percent-encoded, reversibly and stably, so a resume
/// finds the same file it wrote.
///
/// Reserved device names get the same treatment: a file called `CON.json`
/// cannot exist on Windows at any path, for reasons dating to CP/M.
///
/// # And it may not leave the corpus root
///
/// Everything above is about names Windows will not accept. This is about a
/// name that would be accepted and should not be: **the name comes off the
/// wire.** `target_from` takes `o.name` straight out of the bucket listing
/// JSON, and `is_wanted_text` only asks that it contain `/Hebrew/` and end in
/// `/merged.json` — which
/// `anything/Hebrew/../../../../../evil/merged.json` satisfies. `root.join` on
/// that escapes the corpus root and `fetch_one` writes there.
///
/// It needs a compromised bucket or a successful attack on HTTPS, so it is
/// hardening rather than a bug being exploited. It is also four lines, and a
/// sanitiser this careful about `?` and `CON` having no opinion about `..` is
/// the kind of gap that reads as deliberate to whoever finds it next.
///
/// `..`, `.` and an empty component are **encoded rather than dropped**, which
/// is the same call every other rule here makes: a name is refused or
/// preserved, never silently repaired into a different name that then collides
/// with a real one. An object genuinely called `..` — there is no such object —
/// would land as `%2E%2E`, reversibly.
fn disk_path(rel_path: &str) -> String {
    const FORBIDDEN: [char; 8] = ['<', '>', ':', '"', '\\', '|', '?', '*'];
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    // A name that begins with a separator, or with a drive letter, is an
    // **absolute** path, and `Path::join` throws the root away and takes it
    // whole. Trimmed here so that every component below is a component.
    let rel_path = rel_path.trim_start_matches(['/', '\\']);
    let mut out = String::with_capacity(rel_path.len());
    for (i, component) in rel_path.split('/').enumerate() {
        if i > 0 {
            out.push('/');
        }

        let mut encoded = String::with_capacity(component.len());
        for c in component.chars() {
            if FORBIDDEN.contains(&c) || (c as u32) < 0x20 {
                encoded.push_str(&format!("%{:02X}", c as u32));
            } else {
                encoded.push(c);
            }
        }
        // A component may not end in a space or a dot. `.json` is a dot
        // followed by letters, so only a *trailing* one is a problem.
        while encoded.ends_with(' ') || encoded.ends_with('.') {
            let last = encoded.pop().unwrap_or('.');
            encoded.push_str(&format!("%{:02X}", last as u32));
        }
        let stem = encoded.split('.').next().unwrap_or("");
        if RESERVED.contains(&stem.to_ascii_uppercase().as_str()) {
            encoded.insert_str(0, "%00");
        }
        // A component that walks out of the root, or names the directory it is
        // already in, or is nothing at all. See the note above: encoded, not
        // dropped. The trailing-dot rule above already turns a bare `..` into
        // `.%2E`, and that is not something to rely on — it is a rule about
        // Windows that happens to help, and it would stop helping the moment
        // somebody decided trailing dots were fine on this platform.
        if encoded == ".." || encoded == "." || encoded.is_empty() {
            encoded = encoded.replace('.', "%2E");
        }

        out.push_str(&encoded);
    }
    out
}

#[derive(Debug, Deserialize)]
struct Object {
    name: String,
    size: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Listing {
    #[serde(default)]
    items: Vec<Object>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

fn list_prefix(prefix: &str) -> Result<Vec<Object>, FetchError> {
    let mut out = Vec::new();
    let mut token: Option<String> = None;
    loop {
        let mut url = format!(
            "https://storage.googleapis.com/storage/v1/b/{BUCKET}/o\
             ?prefix={}&maxResults=1000&fields=items(name,size),nextPageToken",
            urlencode_path(prefix)
        );
        if let Some(t) = &token {
            url.push_str("&pageToken=");
            url.push_str(&urlencode_query(t));
        }

        let body = get_string(&url)?;
        let page: Listing = serde_json::from_str(&body)?;
        out.extend(page.items);
        eprint!("\r  {prefix} {} objects", out.len());
        match page.next_page_token {
            Some(t) => token = Some(t),
            None => break,
        }
    }
    eprintln!();
    Ok(out)
}

fn urlencode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Running
// ---------------------------------------------------------------------------

/// How the run is going, for a caller that wants to draw a progress bar.
#[derive(Debug, Default)]
pub struct Progress {
    /// Files already on disk when the run started, or finished during it.
    pub done: AtomicUsize,
    /// Files fetched by this run.
    pub fetched: AtomicUsize,
    /// Files that failed every attempt. A non-zero value here means the corpus
    /// is incomplete, and the caller must say so rather than carrying on.
    pub failed: AtomicUsize,
    /// Bytes moved by this run.
    pub bytes: AtomicUsize,
}

/// Fetch everything in the plan that is not already on disk.
///
/// Returns the number of targets that could not be fetched. A caller that
/// treats a non-zero return as success has built the silent-partial-import that
/// BUILDER.md exists to prevent.
pub fn run(root: &Path, plan: &Plan, threads: usize) -> Result<usize, FetchError> {
    let progress = Arc::new(Progress::default());
    let total = plan.targets.len();

    // Skip what is already whole before spawning anything, so the worker count
    // is not spent on files that need no work.
    let outstanding: Vec<Target> = plan
        .targets
        .iter()
        .filter(|t| !already_complete(root, t))
        .cloned()
        .collect();
    progress
        .done
        .store(total - outstanding.len(), Ordering::Relaxed);

    eprintln!(
        "{} of {total} files already on disk; fetching {}",
        total - outstanding.len(),
        outstanding.len()
    );
    if outstanding.is_empty() {
        return Ok(0);
    }

    // Workers take from the back, so the queue is reversed to make them come
    // out in plan order. Without this the ordering `plan()` is careful about is
    // exactly inverted: the texts arrive first and the 32 MB of schemas — the
    // part every later work order is seeded from — arrives last, two hours in.
    let mut outstanding = outstanding;
    outstanding.reverse();
    let queue = Arc::new(Mutex::new(outstanding));
    let root = root.to_path_buf();

    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            let queue = Arc::clone(&queue);
            let progress = Arc::clone(&progress);
            let root = root.clone();
            scope.spawn(move || loop {
                let Some(target) = next_target(&queue) else {
                    return;
                };
                match fetch_one(&root, &target) {
                    Ok(bytes) => {
                        progress.fetched.fetch_add(1, Ordering::Relaxed);
                        progress.bytes.fetch_add(bytes, Ordering::Relaxed);
                    }
                    Err(e) => {
                        progress.failed.fetch_add(1, Ordering::Relaxed);
                        eprintln!("\nFAILED {}: {e}", target.rel_path);
                    }
                }
                let done = progress.done.fetch_add(1, Ordering::Relaxed) + 1;
                if done % 25 == 0 || done == total {
                    let mb = progress.bytes.load(Ordering::Relaxed) / 1_048_576;
                    eprint!("\r  {done}/{total} files · {mb} MB");
                }
            });
        }
    });

    eprintln!();
    Ok(progress.failed.load(Ordering::Relaxed))
}

fn next_target(queue: &Mutex<Vec<Target>>) -> Option<Target> {
    // A poisoned queue means another worker panicked mid-pop. Stopping is
    // right: the alternative is a run that reports success having skipped an
    // unknown number of seforim.
    queue.lock().ok()?.pop()
}

/// Whether the file is already there *and whole*.
///
/// Existence alone is not enough. An interrupted process leaves `x.part`, never
/// `x` — but a disk that filled up, or an older run from before the size was
/// recorded, can still leave a short file at the real path.
fn already_complete(root: &Path, target: &Target) -> bool {
    let path = root.join(disk_path(&target.rel_path));
    match (fs::metadata(&path), target.size) {
        (Ok(meta), Some(want)) => meta.len() == want,
        (Ok(meta), None) => meta.len() > 0,
        (Err(_), _) => false,
    }
}

fn fetch_one(root: &Path, target: &Target) -> Result<usize, FetchError> {
    let final_path = root.join(disk_path(&target.rel_path));

    let mut body = get_bytes(&target.url)?;
    if let Some(want) = target.size {
        if body.len() as u64 != want {
            return Err(FetchError::ShortRead {
                url: target.url.clone(),
                got: body.len() as u64,
                want,
            });
        }
    }

    // Write, flush, close, *then* rename. The rename is the only moment the
    // file becomes visible under its real name, so a reader never sees a
    // partial sefer. `beside::write` does the three steps; its temp name
    // appends `.part` rather than replacing the extension, so two files whose
    // names differ only by extension cannot share one.
    crate::beside::write(&final_path, &body)?;

    let n = body.len();
    body.clear();
    Ok(n)
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

/// Retries, because 12,000 requests over a domestic connection will not all
/// succeed the first time and a whole-corpus fetch that gives up on one blip is
/// not resumable in any useful sense.
const ATTEMPTS: u32 = 4;

fn get_bytes(url: &str) -> Result<Vec<u8>, FetchError> {
    let mut last = None;
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(400 << attempt));
        }
        match try_get(url) {
            Ok(bytes) => return Ok(bytes),
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or(FetchError::WorkerLost))
}

fn try_get(url: &str) -> Result<Vec<u8>, FetchError> {
    let response = ureq::get(url).call().map_err(|source| match &source {
        ureq::Error::StatusCode(status) => FetchError::Http {
            status: *status,
            url: url.to_string(),
        },
        _ => FetchError::Transport {
            url: url.to_string(),
            source: Box::new(source),
        },
    })?;

    let mut buf = Vec::new();
    response
        .into_body()
        .into_reader()
        .read_to_end(&mut buf)
        .map_err(FetchError::Io)?;
    Ok(buf)
}

fn get_string(url: &str) -> Result<String, FetchError> {
    Ok(String::from_utf8_lossy(&get_bytes(url)?).into_owned())
}

// ---------------------------------------------------------------------------
// Reporting what landed
// ---------------------------------------------------------------------------

/// What is on disk right now, for the header that tells a reader which shelves
/// have arrived.
#[must_use]
pub fn landed(root: &Path, plan: &Plan) -> (usize, usize) {
    let done = plan
        .targets
        .iter()
        .filter(|t| already_complete(root, t))
        .count();
    (done, plan.targets.len())
}

/// The distinct top-level parts of the export the plan covers, for a summary.
#[must_use]
pub fn sections(plan: &Plan) -> BTreeSet<&str> {
    plan.targets
        .iter()
        .filter_map(|t| t.rel_path.split('/').next())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_and_cltk_are_not_in_the_plan() {
        assert!(is_wanted_text(
            "json/Talmud/Bavli/Berakhot/Hebrew/merged.json"
        ));
        assert!(!is_wanted_text(
            "json/Talmud/Bavli/Berakhot/English/merged.json"
        ));
        assert!(!is_wanted_text(
            "json/Talmud/Bavli/Berakhot/Hebrew/cltk-merged.json"
        ));
        // A single edition rather than the merged text.
        assert!(!is_wanted_text(
            "json/Talmud/Bavli/Berakhot/Hebrew/Wikisource_Talmud_Bavli.json"
        ));
    }

    #[test]
    fn a_title_with_punctuation_survives_being_put_in_a_url() {
        // The seforim with the most interesting names are the ones that break
        // a naive URL join.
        assert_eq!(
            urlencode_path("schemas/Shulchan_Arukh,_Orach_Chayim.json"),
            "schemas/Shulchan_Arukh%2C_Orach_Chayim.json"
        );
        assert_eq!(
            urlencode_path("json/Tanakh/Torah/Genesis/Hebrew/merged.json"),
            "json/Tanakh/Torah/Genesis/Hebrew/merged.json"
        );
        assert!(urlencode_path("schemas/Ba'al HaTurim.json").contains("%27"));
        assert!(urlencode_path("schemas/Ba'al HaTurim.json").contains("%20"));
    }

    #[test]
    fn a_title_windows_refuses_still_lands_somewhere() {
        // These three are real. Before this, they failed with os error 123 and
        // the corpus was quietly three seforim short on every Windows install.
        for name in [
            "schemas/Will_We_Have_Jewish_Grandchildren%3F_Jewish_Continuity.json",
            "schemas/One_People%3F_Tradition,_Modernity,_and_Jewish_Unity.json",
            "schemas/Conversion_%22According_to_Halakhah%22;_What_Is_It.json",
        ] {
            assert!(
                !name.contains('?') && !name.contains('"'),
                "the expectation itself is wrong"
            );
        }
        assert_eq!(
            disk_path("schemas/One_People?_Tradition.json"),
            "schemas/One_People%3F_Tradition.json"
        );
        assert_eq!(
            disk_path("schemas/Conversion_\"According_to_Halakhah\".json"),
            "schemas/Conversion_%22According_to_Halakhah%22.json"
        );
    }

    #[test]
    fn a_path_separator_survives_but_a_backslash_does_not() {
        // `/` is the directory structure and must stay. `\` is a character in a
        // title on Linux and a separator on Windows, so it is encoded.
        assert_eq!(
            disk_path("json/Tanakh/Torah/Genesis/Hebrew/merged.json"),
            "json/Tanakh/Torah/Genesis/Hebrew/merged.json"
        );
        assert!(disk_path("schemas/A\\B.json").contains("%5C"));
    }

    #[test]
    fn a_component_may_not_end_in_a_dot_or_a_space() {
        assert!(disk_path("schemas/Trailing .json").ends_with(".json"));
        assert_eq!(disk_path("schemas/Odd."), "schemas/Odd%2E");
    }

    #[test]
    fn a_reserved_device_name_is_moved_out_of_the_way() {
        // A file called CON.json cannot exist on Windows at any path.
        assert_ne!(disk_path("schemas/CON.json"), "schemas/CON.json");
        assert_ne!(disk_path("schemas/com1.json"), "schemas/com1.json");
        assert_eq!(
            disk_path("schemas/Connections.json"),
            "schemas/Connections.json"
        );
    }

    /// A name off the wire may not walk out of the corpus root.
    ///
    /// `is_wanted_text` asks only for `/Hebrew/` and `/merged.json`, and
    /// `anything/Hebrew/../../../../../evil/merged.json` has both. This is
    /// hardening — it needs a compromised bucket — and it is the one rule in
    /// `disk_path` whose absence is a security property rather than a missing
    /// sefer.
    #[test]
    fn a_downloaded_name_cannot_leave_the_corpus_root() {
        let root = Path::new("/corpus");
        for name in [
            "json/Tanakh/Hebrew/../../../evil/merged.json",
            "../evil/Hebrew/merged.json",
            "a/./b/Hebrew/merged.json",
            "/absolute/Hebrew/merged.json",
            "a//b/Hebrew/merged.json",
        ] {
            let joined = root.join(disk_path(name));
            assert!(
                !joined
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir)),
                "{name} -> {}",
                joined.display()
            );
            assert!(
                joined.starts_with(root),
                "{name} left the root: {}",
                joined.display()
            );
        }
    }

    #[test]
    fn the_mapping_is_stable_so_a_resume_finds_what_it_wrote() {
        for name in [
            "schemas/One_People?_Tradition.json",
            "json/Tanakh/Torah/Genesis/Hebrew/merged.json",
            "schemas/CON.json",
        ] {
            assert_eq!(disk_path(name), disk_path(name));
            assert_eq!(
                disk_path(&disk_path(name)),
                disk_path(name),
                "not idempotent"
            );
        }
    }

    #[test]
    fn a_short_file_on_disk_does_not_count_as_landed() {
        let dir = std::env::temp_dir().join("girsa-fetch-test-short");
        let _ = fs::remove_dir_all(&dir);
        let Ok(()) = fs::create_dir_all(&dir) else {
            return;
        };
        let target = Target {
            url: "https://example.invalid/x".into(),
            rel_path: "x.json".into(),
            size: Some(100),
        };
        let Ok(()) = fs::write(dir.join("x.json"), b"truncated") else {
            return;
        };
        assert!(
            !already_complete(&dir, &target),
            "a 9-byte file must not pass for a 100-byte one"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
