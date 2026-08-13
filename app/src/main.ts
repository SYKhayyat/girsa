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
  pickFolder,
  whenFilesDropped,
  type AppState,
  type Asked,
  type Mefarshim,
  type PaneId,
  type Presence,
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
import { GIRSA, type Named, sefer, speak } from "./names.ts";
import { fill, ksavAs, say, speakInterface, switchInterfaceTo } from "./say.ts";
import {
  nextIn,
  POINTING_ROUND,
  pointingSaid,
  SHOWING_ROUND,
  showingSaid,
  THEME_ROUND,
  themeSaid,
} from "./toolbar.ts";
import { doorLabel, doorTitle, nothingHere } from "./mefarshim.ts";
import { route, type Caret, type Held, type Panel } from "./panel.ts";
import { presenceSaid } from "./presence.ts";
import { codeOf, sayTrouble, trouble } from "./trouble.ts";
import { whatKey, type Pressed } from "./keys.ts";
import { announces, ask, button, glyph, region } from "./controls.ts";

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

/**
 * Every panel's element, in one list, put into the document **once**.
 *
 * # The bug this shape exists to make impossible
 *
 * `main()` appended eleven panels and `draw()` then called
 * `root.replaceChildren(chrome, …)` with a hand-written list of **eight** of
 * them. So from the first redraw — which is boot — `settingsview` and `writing`
 * were no longer in the document at all, and their buttons still worked
 * perfectly: `toggle()` set `hidden = false` on a node nobody was rendering.
 *
 * The reader's fourth and thirteenth bugs, and they are one bug:
 *
 * > *"hagdaros does absolutely nothing - nothing opens."*
 * > *"ksov does nothing."*
 *
 * There was already a frozen list of the panels in this file — `PANELS`, added
 * after two panels were forgotten from the *keyboard* table, with a test that
 * sweeps this module for anything constructed here and missing from it. It knew
 * about `settingsview` and `writing`. The DOM list was a second list, three
 * lines long, that nothing checked.
 *
 * So there is one list now: `PANELS` is the panels, and this is derived from it.
 * `draw()` replaces the chrome and touches nothing else, so a panel cannot be
 * removed from the document by a redraw — and a twelfth panel is in the document
 * the moment it is in `PANELS`, which is the moment its Escape works.
 */
function panelElements(): HTMLElement[] {
  return PANELS.map((held) => held.panel.element);
}

