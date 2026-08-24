// Find inside the sefer in front of you — Ctrl+F.
//
// # What it is
//
// > *"the search should be the same as regular girsa search (with all the
// > options)."*
//
// It is. The modes, the match, the together, the refusals and the widening all
// come out of `girsa_search` scoped to one sefer, and the chip row is drawn by
// the same `chips.ts` the search panel draws its own with. What is different is
// the **shape of the answer**: a search hands you a ranked list to read; a find
// hands you the next one, then the one after that, and a count — and leaves
// nothing on screen except a highlight in the words you were already looking
// at.
//
// # Why the whole sefer is scanned in Rust
//
// The pane holds a window of a few hundred lines around where the reader is
// standing. A find written here would search what happens to be loaded and
// report *no more matches* while sitting on eleven — see `girsa_app::inside`,
// which is also where the engine's byte marks become offsets the pane can
// highlight, and why those are two different things.

import { api, type Chip, type FoundHere } from "./api.ts";
import { chipRow } from "./chips.ts";
import { field, glyph } from "./controls.ts";
import { Latest } from "./latest.ts";
import { say } from "./say.ts";
import { sayTrouble } from "./trouble.ts";
import type { PaneView } from "./pane.ts";

/** How long a pause counts as *stopped typing*. */
const SETTLED = 160;

/**
 * The one option on the row this bar cannot honour, and why.
 *
 * The row is the search's own — `girsa_search::chips` decides what a chip is,
 * and a webview that assembled its own would be a second opinion about what the
 * engine can do. But one of the modes it offers cannot mean anything here, and
 * offering it anyway was a control that quietly found nothing: `sefer_find`
 * matches `Answer::Cited(_)` and returns an empty list, with a comment saying
 * exactly that — *the bar is inside one sefer and a citation is a jump
 * somewhere else*.
 *
 * **It is one option and not two.** The handoff that filed this said the
 * instruments were the other, on the grounds that gematria and remazim are a
 * whole-shelf instrument. They are not: `Bar::by_instrument` passes
 * `chips.scope` to `prepare_instrument`, and `Bar::over_the_text` — which is
 * how a dilug and a notarikon are run — *refuses* a scope naming more than a
 * few seforim. One sefer is the case those two want. Greying them out would
 * have taken away the only place they work.
 */
function cannotHere(): Record<string, string> {
  return { "mode/Citation": say("findHereNoCitation") };
}

/**
 * `3 / 41`, or what there is instead of that.
 *
 * Four states, and three of them are not a number. Written out here rather than
 * inside the class because it is the whole of what the count element ever says,
 * and because the defect it carries is not visible from any of them: `1 / 33`
 * in a right-to-left window is laid out `33 / 1`, so this string is correct and
 * what the reader sees is backwards. The `dir` attribute is what fixes that and
 * it is set where the element is built, not here — a string cannot carry its
 * own direction.
 */
export function countSaid(query: string, at: number, total: number, shown: number): string {
  if (!query.trim()) return "";
  if (total === 0) return say("findHereNone");
  const place = `${at + 1} / ${total}`;
  // The list is cut and the count is not. `3 / 900` alone would promise 900
  // stops where there are 500.
  return total > shown ? `${place} ${say("findHereCut")}` : place;
}

/**
 * The next place, or the previous one, wrapping round.
 *
 * Wrapping without saying so: a find bar that stops at the end of the sefer
 * leaves a reader pressing a key that does nothing, and every application that
 * has ever had this bar wraps. `%` in JavaScript keeps the sign of the left
 * operand, so `-1 % 33` is `-1` and not `32` — the `+ places` is what makes
 * Shift+Enter on the first match land on the last one instead of on nothing.
 */
export function stepTo(at: number, by: number, places: number): number {
  if (places === 0) return 0;
  return (at + by + places) % places;
}

export class FindHere {
  readonly element: HTMLElement;
  private readonly box: HTMLInputElement;
  private readonly count: HTMLElement;
  private readonly note: HTMLElement;
  private chips: HTMLElement;
  private pane: PaneView | null = null;
  private places: FoundHere[] = [];
  private total = 0;
  private at = 0;
  private open = false;
  /** The keystroke that has not been searched yet. */
  private waiting: number | null = null;
  /** What was last asked for, so a repeat keystroke does not re-ask. */
  private asked: string | null = null;
  /**
   * The stale-answer guard `latest.ts` exists so that no panel has to grow its
   * own — and which this file, of all of them, did not use.
   *
   * Seven panels take tickets: the picker, the search panel, the shelf, the
   * scope, the contents, the links and the chains. This one guarded with
   * `if (query === this.asked) return`, and set `this.asked` **before** the
   * await, which is not the same thing and misses in two ways:
   *
   * * **Out of order.** Clicking a chip resets `asked` to `null` and asks
   *   again, so a chip changed mid-flight left two asks running and
   *   `this.places` belonged to whichever came back last rather than to what
   *   the reader last asked.
   * * **A closed bar still moved the page.** `close()` hid the element and
   *   cleared the debounce and did not cancel the ask — so an answer landing
   *   after the close ran `show()` → `goToWords`, scrolling the reader
   *   somewhere they had not asked to be. Directly against `close`'s own
   *   comment that *the reader keeps the place the find put them on*.
   *
   * The debounce above is a different thing and both are needed: the picker's
   * own note says so, and the picker has both. The file being held up as the
   * model for the debounce had one.
   */
  private readonly draws = new Latest();

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "find-here";
    this.element.hidden = true;

