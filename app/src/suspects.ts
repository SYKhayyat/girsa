// The OCR queue: four thousand ranked candidates, and a way through them.
//
// spec.md §7.3, W21. *Fixing typos you trip over is nice; being handed 4,000
// ranked candidates is a different product.* So this is a list you work down,
// and every row has the two things a decision needs: what the word is, and what
// it is one letter away from — with how often each was seen, because a word
// seen once beside a word seen half a million times is a different claim from
// one seen once beside a word seen ten thousand times.
//
// **Nothing here corrects anything.** Opening a row takes you to the place with
// the word in front of you and the correction box open; the correction itself
// goes through the same path a correction made while reading does.

import { api, type SuspectRow } from "./api.ts";
import { fill, say } from "./say.ts";
import { dock, undock, wideAs } from "./dock.ts";
import { shut } from "./controls.ts";

/** How many to ask for. More than fits on a screen, few enough to draw. */
const PAGE = 60;

export class SuspectsView {
  readonly element: HTMLElement;
  private readonly list: HTMLElement;
  private readonly note: HTMLElement;
  private rows: SuspectRow[] = [];
  private goTo: ((row: SuspectRow) => Promise<void>) | null = null;

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  constructor() {
    this.element = document.createElement("section");
    this.element.className = "suspects";

    const head = document.createElement("header");
    head.className = "suspects-head";
    const title = document.createElement("span");
    title.className = "suspects-title";
    title.textContent = say("suspectsTitle");
    this.note = document.createElement("span");
    this.note.className = "suspects-note";
    const close = shut(() => this.close());
    head.append(title, this.note, close);

    this.list = document.createElement("div");
    this.list.className = "suspects-list";
    this.element.append(head, this.list);
  }

  /** The window says what opening a row means, because which tab a sefer opens
   * in is the window's business and not the drawer's. */
  onOpen(goTo: (row: SuspectRow) => Promise<void>): void {
    this.goTo = goTo;
  }

  async toggle(): Promise<void> {
    if (this.isOpen) {
      this.close();
      return;
    }
    await this.show();
  }

  async show(): Promise<void> {
  // Docked, not laid over the reading. The reading is made **narrower** and this
  // stands beside it — the same answer the bookcase and the search already give,
  // and the reader's complaint that produced it was about a panel exactly like
  // this one: *"it is weirdly over the text, so i cant see it or the text."*
    this.element.classList.add("is-open");
    dock("suspects", wideAs("--suspects-wide"));
    this.note.textContent = say("linksReading");
    this.rows = await api.suspects(PAGE);
    this.draw();
  }

  close(): void {
    this.element.classList.remove("is-open");
    undock("suspects");
  }

  private draw(): void {
    this.list.replaceChildren();
    if (this.rows.length === 0) {
      // Two different statements, and a list of nothing says the wrong one:
      // there is no queue until the batch job has been run.
      this.note.textContent = "";
      const empty = document.createElement("p");
      empty.className = "suspects-empty";
      empty.textContent =
        say("suspectsNoQueue");
      this.list.append(empty);
      return;
    }
    this.note.textContent = fill("suspectsInQueue", { count: this.rows.length });
    for (const row of this.rows) this.list.append(this.rowElement(row));
  }

  private rowElement(row: SuspectRow): HTMLElement {
    const line = document.createElement("div");
    line.className = "suspect";
    line.dataset.id = row.id;

    const words = document.createElement("span");
    words.className = "suspect-words";
    const rare = document.createElement("b");
    rare.textContent = row.rare;
    const arrow = document.createElement("span");
    arrow.className = "suspect-arrow";
    arrow.textContent = " ← ";
    const common = document.createElement("span");
    common.textContent = row.common;
    words.append(rare, arrow, common);

    const counts = document.createElement("span");
    counts.className = "suspect-counts";
    counts.textContent = `${row.rare_count} · ${row.common_count.toLocaleString("he-IL")}`;
    counts.title = say("suspectsCounts");

    const how = document.createElement("span");
    how.className = "suspect-how";
    how.textContent = row.confusion ?? said(row.how);
    how.title = row.confusion ? say("suspectsConfusion") : said(row.how);

    const where = document.createElement("span");
    where.className = "suspect-where";
    // A queue row names a sefer that may not be on this shelf at all, so the
    // title can be absent — and then the row says nothing rather than
    // `undefined`. Which of the sefer's two names it is was chosen in Rust (W41).
    where.textContent = row.title ? `${row.title} ${row.address ?? ""}` : "";

    const open = document.createElement("button");
    open.className = "tool";
    open.textContent = say("open");
    open.title = say("suspectsOpenWhy");
    open.disabled = !row.at;
    open.addEventListener("click", () => {
      void this.goTo?.(row);
    });

    const dismiss = document.createElement("button");
    dismiss.className = "tool";
    dismiss.textContent = say("suspectsNotAnError");
    dismiss.title = say("suspectsNotAnErrorWhy");
    dismiss.addEventListener("click", () => {
      void this.decide(row, line);
    });

    line.append(words, counts, how, where, open, dismiss);
    return line;
  }

  private async decide(row: SuspectRow, line: HTMLElement): Promise<void> {
    await api.suspectDecide(row.id, "dismissed");
    this.rows = this.rows.filter((other) => other.id !== row.id);
    line.remove();
    this.note.textContent = fill("suspectsInQueue", { count: this.rows.length });
  }

  /** A row was corrected. Take it off the list without re-reading the queue —
   * the reader is in the middle of working down it. */
  taken(id: string): void {
    this.rows = this.rows.filter((row) => row.id !== id);
    for (const line of this.list.querySelectorAll<HTMLElement>(".suspect")) {
      if (line.dataset.id === id) line.remove();
    }
    this.note.textContent = fill("suspectsInQueue", { count: this.rows.length });
  }
}

/** What the one edit was, in words. */
function said(how: SuspectRow["how"]): string {
  if (how === "letter") return say("suspectsSwapped");
  if (how === "added") return say("suspectsAdded");
  if (how === "dropped") return say("suspectsDropped");
  return say("suspectsTransposed");
}