async function main(): Promise<void> {
  if (!root) return;
  await connect();
  // The chrome is a single node that `draw()` replaces; the panels sit beside
  // it and are never rebuilt.
  const chrome = document.createElement("div");
  chrome.className = "app";
  root.append(chrome, ...panelElements());
  settingsview.onChanged(() => {
    // Everything on that panel can change how a sefer is drawn, so the panes are
    // rebuilt rather than patched.
    views.clear();
    scans.clear();
    void reload();
  });
  // The one setting that cannot be redrawn into place — see
  // `SettingsView.onInterfaceChanged`. Whatever is in the writing drawer goes to
  // disk first; everything else the window is holding is a copy of what Rust
  // holds, and comes back the same.
  //
  // This used to be `await writing.flush(); window.location.reload()`, and the
  // missing statement between them was the whole of finding 2: the reload
  // rebuilt every panel from a cache that had not been told about the switch
  // yet, so the window came back one language behind — in both directions,
  // until it was restarted. `switchInterfaceTo` is the write and the reload as
  // one act, in the module that owns the cache.
  settingsview.onInterfaceChanged(async (language) => {
    await writing.flush();
    switchInterfaceTo(language);
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
  announce(`${say("opened")} — ${landing.ref}`, false);
}

/** Whether Ksav is there. Polled while the window is open, because the answer
 * changes without anything telling us: Ksav is a separate application and a
 * reader starts and stops it whenever they like. */
let ksav: Presence = { state: "not_running" };

/**
 * Ask whether Ksav is there — at boot, and **at the moment of a send**.
 *
 * # The interval is gone, and the deletion test had already been run
 *
 * `spec.md:759` says *"each app shows whether its sibling is live, so the
 * affordance is never offered when it would fail."* This half implemented it
 * with a `setInterval` every 5,000 ms for the life of the window. **The other
 * half implements nothing** — Ksav has no poller and no `girsaPresence`; with
 * Girsa installed but shut, the call goes out, fails, and lands in its error
 * vocabulary as *"גִּרְסָא אינה פועלת — פתחו אותה ונסו שוב"*.
 *
 * It works. Nobody noticed the asymmetry, which means the experiment of *not*
 * polling has been running in production, in the other application, the whole
 * time.
 *
 * What does **not** survive that is polling to decide whether to draw a button.
 * An IPC round-trip every five seconds, forever, so that a toolbar can be right
 * about something the send itself finds out in the same instant.
 *
 * What does survive is `Presence::Stale` — the endpoint file outlived its
 * listener — which is a real fact `girsa-post` computes and no error string can
 * reconstruct. `presence.ts` is right that collapsing it away *"throws away the
 * only one of the three that is actionable"*. So the **function** stays and the
 * interval goes: once at boot for the chip, and again before each send, which
 * is the only moment the answer changes anything.
 */
async function watchForKsav(): Promise<void> {
  if (!isShell()) return;
  await lookForKsav();
}

async function lookForKsav(): Promise<void> {
  if (!isShell()) return;
  const now = await api.ksavPresence();
  // Redrawn only when it changed: a toolbar that rebuilds itself takes the
  // reader's text selection with it.
  if (now.state !== ksav.state) {
    ksav = now;
    document.querySelector(".tools")?.replaceWith(toolBar());
  } else {
    ksav = now;
  }
  writing.setKsav(now);
}

/** Files dropped on the window become seforim (spec.md §5). */
async function whenDropped(paths: string[]): Promise<void> {
  if (paths.length === 0) return;
  if (!shelf.isOpen) await shelf.show(openTab);
  shelf.say(`${say("readingFiles")} ${paths.length} ${say("files")}…`, false);
  const dropped = await api.addMine(paths);
  await shelf.refresh();

  const added = dropped.added.map(sefer).join(", ");
  // Both halves, always. A drop that half-worked and said nothing leaves a
  // reader believing a sefer is on the shelf when it is not.
  const refused = dropped.refused.map((r) => r.why).join(" · ");
  if (dropped.added.length > 0 && dropped.refused.length === 0) {
    shelf.say(`${say("addedSeforim")}: ${added}`, false);
  } else if (dropped.added.length > 0) {
    shelf.say(`${say("addedSeforim")}: ${added} — ${say("refusedSeforim")}: ${refused}`, true);
  } else {
    shelf.say(refused || say("nothingAdded"), true);
  }
}

async function openTab(slug: string): Promise<void> {
  await api.openTab(slug);
  await reload();
}

async function reload(): Promise<void> {
  state = await api.state();
  // **A number, not a percentage.** `styles.css` scales every reading size with
  // `calc(19px * var(--reading-size) / 100)`, and `calc` cannot multiply a
  // length by a percentage — the declaration is invalid at computed-value time
  // and thrown away. So `א+`, `א−`, `Ctrl+=`, `Ctrl+-` and the size row in the
  // settings panel all worked perfectly, wrote the session, redrew the window
  // and changed nothing anybody could see. The reader's fourth bug, second half:
  // *"Same for the two font size buttons."*
  document.documentElement.style.setProperty("--reading-size", String(state.text_size));
  // The language the **seforim** are named in (W41), set once from the session so
  // that every `sefer()` in every module answers the same way.
  speak(state.language);
  // …and the language the **window** speaks, which is a different setting and
  // used to be no setting at all.
  speakInterface(state.interface);
  // How the reading looks (B13): theme, the two fonts, leading and measure. On the
  // document as custom properties, so `styles.css` keeps owning the appearance.
  applyLook(state.look);
  // **What every open sefer is called, before anything is drawn.**
  //
  // `named` used to be filled only when a pane was *drawn*, so after a restart
  // every tab but the active one was labelled with its English internal id —
  // `bavli/tosafot-on-berakhot +1 | mishnah-berurah | bavli/shabbat` — as the
  // first thing on screen, every launch. A tab knew its Hebrew name only while
  // the pane that made it was in memory; a restored tab is drawn from the
  // session file, which stores the slug, and nothing asked the catalogue what
  // that slug is called.
  //
  // It costs one call and no sefer is opened.
  await nameTheOpenSeforim();
  await draw();
}

/** Fill `named` for every slug in every tab, from the catalogue. */
async function nameTheOpenSeforim(): Promise<void> {
  const wanted = [
    ...new Set(
      (state?.workspace.tabs ?? []).flatMap((open) => open.panes.map((pane) => pane.slug)),
    ),
  ].filter((slug) => !named.has(slug));
  if (wanted.length === 0) return;
  try {
    for (const card of await api.titles(wanted)) named.set(card.slug, card);
  } catch {
    // The catalogue is not there — a window with no corpus. `titleOf` falls
    // back to the slug, which is what it did before and is at least a name.
  }
}

function tab(): Tab | null {
  return state?.workspace.tabs[state.workspace.active] ?? null;
}

/** Put the freshly built chrome where the old one was, leaving the panels
 * alone — see [`panelElements`]. */
function replaceChrome(chrome: HTMLElement): void {
  const standing = root?.querySelector<HTMLElement>(":scope > .app");
  if (standing) standing.replaceWith(chrome);
  else root?.prepend(chrome);
}

async function draw(): Promise<void> {
  if (!root || !state) return;
  const chrome = document.createElement("div");
  chrome.className = "app";
  chrome.append(tabBar(), toolBar());

  const open = tab();
  if (!open) {
    chrome.append(nothingOpen());
    replaceChrome(chrome);
    return;
  }

  const { root: boxes, slots } = build(open.layout, state.share_bounds, (pane, ratio) => {
    void api.setRatio(pane, ratio);
  });
  boxes.classList.add("panes");
  boxes.setAttribute("role", "main");
  boxes.setAttribute("aria-label", say("theReading"));
  chrome.append(boxes);
  replaceChrome(chrome);

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
    //
    // Caught, because this was the whole of *"the body below the error is black
    // and empty"*. A restored tab whose sefer will not open — the corpus is
    // gone, the slug was renamed by a re-import — threw out of the middle of
    // this loop, after the chrome had been placed and before any pane was
    // built. The reader got a toolbar over an unlabelled black rectangle, and
    // every pane after the failing one was never drawn either.
    let text;
    try {
      text = await api.openSefer(pane.slug);
    } catch (e) {
      sayTrouble(slot, e, "read_page");
      slot.classList.add("pane-broken");
      continue;
    }
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
      await openBeside(id, opened);
    },
    tick: (work, on) => void tickMefaresh(slug, work, on),
  });
}

