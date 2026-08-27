//! The OCR engine — which one, and the afternoon that decided it.
//!
//! spec.md §17 left this open: *Hebrew OCR on old print is genuinely hard and
//! Tesseract is mediocre at it. An afternoon of evaluation decides whether
//! "optional OCR" is a good feature or a disappointing one.* This module is the
//! answer, with the numbers, because the numbers are what make the rest of W26
//! shaped the way it is.
//!
//! # What was measured
//!
//! Five pages of a real sefer on this shelf — a Berachos with the mishnah set
//! in square script with full nikud and the commentary underneath it in **Rashi
//! script** — rendered at 300 dpi and given to tesseract 5.4.0 with the
//! `tessdata_best` Hebrew model. The file carries its own text layer, so every
//! word on every page has a known right answer to score against, which is a
//! luxury this evaluation had and a Vilna Shas would not.
//!
//! Scored the way search cares about: of the distinct words on the page, after
//! nikud and final forms are normalized away, how many did the engine also
//! produce (recall) — and of what it produced, how much is really there
//! (precision).
//!
//! | page | what is on it | recall | precision |
//! |---|---|---|---|
//! | 151 | square script, unvocalized | **99%** | **99%** |
//! | 301 | square script, unvocalized, heavy abbreviation | 83% | 76% |
//! | 7 | square + nikud, Rashi script, footnote figures | **27%** | **23%** |
//! | 8 | the same | 28% | 23% |
//! | 51 | the same | 18% | 15% |
//! | | **all five** | 50% | 44% |
//!
//! # What that decided
//!
//! **Three things, and none of them is "pick a better engine".**
//!
//! **1 · The precision number is the one that matters.** On the Rashi-script
//! pages tesseract produced roughly four words that are not on the page for
//! every one that is. A word that is not there is not a gap in the index — it
//! is a **hit that does not exist**, and a reader sent to a page that does not
//! contain what they searched for has been lied to by the search box. That is
//! BUILDER.md rule 6 in the one place a reader cannot check without reading the
//! whole daf. It is why spec.md §9.7's badge is not a nicety.
//!
//! **2 · You cannot threshold your way out of it.** The obvious repair is to
//! throw away the low-confidence words. It does not work, because tesseract is
//! *confidently* wrong on a script it does not know — on page 7, raising the
//! floor from 0 to 90 costs three quarters of the recall and buys fifteen
//! points of precision:
//!
//! | least confidence kept | recall | precision |
//! |---|---|---|
//! | 0 | 27% | 23% |
//! | 50 | 18% | 25% |
//! | 70 | 11% | 25% |
//! | 90 | 7% | 38% |
//!
//! So there is no knob shipped. The reading goes in as the engine gave it, with
//! its confidence recorded per word for the repair screen, and the honest
//! signal to the reader is the badge and the photograph beside it.
//!
//! **3 · The engine that works is the one that does not run.** A PDF that was
//! typeset rather than photographed carries its own text, and asking it is
//! exact, instant, needs no model and cannot invent a word — the 831 words this
//! evaluation scored *against* came out of it. So the default for any PDF is
//! [`crate::reading::Reader::Embedded`], and OCR is what happens to the pages
//! that have no text layer.
//!
//! # And why tesseract is still wired in
//!
//! Because 27% of a page that was 0% searchable is worth having when the reader
//! is told what it is, and because the decision is **reversible by
//! construction**: a reading is an overlay in the personal layer, corrections
//! anchor to the ink rather than to the words, and re-reading a scan with
//! something better is one pass with nothing to migrate
//! (`tests/the_image_is_ground_truth.rs`). That is what makes it safe to ship
//! the best engine available today instead of waiting for a good one.
//!
//! It is **found, not bundled and not fetched**. Nothing here downloads a
//! model — spec.md §14, offline is the product, and BUILDER.md §0.1 makes a
//! runtime network dependency a decision that is not mine to take. If tesseract
//! is on the PATH with a Hebrew model, Girsa uses it; if it is not, the window
//! says so, which is a state with a name rather than a button that does
//! nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::reading::{Area, Read, Reader, Word};

