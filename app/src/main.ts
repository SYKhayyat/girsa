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
  whenAskedToOpen,
  whenAskedToSearch,
  whenFilesDropped,
  type AppState,
  type Landing,
  type PaneId,
  type Presence,
  type Showing,
  type SuspectRow,
  type Tab,
} from "./api.ts";
import { FixBox } from "./fix.ts";
import { build } from "./layout.ts";
import { LinksView } from "./linksview.ts";
import { PaneView } from "./pane.ts";
import { ScanView } from "./scanview.ts";
import { Picker } from "./picker.ts";
import { SearchView } from "./search.ts";
import { ShelfView } from "./shelf.ts";
import { SuspectsView } from "./suspects.ts";
import { WritingView } from "./writing.ts";

const root = document.querySelector<HTMLElement>("#app");
const picker = new Picker();
const shelf = new ShelfView();
const find = new SearchView();
const writing = new WritingView();
const fixbox = new FixBox();
const suspects = new SuspectsView();
const linksview = new LinksView();
const views = new Map<PaneId, PaneView>();
/** Panes holding a scan (W25). A second map rather than a union: a scan has no
 * lines, so none of the questions asked of a reading pane — what is
 * highlighted, which line is this, what corrections are on it — have an answer
 * here, and `views.get` coming back empty for a scan pane is the right answer
 * to every one of them. */
const scans = new Map<PaneId, ScanView>();
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
  root.append(
    picker.element,
    shelf.element,
    find.element,
    writing.element,
    suspects.element,
    linksview.element,
    fixbox.element,
  );
  suspects.onOpen(openSuspect);
  linksview.onOpen(openFound);
  linksview.onHere(whereIAm);
  linksview.onPinTo(whatIsHighlighted);
  // The drawer asks the window for a source, because which pane is focused is
  // the window's business and not the drawer's.
  writing.onSourceWanted(sourceForBuffer);
  document.addEventListener("keydown", shortcut);
  await whenFilesDropped(whenDropped);
  // Ksav, or a citation clicked in a document, asking for a page (§10.6).
  await whenAskedToOpen(whenAskedFor);
  await whenAskedToSearch((phrase) => void find.showPhrase(openFound, phrase));
  await watchForKsav();
  await reload();
}

/** Something asked for a place: Ksav over the loopback, or a `girsa://` link
 * clicked in a Word document or a compiled PDF. It arrives already turned into
 * a segment id, so this is the same landing a search result gets. */
async function whenAskedFor(landing: Landing): Promise<void> {
  await openFound(landing.slug, landing.id);
  say(`נפתח — ${landing.ref}`, false);
}

/** Whether Ksav is there. Polled while the window is open, because the answer
 * changes without anything telling us: Ksav is a separate application and a
 * reader starts and stops it whenever they like. */
let ksav: Presence = { state: "not_running" };

