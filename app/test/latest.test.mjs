// Only the newest answer draws.
//
// > *"when i search, it gets the right search for a second, then that drops out
// > and it has a list of totally different things. the real search results only
// > flash there for a second or less."*
//
// The panel opened, re-ran the **previous** query, and the reader typed a new
// one on top of it. Two round trips in flight; the older, broader, slower one
// came back second and `replaceChildren` did what it was told. Nothing in the
// window had any idea which answer belonged to which question.
//
// This is that rule as a unit test, which it can be because `Latest` is a
// function of nothing but the order things were asked in.

import { check, ok, notOk } from "./harness.mjs";
import { Latest } from "../.tmp-test/latest.mjs";

/** A promise that resolves when told to. */
function held() {
  let release;
  const promise = new Promise((resolve) => {
    release = resolve;
  });
  return { promise, release };
}

export async function run() {
  // ---------------------------------------------------- the bug, reproduced

  {
    const slow = held();
    const fast = held();
    const drawn = [];
    const latest = new Latest();

    // The panel opens and re-runs the previous, broad query.
    const first = latest.run(
      () => slow.promise,
      (answer) => drawn.push(answer),
    );
    // The reader types and presses Enter before it comes back.
    const second = latest.run(
      () => fast.promise,
      (answer) => drawn.push(answer),
    );

    fast.release("what the reader asked for");
    check("the newer answer draws", await second, true);
    slow.release("the previous query");
    notOk("and the older one, arriving second, does not", await first);
    check("so the panel holds exactly one list", drawn, ["what the reader asked for"]);
  }

  // ---------------------------------------------------- the ordinary case

  {
    const latest = new Latest();
    const drawn = [];
    check(
      "one ask on its own draws",
      await latest.run(
        () => Promise.resolve("only"),
        (answer) => drawn.push(answer),
      ),
      true,
    );
    check("and it is what came back", drawn, ["only"]);
  }

  // Three in a row, resolving in the order they were asked: every one of them is
  // the newest when it lands, so every one draws. A guard that dropped these
  // would break the ordinary case to fix the race.
  {
    const latest = new Latest();
    const drawn = [];
    for (const n of [1, 2, 3]) {
      await latest.run(
        () => Promise.resolve(n),
        (answer) => drawn.push(answer),
      );
    }
    check("answers that land in order all draw", drawn, [1, 2, 3]);
  }

  // ---------------------------------------------------- a failure is an answer

  {
    const slow = held();
    const latest = new Latest();
    const said = [];
    const first = latest.attempt(
      () => slow.promise.then(() => Promise.reject(new Error("no index"))),
      () => said.push("drew"),
      (e) => said.push(`failed: ${e.message}`),
    );
    await latest.attempt(
      () => Promise.resolve("fresh"),
      (answer) => said.push(answer),
      () => said.push("failed"),
    );
    slow.release();
    notOk("a stale failure is dropped like a stale answer", await first);
    check(
      "so a refusal from a question nobody asked any more is not shown",
      said,
      ["fresh"],
    );
  }

  {
    const latest = new Latest();
    const said = [];
    await latest.attempt(
      () => Promise.reject(new Error("no index")),
      () => said.push("drew"),
      (e) => said.push(e.message),
    );
    check("and a current failure is shown", said, ["no index"]);
  }

  // ---------------------------------------------------- tickets, for the callers
  // that cannot use `run`

  {
    const latest = new Latest();
    const mine = latest.take();
    ok("a ticket is current the moment it is taken", mine.current());
    latest.take();
    notOk("and stops being current when a newer one is taken", mine.current());
  }
}
