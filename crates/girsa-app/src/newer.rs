//! Whether there is a newer Girsa, asked only when somebody asks.
//!
//! # Why this is a button and not a background check
//!
//! spec.md §14 is the shortest rule in the document: **offline is the
//! product**. Corpus updates are the one sanctioned network use, and
//! `BUILDER.md` §0.1 lists *adding a network dependency at runtime* among the
//! things not to decide alone.
//!
//! An updater that phones home on start is that dependency, and it is one the
//! reader never asked for. So nothing here runs unless a person presses a
//! button. A window that has never been asked makes no requests, has no
//! background timer and needs no setting to turn off — which is a stronger
//! promise than *the setting defaults to off*.
//!
//! Otzaria checks GitHub on start and offers to download. This checks GitHub
//! when asked and says what it found, which is the same information one gesture
//! later and no traffic a reader did not ask for.
//!
//! # And why it does not install anything
//!
//! Installing a build means verifying a signature, and verifying a signature
//! means a private key that signs releases. That key is a piece of a
//! release process rather than a piece of this repository, and an updater that
//! downloaded and ran an unsigned binary from the internet would be the single
//! worst thing in the application by a distance. So this answers *there is a
//! newer one, here is where it is* and stops.

use std::time::Duration;

/// Where the releases are.
///
/// The repository this is built from, by name and not by a setting: an updater
/// pointed at an address a reader can change is an updater that can be pointed
/// somewhere else, which is a way of getting a person to install something.
const RELEASES: &str = "https://api.github.com/repos/SYKhayyat/girsa/releases/latest";

/// How long to wait before deciding the network is not there.
///
/// Short, because this is a button in a window and the failure — *no answer* —
/// is a perfectly good answer that a reader can act on.
const PATIENCE: Duration = Duration::from_secs(8);

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Newer {
    /// What is running.
    pub running: String,
    /// What the latest release is called, when one could be read.
    pub latest: Option<String>,
    /// Where to get it. Only ever a URL out of the release itself.
    pub at: Option<String>,
    /// Whether the latest is newer than what is running.
    pub newer: bool,
}

/// Why a check could not answer.
#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    #[error("could not reach the releases: {0}")]
    Unreachable(String),
    #[error("the releases answered something that is not a release")]
    Unreadable,
}

/// Ask GitHub what the latest release is.
///
/// # Errors
///
/// If the network is not there, or the answer is not a release. Both are
/// reported rather than swallowed: a check that silently says *you are up to
/// date* when it could not ask is worse than no check.
pub fn check(running: &str) -> Result<Newer, CheckError> {
    let body = ureq::get(RELEASES)
        // GitHub refuses a request with no user agent, with a 403 that reads
        // like a rate limit.
        .header("User-Agent", &format!("girsa/{running}"))
        .header("Accept", "application/vnd.github+json")
        .config()
        .timeout_global(Some(PATIENCE))
        .build()
        .call()
        .map_err(|e| CheckError::Unreachable(e.to_string()))?
        .body_mut()
        .read_to_string()
        .map_err(|e| CheckError::Unreachable(e.to_string()))?;
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|_| CheckError::Unreadable)?;
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or(CheckError::Unreadable)?;
    let at = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(Newer {
        running: running.to_string(),
        newer: is_newer(tag, running),
        latest: Some(tag.to_string()),
        at,
    })
}

/// Whether `tag` names a later version than `running`.
///
/// Compared **number by number** and not as strings: `0.10.0` is later than
/// `0.9.0` and sorts before it, and an updater that got that backwards would
/// tell a reader on the newest build to go and get an older one. A leading `v`
/// is taken off because that is how tags are written and not how versions are.
///
/// Anything that is not three numbers — a tag like `nightly`, a release
/// somebody named — is **not newer**. Refusing to compare is the right answer:
/// the alternative is guessing at an ordering nobody defined.
#[must_use]
pub fn is_newer(tag: &str, running: &str) -> bool {
    let Some(there) = numbers(tag) else {
        return false;
    };
    let Some(here) = numbers(running) else {
        return false;
    };
    there > here
}

/// `v0.1.1` → `[0, 1, 1]`, and `None` for anything that is not three numbers.
fn numbers(version: &str) -> Option<[u32; 3]> {
    let bare = version.trim().trim_start_matches(['v', 'V']);
    let mut parts = bare.split('.');
    let mut out = [0u32; 3];
    for slot in &mut out {
        // A pre-release suffix — `1.2.3-rc1` — is cut at the dash. The numbers
        // in front of it are the version, and `rc1` is a thing this cannot
        // order and does not try to.
        let part = parts.next()?.split(['-', '+']).next()?;
        *slot = part.parse().ok()?;
    }
    parts.next().is_none().then_some(out)
}

/// The releases page, on the machine's own browser.
///
/// # Why a constant and not the URL the check came back with
///
/// This opens **one address, compiled in**. A command that opened whatever URL
/// it was handed is a command that opens whatever a bug hands it, and the whole
/// value of an updater that does not install anything is that there is nothing
/// here worth attacking. The release's own `html_url` is shown to the reader
/// and is not what is opened.
///
/// No plugin: `tauri-plugin-opener` would grant the window the ability to open
/// URLs and then take it back with a scope, which is a larger surface than one
/// function with no argument.
///
/// # Errors
///
/// If the platform's opener is not there — a machine with no browser, or a
/// Linux session with no `xdg-open`, both of which are real.
pub fn open_releases() -> Result<(), std::io::Error> {
    /// Where a person goes to get one, which is not the API address the check
    /// reads.
    const PAGE: &str = "https://github.com/SYKhayyat/girsa/releases/latest";
    let mut how = if cfg!(target_os = "windows") {
        // `start` is a shell builtin, not a program, and its first argument is
        // the window title — hence the empty string, which is the standard
        // incantation and looks like a mistake without this sentence.
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "start", "", PAGE]);
        cmd
    } else if cfg!(target_os = "macos") {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(PAGE);
        cmd
    } else {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(PAGE);
        cmd
    };
    how.spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_compared_number_by_number_and_not_as_a_string() {
        // The one that matters: `0.10.0` sorts *before* `0.9.0` as a string,
        // and an updater that compared strings would tell a reader on the
        // newest build to go and get an older one.
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(!is_newer("0.9.0", "0.10.0"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(is_newer("0.1.2", "0.1.1"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
    }

    #[test]
    fn a_tag_is_written_with_a_v_and_a_version_is_not() {
        assert!(is_newer("v0.2.0", "0.1.1"));
        assert!(!is_newer("v0.1.1", "0.1.1"));
    }

    #[test]
    fn a_tag_that_is_not_three_numbers_is_never_newer() {
        // Refusing to compare rather than guessing at an ordering nobody
        // defined. A `nightly` tag telling every reader they are out of date,
        // for ever, is the failure this prevents.
        assert!(!is_newer("nightly", "0.1.1"));
        assert!(!is_newer("", "0.1.1"));
        assert!(!is_newer("0.2", "0.1.1"));
        assert!(!is_newer("0.2.0.1", "0.1.1"));
        assert!(!is_newer("0.2.0", "not-a-version"));
    }

    #[test]
    fn a_pre_release_is_ordered_on_its_numbers_and_not_on_its_name() {
        // `1.0.0-rc1` is the numbers `1.0.0`, which is genuinely newer than
        // `0.9.0`. Whether it is newer than `1.0.0` itself is a question this
        // does not answer and says so by answering no.
        assert!(is_newer("1.0.0-rc1", "0.9.0"));
        assert!(!is_newer("1.0.0-rc1", "1.0.0"));
    }
}
