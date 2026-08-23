#!/usr/bin/env node
// Build a whole shelf from a fresh clone, in one command.
//
//     node tools/build-a-shelf.mjs corpus --download-otzaria
//
// Everything Girsa needs to be a library rather than an empty window: Sefaria
// fetched, a `.txt` library downloaded and unpacked, both imported onto
// permanent ids, the link graph built, the caches that read it backwards, and
// which seforim are worth opening beside which, and the search index. Seven
// steps, in the order they have to happen, with the two downloads that were
// previously a paragraph telling you to go and get them.
//
// # Why this exists
//
// The steps were written down in four places and performed by hand. Written
// down is not the same as reproducible: a shelf built by reading a table and
// typing is a shelf whose exact contents nobody can recreate, including the
// person who built it. This is the difference between *here is how* and *here*.
//
// # It stops rather than half-doing
//
// Every step checks whether its output is already there and skips it if so, so
// an interrupted run is resumed by running it again. Any step that fails ends
// the run — a shelf missing its middle is worse than a shelf that is obviously
// not built, and `girsa-link-types` in particular fails *silently useful*: skip
// it and every daf simply has no mefarshim, which reads exactly like a sefer
// nobody wrote on.
//
// "Already there" is a claim this file has to be able to keep. For the
// downloads that means **bytes verified against content-length**, with a
// sidecar marker written only once the count agrees — a truncated archive is
// deleted, never cached. For the long steps that call the binaries, whose
// outputs are megabytes of derived files nobody wants to stat one by one, it
// means a `.done` marker under `<corpus>/.shelf-build/`, written after the
// step exits zero. Delete a marker (or the folder) to make a step run again.

