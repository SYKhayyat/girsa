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

import type { Layout, PaneId } from "./api.ts";

export interface Boxes {
  /** The element to put in the window. */
  root: HTMLElement;
  /** Where each pane's own element goes. */
  slots: Map<PaneId, HTMLElement>;
}

export function build(
  layout: Layout,
  onRatio: (pane: PaneId, ratio: number) => void,
): Boxes {
  const slots = new Map<PaneId, HTMLElement>();
  const root = walk(layout, slots, onRatio);
  return { root, slots };
}

function walk(
  layout: Layout,
  slots: Map<PaneId, HTMLElement>,
  onRatio: (pane: PaneId, ratio: number) => void,
): HTMLElement {
  if (layout.kind === "leaf") {
    const slot = document.createElement("div");
    slot.className = "slot";
    slots.set(layout.pane, slot);
    return slot;
  }

  const box = document.createElement("div");
  box.className = `split split-${layout.axis}`;
  const first = walk(layout.first, slots, onRatio);
  const second = walk(layout.second, slots, onRatio);
  const share = layout.ratio / 10;
  first.style.flexBasis = `${share}%`;
  second.style.flexBasis = `${100 - share}%`;

  const divider = document.createElement("div");
  divider.className = "divider";
  divider.setAttribute("role", "separator");
  divider.tabIndex = 0;
  drag(divider, box, layout, first, second, onRatio);

  box.append(first, divider, second);
  return box;
}

function drag(
  divider: HTMLElement,
  box: HTMLElement,
  layout: Extract<Layout, { kind: "split" }>,
  first: HTMLElement,
  second: HTMLElement,
  onRatio: (pane: PaneId, ratio: number) => void,
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
    share = Math.min(85, Math.max(15, share));
    first.style.flexBasis = `${share}%`;
    second.style.flexBasis = `${100 - share}%`;
    divider.dataset.share = String(Math.round(share * 10));
  };

  divider.addEventListener("pointerdown", (event) => {
    divider.setPointerCapture(event.pointerId);
    divider.classList.add("is-dragging");
    const stop = () => {
      divider.classList.remove("is-dragging");
      divider.removeEventListener("pointermove", move);
      const share = Number(divider.dataset.share ?? layout.ratio);
      const pane = firstPaneOf(layout.first);
      if (pane !== null) onRatio(pane, share);
    };
    divider.addEventListener("pointermove", move);
    divider.addEventListener("pointerup", stop, { once: true });
    divider.addEventListener("pointercancel", stop, { once: true });
  });
}

function firstPaneOf(layout: Layout): PaneId | null {
  return layout.kind === "leaf" ? layout.pane : firstPaneOf(layout.first);
}
