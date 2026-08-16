//! What the app remembers between one evening and the next.
//!
//! Two different kinds of memory, and they are kept apart on purpose:
//!
//! - the **workspace** — which tabs are open and how they are arranged;
//! - **where you were in each sefer**, for every sefer you have ever opened,
//!   whether or not it is open now. BUILDER.md W9 asks for per-sefer position
//!   memory, and the point of it is the sefer you closed three weeks ago.
//!
//! Written as one JSON file, local, no account (spec.md §11). It is a
//! preference file, not the corpus: losing it costs a layout, and the same rule
//! applies as everywhere else here — text files are the truth and this is not
//! one of them, so nothing in it is allowed to be the only copy of anything.

use std::collections::BTreeMap;
use std::path::Path;

use girsa_corpus::segment::SegmentId;
use serde::{Deserialize, Serialize};

use crate::workspace::Workspace;

/// Everything the app remembers.
///
/// [`Default`] is written out rather than derived, so that every field's
/// default is a decision somebody made and can be read in one place. It matters
/// most for the ones a corrupt preferences file would otherwise fall back
/// through: a session that will not parse gives a fresh one, and a fresh one
/// that stripped the nikud out of every sefer on the shelf would look like a
/// rendering bug rather than a lost file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub workspace: Workspace,
    /// The folder of seforim the reader pointed the window at, if they did.
    ///
    /// The one thing in this file that is about where the *corpus* is rather
    /// than about what the reader was doing in it, and it is here for the
    /// reason the corpus cannot hold it: a reader with no corpus has nowhere
    /// else to write. Ranked by `girsa_corpus::roots::candidates`, which is
    /// where the order between this and `GIRSA_CORPUS` is argued.
    ///
    /// It is not a guarantee. A corpus can be moved or deleted after it was
    /// chosen, so this is a candidate like every other candidate — the marker
    /// file decides, never the fact that somebody once said so.
    #[serde(default)]
    pub corpus: Option<std::path::PathBuf>,
    /// Sefer → the segment you were last looking at in it.
    #[serde(default)]
    pub positions: BTreeMap<String, SegmentId>,
    /// Sefer → the mefarshim you ticked on it (W43).
    ///
    /// A `Vec` and not a set, because the order is the reader's: somebody who
    /// ticked the Rosh before Rashi meant it, and an alphabetical answer would
    /// quietly reorder their own list every time they opened the daf.
    ///
    /// Per sefer, and that is the point — the mefarshim you follow on Berakhot
    /// are not the ones you follow on Chullin.
    #[serde(default)]
    pub chosen: BTreeMap<String, Vec<String>>,
    /// Sefer → seforim the **reader** says run alongside it.
    ///
    /// # Why the corpus is not the last word on this
    ///
    /// `taxonomy::Keeping` settles *parallel seforim* from what the graph shows:
    /// twenty-five or more simanim joined to the siman of the same number. It is
    /// exact, and it is only as good as the links — and there is a case that
    /// proves it. **The Shulchan Arukh HaRav is written on Orach Chayim's
    /// simanim**, and of the 505 of its simanim the graph joins to Orach Chayim,
    /// two land on their own number: Sefaria's links between those two are
    /// citations, not the structural mapping. So the corpus does not say they
    /// run alongside each other, and nothing in this application will say it on
    /// the corpus's behalf.
    ///
    /// The reader knows. *"The user can add"* is his answer to exactly this, and
    /// it is the right one: a claim about two seforim that a person made is a
    /// better thing to keep than a claim this code inferred.
    ///
    /// # Read both ways
    ///
    /// Stored once, under whichever sefer he was standing on. Parallel is
    /// **symmetric** — if the Shulchan Arukh HaRav runs alongside Orach Chayim
    /// then Orach Chayim runs alongside it — so [`Session::alongside_of`] reads
    /// the map in both directions and the reader never has to say it twice.
    #[serde(default)]
    pub alongside: BTreeMap<String, Vec<String>>,
    /// How much of the pointing is shown. One setting for the window, because a
    /// reader who turns it off wants it off — not off in this pane and on in the
    /// one beside it.
    #[serde(default)]
    pub pointing: Pointing,
    /// Whether the shemos are written as the corpus has them, or with a letter
    /// changed so a printout may be discarded (`crate::shemos`).
    ///
    /// One setting for the window, for the same reason the pointing is: a
    /// reader who turned this on wants a page they can throw away, and a
    /// second pane that did not get the message is a page they cannot.
    #[serde(default)]
    pub shemos: crate::shemos::Shemos,
    /// A session written when this was a bool, read once and folded into
    /// [`Session::pointing`] by [`Session::sane`].
    ///
    /// The alternative is that a reader who had turned the nikud off finds it
    /// back on after an update, with nothing saying why — and there is a whole
    /// decision record in the sibling repository called *no preference ever
    /// survived a reload*. Never written out: the file says `pointing` from the
    /// first save onwards.
    #[serde(default, rename = "nikud", skip_serializing)]
    was_nikud: Option<bool>,
    /// Reading size, as a percentage. Hebrew with nikud at a small size is
    /// unreadable in a way Latin text at the same size is not.
    #[serde(default = "hundred")]
    pub text_size: u16,
    /// How a citation is printed when a source is sent (spec.md §10.2, W15).
    ///
    /// A preference and not a fact about the quote: what the document stores
    /// is the ref, so changing this changes how every citation *prints* and
    /// nothing about where any of them point.
    #[serde(default = "full")]
    pub cite: girsa_cite::CiteStyle,
    /// Which language the window is in (W41).
    ///
    /// > *"hebrew and english ui. all seforim names in hebrew ui should be heb
    /// > all in english ui should be english."*
    ///
    /// A setting and not a guess from the machine's locale: a reader in New York
    /// whose Windows is in English still wants ברכות to be called ברכות, and the
    /// one who wants Berakhot wants it everywhere at once.
    ///
    /// **Only the seforim.** What the buttons say is [`Session::interface`], and
    /// they are two settings because they are two questions: *"there is no way
    /// to change UI into english — only seforim names. there should be 2
    /// seperate commands."* A reader who learns in Hebrew and reads an English
    /// interface is ordinary, and so is the reverse.
    #[serde(default)]
    pub language: Language,
    /// What the window itself says — buttons, headings, panel titles, the
    /// sentences it says back.
    ///
    /// Defaults to Hebrew, like the seforim, because that is what shipped and a
    /// reader who never opens the settings must not find the application in a
    /// language they did not ask for.
    #[serde(default)]
    pub interface: Language,
    /// How the reading looks: theme, fonts, line height, column width (B13).
    ///
    /// > *"There is no settings panel. No font. No theme control … Otzar
    /// > HaChochma and Bar Ilan both ship deep display and search preferences.
    /// > **This is a step backwards from what you are replacing.**"*
    #[serde(default)]
    pub look: Look,
    /// Keys the reader has rebound (B13). Action id → the combination.
    ///
    /// Only the ones they changed. A full table would mean a reader's file
    /// disagreeing with a later version of the app about what the *unchanged*
    /// keys are, and then a shortcut moving for a reason nobody could see.
    #[serde(default)]
    pub keys: BTreeMap<String, String>,
    /// How much of your correction layer is applied to what you read (W20).
    ///
    /// Remembered like the nikud toggle, and for the same reason: a reader who
    /// turned the corrections off to check what was printed wants them off
    /// until they say otherwise.
    #[serde(default)]
    pub showing: girsa_fix::Showing,
    /// Where the last file was written, so the next one is offered there.
    ///
    /// > *"send to ksav and export dont let you pick a folder."*
    ///
    /// They did not: an export went to `personal/exports/` and said the path
    /// afterwards. This is what a folder dialog opens on the second time, and
    /// `None` is *nobody has chosen one yet*, which is the reader's own
    /// documents folder rather than a directory inside the application's data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_into: Option<String>,
}

