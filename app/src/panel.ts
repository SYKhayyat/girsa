// What a panel is, and who gets the keypress while one is open.
//
// # What was there instead
//
// **Eight panels across three mechanisms.** `fix`, `laneview` and `search` keep
// a private `open` boolean; `picker`, `settingsview` and `shelf` read
// `element.hidden`; `linksview` and `suspects` read an `is-open` class. Every
// one of them is correct on its own, and none of them could be asked a question
// generically — so `main.ts` carried **forty-eight hand-written lines** of
// Escape and focus routing that had to name each panel, in the right order,
// with the right one of the three tests:
//
// ```ts
// if (linksview.isOpen && event.key === "Escape") { …close… return; }
// if (yoursview.isOpen && yoursview.element.contains(event.target as Node)) { … }
// if (yoursview.isOpen && event.key === "Escape") { …close… return; }
// if (suspects.isOpen && event.key === "Escape") { …close… return; }
// ```
//
// Nine panels and ten branches, because `yoursview` needs two. Add a panel and
// the way you find out you forgot a line is that Escape does nothing.
//
// # The three questions a keypress asks
//
// 1. **Is anything open that owns the keyboard?** The shelf is a place, not an
//    overlay: while it is open a typed letter is a search in it, and the
//    reading shortcuts are not live. A correction box owns the keyboard only
//    while the caret is inside it — `Ctrl+C` there is *copy*.
// 2. **Does Escape close it?** For most, yes. For the picker, no: it has its
//    own Escape and this must not race it.
// 3. **Is this the key that opens it?** `Ctrl+F` reaches the search panel even
//    while the search panel is open, because that is how a reader closes it.
//    The old routing spelled that as *check `Ctrl+F` between the two `find`
//    branches*, which is the same rule written as an ordering accident.
//
// A panel answers all three by being in the table below rather than by having
// four branches written about it somewhere else.

/** Anything the window can open, close, and ask whether it is open. */
export interface Panel {
  readonly element: HTMLElement;
  readonly isOpen: boolean;
  close(): void;
}

/** Whose keyboard it is while a panel is open. */
export type Keyboard =
  /** The reading shortcuts stay live. Nothing does this yet, and a panel that
   * wants to has somewhere to say so. */
  | "reading"
  /** Only while the caret is inside it — a text box. `Ctrl+C` in one is copy. */
  | "inside"
  /** All of it. A place, like the shelf or the search: a typed letter goes
   * into it and the reading shortcuts are not live. */
  | "all";

/** How far Escape reaches into a panel. */
export type Escapes =
  /** It handles its own — the picker does, and a second one here would race it. */
  | false
  /** Only while the caret is inside it. A buffer does not close because
   * somebody pressed Escape while reading. */
  | "inside"
  /** From anywhere. A drawer over the reading closes wherever the caret is. */
  | "anywhere";

/** One panel, and what it does with a keypress. */
export interface Held {
  panel: Panel;
  keyboard: Keyboard;
  escape: Escapes;
  /**
   * The action id that opens it — which reaches it **even while it is open**,
   * because that is how a reader closes a panel they opened with a key.
   */
  toggle?: string;
  /** Any other key it answers itself while it holds the keyboard. */
  answers?: (event: KeyboardEvent) => boolean;
}

/** What routing decided. `null` means the keypress is the reading's. */
export type Routed = "closed" | "answered" | "swallowed" | null;

/**
 * Give a keypress to the first open panel that wants it.
 *
 * Two questions, in order, because they are two questions: **who does Escape
 * close**, and **whose keyboard is it**. The old routing answered them in one
 * pass of nine `if`s, which is why `yoursview` needed two of them — it takes
 * Escape from anywhere and owns the keyboard only while the caret is in it, and
 * there was nowhere to say that except twice.
 *
 * `inside` says whether the caret is in a panel — a function rather than a
 * `contains` call, so a test needs no DOM. `did` is the action the key is bound
 * to, from `girsa_app::keys`, so a panel's own toggle can reach it.
 */
export function route(
  // `readonly`, because the caller's table is frozen. Routing reads it and has
  // no business holding a handle that could reorder the panels — the order *is*
  // the rule (the first open one that wants a key gets it).
  panels: readonly Held[],
  event: KeyboardEvent,
  inside: (panel: Panel) => boolean,
  did: string | null,
): Routed {
  if (event.key === "Escape") {
    const takes = panels.find(
      (held) =>
        held.panel.isOpen &&
        held.escape !== false &&
        (held.escape === "anywhere" || inside(held.panel)),
    );
    if (takes) {
      takes.panel.close();
      return "closed";
    }
  }
  for (const held of panels) {
    if (!held.panel.isOpen) continue;
    const holds = held.keyboard === "all" || (held.keyboard === "inside" && inside(held.panel));
    if (!holds) continue;
    if (held.answers?.(event)) return "answered";
    // Its own key still reaches it. A reader who opened the search with Ctrl+F
    // closes it with Ctrl+F — and the old routing spelled that as *check Ctrl+F
    // between the two `find` branches*, the same rule written as an ordering
    // accident the next panel would not inherit.
    if (held.toggle && did === held.toggle) return null;
    return "swallowed";
  }
  return null;
}
