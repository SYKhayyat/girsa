// What a key press is called, so the window can ask what it means (B13).
//
// > *"No keyboard rebinding."*
//
// # Why the window spells a press at all
//
// The table of shortcuts and the resolution both live in Rust
// (`girsa_app::keys`), where they are tested and where the shortcut card is
// generated from the same list. This file exists for one reason: a `keydown`
// handler has to decide **synchronously** whether to call `preventDefault`, and it
// cannot await a round trip to do it. So Rust hands the window the resolved table
// keyed by the one spelling of each combination, and this turns an event into that
// spelling.
//
// That makes two implementations of *what a press is called*, which is the shape
// this project bans everywhere else — so it is one exported function, four lines
// long, with a test that walks every shipped default and asserts this side spells
// it the way Rust does. If the two ever drift, that test says so rather than a
// reader finding that Ctrl+Shift+C stopped working.

/** Only the parts of a `KeyboardEvent` a binding is made of. */
export interface Pressed {
  key: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
}

/**
 * The one spelling of a press: `Ctrl+Shift+C`, never `Shift+Ctrl+C`.
 *
 * Modifier order is fixed and matches `Press::said` in Rust. Cmd counts as Ctrl,
 * because a Mac reader pressing ⌘F means search and the alternative is a second
 * table for one platform.
 */
export function said(event: Pressed): string {
  let out = "";
  if (event.ctrlKey || event.metaKey) out += "Ctrl+";
  if (event.altKey) out += "Alt+";
  if (event.shiftKey) out += "Shift+";
  // A single character goes up; a named key keeps its name, so `Escape` does not
  // become `ESCAPE` and a card does not read like a ransom note.
  return out + (Array.from(event.key).length === 1 ? event.key.toUpperCase() : event.key);
}

/**
 * What this press was bound to, if anything.
 *
 * `bindings` came from Rust already resolved — the reader's changes over the
 * shipped table, with a reader's binding having displaced whatever default held
 * that key. Nothing is decided here.
 */
export function whatKey(bindings: Record<string, string>, event: Pressed): string | null {
  return bindings[said(event)] ?? null;
}