async function watchForKsav(): Promise<void> {
  if (!isShell()) return;
  const look = async (): Promise<void> => {
    const now = await api.ksavPresence();
    // Redrawn only when it changed: a toolbar that rebuilds every five seconds
    // takes the reader's text selection with it.
    if (now.state !== ksav.state) {
      ksav = now;
      document.querySelector(".tools")?.replaceWith(toolBar());
    } else {
      ksav = now;
    }
    writing.setKsav(now);
  };
  await look();
  window.setInterval(() => void look(), 5000);
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
    root.replaceChildren(
      chrome,
      picker.element,
      shelf.element,
      find.element,
      suspects.element,
      linksview.element,
      fixbox.element,
    );
    return;
  }

  const { root: boxes, slots } = build(open.layout, (pane, ratio) => {
    void api.setRatio(pane, ratio);
  });
  boxes.classList.add("panes");
  chrome.append(boxes);
  root.replaceChildren(
    chrome,
    picker.element,
    shelf.element,
    find.element,
    suspects.element,
    linksview.element,
    fixbox.element,
  );

  // Panes that are no longer open go, and the ones that stayed keep their
  // scroll position rather than being rebuilt underneath the reader.
  for (const id of [...views.keys()]) {
    if (!open.panes.some((p) => p.id === id)) views.delete(id);
  }
  for (const id of [...scans.keys()]) {
    if (!open.panes.some((p) => p.id === id)) scans.delete(id);
  }

  for (const pane of open.panes) {
    const slot = slots.get(pane.id);
    if (!slot) continue;
    const held = views.get(pane.id) ?? scans.get(pane.id);
    if (held) {
      slot.replaceChildren(held.element);
      held.setFocused(open.focused === pane.id);
      held.setFollowing(followLabel(pane.follows));
      continue;
    }

    // Which of the two reading modes this is (spec.md §6.2, §6.3). The card
    // says, because the window has to know before it builds a pane — and
    // because a PDF opened into the reading pane is a sefer of blank lines,
    // which is what this window did until W25.
    const text = await api.openSefer(pane.slug);
    titles.set(pane.slug, text.work.he_title);
    // The tab was drawn before the title was known.
    redrawTabs();

    if (text.work.scan) {
      const scan = new ScanView(pane.id, pane.slug, whenMoved, whenFocused);
      scans.set(pane.id, scan);
      addScanControls(scan, pane.id);
      slot.replaceChildren(scan.element);
      scan.setFocused(open.focused === pane.id);
      scan.setFollowing(followLabel(pane.follows));
      const opened = await api.scan(pane.slug);
      await scan.show(opened, opened.at);
      continue;
    }

    const view = new PaneView(pane.id, pane.slug, whenMoved, whenFocused);
    views.set(pane.id, view);
    addControls(view, pane.id);
    slot.replaceChildren(view.element);
    view.setFocused(open.focused === pane.id);
    view.setFollowing(followLabel(pane.follows));
    view.show(text, pane.at ?? null);
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
  const links = button("קישורים", "מה מקושר לשורה הזאת (Ctrl+L)", () => {
    void showLinks();
  });
  // W22: base text + your patches → a file. On the pane, because what is
  // written out is the sefer this pane is reading, corrections and all.
  const save = button("ייצא", "כתוב את הספר לקובץ, עם התיקונים שלך", () => {
    const pane = tab()?.panes.find((p) => p.id === id);
    if (pane) void exportSefer(pane.slug);
  });
  const close = button("סגור", "סגור את הטור (Ctrl+W)", async () => {
    await api.closePane(id);
    views.delete(id);
    scans.delete(id);
    await reload();
  });
  view.addControl(beside);
  view.addControl(unfollow);
  view.addControl(links);
  view.addControl(save);
  view.addControl(close);
}

/**
 * The buttons a scan's header carries.
 *
 * Fewer than a reading pane's, and the missing ones are missing for a reason: a
 * scan has no lines to correct, no links on its words yet, and nothing to
 * export that is not the file the reader already has. A button that did
 * nothing would be a button that teaches the reader the buttons lie.
 */
function addScanControls(view: ScanView, id: PaneId): void {
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
    scans.delete(id);
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
    // A scan has no lines to scroll to: where it goes is a page, counted in
    // Rust. `undefined` is *the pane stays where it is*, which is what a daf
    // this scan does not carry has to do (W9's `NoPlace`, in a photograph).
    scans.get(move.pane)?.turnTo(move.page ?? null, move.place.kind);
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
  bar.append(button("חפש", "חפש בכל המדף (Ctrl+F)", search));
  bar.append(button("כתוב", "פתח את הכתיבה (Ctrl+E)", () => void writing.toggle()));
  // The queue, where there is one. Not shown at all when the batch job has
  // never been run: a button that opens an empty list teaches the reader that
  // the feature does nothing.
  if ((state?.suspects ?? 0) > 0) {
    bar.append(
      button(
        `טעויות ${state?.suspects ?? 0}`,
        "תור שגיאות הסריקה (Ctrl+J)",
        () => void suspects.toggle(),
      ),
    );
  }
  return bar;
}

function browseShelf(): void {
  void shelf.toggle(openTab);
}

function search(): void {
  void find.toggle(openFound);
}

