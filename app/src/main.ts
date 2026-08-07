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
  type Asked,
  type Mefarshim,
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
import { LanePanel } from "./laneview.ts";
import { SettingsView, applyLook } from "./settingsview.ts";
import { SuspectsView } from "./suspects.ts";
import { WritingView } from "./writing.ts";
import { YoursView } from "./yoursview.ts";
import { KSAV, type Named, sefer, speak, withPrefix } from "./names.ts";
import { doorLabel, doorTitle, nothingHere } from "./mefarshim.ts";
import { presenceSaid } from "./presence.ts";
import { whatKey, type Pressed } from "./keys.ts";
import { announces, button, glyph, region } from "./controls.ts";

const root = document.querySelector<HTMLElement>("#app");
const picker = new Picker();
const shelf = new ShelfView();
const find = new SearchView();
const writing = new WritingView();
const fixbox = new FixBox();
const suspects = new SuspectsView();
const linksview = new LinksView();
const yoursview = new YoursView();
/** The semantic lane's settings (spec.md §9.9, W30). Off in a fresh install,
 * and off costs nothing — so the panel is always reachable and never nags. */
const lanepanel = new LanePanel();
/** The settings panel Girsa did not have (B13). */
const settingsview = new SettingsView();
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
/**
 * Slug → **both** of a sefer's titles, so a tab is labelled `ברכות` and not
 * `bavli/berakhot`. Filled in as seforim are opened.
 *
 * Both, and not the rendered name, because which of the two to print is a pure
 * function of data the window already holds — `titleIn`. Holding only the
 * rendered one is why switching the interface language used to **re-fetch every
 * open sefer over IPC**, an 18,000-segment Mishnah Berurah included, to change
 * which of two strings a tab prints.
 */
const named = new Map<string, Named>();

function titleOf(slug: string): string {
  const held = named.get(slug);
  return held ? sefer(held) : slug;
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
    yoursview.element,
    lanepanel.element,
    settingsview.element,
    fixbox.element,
  );
  settingsview.onChanged(() => {
    // Everything on that panel can change how a sefer is drawn, so the panes are
    // rebuilt rather than patched.
    views.clear();
    scans.clear();
    void reload();
  });
  suspects.onOpen(openSuspect);
  linksview.onOpen(openFound);
  linksview.onHere(whereIAm);
  linksview.onPinTo(whatIsHighlighted);
  // The drawer asks the window for a source, because which pane is focused is
  // the window's business and not the drawer's.
  writing.onSourceWanted(sourceForBuffer);
  // Your own layer (W27). The drawer asks the window where to go and how to
  // ask a question again, for the same reason the links panel does: which pane
  // is focused is the window's business.
  yoursview.onOpen(openFound);
  yoursview.onAsk((typed) => find.askAgain(openFound, typed));
  yoursview.onChanged(repaintMarks);
  find.onKeep(keepQuery);
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
async function whenAskedFor(landing: Asked): Promise<void> {
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

  const added = dropped.added.map(sefer).join(", ");
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
  // The language the window is in (W41), set once from the session so that every
  // `sefer()` in every module answers the same way. Rust holds the setting; this
  // is the one place the window is told.
  speak(state.language);
  // How the reading looks (B13): theme, the two fonts, leading and measure. On the
  // document as custom properties, so `styles.css` keeps owning the appearance.
  applyLook(state.look);
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
      yoursview.element,
      fixbox.element,
    );
    return;
  }

  const { root: boxes, slots } = build(open.layout, state.share_bounds, (pane, ratio) => {
    void api.setRatio(pane, ratio);
  });
  boxes.classList.add("panes");
  boxes.setAttribute("role", "main");
  boxes.setAttribute("aria-label", "הקריאה");
  chrome.append(boxes);
  root.replaceChildren(
    chrome,
    picker.element,
    shelf.element,
    find.element,
    suspects.element,
    linksview.element,
    yoursview.element,
    lanepanel.element,
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
    named.set(pane.slug, text.work);
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
    // Your highlights, where Rust placed them against the same lines this pane
    // was just sent (W27). One call for the sefer rather than one a line.
    view.setMarks(await api.marksIn(pane.slug));
    // And which lines the mefarshim you ticked speak on (W43). Also one call:
    // `inbound.jsonl` is one file per sefer, and reading Berakhot's 21,065 rows
    // into per-line answers takes 0.07s.
    view.whenComments((at) => void openComments(view, at));
    await drawMefarshim(view);
  }
}

