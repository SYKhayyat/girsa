//! The command line, read the same way by all sixteen binaries.
//!
//! # What was there instead
//!
//! Sixteen binaries, five conventions, and no shared line of code. The
//! `corpus personal` prefix alone had six answers:
//!
//! | | prefix |
//! |---|---|
//! | `girsa-shelf`, `girsa-read`, `girsa-notes`, `girsa-daf` | defaulted to the literal strings `"corpus"` and `"personal"` |
//! | `girsa-lane`, `girsa-chain`, `girsa-mcp` | **required**, exit 2 if absent |
//! | `girsa-import`, `girsa-link-import` | `<corpus> <otzaria>` |
//! | `girsa-suspects` | `<index> <personal>` |
//! | `girsa-link-orient` | a root defaulting to `"corpus"`, and no personal |
//! | `girsa-index` | the prefix comes **after** the subcommand |
//!
//! `girsa-lane` and `girsa-read` are in one crate, in one directory, and
//! disagree about whether the prefix may be left off.
//!
//! # The three that actually cost something
//!
//! Most of the above is untidiness. Three of them are defects:
//!
//! 1. **`girsa-chain` advertises a syntax it rejects.** Its usage line says
//!    `[--depth N]`; its parser is
//!    `strip_prefix(name)?.strip_prefix('=')?`, so only `--depth=N` works.
//!    Typing what the usage says puts `--depth` in the flags and leaves the
//!    bare `N` among the segment ids, where it is parsed as one and fails with
//!    a message about segment ids.
//! 2. **`girsa-notes`' `split_flags` makes every flag take a value.** A `--x`
//!    unconditionally swallows the following token, so a switch eats a
//!    positional, and `--title=x` is stored under the key `title=x` while
//!    still eating the next word.
//! 3. **`girsa-link-orient` assigns unknown flags to the root path.** Its
//!    parser is `other => root = PathBuf::from(other)`, so `--replce` — a
//!    typo — silently becomes the corpus root, and the run reads a directory
//!    called `--replce`, finds nothing, and reports success over nothing.
//!
//! Each is the same shape: a parser that cannot tell a switch from a value
//! flag, or a flag from a word, because nothing told it which is which.
//!
//! # So this one is told
//!
//! [`Argv::of`] takes the flags that expect a value. That single piece of
//! knowledge is what makes `--depth 5` and `--depth=5` both work, makes a
//! switch stop eating the next word, and makes `--replce` an error rather than
//! a path. It is four characters of extra call site and it is the whole
//! difference.
//!
//! # Why not `clap`
//!
//! Because this is 200 lines and `clap` is a hundred thousand, in a project
//! whose reason for existing is that a reader can hold the whole of it. The
//! bins need positionals, switches, value flags and `--help`; they do not need
//! derive macros, shell completion or coloured help. When they do, this is the
//! one place that would have to change.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// What a binary exits with when it was **invoked** wrongly, as opposed to
/// having run and failed.
///
/// Ten binaries used 2 and four used 1 — and the four that used 1 routed a
/// mistyped verb through the same path as *the shelf will not open*, so a
/// script could not tell a typo from a broken corpus. `girsa-suspects` carries
/// a comment recording that it was changed from 1 to 2 "like the other thirteen
/// binaries here", and the count was wrong when it was written.
pub const WRONG_INVOCATION: u8 = 2;

/// What went wrong with the words, as opposed to with the work.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArgvError {
    #[error("no such option: {0}")]
    NoSuchOption(String),
    #[error("{0} takes a value")]
    WantsAValue(String),
    #[error("{name} takes a number, and {value} is not one")]
    NotANumber { name: String, value: String },
}

/// A command line, split into words and options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Argv {
    words: Vec<String>,
    options: Vec<(String, Option<String>)>,
}

impl Argv {
    /// Read the arguments.
    ///
    /// `switches` are the options that stand alone — `--replace`. `values` are
    /// the options that take one — `--depth`. Both spellings work for the
    /// second: `--depth 5` and `--depth=5`.
    ///
    /// # Errors
    ///
    /// An option in neither list, or a value option at the end with nothing
    /// after it. Both used to be silence, in four different flavours of it.
    pub fn of<I>(args: I, switches: &[&str], values: &[&str]) -> Result<Self, ArgvError>
    where
        I: IntoIterator<Item = String>,
    {
        let mut argv = Self::default();
        let mut args = args.into_iter().peekable();
        while let Some(arg) = args.next() {
            let Some(rest) = arg.strip_prefix("--") else {
                argv.words.push(arg);
                continue;
            };
            // `--` on its own: everything after it is a word, however it looks.
            // A sefer whose slug begins with a dash is not a thing today, and a
            // query word that does is.
            if rest.is_empty() {
                argv.words.extend(args);
                break;
            }
            let (name, inline) = match rest.split_once('=') {
                Some((name, value)) => (name.to_string(), Some(value.to_string())),
                None => (rest.to_string(), None),
            };
            let flag = format!("--{name}");
            if switches.contains(&flag.as_str()) {
                if inline.is_some() {
                    return Err(ArgvError::NoSuchOption(arg));
                }
                argv.options.push((flag, None));
            } else if values.contains(&flag.as_str()) {
                let value = match inline {
                    Some(value) => value,
                    None => args
                        .next()
                        .ok_or_else(|| ArgvError::WantsAValue(flag.clone()))?,
                };
                argv.options.push((flag, Some(value)));
            } else {
                return Err(ArgvError::NoSuchOption(arg));
            }
        }
        Ok(argv)
    }