/// A result, opened: the sefer in a tab, at the segment that was found.
///
/// The scroll goes through the same `goTo` the commentary column uses, so a hit
/// lands on the line by its **permanent id** and not by counting lines — which
/// is the whole of W6 showing up in a place nobody would think to look.
async function openFound(slug: string, id: string | null, marked?: string[]): Promise<void> {
  await api.openTab(slug);
  await reload();
  // No id is *open it where it was left*: a scan whose pages nobody has read
  // has no segment worth landing on, and the pane it opens is the one carrying
  // the control that reads it (W26).
  if (id === null) return;
  const open = tab();
  const pane = open?.panes.find((p) => p.slug === slug);
  if (!pane) return;
  views.get(pane.id)?.goTo({ kind: "at", ids: [id] }, "linked");
  // A hit on a page of a scan is highlighted with a rectangle on the
  // photograph rather than a span of text — spec.md §9.7's *only the highlight
  // differs*. The words come from the search that ran, not from what was typed.
  const scan = scans.get(pane.id);
  if (scan && marked?.length) scan.markWords(marked.map(bareWord));
}

/** A word without its marks, for comparing with what is drawn on a page. */
function bareWord(word: string): string {
  return word.replace(/[\u0591-\u05C7]/gu, "");
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

  const showing = button(showingSaid(state?.showing ?? "fixed"), showingWhy(), () => {
    void nextShowing();
  });
  showing.classList.add("tool-wide");
  if ((state?.fixes ?? 0) === 0) showing.classList.add("is-quiet");

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

  bar.append(nikud, showing, smaller, bigger, where);

  // Presence (spec.md §10.6): the affordance is never offered when it would
  // fail. Live, it is a button; not live, it is a word saying which of the two
  // reasons it is.
  if (isShell()) {
    if (ksav.state === "live") {
      const send = button("שלח לכסב", "Ctrl+Shift+C — שלח את הבחירה למסמך הפתוח", () =>
        void sendToKsav(),
      );
      send.classList.add("tool-wide");
      bar.append(send);
    } else {
      const off = document.createElement("span");
      off.className = "tools-note";
      off.textContent = ksav.state === "stale" ? `כסב — ${ksav.why}` : "כסב אינו פועל";
      if (ksav.state === "stale") off.classList.add("is-trouble");
      bar.append(off);
    }
  }
  return bar;
}

/** What the three settings are called, and what each one promises. */
function showingSaid(showing: Showing): string {
  if (showing === "as_printed") return "כפי שנדפס";
  if (showing === "fixed_with_variants") return "עם גרסאות";
  return "מתוקן";
}

function showingWhy(): string {
  return (
    "מתוקן — טעויות דפוס מתוקנות, גרסאות נרשמות בלבד · " +
    "כפי שנדפס — הטקסט המקורי · עם גרסאות — גם ההגהות מוחלות (Ctrl+Shift+K)"
  );
}

/** Round the three states (spec.md §7.1, §7.2). Everything open is redrawn,
 * because the words themselves changed. */
async function nextShowing(): Promise<void> {
  if (!state) return;
  const order: Showing[] = ["fixed", "as_printed", "fixed_with_variants"];
  const next = order[(order.indexOf(state.showing) + 1) % order.length];
  await api.setShowing(next);
  views.clear();
  await reload();
  say(showingSaid(next), false);
}

/**
 * Correct a typo, from where you are reading (spec.md §7.5, W20).
 *
 * With something highlighted, that is what is corrected. With nothing
 * highlighted, the box opens on the line the reader is standing on, showing
 * what is already there — which is how a correction is taken back.
 */