/** Ask again where your highlights land, after your layer changed. */
async function repaintMarks(): Promise<void> {
  for (const view of views.values()) {
    try {
      view.setMarks(await api.marksIn(view.slug));
    } catch {
      // A pane whose sefer has gone — a note you just deleted — has no marks
      // to draw and is about to be closed by the reload that follows.
    }
  }
}

/**
 * The door, doing both jobs.
 *
 * One list: click a row and that sefer opens in the column beside this one, which
 * is the split the reader asked to keep; tick a row and its comments are marked
 * on the lines they are about. Both were asked for in the same breath, so they
 * are behind the same button.
 */
async function openMefarshim(id: PaneId): Promise<void> {
  const pane = tab()?.panes.find((p) => p.id === id);
  if (!pane) return;
  const slug = pane.slug;
  picker.openBeside({
    slug,
    title: titleOf(slug),
    mefarshim: await mefarshimFor(slug),
    chosen: async (opened) => {
      await api.split(id, "vertical", opened, true);
      await reload();
    },
    tick: (work, on) => void tickMefaresh(slug, work, on),
  });
}

/** Tick one mefaresh, and redraw the markers on every pane reading this sefer. */
async function tickMefaresh(slug: string, work: string, on: boolean): Promise<void> {
  const held = await mefarshimFor(slug);
  const marked = await api.chooseMefaresh(slug, work, on);
  // Rust owns which lines are marked; this only records what was ticked, so the
  // next opening of the list draws the boxes the reader left.
  mefarshimOf.set(slug, {
    ...held,
    marked,
    works: held.works.map((w) => (w.slug === work ? { ...w, chosen: on } : w)),
  });
  for (const view of views.values()) {
    if (view.slug === slug) await drawMefarshim(view);
  }
}

/**
 * Which mefarshim can be placed on which sefer, and which the reader ticked.
 *
 * Kept per sefer rather than per pane: the same masechta open in two panes has
 * one set of ticked mefarshim, because the reader ticked them on the sefer and
 * not on a column.
 */
const mefarshimOf = new Map<string, Mefarshim>();

/** Read the tick-list for a sefer, once. */
async function mefarshimFor(slug: string): Promise<Mefarshim> {
  const held = mefarshimOf.get(slug);
  if (held) return held;
  try {
    const read = await api.mefarshim(slug);
    mefarshimOf.set(slug, read);
    return read;
  } catch {
    // No link graph, or a sefer with no inbound cache. An empty tick-list is the
    // honest answer and `ticked()` says which kind of empty it is.
    return { works: [], alongside: [], folders: [], listed: [], marked: [], touched: 0, unbuilt: null };
  }
}

/** Draw the markers on a pane, for whatever is ticked now. */
async function drawMefarshim(view: PaneView): Promise<void> {
  const on = await mefarshimFor(view.slug);
  view.setMefarshim(
    on.marked,
    on.works.filter((w) => w.chosen).length,
  );
}

/**
 * A click on a line: what the ticked mefarshim say about it (W43).
 *
 * Otzaria's model, and the half the reader asked for that the split does not
 * answer — *of the six mefarshim I follow, which said something about **this
 * line**, and what?* The comments open under the line, not in a panel over it.
 */
async function openComments(view: PaneView, at: string): Promise<void> {
  const on = await mefarshimFor(view.slug);
  const chosen = on.works.filter((w) => w.chosen).length;
  try {
    const comments = await api.mefarshimAt(view.slug, at);
    view.showSaid(at, comments.said, nothingHere(comments, chosen));
  } catch (e) {
    // A read that failed is not *nobody wrote here*, and must not be shown as it.
    view.showSaid(at, [], `לא הצלחתי לקרוא את המפרשים: ${e}`);
  }
}

/**
 * Put the mefarshim count on a pane's beside-button.
 *
 * Failure is silent and that is deliberate: the button already works and its
 * fallback label is honest. A pane header is not the place to report that a
 * cache is cold — `linksview` already says that where it matters, in a sentence.
 */
async function nameTheDoor(control: HTMLElement, id: PaneId): Promise<void> {
  const pane = tab()?.panes.find((p) => p.id === id);
  if (!pane) return;
  try {
    const companions = await api.companions(pane.slug);
    control.textContent = doorLabel(companions);
    control.title = doorTitle(companions);
  } catch {
    // Leave `לצד` and the tooltip that came with it.
  }
}

