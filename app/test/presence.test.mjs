// The three presence states, and the three sentences.
//
// The chip used to say two things for three states and put `PostError`'s English
// `Display` string in one of them. What a reader saw on the first screen of the
// desktop app was:
//
//     could not reach ksav: connection timed out — כסב
//
// Every assertion here is about that line: the name, the language, and the fact
// that `stale` is the one state of the three that tells a reader something they
// can act on.

import { check, ok, notOk } from "./harness.mjs";
import { presenceSaid } from "../.tmp-test/presence.mjs";
import { KSAV } from "../.tmp-test/names.mjs";

/** The exact string that was on the first screen. */
const OBSERVED = "could not reach ksav: connection timed out";

/** Any Latin letter in a sentence meant for a Hebrew toolbar. */
const LATIN = /[A-Za-z]/;

export async function run() {
  const live = presenceSaid({ state: "live", version: "0.1.0" });
  ok("live names the application", live.said.includes(KSAV));
  ok("live offers the send", live.canSend);
  notOk("live is not trouble", live.trouble);

  const off = presenceSaid({ state: "not_running" });
  ok("not_running names the application", off.said.includes(KSAV));
  notOk("not_running does not offer the send", off.canSend);
  notOk("not_running is not trouble — it is a choice, not a fault", off.trouble);
  check("and has nothing to hover", off.detail, "");

  const stale = presenceSaid({ state: "stale", why: OBSERVED });
  ok("stale names the application", stale.said.includes(KSAV));
  notOk("stale does not offer the send", stale.canSend);
  ok("stale is trouble", stale.trouble);

  // The finding, stated: the transport's English string is not the message.
  notOk("stale's sentence does not carry the transport string", stale.said.includes(OBSERVED));
  notOk("stale's sentence has no Latin in it at all", LATIN.test(stale.said));
  check("the transport string is behind the details affordance", stale.detail, OBSERVED);

  // Three states, three distinct sentences. Collapsing two of them is how the
  // only actionable one got lost.
  const all = [live.said, off.said, stale.said];
  check("three states give three different sentences", new Set(all).size, 3);

  // Nowhere does the misspelling survive.
  for (const s of all) notOk(`"${s}" is not the transliteration`, s.includes("כסב"));

  // And stale says what to do about it, not merely that something is wrong.
  ok("stale's sentence says the sibling may have closed badly", stale.said.includes("נסגר"));
}
