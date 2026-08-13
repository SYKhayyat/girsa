// Controls that carry their own name.
//
// # Why this module exists
//
// An accessibility snapshot of the window listed 29 controls and **neither text
// field was among them** — not the search box, which is the one control the whole
// application is about:
//
//     {"tag":"INPUT","type":"text","placeholder":"חפש בכל המדף…","aria-label":null,"id":"","class":"find-box"}
//     {"tag":"INPUT","type":"search","placeholder":"","aria-label":null,"id":"","class":"picker-input"}
//
// A placeholder is not a name. It disappears the moment anything is typed, screen
// readers are inconsistent about announcing it, and it is not what `<label>` is
// for. Ksav's README claims *"every control has a name, the page has landmarks,
// the status bar is a live region"* and its snapshot bears that out — 94 named
// nodes, both text fields among them. **Girsa should meet the bar its sibling
// already meets.**
//
// # And it is where the four `button()` helpers went
//
// There were four separate `button(label, title, click)` functions —
// `main.ts:1092`, `scanview.ts:594`, `linksview.ts:278`, `writing.ts:192` — which
// is the "two readers of one value" shape this project's own rules ban, applied to
// the thing every screen is made of. One of them here, and it takes the name as an
// argument rather than hoping the caller sets one.
//
// # The rule
//
// **Nothing in this module can make an unnamed control.** A `label` is required by
// the type, so a control with no name is a compile error rather than an
// accessibility finding. `test/sources.test.mjs` holds the other half: a control
// built by hand, bypassing this, is a test failure.

/** A button with a visible label. The label is the name. */
export function button(label: string, title: string, click: () => void): HTMLButtonElement {
  const out = document.createElement("button");
  out.type = "button";
  out.className = "tool";
  out.textContent = label;
  out.title = title;
  out.addEventListener("click", click);
  return out;
}

/**
 * A button whose face is a glyph — `×`, `−`, `+`.
 *
 * `name` is what it is called, and it goes on `aria-label`, because `−` is not a
 * name and `title` is a tooltip rather than an accessible name in every reader.
 */
export function glyph(face: string, name: string, click: () => void): HTMLButtonElement {
  const out = document.createElement("button");
  out.type = "button";
  out.textContent = face;
  out.title = name;
  out.setAttribute("aria-label", name);
  out.addEventListener("click", click);
  return out;
}

export interface FieldOptions {
  className?: string;
  type?: string;
  placeholder?: string;
  dir?: string;
  inputMode?: string;
  value?: string;
}

/**
 * A text field with a name.
 *
 * `name` is required and goes on `aria-label`. A placeholder is a hint, not a
 * name, and it is optional — which is the right way round: the two fields this
 * module exists because of both had a placeholder and no name.
 */
export function field(name: string, options: FieldOptions = {}): HTMLInputElement {
  const out = document.createElement("input");
  out.type = options.type ?? "text";
  if (options.className) out.className = options.className;
  if (options.placeholder) out.placeholder = options.placeholder;
  if (options.dir) out.setAttribute("dir", options.dir);
  if (options.inputMode) out.inputMode = options.inputMode;
  if (options.value !== undefined) out.value = options.value;
  out.setAttribute("aria-label", name);
  return out;
}

/** A `<textarea>` with a name. */
export function area(name: string, options: FieldOptions = {}): HTMLTextAreaElement {
  const out = document.createElement("textarea");
  if (options.className) out.className = options.className;
  if (options.placeholder) out.placeholder = options.placeholder;
  if (options.dir) out.setAttribute("dir", options.dir);
  if (options.value !== undefined) out.value = options.value;
  out.setAttribute("aria-label", name);
  return out;
}

/** A `<select>` with a name. */
export function choice(name: string, className?: string): HTMLSelectElement {
  const out = document.createElement("select");
  if (className) out.className = className;
  out.setAttribute("aria-label", name);
  return out;
}

/**
 * A region of the page, so a reader can jump to it.
 *
 * `role` and a name, together — a landmark with no name is one of several
 * unlabelled regions, which is barely better than none.
 */
export function region(role: string, name: string, className?: string): HTMLElement {
  const out = document.createElement("div");
  if (className) out.className = className;
  out.setAttribute("role", role);
  out.setAttribute("aria-label", name);
  return out;
}

/**
 * The strip of buttons in a pane's header, as **one box**.
 *
 * # Why the buttons are a box and not five loose children
 *
 * They were five loose children of `.pane-head`, each `flex: 0 1 auto`, beside
 * a `.pane-title` that was `min-width: 0` with an ellipsis. So the sefer's name
 * was the first thing the flexbox squeezed and it went all the way to zero.
 * Measured at 1360px with a Gemara and two mefarshim:
 *
 * ```text
 * pane 1 (בראשית)            title 42 px wide
 * pane 2 (רש"י על בראשית)    title  0 px wide     ← invisible
 * pane 3 (רמב"ן על בראשית)   title  6 px wide     ← one letter
 * ```
 *
 * An ellipsis on a zero-width box shows nothing, so the application's signature
 * arrangement — the daf with two commentaries — gave a reader three columns
 * they could not tell apart. In English the header overflowed the other way and
 * clipped the leftmost *button* to `se`.
 *
 * The owner's decision the audit left open is *what gives way first, the name
 * of the sefer or the fifth button*, and the answer is **neither**. As one box
 * the buttons wrap to a second line when the header runs out of room: the title
 * keeps its width, every button stays where it was and stays reachable, and
 * nothing is hidden behind a menu that would have to be built, routed and given
 * an Escape. It costs a row of header in a narrow column, which is the cheapest
 * thing on the screen.
 */
export function toolStrip(): HTMLElement {
  const out = document.createElement("div");
  out.className = "pane-tools";
  return out;
}

/**
 * Mark an element as a live region.
 *
 * `polite` for a status line, which is everything here: `assertive` interrupts
 * whatever a reader is in the middle of, and nothing this application says is
 * worth that.
 */
export function announces(element: HTMLElement, name: string): void {
  element.setAttribute("role", "status");
  element.setAttribute("aria-live", "polite");
  element.setAttribute("aria-label", name);
}
