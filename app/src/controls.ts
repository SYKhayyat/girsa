// Controls that carry their own name.
//
// The one import is `say.ts`, and it arrived with `ask()` at the bottom: a
// question the reader has to answer needs an OK and a Cancel, and those are
// words. Everything above it is still furniture with no vocabulary of its own.
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
import { say } from "./say.ts";

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
export function glyph(
  face: string,
  name: string,
  // The event, for the one case that needs it: a glyph **inside a row that is
  // itself clickable** has to stop the click reaching the row. Callers that do
  // not care still pass `() => void`, which TypeScript accepts for a handler
  // taking an argument.
  click: (event: MouseEvent) => void,
): HTMLButtonElement {
  const out = document.createElement("button");
  out.type = "button";
  out.textContent = face;
  out.title = name;
  out.setAttribute("aria-label", name);
  out.addEventListener("click", click);
  return out;
}

/**
 * The `×` that shuts a panel.
 *
 * Eight panels drew their own close affordance and six of them spelled it as the
 * **word** `סגור` on a `.tool` button. The reader's ruling: *"this app has a
 * habit of using close instead of the ×, and that is bad."* He is right, and the
 * argument against him is one this repository wrote down at `main.ts`'s tab
 * close — *"`×` is a glyph and a glyph is not a name"*. That argument is sound
 * about **names** and it answered the wrong question. A name is what a control
 * is called; a face is what a reader's eye finds without reading. `×` is the one
 * face every window in every operating system has taught him, and `say("close")`
 * is still the name — it goes on `aria-label` and the tooltip, where a name
 * belongs, which is exactly what [`glyph`] was built to do.
 *
 * And Escape is not an affordance. It is a keystroke nothing on the screen
 * mentions, which is why `panel.ts` routing all of them correctly did not make a
 * single panel look closable.
 *
 * Here rather than in each panel so there is one `×` to change, and so a ninth
 * panel cannot be added without one.
 */
export function shut(click: () => void): HTMLButtonElement {
  const out = glyph("×", say("close"), click);
  out.className = "panel-shut";
  return out;
}

/**
 * The sentence under a panel's title that says what the panel answers.
 *
 * # Why a panel needs one
 *
 * Asked what the chain panel and the links panel show him, the reader answered
 * both times: *"All of it — I don't know what I'm looking at."* Later, plainer:
 * *"idk what links is."*
 *
 * Neither panel was missing an explanation. `say.ts` already carries
 * `chainForwardWhy`, `chainBackWhy`, `chainForksWhy`, `linksShowWork` — good
 * sentences, every one of them written to explain exactly this. They are on
 * `title`, so they are **tooltips**: they appear if you hold a pointer still
 * over the right control for a second, having already guessed that the control
 * is the thing you do not understand. A reader who does not know what a panel is
 * has no reason to hover over anything in it.
 *
 * So the sentence goes on the screen, under the title, where the question *what
 * am I looking at* is asked. It costs one line of a panel that is a column tall.
 */
