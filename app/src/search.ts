// The search bar: the query, the chips under it, the results, and the facets
// beside them (spec.md §9.5, §9.6, §9.8 — BUILDER.md W14).
//
// Nothing here decides anything. Which chips exist, what they can be set to,
// what a facet row means and what clicking it narrows by are all answered in
// `girsa-search`, where they are tested; this file draws them and sends the
// clicks back. That division is the same one the rest of the window keeps, and
// it matters most here: a webview that worked out its own chip row would be a
// second opinion about what the engine can do, and the first thing a reader
// would notice is a chip that lies.
//
// Three things it is careful about, all of them §9's governing rule — *the
// engine never changes your query without you knowing*:
//
//   · the header says what was searched for, and it comes from the search that
//     ran rather than from the box;
//   · a zero shows the ladder with counts and applies **nothing** until it is
//     clicked;
//   · a refusal is shown as a refusal, with its reason, and never as an empty
//     list of results.

import { api, type Chip, type Dimension, type FacetRow, type Found, type Run } from "./api.ts";

/** Open a sefer at a segment — or, with no id, at wherever it was left.
 *
 * The second is W26's *read them*: a scan with no words has no segment worth
 * landing on, and the pane it opens is the one carrying the control that reads
 * it. */
type Opened = (slug: string, id: string | null, marked?: string[]) => void;

/** The facets, in the order spec.md §9.8 lists them. */
const FACETS: { dimension: Dimension; label: string }[] = [
  { dimension: "shelf", label: "מדף" },
  { dimension: "era", label: "תקופה" },
  { dimension: "author", label: "מחבר" },
  { dimension: "sefer", label: "ספר" },
  { dimension: "link", label: "קישור" },
];

/** How many rows of one facet before the rest are counted rather than listed. */
const ROWS = 8;

export class SearchView {
  readonly element: HTMLElement;
  private readonly box: HTMLInputElement;
  private chipRow: HTMLElement;
  private readonly head: HTMLElement;
  private readonly list: HTMLElement;
  private readonly rail: HTMLElement;
  private open = false;
  private page = 1;
  /** The rung the reader clicked, if any. Cleared by anything that makes it a
   * different search — which is the undo of spec.md §9.6. */
  private rung: string | null = null;
  private opened: Opened = () => {};

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "find";
    this.element.hidden = true;

    const sheet = document.createElement("div");
    sheet.className = "find-sheet";

