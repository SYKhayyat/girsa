// One column of reading.
//
// A pane holds a sefer, renders a window of it, and reports where the reader
// is. It does not decide where anything goes — it is told, by `main.ts`, which
// asks Rust.

import type { Line, PaneId, Place, Relation, Run, Text } from "./api.ts";

/** How many lines are put on the page at once, and how many more at an edge. */
const WINDOW = 400;
const STEP = 300;

/**
 * A sefer is up to eighteen thousand segments (Mishnah Berurah), and putting
 * all of them in the document makes opening one feel like waiting. So a window
 * of lines around where the reader is goes in, and it grows when they reach an
 * edge — the scrollbar tells a small lie about the length of the sefer, which
 * is the same lie every reader in the world is already used to from a book.
 */
export class PaneView {
  readonly id: PaneId;
  readonly slug: string;
  readonly element: HTMLElement;
  private readonly body: HTMLElement;
  private readonly note: HTMLElement;
  private readonly title: HTMLElement;
  private readonly where: HTMLElement;
  private text: Text | null = null;
  private from = 0;
  private to = 0;
  private byId = new Map<string, number>();
  private highlighted: string[] = [];
  /** Set while a following pane is being moved, so its own scroll handler does
   * not report the move back and start the two panes chasing each other. */
  private quiet = false;

  constructor(
    id: PaneId,
    slug: string,
    private readonly onMove: (pane: PaneId, at: string) => void,
    private readonly onFocus: (pane: PaneId) => void,
  ) {
    this.id = id;
    this.slug = slug;
    this.element = el("section", "pane");
    this.element.dataset.pane = String(id);

    const header = el("header", "pane-head");
    this.title = el("span", "pane-title");
    this.where = el("span", "pane-where");
    this.note = el("span", "pane-note");
    header.append(this.title, this.where, this.note);
    this.element.append(header);

    this.body = el("div", "pane-body");
    this.body.tabIndex = 0;
    this.element.append(this.body);

    this.body.addEventListener("scroll", () => this.scrolled(), { passive: true });
    this.body.addEventListener("pointerdown", () => this.onFocus(this.id));
    this.body.addEventListener("focus", () => this.onFocus(this.id));
  }

  /** The buttons a pane's header carries. Added by the caller, which owns what
   * they do. */
  addControl(control: HTMLElement): void {
    this.element.querySelector(".pane-head")?.append(control);
  }

  /** The sefer's Hebrew title, once it has been read. */
  title_he = "";

  show(text: Text, at: string | null): void {
    this.text = text;
    this.title_he = text.work.he_title;
    this.byId = new Map(text.lines.map((line, i) => [line.id, i]));
    this.title.textContent = text.work.he_title;
    this.title.title = text.work.en_title;
    const start = at ? (this.byId.get(at) ?? 0) : 0;
    this.render(start);
    if (at) this.scrollTo([at], false);
  }

  /** Put a window of lines centred on `index` into the document. */
  private render(index: number): void {
    if (!this.text) return;
    const lines = this.text.lines;
    this.from = Math.max(0, index - WINDOW / 2);
    this.to = Math.min(lines.length, this.from + WINDOW);
    this.body.replaceChildren(...lines.slice(this.from, this.to).map(lineElement));
  }

  private extend(where: "up" | "down"): void {
    if (!this.text) return;
    const lines = this.text.lines;
    if (where === "down" && this.to < lines.length) {
      const next = Math.min(lines.length, this.to + STEP);
      this.body.append(...lines.slice(this.to, next).map(lineElement));
      this.to = next;
    } else if (where === "up" && this.from > 0) {
      const next = Math.max(0, this.from - STEP);
      const before = this.body.scrollHeight;
      this.body.prepend(...lines.slice(next, this.from).map(lineElement));
      this.from = next;
      // Adding lines above moves everything down; put the reader back on the
      // words they were looking at.
      this.body.scrollTop += this.body.scrollHeight - before;
    }
  }

