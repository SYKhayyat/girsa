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
//
// # An interrupted run does not leave a guard broken (#78)
//
// Every mutation is a surgical break of an invariant, so a tree left holding
// one is a tree whose tests are lying — and the mutation in flight is by
// construction the one only a single test can see. A `finally` block covers a
// throw and not a signal: `SIGINT` does not unwind, `SIGKILL` cannot be caught
// at all, and this run takes minutes with the terminal closed over it.
//
// So three layers, and only the first is load-bearing:
//
// 1. **A ledger on disk.** Before the first write of a mutation, the original
//    bytes go to `tools/.mutation-in-flight/` with a `state.json` naming the
//    files. It is deleted when the tree is clean again. A file that exists is
//    unambiguous evidence of an interrupted run, and it survives `SIGKILL`,
//    which no handler does.
// 2. **A sweep at startup.** If the ledger is there, the originals are put back
//    *before* anything else happens, and the run says which file it found and
//    restored. `node tools/mutation.mjs --restore` does only that.
// 3. **Handlers for `SIGINT`/`SIGTERM`**, which restore synchronously and say
//    what they stopped at. Belt to the ledger's braces, not a substitute.
//
// The ledger is listed in `.gitignore`; a run that ends normally removes it.

import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
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

/**
 * Where an interrupted run leaves what it broke.
 *
 * A directory and not one file, because the original bytes of a mutated file can
 * be tens of kilobytes and a JSON of them is a JSON nobody can read at two in
 * the morning. Gitignored: it is debris, and every run that gets to the end
 * deletes it.
 */
const LEDGER = path.join(HERE, ".mutation-in-flight");
const STATE = path.join(LEDGER, "state.json");

/**
 * The same ledger, anywhere else.
 *
 * `--self-test` runs this file's own restore path against a scratch directory,
 * because the code under test is the code that runs when the tree is already
 * broken — and a self-test that used the real `tools/` would be a test that
 * breaks the repository when it fails. Everything below is parameterised by the
 * two paths instead of closing over `HERE`, and the run uses the real ones.
 */
const ledgerAt = (here) => path.join(here, ".mutation-in-flight");
const stateAt = (here) => path.join(ledgerAt(here), "state.json");

/** The file's name inside the ledger — its path, with the separators flattened. */
const heldIn = (here, source) => path.join(ledgerAt(here), source.split(/[\\/]/).join("%"));
const held = (source) => heldIn(HERE, source);

/** Write the originals down before breaking anything. */
function record(sources, here = HERE) {
  mkdirSync(ledgerAt(here), { recursive: true });
  for (const [source, body] of sources) writeFileSync(heldIn(here, source), body, "utf8");
  writeFileSync(stateAt(here), JSON.stringify([...sources.keys()], null, 2) + "\n", "utf8");
}

/** Take one file back out of the ledger, and drop the ledger with the last one. */
function unrecord(source, here = HERE) {
  rmSync(heldIn(here, source), { force: true });
  const state = stateAt(here);
  const left = existsSync(state)
    ? JSON.parse(readFileSync(state, "utf8")).filter((one) => one !== source)
    : [];
  if (left.length === 0) rmSync(ledgerAt(here), { recursive: true, force: true });
  else writeFileSync(state, JSON.stringify(left, null, 2) + "\n", "utf8");
}

/**
 * Put back whatever an interrupted run left broken, and say so.
 *
 * Called before anything else, and this is the layer that survives `SIGKILL`: a
 * handler cannot run there, and a file on disk does not need to.
 */
function sweep(here = HERE, root = ROOT) {
  const state = stateAt(here);
  if (!existsSync(state)) return 0;
  const sources = JSON.parse(readFileSync(state, "utf8"));
  for (const source of sources) {
    const backup = heldIn(here, source);
    if (!existsSync(backup)) continue;
    writeFileSync(path.join(root, source), readFileSync(backup, "utf8"), "utf8");
    console.log(
      "found a mutation left in flight by an interrupted run: restored " + source + "\n",
    );
  }
  rmSync(ledgerAt(here), { recursive: true, force: true });
  return sources.length;
}

/**
 * The ledger's own test: break a guard in a scratch tree, sweep it, demand it
 * come back — and then the cases that are easy to get wrong.
 *
 * It is a step of the gate because the code it covers only runs when something
 * has already gone wrong, which is the code least likely to be exercised by
 * hand. Deliberately not the real `tools/`: a restore that is broken must fail
 * this test rather than this checkout.
 */
