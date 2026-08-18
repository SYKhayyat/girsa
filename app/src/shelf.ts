// The shelf: browsing it, and rearranging it.
//
// spec.md §5 — *browsable the way seforim are actually organized, **with the
// arrangement editable.*** Two columns: the shelves on the right, what stands
// on the chosen one on the left. Every decision behind it — which shelf a sefer
// is on, what a shelf is called, whether a move is allowed — is answered in
// Rust (`girsa_app::taxonomy`, `girsa_app::arrangement`). This file drags and
// drops and draws.
//
// Nothing here writes to the corpus, because nothing here can: the only way to
// change anything is a command, and each of those writes one file in your own
// layer.

import { api, isShell, type Branch, type Card } from "./api.ts";
import { clearTrouble, sayTrouble } from "./trouble.ts";
import { ask, button, confirmThat, field, glyph, shut } from "./controls.ts";
import { dock, isDocked, minimise, undock } from "./dock.ts";
import { Latest } from "./latest.ts";
import { sefer } from "./names.ts";
import { say } from "./say.ts";

/**
 * Where a sefer the reader picked off the shelf should land.
 *
 * `"tab"` is a tab of its own, which is what a click has always meant.
 * `"here"` puts it **in the tab you are already in**, beside what you are
 * reading — a Gemara and its Rashi and its Tosafos are one tab and three panes,
 * and every way of building that shape went through the mefarshim door. A sefer
 * that is not a mefaresh on the one beside it could only be opened by leaving
 * the tab.
 */
export type Where = "tab" | "here" | "newTab";

type Opened = (slug: string, where: Where) => void;

/**
 * Where a sefer actually lands, given where the reader asked for it and what is
 * open.
 *
 * The decision, out of the handler that would otherwise hold it — the same
 * reason [`dropping`] is a function: `app/test` has no DOM, so an event handler
 * is the one place in this file nothing can execute, and *which tab does this
 * sefer go into* is not a thing to decide somewhere unreachable.
 *
 * The case worth naming is **asking for `here` with nothing open**. There is no
 * *here* to open beside, and the answer is a tab rather than a refusal: a reader
 * who presses *open beside what I am reading* while reading nothing has asked
 * to open a sefer, and getting nothing at all would read as a broken button.
 */
export function landing(where: Where, focused: number | null): { beside: number } | "tab" {
  if (where === "here" && focused !== null) return { beside: focused };
  return "tab";
}

/** What is being dragged: a sefer by slug, or a shelf by key. */
export interface Held {
  what: "work" | "shelf";
  id: string;
  from: string;
}

/** What a drop asks Rust to do, or nothing. */
export interface Move {
  what: "work" | "shelf";
  id: string;
  into: string;
}

/**
 * What a drop means — the decision, out of the handler that used to hold it.
 *
 * It was five lines inside a `drop` listener, which is the only reason nobody
 * had ever run it: `app/test` has no DOM, so a module's exported functions are
 * reachable and its event handlers are not, and the shelf panel was driven in
 * the browser build where `row.draggable` is `false` because dragging is the
 * shell's. So the one path in this file that rearranges a reader's shelf was
 * the one path nothing anywhere had executed.
 *
 * Three refusals, and each is a real drop somebody will do:
 *
 * - **nothing held.** A `drop` can arrive from outside the window — a file, a
 *   selection, another application — and `held` is null. Moving *something* on
 *   the strength of a drop nobody started is the worst of the three.
 * - **onto itself.** Picking a shelf up and putting it back is not an edit, and
 *   asking Rust to do it would produce a refusal a reader would read as *that
 *   did not work* rather than as *that was not anything*.
 * - **back where it came from.** The same, one step out: dropping a sefer on
 *   the shelf it is already on is a no-op, and `from` is carried on the held
 *   thing precisely so this can be seen without asking the shelf.
 *
 * A shelf dropped into its own child is **not** refused here, and that is
 * deliberate: `girsa_app::Arrangement` already refuses it, with the one walk of
 * the tree that knows the whole shape. A second check here would be a second
 * answer to that question, and the two would drift.
 */
export function dropping(held: Held | null, onto: string): Move | null {
  if (!held) return null;
  if (held.id === onto) return null;
  if (held.from === onto) return null;
  return { what: held.what, id: held.id, into: onto };
}

/** What the number beside a shelf's name counts. See [`countedOn`]. */
export interface Counted {
  /** The number itself, in the reader's digits. */
  said: string;
  /** What it counts, in words, for the hover. */
  why: string;
  /** Some or all of it stands on shelves under this one — so clicking this
   * shelf will **not** produce a list of `said` seforim. */
  below: boolean;
}

