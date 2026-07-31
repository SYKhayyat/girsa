// Choosing a sefer.
//
// Two jobs, one overlay: open a sefer in a new tab, or put one in the column
// beside what you are reading. The second opens showing **what the corpus says
// belongs there** — the commentaries on this sefer, and the seforim the link
// graph joins it to — rather than 7,189 titles in alphabetical order, which is
// a list and not a choice.

import { api, type Card, type Mefarshim } from "./api.ts";
import { field } from "./controls.ts";
import { choices, listed, ticked, type Choice, type Listed } from "./mefarshim.ts";

type Chosen = (slug: string) => void;

/**
 * What the mefarshim door is opened with.
 *
 * Both jobs at once, because it is one list: `chosen` opens a sefer into the
 * column beside this one (the split), and `tick` marks a mefaresh's comments on
 * the daf. The reader asked for the second and asked to keep the first.
 */
export interface Beside {
  slug: string;
  title: string;
  chosen: Chosen;
  /** Who the link graph can place on this sefer's lines, and who is ticked. */
  mefarshim: Mefarshim;
  /** Tick or untick one. The caller redraws the markers. */
  tick: (work: string, on: boolean) => void;
}

export class Picker {
  readonly element: HTMLElement;
  private readonly input: HTMLInputElement;
  private readonly list: HTMLElement;
  private readonly heading: HTMLElement;
  private rows: { slug: string; node: HTMLElement }[] = [];
  private cursor = 0;
  private chosen: Chosen = () => {};
  private beside: string | null = null;
  private mefarshim: Mefarshim = { works: [], folders: [], marked: [], touched: 0 };
  private tick: (work: string, on: boolean) => void = () => {};
  /** The sentence under the list: how much of the sefer has commentary at all. */
  private readonly said: HTMLElement;

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "picker";
    this.element.hidden = true;

    const sheet = document.createElement("div");
    sheet.className = "picker-sheet";
    this.heading = document.createElement("p");
    this.heading.className = "picker-heading";
    // The second field the snapshot could not see.
    this.input = field("סינון הרשימה", {
      className: "picker-input",
      type: "search",
      dir: "auto",
    });
    this.list = document.createElement("ul");
    this.list.className = "picker-list";
    this.said = document.createElement("p");
    this.said.className = "picker-said";
    sheet.append(this.heading, this.input, this.list, this.said);
    this.element.append(sheet);

