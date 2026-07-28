// The window: tabs, panes, and the wiring between a scroll and the column
// beside it.
//
// Everything with a decision in it is one directory up, in Rust. What happens
// here is: the reader scrolls, we ask which panes have to move and where, and
// we move them. When the answer is *nowhere*, nothing moves and the pane says
// so — see `girsa_app::beside`.

import {
  api,
  connect,
  isShell,
  whenFilesDropped,
  type AppState,
  type PaneId,
  type Tab,
} from "./api.ts";
import { build } from "./layout.ts";
import { PaneView } from "./pane.ts";
import { Picker } from "./picker.ts";
import { ShelfView } from "./shelf.ts";

const root = document.querySelector<HTMLElement>("#app");
const picker = new Picker();
const shelf = new ShelfView();
const views = new Map<PaneId, PaneView>();
let state: AppState | null = null;
/** The last position each pane reported, so a repeat scroll is not re-asked. */
const reported = new Map<PaneId, string>();
/** Slug → the Hebrew title, so a tab is labelled `ברכות` and not
 * `bavli/berakhot`. Filled in as seforim are opened. */
const titles = new Map<string, string>();

function titleOf(slug: string): string {
  return titles.get(slug) ?? slug;
}

async function main(): Promise<void> {
  if (!root) return;
  await connect();
  root.append(picker.element, shelf.element);
  document.addEventListener("keydown", shortcut);
  await whenFilesDropped(whenDropped);
  await reload();
}

/** Files dropped on the window become seforim (spec.md §5). */
async function whenDropped(paths: string[]): Promise<void> {
  if (paths.length === 0) return;
  if (!shelf.isOpen) await shelf.show(openTab);
  shelf.say(`קורא ${paths.length} קבצים…`, false);
  const dropped = await api.addMine(paths);
  await shelf.refresh();

  const added = dropped.added.map((card) => card.he_title).join(", ");
  // Both halves, always. A drop that half-worked and said nothing leaves a
  // reader believing a sefer is on the shelf when it is not.
  const refused = dropped.refused.map((r) => r.why).join(" · ");
  if (dropped.added.length > 0 && dropped.refused.length === 0) {
    shelf.say(`נוסף: ${added}`, false);
  } else if (dropped.added.length > 0) {
    shelf.say(`נוסף: ${added} — ולא נוסף: ${refused}`, true);
  } else {
    shelf.say(refused || "לא נוסף כלום", true);
  }
}

async function openTab(slug: string): Promise<void> {
  await api.openTab(slug);
  await reload();
}

async function reload(): Promise<void> {
  state = await api.state();
  document.documentElement.style.setProperty("--reading-size", `${state.text_size}%`);
  await draw();
}

function tab(): Tab | null {
  return state?.workspace.tabs[state.workspace.active] ?? null;
}

async function draw(): Promise<void> {
  if (!root || !state) return;
  const chrome = document.createElement("div");
  chrome.className = "app";
  chrome.append(tabBar(), toolBar());

  const open = tab();
  if (!open) {
    chrome.append(nothingOpen());
    root.replaceChildren(chrome, picker.element, shelf.element);
    return;
  }

  const { root: boxes, slots } = build(open.layout, (pane, ratio) => {
    void api.setRatio(pane, ratio);
  });
  boxes.classList.add("panes");
  chrome.append(boxes);
  root.replaceChildren(chrome, picker.element, shelf.element);

  // Panes that are no longer open go, and the ones that stayed keep their
  // scroll position rather than being rebuilt underneath the reader.
  for (const id of [...views.keys()]) {
    if (!open.panes.some((p) => p.id === id)) views.delete(id);
  }

  for (const pane of open.panes) {
    const slot = slots.get(pane.id);
    if (!slot) continue;
    let view = views.get(pane.id);
    const fresh = !view;
    if (!view) {
      view = new PaneView(pane.id, pane.slug, whenMoved, whenFocused);
      views.set(pane.id, view);
      addControls(view, pane.id);
    }
    slot.replaceChildren(view.element);
    view.setFocused(open.focused === pane.id);
    view.setFollowing(followLabel(pane.follows));
    if (fresh) {
      const text = await api.openSefer(pane.slug);
      titles.set(pane.slug, text.work.he_title);
      view.show(text, pane.at ?? null);
      // The tab was drawn before the title was known.
      redrawTabs();
    }
  }
}

function followLabel(leader: PaneId | undefined): string {
  if (leader === undefined) return "";
  const open = tab();
  const of = open?.panes.find((p) => p.id === leader);
  return of ? `עוקב אחרי ${titleOf(of.slug)}` : "";
}

function addControls(view: PaneView, id: PaneId): void {
  const beside = button("לצד", "פתח ספר בטור שלצדו (Ctrl+\)", () => {
    const pane = tab()?.panes.find((p) => p.id === id);
    if (!pane) return;
    picker.openBeside(pane.slug, titleOf(pane.slug), async (slug) => {
      await api.split(id, "vertical", slug, true);
      await reload();
    });
  });
  const unfollow = button("עוקב", "עקוב אחרי הטור שלצדו, או הפסק", async () => {
    const pane = tab()?.panes.find((p) => p.id === id);
    if (!pane) return;
    const others = tab()?.panes.filter((p) => p.id !== id) ?? [];
    const leader = pane.follows === undefined ? (others[0]?.id ?? null) : null;
    await api.setFollows(id, leader);
    await reload();
  });
  const close = button("סגור", "סגור את הטור (Ctrl+W)", async () => {
    await api.closePane(id);
    views.delete(id);
    await reload();
  });
  view.addControl(beside);
  view.addControl(unfollow);
  view.addControl(close);
}

