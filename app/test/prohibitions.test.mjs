// The classes this repository has already named, as executable prohibitions.
//
// # Why this file exists
//
// The 9 August three-repository report's finding is not a list of bugs. It is a
// habit, and it counted eighteen instances of it:
//
//   > the diagnosis is written down correctly and the sweep never runs
//
// A class named in prose, one member fixed, the siblings left standing. The
// prose here is not vague — `trouble.ts:190` says *"**Every** `textContent =
// String(e)` in this application goes through here"*, and sixteen sites did
// not. `the_rules_this_repository_wrote_down.rs:912` says *"a second markup
// writer here would pass a `contains` for years and produce documents that
// differ"*, and the second writer is in the other repository, guarded by *its
// own* one-producer sweep, each blind to the other's tree.
//
// Ksav invented the instrument — a prohibition sweep, in `runner.test.mjs` —
// and scoped it to two directories of one application. This is that instrument,
// repo-wide, in every language, and the rule for the future is the second half
// of it: **when a finding names a class, the commit adds the sweep.**
//
// # How a prohibition is written here
//
// Each one is a *class*, not an instance, and each carries the finding that
// produced it. Comments are stripped before matching, because every paragraph
// below that explains what the old arrangement was would otherwise trip the
// test that forbids it — and an exemption is always a **claim with a test
// attached**, never a name on a skip list: an exempt file that stops containing
// the thing it is exempt for turns this red too.

import { check, ok } from "./harness.mjs";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..", "..");

/** Directories that hold other people's code, or output. */
const SKIP = new Set([
  "target",
  "node_modules",
  ".git",
  "dist",
  ".tmp-test",
  "corpus",
  "index",
  "personal",
  // The record. `lamdan/` and `docs/` describe the defects at length and quote
  // the code that had them; a prohibition that forbade *naming* a bug would
  // forbid recording it.
  "lamdan",
]);

/** Every source file in the repository, with comments stripped. */
function sources() {
  const out = [];
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      if (SKIP.has(name)) continue;
      const full = path.join(dir, name);
      if (statSync(full).isDirectory()) {
        walk(full);
        continue;
      }
      if (!/\.(ts|mjs|js|rs)$/u.test(name)) continue;
      // This file states each forbidden pattern as a literal in order to look
      // for it, which is the one exemption every prohibition sweep needs and
      // the only one any of them has.
      if (name === "prohibitions.test.mjs") continue;
      out.push([path.relative(ROOT, full).replace(/\\/gu, "/"), strip(readFileSync(full, "utf8"))]);
    }
  };
  walk(ROOT);
  return out;
}

/**
 * Comments out.
 *
 * A block comment must **begin its own line**. A greedy-from-anywhere strip
 * swallows everything between a `/*` that happens to sit inside a string and
 * the next `*​/` — which in the sibling repository silently deleted three
 * hundred lines of the file being swept, including the one instance the sweep
 * existed to find. A sweep that deletes the region it is sweeping reports
 * green.
 */
