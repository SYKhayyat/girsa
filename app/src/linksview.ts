// The links on the line you are standing on, and what you can say about them.
//
// spec.md §8.3, W23. The panel exists because the data is wrong in known ways —
// 40% of it carries no type — and the person who can see that is the one
// reading both texts. So every row **shows its work**: which end, what the
// corpus said, how it was found, how much to believe it, and which of that was
// you.
//
// Nothing here decides anything. Whether a link may be shown as a statement
// about the texts (`curated`) is answered in Rust, because it is a rule about
// evidence and not about a stylesheet.

import { api, type LinkRow, type Links } from "./api.ts";

/** What each type is called on the page. */
const TYPES: Record<string, string> = {
  "comments-on": "מפרש",
  quotes: "מצטט",
  paraphrases: "מביא",
  codifies: "פוסק",
  disputes: "חולק",
  emends: "מגיה",
  "parallel-to": "מקביל",
  translates: "מתרגם",
  references: "קשור",
};

function said(kind: string): string {
  return TYPES[kind] ?? kind;
}

export class LinksView {
  readonly element: HTMLElement;
  private readonly list: HTMLElement;
  private readonly note: HTMLElement;
  private at: string | null = null;
  private goTo: ((work: string, at: string) => Promise<void>) | null = null;
  /** Where the reader is standing, for *reanchor to here* and *draw from here*. */
  private here: (() => string | null) | null = null;

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  constructor() {
    this.element = document.createElement("section");
    this.element.className = "links";

    const head = document.createElement("header");
    head.className = "links-head";
    const title = document.createElement("span");
    title.className = "links-title";
    title.textContent = "קישורים";
    this.note = document.createElement("span");
    this.note.className = "links-note";
    const close = document.createElement("button");
    close.className = "tool";
    close.textContent = "סגור";
    close.title = "Esc";
    close.addEventListener("click", () => this.close());
    head.append(title, this.note, close);

    this.list = document.createElement("div");
    this.list.className = "links-list";
    this.element.append(head, this.list);
  }

  onOpen(goTo: (work: string, at: string) => Promise<void>): void {
    this.goTo = goTo;
  }

  /** The window says where the reader is, because which pane is focused is the
   * window's business. */
  onHere(here: () => string | null): void {
    this.here = here;
  }

  async toggle(at: string | null): Promise<void> {
    if (this.isOpen) {
      this.close();
      return;
    }
    await this.show(at);
  }

  async show(at: string | null): Promise<void> {
    if (!at) return;
    this.at = at;
    this.element.classList.add("is-open");
    this.note.textContent = "קורא…";
    this.list.replaceChildren();
    await this.draw();
  }

  close(): void {
    this.element.classList.remove("is-open");
  }

  private async draw(): Promise<void> {
    if (!this.at) return;
    let found: Links;
    try {
      found = await api.links(this.at);
    } catch (e) {
      this.note.textContent = String(e);
      return;
    }
    const shown = found.links.filter((link) => !link.rejected);
    this.note.textContent =
      shown.length === 0 ? "אין קישורים לשורה זו" : `${shown.length} קישורים`;
    if (found.incoming_unknown) {
      // Two different statements, and a short list says the wrong one.
      const warn = document.createElement("p");
      warn.className = "links-warn";
      warn.textContent =
        "אין מטמון שכנים — הקישורים אל השורה הזאת אינם מוצגים. הרץ girsa-companions.";
      this.list.append(warn);
    }
    this.list.append(...found.links.map((link) => this.row(link, found.types)));
  }

  private row(link: LinkRow, types: string[]): HTMLElement {
    const row = document.createElement("div");
    row.className = "link" + (link.rejected ? " is-rejected" : "");
    if (link.mine) row.classList.add("is-mine");

    const kind = document.createElement("span");
    kind.className = "link-kind" + (link.curated ? "" : " is-uncurated");
    kind.textContent = said(link.kind);
    kind.title = link.curated
      ? "טענה על הטקסטים"
      : "הקורפוס לא אמר איזה קשר — לא מוצג כעובדה";

    const where = document.createElement("button");
    where.className = "link-where";
    where.textContent = `${link.outgoing ? "←" : "→"} ${link.said}`;
    where.title = "פתח את המקום";
    where.addEventListener("click", () => void this.goTo?.(link.work, link.at));

    // Its work, shown: how it was found, how much to believe it, what the
    // corpus called it, and what you have done to it.
    const work = document.createElement("span");
    work.className = "link-work";
    const bits = [`${Math.round(link.confidence * 100)}%`, link.method];
    if (link.label) bits.push(`"${link.label}"`);
    if (link.was && link.was !== link.kind) bits.push(`היה: ${said(link.was)}`);
    if (link.changed.length > 0) bits.push(link.changed.join(", "));
    if (link.who) bits.push(link.who);
    work.textContent = bits.join(" · ");

    row.append(kind, where, work, this.actions(link, types));
    return row;
  }

  private actions(link: LinkRow, types: string[]): HTMLElement {
    const box = document.createElement("span");
    box.className = "link-actions";

    if (link.rejected) {
      box.append(this.button("בטל דחייה", "החזר את הקישור", () => this.repair(link, "undo")));
      return box;
    }

    if (!link.confirmed) {
      box.append(
        this.button("אשר", "בדקתי — הקישור נכון", () => this.repair(link, "confirm")),
      );
    }
    box.append(this.button("דחה", "בדקתי — הקישור שגוי", () => this.repair(link, "reject")));

    const retype = document.createElement("select");
    retype.className = "link-retype";
    retype.title = "קבע את סוג הקשר";
    const keep = document.createElement("option");
    keep.textContent = "סוג…";
    keep.value = "";
    retype.append(keep);
    for (const type of types) {
      const option = document.createElement("option");
      option.value = type;
      option.textContent = said(type);
      option.selected = type === link.kind;
      retype.append(option);
    }
    retype.addEventListener("change", () => {
      if (retype.value) void this.repair(link, "retype", retype.value);
    });
    box.append(retype);

    // Reanchoring: onto the line the reader is standing on, which is the only
    // segment the window can name without asking a second question.
    box.append(
      this.button("העבר לכאן", "העבר את הקצה הזה לשורה שאתה עומד בה", async () => {
        const here = this.here?.();
        if (!here) return;
        try {
          await api.linkReanchor(link.edge, link.outgoing ? "to" : "from", here);
          await this.draw();
        } catch (e) {
          this.note.textContent = String(e);
        }
      }),
    );
    if (link.changed.length > 0 && !link.mine) {
      box.append(this.button("בטל", "בטל את מה שאמרת על הקישור", () => this.repair(link, "undo")));
    }
    return box;
  }

  private async repair(link: LinkRow, does: string, value?: string): Promise<void> {
    try {
      await api.linkRepair(link.edge, does, value);
      await this.draw();
    } catch (e) {
      this.note.textContent = String(e);
    }
  }

  private button(label: string, title: string, click: () => void): HTMLElement {
    const node = document.createElement("button");
    node.className = "tool";
    node.textContent = label;
    node.title = title;
    node.addEventListener("click", click);
    return node;
  }
}
