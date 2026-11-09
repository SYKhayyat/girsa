// Measure the resolver against real citations, and hold the number.
//
//     node tools/resolver.mjs            # check, and say what was recorded
//     node tools/resolver.mjs --record   # measure and write the numbers down
//
// # Why this exists
//
// BUILDER.md W3 promises *"≥95% exact resolution with every miss classified as
// `Ambiguous` rather than wrong"* over a regression corpus of **real** citations,
// and `measure-resolver` is the program that computes it. Nothing ran it: the
// example was a thing a person ran by hand, over `corpus/` — which is gitignored,
// so CI has never seen it, and the figure has lived in a sentence rather than in
// a number anybody could be wrong about.
//
// Everything the resolver does is a decision, and the three outcomes are not
// equally bad. `Unresolved` is a gap. `Ambiguous` is correct behaviour that costs
// a click. **Wrong is unacceptable** — a wrong ref opens a page that does not
// look wrong, and if it reaches Ksav it is wrong in a printed sefer. A percentage
// is the only instrument that sees that whole distribution at once.
//
// # The two answers, and why neither is a lie
//
// With `corpus/` here it measures, compares against what is written in
// `docs/tools.md`, and fails on a regression. Without it — which is every CI run
// — it prints the recorded numbers and says plainly that it did not measure. A
// green step that looked at nothing would be the vacuous pass this repository's
// gate is built against, so the difference is said out loud rather than implied
// by an exit code of 0 either way.

