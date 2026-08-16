//! What a key press means, and what a reader may change it to (B13).
//!
//! > *"No keyboard rebinding."*
//!
//! # Why this is Rust and not eighteen `else if`s in the window
//!
//! It was eighteen `else if`s in the window — `Ctrl+O`, `Ctrl+B`, `Ctrl+F`,
//! `Ctrl+L`, `Ctrl+\`, `Ctrl+W`, and a dozen more — each comparing
//! `event.key.toLowerCase()` against a letter written in place. Three things
//! follow from that shape and all three were true:
//!
//! - there is **no list** of what the shortcuts are, so the shortcut card the
//!   documentation needs (B36) cannot be generated and has to be written by hand
//!   and kept in step by hope;
//! - a reader cannot rebind one, because there is nothing to rebind *to*;
//! - two of them can quietly claim the same key and nothing says so.
//!
//! So the table is here, the resolution is a function of a table and a press, and
//! both are tested. The window asks *what did they mean* and does it.
//!
//! # What a binding is allowed to be
//!
//! `Ctrl+F`, `Ctrl+Shift+C`, `Alt+N`, `F3`, `Escape`. Modifiers in a fixed order
//! so that a file which says `Shift+Ctrl+C` and one which says `Ctrl+Shift+C`
//! cannot be two different bindings for the same keys — [`Press::said`] is the one
//! spelling, and parsing accepts any order because a reader typing into a box is
//! not thinking about ours.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Everything a reader can ask for by pressing keys.
///
/// The **id** is what a session file stores, so it may never change; the two
/// labels are what a panel and a shortcut card print. Adding a row here is what
/// adding a shortcut means, and the `every_action_is_reachable` test is what stops
/// a row being added with no default and no way to reach it.
pub struct Action {
    pub id: &'static str,
    pub he: &'static str,
    pub en: &'static str,
    /// What it is bound to out of the box.
    pub default: &'static str,
}