    /// Straight from the process, minus the program name.
    ///
    /// # Errors
    ///
    /// As [`Argv::of`].
    pub fn read(switches: &[&str], values: &[&str]) -> Result<Self, ArgvError> {
        Self::of(std::env::args().skip(1), switches, values)
    }

    /// Whether the reader asked for the usage.
    ///
    /// Only `girsa-index` understood `-h`. `girsa-shelf --help` set the corpus
    /// root to the string `"--help"` and failed to open a shelf there.
    #[must_use]
    pub fn wants_help(args: &[String]) -> bool {
        args.iter()
            .any(|a| matches!(a.as_str(), "-h" | "--help" | "help"))
    }

    /// The positional words, in order.
    #[must_use]
    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// The nth word.
    #[must_use]
    pub fn word(&self, n: usize) -> Option<&str> {
        self.words.get(n).map(String::as_str)
    }

    /// Everything from the nth word on.
    #[must_use]
    pub fn from(&self, n: usize) -> &[String] {
        self.words.get(n..).unwrap_or(&[])
    }

    /// Whether a switch was given.
    #[must_use]
    pub fn switch(&self, name: &str) -> bool {
        self.options.iter().any(|(flag, _)| flag == name)
    }

    /// The value of an option, or `None` if it was not given.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .rev()
            .find(|(flag, _)| flag == name)
            .and_then(|(_, value)| value.as_deref())
    }

    /// Every value an option was given, for the ones a reader may repeat —
    /// `--tag` on a note.
    #[must_use]
    pub fn every(&self, name: &str) -> Vec<&str> {
        self.options
            .iter()
            .filter(|(flag, _)| flag == name)
            .filter_map(|(_, value)| value.as_deref())
            .collect()
    }

    /// A number, refused rather than defaulted where it will not parse.
    ///
    /// `girsa-suspects` did `get(at + 1)?.parse().ok()`, so `--common banana`
    /// kept the default and said nothing about it.
    ///
    /// # Errors
    ///
    /// If the value is not a number.
    pub fn number<T>(&self, name: &str) -> Result<Option<T>, ArgvError>
    where
        T: std::str::FromStr,
    {
        match self.value(name) {
            None => Ok(None),
            Some(value) => value.parse().map(Some).map_err(|_| ArgvError::NotANumber {
                name: name.to_string(),
                value: value.to_string(),
            }),
        }
    }
}

/// The `corpus personal` prefix, read one way.
///
/// **Defaulted, everywhere.** Four binaries defaulted these to the literal
/// strings and three required them, and defaulting is the one of the two that
/// breaks no invocation anybody is already typing.
///
/// # The default that only worked when nothing was typed
///
/// The first version of this read word 0 and word 1 and defaulted each when it
/// was absent, and every usage line was written to match: `girsa-lane [corpus]
/// [personal] <command>`, brackets and all. A word is absent only when the line
/// ends before it, so the roots defaulted for `girsa-shelf` with no arguments
/// at all, and for nothing else. Every other invocation the usage lines
/// advertise bound the **command** as the corpus root:
///
/// ```text
/// $ girsa-lane state
/// no shelf at state — has the import run?
/// $ girsa-read girsa:bavli/berakhot/2a:1
/// no shelf at girsa:bavli/berakhot/2a:1 — has the import run?
/// ```
///
/// Six binaries, and the error names the mis-bound word every time without
/// anybody reading it as one. The message is even honest — there really is no
/// shelf at `state` — which is why it reads as *the import has not run* rather
/// than *your first word went somewhere you did not mean*.
///
/// So the prefix has to be **recognised** rather than counted, and the only
/// thing that separates `corpus` from `state` is that one of them is a
/// directory and the other is not. Hence [`Roots::of`], which is the single
/// place in this crate that touches the filesystem, and [`Roots::read`], which
/// is the same rule with the question passed in so it can be tested without
/// one.
///
/// It is a heuristic, and the shape of its failure is worth stating plainly: a
/// tool run beside a directory that happens to share a name with one of its own
/// commands binds that command as a root and lands back on exactly the error
/// above. That is a coincidence rather than a certainty, which is the whole of
/// the improvement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    pub corpus: PathBuf,
    pub personal: PathBuf,
    /// Where the words after the prefix start — 0, 1 or 2, depending on how
    /// much of it was typed.
    ///
    /// This was `const AFTER: usize = 2`, which is the number the prefix
    /// occupies when it is written out in full and the wrong number every other
    /// time. A constant cannot be told that a word was not there.
    pub after: usize,
}

