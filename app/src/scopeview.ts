// Where the search looks: shelves and seforim, added and taken out.
//
// > *"i dont know how to add some and minus some things from the search (some
// > seforim or folders). often the tree to pick from (where i can see only how
// > to pick a certain folder) is not even visible - it flashes, then flashes
// > off."*
//
// Both halves of that are one mistake. The only surface for choosing where to
// look was the **facet rail**, and a facet is a count *over a result set*: it is
// computed from the answer, so it did not exist until a search had returned
// hits, it showed only the shelves those hits happened to land on, and
// `drawRail` cleared it at the start of every new search. Hence the flash. And
// the `where` chip — the one control named after the question — opened a menu
// whose single item was *back to the whole shelf*: a doorway that only led out.
//
// A scope is not derived from an answer. It exists before any search and
// outlives every one; `girsa_search::scope::Scope` has always been that, and it
// keeps its steps in order so each one can be taken back on its own rather than
// the whole thing being thrown away. This draws it:
//
//   · the shelf tree, from `shelfTree()` — every shelf, not the ones that
//     happen to be in an answer — with `+` and `−` on each row;
//   · a box for finding one sefer by name, with the same two buttons;
//   · what is currently in the scope, one row per step, each with a `×`.
//
// Nothing here decides anything: `find_scope_add` resolves a shelf key or a slug
// through the same `facets::narrow`/`exclude` a facet click uses, so a scope
// built here and one built from a result row are the same scope.

import { api, type Branch, type Card, type ScopeView } from "./api.ts";
import { button, field, glyph } from "./controls.ts";
import { Latest } from "./latest.ts";
import { sefer } from "./names.ts";
import { say } from "./say.ts";

export class ScopePanel {
  readonly element: HTMLElement;
  private readonly said: HTMLElement;
  private readonly steps: HTMLElement;
  private readonly tree: HTMLElement;
  private readonly found: HTMLElement;
  private readonly box: HTMLInputElement;
  private branches: Branch[] = [];
  private openShelves = new Set<string>();
  private changed: () => void = () => {};
  private readonly draws = new Latest();
  private readonly searches = new Latest();

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "scope";
    this.element.hidden = true;

    const head = document.createElement("div");
    head.className = "scope-head";
    const title = document.createElement("p");
    title.className = "scope-title";
    title.textContent = say("scope");
    this.said = document.createElement("p");
    this.said.className = "scope-said";
    const all = button(say("wholeShelf"), say("scopeEverything"), () => {
      void this.edit(() => api.findWholeShelf().then(() => api.findScope()));
    });
    all.classList.add("scope-all");
    head.append(title, this.said, all);

    this.steps = document.createElement("div");
    this.steps.className = "scope-steps";

    this.box = field(say("scopeFindSefer"), {
      className: "scope-box",
      type: "search",
      dir: "auto",
      placeholder: say("scopeFindSefer"),
    });
    this.box.addEventListener("input", () => void this.lookUp());
    this.found = document.createElement("div");
    this.found.className = "scope-found";

    this.tree = document.createElement("div");
    this.tree.className = "scope-tree";

