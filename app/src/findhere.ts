// Find a phrase inside the sefer in front of you — Ctrl+F, in the sense every
// other application has meant by Ctrl+F for forty years.
//
// # Why this is not the search panel
//
// > *"narrowing a global search by facet is not the same gesture as Ctrl+F in
// > the Mishnah Berurah in front of you."*
//
// It is not, and the difference is not the scope — it is the shape of the
// answer. A search hands you a ranked list to read; a find hands you the next
// one, and then the one after that, and a count. Nothing is left on screen
// afterwards except a highlight in the words you were already looking at.
//
// So this is a bar and not a panel: one row, over the pane it belongs to, and
// it closes itself when you press Escape and leaves the page where it put you.
//
// # Why the whole sefer is scanned in Rust
//
// The pane holds a window of a few hundred lines around where the reader is
// standing. A find written here would search what happens to be loaded and
// report *no more* while sitting on eleven matches — see
// `girsa_app::inside`, which is where the scan is and why.

import { api, type Found } from "./api.ts";
import { field, glyph } from "./controls.ts";
import { say } from "./say.ts";
import type { PaneView } from "./pane.ts";

/** How long a pause counts as *stopped typing*. */
const SETTLED = 140;

export class FindHere {
  readonly element: HTMLElement;
  private readonly box: HTMLInputElement;
  private readonly count: HTMLElement;
  private pane: PaneView | null = null;
  private places: Found[] = [];
  private total = 0;
  private at = 0;
  private open = false;
  /** The keystroke that has not been searched yet. */
  private waiting: number | null = null;
  /** What was last asked for, so a repeat keystroke does not re-ask. */
  private asked = "";

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "find-here";
    this.element.hidden = true;

    this.box = field(say("findHere"), {
      className: "find-here-box",
      placeholder: say("findHerePlaceholder"),
      dir: "rtl",
    });
    this.box.addEventListener("input", () => this.typed());
    this.box.addEventListener("keydown", (event) => {
      if (event.key !== "Enter") return;
      event.preventDefault();
      // Shift+Enter walks backwards, which is what a find bar does everywhere.
      this.step(event.shiftKey ? -1 : 1);
    });

    this.count = document.createElement("span");
    this.count.className = "find-here-count";
    this.count.setAttribute("aria-live", "polite");

    const back = glyph("↑", say("findHerePrevious"), () => this.step(-1));
    const on = glyph("↓", say("findHereNext"), () => this.step(1));
    const shut = glyph("✕", say("findHereClose"), () => this.close());
    this.element.append(this.box, this.count, back, on, shut);
  }

  get isOpen(): boolean {
    return this.open;
  }

  /**
   * Open over a pane, with whatever is already typed.
   *
   * Pressing the key again while it is open **selects what is in the box**
   * rather than clearing it, so a second Ctrl+F is *search for something else*
   * and a reader who wanted the same phrase can press Enter. That is what every
   * editor does and it is the reason the box is not reset here.
   */
  openOn(pane: PaneView): void {
    if (this.pane !== pane) {
      this.places = [];
      this.total = 0;
      this.at = 0;
      this.asked = "";
      this.say();
    }
    this.pane = pane;
    pane.element.append(this.element);
    this.element.hidden = false;
    this.open = true;
    this.box.focus();
    this.box.select();
    if (this.box.value.trim()) void this.look();
  }

  close(): void {
    this.open = false;
    this.element.hidden = true;
    this.element.remove();
    if (this.waiting !== null) window.clearTimeout(this.waiting);
    this.waiting = null;
    // The pane keeps the keyboard, and the reader keeps the place the find put
    // them on. Nothing is scrolled back.
    this.pane?.element.focus();
  }

  /** A pane went away; do not hold a handle to it. */
  forget(pane: PaneView): void {
    if (this.pane === pane) this.close();
    if (this.pane === pane) this.pane = null;
  }

  private typed(): void {
    if (this.waiting !== null) window.clearTimeout(this.waiting);
    // Debounced, because every keystroke is a whole-sefer scan. 140ms is a
    // pause a person makes between words and does not make inside one.
    this.waiting = window.setTimeout(() => void this.look(), SETTLED);
  }

  private async look(): Promise<void> {
    const pane = this.pane;
    const query = this.box.value;
    if (!pane) return;
    if (query === this.asked) return;
    this.asked = query;
    try {
      const found = await api.seferFind(pane.slug, query);
      this.places = found.places;
      this.total = found.total;
    } catch {
      // A sefer that will not open is said by whatever opened it. A find bar
      // reporting it a second time is noise over the same fact.
      this.places = [];
      this.total = 0;
    }
    this.at = 0;
    this.say();
    if (this.places.length > 0) await this.show();
  }

  /**
   * Walk to the next place, or the previous one, wrapping round.
   *
   * Wrapping without saying so: a find bar that stops at the end of the sefer
   * leaves a reader pressing a key that does nothing, and every application
   * that has ever had this bar wraps.
   */
  private step(by: number): void {
    if (this.places.length === 0) return;
    this.at = (this.at + by + this.places.length) % this.places.length;
    this.say();
    void this.show();
  }

  private async show(): Promise<void> {
    const place = this.places[this.at];
    if (!place || !this.pane) return;
    await this.pane.goToWords(place.id, place.from, place.to);
    // The bar is inside the pane and the pane just scrolled; the caret goes
    // back to the box so the next Enter is another step rather than nothing.
    this.box.focus();
  }

  /** `3 / 41`, or what there is instead of that. */
  private say(): void {
    if (!this.box.value.trim()) {
      this.count.textContent = "";
      return;
    }
    if (this.total === 0) {
      this.count.textContent = say("findHereNone");
      return;
    }
    const shown = `${this.at + 1} / ${this.total}`;
    // The list is cut at 500 and the count is not. Saying only `3 / 900` would
    // promise 900 stops when there are 500 — so the difference is on the bar.
    this.count.textContent =
      this.total > this.places.length ? `${shown} ${say("findHereCut")}` : shown;
  }
}