    const bar = document.createElement("div");
    bar.className = "find-bar";
    this.box = document.createElement("input");
    this.box.className = "find-box";
    this.box.dir = "rtl";
    this.box.placeholder = "חפש בכל המדף…";
    this.box.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        // A new query is a new search: a rung applied to the last one does not
        // carry over silently.
        this.page = 1;
        this.rung = null;
        void this.run();
      }
    });
    // Keep the question, not the answer (spec.md §11, W27): the corpus grows
    // and your own seforim go on the shelf, so what is worth keeping is the
    // asking. The chips and the scope are read off the engine, not off this
    // box — see `query_keep`.
    const keep = document.createElement("button");
    keep.className = "tool find-keep";
    keep.textContent = "שמור";
    keep.title = "keep this question";
    keep.addEventListener("click", () => void this.keep?.(this.box.value.trim()));

    const close = document.createElement("button");
    close.className = "tool find-close";
    close.textContent = "סגור";
    close.title = "Esc";
    close.addEventListener("click", () => this.close());
    bar.append(this.box, keep, close);

    this.chipRow = document.createElement("div");
    this.chipRow.className = "find-chips";
    this.head = document.createElement("div");
    this.head.className = "find-head";

    const body = document.createElement("div");
    body.className = "find-body";
    this.list = document.createElement("div");
    this.list.className = "find-list";
    this.rail = document.createElement("div");
    this.rail.className = "find-rail";
    body.append(this.list, this.rail);

    sheet.append(bar, this.chipRow, this.head, body);
    this.element.append(sheet);
    this.element.addEventListener("pointerdown", (event) => {
      if (event.target === this.element) this.close();
    });
  }

  get isOpen(): boolean {
    return this.open;
  }

  private keep: ((typed: string) => Promise<void>) | null = null;

  /** The window says what *keep this question* does — asking a reader for a
   * name is the window's business, not this view's. */
  onKeep(keep: (typed: string) => Promise<void>): void {
    this.keep = keep;
  }

  /** Ask something again, with the chips already set back by `query_recall`. */
  async askAgain(opened: Opened, typed: string): Promise<void> {
    this.opened = opened;
    this.open = true;
    this.element.hidden = false;
    this.box.value = typed;
    this.page = 1;
    this.rung = null;
    this.box.focus();
    await this.run();
  }

  async show(opened: Opened): Promise<void> {
    this.opened = opened;
    this.open = true;
    this.element.hidden = false;
    this.box.focus();
    this.box.select();
    // The chips are drawn before anything has been searched for, because they
    // are what the search *will* do — a control a reader can only see after
    // they have already used it is not a control.
    await this.run(true);
  }

  close(): void {
    this.open = false;
    this.element.hidden = true;
  }

  async toggle(opened: Opened): Promise<void> {
    if (this.open) this.close();
    else await this.show(opened);
  }

  /**
   * Open with a phrase already in the box, as one thing (W18).
   *
   * The quotation marks are the sigil for *one after the other* (§9.5), so
   * this arrives as a phrase search and the chip says so — the reader sees
   * what was asked, not a query that behaves differently from how it reads.
   */
  async showPhrase(opened: Opened, phrase: string): Promise<void> {
    this.opened = opened;
    this.open = true;
    this.element.hidden = false;
    this.box.value = `"${phrase.trim()}"`;
    this.box.focus();
    await this.run();
  }

  /** Ask, and draw what came back. */
  private async run(chipsOnly = false): Promise<void> {
    const typed = this.box.value.trim();
    if (chipsOnly && typed === "") {
      const empty = await api.find("", 1);
      this.drawChips(empty.chips);
      this.head.textContent = "";
      this.list.replaceChildren();
      this.rail.replaceChildren();
      return;
    }
    if (typed === "") return;
    const found = this.rung
      ? await api.findRung(typed, this.page, this.rung)
      : await api.find(typed, this.page);
    // A sigil sets a chip; the box shows what is left, so what is on screen is
    // what was searched for.
    this.drawChips(found.chips);
    this.drawHead(found);
    this.drawHits(found);
    this.drawRail(found);
  }

  private drawChips(chips: Chip[]): void {
    const row = document.createElement("div");
    row.className = "find-chips";
    for (const chip of chips) {
      row.append(this.chip(chip));
    }
    this.chipRow.replaceWith(row);
    this.chipRow = row;
  }

  /** One chip: what it is set to, and every other thing it could be set to. */
  private chip(chip: Chip): HTMLElement {
    const shown = chip.choices.find((c) => c.chosen) ?? chip.choices[0];
    const wrap = document.createElement("div");
    wrap.className = "find-chip";

    const button = document.createElement("button");
    button.className = "find-chip-face";
    button.textContent = `${shown?.label ?? chip.name} ▾`;
    button.title = chip.name;
    const menu = document.createElement("div");
    menu.className = "find-chip-menu";
    menu.hidden = true;

    for (const choice of chip.choices) {
      const item = document.createElement("button");
      item.className = "find-chip-item" + (choice.chosen ? " is-chosen" : "");
      item.textContent = choice.label;
      if (choice.sigil) {
        const sigil = document.createElement("span");
        sigil.className = "find-sigil";
        // The sigil is shown **on** the chip, which is how §9.5's *the power
        // syntax teaches itself* actually happens: you click it once and see
        // what you could have typed.
        sigil.textContent = choice.sigil;
        item.append(sigil);
      }
      item.addEventListener("click", async () => {
        menu.hidden = true;
        if (chip.name === "where") {
          await api.findWholeShelf();
        } else {
          await api.findChip(chip.name, choice.key);
        }
        this.page = 1;
        this.rung = null;
        await this.run();
      });
      menu.append(item);
    }
    // The scope chip is not a list of options — it is what the facets set — so
    // its one item is the way back to the whole shelf.
    if (chip.name === "where") {
      menu.replaceChildren();
      const all = document.createElement("button");
      all.className = "find-chip-item";
      all.textContent = "כל המדף";
      all.addEventListener("click", async () => {
        menu.hidden = true;
        await api.findWholeShelf();
        this.page = 1;
        await this.run();
      });
      menu.append(all);
    }

    button.addEventListener("click", () => {
      menu.hidden = !menu.hidden;
    });
    wrap.append(button, menu);
    return wrap;
  }

  private drawHead(found: Found): void {
    this.head.replaceChildren();
    this.head.classList.toggle("is-trouble", Boolean(found.refused));
    if (found.refused) {
      this.head.textContent = found.refused;
      return;
    }
    const said = document.createElement("p");
    said.className = "find-said";
    said.textContent = found.header;
    const count = document.createElement("p");
    count.className = "find-count";
    count.textContent =
      found.pages > 1
        ? `${found.total} · עמוד ${found.page} מתוך ${found.pages}`
        : `${found.total}`;
    this.head.append(said, count);
    // What this search could not see (spec.md §9.7, W26). Drawn on every
    // result page, above the note and the offers, because it is a statement
    // about the answer and not a suggestion about the query: a reader given
    // forty hits over a shelf holding four unread scans has been told *these
    // are the forty places this appears*, and the forty-first is on a page
    // nobody has read.
    this.head.append(this.gapLine());
    if (found.note) {
      const note = document.createElement("p");
      note.className = "find-note";
      note.textContent = found.note;
      this.head.append(note);
    }
    // One click back to the literal query (spec.md §9.6 — reversibly).
    if (this.rung) {
      const undo = document.createElement("button");
      undo.className = "find-offer";
      undo.textContent = "בטל";
      undo.addEventListener("click", async () => {
        this.rung = null;
        this.page = 1;
        await this.run();
      });
      this.head.append(undo);
    }
    // The ladder: counts worked out before the click, and nothing applied.
    for (const offer of found.offers) {
      const chip = document.createElement("button");
      chip.className = "find-offer";
      chip.textContent = `${offer.label} — ${offer.count}`;
      chip.addEventListener("click", async () => {
        // The click. Until here nothing has been applied — the count beside the
        // offer was worked out from this very query, before it was asked for.
        this.rung = offer.rung;
        this.page = 1;
        await this.run();
      });
      this.head.append(chip);
    }
  }

  /**
   * *4 PDFs on this shelf aren't searchable yet — [קרא אותם]*.
   *
   * The sentence is composed in Rust (`girsa_app::reading::Gap::said`) so the
   * header, the CLI and the test cannot drift; the button opens the first of
   * the scans, where the *read* control on its pane is. Empty until the answer
   * arrives, and empty forever when there is nothing to say — which is a
   * different silence from the one this exists to prevent.
   */
  private gapLine(): HTMLElement {
    const line = document.createElement("p");
    line.className = "find-gap";
    void api
      .scanGap()
      .then((gap) => {
        if (!gap) return;
        const said = document.createElement("span");
        said.textContent = gap.said;
        line.append(said);
        const open = document.createElement("button");
        open.className = "find-offer";
        open.textContent = "קרא אותם";
        open.title = gap.scans.map((s) => `${s.title} — ${s.read}/${s.pages}`).join("\n");
        open.addEventListener("click", () => {
          const first = gap.scans[0];
          if (!first) return;
          this.close();
          this.opened(first.slug, null);
        });
        line.append(open);
      })
      .catch(() => undefined);
    return line;
  }

  private drawHits(found: Found): void {
    this.list.replaceChildren();
    if (found.landing) {
      this.list.append(this.landing(found));
      return;
    }
    for (const hit of found.hits) {
      const row = document.createElement("button");
      row.className = "find-hit";
      const where = document.createElement("span");
      where.className = "find-where";
      where.textContent =
        hit.page === null ? `${hit.he_title} ${hit.address}` : `${hit.he_title} — עמוד ${hit.page}`;
      const text = document.createElement("p");
      text.className = "find-text";
      text.append(...runs(hit.runs));
      row.append(where);
      // The badge (spec.md §9.7). **Badge them, don't demote them**: the row is
      // where the score put it, and this says what kind of reading it is. The
      // two badges are not the same claim — a file that carries its own text
      // said what its words are, and an engine guessed at a photograph, and the
      // measurement in `girsa-scan/src/engine.rs` puts those forty points of
      // precision apart.
      if (hit.by !== null) {
        const badge = document.createElement("span");
        badge.className = hit.guessed ? "find-badge is-guessed" : "find-badge";
        badge.textContent = hit.guessed ? "OCR" : "סריקה";
        badge.title = hit.guessed
          ? `נקרא במכונה (${hit.by}) — יש לבדוק מול הצילום`
          : "המילים מתוך הקובץ עצמו";
        row.append(badge);
      }
      row.append(text);
      row.addEventListener("click", () => {
        this.close();
        this.opened(hit.work, hit.id, hit.marked);
      });
      this.list.append(row);
    }
    if (found.pages > found.page) {
      const more = document.createElement("button");
      more.className = "tool find-more";
      more.textContent = "עוד";
      more.addEventListener("click", async () => {
        this.page += 1;
        await this.run();
      });
      this.list.append(more);
    }
  }

  /** A mareh makom: where it lands, or the candidates it could be. */
  private landing(found: Found): HTMLElement {
    const box = document.createElement("div");
    box.className = "find-landing";
    const said = document.createElement("p");
    said.className = "find-said";
    said.textContent = found.landing?.said ?? "";
    box.append(said);
    for (const place of found.landing?.places ?? []) {
      const row = document.createElement("button");
      row.className = "find-hit";
      row.textContent = place.reference;
      row.addEventListener("click", () => {
        this.close();
        this.opened(place.work, place.id);
      });
      box.append(row);
    }
    // Offered, never taken. Every one of these is something the shelf could not
    // rule out, or a spelling close to what was typed.
    for (const near of found.landing?.near ?? []) {
      const note = document.createElement("p");
      note.className = "find-near";
      note.textContent = near;
      box.append(note);
    }
    return box;
  }

  private drawRail(found: Found): void {
    this.rail.replaceChildren();
    if (!found.facets || found.total === 0) return;

    for (const { dimension, label } of FACETS) {
      const group = document.createElement("div");
      group.className = "find-facet";
      const title = document.createElement("p");
      title.className = "find-facet-title";
      title.textContent = label;
      group.append(title);

      if (dimension === "link" && found.facets.link.state === "not_built") {
        const why = document.createElement("p");
        why.className = "find-facet-none";
        // Never a silent gap: *nothing is commented on* and *nobody worked out
        // what is commented on* are different statements (spec.md §9.7's rule,
        // one facet over).
        why.textContent = "לא נבנה — הרץ girsa-link-types ובנה אינדקס מחדש";
        group.append(why);
        this.rail.append(group);
        continue;
      }

      const rows: FacetRow[] =
        dimension === "link"
          ? found.facets.link.state === "counted"
            ? found.facets.link.rows
            : []
          : found.facets[dimension];
      if (rows.length === 0) continue;

      for (const row of rows.slice(0, ROWS)) {
        group.append(this.facetRow(dimension, row));
      }
      if (rows.length > ROWS) {
        const more = document.createElement("p");
        more.className = "find-facet-none";
        more.textContent = `ועוד ${rows.length - ROWS}`;
        group.append(more);
      }
      this.rail.append(group);
    }

    if (found.facets.uncatalogued > 0) {
      const note = document.createElement("p");
      note.className = "find-facet-none";
      note.textContent = `${found.facets.uncatalogued} תוצאות בספרים שאינם בקטלוג — המדפים שלמעלה חסרים אותן`;
      this.rail.append(note);
    }
  }

  /** One facet row: its count, and the two clicks on it. */
  private facetRow(dimension: Dimension, row: FacetRow): HTMLElement {
    const line = document.createElement("div");
    line.className = "find-facet-row";
    line.style.paddingInlineStart = `${row.depth * 0.75}rem`;

    const narrow = document.createElement("button");
    narrow.className = "find-facet-narrow";
    narrow.textContent = row.label;
    narrow.title = `צמצם ל־${row.label}`;
    narrow.addEventListener("click", async () => {
      await api.findNarrow(dimension, row, false);
      this.page = 1;
      await this.run();
    });

    const count = document.createElement("span");
    count.className = "find-facet-count";
    count.textContent = String(row.count);

    const out = document.createElement("button");
    out.className = "find-facet-out";
    out.textContent = "−";
    out.title = `הוצא את ${row.label}`;
    out.addEventListener("click", async () => {
      await api.findNarrow(dimension, row, true);
      this.page = 1;
      await this.run();
    });

    line.append(narrow, count, out);
    return line;
  }
}

/** The runs of a segment, as elements — corpus text is never put in as markup. */
function runs(list: Run[]): Node[] {
  return list.map((run) => {
    if (run.style === "break") return document.createElement("br");
    const node = document.createElement("span");
    node.className = `run run-${run.style}`;
    node.textContent = run.text;
    return node;
  });
}
