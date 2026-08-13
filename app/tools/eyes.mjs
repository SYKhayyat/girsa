// Somebody in this repository has to look at the screen.
//
//     node tools/eyes.mjs
//
// # Why it exists
//
// Every guard in `app/test` reads **source**. `styles.test.mjs` proves the sheet
// defines every property it reads. `prohibitions.test.mjs` proves no module
// hand-rolls a path. `sources.test.mjs` proves every string goes through `say`.
// Two hundred and thirty-eight assertions, and not one of them has ever seen a
// pixel.
//
// So these got through:
//
// * a mefaresh's comment drawn at `opacity: 0`, fixed to the foot of the window,
//   in a container 16px tall — the feature W43 exists for, invisible from the day
//   it was written;
// * a pane title in a flex header with five buttons that do not shrink, measured
//   at **0px** in a three-way split, so a column had no name — and the honest
//   `אין כאן` note clipped to a smudge beside it.
//
// Both are one assertion each in a browser, and neither is expressible in a
// string search. This is that browser.
//
// # What it does not do
//
// It does not run the application. It builds the small pieces of markup the
// window builds, over the real `src/styles.css`, and measures them. That is a
// deliberate limit: a check that needs the shell, the corpus and the IPC bridge
// is a check that does not run, and the two bugs above are both **the
// stylesheet's answer to a shape**, which is exactly what this can ask about.
//
// Headless Edge is used because this is a WebView2 application and Edge is the
// engine the window is already running — the same layout, not an approximation
// of it. `EYES_BROWSER` overrides the path. Where no browser is found this says
// so and exits 0: a developer without one is not a failing build.
//
// `EYES_REQUIRED=1` makes that a failure instead, and CI sets it. The original
// line here said *"CI that wants it enforced can set the variable"* and no CI
// job ever did — so on a machine with no browser this printed one sentence and
// returned success, which is the same shape as a test that passes because it
// could not find its input. BUILDER.md forbids that for tests by name; there is
// no reason it should be allowed for the only check in this repository that has
// ever seen a pixel.

import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { APP } from "./paths.mjs";

/** Where Edge is, on a machine that has it. */
const EDGES = [
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
  "/usr/bin/microsoft-edge",
  "/usr/bin/google-chrome",
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
];

/**
 * `EYES_BROWSER` **instead of** the list, not in front of it.
 *
 * It used to be the first candidate, under a header saying it *overrides* the
 * path — so a variable pointing at a browser that had moved fell through to
 * whatever else happened to be installed, and reported on an engine nobody had
 * asked for. That is the same defect `roots.rs` documents about `GIRSA_CORPUS`
 * pointing at an empty directory: *somebody said so* is not a hint.
 */
async function findBrowser() {
  const { access } = await import("node:fs/promises");
  const said = process.env.EYES_BROWSER;
  for (const candidate of said ? [said] : EDGES) {
    try {
      await access(candidate);
      return candidate;
    } catch {
      // The next one.
    }
  }
  return null;
}

// ------------------------------------------------------------------ the page
//
// One document, several specimens. Each is the markup a module builds, named by
// the module that builds it, so a failure points at a file rather than at a
// selector.

/**
 * The comment block `pane.ts:showSaid` builds when a ticked mefaresh has
 * something to say about a line.
 */
const COMMENT = `
<div class="pane"><div class="pane-body">
  <div class="line" data-id="x"><span class="line-text">מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין?</span></div>
  <div class="line-said" id="said-box">
    <div class="said-one" id="said-first">
      <p class="said-who">רש״י על ברכות 2a:8:1</p>
      <p class="said-line">אקרא קאי – ושם למד חובת הקריאה:</p>
    </div>
    <div class="said-one" id="said-second">
      <p class="said-who">תוספות על ברכות 2a:8:1</p>
      <p class="said-line">מאימתי קורין וכו׳ – פי׳ רש״י ואנן היכי קרינן מבעוד יום.</p>
    </div>
  </div>
</div></div>`;

