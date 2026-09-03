// The scope panel's shelf tree, and what a failed read is allowed to be.
//
// `scopeview.ts` used to load the tree with `.catch(() => [])`. That cached a
// failure as *loaded, empty* for the life of the view: the gate was
// `branches.length === 0`, so nothing ever read again, and a reader whose
// library hiccuped once saw a scope panel with no shelves in it, with no error
// and no way back — the same disease `twist()` already cured for the seforim
// on a shelf, left in for the tree itself.
//
// `folded` is the state machine the refresh now runs through. Its one rule is
// the fix: **a failure does not become a cached empty tree.** `null` branches
// mean *not successfully read*, so a failed read leaves them `null`, the next
// `refresh()` reads again, and a success clears the trouble sentence the tree
// was showing.

import { check, notOk, ok } from "./harness.mjs";
import { folded } from "../.tmp-test/scopeview.mjs";
import { sayIn } from "../.tmp-test/say.mjs";

/** A shelf, with only the fields the fold cares about. */
function branch(key, count = 1) {
  return { key, title: key, count, here: 0, mine: false, edited: false, children: [] };
}

const TWO = [branch("tanakh"), branch("shas")];

/** A read that counts the attempts, so a test can tell a retry from a cache. */
function reading(attempts, answer) {
  attempts.value += 1;
  return Promise.resolve(answer);
}

export async function run() {
  // --- the bug: a failure was cached as an empty tree ------------------------
  //
  // The old code returned `[]` on failure, and the tree stayed empty and silent
  // for the whole view. The one thing the fix must not do is hand a failure to
  // the panel as a shelf with nothing on it.
  const failed = await folded(null, null, null, () => Promise.reject(new Error("no shelf")));
  ok(
    "a failed read leaves the tree unloaded, not cached empty",
    failed.branches === null,
  );
  ok("a failed read says something", typeof failed.trouble === "string" && failed.trouble.length > 0);
  ok("and keeps the machine's words for the hover", failed.detail === "no shelf");

  // --- the retry: the next refresh reads again ---------------------------------
  //
  // Because the failure left `branches === null`, the fold has no cache to
  // honour and must ask again. This is the whole recovery — a transient failure
  // is over by the next refresh.
  const attempts = { value: 0 };
  await folded(null, "a past trouble", "old detail", () => reading(attempts, TWO));
  const retried = await folded(null, null, null, () => reading(attempts, TWO));
  check("a failure does not stop the next read", attempts.value, 2);
  check("and the retry can load the tree", retried.branches, TWO);
  check("a success clears the trouble", retried.trouble, null);
  check("and clears the detail with it", retried.detail, null);

  // --- a success is a cache, not a re-read ------------------------------------
  //
  // The tree changes when the reader rearranges the bookcase, not when they
  // search — which is why the old code read it once and kept it. The fix keeps
  // that property: once loaded, a later refresh must not ask again.
  const once = { value: 0 };
  await folded(null, null, null, () => reading(once, TWO));
  const cached = await folded(TWO, null, null, () => reading(once, TWO));
  check("a loaded tree is not read again", once.value, 1);
  check("and a cached tree survives the fold", cached.branches, TWO);

  // --- the sentence is the window's, and names what was being read -------------
  //
  // `folded` routes the failure through `trouble()`, so the tree shows a Hebrew
  // sentence naming the read — never the machine's English string as the
  // message. The Hebrew window is the default, same as `trouble.test.mjs`.
  const said = await folded(null, null, null, () => Promise.reject(new Error("boom")));
  notOk("the sentence has no Latin in it", /[A-Za-z]/.test(said.trouble));
  ok(
    "the sentence names the shelf read",
    said.trouble.includes(sayIn("doingReadShelf", "hebrew")),
  );
  check("and the machine's string is only on the hover", said.detail, "boom");
}