/// How much of the pointing is drawn.
///
/// Three, and the middle one is the reader's: *"im nikkud and bli nikkud are
/// backwards. Also, there is no way to have nikud and no trup."* A bool could
/// hold two of these and the one that was missing is the one most people
/// actually read a Chumash in — the nikud without the te'amim, because the trup
/// is for laining and on a screen it is a second storey of marks over every
/// word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Pointing {
    /// Everything the corpus has: nikud and te'amim both.
    #[default]
    Full,
    /// The nikud, without the te'amim.
    Nikud,
    /// The letters alone.
    Plain,
}

girsa_corpus::spelled!(Pointing {
    Full => "full",
    Nikud => "nikud",
    Plain => "plain",
});

impl Pointing {
    /// Every setting, in the order the control rounds them.
    pub const ALL: [Self; 3] = [Self::Full, Self::Nikud, Self::Plain];

    /// The next one round, for a control that cycles.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Full => Self::Nikud,
            Self::Nikud => Self::Plain,
            Self::Plain => Self::Full,
        }
    }

    /// Whether a mark is drawn under this setting.
    ///
    /// **The one predicate.** `display::Shown`, the export, the copy and the
    /// correction box all ask it, and they have to agree to the character or a
    /// highlight lands on the wrong letters (`girsa_app::display`'s whole
    /// reason for existing).
    #[must_use]
    pub fn draws(self, ch: char) -> bool {
        match self {
            Self::Full => true,
            // The te'amim only. `girsa_hebrew::is_mark` cannot make this
            // distinction — it is one range, and it is the right answer for the
            // *index*, where nobody types either. On the page they are two
            // different things to a reader.
            Self::Nikud => !is_taam(ch),
            Self::Plain => !girsa_hebrew::is_mark(ch),
        }
    }

    /// What the control says it will do **next**, which is what a button is for.
    ///
    /// The toolbar printed the state it was already in — `עם ניקוד` while the
    /// nikud was on — beside a language button whose own comment says *"a button
    /// labelled with the state you are already in is a button nobody can
    /// predict."* Two buttons, two conventions, one toolbar.
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Full => "ניקוד וטעמים",
            Self::Nikud => "ניקוד בלי טעמים",
            Self::Plain => "בלי ניקוד",
        }
    }

    /// The same, for a window running in English.
    #[must_use]
    pub const fn said_en(self) -> &'static str {
        match self {
            Self::Full => "nikud and te'amim",
            Self::Nikud => "nikud, no te'amim",
            Self::Plain => "no nikud",
        }
    }
}