/**
 * The header `pane.ts` builds, in the width a third of a 1360px window leaves
 * it — the case the 9 August sitting measured at zero.
 */
const HEADER = `
<div style="width: 430px" id="narrow-pane">
  <header class="pane-head">
    <span class="pane-title" id="narrow-title">מסכת בבא בתרא עם תוספות</span>
    <span class="pane-where">קכ״ז.</span>
    <span class="pane-note is-empty" id="narrow-note">אין כאן</span>
    <div class="pane-tools" id="narrow-tools">
      <button class="tool">מפרשים · 34</button>
      <button class="tool">גלילה משותפת</button>
      <button class="tool">קישורים</button>
      <button class="tool">ייצא</button>
      <button class="tool" id="narrow-last">סגור</button>
    </div>
  </header>
</div>`;

/**
 * The same header in English, where it used to overflow the other way and clip
 * the leftmost **button** to `se`.
 */
const HEADER_EN = `
<div style="width: 430px" id="english-pane" dir="ltr" lang="en">
  <header class="pane-head">
    <span class="pane-title">Bava Basra with Tosafos</span>
    <span class="pane-where">127a</span>
    <div class="pane-tools">
      <button class="tool" id="english-first">mefarshim · 34</button>
      <button class="tool">scrolling together</button>
      <button class="tool">links</button>
      <button class="tool">export</button>
      <button class="tool">close</button>
    </div>
  </header>
</div>`;

/**
 * The shelf's two-column body, in a box whose width the test drives.
 *
 * `.shelf-tree` is 320px and `--dock` is 380px, four hundred lines apart in the
 * stylesheet, and what that left for the seforim was about seventy. The fix is
 * `@container shelf (width < 640px)`, and a container query is exactly the kind
 * of rule that silently does nothing — misname the container, forget
 * `container-type`, put the query on the wrong element, and the sheet still
 * parses and every selector still matches. `styles.test.mjs` reads the text and
 * can prove the names line up. Only a browser can prove the layout changed.
 *
 * The width goes on the **sheet**, which is the container, and not on a wrapper
 * around it: `.shelf-sheet` carries its own `width: min(1080px, 94vw)` and would
 * have sat at 1278px inside a 380px box, which is a specimen that measures
 * nothing. Where the sheet gets its width from — `is-docked`, the viewport, or
 * this line — is precisely what the query does not care about, and that is the
 * argument the stylesheet makes for asking about width instead of about a class.
 */
const SHELF = `
<div class="shelf-sheet" id="shelf-box" style="height: 300px">
  <div class="shelf-body" id="shelf-body">
    <div class="shelf-tree" id="shelf-tree"><p class="shelf-row">תלמוד בבלי</p></div>
    <div class="shelf-column" id="shelf-column"><p class="shelf-work">ברכות</p></div>
  </div>
</div>`;

const PAGE = (sheet) => `<!doctype html>
<html lang="he" dir="rtl"><head><meta charset="utf-8">
<link rel="stylesheet" href="${sheet}"></head>
<body style="margin:0">${COMMENT}${HEADER}${HEADER_EN}${SHELF}</body></html>`;

// -------------------------------------------------------------------- the eye

let pass = 0;
let fail = 0;

function seen(name, ok, detail) {
  if (ok) {
    pass += 1;
  } else {
    fail += 1;
    console.log(`FAIL ${name}\n  ${detail}`);
  }
}