    this.input.addEventListener("input", () => void this.refresh());
    this.input.addEventListener("keydown", (event) => this.key(event));
    this.element.addEventListener("pointerdown", (event) => {
      if (event.target === this.element) this.close();
    });
  }

  /** Open a sefer in a new tab. */
  openTab(chosen: Chosen): void {
    this.beside = null;
    this.chosen = chosen;
    this.heading.textContent = "פתח ספר";
    this.show();
  }

  /** Open the mefarshim on the sefer already open: to read beside it, or to tick. */
  openBeside(open: Beside): void {
    this.beside = open.slug;
    this.chosen = open.chosen;
    this.mefarshim = open.mefarshim;
    this.tick = open.tick;
    this.heading.textContent = `מפרשים · ${open.title}`;
    this.show();
  }

  private show(): void {
    this.element.hidden = false;
    this.input.value = "";
    this.input.focus();
    void this.refresh();
  }

  close(): void {
    this.element.hidden = true;
  }

  get isOpen(): boolean {
    return !this.element.hidden;
  }

  private async refresh(): Promise<void> {
    const query = this.input.value.trim();
    if (query.length === 0 && this.beside) {
      // Sorted in `choices` rather than taken as it arrived: `companions()`
      // builds its list in two passes, and the same daf opened twice must offer
      // the same order. Mefarshim first — a reader who pressed this button came
      // for one.
      const rows = choices(await api.companions(this.beside), this.mefarshim.works);
      this.fill(
        listed(rows, this.mefarshim.folders).map(listedRow),
        "אין ספר שהחיבור מעיד עליו — חפש אחד",
      );
      this.said.textContent = ticked(
        this.mefarshim.touched,
        this.mefarshim.works.filter((w) => w.chosen).length,
      );
      return;
    }
    this.said.textContent = "";
    if (query.length === 0) {
      this.fill((await api.recent()).map(cardRow), "התחל להקליד שם של ספר");
      return;
    }
    this.fill((await api.search(query)).map(cardRow), "אין ספר בשם הזה");
  }

  private fill(rows: Row[], empty: string): void {
    this.list.replaceChildren();
    this.rows = [];
    if (rows.length === 0) {
      const none = document.createElement("li");
      none.className = "picker-empty";
      none.textContent = empty;
      this.list.append(none);
      return;
    }
    for (const row of rows) {
      // A folder heading (W44). Not a row you can choose: it is the shelf the
      // seforim under it stand on, and the arrow keys skip it for that reason.
      if (row.heading) {
        const head = document.createElement("li");
        head.className = "picker-folder";
        head.style.setProperty("--depth", String(row.heading.depth));
        head.textContent = row.heading.count > 0
          ? `${row.title} · ${row.heading.count}`
          : row.title;
        this.list.append(head);
        continue;
      }
      const node = document.createElement("li");
      node.className = "picker-row";
      if (row.depth) node.style.setProperty("--depth", String(row.depth));
      const title = document.createElement("span");
      title.className = "picker-row-title";
      title.textContent = row.title;
      const aside = document.createElement("span");
      aside.className = "picker-row-aside";
      aside.textContent = row.aside;
      if (row.why) aside.title = row.why;
      // The tick-box, on the rows that can carry one (W43). Its own control and
      // not part of the row's click, because the two do different things: ticking
      // marks this mefaresh on the daf and leaves the list open, clicking the row
      // opens the sefer in the column beside you and closes it.
      if (row.tick) {
        const tick = row.tick;
        // Through `field`, like every other control here: a checkbox with no name
        // is one of thirty unlabelled boxes to a screen reader, and B14's guard
        // in `sources.test.mjs` fails the build over exactly this.
        const box = field(`סמן את ${row.title}`, {
          type: "checkbox",
          className: "picker-tick",
        });
        box.checked = tick.on;
        box.title = "סמן כדי לראות מה כתב על השורות של הספר";
        box.addEventListener("click", (event) => {
          // Or the row's own handler would open it beside as well.
          event.stopPropagation();
          this.tick(row.slug, box.checked);
        });
        box.addEventListener("pointerdown", (event) => event.stopPropagation());
        node.append(box);
      }
      node.append(title, aside);
      node.addEventListener("pointerdown", () => this.take(row.slug));
      this.list.append(node);
      this.rows.push({ slug: row.slug, node });
    }
    this.cursor = 0;
    this.mark();
  }

  private mark(): void {
    this.rows.forEach((row, i) => row.node.classList.toggle("is-cursor", i === this.cursor));
    this.rows[this.cursor]?.node.scrollIntoView({ block: "nearest" });
  }

  private key(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      this.close();
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      this.cursor = Math.min(this.rows.length - 1, Math.max(0, this.cursor + step));
      this.mark();
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const row = this.rows[this.cursor];
      if (row) this.take(row.slug);
    }
  }

  private take(slug: string): void {
    this.close();
    this.chosen(slug);
  }
}

interface Row {
  slug: string;
  title: string;
  aside: string;
  why?: string;
  /** Present on a row that can be ticked, with whether it is (W43). */
  tick?: { on: boolean };
  /** Present on a folder heading rather than a sefer (W44). */
  heading?: { depth: number; count: number };
  /** How far in a sefer under a folder is drawn. */
  depth?: number;
}

/** One entry of the grouped list: a heading, or a sefer under one. */
function listedRow(entry: Listed): Row {
  if (entry.kind === "folder") {
    return {
      slug: "",
      title: entry.title,
      aside: "",
      heading: { depth: entry.depth, count: entry.count },
    };
  }
  return companionRow(entry.choice);
}

function cardRow(card: Card): Row {
  return {
    slug: card.slug,
    title: card.he_title,
    aside: [card.author, card.era].filter(Boolean).join(" · "),
  };
}

/**
 * A declared commentary and a sefer that merely shares edges are different
 * claims, and the row says which. `815 קישורים` is a count of links somebody
 * recorded; `פירוש` is the corpus stating that this sefer is a commentary on
 * that one. Collapsing the two into one ranking would present a tally as a
 * fact.
 */
function companionRow(companion: Choice): Row {
  return {
    slug: companion.slug,
    title: companion.he_title,
    aside: companion.declared
      ? "פירוש"
      : companion.links > 0
        ? `${companion.links} קישורים`
        : "מפרש",
    why: companion.declared
      ? "the corpus declares this a commentary on what you are reading"
      : companion.links > 0
        ? `${companion.links} links join the two; nothing declares a commentary`
        : "the link graph places this sefer's comments on lines of what you are reading",
    tick: companion.tickable ? { on: companion.chosen } : undefined,
  };
}
