// Only the newest answer is allowed to draw.
//
// > *"when i search, it gets the right search for a second, then that drops out
// > and it has a list of totally different things. the real search results only
// > flash there for a second or less."*
//
// Every panel in this window does the same thing: `await` a round trip, then
// `replaceChildren` with what came back. That is correct exactly once. Two of
// them in flight and the **slower** one wins, whichever question it was
// answering — so opening the search panel, which re-ran the previous query, and
// then typing a new one gave the new results for as long as it took the old
// query to come back, and then the old ones.
//
// It is not only the search. The picker asks once per keystroke; the shelf asks
// when a shelf is clicked and a fast reader clicks three; the links panel redraws
// on every lens; the lane runs a model and takes seconds. Every one of them can
// land out of order, and none of them could tell.
//
// # The shape
//
// A `Latest` hands out tickets. A ticket knows whether it is still the newest,
// and `run` simply does not call the drawing half when it is not:
//
// ```ts
// private readonly draws = new Latest();
// …
// await this.draws.run(
//   () => api.find(typed, page),
//   (found) => this.drawEverything(found),
// );
// ```
//
// **Not a cancellation.** The round trip completes; what is dropped is the
// drawing. Cancelling would need every command to take a signal, and the answer
// is usually already in flight in Rust by the time a reader has typed the next
// letter.
//
// # Why a class and not `if (query !== this.latest) return`
//
// Because that is what every one of these panels would have to grow, separately,
// and the way you find out one of them is missing it is a reader watching the
// right answer flash and vanish. One object, one rule, and `test/latest.test.mjs`
// holds it.

/** Hands out tickets, and knows which one is newest. */
export class Latest {
  private issued = 0;
  private newest = 0;

  /**
   * Take a ticket. Everything issued before it is now stale.
   *
   * For a caller that cannot use [`Latest.run`] — one that has to interleave
   * other work between the ask and the draw.
   */
  take(): Ticket {
    this.issued += 1;
    this.newest = this.issued;
    const mine = this.issued;
    return {
      current: () => mine === this.newest,
    };
  }

  /**
   * Ask, and draw only if nothing newer was asked while waiting.
   *
   * Returns whether it drew, which is what a test asserts on and what a caller
   * can use to skip its own follow-up work.
   */
  async run<T>(ask: () => Promise<T>, draw: (answer: T) => void): Promise<boolean> {
    const ticket = this.take();
    const answer = await ask();
    if (!ticket.current()) return false;
    draw(answer);
    return true;
  }

  /**
   * The same, for an ask that can fail.
   *
   * A failure is still an answer and still has to be dropped when it is stale —
   * a panel that showed *could not read that* from a request the reader has
   * already replaced is the same bug wearing an error message.
   */
  async attempt<T>(
    ask: () => Promise<T>,
    draw: (answer: T) => void,
    failed: (error: unknown) => void,
  ): Promise<boolean> {
    const ticket = this.take();
    try {
      const answer = await ask();
      if (!ticket.current()) return false;
      draw(answer);
      return true;
    } catch (error) {
      if (!ticket.current()) return false;
      failed(error);
      return true;
    }
  }
}

/** One request's claim on the panel it will draw into. */
export interface Ticket {
  /** Whether nothing newer has been asked since this one was taken. */
  current: () => boolean;
}