/// The whole shortcut table, in the order a card should print it.
///
/// One row per line, which `rustfmt` would turn into six. A table you can read down
/// is the point of it being a table — it is the shortcut card as well as the
/// resolver, and a hundred-and-twenty-line version of it is neither.
#[rustfmt::skip]
pub const ACTIONS: &[Action] = &[
    Action { id: "open", he: "פתח ספר", en: "Open a sefer", default: "Ctrl+O" },
    Action { id: "shelf", he: "עיין במדף", en: "Browse the shelf", default: "Ctrl+B" },
    // **Ctrl+F is the sefer in front of you, and Ctrl+Shift+F is the shelf.**
    //
    // It used to be the other way round, with nothing at all bound to the first
    // one, and that was the whole of the reader's complaint about search:
    // *"narrowing a global search by facet is not the same gesture as Ctrl+F in
    // the Mishnah Berurah in front of you."* He is right, and so is every other
    // application ever written — an editor finds in the file on Ctrl+F and
    // across the project on Ctrl+Shift+F, and a reader's fingers already know
    // that. A reader who rebound `search` keeps their binding; this changes what
    // the two ship as.
    Action { id: "find-here", he: "חפש בספר הזה", en: "Find in this sefer", default: "Ctrl+F" },
    Action { id: "search", he: "חפש בכל המדף", en: "Search the whole shelf", default: "Ctrl+Shift+F" },
    Action { id: "write", he: "פתח את הכתיבה", en: "Open the writing pane", default: "Ctrl+E" },
    Action { id: "beside", he: "מפרשים / ספר לצד", en: "Mefarshim, or a sefer alongside", default: "Ctrl+\\" },
    Action { id: "links", he: "קישורים על השורה", en: "Links on this line", default: "Ctrl+L" },
    // Ctrl+Shift+L, and it is a fix. Both the links button and the lane button
    // printed *(Ctrl+L)* in their tooltips; only the links one was ever wired, so
    // the lane's tooltip named a key that did nothing. Building this table is what
    // found it — the collision test below is the reason.
    Action { id: "lane", he: "הלשון הסמוכה", en: "The adjacent language", default: "Ctrl+Shift+L" },
    // W28's walk, which had a library and a terminal tool and no way in from the
    // window at all.
    Action { id: "chain", he: "שלשלת המסירה", en: "The transmission chain", default: "Ctrl+Shift+M" },
    // The sefer's own contents (A3), so a reader can jump around inside it
    // without a mouse. `T` for תוכן, and Shift because Ctrl+T belongs to the
    // browser everywhere a reader has ever pressed it.
    Action { id: "contents", he: "תוכן הספר", en: "The sefer's contents", default: "Ctrl+Shift+T" },
    // *Here is my place*, reachable without opening a drawer (A15). The marks
    // became visible when the painter stopped skipping every span-less mark;
    // this is the other half, which was still only true of the `yours` panel.
    Action { id: "my-place", he: "המקום שסימנתי", en: "The place I marked", default: "Ctrl+Shift+B" },
    Action { id: "close-pane", he: "סגור את הטור", en: "Close this column", default: "Ctrl+W" },
    Action { id: "send", he: "שלח מקור לכתב", en: "Send a source to Ksav", default: "Ctrl+Shift+C" },
    Action { id: "copy", he: "העתק עם מקור", en: "Copy with its citation", default: "Ctrl+C" },
    Action { id: "note", he: "כתוב על השורה", en: "Write a note on this line", default: "Ctrl+N" },
    Action { id: "highlight", he: "סמן את המילים", en: "Highlight these words", default: "Ctrl+D" },
    Action { id: "mark", he: "סמן בלי צבע", en: "Mark without a colour", default: "Ctrl+Shift+H" },
    Action { id: "mine", he: "מה שכתבתי", en: "What I have written", default: "Ctrl+M" },
    Action { id: "fix", he: "תקן את המילה", en: "Correct this word", default: "Ctrl+K" },
    Action { id: "showing", he: "מה מוצג מהתיקונים", en: "How much of the corrections to apply", default: "Ctrl+Shift+K" },
    Action { id: "queue", he: "תור התיקונים", en: "The correction queue", default: "Ctrl+J" },
    Action { id: "nikud", he: "ניקוד", en: "Nikud", default: "Alt+N" },
    Action { id: "bigger", he: "הגדל", en: "Larger", default: "Ctrl+=" },
    Action { id: "smaller", he: "הקטן", en: "Smaller", default: "Ctrl+-" },
    // Arrangements you named. A tab strip answers *what is open*; it cannot
    // answer *what was I set up for last Tuesday*.
    Action { id: "desks", he: "שולחנות", en: "Named arrangements", default: "Ctrl+Shift+D" },
    // Paper. `girsa-export` writes a `.docx` and this puts the siman in your
    // hand, which are two different mornings.
    Action { id: "print", he: "הדפס את הסימן", en: "Print this section", default: "Ctrl+P" },
    Action { id: "settings", he: "הגדרות", en: "Settings", default: "Ctrl+," },
];

/// A key press, as the window reports one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Press {
    /// The key itself, as the browser names it: `f`, `\`, `F3`, `Escape`.
    pub key: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}

impl Press {
    /// The one spelling of this press. `Ctrl+Shift+C`, never `Shift+Ctrl+C`.
    #[must_use]
    pub fn said(&self) -> String {
        let mut out = String::new();
        if self.ctrl {
            out.push_str("Ctrl+");
        }
        if self.alt {
            out.push_str("Alt+");
        }
        if self.shift {
            out.push_str("Shift+");
        }
        // One letter goes up; a named key keeps its name. `Escape` must not become
        // `ESCAPE` and `f` must not stay `f`, or a card reads like a ransom note.
        if self.key.chars().count() == 1 {
            out.extend(self.key.chars().flat_map(char::to_uppercase));
        } else {
            out.push_str(&self.key);
        }
        out
    }

    /// Read a binding a reader typed, or a default from the table.
    ///
    /// Any modifier order, any case. A reader typing into a box is not thinking
    /// about ours, and a binding that silently did not take because they wrote
    /// `shift+ctrl+c` would be a setting that lies.
    #[must_use]
    pub fn parse(said: &str) -> Option<Self> {
        if said.trim().is_empty() {
            return None;
        }
        let mut out = Self {
            key: String::new(),
            ctrl: false,
            shift: false,
            alt: false,
        };
        for part in said.split('+') {
            let part = part.trim();
            if part.is_empty() {
                // `Ctrl++` — the second `+` is the key, not an empty modifier.
                // Reached only because the text held a `+`, which the guard above
                // is what makes true: splitting `""` also yields one empty part,
                // and calling that the plus key would turn *nothing* into a
                // binding.
                out.key = "+".to_string();
                continue;
            }
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "cmd" | "meta" => out.ctrl = true,
                "shift" => out.shift = true,
                "alt" | "option" => out.alt = true,
                _ => out.key = part.to_string(),
            }
        }
        (!out.key.is_empty()).then_some(out)
    }
}

