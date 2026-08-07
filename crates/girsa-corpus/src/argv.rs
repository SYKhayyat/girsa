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

use std::path::PathBuf;
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    pub corpus: PathBuf,
    pub personal: PathBuf,
}

impl Roots {
    /// The first two words, or `corpus` and `personal` beside the working
    /// directory.
    #[must_use]
    pub fn of(argv: &Argv) -> Self {
        Self {
            corpus: PathBuf::from(argv.word(0).unwrap_or("corpus")),
            personal: PathBuf::from(argv.word(1).unwrap_or("personal")),
        }
    }

    /// Where the words after the prefix start.
    pub const AFTER: usize = 2;
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

    #[test]
    fn the_prefix_defaults_the_way_four_of_the_seven_did() {
        // Defaulted rather than required, because that is the one of the two
        // that breaks nothing anybody is already typing.
        let bare = argv("", &[], &[]).expect("reads");
        let roots = Roots::of(&bare);
        assert_eq!(roots.corpus, PathBuf::from("corpus"));
        assert_eq!(roots.personal, PathBuf::from("personal"));

        let given = argv("/a /b show", &[], &[]).expect("reads");
        let roots = Roots::of(&given);
        assert_eq!(roots.corpus, PathBuf::from("/a"));
        assert_eq!(roots.personal, PathBuf::from("/b"));
        assert_eq!(given.from(Roots::AFTER), ["show"]);
    }

    #[test]
    fn help_is_asked_for_the_three_ways_a_reader_asks_for_it() {
        for asking in ["-h", "--help", "help"] {
            assert!(Argv::wants_help(&[asking.to_string()]), "{asking}");
        }
        assert!(!Argv::wants_help(&["show".to_string()]));
    }
}
