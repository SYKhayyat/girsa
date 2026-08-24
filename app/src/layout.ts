// The split tree, turned into boxes.
//
// The tree itself lives in Rust (`girsa_app::workspace::Layout`) and is tested
// there. This walks it and puts a `div` around each half, with a draggable
// divider between them.
//
// **RTL is not a stylesheet setting here.** With `direction: rtl` on a flex
// row, the *first* child is the rightmost one — so a Gemara split with Rashi
// beside it puts the Gemara on the right without anything reversing a list,
// which is where a person looking at a daf expects it.
//
// **A divider is named by which divider it is.** Pre-order, the order this walk
// meets them, and the same number Rust counts to. It used to be named by a pane
// beside it — `firstPaneOf(layout.first)`, which for a nested first child is a
// grandchild rather than a child — and a drag on the outer divider of
// `Split { Split { a | b } | c }` resized the inner one. See
// `girsa_app::workspace::Layout::at_split`.

import type { Layout, PaneId } from "./api.ts";
import { glyph } from "./controls.ts";
import { say } from "./say.ts";

export interface Boxes {
  /** The element to put in the window. */
  root: HTMLElement;
  /** Where each pane's own element goes. */
  slots: Map<PaneId, HTMLElement>;
}

/**
 * What a divider can do to the split it draws.
 *
 * > *"Tabs should be splittable in any way and movable, like we want in ksav."*
 *
 * On the divider rather than on a pane header, because all three are facts
 * about the **split** and not about either pane in it — and because the header
 * had eight controls in it already and a pane in a small window could not show
 * the ones it had (finding 8). A reader looking for *how are these two
 * arranged* looks at the line between them.
 */
export interface Hands {
  /** Where the divider was dropped, in tenths of a per cent. */
  onRatio: (split: number, ratio: number) => void;
  /** Side by side becomes one above the other, and back. */
  onTurn: (split: number) => void;
  /** The two halves change places. */
  onSwap: (split: number) => void;
}

export function build(
  layout: Layout,
  /** In tenths of a per cent, from `girsa_app::workspace` — never from here. */
  bounds: [number, number],
  hands: Hands,
): Boxes {
  const slots = new Map<PaneId, HTMLElement>();
  const root = walk(layout, bounds, slots, hands, { next: 0 });
  return { root, slots };
}

/** The divider counter, carried down the walk. */
interface Counting {
  next: number;
}

function walk(
  layout: Layout,
  bounds: [number, number],
  slots: Map<PaneId, HTMLElement>,
  hands: Hands,
  counting: Counting,
): HTMLElement {
  if (layout.kind === "leaf") {
    const slot = document.createElement("div");
    slot.className = "slot";
    slots.set(layout.pane, slot);
    return slot;
  }

  // Taken **before** the children are walked, because Rust counts pre-order:
  // the split, then everything inside its first half, then its second.
  const which = counting.next;
  counting.next += 1;

  const box = document.createElement("div");
  box.className = `split split-${layout.axis}`;
  const first = walk(layout.first, bounds, slots, hands, counting);
  const second = walk(layout.second, bounds, slots, hands, counting);
  const share = layout.ratio / 10;
  first.style.flexBasis = `${share}%`;
  second.style.flexBasis = `${100 - share}%`;

  const divider = document.createElement("div");
  divider.className = "divider";
  divider.setAttribute("role", "separator");
  divider.dataset.split = String(which);
  divider.tabIndex = 0;
  divider.title = say("dividerWhy");
  drag(divider, box, which, layout, bounds, first, second, hands);
  divider.append(controls(which, layout, hands));

  box.append(first, divider, second);
  return box;
}

/**
 * The two buttons on the divider.
 *
 * Quiet until the pointer is on the line, the same rule `.tab-shut` follows: a
 * pair of glyphs on every divider of a three-way split is a row of things to
 * click by accident. The line itself carries the sentence on its `title`, so
 * hovering it says what is there before the buttons have faded in.
 *
 * `stopPropagation` on `pointerdown`, or a click on a button starts a drag of
 * the line it is sitting on and the reader resizes the split they meant to
 * turn.
 */
function controls(
  which: number,
  layout: Extract<Layout, { kind: "split" }>,
  hands: Hands,
): HTMLElement {
  const row = document.createElement("div");
  row.className = "divider-controls";
  // The face is the arrangement it **gives you**, which is the convention the
  // toolbar's three state buttons already follow: a control labelled with the
  // state you are already in is a control nobody can predict.
  const stacking = layout.axis === "vertical";
  const turn = glyph(stacking ? "⇅" : "⇄", say(stacking ? "splitStacked" : "splitBeside"), () =>
    hands.onTurn(which),
  );
  const swap = glyph("⇌", say("swapSplit"), () => hands.onSwap(which));
  for (const control of [turn, swap]) {
    control.className = "divider-control";
    control.addEventListener("pointerdown", (event) => event.stopPropagation());
  }
  row.append(turn, swap);
  return row;
}

function drag(
  divider: HTMLElement,
  box: HTMLElement,
  which: number,
  layout: Extract<Layout, { kind: "split" }>,
  bounds: [number, number],
  first: HTMLElement,
  second: HTMLElement,
  hands: Hands,
): void {
  const move = (event: PointerEvent) => {
    const area = box.getBoundingClientRect();
    let share: number;
    if (layout.axis === "vertical") {
      // Right to left: the first child is on the right, so its share grows as
      // the pointer moves right.
      share = ((area.right - event.clientX) / area.width) * 100;
    } else {
      share = ((event.clientY - area.top) / area.height) * 100;
    }
    // The bounds Rust clamps to, not a second pair. `girsa_app::workspace`
    // holds `SMALLEST_SHARE`/`LARGEST_SHARE`, `Workspace::sane` applies them on
    // load as well as on the setter, and this draws a drag inside them so the
    // pointer and the stored value cannot disagree.
    share = Math.min(bounds[1] / 10, Math.max(bounds[0] / 10, share));
    first.style.flexBasis = `${share}%`;
    second.style.flexBasis = `${100 - share}%`;
    divider.dataset.share = String(Math.round(share * 10));
  };

  divider.addEventListener("pointerdown", (event) => {
    divider.setPointerCapture(event.pointerId);
    divider.classList.add("is-dragging");
    // Both ends are removed by name. With `{ once }` the listener that did
    // *not* fire stayed attached for the life of the element — one more drag,
    // one more stale `stop`, and a genuine pointercancel eventually fired N of
    // them, each reporting a ratio.
    const stop = () => {
      divider.classList.remove("is-dragging");
      divider.removeEventListener("pointermove", move);
      divider.removeEventListener("pointerup", stop);
      divider.removeEventListener("pointercancel", stop);
      hands.onRatio(which, Number(divider.dataset.share ?? layout.ratio));
    };
    divider.addEventListener("pointermove", move);
    divider.addEventListener("pointerup", stop);
    divider.addEventListener("pointercancel", stop);
  });

  // **The keyboard, on a control that has had `tabIndex = 0` and no handler.**
  // A separator a reader can focus and cannot use is a stop on the tab route
  // that does nothing. The arrows that move it are the ones that point along
  // its own axis, one per cent at a time.
  divider.addEventListener("keydown", (event) => {
    const along =
      layout.axis === "vertical"
        ? { ArrowRight: 10, ArrowLeft: -10 }
        : { ArrowDown: 10, ArrowUp: -10 };
    const step = (along as Record<string, number | undefined>)[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const moved = Math.min(bounds[1], Math.max(bounds[0], layout.ratio + step));
    hands.onRatio(which, moved);
  });
}
