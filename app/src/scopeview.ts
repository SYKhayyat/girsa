// Where the search looks: shelves and seforim, ticked and unticked.
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
// outlives every one; `girsa_search::scope::Scope` has always been that.
//
// # The second reading, and what it cost
//
// The panel that replaced the rail offered a `+` and a `−` on every row, and a
// reader did the obvious thing with them: ticked the masechtos of Shas one at a
// time and searched `חייב`. It came back **`0 found`**, under a chip listing all
// thirty-seven of them. Every `+` was its own `Must`, so twelve clicks asked for
// a segment inside twelve masechtos at once. See `girsa_search::scope::Asked`
// for the fix underneath; what changed here is the control on top of it:
//
//   · a **box**, not a `+` and a `−`, because a box is a state and a `+` is an
//     event — a reader can read a ticked box off the screen and cannot read a
//     click they made four minutes ago;
//   · **one line that says what it all comes to** — `מחפש ב: 37 מתוך 7,189
//     ספרים` — which is the whole of *"it should be more clear what is and is
//     not included"*, and which no list of labels can answer;
//   · a shelf **opens down to its seforim**, so a single sefer is one click and
//     not a name you have to already know how to spell;
//   · one button that unticks everything, which is also the button that ticks
//     everything, because a search with nothing ticked runs over the whole
//     shelf. Two labels for one state would be two ways to read one screen.
//
// Nothing here decides anything: `find_scope_set` resolves a shelf key or a slug
// through the same `facets::narrow`/`exclude` a facet click uses, so a scope
// built here and one built from a result row are the same scope.

import { api, type Branch, type Card, type Dimension, type ScopeView } from "./api.ts";
import { button, field, glyph } from "./controls.ts";
import { Latest } from "./latest.ts";
import { sefer } from "./names.ts";
import { fill, say } from "./say.ts";

/** What a box on one row reads. */
type Tick = "in" | "out" | "off";

export class ScopePanel {
  readonly element: HTMLElement;
  private readonly counted: HTMLElement;
  private readonly said: HTMLElement;
  private readonly steps: HTMLElement;
  private readonly tree: HTMLElement;
  private readonly found: HTMLElement;
  private readonly box: HTMLInputElement;
  private branches: Branch[] = [];
  private openShelves = new Set<string>();
  /** Shelf key → the seforim standing on it, once it has been opened. Asked for
   * on the first twist and kept: a shelf's contents change when the reader
   * rearranges the bookcase, not while they are ticking boxes. */
  private readonly seforim = new Map<string, Card[]>();
  /** What the scope holds, as the rows need to ask it: key → direction. */
  private ticked = new Map<string, boolean>();
  /** The last rows the name box found, so a tick can redraw them with their
   * boxes in the state Rust now holds. */
  private lastFound: Card[] = [];
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
    // The line a reader actually reads. It is first because it is the answer to
    // the question the panel is named after.
    this.counted = document.createElement("p");
    this.counted.className = "scope-counted";
    this.said = document.createElement("p");
    this.said.className = "scope-said";
    const all = button(say("scopeClear"), say("scopeClearWhy"), () => {
      void this.edit(() => api.findWholeShelf().then(() => api.findScope()));
    });
    all.classList.add("scope-all");
    head.append(title, this.counted, this.said, all);

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
    this.counted.textContent = fill("scopeCounted", {
      n: scope.seforim,
      all: scope.shelf,
    });
    this.counted.classList.toggle("is-everything", scope.everything);
    this.said.textContent = scope.everything ? say("scopeEverything") : scope.said;
    this.said.classList.toggle("is-everything", scope.everything);

