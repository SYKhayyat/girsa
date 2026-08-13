// The gate, as one command.
//
//     node tools/verify.mjs
//
// # Why this exists
//
// BUILDER.md rule 4 said *the four verify commands*, and then grew a fifth and a
// sixth for the Tauri shell — which `default-members` excludes — and then two
// more for the window, and then `npm run eyes`. Nine commands in three
// directories, each with its own flags, half of them below the fold of the rule
// that lists them.
//
// What happens to a nine-command gate is what happened here: on 13 August
// `cargo fmt -- --check` — **the fourth of the four**, named first in the rule,
// listed before any of the others — was found failing on eleven files across
// both trees. Some of them had been unformatted for weeks. Nobody had skipped it
// on purpose; it is simply the one that never fails when you are in a hurry, so
// it is the one that stops being run, and the gate silently became eight
// commands and then seven.
//
// This is the audit's first lesson wearing different clothes. *Nothing in this
// project has eyes* was about guards that read source instead of looking at a
// screen; this is a gate that exists in prose instead of in a program. A command
// listed in a gate that nobody runs is not in the gate.
//
// # What it is not
//
// It is not CI, and it does not try to be. CI checks out Ksav beside this
// repository to catch drift between what Girsa writes and what the pen asserts
// on, and that is not something a developer's machine should be doing on every
// change. This runs what a person is supposed to run before committing, in the
// order the rule already gives, stopping at the first red.
//
// `--from <n>` picks up at step n, which is for the one honest use: the fourth
// step failed, you fixed it, and rebuilding the world to get back there is three
// minutes you do not have to spend. There is deliberately no `--skip`.

import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");

/**
 * The gate, in order, and this list is the only place it is written down.
 *
 * BUILDER.md rule 4 points here rather than repeating it, because a rule and a
 * runner that each list the commands are two lists to keep in step, and the one
 * nobody edits is the one that drifts. `the_rules_this_repository_wrote_down.rs`
 * holds that: rule 4 names this file and spells out no `cargo` line of its own.
 *
 * The order is not arbitrary. Compilation first, because everything after it is
 * noise if the tree does not build; the cheap lint before the slow browser; the
 * shell's two after the workspace's four, because they are the ones that catch
 * what `default-members` hides and running them first would report the
 * interop's problems before the library's.
 */
const GATE = [
  { at: ".", say: "build", run: ["cargo", "build", "--all-targets"] },
  { at: ".", say: "test", run: ["cargo", "test"] },
  {
    at: ".",
    say: "clippy",
    run: ["cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"],
  },
  { at: ".", say: "fmt", run: ["cargo", "fmt", "--all", "--", "--check"] },
  // The shell. `default-members` excludes it because it cannot build before
  // `app/dist` exists, so the four above compile everything in this repository
  // except the lines that own all the interop.
  {
    at: "app/src-tauri",
    say: "shell clippy",
    run: ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
  },
  { at: "app/src-tauri", say: "shell fmt", run: ["cargo", "fmt", "--", "--check"] },
  // The window.
  { at: "app", say: "types", run: ["npx", "tsc", "--noEmit"] },
  { at: "app", say: "window tests", run: ["node", "test/run.mjs"] },
  // And the one thing in here that has ever seen a pixel. It exits 0 with no
  // browser installed and says so, which is why it can be in the gate at all.
  { at: "app", say: "eyes", run: ["node", "tools/eyes.mjs"] },
];

function main(argv) {
  const from = Number(argv[argv.indexOf("--from") + 1]) || 1;
  if (argv.includes("--list")) {
    GATE.forEach((step, i) => console.log(`${i + 1}. ${step.say} — ${step.run.join(" ")}`));
    return 0;
  }

  const began = Date.now();
  for (const [i, step] of GATE.entries()) {
    if (i + 1 < from) continue;
    const label = `${i + 1}/${GATE.length} ${step.say}`;
    console.log(`\n── ${label} ${"─".repeat(Math.max(0, 60 - label.length))}`);
    console.log(`   ${step.at === "." ? "" : `${step.at}$ `}${step.run.join(" ")}\n`);
    const [command, ...args] = step.run;
    const done = spawnSync(command, args, {
      cwd: path.join(ROOT, step.at),
      stdio: "inherit",
      // `npx` is `npx.cmd` on Windows and `spawn` will not find it otherwise.
      // Every argument in `GATE` is a literal written above, so there is
      // nothing here for a shell to interpolate.
      shell: process.platform === "win32",
    });
    if (done.status !== 0) {
      console.log(
        `\n✗ ${step.say} — ${step.run.join(" ")}` +
          `\n  in ${step.at}` +
          `\n  fix it and pick up here: node tools/verify.mjs --from ${i + 1}\n`,
      );
      return 1;
    }
  }
  const took = Math.round((Date.now() - began) / 1000);
  console.log(`\n✓ the gate is green — ${GATE.length - from + 1} of ${GATE.length} in ${took}s\n`);
  return 0;
}

process.exit(main(process.argv.slice(2)));