import { spawn } from "node:child_process";
import { readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");
const DOC = "docs/tools.md";
const LEXICON = "corpus/lexicon.tsv";
const LINKS = "corpus/sefaria/links";

/** The marked claims, in the order they are written. */
const MARKS = [
  ["exact", "resolver-exact", /(\d+\.\d+)%<\/?/],
  ["ambiguous", "resolver-ambiguous", /(\d+\.\d+)%<\/?/],
  ["unresolved", "resolver-unresolved", /(\d+\.\d+)%<\/?/],
  ["citations", "resolver-citations", /(\d+)/],
];

/** `**99.98**<!--=resolver-exact-->` → `99.98`. */
function recorded(text) {
  const out = {};
  for (const [key, mark] of MARKS) {
    const at = text.indexOf(`<!--=${mark}-->`);
    if (at < 0) continue;
    const before = text.slice(Math.max(0, at - 40), at);
    const number = before.match(/([\d.,]+)\**$/);
    out[key] = number ? Number(number[1].replace(/[^\d.]/g, "")) : undefined;
  }
  return out;
}

const exists = async (rel) => {
  try {
    await stat(path.join(ROOT, rel));
    return true;
  } catch {
    return false;
  }
};

/** Run `measure-resolver` over Sefaria's link CSVs and read its counts. */
async function measure() {
  const output = await new Promise((settle) => {
    const child = spawn(
      "cargo",
      [
        "run",
        "-q",
        "--release",
        "-p",
        "girsa-corpus",
        "--example",
        "measure-resolver",
        "--",
        LEXICON,
        LINKS,
      ],
      { cwd: ROOT },
    );
    let out = "";
    child.stdout.on("data", (d) => {
      out += d;
    });
    child.stderr.on("data", (d) => {
      out += d;
    });
    child.on("close", (code) => settle({ code, out }));
  });
  if (output.code !== 0) {
    process.stdout.write(output.out);
    throw new Error(`measure-resolver exited ${output.code}`);
  }

  // Only the Sefaria slice: these are English canonical citations produced by
  // Sefaria for the whole corpus, which is the half BUILDER.md W3 is about. The
  // Hebrew half needs an Otzaria install, which is not on this machine and is
  // not something a gate may require.
  const sefaria = output.out.split("== ").find((s) => s.startsWith("Sefaria"));
  if (!sefaria) throw new Error("measure-resolver printed no Sefaria report");
  const count = (label) => {
    const found = sefaria.match(new RegExp(`${label}\\s+(\\d+)`));
    if (!found) throw new Error(`measure-resolver printed no ${label}`);
    return Number(found[1]);
  };
  const citations = Number((sefaria.match(/(\d+) citations/) ?? [])[1]);
  if (!citations) throw new Error("measure-resolver printed no citation count");
  // Percentages computed from the counts, not read from the line above them:
  // the example prints one decimal, and 19,997 of 20,000 rounds to *100.0* — a
  // number that says nothing was ambiguous when three things were.
  const share = (n) => (n * 100) / citations;
  const one = (label) => {
    const n = count(label);
    return { count: n, pct: share(n) };
  };
  return {
    citations,
    exact: one("exact"),
    ambiguous: one("ambiguous"),
    unresolved: one("unresolved"),
  };
}

/** Rewrite the marked numbers in `docs/tools.md`. */
async function record(m) {
  const at = path.join(ROOT, DOC);
  let text = await readFile(at, "utf8");
  const put = (mark, value) => {
    const pattern = new RegExp(`(\\*\\*)[\\d.]+(\\*\\*<!--=${mark}-->)`);
    if (!pattern.test(text)) throw new Error(`docs/tools.md has no ` + mark + ` to write`);
    text = text.replace(pattern, `$1${value}$2`);
  };
  put("resolver-exact", m.exact.pct.toFixed(2));
  put("resolver-ambiguous", m.ambiguous.pct.toFixed(2));
  put("resolver-unresolved", m.unresolved.pct.toFixed(2));
  put("resolver-citations", m.citations.toLocaleString("en-US"));
  await writeFile(at, text);
  process.stdout.write(`recorded in ${DOC}\n`);
}

const wrote = process.argv.includes("--record");
const doc = await readFile(path.join(ROOT, DOC), "utf8");
const was = recorded(doc);

if (!(await exists(LEXICON)) || !(await exists(LINKS))) {
  process.stdout.write(
    `not measured this run: ${LEXICON} is not here (it is generated, and gitignored).\n` +
      `last recorded: ${was.exact}% exact, ${was.ambiguous}% ambiguous, ` +
      `${was.unresolved}% unresolved over ${was.citations} real citations.\n` +
      `regenerate with:\n` +
      `  cargo run --release -p girsa-corpus --example build-lexicon -- \\\n` +
      `      corpus/sefaria/schemas corpus/lexicon.tsv\n` +
      `then re-run: node tools/resolver.mjs --record\n`,
  );
  process.exit(0);
}

const now = await measure();
process.stdout.write(
  `${now.citations} real citations: ${now.exact.pct.toFixed(2)}% exact ` +
    `(${now.exact.count}), ${now.ambiguous.pct.toFixed(2)}% ambiguous ` +
    `(${now.ambiguous.count}), ${now.unresolved.pct.toFixed(2)}% unresolved ` +
    `(${now.unresolved.count})\n`,
);

if (wrote) {
  await record(now);
  process.exit(0);
}

let problems = 0;
if (was.exact !== undefined && now.exact.pct < was.exact) {
  problems++;
  process.stdout.write(
    `REGRESSED: exact resolution is ${now.exact.pct.toFixed(2)}%, and ${DOC} claims ` +
      `${was.exact}%. Something in the lexicon or the resolver lost a spelling.\n`,
  );
}
// W3's floor, checked every run rather than only against the record: a record
// can be lowered by a `--record` that was not being watched.
if (now.exact.pct < 95) {
  problems++;
  process.stdout.write(
    `BELOW THE FLOOR: BUILDER.md W3 asks for ≥95% exact, this is ${now.exact.pct.toFixed(2)}%.\n`,
  );
}
if (was.citations !== undefined && now.citations !== was.citations) {
  process.stdout.write(
    `note: the sample size moved (${was.citations} -> ${now.citations}); re-record with ` +
      `\`node tools/resolver.mjs --record\` if that is expected.\n`,
  );
}
if (was.exact !== undefined && now.exact.pct > was.exact + 0.005) {
  process.stdout.write(
    `better than recorded (${was.exact}% -> ${now.exact.pct.toFixed(2)}%): ` +
      `\`node tools/resolver.mjs --record\` to hold it.\n`,
  );
}
process.exit(problems === 0 ? 0 : 1);