function selfTest() {
  const here = mkdtempSync(path.join(tmpdir(), "mutation-selftest-"));
  const guard = "the guard";
  const said = [];
  const say = console.log;
  console.log = (...parts) => said.push(parts.join(" "));
  const fails = [];
  const want = (about, held) => {
    if (!held) fails.push(about);
  };
  try {
    // 1. A run that dies mid-mutation leaves the guard broken and the ledger
    //    holding it; the next sweep puts it back.
    mkdirSync(path.join(here, "crate", "src"), { recursive: true });
    const source = path.join(here, "crate", "src", "guard.rs");
    writeFileSync(source, guard, "utf8");
    record(new Map([["crate/src/guard.rs", guard]]), here);
    writeFileSync(source, "broken", "utf8");
    want("a ledger exists between mutations", existsSync(stateAt(here)));
    want("the guard really was broken", readFileSync(source, "utf8") === "broken");
    const put = sweep(here, here);
    want("the sweep restores one file", put === 1);
    want("the guard is back", readFileSync(source, "utf8") === guard);
    want("the ledger is gone after the sweep", !existsSync(ledgerAt(here)));

    // 2. A clean restore empties the ledger, and the last entry takes the
    //    directory with it.
    record(new Map([["crate/src/guard.rs", guard]]), here);
    unrecord("crate/src/guard.rs", here);
    want("a clean restore leaves no ledger", !existsSync(ledgerAt(here)));

    // 3. A sweep with nothing recorded is a no-op and not a throw.
    want("a sweep with no ledger restores nothing", sweep(here, here) === 0);

    // 4. A ledger whose backup went missing must not throw or claim a restore:
    //    a tool that dies inside its own recovery leaves a person with nothing.
    mkdirSync(ledgerAt(here), { recursive: true });
    writeFileSync(stateAt(here), JSON.stringify(["crate/src/guard.rs"]), "utf8");
    want("a missing backup is not fatal", sweep(here, here) === 1);
    want("a missing backup does not rewrite the file", readFileSync(source, "utf8") === guard);
  } finally {
    console.log = say;
    rmSync(here, { recursive: true, force: true });
  }
  if (fails.length > 0) {
    console.log("✗ the ledger would not hold a guard in place:");
    for (const about of fails) console.log("   " + about);
    process.exit(1);
  }
  console.log("✓ the ledger restores a guard an interrupted run left broken");
}

if (process.argv.includes("--self-test")) {
  selfTest();
  process.exit(0);
}

if (process.argv.includes("--restore")) {
  const put = sweep();
  console.log(
    put === 0
      ? "nothing in flight: the tree is as the last run left it"
      : put + " file(s) restored - the guards are whole again",
  );
  process.exit(0);
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

// Before a single guard is broken: anything a previous run left broken is put
// back, and said out loud. Skipping this is how a tree holds a mutation for a
// day.
sweep();

let escaped = 0;
let unrestored = 0;

/**
 * The mutation on the disk right now, if any — what a signal has to put back.
 *
 * A `finally` block does not run on a signal, so the thing the handler needs
 * cannot live in that block's scope.
 */
let inFlight = null;

/**
 * Stop, restore, and say what was stopped.
 *
 * Synchronous on purpose: a handler that awaits is a handler the process may
 * exit from underneath. `SIGKILL` is not here because nothing can be, which is
 * what the ledger is for.
 */
function stop(signal) {
  if (inFlight) {
    writeFileSync(inFlight.source, inFlight.original, "utf8");
    unrecord(inFlight.path);
    console.log(
      `\n${signal}: stopped mid-mutation — restored ${inFlight.path}` +
        `\n${inFlight.done} of ${mutations.length} mutations had been run.\n`,
    );
    inFlight = null;
  } else {
    console.log(`\n${signal}: stopped between mutations, nothing to restore.\n`);
  }
  process.exit(signal === "SIGINT" ? 130 : 143);
}

process.on("SIGINT", () => stop("SIGINT"));
process.on("SIGTERM", () => stop("SIGTERM"));

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

  record(new Map([[m.source, original]]));
  inFlight = { source, path: m.source, original, done: mutations.indexOf(m) };
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
      // And the tree has to be the tree: a restore that wrote something else
      // would leave the ledger clean over a broken guard.
      const after = await readFile(source, "utf8");
      if (after !== original) {
        console.log(`✗ ${m.source} did not come back byte for byte — fix it by hand`);
        unrestored += 1;
      } else {
        unrecord(m.source);
      }
    } catch {
      console.log(`✗ could not restore ${m.source} — fix it by hand`);
      unrestored += 1;
    }
    inFlight = null;
  }
}

console.log(
  escaped === 0 && unrestored === 0
    ? "✓ every mutation was caught by the test that names it"
    : `✗ ${escaped} mutation(s) escaped and ${unrestored} file(s) not restored`,
);
process.exit(escaped > 0 || unrestored > 0 ? 1 : 0);
