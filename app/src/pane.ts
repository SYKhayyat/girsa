// One column of reading.
//
// A pane holds a sefer, renders a window of it, and reports where the reader
// is. It does not decide where anything goes — it is told, by `main.ts`, which
// asks Rust.

import type { FixMark, Line, PaneId, Place, Relation, Run, Text } from "./api.ts";

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

  /**
   * What the reader has highlighted, as segment ids and character offsets
   * (spec.md §10.2 — *highlight part of a passage; only that goes*).
   *
   * The offsets are counted over the **text of the line as it stands in the
   * document**, which is the text Rust sent: markup already turned into runs,
   * nikud already applied. So `girsa_app::sending` can slice its own copy of
   * the same string and get the same words, without either side having to
   * describe a selection to the other.
   *
   * The address label in the margin is not part of the line's text, and is
   * deliberately excluded — a reader dragging across a se'if is not asking to
   * quote its number.
   *
   * `null` when nothing here is selected: that is the whole-line case, and it
   * is the caller's to decide, because "what is the reader standing on" is a
   * different question.
   */
  selection(): { from: string; to: string; fromChar: number; toChar: number } | null {
    const chosen = window.getSelection();
    if (!chosen || chosen.isCollapsed || chosen.rangeCount === 0) return null;
    const range = chosen.getRangeAt(0);
    const from = lineOf(range.startContainer);
    const to = lineOf(range.endContainer);
    if (!from || !to) return null;
    if (!this.body.contains(from) || !this.body.contains(to)) return null;

    const fromChar = offsetIn(from, range.startContainer, range.startOffset);
    const toChar = offsetIn(to, range.endContainer, range.endOffset);
    if (fromChar === null || toChar === null) return null;
    return {
      from: from.dataset.id ?? "",
      to: to.dataset.id ?? "",
      fromChar,
      toChar,
    };
  }

  /** The line the reader is standing on — the whole-line case for a copy. */
  here(): string | null {
    return this.topLine()?.dataset.id ?? null;
  }

  /**
   * One line, redrawn — after a correction (W20).
   *
   * The line is replaced where it stands rather than the sefer being rebuilt:
   * a reader who has just fixed a typo is looking at the word, and a rebuild
   * would take the page out from under them. spec.md §7.5 is a requirement
   * about how this feels, and this is most of what it costs here.
   */
  replaceLine(line: Line): void {
    if (!this.text) return;
    const at = this.byId.get(line.id);
    if (at === undefined) return;
    this.text.lines[at] = line;
    const drawn = this.body.querySelector<HTMLElement>(`[data-id="${cssEscape(line.id)}"]`);
    drawn?.replaceWith(lineElement(line));
  }

  /**
   * What the reader has highlighted, for a correction.
   *
   * `null` unless the highlight is inside one line: a patch names one segment
   * (spec.md §7.1), and a highlight running across three of them is not one
   * correction — it is three, and which words in which of them is a question
   * this window is not entitled to answer.
   */
  fixSelection(): {
    at: string;
    fromChar: number;
    toChar: number;
    words: string;
    fixed: FixMark[];
    printed: string | null;
  } | null {
    const chosen = this.selection();
    if (!chosen || chosen.from !== chosen.to || !this.text) return null;
    const at = this.byId.get(chosen.from);
    if (at === undefined) return null;
    const line = this.text.lines[at];
    const letters = Array.from(line.runs.map((run) => run.text).join(""));
    const words = letters.slice(chosen.fromChar, chosen.toChar).join("");
    if (!words.trim()) return null;
    return {
      at: chosen.from,
      fromChar: chosen.fromChar,
      toChar: chosen.toChar,
      words,
      fixed: line.fixed ?? [],
      printed: line.printed ?? null,
    };
  }

  /** The corrections on the line the reader is standing on. */
  fixesHere(): { at: string; fixed: FixMark[]; printed: string | null } | null {
    const here = this.here();
    if (!here || !this.text) return null;
    const at = this.byId.get(here);
    if (at === undefined) return null;
    const line = this.text.lines[at];
    return { at: here, fixed: line.fixed ?? [], printed: line.printed ?? null };
  }

  /**
   * Point at one word of one line, and say where it ended up on the screen.
   *
   * What the OCR queue needs (W21): the reader arrives at a line they have
   * never seen, and the word in question has to be the one their eye lands on.
   * The offsets are the ones Rust worked out — this only draws them.
   */
  markWord(id: string, fromChar: number, toChar: number): DOMRect | null {
    const at = this.byId.get(id);
    if (at === undefined) return null;
    if (at < this.from + 5 || at >= this.to - 5) this.render(at);
    const line = this.body.querySelector<HTMLElement>(`[data-id="${cssEscape(id)}"]`);
    const words = line?.querySelector<HTMLElement>(".line-text");
    if (!line || !words) return null;

    this.quiet = true;
    this.body.scrollTop +=
      line.getBoundingClientRect().top - this.body.getBoundingClientRect().top - 8;
    window.setTimeout(() => {
      this.quiet = false;
    }, 120);

    // A range over the characters themselves, so the mark is on the word and
    // not on the line — and so its rectangle is where the box opens.
    const range = charRange(words, fromChar, toChar);
    if (!range) return null;
    const chosen = window.getSelection();
    chosen?.removeAllRanges();
    chosen?.addRange(range);
    return range.getBoundingClientRect();
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
  if (line.fixed?.length) row.append(fixMark(line));
  return row;
}