/// Whether a mark is a **taam** — cantillation — rather than a nikud point.
///
/// The accents are their own block, `U+0591`–`U+05AF`; meteg and the two dots
/// are counted with them because they are set by the same hand and read by the
/// same eye. Everything else in the mark range — sheva through dagesh, rafe, the
/// shin and sin dots, qamats qatan — is nikud and stays.
///
/// Here and not in `girsa-hebrew`: that crate's `is_mark` is the **index's**
/// question (spec.md §9.1), one range, deliberately blunt, and shared with Ksav
/// through a pinned release. This is a question about a page.
#[must_use]
pub fn is_taam(ch: char) -> bool {
    matches!(
        ch,
        '\u{0591}'..='\u{05AF}' | '\u{05BD}' | '\u{05C4}' | '\u{05C5}'
    )
}

/// Which language the window speaks.
///
/// Two, and no `Auto`. A window that picked for itself would be a window whose
/// language changes when a reader travels, and the corpus has seforim whose
/// English title is a transliteration nobody says out loud — so which one is
/// wanted is a decision, not a fact about the machine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Hebrew,
    English,
}

girsa_corpus::spelled!(Language {
    Hebrew => "hebrew",
    English => "english",
});

impl Language {
    /// Which of a sefer's two titles to print.
    ///
    /// Falls back to the other when the one asked for is empty, because a row
    /// with no name on it is worse than a row named in the wrong language — and
    /// the corpus does have works with one title and not the other.
    #[must_use]
    pub fn title_of<'a>(self, he: &'a str, en: &'a str) -> &'a str {
        let (first, second) = match self {
            Self::Hebrew => (he, en),
            Self::English => (en, he),
        };
        if first.trim().is_empty() {
            second
        } else {
            first
        }
    }

    /// What the document's `lang` and `dir` should be.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Hebrew => "he",
            Self::English => "en",
        }
    }

    #[must_use]
    pub const fn rtl(self) -> bool {
        matches!(self, Self::Hebrew)
    }
}

/// Which theme, said out loud.
///
/// Three and not two. `styles.css` set `color-scheme: dark` with a
/// `prefers-color-scheme: light` override, so the operating system decided and a
/// reader who wanted the other one could not have it. *Follow the system* is a
/// perfectly good answer and it is now one of the three rather than the only one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

girsa_corpus::spelled!(Theme {
    System => "system",
    Light => "light",
    Dark => "dark",
});

