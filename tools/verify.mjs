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
// order the rule already gives.
//
// `--from <n>` picks up at step n, which is for the one honest use: the fourth
// step failed, you fixed it, and rebuilding the world to get back there is three
// minutes you do not have to spend. There is deliberately no `--skip`.
//
// # Two lanes
//
// Steps 1–6 all invoke `cargo` against one workspace, one lockfile and one
// `target/`. They share the cargo lock and they **must** stay in the order
// below.
//
// Steps 7–9 share nothing with them: `tsc`, `node test/run.mjs` and
// `node tools/eyes.mjs` are TypeScript and esbuild, they never read `target/`,
// they never take the cargo lock, and they do not need `app/dist`. Run after
// the cargo lane they were pure addition to the wall clock; run beside it they
// are free, because on a warm cache the cargo lane dominates by minutes.
//
// So the gate is two lanes that start together and are joined at the end, and
// **neither short-circuits the other**. That is the one behaviour change worth
// stating: a failing `cargo test` used to mean you never found out that `tsc`
// was also red. Both lanes run to their own first red and both reds are
// reported, so one pass through the gate tells you everything that is wrong.
//
// The cargo lane keeps the terminal — it is the long one and it is the one you
// watch. The window lane is captured and printed when the run joins, because
// two live streams into one terminal is a transcript nobody can read.

import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");

/**
 * The gate, in order, and this list is the only place it is written down.
 *
 * `lane` says which of the two a step belongs to and is the whole of what
 * makes them concurrent — `cargo` for everything that takes the cargo lock,
 * `window` for everything that does not. The numbering the reader sees, and
 * the numbering `--from` takes, is this array's and does not change.
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
  { lane: "cargo", at: ".", say: "build", run: ["cargo", "build", "--all-targets"] },
  { lane: "cargo", at: ".", say: "test", run: ["cargo", "test"] },
  {
    lane: "cargo",
    at: ".",
    say: "clippy",
    run: ["cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"],
  },
  { lane: "cargo", at: ".", say: "fmt", run: ["cargo", "fmt", "--all", "--", "--check"] },
  // The shell. `default-members` excludes it because it cannot build before
  // `app/dist` exists, so the four above compile everything in this repository
  // except the lines that own all the interop.
  {
    lane: "cargo",
    at: "app/src-tauri",
    say: "shell clippy",
    run: ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
  },
  { lane: "cargo", at: "app/src-tauri", say: "shell fmt", run: ["cargo", "fmt", "--", "--check"] },
  // The window.
  { lane: "window", at: "app", say: "types", run: ["npx", "tsc", "--noEmit"] },
  { lane: "window", at: "app", say: "window tests", run: ["node", "test/run.mjs"] },
  // And the one thing in here that has ever seen a pixel. It exits 0 with no
  // browser installed and says so, which is why it can be in the gate at all.
  { lane: "window", at: "app", say: "eyes", run: ["node", "tools/eyes.mjs"] },
];

/** The banner one step prints above itself. */
function heading(step, i) {
  const label = `${i + 1}/${GATE.length} ${step.say}`;
  return (
    `\n── ${label} ${"─".repeat(Math.max(0, 60 - label.length))}\n` +
    `   ${step.at === "." ? "" : `${step.at}$ `}${step.run.join(" ")}\n\n`
  );
}

/**
 * Run one step.
 *
 * `live` gives the step the terminal; otherwise its output is collected and
 * handed back, for a caller that will print it when there is nobody else
 * writing to the screen.
 */
function step(one, i, live) {
  return new Promise((settle) => {
    if (live) process.stdout.write(heading(one, i));
    const [command, ...args] = one.run;
    const child = spawn(command, args, {
      cwd: path.join(ROOT, one.at),
      stdio: live ? "inherit" : ["ignore", "pipe", "pipe"],
      // `npx` is `npx.cmd` on Windows and `spawn` will not find it otherwise.
      // Every argument in `GATE` is a literal written above, so there is
      // nothing here for a shell to interpolate.
      shell: process.platform === "win32",
    });
    let said = "";
    child.stdout?.on("data", (chunk) => {
      said += chunk;
    });
    child.stderr?.on("data", (chunk) => {
      said += chunk;
    });
    // A command that will not start at all — `npx` missing, say — is a red
    // step and not a crashed runner.
    child.on("error", (e) => settle({ ok: false, said: `${said}${e}\n` }));
    child.on("close", (code) => settle({ ok: code === 0, said }));
  });
}

/**
 * One lane, in order, to its own first red.
 *
 * The lane stops there — the steps after it are the same *this is noise if the
 * tree does not build* argument the order was written for. The **other** lane
 * carries on, which is the point of there being two.
 */
async function lane(which, from, live) {
  for (const [i, one] of GATE.entries()) {
    if (one.lane !== which || i + 1 < from) continue;
    const done = await step(one, i, live);
    if (!live) process.stdout.write(heading(one, i) + done.said);
    if (!done.ok) return { at: i, one };
  }
  return null;
}

async function main(argv) {
  const from = Number(argv[argv.indexOf("--from") + 1]) || 1;
  if (argv.includes("--list")) {
    GATE.forEach((one, i) =>
      console.log(`${i + 1}. [${one.lane}] ${one.say} — ${one.run.join(" ")}`),
    );
    return 0;
  }

  const began = Date.now();
  // Started together, joined here. The window lane is captured rather than
  // live; `lane` prints each of its steps as that step finishes, and by then
  // the cargo lane may be mid-compile — so the two are only ever interleaved
  // at step boundaries, never mid-line.
  const [cargo, window] = await Promise.all([
    lane("cargo", from, true),
    lane("window", from, false),
  ]);

  const red = [cargo, window].filter(Boolean);
  const took = Math.round((Date.now() - began) / 1000);
  if (red.length > 0) {
    // Every failure, not the first one. Two independent lanes have two
    // independent answers and reporting one of them sends the reader back
    // through the whole gate to find the other.
    for (const { at, one } of red) {
      console.log(
        `\n✗ ${one.say} — ${one.run.join(" ")}` +
          `\n  in ${one.at}` +
          `\n  fix it and pick up here: node tools/verify.mjs --from ${at + 1}`,
      );
    }
    // The earliest red, because that is the one whose `--from` re-runs the
    // rest of what has to be re-run.
    const first = Math.min(...red.map(({ at }) => at)) + 1;
    if (red.length > 1) {
      console.log(`\n  both lanes are red — from the top of them: --from ${first}`);
    }
    console.log(`\n  ${took}s\n`);
    return 1;
  }
  const ran = GATE.length - from + 1;
  console.log(`\n✓ the gate is green — ${ran} of ${GATE.length} in ${took}s\n`);
  return 0;
}

process.exit(await main(process.argv.slice(2)));
