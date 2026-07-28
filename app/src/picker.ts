// Choosing a sefer.
//
// Two jobs, one overlay: open a sefer in a new tab, or put one in the column
// beside what you are reading. The second opens showing **what the corpus says
// belongs there** — the commentaries on this sefer, and the seforim the link
// graph joins it to — rather than 7,189 titles in alphabetical order, which is
// a list and not a choice.

import { api, type Card, type Companion } from "./api.ts";

type Chosen = (slug: string) => void;

export class Picker {
  readonly element: HTMLElement;
  private readonly input: HTMLInputElement;
  private readonly list: HTMLElement;
  private readonly heading: HTMLElement;
  private rows: { slug: string; node: HTMLElement }[] = [];
  private cursor = 0;
  private chosen: Chosen = () => {};
  private beside: string | null = null;

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "picker";
    this.element.hidden = true;

    const sheet = document.createElement("div");
    sheet.className = "picker-sheet";
    this.heading = document.createElement("p");
    this.heading.className = "picker-heading";
    this.input = document.createElement("input");
    this.input.className = "picker-input";
    this.input.type = "search";
    this.input.setAttribute("dir", "auto");
    this.list = document.createElement("ul");
    this.list.className = "picker-list";
    sheet.append(this.heading, this.input, this.list);
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

  /** Open a sefer beside the one already open. */
  openBeside(slug: string, title: string, chosen: Chosen): void {
    this.beside = slug;
    this.chosen = chosen;
    this.heading.textContent = `לצד ${title}`;
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
      this.fill(
        (await api.companions(this.beside)).map(companionRow),
        "אין ספר שהחיבור מעיד עליו — חפש אחד",
      );
      return;
    }
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
      const node = document.createElement("li");
      node.className = "picker-row";
      const title = document.createElement("span");
      title.className = "picker-row-title";
      title.textContent = row.title;
      const aside = document.createElement("span");
      aside.className = "picker-row-aside";
      aside.textContent = row.aside;
      if (row.why) aside.title = row.why;
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
function companionRow(companion: Companion): Row {
  return {
    slug: companion.slug,
    title: companion.he_title,
    aside: companion.declared ? "פירוש" : `${companion.links} קישורים`,
    why: companion.declared
      ? "the corpus declares this a commentary on what you are reading"
      : `${companion.links} links join the two; nothing declares a commentary`,
  };
}