/// How the reading looks (B13).
///
/// Everything here is a fact about the reader's eyes and the room they are in,
/// which is why it is one struct: a panel that sets six of these and a session
/// that stores them in six places is how one of them ends up unsaved.
///
/// The **two** font families are the point. A daf is Hebrew with an English
/// footnote, or an English translation beside a Hebrew source, and one font
/// setting for both means choosing which of the two languages reads badly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Look {
    #[serde(default)]
    pub theme: Theme,
    /// The family for Hebrew. Empty means the stylesheet's own stack, which is
    /// what a reader who has never opened the panel gets.
    #[serde(default)]
    pub hebrew_font: String,
    /// The family for Latin text — the interface, and English inside a sefer.
    #[serde(default)]
    pub latin_font: String,
    /// Line height as a multiple. Hebrew with nikud is two storeys tall and the
    /// leading that looks smart in Latin type makes a menukad Gemara unreadable,
    /// so the default is generous and the reader can still tighten it.
    #[serde(default = "leading")]
    pub line_height: u16,
    /// How wide a column of text may get, in characters. A pane maximised on a
    /// 27-inch monitor is a line of ninety words, which nobody can read.
    #[serde(default = "measure")]
    pub column_ch: u16,
}

/// The families a reader can choose between, in the order the panel offers
/// them.
///
/// Named rather than enumerated from the system: a webview cannot list
/// installed fonts, and a list this project invented would offer families the
/// machine does not have. These are the ones a Hebrew reader is likely to have
/// and the ones the stylesheet already names — and [`Look::hebrew_font`] takes
/// any text, so a reader with something else types it.
///
/// The empty first entry is *the stylesheet's own stack*, which is what a
/// reader who has never opened the panel is already reading in. Offering it
/// as a choice is how they get back to it.
///
/// It lived in the shell, which the README says decides nothing. Which fonts
/// a Hebrew reading application offers is a decision, and it is the kind that
/// wants to be beside `Look` rather than beside the command that serialises
/// it.
pub const FONTS: &[&str] = &[
    "",
    "Frank Ruehl CLM",
    "David CLM",
    "Taamey Frank CLM",
    "Frank Ruhl Hofshi",
    "David Libre",
    "Narkisim",
    "SBL Hebrew",
    "Times New Roman",
    "Segoe UI",
];

/// A hundredth of a line, so a session compares equal after a round trip — the
/// same reason `Layout::ratio` is in tenths of a percent and not a float.
const fn leading() -> u16 {
    195
}

const fn measure() -> u16 {
    0
}

impl Default for Look {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            hebrew_font: String::new(),
            latin_font: String::new(),
            line_height: leading(),
            column_ch: measure(),
        }
    }
}

impl Look {
    /// Keep every setting inside what a reader can actually read at.
    ///
    /// Clamped in **one** place, here, rather than in the window and again in the
    /// command — `percent.clamp(60, 250)` in the shell and
    /// `Math.min(250, Math.max(60, …))` in the window is the exact shape B27
    /// points at, and this is the same shape one order later.
    #[must_use]
    pub fn sane(mut self) -> Self {
        self.line_height = self.line_height.clamp(100, 320);
        // Zero is *no limit*, which is a real answer and not a small number.
        if self.column_ch != 0 {
            self.column_ch = self.column_ch.clamp(30, 200);
        }
        self.hebrew_font = self.hebrew_font.trim().to_string();
        self.latin_font = self.latin_font.trim().to_string();
        self
    }
}

const fn hundred() -> u16 {
    100
}

/// How a sefer prints a mekor, which is what a reader expects to see.
const fn full() -> girsa_cite::CiteStyle {
    girsa_cite::CiteStyle::HebrewFull
}

impl Default for Session {
    fn default() -> Self {
        Self {
            workspace: Workspace::default(),
            // Nobody has been asked yet, which is not the same as *there is no
            // corpus*: the four places `roots` already looks are tried first
            // and answer for every ordinary installation.
            corpus: None,
            positions: BTreeMap::new(),
            chosen: BTreeMap::new(),
            alongside: BTreeMap::new(),
            pointing: Pointing::default(),
            shemos: crate::shemos::Shemos::default(),
            was_nikud: None,
            text_size: hundred(),
            cite: full(),
            language: Language::default(),
            interface: Language::default(),
            look: Look::default(),
            keys: BTreeMap::new(),
            showing: girsa_fix::Showing::default(),
            export_into: None,
        }
    }
}

