// A parenthesised list, assembled: several passages on one page.
//
// spec.md §9.7 and #63. The difference between `berachos 2a, shabbos 3b` and
// `(berachos 2a, shabbos 3b)` is one character in what the reader typed and the
// whole of what they wanted: two places to go, or one page to look at. `nav`
// reads the parentheses and `girsa_app::source_sheet` does the finding; this
// column draws what came back.
//
// # Why a column and not a pane
//
// A pane is a sefer, and a sheet is not a sefer: it has no address, no margin, no
// mefarshim, and the reader will close it in a minute. Docking it beside the
// reading is what makes it usable while composing — a writer with the sheet on
// one side and Ksav on the other is the shape this is for.
//
// # The hole, said where the hole is
//
// A reference that could not be found is drawn **in place**, after the passages,
// in the shelf's own words. A sheet that quietly left it out would read as
// *these are the passages you asked for* and be one short, and a reader comparing
// it against what they typed would have no way to see which line went.

import { api, type Asked, type SourceSheet } from "./api.ts";
import { about, button, shut } from "./controls.ts";
import { dock, undock } from "./dock.ts";
import { Latest } from "./latest.ts";
import { say } from "./say.ts";
import { sayTrouble } from "./trouble.ts";

export class SheetView {
  readonly element: HTMLElement;
  private readonly list: HTMLElement;
  private readonly note: HTMLElement;
  private goTo: ((landing: Asked) => Promise<void>) | null = null;
  private readonly draws = new Latest();

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  constructor() {
    this.element = document.createElement("section");
    this.element.className = "sheet";

    const head = document.createElement("header");
    head.className = "sheet-head";
    const title = document.createElement("span");
    title.className = "sheet-title";
    title.textContent = say("sheetTitle");
    this.note = document.createElement("span");
    this.note.className = "sheet-note";
    head.append(title, this.note, shut(() => this.close()));

    this.list = document.createElement("div");
    this.list.className = "sheet-list";
    this.element.append(head, about(say("sheetAbout")), this.list);
  }

  /**
   * Who lands a row, because which pane receives it is the window's fact.
   *
   * The view asks `cite_open` for the landing and hands it over: a column that
   * moved the reading itself would be a second answer to *where does this go*,
   * and the window already has the first one.
   */
  onGo(goTo: (landing: Asked) => Promise<void>): void {
    this.goTo = goTo;
  }

  /**
   * Assemble and show.
   *
   * The refs are the resolver's own strings, handed straight back to Rust: the
   * window does not parse a reference, and a second reading of
   * `girsa:bavli/berakhot/2a:1` here would be a second answer to *where is that*.
   */
  async show(refs: string[]): Promise<void> {
    dock("sheet");
    this.element.classList.add("is-open");
    this.list.replaceChildren();
    this.note.textContent = "";
    await this.draws.attempt(
      () => api.sheetOpen(refs),
      (sheet) => this.draw(sheet),
      (e) => sayTrouble(this.note, e, "open_ref"),
    );
  }

  close(): void {
    this.element.classList.remove("is-open");
    undock("sheet");
    // A redraw the reader has closed past must not land in a hidden column — the
    // same rule `findhere.ts` follows, through the same ticket.
    this.draws.take();
  }

  private draw(sheet: SourceSheet): void {
    this.note.textContent = sheet.entries.length > 0 ? String(sheet.entries.length) : "";
    if (sheet.entries.length === 0 && sheet.unread.length === 0) {
      const none = document.createElement("p");
      none.className = "sheet-none";
      none.textContent = say("sheetEmpty");
      this.list.append(none);
      return;
    }
    for (const entry of sheet.entries) {
      const row = document.createElement("article");
      row.className = "sheet-row";

      const head = document.createElement("div");
      head.className = "sheet-where";
      const open = button(entry.title, entry.reference, () => {
        void this.follow(entry.reference);
      });
      open.classList.add("sheet-open");
      const ref = document.createElement("span");
      ref.className = "sheet-ref";
      // `girsa:` is the storage spelling and not what a reader writes.
      ref.textContent = entry.reference.replace(/^girsa:/, "");
      head.append(open, ref);

      const words = document.createElement("p");
      words.className = "sheet-words";
      words.textContent = entry.words;

      row.append(head, words);
      this.list.append(row);
    }
    // The holes last, and in their own words: a reference that could not be found
    // is not a passage with no words.
    for (const missing of sheet.unread) {
      const said = document.createElement("p");
      said.className = "sheet-missing";
      said.textContent = missing.why;
      this.list.append(said);
    }
  }

  private async follow(reference: string): Promise<void> {
    if (!this.goTo) return;
    const at = reference.startsWith("girsa:") ? reference : `girsa:${reference}`;
    try {
      const landing = await api.citeOpen(at);
      await this.goTo(landing);
    } catch (e) {
      // A row that will not open leaves the sheet as it was, and says so in the
      // column rather than in a toast the reader may be looking away from.
      sayTrouble(this.note, e, "open_ref");
    }
  }
}
