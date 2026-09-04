// The table of contents of the sefer you are reading, to jump around in.
//
// > *"there should be a table of contents on the side for each sefer, so you
// > can jump around."*
//
// Nothing here decides what the contents *are*: `girsa_app::contents` builds
// them from the segments' own addresses, which is exact, and says why scanning
// the text for headings — which is how Otzaria does it — is a guess this corpus
// would fail. This file draws them and filters them.
//
// # What is taken from Otzaria
//
// Three things, and they are the three that make its TOC usable
// (`lib/text_book/view/toc_navigator_screen.dart`):
//
// 1. **A filter box over the whole tree**, matching at every depth, so finding
//    סימן פ"ט in seven hundred is typing rather than scrolling.
// 2. **The entry you are inside is marked**, so the panel says where you are and
//    not only where you could go.
// 3. **…and scrolled to**, because a mark you have to hunt for in a column of
//    seven hundred rows is a mark that is not there.
//
// One thing is not taken: Otzaria flattens the tree the moment you type, losing
// which chelek a siman is in. The depth is kept here — it is the only thing
// telling יורה דעה סימן א from אורח חיים סימן א in a Tur, and those are two
// different places with one name.

import { api, type TocEntry } from "./api.ts";
import { about, field, shut } from "./controls.ts";
import { dock, undock, wideAs } from "./dock.ts";
import { Latest } from "./latest.ts";
import { say } from "./say.ts";
import { sayTrouble } from "./trouble.ts";

/**
 * The height of one row of the table of contents, in CSS pixels.
 *
 * **A constant, enforced by the stylesheet.** `.toc-row` declares `height:
 * 27px` and `box-sizing: border-box`, so a row measures exactly this and the
 * windowing below can know where every row sits without measuring it. There is
 * no drift to fear: the *title* is one line by rule (`.toc-row-title` is
 * `nowrap` with an ellipsis), so no entry can ever make its row taller than
 * the constant says. If the two ever disagree, the rows overlap, which is a
 * bug a screenshot finds instantly.
 *
 * Why the whole list is not drawn instead of windowed: it is the same answer
 * `pane.ts` gives for the page — a table of contents that draws every entry
 * makes typing in its filter build 17,418 `<button>`s on the UI thread, which
 * is a visible freeze on the one seforim where a table of contents matters
 * most. The windowing below is that decision applied to a column of rows
 * instead of a column of lines.
 */
const ROW = 27;
/** The former top padding of `.toc-list`, folded into the row arithmetic. */
const TOP_PAD = 4;
/** The former bottom padding, for the same reason. */
const BOTTOM_PAD = 14;
/**
 * Rows kept rendered beyond the fold on both sides, so the first frame of a
 * scroll starts on a full page instead of drawing the next screen the frame
 * the reader is looking at it.
 */
const OVERSCAN = 8;
/**
 * The pause that means *the reader stopped typing* — the same 160ms the find
 * bar uses, which is a pause a person makes between words and not inside one.
 */
const SETTLED = 160;

/** Which stretch of a `total`-row list the viewport needs drawn. */
export interface TocWindow {
  /** The first row to draw, as an index into the list. */
  first: number;
  /** How many rows after it — `0` for an empty list. */
  count: number;
}

/**
 * The slice of a fixed-height list that fills a viewport.
 *
 * What keeps a redraw cheap: `count` comes from the viewport and not from
 * `total`, so a Mishnah Berurah-sized table costs the same to draw as a
 * forty-entry one — the number of `<button>`s in the DOM stops scaling with
 * the sefer, which is the whole of this issue.
 */
export function windowed(total: number, viewport: number, at: number): TocWindow {
  if (total === 0) return { first: 0, count: 0 };
  const last = total - 1;
  const first = Math.max(0, Math.min(at, last));
  const count = Math.min(total - first, Math.ceil(viewport / ROW) + 1 + 2 * OVERSCAN);
  return { first, count };
}

/** The row whose top edge is the first one at or above the viewport's top. */
export function firstRow(scrollTop: number): number {
  return Math.max(0, Math.floor((scrollTop - TOP_PAD) / ROW));
}

/** The y-offset of a row inside the list, pads included. */
export function rowTop(i: number): number {
  return TOP_PAD + i * ROW;
}

/**
 * The scroll position that brings row `i` fully into view — `null` when it
 * already is. This is what `scrollIntoView({ block: "nearest" })` answered
 * before the list was windowed, computed rather than asked for, because the
 * row to scroll to is not always in the document any more.
 */
export function nearestScroll(i: number, viewport: number, scrollTop: number): number | null {
  const top = rowTop(i);
  if (top < scrollTop) return top;
  const bottom = top + ROW;
  if (bottom > scrollTop + viewport) return bottom - viewport;
  return null;
}

/**
 * One row of the table of contents, as a button.
 *
 * Its own function because it is the unit of DOM work the windowing above
 * counts: `drawList` builds `windowed(…).count` of these and nothing else, and
 * `toc.test.mjs` measures one against a counting fake document so the fix has
 * a number attached to it rather than a claim.
 */