/** A minimal CDP client — one socket, one page, promises by id. */
class Eye {
  constructor(socket) {
    this.socket = socket;
    this.id = 0;
    this.waiting = new Map();
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      const waiter = this.waiting.get(message.id);
      if (!waiter) return;
      this.waiting.delete(message.id);
      if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
      else waiter.resolve(message.result);
    });
  }

  send(method, params = {}) {
    const id = (this.id += 1);
    return new Promise((resolve, reject) => {
      this.waiting.set(id, { resolve, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
      setTimeout(() => {
        if (this.waiting.delete(id)) reject(new Error(`timeout on ${method}`));
      }, 20_000);
    });
  }

  /** Evaluate in the page and hand back the value. */
  async look(expression) {
    const out = await this.send("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (out.exceptionDetails) {
      throw new Error(out.exceptionDetails.exception?.description ?? "threw");
    }
    return out.result.value;
  }
}

/**
 * A port nobody is using, chosen by the operating system.
 *
 * Bind to 0, ask what we got, let it go, hand the number to the browser. The
 * gap between releasing it and the browser binding it is a real race and a
 * vanishingly small one; the alternative it replaces was a **constant**, which
 * is not a race but a certainty as soon as two copies run.
 *
 * The first attempt at this asked the browser instead — `--remote-debugging-port=0`
 * and read `DevToolsActivePort` out of the profile directory, which is what
 * Chrome documents and what works on the machine this was written on. On a
 * Linux CI runner the file never appeared, and thirty seconds later the run
 * failed saying so. Rather than find out which of headless mode, the sandbox
 * flags and the profile path was responsible, the port is ours to choose: this
 * needs no cooperation from the browser and behaves the same everywhere.
 */
async function freePort() {
  const { createServer } = await import("node:net");
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

/**
 * The browser's page target, once it has one.
 *
 * `said` hands back whatever the browser has written to stderr, so that a
 * failure here names the reason rather than the symptom. Without it this threw
 * *the browser never opened a page* and left the browser's own sentence —
 * `Failed to move to new namespace`, or whatever it was — discarded.
 *
 * # Two waits, and only one of them had been fixed
 *
 * The earlier flake was the **page** race: the first evaluation could land on a
 * document still loading, which `settled` now waits out. This is the **browser**
 * race, and it is a different clock. On the same commit, a CI run went green on
 * `main` and red on the tag ten minutes later — the giveaway that this was a
 * budget and not a broken environment, since nothing about the machine differed.
 * Fifteen seconds is plenty for a warm desktop and not always enough for a cold
 * runner starting Chrome for the first time.
 *
 * Thirty seconds now, and — the part that matters more — it stops waiting the
 * moment the browser **dies**. Polling a port for fifteen seconds after the
 * process has exited is fifteen seconds spent proving something already known,
 * and it reports a timeout for what was a crash.
 *
 * The stderr it prints is raw, and on Linux it is mostly `Failed to connect to
 * the bus` — noise Chrome emits on any machine with no session D-Bus, and not
 * the reason for anything. Filtering it would mean deciding which of somebody
 * else's diagnostics matter, which is the mistake `trouble.ts` is about. It is
 * the browser's own words, printed whole, for a person to read.
 */
async function pageOf(port, said = () => "", alive = () => true) {
  for (let tries = 0; tries < 120; tries += 1) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      const page = list.find((t) => t.type === "page");
      if (page?.webSocketDebuggerUrl) return page;
    } catch {
      // Not up yet.
    }
    if (!alive()) break;
    await new Promise((r) => setTimeout(r, 250));
  }
  const why = said().trim();
  const what = alive() ? "never opened a page" : "started and then exited";
  throw new Error(why ? `the browser ${what}. It said:\n${why}` : `the browser ${what}`);
}

/**
 * Wait until the page is the page — `readyState` complete and the sheet
 * applied.
 *
 * `pageOf` returns as soon as the browser has a page *target*, which is before
 * it has finished navigating to one, so the first `Runtime.evaluate` could land
 * on a document that was still loading. This run went red once with
 * `overflow-y was visible` and green on the retry, which is the worst possible
 * outcome: a flaky guard is one a person learns to re-run rather than read, and
 * the next time it means it nobody will believe it.
 *
 * The guard itself is not what changed. It was right, and it was doing exactly
 * its job — refusing to measure unstyled HTML rather than passing sixteen
 * assertions against it. What it lacked was the patience to let the thing it is
 * asking about happen. A timeout is still a failure and still says what it saw.
 *
 * **Thirty seconds, not four.** The first version gave the browser thirty to
 * start and left the page four to load, which is the same mistake twice in two
 * clocks — and the second one duly went red on a cold runner with `no
 * .pane-body after 4s`. There is no reason for the two budgets to differ; both
 * are *how long a cold machine might take*, and neither costs anything on a warm
 * one, because both return the moment the thing they want is true.
 */