/**
 * The mark on a line a correction touched (spec.md §7.1).
 *
 * Two shapes, because they are two different statements: a scanning error that
 * has been repaired, and a variant that is only noted. A reader has to be able
 * to tell at a glance whether the words in front of them are the printed ones.
 */
function fixMark(line: Line): HTMLElement {
  const fixed = line.fixed ?? [];
  const applied = fixed.filter((f) => f.applied);
  const mark = el("span", applied.length > 0 ? "line-fix" : "line-fix is-noted");
  mark.textContent = applied.length > 0 ? "✓" : "≠";
  const said = fixed
    .map((f) => {
      const claim = f.kind === "ocr" ? "תוקן" : "גרסה";
      const state = f.applied ? "" : " (לא הוחל)";
      const who = f.source ? ` · ${f.source}` : f.who ? ` · ${f.who}` : "";
      return `${claim}${state}: ${f.was} ← ${f.now}${who}`;
    })
    .join("\n");
  mark.title = line.printed ? `${said}\nכפי שנדפס: ${line.printed}` : said;
  return mark;
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

/**
 * A range over characters `from..to` of a line's words.
 *
 * Walked over the text nodes, because a line is runs and `<br>`s rather than
 * one string — the same reason `offsetIn` asks the document rather than adding
 * up lengths.
 */
function charRange(words: HTMLElement, from: number, to: number): Range | null {
  const walker = document.createTreeWalker(words, NodeFilter.SHOW_TEXT);
  const range = document.createRange();
  let seen = 0;
  let started = false;
  let node = walker.nextNode();
  while (node) {
    const length = node.textContent?.length ?? 0;
    if (!started && seen + length >= from) {
      range.setStart(node, from - seen);
      started = true;
    }
    if (started && seen + length >= to) {
      range.setEnd(node, to - seen);
      return range;
    }
    seen += length;
    node = walker.nextNode();
  }
  return started ? range : null;
}

function addressOf(line: HTMLElement): string {
  return line.dataset.address ?? "";
}

/** The `.line` a node sits inside, if it is inside one. */
function lineOf(node: Node): HTMLElement | null {
  const element = node.nodeType === Node.TEXT_NODE ? node.parentElement : (node as HTMLElement);
  return element?.closest<HTMLElement>(".line") ?? null;
}

/**
 * How many characters into a line's words a selection boundary is.
 *
 * Measured by asking the document, rather than by walking the runs and adding
 * up lengths: a range from the start of `.line-text` to the boundary knows
 * about every node between them, including the `<br>` a break run draws, and
 * cannot drift from what is on the screen.
 */
function offsetIn(line: HTMLElement, container: Node, offset: number): number | null {
  const words = line.querySelector<HTMLElement>(".line-text");
  if (!words) return null;
  if (!words.contains(container)) {
    // The boundary is in the margin label, or on the line element itself —
    // which is what a triple-click gives. Either way it means *this end of the
    // line*, and which end is told by where the other one is.
    return container === line && offset > 0 ? (words.textContent?.length ?? 0) : 0;
  }
  const upTo = document.createRange();
  upTo.setStart(words, 0);
  upTo.setEnd(container, offset);
  return upTo.toString().length;
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