import { execFileSync, spawnSync } from "node:child_process";
import {
  createWriteStream,
  existsSync,
  mkdirSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const OTZARIA_ZIP =
  "https://github.com/Sivan22/otzaria-library/releases/download/latest/otzaria_latest.zip";
const OTZARLIB_GIT = "https://github.com/gwngdwl/seforim.git";

/** What OtzarLib's own README says, which anybody taking it should read. */
const OTZARLIB_TERMS = `
  OtzarLib states that parts of its contents are subject to copyright and are
  forbidden for public distribution, copying or commercial use; that the files
  are for private use only; and that uploading them waives nothing. The
  repository carries no licence.

  Girsa does not fetch it by default, does not ship it, and does not
  redistribute it. Passing --otzarlib is you choosing to put it on your own
  machine, and the terms above are between you and whoever wrote them.
`;

const argv = process.argv.slice(2);
/** The flags that take a value, so the value is not read as a word. */
const TAKES_A_VALUE = new Set(["--otzaria", "--index", "--personal", "--libraries"]);
const has = (flag) => argv.includes(flag);
const value = (flag) => {
  const at = argv.indexOf(flag);
  return at >= 0 ? (argv[at + 1] ?? null) : null;
};
const words = argv.filter(
  (a, i) => !a.startsWith("--") && !(i > 0 && TAKES_A_VALUE.has(argv[i - 1])),
);
const dryRun = has("--dry-run");

if (words.length !== 1 || has("--help") || has("-h")) {
  console.log(`usage: node tools/build-a-shelf.mjs <corpus> [options]

  <corpus>              where the shelf is written. 15 GB when it is done.

  --otzaria <path>      an Otzaria library already on disk
  --download-otzaria    fetch and unpack it instead (1.28 GB)
  --otzarlib            also clone OtzarLib and lay it out — read its terms,
                        printed when you pass it
  --libraries <path>    where downloaded libraries are unpacked
                        (default: a libraries/ folder beside <corpus>)
  --index <path>        where the search index goes (default: ./index)
  --personal <path>     your own layer (default: ./personal)
  --skip-search         stop before the search index, which is the long step
  --dry-run             print every command and run none of them

  Steps already done are skipped, so an interrupted run resumes by being run
  again. Nothing is deleted except a library tree this script generated itself.`);
  process.exit(words.length === 1 ? 0 : 2);
}

const corpus = resolve(words[0]);
const index = resolve(value("--index") ?? "index");
// Downloaded libraries land here rather than in whatever directory this was
// run from. A fresh clone is run from the repository root, and 1 GB of
// somebody else's seforim appearing as untracked files inside it is not a
// thing to do to a person.
const librariesDir = resolve(value("--libraries") ?? join(corpus, "..", "libraries"));
const personal = resolve(value("--personal") ?? "personal");
// `fileURLToPath`, never `.pathname`. The latter stays percent-encoded, so a
// checkout under `C:\Users\Some One\` resolves to `Some%20One` and every path
// built from it points at nothing. The window suite forbids the other form by
// name, and caught this line.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

let step = 0;
const say = (message) => console.log(`\n── ${++step} · ${message}`);

/** Run a command, echoing it, and stop the whole run if it fails. */
function run(command, args, options = {}) {
  console.log(`   ${command} ${args.join(" ")}`);
  if (dryRun) return;
  const result = spawnSync(command, args, { stdio: "inherit", ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    console.error(`\n${command} exited ${result.status}. Nothing after this has run.`);
    process.exit(result.status ?? 1);
  }
}

/**
 * A Girsa binary, built in release if it is not there yet.
 *
 * `cargo run` would rebuild-check on every call and print its own noise between
 * steps; building once up front and then calling the executables keeps the
 * output of a two-hour run readable.
 */
const CRATE_OF = {
  "girsa-fetch": "girsa-corpus",
  "girsa-import": "girsa-corpus",
  "girsa-link-import": "girsa-link",
  "girsa-link-types": "girsa-link",
  "girsa-index": "girsa-search",
  "girsa-companions": "girsa-app",
};
function tool(name) {
  const exe = join(root, "target", "release", process.platform === "win32" ? `${name}.exe` : name);
  if (!existsSync(exe) && !dryRun) {
    run("cargo", ["build", "--release", "-p", CRATE_OF[name], "--bin", name], { cwd: root });
  }
  return exe;
}

/** Unpack a zip with whatever this machine has. */
function unzip(archive, into) {
  mkdirSync(into, { recursive: true });
  const candidates = [
    // bsdtar, which is `tar` on Windows 10+ and macOS and does read zips.
    ["tar", ["-xf", archive, "-C", into]],
    ["unzip", ["-q", "-o", archive, "-d", into]],
    [
      "powershell",
      ["-NoProfile", "-Command", `Expand-Archive -LiteralPath '${archive}' -DestinationPath '${into}' -Force`],
    ],
  ];
  for (const [command, args] of candidates) {
    console.log(`   ${command} ${args.join(" ")}`);
    if (dryRun) return;
    const result = spawnSync(command, args, { stdio: "inherit" });
    if (!result.error && result.status === 0) return;
    console.log(`   (${command} could not do it, trying the next)`);
  }
  throw new Error(
    `could not unpack ${archive}. Unpack it by hand and re-run with --otzaria <path>.`,
  );
}

/**
 * Download to a file, saying how far along it is — and **meaning it**.
 *
 * Three things a dropped connection used to defeat at once: the stream error
 * was swallowed by the loop ending early, `content-length` was read for the
 * progress bar and never checked against what arrived, and the truncated file
 * was left in place to be "resumed" from for ever. Now the count is compared
 * when the stream ends, the write is awaited before anything is claimed, and
 * any failure takes the partial file with it. There is no published checksum
 * for these archives to check against; length is the one fact on offer, and
 * not checking it is how 1.28 GB of truncation looked like success.
 */
async function download(url, to) {
  console.log(`   GET ${url}`);
  if (dryRun) return;
  const response = await fetch(url, { redirect: "follow" });
  if (!response.ok) throw new Error(`${url} — ${response.status} ${response.statusText}`);
  const total = Number(response.headers.get("content-length") ?? 0);
  const out = createWriteStream(to);
  let seen = 0;
  let printed = 0;
  try {
    for await (const chunk of response.body) {
      seen += chunk.length;
      if (!out.write(chunk)) {
        // Backpressure: wait for the drain rather than buffering the whole
        // archive in memory while the disk catches up.
        await new Promise((resume) => out.once("drain", resume));
      }
      const percent = total ? Math.floor((seen / total) * 100) : 0;
      if (percent >= printed + 5) {
        printed = percent;
        process.stdout.write(`\r   ${percent}%  ${(seen / 1048576).toFixed(0)} MB`);
      }
    }
    await new Promise((settled, failed) => {
      out.end((error) => (error ? failed(error) : settled()));
    });
    if (total > 0 && seen !== total) {
      throw new Error(
        `${url} — the connection ended at ${(seen / 1048576).toFixed(0)} MB of ` +
          `${(total / 1048576).toFixed(0)} MB; the partial file was deleted`,
      );
    }
  } catch (error) {
    out.destroy();
    rmSync(to, { force: true });
    throw error;
  }
  process.stdout.write("\n");
}

/** Is this path a directory with anything in it? */
const built = (path) => {
  try {
    return statSync(path).isDirectory() && readdirSync(path).length > 0;
  } catch {
    return false;
  }
};

/**
 * The marker that says a long step finished.
 *
 * The header promises that every step skips when it is done; steps whose
 * output is "the corpus now contains an import" have no single file to stat,
 * so they write one. Under the corpus, dot-named, so it is invisible to the
 * shelf and obvious to anybody who goes looking.
 */
const markers = join(corpus, ".shelf-build");
const alreadyDone = (marker) => existsSync(join(markers, `${marker}.done`));
const markDone = (marker) => {
  mkdirSync(markers, { recursive: true });
  writeFileSync(join(markers, `${marker}.done`), `${new Date().toISOString()}\n`);
};

/** Run a step's command unless its marker is down, then mark it. */
function runStep(marker, command, args) {
  if (!dryRun && alreadyDone(marker)) {
    console.log("   already done, skipping");
    return;
  }
  run(command, args);
  if (!dryRun) markDone(marker);
}

async function main() {
  console.log(`Building a shelf in ${corpus}`);
  if (dryRun) console.log("(--dry-run: printing the plan, running nothing)");

  // 1 · Sefaria.
  say("Sefaria — 3.4 GB, resumable");
  if (built(join(corpus, "sefaria", "schemas"))) {
    console.log("   already fetched, skipping");
  } else {
    run(tool("girsa-fetch"), [join(corpus, "sefaria")]);
  }

  // 2 · A `.txt` library.
  say("a .txt library");
  const libraries = [];
  let otzaria = value("--otzaria");
  if (otzaria) {
    otzaria = resolve(otzaria);
    if (!built(join(otzaria, "אוצריא"))) {
      console.error(`   ${otzaria} has no אוצריא/ in it — is that an Otzaria library?`);
      process.exit(1);
    }
    console.log(`   using ${otzaria}`);
  } else if (has("--download-otzaria")) {
    otzaria = join(librariesDir, "otzaria_latest");
    if (built(join(otzaria, "אוצריא"))) {
      console.log(`   ${otzaria} is already unpacked, skipping`);
    } else {
      const zip = join(librariesDir, "otzaria_latest.zip");
      // The marker, not mere existence: a zip that exists may be a truncated
      // one a previous run left behind, and resuming from that died in
      // `unzip` every time with no way out but the hand. Only a verified
      // download writes `.done`; anything else is fetched again.
      const settled = `${zip}.done`;
      mkdirSync(librariesDir, { recursive: true });
      if (!existsSync(settled)) {
        rmSync(zip, { force: true });
        await download(OTZARIA_ZIP, zip);
        // Written only after `download` verified what it got; and never
        // during --dry-run, where there is no archive to measure.
        if (!dryRun) writeFileSync(settled, `${statSync(zip).size}\n`);
      } else {
        console.log("   the archive is already downloaded and verified, skipping");
      }
      unzip(zip, otzaria);
      // Some builds of the archive nest one level. Point at whichever holds it.
      if (!built(join(otzaria, "אוצריא"))) {
        const inner = join(otzaria, basename(otzaria));
        if (built(join(inner, "אוצריא"))) otzaria = inner;
      }
    }
  } else {
    console.error(
      "   no library. Pass --otzaria <path> if you have one, or --download-otzaria\n" +
        "   to fetch it. docs/the-libraries.md says where every part of a shelf\n" +
        "   comes from and what its terms are.",
    );
    process.exit(2);
  }
  libraries.push(otzaria);

  // 2b · OtzarLib, opt-in.
  if (has("--otzarlib")) {
    say("OtzarLib — opt-in");
    console.log(OTZARLIB_TERMS);
    const clone = join(librariesDir, "otzarlib");
    const shelf = join(librariesDir, "otzarlib-shelf");
    if (!dryRun) mkdirSync(librariesDir, { recursive: true });
    if (!built(clone)) run("git", ["clone", "--depth", "1", OTZARLIB_GIT, clone]);
    else console.log(`   ${clone} is already cloned, skipping`);
    if (!dryRun) rmSync(shelf, { recursive: true, force: true });
    run(process.execPath, [join(root, "tools", "lay-out-otzarlib.mjs"), clone, shelf]);
    libraries.push(shelf);
  }

  // Onto permanent ids.
  say("the shelf — permanent segment ids");
  runStep("import", tool("girsa-import"), [corpus, ...libraries]);

  // The links.
  say("the links between them");
  runStep("links", tool("girsa-link-import"), [corpus, ...libraries]);

  // The caches. Skipping this is why a daf has no mefarshim.
  say("the caches that read the links backwards");
  runStep("types", tool("girsa-link-types"), [corpus, personal]);

  // Which seforim are worth opening beside which.
  //
  // **This was not in the six steps, and leaving it out is invisible.** The
  // shelf still opens, every daf still has its links, and the מפרשים list
  // simply offers the *declared* commentaries only — which for a `.txt`
  // library is none of them, because nothing in a `.txt` declares a base text.
  // The Encyclopedia Talmudit's own footnote volume, 5,657 edges of it per
  // letter, would not be offered beside the entry it annotates.
  //
  // `docs/tools.md` calls this one "worth running, and nothing refuses without
  // it", which is exactly the shape of a step that stops being run.
  say("which seforim are worth opening beside which");
  runStep("companions", tool("girsa-companions"), [corpus]);

  // Search.
  if (has("--skip-search")) {
    say("search — skipped");
    console.log("   --skip-search: nothing is searchable until girsa-index runs");
  } else {
    say("search — about 4 GB, the long one");
    runStep("index", tool("girsa-index"), ["build", index, corpus, personal]);
  }

  console.log(`
Done. Point Girsa at it:

    GIRSA_CORPUS=${corpus}

Girsa looks there first, then beside the executable, then two levels up.`);
}

main().catch((error) => {
  console.error(`\n${error.message}`);
  process.exit(1);
});
