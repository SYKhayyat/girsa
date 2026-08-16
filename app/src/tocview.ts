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
  private readonly note: HTMLElement;
  private readonly filter: HTMLInputElement;
  private slug: string | null = null;
  private entries: TocEntry[] = [];
  /** Where the reader is, as an index into the sefer's segments. */
  private line = 0;
  private goTo: ((work: string, at: string) => Promise<void>) | null = null;
  private readonly draws = new Latest();

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
    this.filter.addEventListener("input", () => this.drawList());

    this.list = document.createElement("div");
    this.list.className = "toc-list";
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
    this.list.replaceChildren();
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
  }

  private drawList(): void {
    const shown = matching(this.entries, this.filter.value);
    this.list.replaceChildren();
    this.note.textContent =
      this.entries.length === 0
        ? say("tocNone")
        : `${shown.length.toLocaleString("he-IL")} / ${this.entries.length.toLocaleString("he-IL")}`;
    for (const entry of shown) {
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
      row.addEventListener("click", () => {
        const slug = this.slug;
        if (slug) void this.goTo?.(slug, entry.at);
      });
      this.list.append(row);
    }
    this.mark();
  }

  /** Mark where the reader is, and bring it into view. */
  private mark(): void {
    const at = inside(this.entries, this.line);
    const here = at < 0 ? null : this.entries[at];
    let marked: HTMLElement | null = null;
    for (const row of this.list.querySelectorAll<HTMLElement>(".toc-row")) {
      const on = !!here && row.dataset.at === here.at;
      row.classList.toggle("is-here", on);
      row.setAttribute("aria-current", on ? "true" : "false");
      if (on) marked = row;
    }
    // `nearest`, so a reader who is already looking at the right part of the
    // list is not scrolled at all. Otzaria centres it on every move, which
    // pulls the column out from under a pointer that was about to click.
    marked?.scrollIntoView({ block: "nearest" });
  }
}