/**
 * What the number beside a shelf's name counts.
 *
 * > *"`תנ״ך · 66` is a parent whose children are indented 14 px; it reads as a
 * > category with 66 seforim and nothing under it."*
 *
 * Two faults in one row, and the number is the one nobody would have called a
 * bug. `Branch` carries **two** counts — `here`, the seforim standing on this
 * shelf, and `count`, those and everything beneath — and the row drew `count`
 * with nothing saying which. On תנ״ך `here` is 0 and `count` is 66, so the row
 * promised sixty-six seforim and clicking it produced an empty column. The
 * number was never wrong; it just never said what it was a number of.
 *
 * It is the same fault as the mefarshim door promising 67 over a list of 76, and
 * it takes the same answer: the face keeps the number a reader wants — *how much
 * is in here* — and says what it counts rather than being quietly reinterpreted.
 * `below` is what the drawing hangs off, because a count you cannot click
 * through to is a different kind of claim from one you can.
 */
export function countedOn(branch: Branch): Counted {
  const said = branch.count.toLocaleString("he-IL");
  const shelves = branch.children.filter((child) => !child.loose);
  if (shelves.length === 0) return { said, why: `${said} ${say("shelfCountHere")}`, below: false };
  if (branch.here === 0) return { said, why: `${said} ${say("shelfCountUnder")}`, below: true };
  return { said, why: `${said} ${say("shelfCountBoth")}`, below: true };
}

export class ShelfView {
  readonly element: HTMLElement;
  private readonly tree: HTMLElement;
  private readonly list: HTMLElement;
  private readonly heading: HTMLElement;
  private readonly note: HTMLElement;
  private branches: Branch[] = [];
  private open = new Set<string>();
  private chosen = "";
  private held: Held | null = null;
  private opened: Opened = () => {};
  /** One answer at a time — a reader who clicks three shelves quickly used to
   * be shown whichever of the three answered last. See `latest.ts`. */
  private readonly draws = new Latest();

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "shelf";
    this.element.hidden = true;

    const sheet = document.createElement("div");
    sheet.className = "shelf-sheet";
    // What the strip says when the bookcase is minimised.
    sheet.dataset.name = say("theShelf");
    sheet.addEventListener("click", (event) => {
      // **The strip itself**, not anything inside it. Every control is a child
      // of the sheet, so a bare `click` handler here fires for them too — and
      // the minimise button is one of them: it added `is-small`, the click
      // bubbled to this handler, and this handler took it straight off again.
      // Minimising did nothing at all, visibly, which is the exact family of bug
      // this whole pass is about. Minimised, the children are `display: none`,
      // so the sheet is the only thing left to hit.
      if (event.target === sheet && this.element.classList.contains("is-small")) {
        this.dock();
      }
    });

    const bar = document.createElement("div");
    bar.className = "shelf-bar";
    const title = document.createElement("p");
    title.className = "shelf-title";
    title.textContent = say("theShelf");
    bar.append(title, this.tool(say("newShelf"), say("newShelfWhy"), () => void this.make()));
    bar.append(this.tool(say("resetShelf"), say("resetShelfWhy"), () => void this.reset()));
    // > *"it should be minimizable in a way that you can easily reopen it."*
    //
    // Minimise, and not another close: the bookcase keeps the shelf you were on
    // and the shelves you had open, and the strip it leaves behind is the way
    // back. Docking already made the reading narrower rather than covering it;
    // this is the same idea one notch further.
    const shrink = this.tool(say("minimize"), say("minimizeWhy"), () => this.minimise());
    shrink.classList.add("shelf-minimise");
    bar.append(shrink);
    const close = shut(() => this.close());
    close.classList.add("shelf-close");
    bar.append(close);

    const body = document.createElement("div");
    body.className = "shelf-body";
    this.tree = document.createElement("div");
    this.tree.className = "shelf-tree";
    this.list = document.createElement("div");
    this.list.className = "shelf-list";
    const right = document.createElement("div");
    right.className = "shelf-column";
    this.heading = document.createElement("p");
    this.heading.className = "shelf-heading";
    right.append(this.heading, this.list);
    body.append(this.tree, right);

    this.note = document.createElement("p");
    this.note.className = "shelf-note";