function correct(): void {
  const open = tab();
  if (!open) return;
  const view = views.get(open.focused);
  if (!view) return;
  if (fixbox.isOpen) {
    fixbox.close();
    return;
  }

  const chosen = view.fixSelection();
  const where = window.getSelection();
  const near =
    where && where.rangeCount > 0 && !where.isCollapsed
      ? where.getRangeAt(0).getBoundingClientRect()
      : null;

  if (chosen) {
    fixbox.show(chosen, near, {
      save: (now, kind) => applyFix(view, chosen.at, chosen.fromChar, chosen.toChar, now, kind),
      revert: (patch) => revertFix(view, chosen.at, patch),
    });
    return;
  }

  // Nothing highlighted: the line the reader is on, and its corrections. There
  // is nothing to correct without a highlight, so this is the way back — and
  // it says so rather than opening an empty box.
  const here = view.fixesHere();
  if (!here || here.fixed.length === 0) {
    say("סמן את המילה שצריכה תיקון", false);
    return;
  }
  fixbox.show(
    { at: here.at, fromChar: 0, toChar: 0, words: "", fixed: here.fixed, printed: here.printed },
    null,
    {
      save: async () => say("סמן את המילה שצריכה תיקון", false),
      revert: (patch) => revertFix(view, here.at, patch),
    },
  );
}

/**
 * Open a candidate from the queue (W21).
 *
 * The place, the word marked, and the correction box on it with the common
 * spelling suggested — and **nothing applied**. The reader is looking at the
 * sefer while they decide, which is the whole reason the queue points at a
 * place rather than offering a button that says *fix*.
 */
async function openSuspect(row: SuspectRow): Promise<void> {
  if (!row.at || !row.work) return;
  await openFound(row.work, row.at);
  const open = tab();
  const view = open?.panes.find((p) => p.slug === row.work);
  const pane = view ? views.get(view.id) : undefined;
  if (!pane) return;
  try {
    const standing = await api.suspectAt(row.id, row.at);
    const at = pane.markWord(standing.at, standing.from_char, standing.to_char);
    fixbox.show(
      {
        at: standing.at,
        fromChar: standing.from_char,
        toChar: standing.to_char,
        words: standing.suggestion ?? standing.printed,
        fixed: [],
        printed: standing.printed,
      },
      at,
      {
        save: async (now, kind) => {
          await applyFix(pane, standing.at, standing.from_char, standing.to_char, now, kind);
          await api.suspectDecide(row.id, "fixed");
          suspects.taken(row.id);
        },
        revert: async () => {},
      },
    );
  } catch (e) {
    say(String(e), true);
  }
}

async function applyFix(
  view: PaneView,
  at: string,
  fromChar: number,
  toChar: number,
  now: string,
  kind: "ocr" | "girsa",
): Promise<void> {
  try {
    const fixed = await api.fix(at, fromChar, toChar, now, kind);
    view.replaceLine(fixed.line);
    say(`${kind === "ocr" ? "תוקן" : "נרשמה גרסה"} — ${fixed.said}`, false);
    if (state) state.fixes += 1;
  } catch (e) {
    // A refusal is shown as it came: "there is already a correction here" and
    // "nothing is selected" are different things to a reader.
    say(String(e), true);
  }
}

async function revertFix(view: PaneView, at: string, patch: string): Promise<void> {
  try {
    const fixed = await api.unfix(at, patch);
    view.replaceLine(fixed.line);
    say(fixed.said, false);
    if (state) state.fixes = Math.max(0, state.fixes - 1);
  } catch (e) {
    say(String(e), true);
  }
}

/** The segment the focused pane is standing on. */
function whereIAm(): string | null {
  const open = tab();
  if (!open) return null;
  return views.get(open.focused)?.here() ?? null;
}

/**
 * What is linked to the line you are on (spec.md §8.3, W23).
 *
 * The line the reader is standing on, not a selection: a link is on a segment,
 * and asking about "the highlighted part" would be W24's question.
 */
async function showLinks(): Promise<void> {
  const open = tab();
  if (!open) return;
  const view = views.get(open.focused);
  // A highlight asks a narrower question: *which links are on these words*
  // (spec.md §8.4). With nothing highlighted it is the whole line.
  const chosen = view?.fixSelection();
  const at = chosen?.at ?? whereIAm();
  if (!at) return;
  await linksview.toggle(at, chosen ? [chosen.fromChar, chosen.toChar] : null);
}