/// The shortcuts in force: the table, with the reader's changes over it.
#[derive(Debug, Clone, Default)]
pub struct Bound {
    /// Binding → action id. Keyed this way round because the question a key press
    /// asks is *what is this bound to*, and the other direction would be a scan.
    by_press: BTreeMap<String, String>,
}

impl Bound {
    /// Build the live table from what the reader has changed.
    ///
    /// A reader's binding **displaces** the default that held that key, rather
    /// than colliding with it: bind `Ctrl+F` to something else and the action that
    /// had it is left unbound, which is honest, rather than two actions answering
    /// one key and the first one in the list winning.
    #[must_use]
    pub fn of(changed: &BTreeMap<String, String>) -> Self {
        let mut out = Self::default();
        // The reader's first, so a default can never overwrite one of theirs.
        for (id, said) in changed {
            if let Some(press) = Press::parse(said) {
                out.by_press.insert(press.said(), id.clone());
            }
        }
        for action in ACTIONS {
            if changed.contains_key(action.id) {
                continue;
            }
            if let Some(press) = Press::parse(action.default) {
                out.by_press
                    .entry(press.said())
                    .or_insert_with(|| action.id.to_string());
            }
        }
        out
    }

    /// What this press means, if it means anything.
    #[must_use]
    pub fn what(&self, press: &Press) -> Option<&str> {
        self.by_press.get(&press.said()).map(String::as_str)
    }

    /// The whole resolved table, spelling → action id.
    ///
    /// Handed to the window so its `keydown` handler can answer synchronously —
    /// see `app/src/keys.ts`, which is the only other place a press is spelled and
    /// which is tested against these very strings.
    #[must_use]
    pub fn table(&self) -> &BTreeMap<String, String> {
        &self.by_press
    }