async function settled(eye) {
  for (let tries = 0; tries < 300; tries += 1) {
    const found = await eye.look(`(() => {
      if (document.readyState !== 'complete') return 'loading';
      const body = document.querySelector('.pane-body');
      return body ? getComputedStyle(body).overflowY : 'no .pane-body yet';
    })()`);
    if (found === "auto") return found;
    await new Promise((r) => setTimeout(r, 100));
  }
  // Whatever it last saw, so the failure names the state rather than the wait.
  return eye.look(`(() => {
    const body = document.querySelector('.pane-body');
    return body ? getComputedStyle(body).overflowY : 'no .pane-body after 30s';
  })()`);
}

async function main() {
  const browser = await findBrowser();
  if (!browser) {
    console.log("no browser found — set EYES_BROWSER to one. Nothing looked at.");
    return process.env.EYES_REQUIRED ? 1 : 0;
  }

  const room = await mkdtemp(path.join(tmpdir(), "girsa-eyes-"));
  const sheet = pathToFileURL(path.join(APP, "src", "styles.css")).href;
  const page = path.join(room, "specimens.html");
  await writeFile(page, PAGE(sheet), "utf8");

  // **Port 0: let the browser pick, and ask it which.**
  //
  // This was `9333`, a constant, and two runs at once fought over it — the
  // second either attached to the first one's browser or timed out on a port
  // that was answering somebody else's questions. Found the way these things
  // are found: running `eyes` six times to test the flake fix, while the gate
  // was running its own copy, and watching the gate go red for a reason that
  // had nothing to do with the gate.
  //
  // A fixed port in a test tool is a claim that only one of it will ever run on
  // a machine, and nothing makes that true — not two terminals, not a gate
  // beside a developer, not two CI jobs on one runner. Chrome writes the port
  // it actually took into `DevToolsActivePort` in its profile directory, which
  // is the answer rather than a guess at a free number.
  const profile = path.join(room, "profile");
  const port = await freePort();
  const child = spawn(
    browser,
    [
      "--headless=new",
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${profile}`,
      "--no-first-run",
      "--disable-gpu",
      "--window-size=1360,900",
      "--allow-file-access-from-files",
      // What a CI runner needs, and what a desktop does not mind.
      //
      // The first time this ran on a machine other than the author's it found
      // `/usr/bin/google-chrome`, started it, and got *the browser never opened
      // a page* — because Chrome's own sandbox will not initialise under the
      // runner's user namespaces, and `/dev/shm` there is 64 MB. Both are the
      // standard headless-in-CI pair. The sandbox is worth exactly nothing here
      // anyway: this browser opens one local file written by this script, and
      // then it is killed.
      "--no-sandbox",
      "--disable-dev-shm-usage",
      pathToFileURL(page).href,
    ],
    // **Not `ignore`.** It was, and so the browser's own explanation of why it
    // would not start went to nowhere and this reported a generic *never opened
    // a page* — a tool that cannot say what went wrong, which is the fault this
    // whole file exists to catch on other people's behalf.
    { stdio: ["ignore", "ignore", "pipe"] },
  );
  let complained = "";
  child.stderr?.on("data", (chunk) => {
    complained += String(chunk);
  });

  try {
    const alive = () => child.exitCode === null && child.signalCode === null;
    const target = await pageOf(port, () => complained, alive);
    const socket = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve);
      socket.addEventListener("error", reject);
    });
    const eye = new Eye(socket);
    await eye.send("Runtime.enable");
    await eye.send("Page.enable");

    // The stylesheet has to have arrived, or every measurement below is a
    // measurement of unstyled HTML and all of them pass.
    const styled = await settled(eye);
    seen("the stylesheet loaded", styled === "auto", `overflow-y was ${styled}`);
    // And if it did not, **stop**. Every assertion below reaches into the page
    // by id and hands the result to `getComputedStyle`; with no page they do not
    // fail, they throw `parameter 1 is not of type 'Element'` — a stack trace
    // printed over the top of the one line that had already said what was
    // actually wrong. The report is the product here. Burying it under the
    // consequences of ignoring it is the same fault this file catches for other
    // people, committed by the file itself.
    if (styled !== "auto") {
      console.log(`${pass} seen, ${fail} wrong — stopped: there is no page to measure`);
      return 1;
    }

    // ------------------------------------------ who is holding the reader's place
    //
    // The pane adds and removes lines above the fold and corrects `scrollTop`
    // itself (`pane.ts:holdingPlace`). Scroll anchoring is the browser doing the
    // same job by guessing which element to hold, and two corrections of one
    // shift move the page twice. Asked of the *computed* style rather than of
    // the file, because "the sheet says `none`" and "this element is `none`" are
    // different claims once anything else in 3,200 lines can set it.
    const anchoring = await eye.look(
      `getComputedStyle(document.querySelector('.pane-body')).overflowAnchor`,
    );
    seen(
      "nothing but the pane holds the reader's place",
      anchoring === "none",
      `overflow-anchor computed to ${anchoring} — the browser corrects the scroll too`,
    );

    // ------------------------------------------------------- what a mefaresh said
    const comment = await eye.look(`(() => {
      const box = document.getElementById('said-box');
      const one = document.getElementById('said-first');
      const two = document.getElementById('said-second');
      const cs = getComputedStyle(one);
      return {
        boxHeight: Math.round(box.getBoundingClientRect().height),
        height: Math.round(one.getBoundingClientRect().height),
        width: Math.round(one.getBoundingClientRect().width),
        position: cs.position,
        opacity: cs.opacity,
        pointerEvents: cs.pointerEvents,
        separated: getComputedStyle(two).borderTopWidth,
      };
    })()`);

    seen(
      "a mefaresh's comment is in the flow, not fixed to the window",
      comment.position === "static" || comment.position === "relative",
      `position was ${comment.position} — the toast rule reached it`,
    );
    seen(
      "a mefaresh's comment can be seen",
      Number(comment.opacity) === 1,
      `opacity was ${comment.opacity}`,
    );
    seen(
      "the words can be selected and copied",
      comment.pointerEvents !== "none",
      `pointer-events was ${comment.pointerEvents}`,
    );
    seen(
      "the box around the comments is as tall as the comments",
      comment.boxHeight >= comment.height,
      `box ${comment.boxHeight}px around a ${comment.height}px comment`,
    );
    seen(
      "two mefarshim on one line are told apart",
      comment.separated !== "0px",
      `the second block's border-top was ${comment.separated}`,
    );

    // ------------------------------------------------ a column still has a name
    //
    // The audit measured 42px, **0px** and 6px across a three-way split and left
    // the fix as the owner's call: *what should give way first in that header —
    // the name of the sefer or the fifth button?* The answer is neither. The
    // buttons are one box that wraps to a second row, so the title keeps its
    // width and every button stays where it was. These assert that at three
    // widths, because the answer that is only right at one width is the answer
    // that was there before.

    const header = await eye.look(`(() => {
      const measure = (px) => {
        const pane = document.getElementById('narrow-pane');
        pane.style.width = px + 'px';
        const t = document.getElementById('narrow-title');
        const note = document.getElementById('narrow-note');
        const last = document.getElementById('narrow-last');
        return {
          title: Math.round(t.getBoundingClientRect().width),
          note: Math.round(note.getBoundingClientRect().width),
          noteFits: note.scrollWidth <= note.getBoundingClientRect().width + 1,
          lastButton: Math.round(last.getBoundingClientRect().width),
          lastFits: last.scrollWidth <= last.getBoundingClientRect().width + 1,
        };
      };
      const out = { at430: measure(430), at240: measure(240), at1000: measure(1000) };
      document.getElementById('narrow-pane').style.width = '430px';
      return out;
    })()`);

    for (const [where, seenAt] of Object.entries(header)) {
      // Not *un*truncated — an ellipsis on a long name in a narrow column is the
      // right answer. But a title clipped to nothing is a column whose sefer the
      // reader cannot name, which is what a flex row of five buttons that never
      // shrink produces.
      seen(
        `a column ${where} still shows its name`,
        seenAt.title >= 40,
        `the title measured ${seenAt.title}px wide beside five buttons`,
      );
      seen(
        `the "nothing here" note is readable ${where}`,
        seenAt.noteFits,
        `the note measured ${seenAt.note}px around text that needs more`,
      );
      seen(
        `no button is clipped ${where}`,
        seenAt.lastFits,
        `the last button measured ${seenAt.lastButton}px around its own label`,
      );
    }

    // ------------------------------------------- the shelf when it is narrow
    //
    // Measured, not read. The assertion is about the **seforim**, which is what
    // the panel is for: at 380px — the docked width — the column holding them
    // must be most of the sheet rather than the seventy pixels 320 + 380 leaves.
    const shelf = await eye.look(`(() => {
      const measure = (px) => {
        const box = document.getElementById('shelf-box');
        box.style.width = px + 'px';
        const body = document.getElementById('shelf-body');
        const tree = document.getElementById('shelf-tree');
        const column = document.getElementById('shelf-column');
        return {
          direction: getComputedStyle(body).flexDirection,
          tree: Math.round(tree.getBoundingClientRect().width),
          column: Math.round(column.getBoundingClientRect().width),
        };
      };
      const out = { docked: measure(380), narrow: measure(560), wide: measure(1000) };
      document.getElementById('shelf-box').style.width = '1000px';
      return out;
    })()`);

    seen(
      "a narrow shelf stacks the tree over the seforim",
      shelf.docked.direction === "column" && shelf.narrow.direction === "column",
      `at 380px it was ${shelf.docked.direction}, at 560px ${shelf.narrow.direction} ` +
        `— the container query did not fire`,
    );
    seen(
      "and the seforim get the width",
      shelf.docked.column > shelf.docked.tree * 0.9,
      `the tree took ${shelf.docked.tree}px and left the seforim ${shelf.docked.column}px`,
    );
    seen(
      "a wide shelf still puts them side by side",
      shelf.wide.direction === "row" && shelf.wide.tree > 200,
      `at 1000px it was ${shelf.wide.direction} with a ${shelf.wide.tree}px tree`,
    );

    // The English header overflowed the other way and cut the leftmost button
    // to `se`.
    const english = await eye.look(`(() => {
      const b = document.getElementById('english-first');
      const r = b.getBoundingClientRect();
      return { width: Math.round(r.width), fits: b.scrollWidth <= r.width + 1 };
    })()`);
    seen(
      "an English header does not clip its first button",
      english.fits,
      `the first button measured ${english.width}px around its own label`,
    );
  } finally {
    // Wait for it to actually go: on Windows the profile keeps a lockfile open
    // until the process is gone, and removing the directory under it throws
    // EBUSY — which would be the whole run failing on the tidying-up.
    const gone = new Promise((resolve) => child.once("exit", resolve));
    child.kill();
    await Promise.race([gone, new Promise((r) => setTimeout(r, 3000))]);
    await rm(room, { recursive: true, force: true, maxRetries: 5 }).catch(() => {});
  }

  console.log(`${pass} seen, ${fail} wrong`);
  return fail === 0 ? 0 : 1;
}

process.exit(await main());