function followLabel(leader: PaneId | undefined): string {
  if (leader === undefined) return "";
  const open = tab();
  const of = open?.panes.find((p) => p.id === leader);
  return of ? `עוקב אחרי ${titleOf(of.slug)}` : "";
}

function addControls(view: PaneView, id: PaneId): void {
  const beside = button("לצד", doorTitle([]), () => {
    void openMefarshim(id);
  });
  // Named after what is behind it, once the shelf has said what that is.
  //
  // *"i have no clue how to even open mefarshim"* was this button — working,
  // opening a list that marks each declared commentary `פירוש`, and labelled
  // `לצד`: a preposition. Named after the round trip rather than before it,
  // because a header that waits on the shelf before drawing is worse than one
  // whose label sharpens a moment later.
  void nameTheDoor(beside, id);
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
  const beside = button("לצד", doorTitle([]), () => {
    void openMefarshim(id);
  });
  // A scan of a daf has mefarshim like any other copy of that daf, so it gets
  // the same name on the same button. Fixing one and not the other is how the
  // label drifts back apart.
  void nameTheDoor(beside, id);
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
  bar.setAttribute("aria-label", "לשוניות");
  state?.workspace.tabs.forEach((open, index) => {
    const named = titleOf(open.panes[0]?.slug ?? "—");
    const holder = document.createElement("span");
    holder.className = "tab-holder" + (index === state?.workspace.active ? " is-active" : "");
    const go = document.createElement("button");
    go.className = "tab";
    go.textContent = named;
    go.addEventListener("click", async () => {
      if (state) state.workspace.active = index;
      const first = open.panes[0];
      if (first) await api.focus(first.id);
      await draw();
    });
    // W40: *"needs a way to close tab without going in."* Named after the sefer
    // it closes, because `×` is a glyph and a glyph is not a name — B14's guard
    // is about exactly this, and a strip of eight identical × buttons is a strip
    // a screen reader cannot tell apart.
    const shut = glyph("×", `סגור ${named}`, () => {
      void (async () => {
        await api.closeTab(index);
        for (const pane of open.panes) {
          views.delete(pane.id);
          scans.delete(pane.id);
        }
        await reload();
      })();
    });
    shut.classList.add("tab-shut");
    holder.append(go, shut);
    bar.append(holder);
  });
  bar.append(button("＋", "פתח ספר (Ctrl+O)", openSomething));
  bar.append(button("מדף", "עיין במדף (Ctrl+B)", browseShelf));
  bar.append(button("חפש", "חפש בכל המדף (Ctrl+F)", search));
  bar.append(button("כתוב", "פתח את הכתיבה (Ctrl+E)", () => void writing.toggle()));
  // The semantic lane (spec.md §9.9, W30). Always here, whether or not it is
  // on: it is a setting rather than a queue, and a reader who has never met it
  // needs somewhere to meet it. Standing beside it is the sefer in the focused
  // pane, so *put this one in the lane* has something to name.
  bar.append(
    button("לשון סמוכה", "הלשון הסמוכה — מציאה לפי עניין (Ctrl+Shift+L)", openLane),
  );
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
  // A landmark with a name, so a reader can reach the toolbar without walking the
  // tab strip first (B14).
  const bar = region("toolbar", "כלים", "tools");

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

  // W41. *"hebrew and english ui. all seforim names in hebrew ui should be heb
  // all in english ui should be english."* The control names the language it
  // switches **to**, because a button labelled with the state you are already in
  // is a button nobody can predict.
  const language = button(
    state?.language === "english" ? "עברית" : "English",
    state?.language === "english"
      ? "החזר את שמות הספרים לעברית"
      : "name every sefer in English",
    async () => {
      if (!state) return;
      await api.setLanguage(state.language === "english" ? "hebrew" : "english");
      // The panes are **kept**. Every header carries a sefer's name, and which
      // name that is is `titleIn` over the two titles this window already
      // holds — so redrawing is enough, and clearing the views made the window
      // re-fetch every open sefer over IPC to change one string. Mishnah
      // Berurah is 18,120 segments.
      await reload();
    },
  );
  language.classList.add("tool-wide");

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

  // B13. A panel, and a way to reach it — the reading settings that used to be
  // four buttons and nothing else.
  const setup = button("הגדרות", "הגדרות הקריאה (Ctrl+,)", () => void settingsview.toggle());
  bar.append(nikud, language, showing, smaller, bigger, setup, where);

  // Presence (spec.md §10.6): the affordance is never offered when it would
  // fail. Live, it is a button; not live, it is a word saying which of the two
  // reasons it is.
  if (isShell()) {
    const said = presenceSaid(ksav);
    if (said.canSend) {
      const send = button(
        `שלח ${withPrefix("ל", KSAV)}`,
        "Ctrl+Shift+C — שלח את הבחירה למסמך הפתוח",
        () => void sendToKsav(),
      );
      send.classList.add("tool-wide");
      bar.append(send);
    } else {
      // Three states, three sentences, and the transport's own English behind the
      // hover rather than in the chip. `presenceSaid` decides all of it, so the
      // toolbar cannot invent a fourth wording.
      const off = document.createElement("span");
      off.className = "tools-note";
      off.textContent = said.said;
      if (said.detail) off.title = said.detail;
      if (said.trouble) off.classList.add("is-trouble");
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

/**
 * The semantic lane, from the button or from the key (B13).
 *
 * One function, because the tooltip used to say `Ctrl+L` and nothing was wired to
 * it — the links panel had that key. A button and a shortcut that are two copies of
 * one action is how one of them stops matching the label.
 */
function openLane(): void {
  const open = tab();
  const here = open?.panes.find((pane) => pane.id === open.focused)?.slug ?? null;
  lanepanel.standing(here ? { slug: here, title: titleOf(here) } : null);
  void lanepanel.toggle();
}

/** Only the parts of a keyboard event a binding is made of. */
function asPressed(event: KeyboardEvent): Pressed {
  return {
    key: event.key,
    ctrlKey: event.ctrlKey,
    metaKey: event.metaKey,
    shiftKey: event.shiftKey,
    altKey: event.altKey,
  };
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
  if (yoursview.isOpen && yoursview.element.contains(event.target as Node)) {
    // A note is edited in a text box in this drawer; the reading shortcuts are
    // not live inside one.
    if (event.key === "Escape") {
      event.preventDefault();
      yoursview.close();
    }
    return;
  }
  if (yoursview.isOpen && event.key === "Escape") {
    event.preventDefault();
    yoursview.close();
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
  // B13. What the reader asked for, from the table in `girsa_app::keys` with their
  // own rebindings over it. It used to be eighteen comparisons against letters
  // written in place, which is why there was nothing to rebind and why two
  // tooltips could both claim Ctrl+L with only one of them wired.
  // `Ctrl++` and `Ctrl+=` are the same key to a reader and different keys to a
  // keyboard, so one is spelled as the other before the table is asked.
  const pressed = event.key === "+" ? { ...asPressed(event), key: "=" } : asPressed(event);
  const did = whatKey(state?.keys ?? {}, pressed);
  if (did === "shelf") {
    event.preventDefault();
    browseShelf();
    return;
  }
  if (shelf.isOpen) {
    // The shelf is a place, not an overlay on top of the reading: the reading
    // shortcuts are not live while it is open.
    return;
  }
  switch (did) {
    case "open":
      event.preventDefault();
      openSomething();
      return;
    case "beside": {
      event.preventDefault();
      const open = tab();
      if (open) void openMefarshim(open.focused);
      return;
    }
    case "close-pane": {
      event.preventDefault();
      const open = tab();
      if (!open) return;
      void (async () => {
        await api.closePane(open.focused);
        views.delete(open.focused);
        await reload();
      })();
      return;
    }
    case "nikud":
      event.preventDefault();
      void (async () => {
        if (!state) return;
        await api.setNikud(!state.nikud);
        views.clear();
        await reload();
      })();
      return;
    case "write":
      event.preventDefault();
      void writing.toggle();
      return;
    case "links":
      event.preventDefault();
      void showLinks();
      return;
    case "lane":
      event.preventDefault();
      openLane();
      return;
    case "queue":
      event.preventDefault();
      void suspects.toggle();
      return;
    case "mine":
      event.preventDefault();
      void yoursview.toggle();
      return;
    case "note":
      event.preventDefault();
      void noteHere();
      return;
    case "mark":
      event.preventDefault();
      void markHere(false);
      return;
    case "highlight":
      event.preventDefault();
      void markHere(true);
      return;
    case "showing":
      event.preventDefault();
      void nextShowing();
      return;
    case "fix":
      event.preventDefault();
      correct();
      return;
    case "send":
      event.preventDefault();
      void sendToKsav();
      return;
    case "copy":
      // **The user does nothing different** (spec.md §10.2). Ctrl+C is Ctrl+C;
      // what changes is what lands on the clipboard beside the text. The default
      // is not prevented — if this fails, the webview's own copy still happens.
      void copySource();
      return;
    case "bigger":
      event.preventDefault();
      void resize(10);
      return;
    case "smaller":
      event.preventDefault();
      void resize(-10);
      return;
    case "settings":
      event.preventDefault();
      void settingsview.toggle();
      return;
    default:
      // A press nobody bound. Left alone, because a reader typing is not asking
      // for anything.
      return;
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
    say(`נשלח ${withPrefix("ל", KSAV)} — ${sent.display}`, false);
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

// ── Your own layer (spec.md §11, BUILDER.md W27) ───────────────────────────
//
// Note what is **not** here: nothing asks for "my notes on this line". That
// comes back from `showLinks` above, because a note's connection to a sugya is
// a link. What these do is the writing.

/**
 * Write a note about where you are standing (spec.md §11).
 *
 * The three-second one (§7.5's guardrail, inherited): the place is where the
 * reader already is, the title is the first words unless they give one, and
 * there is no *which notebook* to answer. With something highlighted, the note
 * is on that line and the highlight becomes a mark on the same words — which
 * is what highlighting-then-writing means everywhere else.
 */
async function noteHere(): Promise<void> {
  const open = tab();
  if (!open) return;
  const view = views.get(open.focused);
  const at = view?.here();
  if (!at) {
    say("אין כאן שורה לכתוב עליה", true);
    return;
  }
  const text = window.prompt("מה יש לך לומר?", "");
  if (text === null || text.trim() === "") return;
  try {
    const note = await api.noteWrite(at, text);
    say(`נכתב: ${note.title}`, false);
    // The note is a sefer now, so the shelf and the tabs know one more thing.
    await reload();
    if (linksview.isOpen) await linksview.show(at);
    await yoursview.refresh();
  } catch (e) {
    say(String(e), true);
  }
}

/** Highlight the words that are selected, or mark the line you are on. */
async function markHere(bookmark: boolean): Promise<void> {
  const open = tab();
  if (!open) return;
  const view = views.get(open.focused);
  if (!view) return;
  const chosen = bookmark ? null : view.fixSelection();
  const at = chosen?.at ?? view.here();
  if (!at) return;
  try {
    const mark = await api.markHere(
      at,
      chosen ? [chosen.fromChar, chosen.toChar] : undefined,
    );
    say(mark.kind === "bookmark" ? "סימנייה" : `סומן: ${mark.was}`, false);
    await repaintMarks();
    await yoursview.refresh();
  } catch (e) {
    say(String(e), true);
  }
}

/** Keep the question you just asked (spec.md §11). */
async function keepQuery(typed: string): Promise<void> {
  if (typed === "") {
    say("אין מה לשמור — תיבת החיפוש ריקה", true);
    return;
  }
  const name = window.prompt("איך לקרוא לשאילתה?", typed);
  if (name === null || name.trim() === "") return;
  try {
    const kept = await api.queryKeep(name.trim(), typed);
    say(`נשמר: ${kept.name}`, false);
    await yoursview.refresh();
  } catch (e) {
    say(String(e), true);
  }
}

/** A line the window says and then stops saying. */
function say(words: string, trouble: boolean): void {
  if (!root) return;
  let toast = root.querySelector<HTMLElement>(".said");
  if (!toast) {
    toast = document.createElement("p");
    toast.className = "said";
    // A live region, so what the window says is announced rather than only drawn.
    // Ksav's README claims *"the status bar is a live region"* and its snapshot
    // bears that out; this is the same bar in the sibling application (B14).
    announces(toast, "הודעות");
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

// One `button` for the whole window, in `controls.ts` (B14). There were four of
// these — here, `scanview.ts`, `linksview.ts` and `writing.ts` — which is the
// "two readers of one value" shape this project bans, applied to the thing every
// screen is made of.

void main();