    sheet.append(bar, body, this.note);
    this.element.append(sheet);
    this.element.addEventListener("pointerdown", (event) => {
      if (event.target === this.element) this.close();
    });
  }

  private tool(label: string, title: string, click: () => void): HTMLElement {
    return button(label, title, click);
  }

  get isOpen(): boolean {
    return !this.element.hidden;
  }

  /** Standing beside the reading rather than over it — so the reader is
   * reading, and the keyboard is theirs (finding 3). From `dock.ts`, which owns
   * the set, not from this panel's own class. */
  get isDocked(): boolean {
    return isDocked("shelf");
  }

  async show(opened: Opened): Promise<void> {
    this.opened = opened;
    this.element.hidden = false;
    await this.refresh();
  }

  close(): void {
    this.element.hidden = true;
    this.element.classList.remove("is-docked", "is-small");
    undock("shelf");
  }

  /** Shrink to a strip, keeping the shelf you were on. Clicking it opens the
   * column again. */
  private minimise(): void {
    // A closed panel has nothing to minimise, and docking one would take a strip
    // of the reading away for a panel nobody can see.
    if (!this.isOpen) return;
    this.element.classList.add("is-docked", "is-small");
    dock("shelf");
    minimise("shelf", true);
  }

  /**
   * Open a sefer and **keep the shelf** (W47).
   *
   * > *"there should be a way to open while keeping madaf open."*
   *
   * The bookcase used to close on the way out, so browsing to a second sefer
   * meant opening it again and finding your place in it again. Docked, it is a
   * column on the leading edge and the reading is made narrower rather than
   * covered — so what you just opened is visible *and* the shelf you opened it
   * from still is.
   */
  private dock(): void {
    this.element.classList.add("is-docked");
    this.element.classList.remove("is-small");
    dock("shelf");
    minimise("shelf", false);
  }

  async toggle(opened: Opened): Promise<void> {
    if (this.isOpen) this.close();
    else await this.show(opened);
  }

  /** Read the tree again and redraw — after an edit, or after a file drop. */
  async refresh(): Promise<void> {
    this.branches = await api.shelfTree();
    if (!this.chosen) this.chosen = this.branches[0]?.key ?? "";
    this.drawTree();
    await this.drawList();
    this.note.textContent = isShell() ? say("shelfHint") : say("shelfReadOnly");
  }

  // --- the shelves ---------------------------------------------------------

  private drawTree(): void {
    const rows = document.createElement("div");
    // Dropping onto the empty space below the shelves means *the top level* —
    // otherwise a shelf dragged out of a shelf has nowhere to go.
    rows.className = "shelf-rows";
    this.receives(rows, "");
    for (const branch of this.branches) this.row(rows, branch, "");
    this.tree.replaceChildren(rows);
  }

  /**
   * One shelf, and — if it is open — its children **inside a container of their
   * own**.
   *
   * The depth used to be a number here, written onto every row as
   * `paddingInlineStart = 8 + depth * 14`, and 14px is less than the width of
   * one Hebrew letter at this size. A reader could not see that anything hung
   * under anything, which is half of why `תנ״ך · 66` read as a shelf with
   * sixty-six seforim on it rather than as the top of a branch (`countedOn` is
   * the other half). `scopeview.ts` had the identical line — the same
   * arithmetic, the same 14 — because a number in two files is how one decision
   * ends up being made twice.
   *
   * So the nesting *is* the nesting: children go in a `.tree-kids`, which is
   * where the indent and the guide rule live, and neither this file nor
   * `scopeview.ts` knows a pixel. A level cannot be drawn at the wrong depth
   * because nothing computes a depth.
   */
  private row(into: HTMLElement, branch: Branch, parent: string): void {
    const row = document.createElement("div");
    row.className = "shelf-row" + (branch.key === this.chosen ? " is-chosen" : "");
    row.draggable = isShell();

    const twist = document.createElement("button");
    twist.className = "shelf-twist";
    twist.textContent = branch.children.length === 0 ? "" : this.open.has(branch.key) ? "▾" : "◂";
    twist.addEventListener("click", (event) => {
      event.stopPropagation();
      if (this.open.has(branch.key)) this.open.delete(branch.key);
      else this.open.add(branch.key);
      this.drawTree();
    });

    const name = document.createElement("span");
    name.className = "shelf-name";
    name.textContent = branch.title;
    if (branch.mine) name.classList.add("is-mine");
    if (branch.edited) name.title = say("editedShelf");

    const counted = countedOn(branch);
    const count = document.createElement("span");
    count.className = "shelf-count" + (counted.below ? " is-below" : "");
    count.textContent = counted.said;
    count.title = counted.why;

    const pin = document.createElement("button");
    pin.className = "shelf-pin";
    pin.textContent = "⇱";
    pin.title = say("pinToTop");
    pin.addEventListener("click", (event) => {
      event.stopPropagation();
      void this.edit(() => api.shelfPin(parent, branch.key));
    });

    // The gathered-seforim child (W42) is not a shelf. It carries its parent's
    // key so that clicking it lists exactly the loose seforim — and for the same
    // reason it must not be renamed, pinned or dragged: every one of those would
    // silently edit the shelf above it.
    if (!branch.loose) row.append(twist, name, count, pin);
    else row.append(twist, name, count);
    row.addEventListener("click", () => {
      this.chosen = branch.key;
      this.open.add(branch.key);
      this.drawTree();
      void this.drawList();
    });
    if (!branch.loose) {
      row.addEventListener("dblclick", () => this.rename(branch, name));
      row.addEventListener("dragstart", (event) => {
        this.held = { what: "shelf", id: branch.key, from: parent };
        event.dataTransfer?.setData("text/plain", branch.key);
      });
    } else {
      row.draggable = false;
      name.classList.add("is-loose");
      name.title = say("looseSeforim");
    }
    this.receives(row, branch.key);

    into.append(row);
    if (this.open.has(branch.key) && branch.children.length > 0) {
      const kids = document.createElement("div");
      kids.className = "tree-kids";
      for (const child of branch.children) this.row(kids, child, branch.key);
      into.append(kids);
    }
  }

  /** A row (or the empty space) that a sefer or a shelf can be dropped on. */
  private receives(node: HTMLElement, key: string): void {
    node.addEventListener("dragover", (event) => {
      if (!this.held) return;
      event.preventDefault();
      node.classList.add("is-target");
    });
    node.addEventListener("dragleave", () => node.classList.remove("is-target"));
    node.addEventListener("drop", (event) => {
      event.preventDefault();
      event.stopPropagation();
      node.classList.remove("is-target");
      const move = dropping(this.held, key);
      this.held = null;
      if (!move) return;
      void this.edit(() =>
        move.what === "work"
          ? api.shelfPutWork(move.id, move.into)
          : api.shelfPutShelf(move.id, move.into),
      );
    });
  }

  private rename(branch: Branch, name: HTMLElement): void {
    if (!isShell()) return;
    const input = field(say("shelfName"));
    input.className = "shelf-rename";
    input.value = branch.title;
    input.setAttribute("dir", "auto");
    const finish = (keep: boolean) => {
      input.replaceWith(name);
      if (keep && input.value.trim() !== branch.title) {
        void this.edit(() => api.shelfRename(branch.key, input.value));
      }
    };
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") finish(true);
      if (event.key === "Escape") finish(false);
      event.stopPropagation();
    });
    input.addEventListener("blur", () => finish(true));
    name.replaceWith(input);
    input.focus();
    input.select();
  }

  // --- what is on the chosen shelf ----------------------------------------

  private async drawList(): Promise<void> {
    const branch = find(this.branches, this.chosen);
    this.heading.textContent = branch ? heading(branch) : "";

    const asked = this.chosen;
    await this.draws.run(
      () => (asked ? api.shelfWorks(asked) : Promise.resolve([] as Card[])),
      (works) => {
        if (works.length === 0) {
          this.list.replaceChildren(...this.nothingStandsHere(branch));
          return;
        }
        this.list.replaceChildren(...works.map((card) => this.card(card)));
      },
    );
  }

  /**
   * What the column shows when no sefer stands on the chosen shelf.
   *
   * It used to be one grey sentence — *the seforim here are on the shelves under
   * it* — under a heading reading `תנ״ך · 0 מתוך 66`, which is a column that
   * says *there is something, elsewhere, and I will not tell you where*. The
   * shelves under it are right there in the branch we already have, so they are
   * the list: each with its own count, each a click away. Sixty-six becomes
   * five and twenty-one and thirteen, and the number on the tree row is
   * something a reader can follow rather than something they have to trust.
   */
  private nothingStandsHere(branch: Branch | null): HTMLElement[] {
    // Never the gathered-seforim child (W42): it carries its parent's key, so
    // offering it here would be a row that navigates to the shelf you are on.
    const shelves = (branch?.children ?? []).filter((child) => !child.loose);
    if (shelves.length === 0) {
      const none = document.createElement("p");
      none.className = "shelf-empty";
      none.textContent = branch?.count ? say("shelfBelow") : say("shelfEmpty");
      return [none];
    }
    const title = document.createElement("p");
    title.className = "shelf-under-title";
    title.textContent = say("shelfUnderHeading");
    return [title, ...shelves.map((child) => this.under(child))];
  }

  /** One shelf, offered from the column rather than the tree. */
  private under(branch: Branch): HTMLElement {
    const row = document.createElement("div");
    row.className = "shelf-under";
    const name = document.createElement("span");
    name.className = "shelf-under-name";
    name.textContent = branch.title;
    if (branch.loose) name.classList.add("is-loose");
    const counted = countedOn(branch);
    const count = document.createElement("span");
    count.className = "shelf-count" + (counted.below ? " is-below" : "");
    count.textContent = counted.said;
    count.title = counted.why;
    row.append(name, count);
    row.addEventListener("click", () => {
      this.chosen = branch.key;
      this.open.add(branch.key);
      this.drawTree();
      void this.drawList();
    });
    return row;
  }

  private card(card: Card): HTMLElement {
    const row = document.createElement("div");
    row.className = "shelf-work";
    row.draggable = isShell();

    const title = document.createElement("span");
    title.className = "shelf-work-title";
    title.textContent = sefer(card);

    const aside = document.createElement("span");
    aside.className = "shelf-work-aside";
    const said = [card.author, card.era].filter(Boolean).join(" · ");
    aside.textContent = card.source === "mine" ? said || say("mine") : said;
    if (card.source === "mine") aside.classList.add("is-mine");

    // **Into the tab you are already in**, and it is its own control rather
    // than a modifier on the click. A held key is a thing you have to be told
    // about, and the reader who asked for this asked for *a way to open a new
    // sefer in the same tab* — a way, on the row, that can be seen.
    const here = glyph("⊞", say("openHere"), (event) => {
      // The row underneath opens a whole tab. Without this the reader gets
      // both, which is the pane they asked for and a tab they did not.
      event.stopPropagation();
      this.dock();
      this.opened(card.slug, "here");
    });
    here.className = "shelf-work-here";

    row.append(title, aside, here);
    // One gesture, once. `dblclick` and `click` both opened the sefer, so a
    // double click opened it, focused it, and opened it again.
    row.addEventListener("click", () => {
      this.dock();
      this.opened(card.slug, "tab");
    });
    row.addEventListener("dragstart", (event) => {
      this.held = { what: "work", id: card.slug, from: this.chosen };
      event.dataTransfer?.setData("text/plain", card.slug);
    });
    return row;
  }

  // --- edits ---------------------------------------------------------------

  private async make(): Promise<void> {
    if (!isShell()) return;
    const under = find(this.branches, this.chosen);
    const title = await ask(say("newShelfNamed"), { value: say("newShelfDefault") });
    if (!title?.trim()) return;
    await this.edit(async () => {
      const key = await api.shelfMake(under ? under.key : "", title);
      this.open.add(under ? under.key : key);
      this.chosen = key;
    });
  }

  private async reset(): Promise<void> {
    if (!isShell()) return;
    // The one destructive thing this panel does, so it asks — in this
    // window's own furniture, and with the affirmative button saying what it
    // will do rather than `OK`.
    if (!(await confirmThat(say("resetAsk"), { ok: say("resetShelf") }))) return;
    await this.edit(() => api.shelfReset());
  }

  /** Run an edit, say so if it was refused, and redraw either way. */
  private async edit(change: () => Promise<unknown>): Promise<void> {
    try {
      await change();
      this.note.textContent = "";
      clearTrouble(this.note);
    } catch (e) {
      // A refusal — a shelf inside itself, a personal layer that will not
      // write — is shown. The shelf did not move, and saying nothing would
      // leave a reader believing it had.
      sayTrouble(this.note, e);
      window.setTimeout(() => this.note.classList.remove("is-trouble"), 4000);
    }
    await this.refresh();
  }

  /** Say what came of a file drop. */
  say(message: string, trouble: boolean): void {
    this.note.textContent = message;
    this.note.classList.toggle("is-trouble", trouble);
  }
}

/**
 * What the column's heading says about the shelf you chose.
 *
 * `X מתוך Y` is the right sentence for a shelf that holds some of its own
 * seforim and has more below. For a shelf that holds none it read `0 מתוך 66`,
 * which is a heading whose first number is the reason the column looked broken.
 * A pure parent says what it is: sixty-six, on the shelves under it.
 */
function heading(branch: Branch): string {
  const count = branch.count.toLocaleString("he-IL");
  if (branch.here === 0 && branch.children.some((child) => !child.loose)) {
    return `${branch.title} · ${count} ${say("shelfUnderCount")}`;
  }
  return `${branch.title} · ${branch.here.toLocaleString("he-IL")} ${say("shelfOf")} ${count}`;
}

function find(branches: Branch[], key: string): Branch | null {
  for (const branch of branches) {
    if (branch.key === key) return branch;
    const inside = find(branch.children, key);
    if (inside) return inside;
  }
  return null;
}
