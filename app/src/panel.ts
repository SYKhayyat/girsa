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
//
// # The fourth question, which cost every shortcut in the application
//
// **Is it over the reading, or beside it?** W47 and W48 made the bookcase and
// the search *dock* rather than close when you go through them — open a sefer,
// click a result — so the ordinary path leaves one of them standing beside the
// daf. Both were registered `keyboard: "all"`, which was right for the overlay
// and catastrophic for the column: the reader is reading, and every key they
// press is swallowed by a panel they are not looking at.
//
// Measured on the release build:
//
// ```
// search docked (after clicking a result) → Ctrl+C → nothing. No toast.
// search closed                           → Ctrl+C → "הועתק — שבת דף ל: שורה ז'"
// ```
//
// Which took Ctrl+Shift+C with it — *the* five-minute story in `start-here.md`
// is search, click the hit, highlight, send to Ksav, and the send did nothing,
// silently, in every build that has ever existed.
//
// So `keyboard` may be a **function**, and a panel that is an overlay some of
// the time says so; and the caret has three positions rather than two, because
// focus inside a docked panel is almost always on a *button* — every result you
// click is one — and `inside` would hand that button the whole keyboard.

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
  /**
   * Only while the caret is in something you **type into**. A panel standing
   * beside the reading rather than over it: its box owns what is typed into the
   * box, and nothing else does.
   *
   * A third mode rather than `inside`, because focus lands on a *button* inside
   * a docked panel constantly — every result you click is one. Under `inside`
   * that button would hold the whole keyboard while the reader is looking at
   * the daf, which is finding 3.
   */
  | "typing"
  /** All of it. A place, like the shelf or the search: a typed letter goes
   * into it and the reading shortcuts are not live. */
  | "all";

/**
 * Where the caret is, relative to one panel.
 *
 * Three answers rather than a boolean, because a panel beside the reading needs
 * the distinction between *the focus is on my close button* and *the reader is
 * typing into my box*. A boolean cannot carry it.
 */
export type Caret =
  /** Nowhere near this panel. */
  | "away"
  /** Somewhere in it, but not in anything you type into. A button, a row. */
  | "on"
  /** In a text field of it. A typed letter is text, and `Ctrl+C` is copy. */
  | "typing";

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
  /**
   * Whose keyboard it is — or a function, for a panel that is an overlay some
   * of the time and a column beside the reading the rest of it.
   *
   * The search and the bookcase are both: opened with a key they cover the
   * reading and a typed letter is theirs; opened *through* — click a result,
   * open a sefer — they dock, and the reader is reading again. One constant
   * cannot say that, and saying it wrongly cost every shortcut in the
   * application.
   */
  keyboard: Keyboard | (() => Keyboard);
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
 * `caret` says where the caret is relative to a panel — a function rather than a
 * `contains` call, so a test needs no DOM. `did` is the action the key is bound
 * to, from `girsa_app::keys`, so a panel's own toggle can reach it.
 */
export function route(
  // `readonly`, because the caller's table is frozen. Routing reads it and has
  // no business holding a handle that could reorder the panels — the order *is*
  // the rule (the first open one that wants a key gets it).
  panels: readonly Held[],
  event: KeyboardEvent,
  caret: (panel: Panel) => Caret,
  did: string | null,
): Routed {
  if (event.key === "Escape") {
    const takes = panels.find(
      (held) =>
        held.panel.isOpen &&
        held.escape !== false &&
        (held.escape === "anywhere" || caret(held.panel) !== "away"),
    );
    if (takes) {
      takes.panel.close();
      return "closed";
    }
  }
  for (const held of panels) {
    if (!held.panel.isOpen) continue;
    const holds = whoHolds(typeof held.keyboard === "function" ? held.keyboard() : held.keyboard, () =>
      caret(held.panel),
    );
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

/**
 * Whether a panel in this mode holds the keyboard, given where the caret is.
 *
 * `where` is lazy because `reading` and `all` are answers on their own and the
 * caret is a DOM question at every keypress.
 */
function whoHolds(mode: Keyboard, where: () => Caret): boolean {
  switch (mode) {
    case "reading":
      return false;
    case "all":
      return true;
    case "inside":
      return where() !== "away";
    case "typing":
      return where() === "typing";
  }
}