  private scrolled(): void {
    if (this.body.scrollTop < 400) this.extend("up");
    if (this.body.scrollHeight - this.body.scrollTop - this.body.clientHeight < 600) {
      this.extend("down");
    }
    if (this.quiet) return;
    const top = this.topLine();
    if (top) {
      this.where.textContent = addressOf(top);
      this.onMove(this.id, top.dataset.id ?? "");
    }
  }

  /** The first line whose text is actually in view. */
  private topLine(): HTMLElement | null {
    const top = this.body.getBoundingClientRect().top;
    for (const child of this.body.children) {
      const box = child.getBoundingClientRect();
      if (box.bottom > top + 8) return child as HTMLElement;
    }
    return null;
  }

  /** Move this pane because the pane it follows moved. */
  goTo(place: Place, relation: Relation): void {
    this.note.className = "pane-note";
    if (place.kind === "unrelated") {
      this.note.textContent = "";
      return;
    }
    if (place.kind === "no_place") {
      // The honest answer, and the reason this app does not scroll to the
      // nearest thing: there is no comment on this line, and the column stays
      // where it is rather than showing a comment on a different one.
      this.note.textContent = "אין כאן";
      this.note.title = "nothing in this sefer sits on that line";
      this.note.classList.add("is-empty");
      this.highlight([]);
      return;
    }
    this.note.textContent =
      typeof relation === "object" ? "" : relation === "linked" ? "מקושר" : "";
    this.scrollTo(place.ids, true);
  }

  private scrollTo(ids: string[], mark: boolean): void {
    if (!this.text || ids.length === 0) return;
    const index = this.byId.get(ids[0]);
    if (index === undefined) return;
    if (index < this.from + 5 || index >= this.to - 5) this.render(index);

    const target = this.body.querySelector<HTMLElement>(`[data-id="${cssEscape(ids[0])}"]`);
    if (!target) return;
    this.quiet = true;
    this.body.scrollTop += target.getBoundingClientRect().top - this.body.getBoundingClientRect().top - 8;
    this.where.textContent = addressOf(target);
    if (mark) this.highlight(ids);
    // One frame is not enough on a long list; the scroll event lands after.
    window.setTimeout(() => {
      this.quiet = false;
    }, 120);
  }

  private highlight(ids: string[]): void {
    for (const id of this.highlighted) {
      this.body
        .querySelector<HTMLElement>(`[data-id="${cssEscape(id)}"]`)
        ?.classList.remove("is-here");
    }
    this.highlighted = ids;
    for (const id of ids) {
      this.body
        .querySelector<HTMLElement>(`[data-id="${cssEscape(id)}"]`)
        ?.classList.add("is-here");
    }
  }

  setFocused(on: boolean): void {
    this.element.classList.toggle("is-focused", on);
  }

  /** A word about who this pane is following, in its header. */
  setFollowing(label: string): void {
    let chip = this.element.querySelector<HTMLElement>(".pane-follows");
    if (!chip) {
      chip = el("span", "pane-follows");
      this.element.querySelector(".pane-head")?.append(chip);
    }
    chip.textContent = label;
  }
}

function lineElement(line: Line): HTMLElement {
  const row = el("p", line.kind === "heading" ? "line is-heading" : "line");
  row.dataset.id = line.id;
  row.dataset.address = line.address;
  const label = el("span", "line-address");
  label.textContent = line.address;
  const words = el("span", "line-text");
  words.append(...line.runs.map(runElement));
  row.append(label, words);
  return row;
}

/** One run of words. Built as elements, never as a string of HTML — the text
 * comes out of a file and nothing that came out of a file is put into the
 * document as markup. */
function runElement(run: Run): Node {
  if (run.style === "break") return document.createElement("br");
  if (run.style === "plain") return document.createTextNode(run.text);
  const node = document.createElement("span");
  node.className = run.style === "opening" ? "run-opening" : "run-quiet";
  node.textContent = run.text;
  return node;
}

function addressOf(line: HTMLElement): string {
  return line.dataset.address ?? "";
}

function el(tag: string, className: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  return node;
}

/** A segment id carries `:` and `#`, which are CSS selector syntax. */
function cssEscape(value: string): string {
  return CSS.escape(value);
}