/** The highlight in the focused pane, as offsets in its line. */
function whatIsHighlighted(): [number, number] | null {
  const open = tab();
  if (!open) return null;
  const chosen = views.get(open.focused)?.fixSelection();
  return chosen ? [chosen.fromChar, chosen.toChar] : null;
}

/** Write the sefer out with your corrections in it (spec.md §7.4). */
async function exportSefer(slug: string): Promise<void> {
  try {
    const written = await api.exportSefer(slug, "docx");
    // The path, because the file is the point and a reader has to be able to
    // find it — and what did *not* land, because exporting is the moment
    // somebody would otherwise never hear about a stale correction.
    const trouble = written.stale > 0 ? ` · ${written.stale} תיקונים לא חלו` : "";
    say(`נכתב — ${written.said}${trouble} · ${written.path}`, written.stale > 0);
  } catch (e) {
    say(String(e), true);
  }
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
  hint.textContent = "Ctrl+O — פתח ספר · Ctrl+B — עיין במדף · Ctrl+F — חפש · Ctrl+K — תקן";
  const open = button("פתח ספר", "Ctrl+O", openSomething);
  open.classList.add("empty-button");
  const browse = button("עיין במדף", "Ctrl+B", browseShelf);
  browse.classList.add("empty-button");
  const look = button("חפש", "Ctrl+F", search);
  look.classList.add("empty-button");
  empty.append(title, hint, open, browse, look);
  return empty;
}

function openSomething(): void {
  picker.openTab(openTab);
}

function shortcut(event: KeyboardEvent): void {
  if (picker.isOpen) return;
  const control = event.ctrlKey || event.metaKey;
  // While the correction box is open the keyboard is its own — it is a text
  // box, and Ctrl+C in it is copy.
  if (fixbox.isOpen && fixbox.element.contains(event.target as Node)) return;
  // While the caret is in the buffer, the keyboard belongs to the buffer.
  // Ctrl+C there is *copy*, not copy-a-source, and Alt+N is a letter somebody
  // is typing — the reading shortcuts are not live inside a text box.
  if (writing.isOpen && writing.element.contains(event.target as Node)) {
    if (event.key === "Escape") {
      event.preventDefault();
      writing.close();
    } else if (control && event.key.toLowerCase() === "e") {
      event.preventDefault();
      writing.close();
    }
    return;
  }
  if (linksview.isOpen && event.key === "Escape") {
    event.preventDefault();
    linksview.close();
    return;
  }
  if (suspects.isOpen && event.key === "Escape") {
    event.preventDefault();
    suspects.close();
    return;
  }
  if (find.isOpen && event.key === "Escape") {
    event.preventDefault();
    find.close();
    return;
  }
  if (control && event.key.toLowerCase() === "f") {
    event.preventDefault();
    search();
    return;
  }
  if (find.isOpen) {
    // The search is a place, like the shelf: the reading shortcuts are not
    // live while it is open, and a typed letter goes into the query box.
    return;
  }
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
  } else if (control && event.key.toLowerCase() === "e") {
    event.preventDefault();
    void writing.toggle();
  } else if (writing.isOpen && event.key === "Escape") {
    event.preventDefault();
    writing.close();
  } else if (control && event.key.toLowerCase() === "l") {
    event.preventDefault();
    void showLinks();
  } else if (control && event.key.toLowerCase() === "j") {
    event.preventDefault();
    void suspects.toggle();
  } else if (control && event.shiftKey && event.key.toLowerCase() === "k") {
    event.preventDefault();
    void nextShowing();
  } else if (control && event.key.toLowerCase() === "k") {
    event.preventDefault();
    correct();
  } else if (control && event.shiftKey && event.key.toLowerCase() === "c") {
    event.preventDefault();
    void sendToKsav();
  } else if (control && event.key.toLowerCase() === "c") {
    // **The user does nothing different** (spec.md §10.2). Ctrl+C is Ctrl+C;
    // what changes is what lands on the clipboard beside the text. The default
    // is not prevented — if this fails, the webview's own copy still happens.
    void copySource();
  } else if (control && (event.key === "=" || event.key === "+")) {
    event.preventDefault();
    void resize(10);
  } else if (control && event.key === "-") {
    event.preventDefault();
    void resize(-10);
  }
}

