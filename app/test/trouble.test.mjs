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

  // ── the refusals this codebase makes on purpose ───────────────────────────
  //
  // Matched by the **name** Rust puts on them, not by the words. These fourteen
  // used to be regexes against Rust's English `Display` prose, which made every
  // error string in the repository load-bearing API with no test on the Rust
  // side — reword `"there is no index here"` and both halves stayed green while
  // the reader stopped being told what to run.

  ok(
    "a coded refusal is read by its code",
    trouble("no-index: there is no index here", "search").said.includes("girsa-index build"),
  );

  // The whole point: the prose is free to change.
  ok(
    "and rewording the prose changes nothing a reader sees",
    trouble("no-index: no index has been built for this corpus", "search").said.includes(
      "girsa-index build",
    ),
  );

  check(
    "the same sentence, whatever was being done",
    trouble("no-sefer: no sefer here called bavli/berakhot", "read_page").said,
    trouble("no-sefer: no sefer here called anything at all", "search").said,
  );

  ok(
    "a poisoned lock says to reopen the window",
    trouble("poisoned: state is poisoned", "search").said.includes("מחדש"),
  );

  ok(
    "a shelf dragged inside itself says so",
    trouble("cycle: a shelf cannot be put inside itself", "general").said.includes("לתוך עצמו"),
  );

  // A colon in somebody else's message is not a code.
  ok(
    "an operating-system message that happens to have a colon is not read as a code",
    trouble("open failed: The system cannot find the file specified. (os error 2)", "open_pdf")
      .said.includes("אינו נמצא"),
  );

  // The detail keeps the whole string, code and all — it goes on `title`, for
  // whoever is reading a log.
  ok(
    "the raw string survives with its code on it",
    trouble("no-shelf: there is no shelf here").detail.startsWith("no-shelf: "),
  );

}