/** A pane reported a new position. Ask what has to move, and move it. */
async function whenMoved(pane: PaneId, at: string): Promise<void> {
  if (!at || reported.get(pane) === at) return;
  reported.set(pane, at);
  const moves = await api.moved(pane, at);
  for (const move of moves) {
    views.get(move.pane)?.goTo(move.place, move.relation);
  }
}

async function whenFocused(pane: PaneId): Promise<void> {
  const open = tab();
  if (!open || open.focused === pane) return;
  open.focused = pane;
  for (const [id, view] of views) view.setFocused(id === pane);
  await api.focus(pane);
}

function redrawTabs(): void {
  document.querySelector(".tabs")?.replaceWith(tabBar());
}

function tabBar(): HTMLElement {
  const bar = document.createElement("nav");
  bar.className = "tabs";
  state?.workspace.tabs.forEach((open, index) => {
    const button = document.createElement("button");
    button.className = "tab" + (index === state?.workspace.active ? " is-active" : "");
    button.textContent = titleOf(open.panes[0]?.slug ?? "—");
    button.addEventListener("click", async () => {
      if (state) state.workspace.active = index;
      const first = open.panes[0];
      if (first) await api.focus(first.id);
      await draw();
    });
    bar.append(button);
  });
  bar.append(button("＋", "פתח ספר (Ctrl+O)", openSomething));
  bar.append(button("מדף", "עיין במדף (Ctrl+B)", browseShelf));
  return bar;
}

function browseShelf(): void {
  void shelf.toggle(openTab);
}

function toolBar(): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "tools";

  const nikud = button(state?.nikud ? "עם ניקוד" : "בלי ניקוד", "Alt+N", async () => {
    if (!state) return;
    await api.setNikud(!state.nikud);
    views.clear();
    await reload();
  });
  nikud.classList.add("tool-wide");

  const smaller = button("א−", "Ctrl+-", () => resize(-10));
  const bigger = button("א+", "Ctrl+=", () => resize(10));

  const where = document.createElement("span");
  where.className = "tools-note";
  if (state?.trouble) {
    where.textContent = state.trouble;
    where.classList.add("is-trouble");
  } else {
    where.textContent = `${state?.works ?? 0} ספרים`;
    if (!isShell()) where.textContent += " · דפדפן, נתוני דוגמה";
  }

  bar.append(nikud, smaller, bigger, where);
  return bar;
}

async function resize(by: number): Promise<void> {
  if (!state) return;
  const next = Math.min(250, Math.max(60, state.text_size + by));
  await api.setTextSize(next);
  await reload();
}

function nothingOpen(): HTMLElement {
  const empty = document.createElement("div");
  empty.className = "empty";
  const title = document.createElement("p");
  title.className = "empty-title";
  title.textContent = "גִּרְסָא";
  const hint = document.createElement("p");
  hint.className = "empty-hint";
  hint.textContent = "Ctrl+O — פתח ספר · Ctrl+B — עיין במדף";
  const open = button("פתח ספר", "Ctrl+O", openSomething);
  open.classList.add("empty-button");
  const browse = button("עיין במדף", "Ctrl+B", browseShelf);
  browse.classList.add("empty-button");
  empty.append(title, hint, open, browse);
  return empty;
}

function openSomething(): void {
  picker.openTab(openTab);
}

function shortcut(event: KeyboardEvent): void {
  if (picker.isOpen) return;
  const control = event.ctrlKey || event.metaKey;
  if (shelf.isOpen && event.key === "Escape") {
    event.preventDefault();
    shelf.close();
    return;
  }
  if (control && event.key.toLowerCase() === "b") {
    event.preventDefault();
    browseShelf();
  } else if (shelf.isOpen) {
    // The shelf is a place, not an overlay on top of the reading: the reading
    // shortcuts are not live while it is open.
  } else if (control && event.key.toLowerCase() === "o") {
    event.preventDefault();
    openSomething();
  } else if (control && event.key === "\\") {
    event.preventDefault();
    const open = tab();
    if (!open) return;
    const pane = open.panes.find((p) => p.id === open.focused);
    if (!pane) return;
    picker.openBeside(pane.slug, titleOf(pane.slug), async (slug) => {
      await api.split(pane.id, "vertical", slug, true);
      await reload();
    });
  } else if (control && event.key.toLowerCase() === "w") {
    event.preventDefault();
    const open = tab();
    if (!open) return;
    void (async () => {
      await api.closePane(open.focused);
      views.delete(open.focused);
      await reload();
    })();
  } else if (event.altKey && event.key.toLowerCase() === "n") {
    event.preventDefault();
    void (async () => {
      if (!state) return;
      await api.setNikud(!state.nikud);
      views.clear();
      await reload();
    })();
  } else if (control && (event.key === "=" || event.key === "+")) {
    event.preventDefault();
    void resize(10);
  } else if (control && event.key === "-") {
    event.preventDefault();
    void resize(-10);
  }
}

function button(label: string, title: string, click: () => void): HTMLElement {
  const node = document.createElement("button");
  node.className = "tool";
  node.textContent = label;
  node.title = title;
  node.addEventListener("click", click);
  return node;
}

void main();
