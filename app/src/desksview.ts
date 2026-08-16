// Arrangements you named, so you can come back to one.
//
// > *Three sugyos at once.*
//
// A tab strip answers *what is open*. It cannot answer *what was I set up for
// last Tuesday* — and a bachur who has laid out a Gemara, its Rashi, its
// Tosafos and the Rambam on the sugya has done twenty seconds of work that a
// close-and-reopen throws away.
//
// Otzaria saves a list of tabs under a name. What is saved here is the whole
// arrangement: the tabs, the panes inside each of them, which pane follows
// which, and how wide each one is. There is nothing to flatten — a desk is a
// `Workspace`, kept.
//
// # The one rule that makes it usable
//
// **Switching writes the arrangement back first.** A switcher that discarded
// what you had in order to show you something else is a switcher nobody uses
// twice. `desk_open` does that in Rust, where the session is; this panel does
// not have to know.

import { api, type DeskRow } from "./api.ts";
import { about, button, field, glyph, shut } from "./controls.ts";
import { undock } from "./dock.ts";
import { Latest } from "./latest.ts";
import { fill, say } from "./say.ts";
import { sayTrouble } from "./trouble.ts";

export class DesksView {
  readonly element: HTMLElement;
  private readonly list: HTMLElement;
  private readonly note: HTMLElement;
  private readonly box: HTMLInputElement;
  private readonly draws = new Latest();
  /** Told when a desk was opened, so the window redraws what is on screen. */
  private changed: (() => Promise<void>) | null = null;

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  constructor() {
    this.element = document.createElement("section");
    this.element.className = "desks";

    const head = document.createElement("header");
    head.className = "desks-head";
    const title = document.createElement("span");
    title.className = "desks-title";
    title.textContent = say("desks");
    head.append(title, shut(() => this.close()));

    this.note = document.createElement("p");
    this.note.className = "desks-note";

    // Naming the arrangement on screen. A box and a button rather than a
    // prompt: `window.prompt` is a modal the webview draws in its own language
    // and its own direction, which in a right-to-left Hebrew window is a dialog
    // nobody can read.
    const keep = document.createElement("div");
    keep.className = "desks-keep";
    this.box = field(say("desksName"), {
      className: "desks-box",
      placeholder: say("desksNamePlaceholder"),
      dir: "rtl",
    });
    this.box.addEventListener("keydown", (event) => {
      if (event.key !== "Enter") return;
      event.preventDefault();
      void this.keep();
    });
    keep.append(this.box, button(say("desksKeep"), say("desksKeepWhy"), () => void this.keep()));

    this.list = document.createElement("div");
    this.list.className = "desks-list";

    this.element.append(head, this.note, keep, about(say("desksAbout")), this.list);
  }

  onChanged(fn: () => Promise<void>): void {
    this.changed = fn;
  }

  async open(): Promise<void> {
    this.element.classList.add("is-open");
    // Over the reading rather than beside it: naming an arrangement is a thing
    // you do and finish, not a column you read alongside a daf.
    undock("desks");
    await this.draw();
    this.box.focus();
  }

  close(): void {
    this.element.classList.remove("is-open");
    undock("desks");
  }

  async toggle(): Promise<void> {
    if (this.isOpen) {
      this.close();
      return;
    }
    await this.open();
  }

  private async draw(): Promise<void> {
    const mine = this.draws.take();
    let rows: DeskRow[];
    try {
      rows = await api.desks();
    } catch (e) {
      if (!mine.current()) return;
      this.list.replaceChildren();
      sayTrouble(this.note, e, "desks");
      return;
    }
    if (!mine.current()) return;
    this.show(rows);
  }

  private show(rows: DeskRow[]): void {
    this.note.textContent =
      rows.length === 0 ? say("desksNone") : fill("desksCount", { desks: rows.length });
    this.list.replaceChildren(...rows.map((desk) => this.row(desk)));
    // The box carries the name of the desk you are at, so pressing keep is
    // *save this one as it stands now* rather than *invent a name again*.
    const here = rows.find((desk) => desk.here);
    if (here && !this.box.value) this.box.value = here.name;
  }

  private row(desk: DeskRow): HTMLElement {
    const row = document.createElement("div");
    row.className = "desk" + (desk.here ? " is-here" : "");

    const open = document.createElement("button");
    open.type = "button";
    open.className = "desk-open";
    const name = document.createElement("span");
    name.className = "desk-name";
    name.textContent = desk.name;
    // What is in it, so a reader picking between four desks is picking on
    // something rather than on a word they wrote last week.
    const what = document.createElement("span");
    what.className = "desk-what";
    what.textContent = fill("desksHolds", { tabs: desk.tabs, seforim: desk.seforim });
    open.append(name, what);
    open.title = say("desksOpenWhy");
    open.addEventListener("click", () => void this.go(desk.name));

    row.append(open, glyph("✕", say("desksForget"), () => void this.forget(desk.name)));
    return row;
  }

  private async keep(): Promise<void> {
    const name = this.box.value.trim();
    if (!name) {
      this.box.focus();
      return;
    }
    try {
      this.show(await api.deskKeep(name));
    } catch (e) {
      sayTrouble(this.note, e, "desks");
    }
  }

  private async go(name: string): Promise<void> {
    try {
      this.show(await api.deskOpen(name));
    } catch (e) {
      sayTrouble(this.note, e, "desks");
      return;
    }
    this.box.value = name;
    // Everything on screen came from the arrangement that was just replaced.
    await this.changed?.();
  }

  private async forget(name: string): Promise<void> {
    try {
      this.show(await api.deskForget(name));
    } catch (e) {
      sayTrouble(this.note, e, "desks");
    }
  }
}
