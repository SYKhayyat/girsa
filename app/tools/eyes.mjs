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
// so and exits 0: a developer without one is not a failing build, and CI that
// wants it enforced can set the variable.

import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { APP } from "./paths.mjs";

/** Where Edge is, on a machine that has it. */
const EDGES = [
  process.env.EYES_BROWSER,
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
  "/usr/bin/microsoft-edge",
  "/usr/bin/google-chrome",
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
].filter(Boolean);

async function findBrowser() {
  const { access } = await import("node:fs/promises");
  for (const candidate of EDGES) {
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

const PAGE = (sheet) => `<!doctype html>
<html lang="he" dir="rtl"><head><meta charset="utf-8">
<link rel="stylesheet" href="${sheet}"></head>
<body style="margin:0">${COMMENT}${HEADER}${HEADER_EN}</body></html>`;

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

/** The browser's page target, once it has one. */
async function pageOf(port) {
  for (let tries = 0; tries < 60; tries += 1) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      const page = list.find((t) => t.type === "page");
      if (page?.webSocketDebuggerUrl) return page;
    } catch {
      // Not up yet.
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error("the browser never opened a page");
}

async function main() {
  const browser = await findBrowser();
  if (!browser) {
    console.log("no browser found — set EYES_BROWSER to one. Nothing looked at.");
    return 0;
  }

  const room = await mkdtemp(path.join(tmpdir(), "girsa-eyes-"));
  const sheet = pathToFileURL(path.join(APP, "src", "styles.css")).href;
  const page = path.join(room, "specimens.html");
  await writeFile(page, PAGE(sheet), "utf8");

  const port = 9333;
  const child = spawn(
    browser,
    [
      "--headless=new",
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${path.join(room, "profile")}`,
      "--no-first-run",
      "--disable-gpu",
      "--window-size=1360,900",
      "--allow-file-access-from-files",
      pathToFileURL(page).href,
    ],
    { stdio: "ignore" },
  );

  try {
    const target = await pageOf(port);
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
    const styled = await eye.look(
      `getComputedStyle(document.querySelector('.pane-body')).overflowY`,
    );
    seen("the stylesheet loaded", styled === "auto", `overflow-y was ${styled}`);

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