    /// What one action is bound to now — for the panel, and for the card.
    #[must_use]
    pub fn on(&self, id: &str) -> Option<String> {
        self.by_press
            .iter()
            .find(|(_, bound)| *bound == id)
            .map(|(press, _)| press.clone())
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn press(key: &str, ctrl: bool, shift: bool, alt: bool) -> Press {
        Press {
            key: key.to_string(),
            ctrl,
            shift,
            alt,
        }
    }

    #[test]
    fn a_press_has_one_spelling_whatever_order_it_was_written_in() {
        // The whole reason `said` exists. A session file that says
        // `Shift+Ctrl+C` and one that says `Ctrl+Shift+C` are the same binding,
        // and two spellings of one key is two bindings that fight.
        let a = Press::parse("Ctrl+Shift+C").expect("parses");
        let b = Press::parse("shift+ctrl+c").expect("parses");
        assert_eq!(a.said(), b.said());
        assert_eq!(a.said(), "Ctrl+Shift+C");
    }

    #[test]
    fn a_named_key_keeps_its_name_and_a_letter_goes_up() {
        assert_eq!(press("f", true, false, false).said(), "Ctrl+F");
        assert_eq!(press("Escape", false, false, false).said(), "Escape");
        assert_eq!(press("F3", false, false, false).said(), "F3");
        // The one that catches a naive `to_uppercase` on the whole string.
        assert_ne!(press("Escape", false, false, false).said(), "ESCAPE");
    }

    #[test]
    fn the_plus_key_is_a_key_and_not_a_missing_modifier() {
        // `Ctrl++` is the zoom binding on half the keyboards in the world.
        let parsed = Press::parse("Ctrl++").expect("parses");
        assert_eq!(parsed.key, "+");
        assert!(parsed.ctrl);
        assert_eq!(parsed.said(), "Ctrl++");
    }

    #[test]
    fn a_binding_with_no_key_in_it_is_not_a_binding() {
        // A reader who pressed only Ctrl while the box was listening.
        assert!(Press::parse("Ctrl").is_none());
        assert!(Press::parse("Ctrl+Shift").is_none());
        assert!(Press::parse("").is_none());
    }

    #[test]
    fn out_of_the_box_the_shortcuts_are_the_ones_that_were_hardcoded() {
        let bound = Bound::of(&BTreeMap::new());
        // **Ctrl+F is the sefer in front of you now, and Ctrl+Shift+F is the
        // shelf.** This asserted the other way round, and the assertion was
        // right about the table and wrong about the reader — see the note on
        // `find-here` in `ACTIONS`.
        assert_eq!(
            bound.what(&press("f", true, false, false)),
            Some("find-here")
        );
        assert_eq!(bound.what(&press("f", true, true, false)), Some("search"));
        assert_eq!(bound.what(&press("b", true, false, false)), Some("shelf"));
        assert_eq!(bound.what(&press("\\", true, false, false)), Some("beside"));
        assert_eq!(bound.what(&press("c", true, true, false)), Some("send"));
        assert_eq!(bound.what(&press("n", false, false, true)), Some("nikud"));
    }

    #[test]
    fn a_press_nobody_bound_means_nothing() {
        let bound = Bound::of(&BTreeMap::new());
        assert_eq!(bound.what(&press("q", true, false, false)), None);
        // A bare letter is typing, not a shortcut.
        assert_eq!(bound.what(&press("f", false, false, false)), None);
    }

    #[test]
    fn a_reader_can_rebind_one() {
        let mut changed = BTreeMap::new();
        changed.insert("search".to_string(), "Ctrl+Alt+F".to_string());
        let bound = Bound::of(&changed);
        assert_eq!(bound.what(&press("f", true, false, true)), Some("search"));
        assert_eq!(bound.on("search").as_deref(), Some("Ctrl+Alt+F"));
        // And the key they took it off is free, not still theirs.
        assert_eq!(bound.what(&press("f", true, true, false)), None);
    }

    #[test]
    fn a_readers_binding_displaces_the_default_that_held_that_key() {
        // Bind the shelf to Ctrl+F and the find loses it. The alternative is
        // two actions answering one key with the list order deciding, which is
        // a bug nobody can see and nobody can fix.
        let mut changed = BTreeMap::new();
        changed.insert("shelf".to_string(), "Ctrl+F".to_string());
        let bound = Bound::of(&changed);
        assert_eq!(bound.what(&press("f", true, false, false)), Some("shelf"));
        assert_eq!(bound.on("find-here"), None, "find-here still claims Ctrl+F");
    }

    #[test]
    fn no_two_actions_ship_bound_to_the_same_keys() {
        // The table is the shortcut card (B36) as well as the resolver, so a
        // collision here is a card that lies as well as a key that surprises.
        let bound = Bound::of(&BTreeMap::new());
        let mut unreachable = Vec::new();
        for action in ACTIONS {
            let press = Press::parse(action.default).expect("every default parses");
            match bound.what(&press) {
                Some(id) if id == action.id => {}
                other => unreachable.push((action.id, action.default, other.map(str::to_string))),
            }
        }
        assert!(
            unreachable.is_empty(),
            "shortcuts that do not reach their own action: {unreachable:?}"
        );
    }

    #[test]
    fn every_action_is_named_in_both_languages_and_has_a_default() {
        // The panel and the card both print these, and an action with a blank
        // label is a row a reader cannot rebind because they cannot tell what it
        // is.
        for action in ACTIONS {
            assert!(!action.id.is_empty());
            assert!(!action.he.trim().is_empty(), "{}", action.id);
            assert!(!action.en.trim().is_empty(), "{}", action.id);
            assert!(
                Press::parse(action.default).is_some(),
                "{} has no usable default",
                action.id
            );
        }
    }

    #[test]
    fn an_id_is_never_used_twice() {
        // Two rows with one id would mean a session file's binding applying to
        // whichever the code happened to reach first.
        let mut seen = std::collections::BTreeSet::new();
        for action in ACTIONS {
            assert!(
                seen.insert(action.id),
                "{} is in the table twice",
                action.id
            );
        }
    }
}