    // One map, rebuilt from what Rust holds rather than from what was clicked.
    // The panel used to keep its own idea of the scope beside the engine's, and
    // the two could not be told apart on screen when they disagreed.
    this.ticked = new Map(scope.steps.map((step) => [step.key, !step.exclude]));
    this.drawSteps(scope);
    this.drawTree();
    // The rows found by name carry boxes of their own, so a tick anywhere has to
    // reach them too — otherwise ticking a sefer found by name leaves its own
    // box empty, which reads as the click having done nothing.
    this.found.replaceChildren();
    for (const card of this.lastFound) this.seferRow(this.found, card);
  }

  /**
   * One row per step, each with a `×`.
   *
   * Kept beside the tree rather than replaced by it, because not every step has
   * a row in the tree: narrowing by era or by author comes from the facet rail
   * and there is nowhere in a shelf tree to draw it ticked. This is where those
   * become visible, and where any step can be taken back on its own — the undo
   * that used to be *back to the whole shelf*, throwing away four clicks to
   * reverse the fifth.
   */
  private drawSteps(scope: ScopeView): void {
    this.steps.replaceChildren();
    scope.steps.forEach((step, at) => {
      const row = document.createElement("span");
      row.className = "scope-step" + (step.exclude ? " is-out" : "");
      const label = document.createElement("span");
      label.className = "scope-step-label";
      label.textContent = step.exclude ? `− ${step.label}` : `+ ${step.label}`;
      label.title = `${step.seforim} ${say("seforimCount")}`;
      const drop = glyph("×", `${say("scopeDrop")} — ${step.label}`, () => {
        void this.edit(() => api.findScopeDrop(at));
      });
      drop.classList.add("scope-step-drop");
      row.append(label, drop);
      this.steps.append(row);
    });
  }

  private drawTree(): void {
    this.tree.replaceChildren();
    for (const branch of this.branches) this.branch(this.tree, branch);
  }

  private tick(key: string): Tick {
    const held = this.ticked.get(key);
    if (held === undefined) return "off";
    return held ? "in" : "out";
  }

  /**
   * One shelf, its box, and its children under it.
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

    // A shelf twists open whether or not it has shelves under it: what is under
    // it *as far as this panel is concerned* is the seforim, and the reason a
    // reader could only pick a folder was that the tree stopped at folders.
    const open = this.openShelves.has(branch.key);
    const twist = glyph(open ? "▾" : "◂", `${say("scopeOpenShelf")} — ${branch.title}`, () => {
      void this.twist(branch);
    });
    twist.classList.add("scope-twist");
    twist.setAttribute("aria-expanded", String(open));

    const box = this.checkbox("shelf", branch.key, branch.title);

    const name = document.createElement("span");
    name.className = "scope-name";
    name.textContent = branch.title;

    const count = document.createElement("span");
    count.className = "scope-count";
    count.textContent = branch.count.toLocaleString("he-IL");

    row.append(twist, box, name, count);
    row.classList.add(`is-${this.tick(branch.key)}`);
    into.append(row);
    if (!this.openShelves.has(branch.key)) return;

    const kids = document.createElement("div");
    kids.className = "tree-kids";
    for (const child of branch.children) this.branch(kids, child);
    // Then the seforim standing on the shelf itself — the half of the tree that
    // did not exist, and the reason a single sefer could not be picked.
    const here = this.seforim.get(branch.key);
    if (here === undefined) {
      const waiting = document.createElement("div");
      waiting.className = "scope-row is-waiting";
      waiting.textContent = say("scopeLoading");
      kids.append(waiting);
    } else {
      for (const card of here) this.seferRow(kids, card);
    }
    into.append(kids);
  }

  private seferRow(into: HTMLElement, card: Card): void {
    const row = document.createElement("div");
    row.className = `scope-row is-sefer is-${this.tick(card.slug)}`;
    const gap = document.createElement("span");
    gap.className = "scope-twist";
    const name = document.createElement("span");
    name.className = "scope-name";
    name.textContent = sefer(card);
    row.append(gap, this.checkbox("sefer", card.slug, sefer(card)), name);
    into.append(row);
  }

  /**
   * The box on a row.
   *
   * A real `input[type=checkbox]`, not a glyph in a button: it is a state, a
   * screen reader already knows how to say it, and the space bar already works
   * on it. `indeterminate` is the third state — the row was ticked *off*, which
   * a two-state box has nowhere to put.
   */
  private checkbox(dimension: Dimension, key: string, label: string): HTMLElement {
    const state = this.tick(key);
    const title = state === "in" ? say("scopeUntick") : say("scopeTick");
    // Through `field`, like every control in this window: a checkbox with no
    // name is one of two hundred unlabelled boxes to a screen reader, and B14's
    // guard in `sources.test.mjs` fails the build over exactly this.
    const box = field(`${label} — ${title}`, {
      type: "checkbox",
      className: "scope-box-tick",
    });
    box.checked = state === "in";
    box.indeterminate = state === "out";
    box.title = `${title} — ${label}`;
    box.addEventListener("change", () => {
      const on = box.checked;
      void this.edit(() => api.findScopeSet(dimension, key, label, on));
    });
    return box;
  }

  /** Open or close a shelf, fetching what stands on it the first time. */
  private async twist(branch: Branch): Promise<void> {
    if (this.openShelves.has(branch.key)) {
      this.openShelves.delete(branch.key);
      this.drawTree();
      return;
    }
    this.openShelves.add(branch.key);
    this.drawTree();
    if (this.seforim.has(branch.key)) return;
    const works = await api.shelfWorks(branch.key).catch(() => null);
    if (!works) {
      // A failed read is **not** a shelf with nothing on it, and caching the
      // empty list made this one twist permanently broken: closed-looking,
      // unopenable, saying nothing. Left uncached, the next twist reads again.
      this.openShelves.delete(branch.key);
      this.drawTree();
      return;
    }
    this.seforim.set(branch.key, works);
    // Still open? A reader who twisted it shut while the shelf was being read
    // has said what they want, and redrawing would not change what is on screen
    // anyway — but the list is kept, so the next twist is instant.
    if (this.openShelves.has(branch.key)) this.drawTree();
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
    this.lastFound = cards.slice(0, 12);
    this.found.replaceChildren();
    for (const card of this.lastFound) this.seferRow(this.found, card);
  }

  /** Run an edit, redraw from what Rust actually holds, and say so upward. */
  private async edit(change: () => Promise<ScopeView>): Promise<void> {
    await this.draws.run(change, (scope) => {
      this.draw(scope);
      this.changed();
    });
  }
}