export function buildRow(entry: TocEntry, pick: (at: string) => void): HTMLButtonElement {
  const row = document.createElement("button");
  row.type = "button";
  row.className = "toc-row";
  row.style.setProperty("--depth", String(entry.depth));
  row.dataset.at = entry.at;
  const address = document.createElement("span");
  address.className = "toc-row-address";
  address.textContent = entry.address;
  row.append(address);
  if (entry.title) {
    const said = document.createElement("span");
    said.className = "toc-row-title";
    said.textContent = entry.title;
    row.append(said);
  }
  // The name is the whole row, not the number: a screen reader announcing
  // `סימן א'` over seven hundred rows announces nothing.
  row.setAttribute("aria-label", entry.title ? `${entry.address} — ${entry.title}` : entry.address);
  row.addEventListener("click", () => pick(entry.at));
  return row;
}

/**
 * The entry the reader is inside, given where they are in the sefer.
 *
 * **The last entry at or before the line**, which is what *inside* means in a
 * sefer: standing on se'if 4 of siman 12, the place you are in is siman 12, and
 * siman 12's row is the last one that began at or before you.
 *
 * A separate function because it is the one piece of arithmetic here and the
 * one thing that can be wrong in a way nobody would see — an off-by-one marks
 * the siman above the one you are reading, which reads as the panel lagging
 * rather than as a bug.
 *
 * `-1` is *before the first entry*, which is a real place: the front matter of
 * a sefer sits before its first siman.
 */
export function inside(entries: TocEntry[], line: number): number {
  let found = -1;
  for (const [at, entry] of entries.entries()) {
    if (entry.from > line) break;
    found = at;
  }
  return found;
}

/**
 * Which entries a typed filter keeps.
 *
 * Matched against the title **and** the address, because half this corpus names
 * nothing: filtering Berakhos by title would match no daf at all, and a reader
 * typing `ל.` is naming a place.
 *
 * Its own function for the same reason [`inside`] is: `app/test` has no DOM,
 * and a filter that quietly matches nothing is a panel that looks empty.
 */
export function matching(entries: TocEntry[], typed: string): TocEntry[] {
  const needle = typed.trim();
  if (needle === "") return entries;
  return entries.filter(
    (entry) => (entry.title ?? "").includes(needle) || entry.address.includes(needle),
  );
}

export class TocView {
  readonly element: HTMLElement;
  private readonly list: HTMLElement;
  private readonly fill: HTMLElement;
  private readonly note: HTMLElement;
  private readonly filter: HTMLInputElement;
  private slug: string | null = null;
  private entries: TocEntry[] = [];
  /** The entries after the typed filter — what the window is a window *of*. */
  private shown: TocEntry[] = [];
  /** Where each kept entry sits in [`shown`], so `mark` need not scan for it. */
  private readonly shownIndex = new Map<string, number>();
  /** Where the reader is, as an index into the sefer's segments. */
  private line = 0;
  private goTo: ((work: string, at: string) => Promise<void>) | null = null;
  private readonly draws = new Latest();
  /** The slice of `shown` currently in the DOM. `null` until first drawn. */
  private renderedAt: TocWindow | null = null;
  /** The keystroke that has not been searched for yet. */
  private waiting: number | null = null;

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  constructor() {
    this.element = document.createElement("section");
    this.element.className = "toc";

    const head = document.createElement("header");
    head.className = "toc-head";
    const title = document.createElement("span");
    title.className = "toc-title";
    title.textContent = say("tocTitle");
    this.note = document.createElement("span");
    this.note.className = "toc-note";
    head.append(title, this.note, shut(() => this.close()));

    this.filter = field(say("tocFilter"), {
      className: "toc-filter",
      type: "search",
      dir: "auto",
    });
    this.filter.addEventListener("input", () => this.typed());

    this.list = document.createElement("div");
    this.list.className = "toc-list";
    // Rows are absolutely positioned inside the fill, which is as tall as the
    // whole (filtered) list; the list itself scrolls that tall element. The
    // scroll listener redraws the window — the only thing that changes as a
    // reader scrolls is *which* slice is in the viewport.
    this.fill = document.createElement("div");
    this.fill.className = "toc-fill";
    this.list.append(this.fill);
    this.list.addEventListener("scroll", () => this.render());
    // A resized window is a different viewport height, so the window has to be
    // recomputed — the browser tells this panel when, rather than the panel
    // guessing on the next scroll.
    window.addEventListener("resize", () => {
      if (this.isOpen) this.render();
    });
    this.element.append(head, about(say("tocAbout")), this.filter, this.list);
  }

  onOpen(goTo: (work: string, at: string) => Promise<void>): void {
    this.goTo = goTo;
  }

  async toggle(slug: string | null, line: number): Promise<void> {
    if (this.isOpen) {
      this.close();
      return;
    }
    await this.show(slug, line);
  }