/**
 * Open one or more seforim in columns beside this one.
 *
 * > *"there should be a way to open at one time multiple windows with multiple
 * > meforshim."*
 *
 * There was not: the door closed on the first click and each further mefaresh
 * cost the whole round trip again — open the door, find the row, click, watch it
 * shut. Each new column is split off the **one before it** rather than all off
 * the base, so three mefarshim beside a Gemara are three columns of even width
 * instead of one wide one and two slivers; and every one of them follows the
 * base, because that is what a reader means by putting them there.
 */
async function openBeside(id: PaneId, seforim: string[]): Promise<void> {
  let from = id;
  for (const slug of seforim) {
    const opened = await api.split(from, "vertical", slug, true);
    if (opened === null) break;
    // Each pane follows the sefer the reader is actually reading, not the
    // mefaresh that happens to be beside it — `workspace::split` follows the
    // pane it was split from, which for the second mefaresh onward is the first
    // mefaresh. Every commentary keeps step with the daf.
    if (from !== id) await api.setFollows(opened, id);
    from = opened;
  }
  await reload();
}

/**
 * Tick one mefaresh, and redraw the markers on every pane reading this sefer.
 *
 * # Why the whole list comes back
 *
 * > *"checking off a mefarsh does not open it when its line is clicked."*
 *
 * It did not, and this is where. `choose_mefaresh` answered with the marked
 * lines and the window patched the rest of its own copy: it flipped `chosen`
 * inside `works`, and `drawMefarshim` counts that array to decide whether a
 * click on a line means anything at all (`PaneView.ticked`). But the list the
 * reader is ticking in is `listed`, which also carries the seforim running
 * alongside and every mefaresh the link graph knows and the catalogue does not.
 * Tick one of those — and on a masechta most of them are those — and `works`
 * never mentioned it, so the count stayed at zero, so the pane went on ignoring
 * clicks, so nothing opened.
 *
 * The tick-box also un-ticked itself: `listed` was not patched either, and it is
 * what the picker draws.
 *
 * Rust now answers with the whole `Mefarshim` and this holds it. One answer,
 * from the one place that builds it.
 */
async function tickMefaresh(slug: string, work: string, on: boolean): Promise<void> {
  const now = await api.chooseMefaresh(slug, work, on);
  mefarshimOf.set(slug, now);
  picker.refreshMefarshim(slug, now);
  for (const view of views.values()) {
    if (view.slug === slug) await drawMefarshim(view);
  }
}

/** How many mefarshim are ticked on a sefer — over **every** group.
 *
 * `works.filter(chosen).length` counted one of the four, which is the same bug
 * as the one above wearing a different hat. */
function tickedCount(on: Mefarshim): number {
  const slugs = new Set<string>();
  for (const row of on.listed) {
    if (row.kind === "sefer" && row.choice.chosen) slugs.add(row.choice.slug);
  }
  for (const w of [...on.works, ...on.alongside]) {
    if (w.chosen) slugs.add(w.slug);
  }
  return slugs.size;
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
    return { works: [], alongside: [], folders: [], listed: [], marked: {}, touched: 0, unbuilt: null };
  }
}