/// A page of a scan, rendered. PNG bytes, and how big the raster is.
///
/// The window renders it, out of the same pdf.js that draws the page for
/// reading — a second PDF renderer in the Rust half would be a second opinion
/// about what a page looks like, and this crate has never opened a PDF.
#[derive(Debug, Clone)]
pub struct Image {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Why a page could not be read.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// No engine is installed. **A state, not a failure** — spec.md §6.3 says
    /// OCR is optional, and this is what optional looks like from the inside.
    #[error(
        "no OCR engine is installed — Girsa uses tesseract with a Hebrew model when one is on \
         the PATH, and reads a PDF's own text layer when it has one"
    )]
    NoEngine,
    #[error("{engine} would not run: {source}")]
    WouldNotRun {
        engine: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{engine} failed on page {page}: {message}")]
    Failed {
        engine: String,
        page: usize,
        message: String,
    },
}

/// Something that can look at a picture of a page and say what is on it.
///
/// The PDF's own text layer is deliberately **not** one of these. It is not
/// looking at a picture — it is being told, by the file, and it needs no image,
/// no model and no process. It arrives through [`crate::reading::group`]
/// instead, and the two are told apart everywhere downstream by
/// [`crate::reading::Reader`].
pub trait Engine {
    /// The name that goes in the reading and on the badge — with its version,
    /// because a page read by tesseract 5.4 and one read by whatever replaces
    /// it are not the same claim.
    fn name(&self) -> String;

    /// Read one page.
    ///
    /// # Errors
    ///
    /// If the engine will not start or will not answer. A page it reads and
    /// finds nothing on is **not** an error — see [`crate::words`].
    fn read(&self, page: usize, image: &Image) -> Result<Read, EngineError>;
}

/// Tesseract, if it is installed.
#[derive(Debug, Clone)]
pub struct Tesseract {
    binary: PathBuf,
    version: String,
    /// Where the Hebrew model is, when it is not where tesseract keeps its own.
    models: Option<PathBuf>,
}

/// Where tesseract puts itself on Windows when nobody adds it to the PATH.
///
/// Looked in because the alternative is a reader who has installed tesseract
/// being told Girsa cannot find it. Only these, and only for the one binary —
/// nothing here searches the disk.
const USUAL_PLACES: [&str; 3] = [
    r"C:\Program Files\Tesseract-OCR\tesseract.exe",
    r"/usr/bin/tesseract",
    r"/opt/homebrew/bin/tesseract",
];

impl Tesseract {
    /// Find it, and check it can read Hebrew.
    ///
    /// Both halves matter: tesseract with no Hebrew model installed runs
    /// perfectly and returns nothing at all, which on a page of a scan is
    /// indistinguishable from a page with nothing on it.
    ///
    /// # Where the Hebrew model is looked for
    ///
    /// Tesseract's own `tessdata` directory, and **`<personal>/tessdata`**. The
    /// second is not a convenience: tesseract installs into a place the reader
    /// very often cannot write — on Windows that is `C:\Program Files` and it
    /// takes an administrator — while the Hebrew model is a separate download
    /// that does not come with it. Without somewhere of their own to put it,
    /// the answer to *why can Girsa not read my scan* would be an elevation
    /// prompt.
    ///
    /// Nothing here downloads it. spec.md §14 and BUILDER.md §0.1: offline is
    /// the product, and a runtime network dependency is not a decision this
    /// crate takes.
    #[must_use]
    pub fn found(personal: Option<&Path>) -> Option<Self> {
        let models = personal
            .map(|root| root.join("tessdata"))
            .filter(|dir| dir.join(format!("{HEBREW}.traineddata")).is_file());
        std::iter::once(PathBuf::from("tesseract"))
            .chain(USUAL_PLACES.iter().map(PathBuf::from))
            .find_map(|binary| {
                // Both probes are bounded. They are plain synchronous spawns
                // into whatever that path turns out to hold, and a binary
                // that hangs — a dead mount, a broken install — used to hang
                // the caller with it, once per candidate.
                let said = run_bounded(
                    {
                        let mut version = Command::new(&binary);
                        version.arg("--version");
                        version
                    },
                    PROBE_TIMEOUT,
                )?;
                if !said.status.success() {
                    return None;
                }
                let mut listing = Command::new(&binary);
                listing.arg("--list-langs");
                if let Some(dir) = &models {
                    listing.arg("--tessdata-dir").arg(dir);
                }
                let langs = run_bounded(listing, PROBE_TIMEOUT)?;
                if !String::from_utf8_lossy(&langs.stdout)
                    .lines()
                    .any(|line| line.trim() == HEBREW)
                {
                    return None;
                }
                let version = String::from_utf8_lossy(&said.stdout)
                    .lines()
                    .next()
                    .unwrap_or("tesseract")
                    .trim()
                    .to_string();
                Some(Self {
                    binary,
                    version,
                    models: models.clone(),
                })
            })
    }

