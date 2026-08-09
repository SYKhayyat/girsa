// Build the modules under test, then run every test file.
//
// Same shape as `Ksav/ksav/app/test/run.mjs`, for the same reason it has that
// shape: it builds whatever `MODULES` lists and runs whatever `test/*.test.mjs`
// exists, so **adding a test is adding a file**. A runner that needs
// `package.json` edited per test is a small friction that reliably compounds
// into one test file for fifteen modules — or, here, into none for nineteen.

import { build } from "esbuild";
import { readdir, rm } from "node:fs/promises";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

// `fileURLToPath`, not `new URL(…).pathname`.
//
// A `file://` URL's `pathname` is still **percent-encoded**, so a checkout under
// `C:\Users\Some One\Girsa` resolves to `Some%20One` and every path built from
// it finds nothing — the suite dying at import time with a path nobody can read.
// The hand-rolled `.replace(…)` beside it fixed the leading drive letter, which
// is the *other* half of the same problem and the half that shows up on a
// developer's own machine.
//
// Ksav forbids this by name — `runner.test.mjs`: *"nothing hand-rolls a path
// from import.meta.url"* — and this file's own header says it has *"the same
// shape as `Ksav/ksav/app/test/run.mjs`, for the same reason it has that
// shape"*, while carrying the exact expression that file exists to ban. Neither
// repository's guard could read the other's tree. `prohibitions.test.mjs` is in
// both now.
const HERE = path.dirname(fileURLToPath(import.meta.url));
const APP = path.resolve(HERE, "..");
const OUT = path.join(APP, ".tmp-test");

/**
 * The modules a test may import.
 *
 * Bundled, so their own imports come along. Modules that reach for the Tauri
 * bridge or the DOM at import time cannot go in here as they stand; splitting
 * their decisions out from their drawing is what puts them on this list, which
 * is the pressure this file is meant to apply.
 */
const MODULES = ["dock", "keys", "mefarshim", "names", "panel", "presence", "trouble"];

await rm(OUT, { recursive: true, force: true });

await build({
  entryPoints: MODULES.map((m) => path.join(APP, "src", `${m}.ts`)),
  outdir: OUT,
  outExtension: { ".js": ".mjs" },
  format: "esm",
  bundle: true,
  platform: "neutral",
  // Nothing under test talks to Tauri, and pulling it in would make a unit test
  // of a data module depend on a desktop shell being there.
  external: ["@tauri-apps/*", "pdfjs-dist"],
  logLevel: "warning",
});

const files = (await readdir(HERE)).filter((f) => f.endsWith(".test.mjs")).sort();
if (!files.length) {
  console.log("no test files");
  process.exit(1);
}

// The harness keeps one running tally across every file, so a test that forgets
// to report cannot hide a failure.
const { counts } = await import(pathToFileURL(path.join(HERE, "harness.mjs")).href);

for (const f of files) {
  const mod = await import(pathToFileURL(path.join(HERE, f)).href);
  if (typeof mod.run !== "function") {
    console.log(`FAIL ${f} exports no run()`);
    process.exit(1);
  }
  const before = counts();
  await mod.run();
  const after = counts();
  const failed = after.fail - before.fail;
  console.log(
    `${failed ? "✗" : "✓"} ${f.padEnd(24)} ${after.pass - before.pass} passed` +
      (failed ? `, ${failed} FAILED` : ""),
  );
}

const { pass, fail } = counts();
console.log(`\n${files.length} files · ${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