/** Draw the markers on a pane, for whatever is ticked now. */
async function drawMefarshim(view: PaneView): Promise<void> {
  const on = await mefarshimFor(view.slug);
  view.setMefarshim(on.marked, tickedCount(on));
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
  const chosen = tickedCount(on);
  try {
    const comments = await api.mefarshimAt(view.slug, at);
    view.showSaid(at, comments.said, nothingHere(comments, chosen));
  } catch (e) {
    // A read that failed is not *nobody wrote here*, and must not be shown as it.
    const t = trouble(e, "read_links");
    view.showSaid(at, [], t.said);
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
  return of ? `${say("following")} ${titleOf(of.slug)}` : "";
}

/**
 * The control that ties one column's scroll to another's.
 *
 * # It was called `עוקב`
 *
 * > *"i have no clue what okev does."*
 *
 * Neither would anybody. `עוקב` is a bare participle — *following* — with no
 * object and no direction: it does not say what follows what, whether clicking
 * starts or stops it, or which of the two columns it is about. And it toggled
 * blindly: with nothing followed it grabbed `others[0]`, which on a three-way
 * split is whichever pane the layout happens to list first.
 *
 * So it is named after the thing it does — the scroll — it says which column it
 * will tie this one to, and its label is the **state it will move to**, like
 * every other control in this toolbar. The reader's eighth bug asks for exactly
 * this in so many words: *"there should be an option to link or unlink
 * scroll."*
 */
function scrollLink(id: PaneId): HTMLElement {
  const pane = tab()?.panes.find((p) => p.id === id);
  const others = tab()?.panes.filter((p) => p.id !== id) ?? [];
  const linked = pane?.follows !== undefined;
  const to = others[0];
  const control = button(
    linked ? say("unlinkScroll") : say("linkScroll"),
    linked
      ? `${say("scrollNowSharedWhy")}${pane ? ` — ${followLabel(pane.follows)}` : ""}`
      : to
        ? `${say("scrollNowOwnWhy")} — ${titleOf(to.slug)}`
        : say("scrollNowOwnWhy"),
    async () => {
      await api.setFollows(id, linked ? null : (to?.id ?? null));
      await reload();
    },
  );
  control.classList.add("tool-wide");
  control.classList.toggle("is-on", linked);
  // With nothing to follow there is nothing to link to, and a control that
  // cannot do its one job is a control that teaches the reader the buttons lie
  // — the same argument `addScanControls` already makes for the buttons it
  // leaves off.
  control.disabled = !linked && !to;
  return control;
}

function addControls(view: PaneView, id: PaneId): void {
  const beside = button(say("beside"), doorTitle([]), () => {
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
  const links = button(say("links"), say("linksWhy"), () => {
    void showLinks();
  });
  // W22: base text + your patches → a file. On the pane, because what is
  // written out is the sefer this pane is reading, corrections and all — and it
  // asks **where**, which it did not.
  const save = button(say("exportSefer"), say("exportWhy"), () => {
    const pane = tab()?.panes.find((p) => p.id === id);
    if (pane) void exportSefer(pane.slug);
  });
  const close = button(say("closePane"), say("closePaneWhy"), async () => {
    await api.closePane(id);
    views.delete(id);
    scans.delete(id);
    await reload();
  });
  view.addControl(beside);
  view.addControl(scrollLink(id));
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
  const beside = button(say("beside"), doorTitle([]), () => {
    void openMefarshim(id);
  });
  // A scan of a daf has mefarshim like any other copy of that daf, so it gets
  // the same name on the same button. Fixing one and not the other is how the
  // label drifts back apart.
  void nameTheDoor(beside, id);
  const close = button(say("closePane"), say("closePaneWhy"), async () => {
    await api.closePane(id);
    views.delete(id);
    scans.delete(id);
    await reload();
  });
  view.addControl(beside);
  view.addControl(scrollLink(id));
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

/**
 * What a tab is called.
 *
 * **The focused pane's sefer**, not `panes[0]`'s.
 *
 * A tab is an *arrangement* — a pane tree with a sefer in each pane — which is
 * the model settled next door and written down in
 * `Ksav/decisions/2026-08-11-marking-up-the-ui-inventory.md`: *"a tab's label
 * defaults to the title of the document in its focused pane, so until a split is
 * deliberately built the strip reads and behaves exactly like ordinary tabs."*
 * Reading `panes[0]` made a tab holding a Gemara and its Rashi say `ברכות`
 * whichever of the two you were actually in, and a reader with three such tabs
 * had three labels that could not tell them apart.
 */
function tabLabel(open: Tab): string {
  // …and there is one arrangement where the focused pane is the wrong answer:
  // **the one this application is for.** A reader learning Berakhos with
  // Tosafos beside it, scroll linked, puts the cursor in the Tosafos to read it
  // — and the tab became `תוספות על ברכות +1`, with the masechta demoted to the
  // `+1`. The tab is the arrangement, and the arrangement is *Berakhos, with a
  // mefaresh*.
  //
  // The window already knows which is which without asking the shelf: a
  // commentary column **follows** its base, so the pane something else follows
  // is the one the arrangement is built around. Where nothing follows anything
  // — two unrelated seforim side by side — there is no such pane, and the
  // focused one is the right label again.
  const led = open.panes.find((p) => open.panes.some((other) => other.follows === p.id));
  const focused = open.panes.find((p) => p.id === open.focused) ?? open.panes[0];
  const named = titleOf((led ?? focused)?.slug ?? "—");
  // A split says so, because the label is now about one pane out of several.
  return open.panes.length > 1 ? `${named} +${open.panes.length - 1}` : named;
}

function tabBar(): HTMLElement {
  const bar = document.createElement("nav");
  bar.className = "tabs";
  bar.setAttribute("aria-label", say("tabs"));
  const tabs = state?.workspace.tabs ?? [];
  // **One tab draws no strip.** Ksav's decision again: *"the tab strip must
  // therefore hide itself when only one document is open — a single tab is pure
  // noise."* The row of actions below stays either way, because it is not the
  // strip and it is the only route in.
  if (tabs.length > 1) {
    const strip = document.createElement("span");
    strip.className = "tab-strip";
    tabs.forEach((open, index) => {
      const named = tabLabel(open);
      const holder = document.createElement("span");
      holder.className = "tab-holder" + (index === state?.workspace.active ? " is-active" : "");
      const go = document.createElement("button");
      go.className = "tab";
      go.textContent = named;
      go.title = open.panes.map((p) => titleOf(p.slug)).join(" · ");
      go.addEventListener("click", async () => {
        if (state) state.workspace.active = index;
        // The pane that had the cursor in **this** tab, not its first pane: a
        // tab is an arrangement and returning to it means returning to where you
        // were in it.
        await api.focus(open.focused);
        await reload();
      });
      // W40: *"needs a way to close tab without going in."* Named after the
      // sefer it closes, because `×` is a glyph and a glyph is not a name — and
      // it says **close**, never delete: closing a tab closes the arrangement
      // and leaves every sefer exactly where it was.
      const shut = glyph("×", `${say("closeTab")} — ${named}`, () => {
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
      strip.append(holder);
    });
    bar.append(strip);
  }
  bar.append(button(say("newTab"), `${say("openSefer")} (Ctrl+O)`, openSomething));
  bar.append(button(say("browseShelf"), say("browseShelfWhy"), browseShelf));
  bar.append(button(say("search"), say("searchWhy"), search));
  bar.append(button(say("write"), say("writeWhy"), () => void writing.toggle()));
  // The semantic lane (spec.md §9.9, W30). Always here, whether or not it is
  // on: it is a setting rather than a queue, and a reader who has never met it
  // needs somewhere to meet it. Standing beside it is the sefer in the focused
  // pane, so *put this one in the lane* has something to name.
  bar.append(
    button(say("lane"), say("laneWhy"), openLane),
  );
  // The queue, where there is one. Not shown at all when the batch job has
  // never been run: a button that opens an empty list teaches the reader that
  // the feature does nothing.
  if ((state?.suspects ?? 0) > 0) {
    bar.append(
      button(
        `${say("queue")} ${state?.suspects ?? 0}`,
        say("queueWhy"),
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
  const bar = region("toolbar", say("tools"), "tools");

  // **What it will do, not what it is.**
  //
  // > *"im nikkud and bli nikkud are backwards."*
  //
  // They were: this printed `עם ניקוד` while the nikud was on — the state you
  // are already in — twenty lines from a language button whose own comment says
  // *"a button labelled with the state you are already in is a button nobody can
  // predict."* Two buttons, two conventions, one toolbar, and the reader read
  // the toolbar the way the toolbar's own comment says to.
  //
  // Three settings now rather than two, so it rounds: the label is the **next**
  // one, which is the one clicking gets you.
  const next = nextIn(POINTING_ROUND, state?.pointing ?? "full");
  const nikud = button(pointingSaid(next), say("pointingWhy"), async () => {
    await api.setPointing(next);
    // The words themselves change, so the panes are rebuilt.
    views.clear();
    await reload();
  });
  nikud.classList.add("tool-wide");

  // The same convention as the button beside it, which is the whole of finding
  // 12: **the label is the state clicking gets you.** This printed
  // `state.showing` — the state you were already in — twenty lines under a
  // comment explaining why its neighbour must not.
  const showingNext = nextIn(SHOWING_ROUND, state?.showing ?? "fixed");
  const showing = button(showingSaid(showingNext), say("showingWhy"), () => {
    void nextShowing();
  });
  showing.classList.add("tool-wide");
  if ((state?.fixes ?? 0) === 0) showing.classList.add("is-quiet");

  // The theme, one click from the daf (the reader: *"i dont want it stuck in
  // dark mode"*).
  //
  // There has always been a light one, it has always worked, and it was an
  // `<option>` in a `<select>` in a panel — with the default *follow the
  // system*, so a machine whose Windows is dark gives a dark daf and nothing on
  // the reading screen suggests otherwise. The settings row stays: that is where
  // you go to *set up* a window. This is where you go to change your mind about
  // the light in the room, which happens at dusk, not at setup.
  //
  // Same convention as its two neighbours — the label is the state clicking gets
  // you, which is finding 12 — and the same round the settings row now reads.
  const nextTheme = nextIn(THEME_ROUND, state?.look.theme ?? "system");
  const theme = button(themeSaid(nextTheme), say("themeWhy"), () => {
    void (async () => {
      const look = state?.look;
      if (!look) return;
      // The whole record, because `set_look` takes the whole record — and
      // `Look::sane` clamps what it is given, so sending four fields as
      // defaults would quietly reset the reader's line height on a theme click.
      await api.setLook({ ...look, theme: nextTheme });
      await reload();
    })();
  });
  theme.classList.add("tool-wide");

  const smaller = button(say("smaller"), say("smallerWhy"), () => resize(-10));
  const bigger = button(say("bigger"), say("biggerWhy"), () => resize(10));

  const where = document.createElement("span");
  where.className = "tools-note";
  if (state?.trouble) {
    // Finding 19, at its narrowest: this was `textContent = state.trouble`, and
    // what landed at the top of a right-to-left Hebrew window was four lines of
    // Latin file paths with `../../corpus.` reversed into `.corpus./../..` by
    // the bidi algorithm. Every command in the shell wraps its refusals in a
    // code so that `trouble.ts` can say them in Hebrew; the one string the
    // window shows before a reader has done anything went round the outside.
    sayTrouble(where, state.trouble);
  } else {
    where.textContent = `${state?.works ?? 0} ${say("seforimCount")}`;
    if (!isShell()) where.textContent += ` · ${say("inBrowser")}`;
  }

  // B13. A panel, and a way to reach it — the reading settings that used to be
  // four buttons and nothing else. Both language settings live on it: they are
  // settings rather than gestures, and a toolbar that carried one of the two
  // taught a reader that the other did not exist.
  const setup = button(say("settings"), say("settingsWhy"), () => void settingsview.toggle());
  bar.append(theme, nikud, showing, smaller, bigger, setup, where);

  // Presence (spec.md §10.6): the affordance is never offered when it would
  // fail. Live, it is a button; not live, it is a word saying which of the two
  // reasons it is.
  if (isShell()) {
    const said = presenceSaid(ksav);
    if (said.canSend) {
      const send = button(
        fill("sendToKsavNamed", { ksav: ksavAs("ל") }),
        say("sendToKsavWhy"),
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

/** Round the three states (spec.md §7.1, §7.2). Everything open is redrawn,
 * because the words themselves changed.
 *
 * The toast says **what is showing now**, which is the opposite of what the
 * button says: the button is a promise about the next click, the toast is a
 * report of the last one. They used to be the same bare word — the button read
 * `מתוקן` and so did the toast, in the same second, eight pixels apart —
 * which is how one vocabulary came to mean two things. */
async function nextShowing(): Promise<void> {
  if (!state) return;
  const next = nextIn(SHOWING_ROUND, state.showing);
  await api.setShowing(next);
  views.clear();
  await reload();
  announce(fill("showingNow", { what: showingSaid(next) }), false);
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
    announce(say("highlightFirst"), false);
    return;
  }
  fixbox.show(
    { at: here.at, fromChar: 0, toChar: 0, words: "", fixed: here.fixed, printed: here.printed },
    null,
    {
      save: async () => announce(say("highlightFirst"), false),
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
    const t = trouble(e, "read_suspects");
    announce(t.said, true, t.detail);
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
    announce(`${kind === "ocr" ? say("fixed") : say("variantNoted")} — ${fixed.said}`, false);
    if (state) state.fixes += 1;
  } catch (e) {
    // "There is already a correction here" and "nothing is selected" are
    // different things to a reader, and that was the argument for showing
    // `String(e)` as it came. The distinction is real; the raw English was
    // never how to keep it. Both are `girsa_app::trouble::Code` refusals, so
    // `trouble()` reads them by *name* and keeps the distinction exactly —
    // `CODED` is where the two sentences live.
    const t = trouble(e, "fix");
    announce(t.said, true, t.detail);
  }
}

async function revertFix(view: PaneView, at: string, patch: string): Promise<void> {
  try {
    const fixed = await api.unfix(at, patch);
    view.replaceLine(fixed.line);
    announce(fixed.said, false);
    if (state) state.fixes = Math.max(0, state.fixes - 1);
  } catch (e) {
    const t = trouble(e, "fix");
    announce(t.said, true, t.detail);
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

/**
 * Write the sefer out with your corrections in it (spec.md §7.4) — **into a
 * folder the reader chose**.
 *
 * > *"send to ksav and export dont let you pick a folder."*
 *
 * It did not ask: it wrote into `personal/exports/` and then said the path,
 * which is a fine way to produce a file and a poor way to hand somebody a sefer.
 * The dialog is the shell's own, and a reader who cancels it gets nothing
 * written rather than a file somewhere they did not choose.
 *
 * Outside the shell there is no dialog, so `null` comes straight back and Rust
 * falls through to where the last one went — which is also what happens on the
 * second export, so the question is asked once.
 */
async function exportSefer(slug: string): Promise<void> {
  try {
    const into = await pickFolder(say("chooseFolder"));
    if (isShell() && into === null) return;
    const written = await api.exportSefer(slug, "docx", into ?? undefined);
    // The path, because the file is the point and a reader has to be able to
    // find it — and what did *not* land, because exporting is the moment
    // somebody would otherwise never hear about a stale correction.
    const trouble = written.stale > 0 ? ` · ${written.stale} ${say("staleFixes")}` : "";
    announce(`${say("wrote")} — ${written.said}${trouble} · ${written.path}`, written.stale > 0);
  } catch (e) {
    const t = trouble(e, "export");
    announce(t.said, true, t.detail);
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
  // The application, as it spells its own name — `names.ts`, which is where
  // both applications' names live so a seventh site cannot spell one a third
  // way. A name is not translated.
  title.textContent = GIRSA;
  empty.append(title);

  // Two different screens, because they are two different situations and the
  // audit found only one of them handled: *nothing is open* offers the four
  // things a reader can open, and **there is nothing to open** must not, since
  // every one of those buttons leads to an empty list.
  if (codeOf(state?.trouble ?? "") === "no-shelf") {
    empty.append(...noCorpus());
    return empty;
  }

  const hint = document.createElement("p");
  hint.className = "empty-hint";
  hint.textContent = say("emptyHint");
  const open = button(say("openSefer"), say("openSeferKey"), openSomething);
  open.classList.add("empty-button");
  const browse = button(say("browseShelf"), say("browseShelfWhy"), browseShelf);
  browse.classList.add("empty-button");
  const look = button(say("search"), say("searchWhy"), search);
  look.classList.add("empty-button");
  empty.append(hint, open, browse, look);
  return empty;
}

/**
 * The first screen when there is no corpus at all (finding 19).
 *
 * > *"Four lines of Latin paths across the top of a right-to-left window … No
 * > Hebrew. No *there are no seforim here yet*. No button — although
 * > `tauri-plugin-dialog` is already in the build."*
 *
 * The list of directories is not deleted, and deleting it would be the wrong
 * lesson: it is the one thing that tells whoever is debugging an installation
 * that the corpus is one directory away from where they are standing. It moves
 * to a hover, behind a sentence, beside the thing a reader can actually do.
 *
 * The button is offered **only in the shell**, for the reason the presence chip
 * is: outside it there is no folder dialog, so the button would open nothing.
 * An affordance is never offered where it would fail.
 */
function noCorpus(): HTMLElement[] {
  const out: HTMLElement[] = [];
  const hint = document.createElement("p");
  hint.className = "empty-hint";
  hint.textContent = say("noCorpusHint");
  out.push(hint);

  if (isShell()) {
    const choose = button(say("chooseCorpus"), say("chooseCorpusWhy"), () => void chooseCorpus());
    choose.classList.add("empty-button");
    out.push(choose);
  }

  // Where it looked, for whoever is debugging an installation — as a hover on a
  // quiet line rather than as the screen.
  const where = document.createElement("p");
  where.className = "empty-detail";
  where.textContent = say("whereItLooked");
  where.title = trouble(state?.trouble ?? "").detail;
  out.push(where);
  return out;
}

/** Ask for a folder of seforim, and open it. */
async function chooseCorpus(): Promise<void> {
  const at = await pickFolder(say("chooseCorpus"));
  if (at === null) return;
  try {
    await api.chooseCorpus(at);
  } catch (e) {
    // The refusal that matters most in this whole flow — a folder with no
    // catalogue in it — and it is the only way the reader learns they picked
    // the wrong one. `not-a-corpus` says which folder to pick instead.
    const t = trouble(e);
    announce(t.said, true, t.detail);
    return;
  }
  await reload();
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

/**
 * Where the caret is, relative to one panel — the DOM half of `panel.ts`.
 *
 * `on` and `typing` are different questions and the difference is finding 3: a
 * docked panel is full of buttons, one per result, and clicking a result leaves
 * the focus on it. A panel that holds the keyboard whenever focus is anywhere
 * inside it therefore holds the keyboard for the whole time the reader spends
 * reading what they clicked.
 *
 * *Typing* is the `contentEditable` question rather than the tag question: a
 * `<textarea>`, an `<input>` that takes text (not a checkbox or a button), or
 * anything the reader can put a caret into. `readOnly` is excluded — a box you
 * cannot type into does not own what is typed.
 */
function caretIn(panel: Panel, target: EventTarget | null): Caret {
  if (!(target instanceof Node) || !panel.element.contains(target)) return "away";
  const node = target instanceof HTMLElement ? target : target.parentElement;
  if (!node) return "on";
  if (node.isContentEditable) return "typing";
  if (node instanceof HTMLTextAreaElement) return node.readOnly ? "on" : "typing";
  if (node instanceof HTMLInputElement) {
    // The input types a reader types into. A checkbox is an input and a typed
    // letter in one is not text.
    const typed = new Set(["text", "search", "email", "url", "tel", "password", "number", "date"]);
    return typed.has(node.type) && !node.readOnly && !node.disabled ? "typing" : "on";
  }
  return "on";
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

/**
 * Every panel, and what each does with a keypress (B13).
 *
 * In order: the first open one that wants a key gets it. This was
 * **forty-eight hand-written lines** of `if (x.isOpen && …)` — nine panels,
 * ten branches because `yoursview` needed two, and three different ways of
 * asking whether a panel was open. Add a panel and the way you found out you
 * had forgotten a line was that Escape did nothing.
 *
 * `Ctrl+F` used to be written out here, in place, above the second `find`
 * branch — so the one shortcut B13 exists to make rebindable was the one that
 * was not. It is `toggle: "search"` now and goes through the same table as
 * everything else.
 *
 * # A frozen array, not a function, and two panels that were missing from it
 *
 * The 9 August report, comparing the two applications' shells:
 *
 * > Panel registry — Girsa: a **function** in `main.ts:987` — silently omits
 * > `lanepanel` and `settingsview`, so Escape closes neither. Ksav:
 * > module-level frozen array that **throws** on an undeclared ×.
 *
 * Both halves were true. The table fixed *"add a panel and Escape does
 * nothing"* for the nine panels that were in it, and then two panels were added
 * and Escape did nothing — which is the failure the table exists to prevent,
 * arriving through the table.
 *
 * A function reads as *the panels, computed* and invites a condition; the
 * frozen array reads as *the panels*, and `panel.test.mjs` sweeps `main.ts` for
 * anything constructed here that satisfies `Panel` and is not in it. The
 * omission is now a red test rather than a key that does nothing.
 */
const PANELS: readonly Held[] = Object.freeze([
    // Its own Escape, and it must not be raced.
    { panel: picker, keyboard: "all", escape: false },
    // A text box. `Ctrl+C` in one is copy, and it closes itself.
    { panel: fixbox, keyboard: "inside", escape: false },
    // The buffer. Escape and Ctrl+E close it, and only from inside — a reader
    // pressing Escape over the daf is not closing what they are writing.
    {
      panel: writing,
      keyboard: "inside",
      escape: "inside",
      answers: (event) =>
        (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "e"
          ? (writing.close(), true)
          : false,
    },
    // Drawers over the reading: Escape closes them from anywhere, and the
    // reading shortcuts stay live behind them.
    { panel: linksview, keyboard: "reading", escape: "anywhere" },
    // Your own layer docks the moment it opens, so it is never over the
    // reading — the sibling of finding 3, cleared by moving it off `inside`.
    // Its boxes still own what is typed into them; its buttons do not own the
    // daf.
    { panel: yoursview, keyboard: "typing", escape: "anywhere" },
    { panel: suspects, keyboard: "reading", escape: "anywhere" },
    // Places while they are over the reading — a typed letter goes into them —
    // and columns beside it once the reader has gone through them (W47, W48).
    // Docked, they own only what is typed into their own boxes: the reader is
    // reading, and `all` there is finding 3, which silently killed every
    // shortcut in the application including the send to Ksav.
    {
      panel: find,
      keyboard: () => (find.isDocked ? "typing" : "all"),
      escape: "anywhere",
      toggle: "search",
    },
    {
      panel: shelf,
      keyboard: () => (shelf.isDocked ? "typing" : "all"),
      escape: "anywhere",
      toggle: "shelf",
    },
    // The two the report found missing. Both are drawers over the reading with
    // their own × already, so `anywhere` is what their close buttons already
    // promise — Escape was the only way to close them that did not work.
    { panel: lanepanel, keyboard: "reading", escape: "anywhere" },
    { panel: settingsview, keyboard: "inside", escape: "anywhere" },
  ]);

function shortcut(event: KeyboardEvent): void {
  // B13. What the reader asked for, from the table in `girsa_app::keys` with
  // their own rebindings over it. It used to be eighteen comparisons against
  // letters written in place, which is why there was nothing to rebind and why
  // two tooltips could both claim Ctrl+L with only one of them wired.
  // `Ctrl++` and `Ctrl+=` are the same key to a reader and different keys to a
  // keyboard, so one is spelled as the other before the table is asked.
  const pressed = event.key === "+" ? { ...asPressed(event), key: "=" } : asPressed(event);
  const did = whatKey(state?.keys ?? {}, pressed);

  // Whoever has the keyboard gets it first. One table, in `panel.ts`, rather
  // than forty-eight lines of `if (x.isOpen && …)` here.
  const routed = route(PANELS, event, (p) => caretIn(p, event.target), did);
  if (routed === "closed" || routed === "answered") {
    event.preventDefault();
    return;
  }
  if (routed === "swallowed") return;

  switch (did) {
    case "search":
      event.preventDefault();
      search();
      return;
    case "shelf":
      event.preventDefault();
      browseShelf();
      return;
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
        // The same round the toolbar button walks, and the same function that
        // labels it — one rule, so the key and the button cannot get out of
        // step about what *next* means.
        await api.setPointing(nextIn(POINTING_ROUND, state.pointing));
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
      // Read, not announced. `clipboard::put` codes all three of its failures
      // — `will-not-serialize`, `no-clipboard`, `clipboard-refused` — and this
      // was handing the coded string to a toast, so what a reader got was
      // `no-clipboard: Empty clipboard error, code = OSError(1418): Thread does
      // not have a clipboard open.` in a right-to-left window. That exact
      // sentence is quoted at the top of `trouble.ts` as the reason it exists.
      if (cited.put.trouble) {
        const t = trouble(cited.put.trouble, "copy_scan");
        announce(t.said, true, t.detail);
      } else announce(`${say("copied")} — ${cited.display}`, false);
    } catch (e) {
      const t = trouble(e, "copy_scan");
      announce(t.said, true, t.detail);
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
    const t = trouble(copied.put.trouble, "copy_scan");
    announce(t.said, true, t.detail);
    return;
  }
  // Named, not "copied": a reader should be able to see from the confirmation
  // that they took the place they meant, without pasting it somewhere to look.
  const lines = copied.lines > 1 ? ` · ${copied.lines} ${say("copiedLines")}` : "";
  announce(`${say("copied")} — ${copied.display}${lines}`, false);
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

  // The one moment the answer matters. Asked here rather than every five
  // seconds, and asked *before* the send so a stale endpoint file is reported
  // as itself rather than as a timeout.
  await lookForKsav();
  try {
    const sent = chosen
      ? await api.sendToKsav(chosen.from, chosen.to, chosen.fromChar, chosen.toChar)
      : await api.sendToKsav(here!, here!, 0, null);
    announce(fill("sentToKsavNamed", { ksav: ksavAs("ל"), what: sent.display }), false);
  } catch (e) {
    // "Ksav is not running" and "Ksav refused it" *are* different things to a
    // reader — and that was the argument for showing `String(e)` as it came,
    // which put `PostError`'s English on the first screen of a Hebrew
    // application. The distinction is real; printing the English was never how
    // to keep it. `PostError::code()` names the three, `CODED` in `trouble.ts`
    // has a sentence for each, and the transport string goes where every other
    // one goes — behind the details affordance.
    //
    // This line is the bug `presence.ts` and `trouble.ts` both cite as their
    // reason for existing, in the file neither of them reached.
    const t = trouble(e, "send_to_ksav");
    announce(t.said, true, t.detail);
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
    announce(say("noLineHere"), true);
    return;
  }
  // Prose, so Enter is a new line and Ctrl+Enter keeps it. A note on a sugya
  // is not a filename, and the browser dialog this replaced could hold one
  // line — which is a note you write somewhere else and paste in.
  const text = await ask(say("whatDoYouSay"), { prose: true, hint: say("askNoteHint") });
  if (text === null || text.trim() === "") return;
  try {
    const note = await api.noteWrite(at, text);
    announce(`${say("written")}: ${note.title}`, false);
    // The note is a sefer now, so the shelf and the tabs know one more thing.
    await reload();
    if (linksview.isOpen) await linksview.show(at);
    await yoursview.refresh();
  } catch (e) {
    const t = trouble(e, "write_note");
    announce(t.said, true, t.detail);
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
    announce(mark.kind === "bookmark" ? say("bookmark") : `${say("marked")}: ${mark.was}`, false);
    await repaintMarks();
    await yoursview.refresh();
  } catch (e) {
    const t = trouble(e, "mark");
    announce(t.said, true, t.detail);
  }
}

/** Keep the question you just asked (spec.md §11). */
async function keepQuery(typed: string): Promise<void> {
  if (typed === "") {
    announce(say("nothingToKeep"), true);
    return;
  }
  const name = await ask(say("nameTheQuery"), { value: typed });
  if (name === null || name.trim() === "") return;
  try {
    const kept = await api.queryKeep(name.trim(), typed);
    announce(`${say("kept")}: ${kept.name}`, false);
    await yoursview.refresh();
  } catch (e) {
    const t = trouble(e, "keep_query");
    announce(t.said, true, t.detail);
  }
}

/**
 * A line the window says and then stops saying.
 *
 * `detail` is the machine's own string, put on `title` and never in the words —
 * the same arrangement `sayTrouble` makes for an element, for the one surface
 * that is not an element anybody hands over.
 */
function announce(words: string, trouble: boolean, detail?: string): void {
  if (!root) return;
  let toast = root.querySelector<HTMLElement>(".said");
  if (!toast) {
    toast = document.createElement("p");
    toast.className = "said";
    // A live region, so what the window says is announced rather than only drawn.
    // Ksav's README claims *"the status bar is a live region"* and its snapshot
    // bears that out; this is the same bar in the sibling application (B14).
    announces(toast, say("messages"));
    root.append(toast);
  }
  toast.textContent = words;
  if (detail) toast.title = detail;
  else toast.removeAttribute("title");
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