  async show(slug: string | null, line: number): Promise<void> {
    if (!slug) return;
    // A different sefer means a different table. The same one means the reader
    // has scrolled, and the list is already in hand.
    const changed = slug !== this.slug;
    this.slug = slug;
    this.line = line;
    this.element.classList.add("is-open");
    dock("toc", wideAs("--toc-wide"));
    if (!changed && this.entries.length > 0) {
      this.drawList();
      return;
    }
    this.note.textContent = say("tocReading");
    this.fill.textContent = "";
    this.renderedAt = null;
    this.shown = [];
    this.shownIndex.clear();
    await this.draws.attempt(
      () => api.seferContents(slug),
      (entries) => {
        this.entries = entries;
        this.drawList();
      },
      (e) => sayTrouble(this.note, e, "contents"),
    );
  }

  /**
   * The reader has moved. Mark the new place — **without redrawing the list**,
   * which would throw away a filter they had typed and scroll them back to the
   * top on every line they scroll past.
   */
  moved(slug: string | null, line: number): void {
    if (!this.isOpen || slug !== this.slug) return;
    this.line = line;
    this.mark();
  }

  close(): void {
    this.element.classList.remove("is-open");
    undock("toc");
    // A redraw the reader has stopped typing past is a redraw into a hidden
    // panel; the same clearing as `findhere.ts` gives its own bar.
    if (this.waiting !== null) window.clearTimeout(this.waiting);
    this.waiting = null;
  }

  /**
   * A keystroke in the filter box.
   *
   * Debounced, the same way the find bar debounces a query: every keystroke
   * used to redraw the list synchronously, which for a sefer that fills the
   * panel with thousands of rows is a freeze on the UI thread per character.
   * The windowing below makes each redraw cheap; the debounce makes typing
   * cheaper still, because the intermediate keystrokes never draw at all.
   */
  private typed(): void {
    if (this.waiting !== null) window.clearTimeout(this.waiting);
    this.waiting = window.setTimeout(() => {
      this.waiting = null;
      this.drawList();
    }, SETTLED);
  }

  private drawList(): void {
    const shown = matching(this.entries, this.filter.value);
    this.shown = shown;
    this.shownIndex.clear();
    for (let i = 0; i < shown.length; i += 1) this.shownIndex.set(shown[i]!.at, i);
    this.note.textContent =
      this.entries.length === 0
        ? say("tocNone")
        : `${shown.length.toLocaleString("he-IL")} / ${this.entries.length.toLocaleString("he-IL")}`;
    // A filter changes which rows exist, so the list starts at the top of the
    // answers — and the scrollbar, whose length was lying for the old list,
    // lies for the new one instead.
    this.list.scrollTop = 0;
    this.render();
    this.mark();
  }

  /**
   * Draw the slice of [`shown`] that the viewport can see.
   *
   * The heart of the fix: **the slice is bounded by the viewport, not by the
   * sefer.** `buildRow` is called `windowed(…).count` times, which is tens of
   * rows whether the table has forty entries or Mishnah Berurah's seventeen
   * thousand — so a redraw costs the same on either. A row further down the
   * list exists only while the reader is looking near it, and `scrolled`
   * rebuilds the window the frame they arrive.
   *
   * One cost of windowing to be honest about: a keyboard tabbing through the
   * list can only reach the rows that exist. The filter is how a keyboard
   * reader finds a row on a large sefer, which is the thing a table of
   * contents is for — so the windowing trades a fringe interaction for the
   * interaction the panel is built around.
   */
  private render(): void {
    const want = windowed(this.shown.length, this.list.clientHeight, firstRow(this.list.scrollTop));
    if (
      this.renderedAt !== null &&
      this.renderedAt.first === want.first &&
      this.renderedAt.count === want.count
    ) {
      return;
    }
    this.renderedAt = want;
    this.fill.style.height = `${TOP_PAD + this.shown.length * ROW + BOTTOM_PAD}px`;
    const pick = (at: string): void => {
      const slug = this.slug;
      if (slug) void this.goTo?.(slug, at);
    };
    const rows: HTMLButtonElement[] = [];
    for (let i = want.first; i < want.first + want.count; i += 1) {
      const row = buildRow(this.shown[i]!, pick);
      row.style.top = `${rowTop(i)}px`;
      rows.push(row);
    }
    this.fill.replaceChildren(...rows);
  }

  /** Mark where the reader is, and bring it into view. */
  private mark(): void {
    const at = inside(this.entries, this.line);
    const here = at < 0 ? null : this.entries[at];
    const i = here === null ? null : this.shownIndex.get(here.at);
    // The row may not be in the document — it is a window of them now — so
    // `scrollIntoView` is answered by arithmetic: scroll the list so the row's
    // box is visible, then draw that window. `nearestScroll` answers `null`
    // when it already is, which is the ordinary case of scrolling within a
    // siman and costs nothing.
    if (here !== null && i !== null && i !== undefined) {
      const move = nearestScroll(i, this.list.clientHeight, this.list.scrollTop);
      if (move !== null) this.list.scrollTop = move;
    }
    this.render();
    for (const row of this.fill.querySelectorAll<HTMLElement>(".toc-row")) {
      const on = here !== null && row.dataset.at === here.at;
      row.classList.toggle("is-here", on);
      row.setAttribute("aria-current", on ? "true" : "false");
    }
  }
}