impl Roots {
    /// The leading words that name directories, or `corpus` and `personal`
    /// beside the working directory.
    #[must_use]
    pub fn of(argv: &Argv) -> Self {
        Self::read(argv, |word| Path::new(word).is_dir())
    }

    /// The same rule, with *is this a directory* handed in.
    ///
    /// Split out so the rule can be tested against a set of names rather than
    /// against a real filesystem, and so the one impure line in this crate sits
    /// by itself in [`Roots::of`].
    ///
    /// One root may be given without the other: `girsa-daf corpus` is in
    /// `docs/tools.md` and has to keep working, so the prefix is read greedily
    /// up to two words and stops at the first one that is not a directory.
    #[must_use]
    pub fn read(argv: &Argv, is_dir: impl Fn(&str) -> bool) -> Self {
        let mut after = 0;
        while after < 2 && argv.word(after).is_some_and(&is_dir) {
            after += 1;
        }
        Self {
            corpus: PathBuf::from(if after >= 1 {
                argv.word(0).unwrap_or("corpus")
            } else {
                "corpus"
            }),
            personal: PathBuf::from(if after >= 2 {
                argv.word(1).unwrap_or("personal")
            } else {
                "personal"
            }),
            after,
        }
    }
}

/// Print the usage and refuse, with the code that means *you typed it wrong*.
///
/// One function, because `usage` was three functions with three return types:
/// `-> ExitCode` returning `from(2)`, `-> ()` with the code written out at each
/// call site, and an inline `eprintln!` in an `else` block.
#[must_use]
pub fn refuse(usage: &str) -> ExitCode {
    eprintln!("{}", usage.trim_end());
    ExitCode::from(WRONG_INVOCATION)
}