export function about(sentence: string): HTMLElement {
  const out = document.createElement("p");
  out.className = "panel-about";
  out.textContent = sentence;
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

/**
 * Ask the reader a question, in this window's own furniture.
 *
 * # What was here instead
 *
 * `window.prompt`. Four of them: *write a note on this line* — which is on the
 * shortcut card as one of the eleven things Girsa does — naming a saved query,
 * making a shelf, and resetting the arrangement. In the shell that renders as
 * the webview's own modal, captioned with the origin:
 *
 * ```
 * localhost:5174 says
 * מה אתה אומר על השורה?
 * [OK] [Cancel]
 * ```
 *
 * A packaged build says `tauri.localhost says` instead, which is not better. It
 * is the browser talking, in the browser's box, in the browser's language, with
 * the browser's buttons — in an application whose entire argument is that a
 * Hebrew reader deserves furniture that was built for them. `window.prompt`
 * also cannot be styled, cannot be sized, cannot hold more than one line, and
 * blocks the whole webview while it is open.
 *
 * # Modal means modal
 *
 * While a question is open, **no other key in this window does anything**. The
 * listener is on `document` in the capture phase and stops propagation for
 * everything, which is one rule stated once — cheaper and harder to get wrong
 * than an entry in `panel.ts`'s table for a panel that exists for four seconds.
 * Focus goes back where it came from when the question closes, because a reader
 * who cancels should be where they were.
 */
export interface Asked {
  /** What is in the box when it opens. */
  value?: string;
  /** A note under the question — what the answer is for, when that is not
   * obvious from the question itself. */
  hint?: string;
  /** Prose rather than a name, so Enter makes a new line and Ctrl+Enter
   * answers. A note on a line is prose; a shelf's title is not. */
  prose?: boolean;
  /** What the affirmative button says. Defaults to *OK*. */
  ok?: string;
}

/** Ask for a line (or a paragraph) of text. `null` if the reader backed out. */
export function ask(question: string, asked: Asked = {}): Promise<string | null> {
  const box = asked.prose
    ? area(question, { className: "ask-box is-prose", value: asked.value ?? "" })
    : field(question, { className: "ask-box", value: asked.value ?? "" });
  return sheet(question, asked, box).then((yes) => (yes ? box.value : null));
}

/** Ask a yes-or-no. */
export function confirmThat(question: string, asked: Asked = {}): Promise<boolean> {
  return sheet(question, asked, null);
}

function sheet(
  question: string,
  asked: Asked,
  box: HTMLInputElement | HTMLTextAreaElement | null,
): Promise<boolean> {
  return new Promise((settle) => {
    const was = document.activeElement;
    const over = document.createElement("div");
    over.className = "ask";

    const card = region("dialog", question, "ask-sheet");
    const said = document.createElement("p");
    said.className = "ask-question";
    said.textContent = question;
    card.append(said);
    if (asked.hint) {
      const hint = document.createElement("p");
      hint.className = "ask-hint";
      hint.textContent = asked.hint;
      card.append(hint);
    }
    if (box) card.append(box);

    const done = (answered: boolean): void => {
      document.removeEventListener("keydown", key, true);
      over.remove();
      if (was instanceof HTMLElement) was.focus();
      settle(answered);
    };

    const row = document.createElement("div");
    row.className = "ask-buttons";
    const affirm = asked.ok ?? say("askOk");
    const yes = button(affirm, affirm, () => done(true));
    yes.classList.add("is-primary");
    row.append(yes, button(say("askCancel"), say("askCancel"), () => done(false)));
    card.append(row);

    over.append(card);
    over.addEventListener("pointerdown", (event) => {
      if (event.target === over) done(false);
    });

    // Everything, in capture, so nothing else in the window sees a key while a
    // question is open — including the pane under it, which would otherwise
    // turn its page when the reader typed a `d`.
    const key = (event: KeyboardEvent): void => {
      event.stopPropagation();
      if (event.key === "Escape") {
        event.preventDefault();
        done(false);
        return;
      }
      if (event.key === "Tab") {
        // Trapped. A modal that lets Tab walk into the occluded window behind
        // it claimed modality and did not keep it.
        event.preventDefault();
        const focusable = [
          ...card.querySelectorAll<HTMLElement>(
            "button, input, textarea, select, [tabindex]:not([tabindex='-1'])",
          ),
        ].filter((el) => !(el instanceof HTMLButtonElement && el.disabled));
        if (focusable.length === 0) return;
        const now = focusable.indexOf(document.activeElement as HTMLElement);
        const next = event.shiftKey
          ? now <= 0
            ? focusable.length - 1
            : now - 1
          : now < 0 || now === focusable.length - 1
            ? 0
            : now + 1;
        focusable[next].focus();
        return;
      }
      if (event.key !== "Enter") return;
      // In prose, Enter is a new line and Ctrl+Enter answers. Anywhere else,
      // Enter answers — a shelf's title is one line and always will be.
      if (asked.prose && !event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      done(true);
    };
    document.addEventListener("keydown", key, true);

    document.body.append(over);
    if (box) {
      box.focus();
      box.select();
    } else {
      yes.focus();
    }
  });
}