/**
 * Ctrl+C: the quote, the citation, and the source packet (BUILDER.md W15).
 *
 * With something highlighted, only that goes. With nothing highlighted, the
 * line the reader is standing on goes — which is what Ctrl+C does in every
 * list of things ever written, and is the same call with the offsets left off.
 */
async function copySource(): Promise<void> {
  const open = tab();
  if (!open) return;

  // A page of a scan is a **mareh makom** and not a quote: there is nothing to
  // quote off a photograph nobody has OCR'd, and the importer will not invent
  // Hebrew it cannot read. What goes down is the citation and the ref.
  const scan = scans.get(open.focused);
  if (scan) {
    try {
      const cited = await api.scanCopy(scan.slug, scan.here());
      if (cited.put.trouble) say(cited.put.trouble, true);
      else say(`הועתק — ${cited.display}`, false);
    } catch (e) {
      say(String(e), true);
    }
    return;
  }

  const view = views.get(open.focused);
  if (!view) return;

  const chosen = view.selection();
  let copied;
  if (chosen) {
    copied = await api.copy(chosen.from, chosen.to, chosen.fromChar, chosen.toChar);
  } else {
    const here = view.here();
    if (!here) return;
    copied = await api.copy(here, here, 0, null);
  }

  if (copied.put.trouble) {
    say(copied.put.trouble, true);
    return;
  }
  // Named, not "copied": a reader should be able to see from the confirmation
  // that they took the place they meant, without pasting it somewhere to look.
  const lines = copied.lines > 1 ? ` · ${copied.lines} שורות` : "";
  say(`הועתק — ${copied.display}${lines}`, false);
}

/**
 * Straight into the open Ksav document (spec.md §10.2).
 *
 * The clipboard path works whether or not Ksav is running. This one is the
 * AirDrop one, and it is only reachable when presence says it would land.
 */
async function sendToKsav(): Promise<void> {
  const open = tab();
  if (!open) return;
  const view = views.get(open.focused);
  if (!view) return;
  const chosen = view.selection();
  const here = chosen ? null : view.here();
  if (!chosen && !here) return;

  try {
    const sent = chosen
      ? await api.sendToKsav(chosen.from, chosen.to, chosen.fromChar, chosen.toChar)
      : await api.sendToKsav(here!, here!, 0, null);
    say(`נשלח לכסב — ${sent.display}`, false);
  } catch (e) {
    // A refusal from the other side is shown as it came: "Ksav is not running"
    // and "Ksav refused it" are different things to a reader.
    say(String(e), true);
  }
}

/**
 * The passage the reader has highlighted, as real Ksav markup.
 *
 * Built in Rust by `girsa-ksav` — the same writer Ksav compiles — so a quote
 * written into the buffer and a quote sent over the loopback are the same
 * markup (spec.md §10.3).
 */
async function sourceForBuffer(): Promise<string | null> {
  const open = tab();
  if (!open) return null;
  const view = views.get(open.focused);
  if (!view) return null;
  const chosen = view.selection();
  if (chosen) {
    return api.sourceMarkup(chosen.from, chosen.to, chosen.fromChar, chosen.toChar);
  }
  const here = view.here();
  return here ? api.sourceMarkup(here, here, 0, null) : null;
}

/** A line the window says and then stops saying. */
function say(words: string, trouble: boolean): void {
  if (!root) return;
  let toast = root.querySelector<HTMLElement>(".said");
  if (!toast) {
    toast = document.createElement("p");
    toast.className = "said";
    root.append(toast);
  }
  toast.textContent = words;
  toast.classList.toggle("is-trouble", trouble);
  toast.classList.add("is-on");
  window.clearTimeout(Number(toast.dataset.timer ?? 0));
  toast.dataset.timer = String(
    window.setTimeout(() => toast?.classList.remove("is-on"), 4000),
  );
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
