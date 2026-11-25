// The one box: type a place any way you like and go there.
//
// # What it is, and what it is not
//
// spec.md §9.7 and #63 ask for a single input that reads `berachos 2a`,
// `ברכות ב א`, `2a`, `berachos 2a, shabbos 3b` or `(berachos 2a, shabbos 3b)`.
// **The reading is not here.** Which sefer, which daf, whether the parentheses
// mean one source sheet — all of that is `girsa_app::nav` and `girsa_ref`'s
// resolver, called through `nav_ask`, and this file must not hold a second
// opinion about it. What is here is the part the window alone can have: an input
// a reader can reach with a key, and what to say about each of the answers.
//
// # Why the answer is read once, in one place
//
// `nav_ask` returns six different things on purpose — *go here*, *several
// places*, *several places as one sheet*, *choose*, *which sefer*, *nothing* —
// and the window has to treat them differently, because they are six different
// things to do next. The mapping from answer to *what the window does next* is
// [`intentOf`], and it is a function rather than a `switch` inside a handler so
// that it can be tested without a webview. A new answer shape that quietly fell
// through to *nothing* would be the window telling a reader their words meant
// nothing when the library had in fact understood them.

import { api, type Asked, type Unread } from "./api.ts";
import { field } from "./controls.ts";
import { say } from "./say.ts";
import { sayTrouble } from "./trouble.ts";

/** What `nav_ask` said, as `api.ts` spells it. */
export type Answer = Awaited<ReturnType<typeof api.navAsk>>;

/**
 * What the window should do about it.
 *
 * `what` is the six-way answer and the rest is what each of them needs to act:
 * the refs to land, whether they are one sheet, what could not be read, and the
 * question to ask. One struct rather than six because every caller wants the
 * same three fields and only reads the ones its branch uses.
 */
export interface Intent {
  what: "go" | "several" | "sheet" | "choose" | "ask" | "nothing";
  /** The refs in the answer, in the order the resolver gave them. */
  refs: string[];
  /** True when the reader wrote them in parentheses (§9.7). */
  sheet: boolean;
  /** The parts that could not be read, and why — carried beside the refs. */
  unread: Unread[];
  /** What the location is missing, for `ask`: `a sefer`, or nothing. */
  about: string | null;
  /** The landing, when the answer was one place. */
  landing: Asked | null;
}

/**
 * Read one answer into what the window is going to do.
 *
 * `sheet` is checked **before** the count, and that order is the whole of §9.7's
 * parentheses: `(berachos 2a, shabbos 3b)` and `berachos 2a, shabbos 3b` are the
 * same refs and different intentions, and the flag travels with the answer
 * rather than being re-read off what the reader typed. One parenthesised pair is
 * still a sheet.
 */
export function intentOf(answer: Answer): Intent {
  const base: Intent = {
    what: "nothing",
    refs: [],
    sheet: false,
    unread: [],
    about: null,
    landing: null,
  };
  switch (answer.kind) {
    case "go":
      return { ...base, what: "go", landing: answer.landing };
    case "several":
      return {
        ...base,
        what: answer.sheet ? "sheet" : "several",
        refs: answer.refs,
        sheet: answer.sheet,
        unread: answer.unread,
      };
    case "choose":
      return { ...base, what: "choose", refs: answer.refs };
    case "ask":
      return { ...base, what: "ask", about: answer.about };
    case "nothing":
      return base;
  }
}

/**
 * The box itself.
 *
 * Deliberately small: an input, a note under it, and Escape. Everything it can
 * do is something `goToCitation` already did from a citation in a document, so
 * the box is a second way to reach the same question rather than a second
 * implementation of it — which is the difference between a keyboard route and a
 * fork.
 */
export class NavBox {
  readonly element: HTMLElement;
  private readonly box: HTMLInputElement;
  private readonly note: HTMLElement;
  private open = false;
  private ask: ((typed: string) => Promise<string | null>) | null = null;

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "nav-box";
    this.element.hidden = true;

    const row = document.createElement("div");
    row.className = "nav-row";
    this.box = field(say("goToPlace"), { placeholder: say("goToPlaceHint"), dir: "auto" });
    this.box.className = "nav-where";
    this.note = document.createElement("div");
    this.note.className = "nav-note";

    row.append(this.box);
    this.element.append(row, this.note);
    this.box.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        void this.go();
      }
    });
  }

  /**
   * Who to ask. The window supplies this, because going somewhere is the
   * window's business: which pane is focused, whether to open a tab, and what to
   * say when a sefer is not on the shelf are all facts this box does not have.
   *
   * It answers with a sentence to put under the box, or `null` when the window
   * has already said what happened — a landed reader, for instance, is looking at
   * the answer rather than at a note about it.
   */
  onAsk(ask: (typed: string) => Promise<string | null>): void {
    this.ask = ask;
  }

  get isOpen(): boolean {
    return this.open;
  }

  /** Open over whatever the reader is reading, with the box ready to type into. */
  show(over: HTMLElement): void {
    over.append(this.element);
    // **Under the pane's own header, measured.** The find bar learned this the
    // hard way: a fixed offset landed on top of the header's buttons on a wide
    // pane and on a narrow one at the same time, because the header wraps to two
    // lines and carries a following-chip on some of them (`FindHere.openOn`).
    const head = over.querySelector<HTMLElement>(".pane-head");
    this.element.style.setProperty("--under-head", `${(head?.offsetHeight ?? 34) + 4}px`);
    this.element.hidden = false;
    this.open = true;
    this.note.textContent = "";
    this.box.focus();
    this.box.select();
  }

  close(): void {
    this.open = false;
    this.element.hidden = true;
    this.element.remove();
    this.box.value = "";
    this.note.textContent = "";
  }

  private async go(): Promise<void> {
    const typed = this.box.value.trim();
    if (typed === "" || !this.ask) return;
    this.note.textContent = "";
    try {
      // The same question a citation in a document asks, and the same failures:
      // `open_ref` is the window's word for *that mareh makom could not be
      // opened*, and a box that could not read what was typed is not a new kind
      // of trouble.
      const said = await this.ask(typed);
      if (this.open) this.note.textContent = said ?? "";
    } catch (e) {
      if (this.open) this.note.textContent = sayTrouble(e, "open_ref").said;
    }
  }
}
