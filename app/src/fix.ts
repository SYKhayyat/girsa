// Correcting a typo without leaving the line it is on.
//
// spec.md §7.5: *if correcting a typo is not a three-second interaction from
// where you are reading, nobody does it — including you.* So this is not a
// dialog and not a mode: it is a box that opens where the words are, with the
// words already in it, and Enter puts it away.
//
//     highlight the word  →  Ctrl+K  →  type it right  →  Enter
//
// Nothing here decides anything. Which letters of the file a highlight covers,
// whether a correction may be made at all and what it does to the line are all
// answered in Rust — see `girsa_app::fixing`.

import type { FixMark } from "./api.ts";
import { field } from "./controls.ts";

/** What the reader highlighted, and what is already on that line. */
export interface Correcting {
  at: string;
  fromChar: number;
  toChar: number;
  words: string;
  fixed: FixMark[];
  printed: string | null;
}

export interface FixHandlers {
  save: (now: string, kind: "ocr" | "girsa") => Promise<void>;
  revert: (patch: string) => Promise<void>;
}

export class FixBox {
  readonly element: HTMLElement;
  private input: HTMLInputElement | null = null;
  private kind: "ocr" | "girsa" = "ocr";
  private handlers: FixHandlers | null = null;
  private open = false;

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "fixbox";
    this.element.hidden = true;
    this.element.addEventListener("keydown", (event) => this.typed(event));
  }

  get isOpen(): boolean {
    return this.open;
  }

  /** Open on a highlight, at the words themselves. */
  show(what: Correcting, near: DOMRect | null, handlers: FixHandlers): void {
    this.handlers = handlers;
    this.kind = "ocr";
    this.element.replaceChildren();
    this.element.hidden = false;
    this.open = true;

    const head = document.createElement("p");
    head.className = "fixbox-head";
    head.textContent = "תיקון";
    const printed = document.createElement("span");
    printed.className = "fixbox-printed";
    printed.textContent = what.words;
    printed.title = "כפי שנדפס";
    head.append(printed);

    const input = field("התיקון");
    input.className = "fixbox-input";
    input.type = "text";
    input.dir = "rtl";
    input.value = what.words;
    input.spellcheck = false;
    this.input = input;

    // Which claim this is (spec.md §7.2). One mechanism, and the reader says
    // which of the two they mean — a scanning error is repaired, a variant is
    // noted beside the text and not applied to it.
    const kinds = document.createElement("div");
    kinds.className = "fixbox-kinds";
    const ocr = this.kindButton("טעות דפוס", "ocr", "השגיאה של הסורק — מתוקנת בגוף הטקסט");
    const girsa = this.kindButton("גרסה", "girsa", "כך גורס מישהו — נרשם ואינו מוחל");
    kinds.append(ocr, girsa);

    const hint = document.createElement("p");
    hint.className = "fixbox-hint";
    hint.textContent = "Enter — שמור · Esc — בטל";

    this.element.append(head, input, kinds, hint);

    // What is already here, with a way to take it back — which is the other
    // half of the overlay being an overlay (spec.md §7.1).
    for (const fix of what.fixed) {
      this.element.append(this.existing(fix));
    }
    if (what.printed) {
      const was = document.createElement("p");
      was.className = "fixbox-was";
      was.textContent = `כפי שנדפס: ${what.printed}`;
      this.element.append(was);
    }

    this.place(near);
    input.focus();
    input.select();
  }

  private kindButton(label: string, kind: "ocr" | "girsa", why: string): HTMLElement {
    const button = document.createElement("button");
    button.className = "fixbox-kind" + (this.kind === kind ? " is-on" : "");
    button.textContent = label;
    button.title = why;
    button.addEventListener("click", () => {
      this.kind = kind;
      for (const other of this.element.querySelectorAll(".fixbox-kind")) {
        other.classList.toggle("is-on", other === button);
      }
      this.input?.focus();
    });
    return button;
  }

  private existing(fix: FixMark): HTMLElement {
    const row = document.createElement("p");
    row.className = "fixbox-existing";
    const words = document.createElement("span");
    words.textContent = `${fix.kind === "ocr" ? "תוקן" : "גרסה"}: ${fix.was} ← ${fix.now}`;
    const back = document.createElement("button");
    back.className = "fixbox-back";
    back.textContent = "החזר";
    back.title = "בטל את התיקון — הטקסט חוזר כפי שנדפס";
    back.addEventListener("click", () => {
      const handlers = this.handlers;
      this.close();
      void handlers?.revert(fix.id);
    });
    row.append(words, back);
    return row;
  }

  private place(near: DOMRect | null): void {
    const box = this.element;
    box.style.visibility = "hidden";
    const width = box.offsetWidth || 280;
    const height = box.offsetHeight || 120;
    const top = near
      ? Math.min(Math.max(8, near.bottom + 6), window.innerHeight - height - 8)
      : 80;
    const left = near
      ? Math.min(Math.max(8, near.left - width / 2 + near.width / 2), window.innerWidth - width - 8)
      : window.innerWidth / 2 - width / 2;
    box.style.top = `${top}px`;
    box.style.left = `${left}px`;
    box.style.visibility = "visible";
  }

  private typed(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      this.close();
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      const now = this.input?.value ?? "";
      const handlers = this.handlers;
      const kind = this.kind;
      this.close();
      if (now.trim()) void handlers?.save(now, kind);
    }
  }

  close(): void {
    this.open = false;
    this.element.hidden = true;
    this.element.replaceChildren();
    this.input = null;
    this.handlers = null;
  }
}