    /// Where a reader puts a model they downloaded themselves.
    #[must_use]
    pub fn models_in(personal: &Path) -> PathBuf {
        personal.join("tessdata")
    }
}

/// The model, named once. `heb` is the language code, and a build without it
/// installed is a build that cannot do this at all.
const HEBREW: &str = "heb";

/// How long a candidate binary gets to answer `--version` or `--list-langs`.
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Run a probe, and give up if it does not come back in time.
///
/// The outputs are a version line and a language list — far under one pipe
/// buffer — so draining after exit cannot deadlock, and the bounded wait costs
/// one thread sleeping in 10 ms steps for the life of the probe.
fn run_bounded(
    mut command: std::process::Command,
    within: std::time::Duration,
) -> Option<std::process::Output> {
    use std::io::Read;
    let started = std::time::Instant::now();
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(pipe) = child.stdout.as_mut() {
                    let _ = pipe.read_to_end(&mut stdout);
                }
                if let Some(pipe) = child.stderr.as_mut() {
                    let _ = pipe.read_to_end(&mut stderr);
                }
                return Some(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            // Still running, still inside its budget.
            Ok(None) if started.elapsed() < within => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            _ => {
                // Would not answer in time, or would not be waited on.
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

impl Engine for Tesseract {
    fn name(&self) -> String {
        self.version.clone()
    }

    fn read(&self, page: usize, image: &Image) -> Result<Read, EngineError> {
        // Unique per read, not per page: two reads of one page in flight —
        // the window's job and a batch job, say — used to share one name and
        // each write the other's image.
        static READS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let scratch = std::env::temp_dir().join(format!(
            "girsa-ocr-{page}-{}-{}.png",
            std::process::id(),
            READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::write(&scratch, &image.png).map_err(|source| EngineError::WouldNotRun {
            engine: self.version.clone(),
            source,
        })?;

        let mut run = Command::new(&self.binary);
        if let Some(dir) = &self.models {
            run.arg("--tessdata-dir").arg(dir);
        }
        let out = run
            .arg(&scratch)
            .arg("stdout")
            .args(["-l", HEBREW])
            // A page of a sefer is one block of text in one language. Left to
            // itself tesseract also runs layout analysis, which on a page with
            // a commentary under a rule finds two documents and interleaves
            // them — and the order of the words on a page is what spec.md
            // §9.3's *within X words of each other* is asked about.
            .args(["--psm", "6"])
            // Boxes and per-word confidence. Plain text would give neither, and
            // then a hit could not be highlighted on the scan, which is the
            // whole of spec.md §6.3's *OCR text anchors to coordinates on the
            // page image*.
            .args(["-c", "tessedit_create_tsv=1"])
            .output()
            .map_err(|source| EngineError::WouldNotRun {
                engine: self.version.clone(),
                source,
            });
        let _ = std::fs::remove_file(&scratch);
        let out = out?;

        if !out.status.success() {
            return Err(EngineError::Failed {
                engine: self.version.clone(),
                page,
                message: String::from_utf8_lossy(&out.stderr).trim().to_string(),
            });
        }

        Ok(Read::new(
            page,
            Reader::Ocr {
                engine: self.version.clone(),
            },
            from_tsv(
                &String::from_utf8_lossy(&out.stdout),
                image.width,
                image.height,
            ),
        ))
    }
}

/// Tesseract's TSV, as words on a page.
///
/// Boxes come out in pixels of the raster and go in as fractions of the page,
/// here and once — a highlight stored in pixels of somebody's 300-dpi render
/// lands in the margin the first time the reader zooms.
fn from_tsv(tsv: &str, width: u32, height: u32) -> Vec<Word> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    #[allow(clippy::cast_precision_loss)]
    let (width, height) = (width as f32, height as f32);
    let mut words = Vec::new();
    for line in tsv.lines().skip(1) {
        let field: Vec<&str> = line.split('\t').collect();
        // level·page·block·par·line·word·left·top·width·height·conf·text
        let [.., left, top, box_width, box_height, confidence, text] = field.as_slice() else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        let (Ok(left), Ok(top), Ok(box_width), Ok(box_height), Ok(confidence)) = (
            left.parse::<f32>(),
            top.parse::<f32>(),
            box_width.parse::<f32>(),
            box_height.parse::<f32>(),
            confidence.parse::<f32>(),
        ) else {
            continue;
        };
        words.push(Word {
            text: text.trim().to_string(),
            at: Area::new(
                left / width,
                top / height,
                (left + box_width) / width,
                (top + box_height) / height,
            ),
            confidence: (confidence / 100.0).clamp(0.0, 1.0),
        });
    }
    words
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// Two words of a real TSV run, with the header tesseract prints and the
    /// structural rows it prints alongside the words.
    const REAL: &str = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
1\t1\t0\t0\t0\t0\t0\t0\t1801\t2700\t-1\t
4\t1\t1\t1\t1\t0\t796\t265\t209\t37\t-1\t
5\t1\t1\t1\t1\t1\t930\t265\t75\t36\t96.636688\tפרק
5\t1\t1\t1\t1\t2\t796\t265\t112\t37\t91.788208\tראשון";

    #[test]
    fn the_structural_rows_are_not_words_and_the_boxes_are_fractions_of_the_page() {
        let words = from_tsv(REAL, 1801, 2700);
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["פרק", "ראשון"]
        );
        let first = words[0].at;
        assert!((first.left - 930.0 / 1801.0).abs() < 1e-6, "{first:?}");
        assert!((first.right - 1005.0 / 1801.0).abs() < 1e-6, "{first:?}");
        assert!(first.right <= 1.0 && first.bottom <= 1.0, "{first:?}");
        assert!((words[0].confidence - 0.966_366_9).abs() < 1e-5);
    }

    #[test]
    fn a_reading_of_an_empty_raster_is_empty_rather_than_a_division_by_zero() {
        assert!(from_tsv(REAL, 0, 2700).is_empty());
        assert!(from_tsv(REAL, 1801, 0).is_empty());
    }

    #[test]
    fn no_engine_is_a_sentence_and_not_a_silence() {
        // Whether tesseract is installed on the machine running the tests is
        // not this crate's business. That the absence has words is.
        let message = EngineError::NoEngine.to_string();
        assert!(
            message.contains("optional") || message.contains("no OCR engine"),
            "{message}"
        );
        assert!(message.contains("text layer"), "{message}");
    }

    #[test]
    fn a_probe_that_hangs_is_given_up_on_before_the_deadline() {
        // Minor 21: `--version` and `--list-langs` were probed synchronously on
        // up to four candidate binaries, and a binary that hung — a dead mount,
        // a broken install — hung the caller with it, once per candidate. The
        // probe is bounded; pin that a child that would run forever is walked
        // away from well before the caller's patience runs out.
        //
        // `cmd /c` (Windows) and `/bin/sh` (everywhere else) both sleep in
        // seconds, and the deadline here is much shorter than a second.
        let started = std::time::Instant::now();
        let mut hang = std::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" });
        if cfg!(windows) {
            hang.args(["/c", "ping", "-n", "60", "127.0.0.1"]);
        } else {
            hang.args(["-c", "sleep 60"]);
        }
        // A third of the real probe timeout: the bound is what is under test,
        // not the exact seconds. Something that would hang for a minute must
        // come back in a couple of tenths.
        let within = PROBE_TIMEOUT / 3;
        let came_back = run_bounded(hang, within);
        let elapsed = started.elapsed();
        assert!(came_back.is_none(), "a hung probe must not produce output");
        // The probe is given a few wake-ups at 10 ms steps; anything near the
        // deadline means the child was waited on to the end rather than walked
        // away from.
        assert!(
            elapsed < within * 3,
            "gave up after {elapsed:?}, not the ~{within:?} bound"
        );
    }
}