function strip(s) {
  return s
    .replace(/^[ \t]*\/\*[\s\S]*?\*\//gmu, "")
    .replace(/^\s*(\/\/|#).*$/gmu, "")
    .replace(/\s(\/\/|#)\s.*$/gmu, "");
}

const RULES = [
  {
    // §1 #16 — the class, stated at `trouble.ts:190`: **every `textContent =
    // String(e)` in this application goes through here.** Sixteen did not —
    // nine in `main.ts` and seven in `laneview.ts`, the latter routing a raw
    // string into a private method so the guard in `sources.test.mjs`, which
    // requires the `String(e)` and the assignment in one expression, could not
    // see them. They were in different functions.
    //
    // `main.ts:1214` was the failure path for *send to Ksav*, printing
    // `PostError`'s English into a Hebrew UI: the original bug that
    // `presence.ts` and `trouble.ts` both cite as their reason for existing.
    what: "no caught error reaches the window as its own string",
    where: /^app\/src\/.*\.ts$/u,
    contains: ["String(e)", "String(err)", "`${e}`"],
    allow: [
      // `raw()` is the one function whose job is to turn a caught value into
      // the developer's string, and it puts it on `title` and never in a
      // sentence.
      "app/src/trouble.ts",
    ],
  },
  {
    // §1 #9 — a prohibition Ksav wrote by name, in the file whose own header
    // says it has *"the same shape as `Ksav/ksav/app/test/run.mjs`, for the
    // same reason it has that shape"*. `.pathname` on a `file://` URL is still
    // percent-encoded, so a checkout under `C:\Users\Some One\` resolves to
    // `Some%20One` and the suite dies at import time.
    what: "nothing hand-rolls a path from import.meta.url",
    where: /\.(ts|mjs|js)$/u,
    contains: ["import.meta.url).pathname"],
    allow: [],
  },
  {
    // §1 #14 — `כסב` is kaf-samekh-bet, a letter-by-letter transliteration of
    // the Latin "Ksav" back into Hebrew. It is not a word; the application is
    // `כְּתָב`. `sources.test.mjs` already forbids it in `app/src`; this is the
    // same rule over the whole tree, because the six original sites were not
    // all in one directory and the seventh will not be either.
    what: "nothing spells the sibling application כסב",
    where: /\.(ts|mjs|rs)$/u,
    contains: ["כסב"],
    allow: [
      // The three that state the misspelling in order to forbid or explain it —
      // the same exemption this file takes for itself. `names.test.mjs` is the
      // one that argues what the word is; `sources.test.mjs` carries the
      // original `app/src` sweep, which stays because a directory-scoped sweep
      // says something a tree-wide one does not; `presence.test.mjs` records
      // the exact line that was on the first screen and asserts it is *not*
      // what a reader sees.
      "app/test/names.test.mjs",
      "app/test/sources.test.mjs",
      "app/test/presence.test.mjs",
    ],
  },
  {
    // §1 #15 — the class: **keying on another crate's English `Display`.**
    // `girsa_post::PostError` is the one error type that crosses the seam, and
    // both frontends matched its prose with four character-identical regexes.
    // It has `code()` now; the words after the colon are not API.
    what: "nothing matches girsa-post's English prose",
    where: /^app\/(src|test)\/.*\.(ts|mjs)$/u,
    contains: ["refused it", "is not running", "could not reach"],
    allow: [
      // The test corpus: real strings, as the crate produces them.
      "app/test/trouble.test.mjs",
      // A record of the exact line that was on the first screen, asserted to be
      // *absent* from the sentence — which is the opposite of keying on it.
      "app/test/presence.test.mjs",
      // The word table. `codePostNotRunning`'s English column reads
      // `{ksav} is not running`, which collides with `PostError`'s prose
      // because *not running* is what English calls a process that is not
      // running — and for no other reason. This file is the far end of the
      // fix: the refusal arrives as the **name** `post-not-running` and is
      // rendered here, which is the opposite of matching prose. There is no
      // regular expression in it and nothing here reads an error at all.
      "app/src/say.ts",
    ],
  },
  {
    // dup §1.2 / §1 #12 — the class, stated in
    // `the_rules_this_repository_wrote_down.rs:912`: **the writer both
    // applications compile is `girsa_ksav::to_ksav`; a second one here would
    // pass a `contains` for years and produce documents that differ.** That
    // sweep covers `*/src/*` in this repository. This one covers the rest of
    // the tree, including the examples and fixtures — `girsa-desk`'s test
    // fixture built `#מקור:("…")[]`, which is **not a Ksav command** and which
    // Typst cannot compile, and six green tests ran over it because `cited_in`
    // scans for the literal substring `מקור:`.
    what: "nothing but girsa-ksav writes Ksav markup",
    where: /\.rs$/u,
    // **Building** it, not asserting on it. `buffer.rs` and `refreshing.rs`
    // check that the writer's output contains `#ציטוט[` and
    // `#מראה_מקום(מקור:`, which is the right thing for a test to do and the
    // opposite of a second writer.
    //
    // What is forbidden is a `format!` or a literal that *constructs* a
    // command — including `#מקור:(`, which is not a Ksav command at all:
    // `מקור:` is a named argument of `#מראה_מקום`, and `girsa-desk`'s document
    // fixture wrote something Ksav cannot emit and Typst cannot compile. Six
    // green tests ran over it, because `cited_in` scans for the literal
    // substring `מקור:` and found one — in the crate whose thesis is *no
    // second markup writer*.
    match: /format!\("#[֐-׿]|"#מקור:\(|"#ציטוט\[\{/u,
    allow: [],
  },
  {
    // §6 — the class: **a caller that wants where the words are does not want
    // the words.** `girsa_hebrew::tokenize` allocates a `String` per word and
    // hands back a `Vec` of them; four callers in this crate filtered that
    // `Vec` down to `(start, end)` pairs and dropped every string. A hit can be
    // an oversized segment — the largest in the corpus is 1,275,307 characters
    // — and marking runs once per row of a result page, per keystroke.
    //
    // `girsa_hebrew::for_each_token` walks the same words out of one reused
    // buffer. The rule is scoped to `girsa-search`, where the volume is, and the
    // exemption is the one caller that genuinely needs the strings.
    what: "girsa-search marks spans by walking, not by collecting words",
    where: /^crates\/girsa-search\/src\/.*\.rs$/u,
    contains: ["girsa_hebrew::tokenize("],
    allow: [
      // Tantivy's `Token` owns a `String`. This is the one place the words
      // themselves are the product — they go into the index.
      "crates/girsa-search/src/tokenizer.rs",
    ],
  },
];

/** Does this file break the rule? */
function breaks(rule, body) {
  if (rule.match) return rule.match.test(body);
  return rule.contains.some((fragment) => body.includes(fragment));
}

export async function run() {
  const files = sources();
  ok("the sweep found the repository", files.length > 100, `${files.length} files`);
  for (const ext of ["ts", "mjs", "rs"]) {
    ok(
      `…including its ${ext} files`,
      files.some(([f]) => f.endsWith(`.${ext}`)),
    );
  }

  for (const rule of RULES) {
    const looked = files.filter(([f]) => rule.where.test(f));
    ok(`${rule.what}: the sweep reached some files`, looked.length > 0, `${looked.length}`);

    const guilty = looked
      .filter(([f]) => !rule.allow.includes(f))
      .filter(([, s]) => breaks(rule, s))
      .map(([f]) => f);
    check(rule.what, guilty, []);

    // An exemption is a claim with a test attached. A file listed as the owner
    // of a rule and no longer containing what it owns is either a moved
    // authority nobody updated here, or a rule that has quietly stopped
    // matching anything at all — and the second is how a green sweep comes to
    // guard nothing.
    for (const owner of rule.allow) {
      const found = looked.find(([f]) => f === owner);
      ok(`…and ${owner} is in the sweep`, !!found);
      if (found) ok(`…and still owns "${rule.what}"`, breaks(rule, found[1]), owner);
    }
  }
}
