//! Print the shortcut card (B36).
//!
//! > *"no keyboard-shortcut card (Ksav has 29 bindings, discoverable only by
//! > hovering)"*
//!
//! Generated, not written. A card typed out by hand is a card that is right on the
//! day it is typed: the reason B36 asks for one is that nobody could find out what
//! the shortcuts were, and a second hand-maintained list of them would be the same
//! problem with an extra copy. This reads `girsa_app::keys::ACTIONS`, which is the
//! table the window actually resolves against, so the card is wrong only if the
//! application is.
//!
//! ```text
//! cargo run -p girsa-app --bin girsa-card > docs/shortcuts.md
//! ```
//!
//! It prints **both languages**, because B36 asks for both and because the two
//! halves of this project's audience do not overlap as much as it would be
//! convenient to assume.

use girsa_app::keys::{Press, ACTIONS};

fn main() {
    println!("# Girsa — keyboard shortcuts · גִּרְסָא — מקשים");
    println!();
    println!("<!-- Generated: cargo run -p girsa-app --bin girsa-card > docs/shortcuts.md");
    println!("     Do not edit by hand. The table is `crates/girsa-app/src/keys.rs`, which is");
    println!("     what the window resolves a key press against — so this card is wrong only");
    println!("     if the application is. -->");
    println!();
    println!("Every one of these can be changed: **Ctrl+,** opens the settings, and each row");
    println!("there rebinds by pressing the keys you want. `↺` puts one back.");
    println!();
    println!("כל אחד מהם ניתן לשינוי: **Ctrl+,** פותח את ההגדרות, ובכל שורה שם אפשר");
    println!("להקליד את המקש הרצוי. `↺` מחזיר לברירת המחדל.");
    println!();
    println!("| Keys · מקשים | What it does | מה זה עושה |");
    println!("|---|---|---|");
    for action in ACTIONS {
        // Through `Press` rather than printing `action.default` raw, so the card
        // shows the same spelling the settings panel does. Two spellings of one
        // binding in two places a reader looks is exactly the drift this is here to
        // prevent.
        let keys = Press::parse(action.default)
            .map_or_else(|| action.default.to_string(), |press| press.said());
        // Only the pipe. A backslash inside a code span is a backslash — markdown
        // reads no escapes in there — so doubling it printed `Ctrl+\\`, which is a
        // key nobody has. The pipe does have to go, or the row loses a column.
        let keys = keys.replace('|', "\\|");
        println!("| `{keys}` | {} | {} |", action.en, action.he);
    }
    println!();
    println!("## Not on this card");
    println!();
    println!("**Escape** closes whatever is open — the search, the shelf, a drawer, the");
    println!("correction box. It is not in the table because it is not rebindable: a reader");
    println!("who bound Escape to something else would have no way out of a panel.");
    println!();
    println!("**Ctrl+C** deliberately does not stop the webview's own copy. The words go to");
    println!("the clipboard the way they always would, and the citation goes with them");
    println!("(spec.md §10.2 — *the user does nothing different*). If the citation half");
    println!("fails, you still have the text.");
    println!();
    println!("**Clicking a line** opens the mefarshim you have ticked, on that line. Only");
    println!("when you have ticked at least one — otherwise a click is just a click.");
}