/// Print the usage because it was asked for, which is not a refusal.
#[must_use]
pub fn asked(usage: &str) -> ExitCode {
    println!("{}", usage.trim_end());
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn argv(line: &str, switches: &[&str], values: &[&str]) -> Result<Argv, ArgvError> {
        Argv::of(
            line.split_whitespace().map(ToString::to_string),
            switches,
            values,
        )
    }

    #[test]
    fn a_value_option_takes_either_spelling() {
        // `girsa-chain`'s usage line says `[--depth N]` and its parser accepts
        // only `--depth=N`. Typing what the usage says left a bare `5` among
        // the segment ids, which then failed with a message about segment ids.
        for line in ["forward id --depth 5", "forward id --depth=5"] {
            let argv = argv(line, &[], &["--depth"]).expect("it reads");
            assert_eq!(argv.number::<usize>("--depth").unwrap(), Some(5), "{line}");
            assert_eq!(argv.words(), ["forward", "id"], "{line}");
        }
    }

    #[test]
    fn a_switch_does_not_eat_the_next_word() {
        // `girsa-notes`' `split_flags` made every `--x` swallow the token after
        // it, so a switch ate a positional and the command it belonged to was
        // told the wrong thing about what it had been asked to do.
        let argv = argv("--replace corpus", &["--replace"], &[]).expect("it reads");
        assert!(argv.switch("--replace"));
        assert_eq!(argv.words(), ["corpus"]);
    }

    #[test]
    fn an_option_nobody_declared_is_refused_and_not_a_path() {
        // `girsa-link-orient`'s parser was `other => root = PathBuf::from(other)`
        // — so `--replce`, a typo for `--replace`, became the corpus root. The
        // run then read a directory named `--replce`, found no edges, and
        // reported that it had finished.
        assert_eq!(
            argv("--replce corpus", &["--replace"], &[]),
            Err(ArgvError::NoSuchOption("--replce".to_string()))
        );
    }

    #[test]
    fn a_value_option_with_nothing_after_it_says_so() {
        // `girsa-index` turned this into the empty string and searched for it;
        // `girsa-suspects` kept the default silently.
        assert_eq!(
            argv("find --near", &[], &["--near"]),
            Err(ArgvError::WantsAValue("--near".to_string()))
        );
    }

    #[test]
    fn a_number_that_is_not_one_is_refused_rather_than_defaulted() {
        let argv = argv("--common banana", &[], &["--common"]).expect("it reads");
        assert_eq!(
            argv.number::<u64>("--common"),
            Err(ArgvError::NotANumber {
                name: "--common".to_string(),
                value: "banana".to_string(),
            })
        );
    }

    #[test]
    fn an_option_may_be_repeated_where_the_command_means_it_to_be() {
        // `--tag` on a note. `girsa-notes` read these with a `.filter` over its
        // flag list, which worked; nothing else could have.
        let argv = argv("--tag א --tag ב", &[], &["--tag"]).expect("it reads");
        assert_eq!(argv.every("--tag"), ["א", "ב"]);
        // And a single-valued option repeated takes the last, which is what a
        // reader correcting themselves on the command line means.
        assert_eq!(argv.value("--tag"), Some("ב"));
    }

    #[test]
    fn options_may_come_before_or_after_the_words() {
        // Seven binaries silently took a leading `--foo` as the corpus path.
        let before = argv("--replace corpus personal", &["--replace"], &[]).expect("reads");
        let after = argv("corpus personal --replace", &["--replace"], &[]).expect("reads");
        assert_eq!(before, after);
        assert_eq!(before.words(), ["corpus", "personal"]);
    }

    #[test]
    fn a_bare_double_dash_ends_the_options() {
        let argv = argv("find -- --near", &[], &["--near"]).expect("it reads");
        assert_eq!(argv.words(), ["find", "--near"]);
        assert_eq!(argv.value("--near"), None);
    }

    #[test]
    fn a_switch_given_a_value_is_refused_rather_than_ignored() {
        assert_eq!(
            argv("--replace=yes", &["--replace"], &[]),
            Err(ArgvError::NoSuchOption("--replace=yes".to_string()))
        );
    }

    /// The rule with a made-up filesystem: two directories, and everything
    /// else is a word.
    fn typed(line: &str) -> (Roots, Argv) {
        let argv = argv(line, &[], &[]).expect("reads");
        let roots = Roots::read(&argv, |word| {
            matches!(word, "/a" | "/b" | "corpus" | "personal")
        });
        (roots, argv)
    }

    #[test]
    fn the_prefix_defaults_the_way_four_of_the_seven_did() {
        // Defaulted rather than required, because that is the one of the two
        // that breaks nothing anybody is already typing.
        let (roots, _) = typed("");
        assert_eq!(roots.corpus, PathBuf::from("corpus"));
        assert_eq!(roots.personal, PathBuf::from("personal"));
        assert_eq!(roots.after, 0);

        let (roots, given) = typed("/a /b show");
        assert_eq!(roots.corpus, PathBuf::from("/a"));
        assert_eq!(roots.personal, PathBuf::from("/b"));
        assert_eq!(given.from(roots.after), ["show"]);
    }

    #[test]
    fn a_command_is_not_a_root_however_much_it_looks_like_a_word() {
        // The bug this rule exists for. `girsa-lane state` bound `state` as the
        // corpus root and reported `no shelf at state — has the import run?`,
        // and so did every other command-taking binary for every command it
        // takes. Six of them.
        for line in [
            "state",
            "ask מאימתי",
            "list",
            "forward girsa:bavli/berakhot/2a:1",
        ] {
            let (roots, argv) = typed(line);
            assert_eq!(roots.after, 0, "{line}");
            assert_eq!(roots.corpus, PathBuf::from("corpus"), "{line}");
            assert_eq!(roots.personal, PathBuf::from("personal"), "{line}");
            assert_eq!(argv.from(roots.after).len(), argv.words().len(), "{line}");
        }
    }

    #[test]
    fn one_root_may_be_given_without_the_other() {
        // `girsa-daf corpus` and `girsa-chain corpus` are in `docs/tools.md`,
        // so the prefix has to be readable one word at a time. The word after
        // it is not a directory, which is what stops the count.
        let (roots, argv) = typed("/a show");
        assert_eq!(roots.corpus, PathBuf::from("/a"));
        assert_eq!(roots.personal, PathBuf::from("personal"), "still defaulted");
        assert_eq!(roots.after, 1);
        assert_eq!(argv.from(roots.after), ["show"]);
    }

    #[test]
    fn the_prefix_stops_at_two_even_when_the_third_word_is_a_directory() {
        // A greedy rule that did not stop would eat an argument that happens to
        // name a directory — `girsa-notes corpus personal write corpus` is
        // contrived, but the count is not allowed to depend on what comes after
        // the prefix at all.
        let (roots, argv) = typed("/a /b corpus");
        assert_eq!(roots.after, 2);
        assert_eq!(argv.from(roots.after), ["corpus"]);
    }

    #[test]
    fn help_is_asked_for_the_three_ways_a_reader_asks_for_it() {
        for asking in ["-h", "--help", "help"] {
            assert!(Argv::wants_help(&[asking.to_string()]), "{asking}");
        }
        assert!(!Argv::wants_help(&["show".to_string()]));
    }
}