    this.element.append(head, this.steps, this.box, this.found, this.tree);
  }

  /** The panel tells the search bar to ask again, because narrowing is a
   * different answer to the same question. */
  onChanged(fn: () => void): void {
    this.changed = fn;
  }

  get isOpen(): boolean {
    return !this.element.hidden;
  }

  toggle(): void {
    this.element.hidden = !this.element.hidden;
    if (this.isOpen) void this.refresh();
  }

  /** Read the scope and the shelf again, and redraw. Cheap: the tree carries
   * counts and no seforim, which is the whole reason `shelf_tree` exists. */
  async refresh(): Promise<void> {
    await this.draws.run(
      async () => {
        const scope = await api.findScope();
        // The tree is read once and kept: it changes when the reader rearranges
        // the bookcase, not when they search.
        if (this.branches.length === 0) {
          this.branches = await api.shelfTree().catch(() => [] as Branch[]);
        }
        return scope;
      },
      (scope) => this.draw(scope),
    );
  }

  private draw(scope: ScopeView): void {
    this.said.textContent = scope.everything ? say("scopeEverything") : scope.said;
    this.said.classList.toggle("is-everything", scope.everything);

    this.steps.replaceChildren();
    scope.steps.forEach((step, at) => {
      const row = document.createElement("span");
      row.className = "scope-step" + (step.exclude ? " is-out" : "");
      const label = document.createElement("span");
      label.className = "scope-step-label";
      label.textContent = step.exclude ? `− ${step.label}` : `+ ${step.label}`;
      label.title = `${step.seforim} ${say("seforimCount")}`;
      // Each step on its own, because taking one back used to mean starting
      // over: the only undo was *the whole shelf*.
      const drop = glyph("×", `${say("scopeDrop")} — ${step.label}`, () => {
        void this.edit(() => api.findScopeDrop(at));
      });
      drop.classList.add("scope-step-drop");
      row.append(label, drop);
      this.steps.append(row);
    });

    this.tree.replaceChildren();
    for (const branch of this.branches) this.branch(this.tree, branch);
  }

  /**
   * One shelf to narrow to, and its children under it.
   *
   * This carried `row.style.paddingInlineStart = 8 + depth * 14` — the same
   * line, the same numbers, as `shelf.ts`, which is what happens when the shape
   * of a tree is arithmetic in a view instead of structure. Both draw their
   * children into a `.tree-kids` now, and the indent and the guide rule are the
   * stylesheet's, once.
   */
  private branch(into: HTMLElement, branch: Branch): void {
    // The gathered-seforim child carries its parent's key (W42), so offering it
    // as a second row would narrow to the same shelf twice under two names.
    if (branch.loose) return;
    const row = document.createElement("div");
    row.className = "scope-row";

    const twist = document.createElement("button");
    twist.type = "button";
    twist.className = "scope-twist";
    twist.textContent = branch.children.some((c) => !c.loose)
      ? this.openShelves.has(branch.key)
        ? "▾"
        : "◂"
      : "";
    twist.setAttribute("aria-label", branch.title);
    twist.addEventListener("click", () => {
      if (this.openShelves.has(branch.key)) this.openShelves.delete(branch.key);
      else this.openShelves.add(branch.key);
      this.tree.replaceChildren();
      for (const top of this.branches) this.branch(this.tree, top);
    });

    const name = document.createElement("span");
    name.className = "scope-name";
    name.textContent = branch.title;

    const count = document.createElement("span");
    count.className = "scope-count";
    count.textContent = branch.count.toLocaleString("he-IL");

    row.append(twist, name, count, ...this.twoButtons("shelf", branch.key, branch.title));
    into.append(row);
    if (this.openShelves.has(branch.key) && branch.children.some((child) => !child.loose)) {
      const kids = document.createElement("div");
      kids.className = "tree-kids";
      for (const child of branch.children) this.branch(kids, child);
      into.append(kids);
    }
  }

  /** The two clicks every row carries: add it, or take it out. */
  private twoButtons(dimension: "shelf" | "sefer", key: string, label: string): HTMLElement[] {
    const add = glyph("+", `${say("scopeAdd")} ${label}`, () => {
      void this.edit(() => api.findScopeAdd(dimension, key, label, false));
    });
    add.classList.add("scope-add");
    const out = glyph("−", `${say("scopeTake")} ${label}`, () => {
      void this.edit(() => api.findScopeAdd(dimension, key, label, true));
    });
    out.classList.add("scope-out");
    return [add, out];
  }

  private async lookUp(): Promise<void> {
    const query = this.box.value.trim();
    if (query === "") {
      this.found.replaceChildren();
      return;
    }
    // Behind `Latest`, like every other per-keystroke ask in this window.
    await this.searches.run(
      () => api.search(query),
      (cards) => this.drawFound(cards),
    );
  }

  private drawFound(cards: Card[]): void {
    this.found.replaceChildren();
    for (const card of cards.slice(0, 12)) {
      const row = document.createElement("div");
      row.className = "scope-row";
      const name = document.createElement("span");
      name.className = "scope-name";
      name.textContent = sefer(card);
      row.append(name, ...this.twoButtons("sefer", card.slug, sefer(card)));
      this.found.append(row);
    }
  }

  /** Run an edit, redraw from what Rust actually holds, and say so upward. */
  private async edit(change: () => Promise<ScopeView>): Promise<void> {
    await this.draws.run(change, (scope) => {
      this.draw(scope);
      this.changed();
    });
  }
}
