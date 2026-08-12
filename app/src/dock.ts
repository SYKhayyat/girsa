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

/** The panels standing in the dock. */
const standing = new Set<string>();

/** …and which of those are minimised to a strip. A subset of `standing`: a
 * panel that is not docked cannot be a strip. */
const small = new Set<string>();

/**
 * Whether the reading has to make room.
 *
 * A function of the set and nothing else — the reason this file exists, and what
 * makes the rule testable without a window.
 */
export function docked(panels: Set<string>): boolean {
  return panels.size > 0;
}

/**
 * How much room: a column, or a strip.
 *
 * A strip only when **everything** docked is minimised. One panel open beside
 * one minimised panel is a column wide, because the open one is standing in it —
 * and the two of them sharing an edge is what `--dock` being one width is for.
 */
export function width(panels: Set<string>, minimised: Set<string>): "none" | "small" | "full" {
  if (panels.size === 0) return "none";
  return [...panels].every((panel) => minimised.has(panel)) ? "small" : "full";
}

/** Put a panel in the dock. */
export function dock(panel: string): void {
  standing.add(panel);
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
export function inTheDock(): Set<string> {
  return new Set(standing);
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
}
