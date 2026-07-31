// A caught error, as a sentence a reader can act on.
//
// There were ten `textContent = String(e)` sites in this application, so a reader
// whose PDF would not open was shown a Rust `io::Error` in English. The rule this
// file holds is the one the fix adopted: every user-visible failure names what
// failed in the reader's words, the thing they can act on, and exactly one place
// to look. The raw string is kept — on `title` — and is never the message.

import { check, ok, notOk } from "./harness.mjs";
import { trouble, raw } from "../.tmp-test/trouble.mjs";

const LATIN = /[A-Za-z]/;

/** Real strings, as `girsa-post` and the shell produce them. */
const REAL = [
  ["ksav is not running", "reach_ksav"],
  ["could not reach ksav: connection timed out", "reach_ksav"],
  ["ksav refused it: 413 body too large", "send_to_ksav"],
  ["The system cannot find the file specified. (os error 2)", "open_pdf"],
  ["Access is denied. (os error 5)", "open_pdf"],
  ["No connection could be made because the target machine actively refused it", "send_to_ksav"],
  ["expected value at line 1 column 1", "read_links"],
  ["something nobody has ever seen before", "read_page"],
];

export async function run() {
  // The rule, over every shape the machine actually produces.
  for (const [message, doing] of REAL) {
    const t = trouble(message, doing);
    notOk(`"${message.slice(0, 34)}…" → no Latin in the sentence`, LATIN.test(t.said));
    ok(`"${message.slice(0, 34)}…" → the sentence is not empty`, t.said.length > 8);
    check(`"${message.slice(0, 34)}…" → the raw string is kept`, t.detail, message);
    notOk(`"${message.slice(0, 34)}…" → the raw string is not the sentence`, t.said.includes(message));
  }

  // Same underlying error, two things being attempted, two sentences: a timeout
  // reaching a sibling application and a timeout reading a page are the same
  // `io::Error` and entirely different things to be told.
  const a = trouble("connection timed out", "reach_ksav");
  const b = trouble("connection timed out", "read_page");
  ok("a timeout reaching the sibling names the sibling", a.said.includes("כְּתָב"));
  ok("a timeout reading a page names the page", b.said.includes("העמוד"));
  check("so the same error gives two sentences", a.said === b.said, false);

  // The four families that carry a reader-actionable distinction.
  ok("not running says so plainly", trouble("ksav is not running", "reach_ksav").said.includes("אינו פועל"));
  ok(
    "unreachable says it may have closed badly",
    trouble("could not reach ksav: connection timed out", "reach_ksav").said.includes("נסגר שלא כשורה"),
  );
  ok("refused says it was refused", trouble("ksav refused it: 413", "send_to_ksav").said.includes("נדחתה"));
  ok(
    "a missing file says the file is missing",
    trouble("The system cannot find the file specified. (os error 2)", "open_pdf").said.includes("אינו נמצא"),
  );
  ok(
    "no permission says there is no permission",
    trouble("Access is denied. (os error 5)", "open_pdf").said.includes("הרשאה"),
  );

  // An unrecognised error is a worse message, never a missing one.
  const unknown = trouble("", "read_page");
  ok("an unrecognised error still names what failed", unknown.said.includes("העמוד"));
  ok("and still points at the one place to look", unknown.said.includes("הצבה"));

  // Whatever a `catch` can hold.
  check("a string", raw("plain"), "plain");
  check("an Error", raw(new Error("boom")), "boom");
  check("a Tauri rejection object", raw({ message: "invoke failed" }), "invoke failed");
  check("undefined", raw(undefined), "undefined");
  ok("and none of them throw", trouble(undefined).said.length > 0);
}
