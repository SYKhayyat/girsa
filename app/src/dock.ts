// Which panels are docked, and therefore how much room the reading has.
//
// > *"there should be a way to open while keeping madaf open. same for search -
// > be able to go there while keeping search open."*
//
// A docked panel is a column on the leading edge, and the reading is made
// **narrower** rather than covered. Which means something outside every panel —
// the reading — has to move, and that is a fact about the window rather than
// about any one panel.
//
// # Why this is a module and not two lines in each panel
//
// The first version had `search.ts` and `shelf.ts` each add and remove
// `is-docked` on the document root. Two writers, one value: dock the shelf, then
// close the search, and the search's close undocks the shelf's column while the
// shelf is still standing in it — the panel over the text again, which is the
// complaint this is here to answer.
//
// So the set of docked panels is held in one place and the class is derived from
// it. `docked()` is the whole rule, it is a function of a set, and it is tested.

/** The panels standing in the dock, and how wide each one is.
 *
 * A width per panel, because they are not one width: the bookcase and the search
 * are a 380px column, the links panel wants 620 and your own layer 680. The
 * reading makes room for the **widest one standing**, which is the only number
 * that keeps every one of them beside the text rather than over it.
 */
const standing = new Map<string, number>();

/** …and which of those are minimised to a strip. A subset of `standing`: a
 * panel that is not docked cannot be a strip. */
const small = new Set<string>();

/** What a panel takes when it does not say. The bookcase and the search. */
export const A_COLUMN = 380;

/**
 * How wide a drawer is, read off the stylesheet.
 *
 * The widths live in `styles.css` as `--links-wide` and friends, because that is
 * where `min(620px, 50vw)` belongs — and this has to know the same number to
 * make the reading room for it. Read rather than restated: two places holding
 * one width is how a panel ends up half over the text, which is the whole thing
 * this module exists to prevent.
 *
 * Falls back to a column when there is no such property, and to `A_COLUMN` when
 * there is no document at all — `dock.test.mjs` runs in node.
 */
export function wideAs(name: string): number {
  if (typeof document === "undefined") return A_COLUMN;
  // **Resolved by the browser, not parsed.** `getPropertyValue` hands back the
  // token as written — `min(560px, 46vw)` — and `parseFloat` of that is `NaN`,
  // which is how the lane docked 380px of room and then stood 560px wide, 180 of
  // them over the text it had just been moved off. So the value is given to a
  // hidden element as its width and read back after layout, which is the one way
  // to get the number the panel will actually be.
  //
  // Not measured off the panel itself: three of the four animate their width, so
  // at the moment of docking they are still at zero.
  const probe = document.createElement("div");
  probe.style.cssText = `position:absolute;visibility:hidden;height:0;width:var(${name})`;
  document.documentElement.append(probe);
  const wide = probe.getBoundingClientRect().width;
  probe.remove();
  return wide > 0 ? wide : A_COLUMN;
}

/**
 * Whether the reading has to make room.
 *
 * A function of the set and nothing else — the reason this file exists, and what
 * makes the rule testable without a window.
 */
export function docked(panels: Map<string, number>): boolean {
  return panels.size > 0;
}

/**
 * How much room: a column, or a strip.
 *
 * A strip only when **everything** docked is minimised. One panel open beside
 * one minimised panel is a column wide, because the open one is standing in it —
 * and the two of them sharing an edge is what `--dock` being one width is for.
 */
export function width(
  panels: Map<string, number>,
  minimised: Set<string>,
): "none" | "small" | "full" {
  if (panels.size === 0) return "none";
  return [...panels.keys()].every((panel) => minimised.has(panel)) ? "small" : "full";
}

/**
 * How much room the reading has to make: the widest panel standing in the dock,
 * ignoring the ones shrunk to a strip.
 *
 * A function of the two sets, like everything else here, so it can be checked
 * without a window.
 */
export function room(panels: Map<string, number>, minimised: Set<string>): number {
  let widest = 0;
  for (const [panel, wide] of panels) {
    if (minimised.has(panel)) continue;
    widest = Math.max(widest, wide);
  }
  return widest;
}

/** Put a panel in the dock, taking `wide` pixels of the window. */
export function dock(panel: string, wide: number = A_COLUMN): void {
  standing.set(panel, wide);
  apply();
}

/** Take one out. Harmless for a panel that was never in it, because closing a
 * panel that was never docked is the ordinary case. */
export function undock(panel: string): void {
  standing.delete(panel);
  small.delete(panel);
  apply();
}

/** Shrink a docked panel to a strip, or put it back. */
export function minimise(panel: string, on: boolean): void {
  if (on) small.add(panel);
  else small.delete(panel);
  apply();
}

/** The panels currently docked. For a test, and for a reader of this file. */
export function inTheDock(): Map<string, number> {
  return new Map(standing);
}

/** The minimised ones. For a test, and for a reader of this file. */
export function shrunk(): Set<string> {
  return new Set(small);
}

function apply(): void {
  const how = width(standing, small);
  const root = document.documentElement;
  root.classList.toggle("is-docked", how === "full");
  root.classList.toggle("is-docked-small", how === "small");
  // The stylesheet owns what the gap looks like; this owns how wide it is,
  // because how wide it is depends on which panels are standing.
  root.style.setProperty("--dock-now", `${room(standing, small)}px`);
}