    const row = document.createElement("div");
    row.className = "find-here-row";

    this.box = field(say("findHere"), {
      className: "find-here-box",
      placeholder: say("findHerePlaceholder"),
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
    // **`dir="ltr"`, and this is not a detail.** In a right-to-left window a
    // bare `1 / 33` is laid out by the bidi algorithm as `33 / 1`, so the bar
    // reported the reader's position and the total the wrong way round — read
    // straight off a screenshot of the running window.
    this.count.setAttribute("dir", "ltr");
    this.count.setAttribute("aria-live", "polite");

    const walk = document.createElement("div");
    walk.className = "find-here-walk";
    walk.append(
      glyph("↑", say("findHerePrevious"), () => this.step(-1)),
      glyph("↓", say("findHereNext"), () => this.step(1)),
    );

    row.append(this.box, this.count, walk, glyph("✕", say("findHereClose"), () => this.close()));

    this.chips = document.createElement("div");
    this.chips.className = "find-chips";
    this.note = document.createElement("p");
    this.note.className = "find-here-note";

    this.element.append(row, this.chips, this.note);
  }

  get isOpen(): boolean {
    return this.open;
  }

  /**
   * Open over a pane, with whatever is already typed.
   *
   * Pressing the key again **selects what is in the box** rather than clearing
   * it, so a second Ctrl+F is *look for something else* and a reader who wanted
   * the same phrase can press Enter. That is what every editor does.
   */
  openOn(pane: PaneView): void {
    if (this.pane !== pane) {
      this.places = [];
      this.total = 0;
      this.at = 0;
      this.asked = null;
      this.say();
    }
    this.pane = pane;
    pane.element.append(this.element);
    // **Under the pane's own header, measured.** It floated at a fixed offset
    // from the top of the pane and landed on top of the header's buttons —
    // read off a screenshot of the running window, where it covered *The
    // chain*. A constant cannot be right here: the header wraps to two lines on
    // a narrow pane and carries a following-chip on some of them.
    const head = pane.element.querySelector<HTMLElement>(".pane-head");
    this.element.style.setProperty("--under-head", `${(head?.offsetHeight ?? 34) + 4}px`);
    this.element.hidden = false;
    this.open = true;
    this.box.focus();
    this.box.select();
    // Ask with whatever is in the box — `""` on a fresh open, which draws the
    // chip row without searching. The panel does the same and for the same
    // reason: a reader has to be able to see what the options are before
    // deciding whether to change one.
    void this.look();
  }

  close(): void {
    this.open = false;
    this.element.hidden = true;
    this.element.remove();
    if (this.waiting !== null) window.clearTimeout(this.waiting);
    this.waiting = null;
    // Burns the ticket of anything in flight. The round trip still completes —
    // `Latest` is not a cancellation — but nothing it comes back with may draw
    // or scroll into a bar the reader has closed.
    this.draws.take();
    // The reader keeps the place the find put them on. Nothing is scrolled
    // back.
    this.pane?.element.focus();
  }

  /** A pane went away; do not hold a handle to it. */
  forget(pane: PaneView): void {
    if (this.pane !== pane) return;
    this.close();
    this.pane = null;
  }

  private typed(): void {
    if (this.waiting !== null) window.clearTimeout(this.waiting);
    // Debounced, because every keystroke is a query against the index. 160ms is
    // a pause a person makes between words and does not make inside one.
    this.waiting = window.setTimeout(() => void this.look(), SETTLED);
  }

  private async look(): Promise<void> {
    const pane = this.pane;
    const query = this.box.value;
    if (!pane) return;
    if (query === this.asked) return;
    this.asked = query;
    this.note.replaceChildren();
    // The ticket is taken here, checked before anything of this answer's is
    // written down, and checked **again** before the page is scrolled — because
    // between the two the reader may have closed the bar, and a scroll is the
    // one thing this panel does that cannot be undrawn.
    const ticket = this.draws.take();
    try {
      const found = await api.seferFind(pane.slug, query);
      if (!ticket.current()) return;
      this.places = found.places;
      this.total = found.total;
      this.drawChips(found.chips);
      // A refusal in the engine's own words: a regex that will not compile, an
      // index that has not been built. Said, not swallowed — a bar that went
      // quiet on a bad pattern reads as a bar that found nothing.
      if (found.refused) sayTrouble(this.note, found.refused, "general");
    } catch (e) {
      if (!ticket.current()) return;
      this.places = [];
      this.total = 0;
      sayTrouble(this.note, e, "general");
    }
    this.at = 0;
    this.say();
    if (this.places.length > 0 && ticket.current() && this.open) await this.show();
  }

  /** The options, drawn by the same function the search panel draws them with. */
  private drawChips(chips: Chip[]): void {
    const row = chipRow(chips, {
      cannot: cannotHere(),
      chosen: async (chip, key) => {
        await api.findHereChip(chip, key);
        // The options changed, so the same words are a different question.
        this.asked = null;
        await this.look();
      },
    });
    this.chips.replaceWith(row);
    this.chips = row;
  }

  /** Walk to the next place, or the previous one. See `stepTo`. */
  private step(by: number): void {
    if (this.places.length === 0) return;
    this.at = stepTo(this.at, by, this.places.length);
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

  /** What the count element says. See `countSaid`. */
  private say(): void {
    this.count.textContent = countSaid(this.box.value, this.at, this.total, this.places.length);
  }
}
