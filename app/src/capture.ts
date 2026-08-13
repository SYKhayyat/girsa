// Waiting for one key, and being sure the waiting ends.
//
// # The bug
//
// Rebinding a shortcut works by listening: the row's button shows `…` and the
// next key you press is bound. The listener went on `window` in the capture
// phase, and it was removed **in exactly one place** — inside itself, when a
// key arrived. So every other way of ending it left it armed:
//
// ```
// click "Ctrl+O" row → close panel with × → press "g"
// → open = G          (Ctrl+O no longer opens anything)
// ```
//
// No confirmation, no message, and the only way back is the `↺` in a panel you
// may no longer be able to open with its shortcut. Closing was not the only
// door out: clicking a second row's key button armed a second listener beside
// the first, so one press bound two actions and the first row went on showing
// `…` forever; and any redraw of the panel — changing a font, changing the
// theme — replaced the button while the listener kept its reference to it.
//
// # The shape
//
// The class is not *remove it on close*. It is **at most one capture exists,
// and everything that ends it ends it**. Three lines of `removeEventListener`
// at three call sites is the same bug three times: the fourth door is the one
// nobody adds a line for.
//
// So one object owns the waiting. Starting a second capture cancels the first
// and tells it so, `stop()` cancels from anywhere, and a key that arrives
// cancels before it reports. A caller keeps one of these and calls `stop()`
// wherever it likes; calling it when nothing is waiting is the ordinary case
// and does nothing.
//
// # Why it takes its listeners as an argument
//
// So it can be tested. `window` is not there in a unit test, and a rule about
// *when a listener is removed* that cannot be run without a browser is a rule
// held by nobody — which is how this one lasted. `test/capture.test.mjs` drives
// it with a fake pair and counts what is registered.

/** Where a capture listens. `window` in the application; a counter in a test. */
export interface Ears {
  addEventListener(type: "keydown", fn: (event: KeyboardEvent) => void, capture: boolean): void;
  removeEventListener(type: "keydown", fn: (event: KeyboardEvent) => void, capture: boolean): void;
}

/**
 * A key on its own is somebody on their way to a combination.
 *
 * Holding Ctrl to press Ctrl+Shift+K fires three `keydown`s, and binding the
 * first of them would make every shortcut in the table `Ctrl`.
 */
const ON_THE_WAY = ["Control", "Shift", "Alt", "Meta", "AltGraph"];

/** What ended a capture, told to whoever was waiting. */
export type Ended = "took" | "cancelled";

/** At most one wait for a key, cancellable from anywhere. */
export class OneKey {
  private stopWaiting: ((why: Ended) => void) | null = null;

  /** Whether something is waiting for a key right now. */
  get waiting(): boolean {
    return this.stopWaiting !== null;
  }

  /**
   * Wait for one key.
   *
   * `ended` is called exactly once, whichever way the wait ends: with the key
   * that arrived, or with nothing because something cancelled. A caller that
   * put a `…` on a button puts the old label back in the `cancelled` branch
   * and has no other bookkeeping to do.
   *
   * Starting a second wait cancels the first — which is what clicking another
   * row's key button means, and what used to arm two listeners at once.
   */
  wait(ears: Ears, ended: (key: KeyboardEvent | null) => void): void {
    this.stop();
    const heard = (event: KeyboardEvent): void => {
      // Nothing else in the window sees a key while one is being bound: the
      // reader is aiming at a shortcut, and the shortcut they are aiming at
      // must not fire on the way.
      event.preventDefault();
      event.stopPropagation();
      if (ON_THE_WAY.includes(event.key)) return;
      // Escape is how you back out, and it is the one key that can never be
      // bound — a reader who bound Escape would have no way to cancel the next
      // binding, including that one.
      finish(event.key === "Escape" ? "cancelled" : "took", event);
    };
    const finish = (why: Ended, event: KeyboardEvent | null): void => {
      ears.removeEventListener("keydown", heard, true);
      this.stopWaiting = null;
      ended(why === "took" ? event : null);
    };
    this.stopWaiting = (why) => finish(why, null);
    ears.addEventListener("keydown", heard, true);
  }

  /**
   * End the wait, if there is one.
   *
   * Safe to call when nothing is waiting, which is most of the time and the
   * whole point: a caller sprinkles it over every door out without having to
   * know which door the reader used.
   */
  stop(): void {
    this.stopWaiting?.("cancelled");
  }
}