/// The smallest reading size, as a percentage.
///
/// Hebrew with nikud at a small size is unreadable in a way Latin text at the
/// same size is not, which is why the floor is 60 and not 50.
pub const SMALLEST_TEXT: u16 = 60;

/// And the largest.
pub const LARGEST_TEXT: u16 = 250;

impl Session {
    /// Read the session back, or start a fresh one.
    ///
    /// A file that will not parse gives a **fresh session rather than an
    /// error**: a preference file the app refuses to start without is a
    /// preference file that will one day stop somebody reading.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        let mut session: Self = std::fs::read_to_string(path)
            .ok()
            .and_then(|body| serde_json::from_str(&body).ok())
            .unwrap_or_default();
        session.sane();
        session
    }

    /// Keep every number in this file inside what it can mean.
    ///
    /// # On load, and not only in the setters
    ///
    /// `Look::sane` says it clamps *"in one place, here, rather than in the
    /// window and again in the command"* — and it was called from exactly one
    /// place, the `set_look` command, so a session file that arrived with a
    /// line height of 9,999 was believed. `set_text_size` did not use it at
    /// all: it clamped inline, `percent.clamp(60, 250)`, sixty-eight lines
    /// after that sentence. And the clamp that decides what a reader can
    /// actually drag a divider to lived only in `layout.ts`.
    ///
    /// Three numbers, three places, one of them in another language. This is
    /// the one place, and it runs on the way in as well as on the way through —
    /// a clamp that only fires in a setter is a rule about a code path rather
    /// than about the value.
    pub fn sane(&mut self) {
        // A session written when the pointing was a bool. Read once, on the way
        // in, and never written back — see `Session::was_nikud`.
        if let Some(on) = self.was_nikud.take() {
            self.pointing = if on { Pointing::Full } else { Pointing::Plain };
        }
        self.text_size = self.text_size.clamp(SMALLEST_TEXT, LARGEST_TEXT);
        self.look = std::mem::take(&mut self.look).sane();
        self.workspace.sane();
    }

    /// # Errors
    ///
    /// If the directory cannot be made or the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let body = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        // Beside and renamed over. This used to be written on **every scroll
        // event** — the sentence above this one said so as a statement of fact
        // rather than as a finding — which made it the file in the layer most
        // likely to be being written when a machine stops, and losing it loses
        // every pane, every tab and where the reader was in each of them.
        //
        // The shell throttles the scroll now (`State::save_scroll`); every
        // actual decision still writes at once. What this call still owes is
        // the atomicity, which is why it is a rename and not a write.
        let temp = path.with_extension("json.writing");
        std::fs::write(&temp, body)?;
        std::fs::rename(&temp, path)
    }

    /// Remember where a reader is in a sefer.
    pub fn remember(&mut self, at: SegmentId) {
        self.positions.insert(at.work().to_string(), at);
    }

    /// Where they were, last time.
    #[must_use]
    pub fn where_i_was(&self, slug: &str) -> Option<&SegmentId> {
        self.positions.get(slug)
    }

    /// Tick, or untick, one mefaresh on one sefer.
    ///
    /// A newly ticked mefaresh goes on the **end** of the list, where the reader
    /// put it. Unticking the last one removes the sefer's entry entirely rather
    /// than leaving an empty list behind: the file should say what is ticked, not
    /// what once was.
    pub fn choose(&mut self, slug: &str, work: &str, on: bool) {
        let list = self.chosen.entry(slug.to_string()).or_default();
        if on {
            if !list.iter().any(|w| w == work) {
                list.push(work.to_string());
            }
        } else {
            list.retain(|w| w != work);
        }
        if list.is_empty() {
            self.chosen.remove(slug);
        }
    }

    /// The mefarshim ticked on one sefer, in the order they were ticked.
    #[must_use]
    pub fn chosen_for(&self, slug: &str) -> &[String] {
        self.chosen.get(slug).map_or(&[], Vec::as_slice)
    }

    /// Say — or unsay — that two seforim keep the same order.
    ///
    /// Recorded once, under `slug`, and read back in both directions. Unsaying
    /// it clears whichever direction holds it, so a pair the reader made
    /// standing on the Shulchan Arukh can be undone standing on the Tur.
    pub fn pair(&mut self, slug: &str, other: &str, on: bool) {
        // A sefer does not run alongside itself, and a reader who manages to
        // ask should get nothing rather than a row naming the sefer he is in.
        if slug == other {
            return;
        }
        if on {
            let list = self.alongside.entry(slug.to_string()).or_default();
            if !list.iter().any(|w| w == other) {
                list.push(other.to_string());
            }
            return;
        }
        for (here, there) in [(slug, other), (other, slug)] {
            if let Some(list) = self.alongside.get_mut(here) {
                list.retain(|w| w != there);
                if list.is_empty() {
                    self.alongside.remove(here);
                }
            }
        }
    }

    /// The seforim the reader has said run alongside this one, both ways round.
    ///
    /// Sorted and deduplicated, because the two directions can name the same
    /// sefer — a reader who paired A with B and later B with A meant it once.
    #[must_use]
    pub fn alongside_of(&self, slug: &str) -> Vec<String> {
        let mut out: std::collections::BTreeSet<String> = self
            .alongside
            .get(slug)
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        for (here, there) in &self.alongside {
            if there.iter().any(|w| w == slug) {
                out.insert(here.clone());
            }
        }
        out.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::workspace::Axis;

    fn id(work: &str, n: u32) -> SegmentId {
        format!("girsa:{work}/2a:{n}#{n}")
            .parse()
            .expect("a segment id")
    }

    #[test]
    fn a_pair_the_reader_made_is_read_from_either_sefer() {
        // A6. The corpus settles *parallel seforim* from the graph, and the
        // Shulchan Arukh HaRav is the case it cannot: it is written on Orach
        // Chayim's simanim and two of the 505 the graph joins land on their own
        // number, because Sefaria's links between them are citations rather
        // than the structural mapping. So the reader says it.
        //
        // Said once, standing on one of them — and true standing on the other,
        // because parallel is symmetric and asking him to say it twice is
        // asking him to keep a list in two places.
        let mut session = Session::default();
        session.pair("shulchan-arukh/orach-chayim", "shulchan-arukh-harav", true);
        assert_eq!(
            session.alongside_of("shulchan-arukh/orach-chayim"),
            vec!["shulchan-arukh-harav".to_string()]
        );
        assert_eq!(
            session.alongside_of("shulchan-arukh-harav"),
            vec!["shulchan-arukh/orach-chayim".to_string()],
            "the pair reads from the other end too"
        );

        // Saying it twice is saying it once.
        session.pair("shulchan-arukh/orach-chayim", "shulchan-arukh-harav", true);
        assert_eq!(session.alongside_of("shulchan-arukh-harav").len(), 1);

        // And it can be undone **from either end**, which is the other half of
        // reading it both ways: a pair you can make from here and only unmake
        // from there is a pair a reader will think is stuck.
        session.pair("shulchan-arukh-harav", "shulchan-arukh/orach-chayim", false);
        assert!(session
            .alongside_of("shulchan-arukh/orach-chayim")
            .is_empty());
        assert!(
            session.alongside.is_empty(),
            "and nothing empty is left behind"
        );

        // A sefer does not run alongside itself.
        session.pair("tur", "tur", true);
        assert!(session.alongside_of("tur").is_empty());
    }

    #[test]
    fn the_language_says_which_of_a_sefer_s_two_names_to_print() {
        assert_eq!(Language::Hebrew.title_of("ברכות", "Berakhot"), "ברכות");
        assert_eq!(Language::English.title_of("ברכות", "Berakhot"), "Berakhot");
    }

    #[test]
    fn a_sefer_with_only_one_name_is_still_named() {
        // The corpus has works with one title and not the other. A row with no
        // name on it is worse than a row named in the wrong language.
        assert_eq!(Language::English.title_of("ברכות", ""), "ברכות");
        assert_eq!(Language::Hebrew.title_of("   ", "Berakhot"), "Berakhot");
    }

    #[test]
    fn hebrew_is_the_default_and_a_session_from_before_the_setting_still_loads() {
        let dir = std::env::temp_dir().join("girsa-app-session-language");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("session.json");
        std::fs::write(&path, r#"{"text_size":120}"#).expect("writes");

        let session = Session::load(&path);
        assert_eq!(session.language, Language::Hebrew);
        assert_eq!(
            session.text_size, 120,
            "the file was read, not fallen back from"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_language_it_is_in_says_which_way_it_reads() {
        assert!(Language::Hebrew.rtl());
        assert!(!Language::English.rtl());
        assert_eq!(Language::Hebrew.tag(), "he");
    }

    #[test]
    fn a_sefer_reopens_where_it_was_closed() {
        let mut session = Session::default();
        session.remember(id("bavli/berakhot", 7));
        session.remember(id("bavli/shabbat", 2));
        // Reading further on moves the memory rather than adding a second one.
        session.remember(id("bavli/berakhot", 9));

        assert_eq!(
            session.where_i_was("bavli/berakhot"),
            Some(&id("bavli/berakhot", 9))
        );
        assert_eq!(session.where_i_was("bavli/eruvin"), None);
        assert_eq!(session.positions.len(), 2);
    }

    #[test]
    fn the_mefarshim_you_ticked_are_remembered_per_sefer() {
        let mut session = Session::default();
        session.choose("bavli/berakhot", "bavli/tosafot-on-berakhot", true);
        session.choose("bavli/berakhot", "bavli/rashi-on-berakhot", true);
        session.choose("bavli/shabbat", "bavli/rashi-on-shabbat", true);

        // The order is the reader's, not the alphabet's: they put Tosafot first.
        assert_eq!(
            session.chosen_for("bavli/berakhot"),
            ["bavli/tosafot-on-berakhot", "bavli/rashi-on-berakhot"]
        );
        // Per sefer. Ticking Rashi on Berakhot does not tick Rashi on Shabbat,
        // which is a different sefer with a different pshat to follow.
        assert_eq!(
            session.chosen_for("bavli/shabbat"),
            ["bavli/rashi-on-shabbat"]
        );
        assert!(session.chosen_for("bavli/eruvin").is_empty());
    }

    #[test]
    fn ticking_the_same_mefaresh_twice_ticks_it_once() {
        let mut session = Session::default();
        session.choose("bavli/berakhot", "bavli/rashi-on-berakhot", true);
        session.choose("bavli/berakhot", "bavli/rashi-on-berakhot", true);
        assert_eq!(session.chosen_for("bavli/berakhot").len(), 1);

        session.choose("bavli/berakhot", "bavli/rashi-on-berakhot", false);
        assert!(session.chosen_for("bavli/berakhot").is_empty());
        // Unticking the last one does not leave a sefer behind in the file.
        assert!(!session.chosen.contains_key("bavli/berakhot"));
    }

    #[test]
    fn the_ticked_mefarshim_survive_closing_the_sefer() {
        let dir = std::env::temp_dir().join("girsa-app-session-chosen");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let mut session = Session::default();
        session.choose("bavli/berakhot", "bavli/rashi-on-berakhot", true);
        session.save(&path).expect("saves");

        assert_eq!(
            Session::load(&path).chosen_for("bavli/berakhot"),
            ["bavli/rashi-on-berakhot"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_session_written_before_the_mefarshim_were_tickable_still_loads() {
        // The field is `serde(default)`, and this is the test that says why: a
        // reader upgrading has a session file with no `chosen` in it, and a
        // parse failure there costs them every tab they had open.
        let dir = std::env::temp_dir().join("girsa-app-session-old");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("session.json");
        std::fs::write(&path, r#"{"nikud":false,"text_size":120}"#).expect("writes");

        let session = Session::load(&path);
        assert_eq!(
            session.pointing,
            Pointing::Plain,
            "the file was read, not fallen back from — and `nikud: false` is `plain`"
        );
        assert!(session.chosen_for("bavli/berakhot").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_whole_session_survives_a_restart() {
        let dir = std::env::temp_dir().join("girsa-app-session-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("session.json");

        let mut session = Session::default();
        let gemara = session.workspace.open_tab("bavli/berakhot", None);
        session
            .workspace
            .split(gemara, Axis::Vertical, "bavli/rashi-on-berakhot", true)
            .expect("splits");
        session.remember(id("bavli/berakhot", 4));
        session.pointing = Pointing::Nikud;
        session.save(&path).expect("saves");

        assert_eq!(Session::load(&path), session);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_session_file_that_will_not_parse_costs_a_layout_and_not_the_app() {
        let dir = std::env::temp_dir().join("girsa-app-session-broken");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("session.json");
        std::fs::write(&path, "{ this is not json").expect("writes");

        let session = Session::load(&path);
        assert_eq!(session, Session::default());
        assert_eq!(
            session.pointing,
            Pointing::Full,
            "the default is the sefer as printed, marks and all"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
