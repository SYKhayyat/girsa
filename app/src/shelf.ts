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
import { ask, button, confirmThat, field } from "./controls.ts";
import { dock, isDocked, minimise, undock } from "./dock.ts";
import { Latest } from "./latest.ts";
import { sefer } from "./names.ts";
import { say } from "./say.ts";

type Opened = (slug: string) => void;

/** What is being dragged: a sefer by slug, or a shelf by key. */
interface Held {
  what: "work" | "shelf";
  id: string;
  from: string;
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
    const close = this.tool(say("close"), say("esc"), () => this.close());
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
    for (const branch of this.branches) this.row(rows, branch, 0, "");
    this.tree.replaceChildren(rows);
  }

  private row(into: HTMLElement, branch: Branch, depth: number, parent: string): void {
    const row = document.createElement("div");
    row.className = "shelf-row" + (branch.key === this.chosen ? " is-chosen" : "");
    row.style.paddingInlineStart = `${8 + depth * 14}px`;
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

    const count = document.createElement("span");
    count.className = "shelf-count";
    count.textContent = branch.count.toLocaleString("he-IL");

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
    if (this.open.has(branch.key)) {
      for (const child of branch.children) this.row(into, child, depth + 1, branch.key);
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
      const held = this.held;
      this.held = null;
      if (!held || held.id === key) return;
      void this.edit(() =>
        held.what === "work" ? api.shelfPutWork(held.id, key) : api.shelfPutShelf(held.id, key),
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
    this.heading.textContent = branch
      ? `${branch.title} · ${branch.here.toLocaleString("he-IL")} ${say("shelfOf")} ${branch.count.toLocaleString("he-IL")}`
      : "";

    const asked = this.chosen;
    await this.draws.run(
      () => (asked ? api.shelfWorks(asked) : Promise.resolve([] as Card[])),
      (works) => {
        if (works.length === 0) {
          const none = document.createElement("p");
          none.className = "shelf-empty";
          none.textContent = branch?.count ? say("shelfBelow") : say("shelfEmpty");
          this.list.replaceChildren(none);
          return;
        }
        this.list.replaceChildren(...works.map((card) => this.card(card)));
      },
    );
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

    row.append(title, aside);
    row.addEventListener("dblclick", () => {
      this.dock();
      this.opened(card.slug);
    });
    row.addEventListener("click", () => {
      this.dock();
      this.opened(card.slug);
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

function find(branches: Branch[], key: string): Branch | null {
  for (const branch of branches) {
    if (branch.key === key) return branch;
    const inside = find(branch.children, key);
    if (inside) return inside;
  }
  return null;
}
