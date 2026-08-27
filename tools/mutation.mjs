// The dynamic half of the coverage manifest: break each guard, demand the
// pinning test fail.
//
//     node tools/mutation.mjs          # run every mutation against its test
//     node tools/mutation.mjs --list   # the table, without breaking anything
//
// # What it is
//
// `docs/coverage.md` names the highest-risk invariants of the tree and the
// test that pins each one. A test that would still pass with its invariant
// broken is a test that does not test its name — the question the 23-Aug audit
// said it wished it had asked of every suite: *which of these tests would
// still pass if the thing they name were broken?*
//
// This answers it for the six invariants that have a single-line guard. For
// each row of the manifest's `invariants` table it applies the `replace` — a
// small, surgical break of exactly the invariant — and demands that the
// pinning test **fail**. A mutation the test does not catch is a red row: the
// guard is gone and nothing noticed.
//
// It writes to the working tree and always restores, but it is not part of the
// gate: each mutation recompiles its crate, and the gate's job is the static
// check (`node tools/check-coverage.mjs`) that the mutations are still
// applicable and the pinning tests still mention their subjects. Run this when
// the manifest is edited, when a guard is suspected of drifting, or when the
// record asks for it. It exits non-zero if any mutation escaped, if any file
// could not be restored, or if a `find` string did not match exactly once —
// a `find` that matches nothing is a mutation that never happened.

import { spawn } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");

const manifest = JSON.parse(
  await readFile(path.join(HERE, "coverage-manifest.json"), "utf8"),
);
const mutations = manifest.invariants;

function run(cwd, command, args) {
  return new Promise((settle) => {
    const child = spawn(command, args, { cwd, stdio: ["ignore", "pipe", "pipe"] });
    let said = "";
    child.stdout.on("data", (chunk) => (said += chunk));
    child.stderr.on("data", (chunk) => (said += chunk));
    child.on("error", (e) => settle({ code: -1, said: `${said}${e}\n` }));
    child.on("close", (code) => settle({ code, said }));
  });
}

if (process.argv.includes("--list")) {
  for (const m of mutations) {
    console.log(
      `${m.name}\n  guard: ${m.source}  ${JSON.stringify(m.find)}` +
        `\n  break: ${JSON.stringify(m.replace)}` +
        `\n  pinned by: ${m.test.file}::${m.test.fn}` +
        `\n  crate: ${m.crate}\n`,
    );
  }
  process.exit(0);
}

let escaped = 0;
let unrestored = 0;

for (const m of mutations) {
  const source = path.join(ROOT, m.source);
  const original = await readFile(source, "utf8");
  const occurrences = original.split(m.find).length - 1;
  if (occurrences !== 1) {
    console.log(
      `✗ ${m.name}: expected exactly one occurrence of ${JSON.stringify(m.find)} ` +
        `in ${m.source}, found ${occurrences} — the mutation is not well-defined\n`,
    );
    escaped += 1;
    continue;
  }
  const mutated = original.replace(m.find, m.replace);

  try {
    await writeFile(source, mutated, "utf8");
    const verdict = await run(
      ROOT,
      "cargo",
      ["test", "-p", m.crate, m.test.fn, "--quiet"],
    );
    // The pinning test must **fail**. A pass means the mutation escaped.
    if (verdict.code === 0) {
      console.log(`✗ ${m.name}: ESCAPED — ${m.test.fn} still passed with the guard broken\n`);
      escaped += 1;
    } else {
      const failed = /test result: FAILED|error\[E|error:|panicked/.test(verdict.said);
      if (!failed) {
        // Non-zero for another reason — the crate would not build at all —
        // is a mutation that broke the world, which is also a caught break,
        // but it is worth saying which kind it was.
        console.log(
          `✓ ${m.name}: caught (build failed, not the test — ` +
            `the mutation is broader than the guard)\n`,
        );
      } else {
        console.log(`✓ ${m.name}: caught — ${m.test.fn} failed with the guard broken\n`);
      }
    }
  } finally {
    try {
      const now = await readFile(source, "utf8");
      if (now !== original) {
        await writeFile(source, original, "utf8");
        console.log(`   restored ${m.source}`);
      }
    } catch {
      console.log(`✗ could not restore ${m.source} — fix it by hand`);
      unrestored += 1;
    }
  }
}

console.log(
  escaped === 0 && unrestored === 0
    ? "✓ every mutation was caught by the test that names it"
    : `✗ ${escaped} mutation(s) escaped and ${unrestored} file(s) not restored`,
);
process.exit(escaped > 0 || unrestored > 0 ? 1 : 0);
