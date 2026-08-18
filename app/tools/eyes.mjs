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
 *
 * **All eight controls**, because five was the row as it stood in August and
 * the row has grown since. The count is the whole of finding 8: five buttons
 * fit a 430px pane and eight do not, and what a fixture short of the real row
 * proves is that the shorter row fits.
 */
const HEADER = `
<div style="width: 430px" id="narrow-pane">
  <header class="pane-head">
    <span class="pane-title" id="narrow-title">מסכת בבא בתרא עם תוספות</span>
    <span class="pane-where">קכ״ז.</span>
    <span class="pane-note is-empty" id="narrow-note">אין כאן</span>
    <div class="pane-tools" id="narrow-tools">
      <button class="tool">מפרשים · 34</button>
      <button class="tool">גלילה נפרדת</button>
      <button class="tool">קישורים</button>
      <button class="tool">שלשלת המסירה</button>
      <button class="tool">תוכן הספר</button>
      <select class="pane-move"><option>העבר ללשונית</option></select>
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

/**
 * The find bar (Ctrl+F), inside the pane it opens over.
 *
 * Four things were wrong with the first version of this bar and all four were
 * found the same way — by opening the window and looking at a screenshot of it,
 * which is the evidence this tool exists to produce without a person. The block
 * in `styles.css` names them: it was as wide as a paragraph, the count read
 * backwards, the buttons were browser buttons, and it sat on top of the pane's
 * own header. Every one is a measurement.
 *
 * The bare `<button>` beside it is the control. `glyph()` sets no class of its
 * own, so ↑ ↓ ✕ are styled purely by `.find-here button` — and the way to prove
 * that rule reached them is to put the same element outside the bar and show
 * that the browser's own face is what it wears there.
 */
const FIND = `
<div class="pane" id="find-pane" style="width: 560px; height: 240px">
  <header class="pane-head" id="find-head">
    <span class="pane-title">שולחן ערוך אורח חיים עם משנה ברורה</span>
    <span class="pane-where">א׳ ב׳</span>
    <div class="pane-tools">
      <button class="tool">מפרשים · 12</button>
      <button class="tool">גלילה משותפת</button>
      <button class="tool">קישורים</button>
      <button class="tool">השרשרת</button>
      <button class="tool">סגור</button>
    </div>
  </header>
  <div class="pane-body">
    <p class="line" data-id="a"><span class="line-address">א׳ ב׳</span><span class="line-text">יתגבר כארי לעמוד בבוקר לעבודת בוראו, שיהא הוא מעורר השחר.</span></p>
  </div>
  <div class="find-here" id="find-bar">
    <div class="find-here-row">
      <input class="find-here-box" id="find-box" aria-label="חפש בספר" placeholder="מילה או ביטוי" value="כארי">
      <span class="find-here-count" id="find-count" dir="ltr">1 / 33</span>
      <div class="find-here-walk">
        <button type="button" id="find-up" aria-label="הקודם">↑</button>
        <button type="button" aria-label="הבא">↓</button>
      </div>
      <button type="button" aria-label="סגור">✕</button>
    </div>
    <div class="find-chips">
      <div class="find-chip">
        <button type="button" class="find-chip-face" id="find-chip">חכם ▾</button>
        <div class="find-chip-menu" hidden></div>
      </div>
      <div class="find-chip">
        <button type="button" class="find-chip-face">מילה שלמה ▾</button>
        <div class="find-chip-menu" id="find-menu">
          <button type="button" class="find-chip-item is-chosen" id="find-can">מילה שלמה</button>
          <button type="button" class="find-chip-item" id="find-cannot" disabled
            title="מראה מקום הוא קפיצה למקום אחר">מראה מקום</button>
        </div>
      </div>
    </div>
    <p class="find-here-note"></p>
  </div>
</div>
<button type="button" id="bare-button">✕</button>`;

/**
 * One sefer's row in the links panel, in the drawer that holds it.
 *
 * The panel's whole argument is that 280 rows from 61 seforim become 61 lines a
 * person can read — so the line has to be readable, and the thing that decides
 * that is a flex row with a fixed-width count, a title that may ellipsise, and
 * a range that may not shrink at all. That is the same three-part header the
 * pane had when a column's name computed to **0px**, one panel over. `flex: 0 0
 * auto` on the range is the piece that makes it possible again, and a range is
 * as long as the citation is: `ס״ק א׳ … ס״ק ע״ח` is short and
 * `סימן קפ״ג סעיף ד׳ … סימן רמ״ב סעיף י״ז` is not.
 */
const LINKS = `
<section class="links" id="links-drawer">
  <div class="links-list">
    <details class="link-sefer" open>
      <summary>
        <span class="link-sefer-count" id="link-count">78</span>
        <span class="link-sefer-title" id="link-title">כף החיים על שולחן ערוך אורח חיים</span>
        <span class="link-sefer-span" id="link-span">סימן קפ״ג סעיף ד׳ … סימן רמ״ב סעיף י״ז</span>
      </summary>
      <div class="link-sefer-rows"></div>
    </details>
  </div>
</section>`;

/**
 * The named-arrangements drawer, which is the links drawer's twin.
 *
 * Two claims, and the first is the one a drawer gets wrong: **closed is zero
 * wide.** `.desks` is `position: fixed` across the whole height of the window,
 * so a few stray pixels of it are a strip down the leading edge of every screen
 * in the application, over the sefer, all the time — and it is `overflow:
 * hidden` on a `width: 0` box that keeps that from happening rather than
 * anything a reader could see going wrong. The second is the pane header's
 * question again: a row is a name, a sentence about what is in it, and a ✕, and
 * the name is the part that must not be squeezed out.
 */
const DESKS = `
<section class="desks" id="desks-drawer">
  <header class="desks-head">
    <span class="desks-title">שולחנות</span>
    <button type="button" class="panel-shut" aria-label="סגור">×</button>
  </header>
  <p class="desks-note">4 שולחנות</p>
  <div class="desks-keep">
    <input class="desks-box" dir="rtl" aria-label="שם השולחן" placeholder="למשל: סוגיית הכל שוחטין">
    <button type="button" class="tool">שמור כפי שהוא</button>
  </div>
  <p class="panel-about">שולחן הוא הסידור שאתה יושב בו.</p>
  <div class="desks-list">
    <div class="desk is-here" id="desk-row">
      <button type="button" class="desk-open" id="desk-open">
        <span class="desk-name" id="desk-name">סוגיית הכל שוחטין ושחיטתן כשרה</span>
        <span class="desk-what">4 לשוניות · 11 ספרים</span>
      </button>
      <button type="button" id="desk-forget" aria-label="שכח">✕</button>
    </div>
  </div>
</section>`;

/**
 * The sheet that goes to a printer, which nothing has ever looked at.
 *
 * A printer is the one output in this application that a person cannot see
 * before it is on paper, and `@media print` is the least testable kind of CSS
 * there is: it is a whole second stylesheet that never runs while anybody is
 * watching. A browser can be told to pretend, though — `Emulation.setEmulatedMedia`
 * — so the eye can read the printed page without a printer, which is a thing no
 * string search and no unit test can do.
 *
 * Two claims that the block in `styles.css` argues for at length and nothing
 * checked. On screen the sheet is off to the side and **laid out** — the
 * paragraph there says `display: none` would be simpler and would hand a
 * printer a page that has never been laid out. On paper the application is
 * gone, the sheet is in the flow, and the ink is black on white rather than the
 * reader's theme.
 */
const PRINT = `
<article class="print-sheet" id="print-sheet" aria-hidden="true">
  <header class="print-head">
    <p class="print-title">שולחן ערוך אורח חיים</p>
    <p class="print-provenance">מהדורת ווארשא תרמ״ב · CC BY-SA</p>
    <p class="print-where">סימן א׳</p>
  </header>
  <p class="line" data-id="p"><span class="line-address">א׳ א׳</span><span class="line-text">יתגבר כארי לעמוד בבוקר לעבודת בוראו.</span></p>
</article>`;

// `is-printing` on the body all the time, because every rule it gates is inside
// `@media print` and does nothing until the browser is asked to pretend. It is
// on the element here rather than added by an assertion so that what the print
// pass looks at is the document the application really hands a printer.

/**
 * A split and its divider, with the two controls that hang off it.
 *
 * The controls are `opacity: 0` until the pointer is on the line — the same
 * shape as the very first bug this tool was written for, a mefaresh's comment
 * drawn at `opacity: 0` in a container 16px tall. A control that fades in is a
 * control a source search cannot tell apart from one that never appears, so it
 * is measured: zero before, something after, and inside the line it hangs off.
 */
const SPLIT = `
<div class="split split-vertical" id="split-box" style="width: 620px; height: 260px">
  <div class="slot" style="flex-basis: 50%"></div>
  <div class="divider" id="split-divider" role="separator" tabindex="0" data-split="0">
    <div class="divider-controls" id="split-controls">
      <button type="button" class="divider-control" aria-label="שים זה מעל זה">⇅</button>
      <button type="button" class="divider-control" aria-label="החלף בין הצדדים">⇌</button>
    </div>
  </div>
  <div class="slot" style="flex-basis: 50%"></div>
</div>`;

const PAGE = (sheet) => `<!doctype html>
<html lang="he" dir="rtl"><head><meta charset="utf-8">
<link rel="stylesheet" href="${sheet}"></head>
<body style="margin:0" class="is-printing">${COMMENT}${HEADER}${HEADER_EN}${SHELF}${FIND}${LINKS}${DESKS}${SPLIT}${PRINT}</body></html>`;

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
 * A port this process can have to itself.
 *
 * Third attempt, and the first two are worth the paragraph because CI killed
 * both. It began as `const port = 9333` — no race, and a certainty of collision
 * the moment two copies run, which is how the gate went red while six runs
 * tested a different fix. Then `--remote-debugging-port=0` and read
 * `DevToolsActivePort`, which Chrome documents: the file never appeared on the
 * Linux runner. Then a port taken from the operating system by binding zero and
 * letting go: on the same runner Chrome then served no page on it at all.
 *
 * What CI has *shown* to work on Linux is an explicit port that Chrome binds
 * itself — that is the configuration the green runs used. So the fix is that
 * configuration with the collision removed, and nothing more clever than
 * arithmetic: the pid, which the operating system has already guaranteed is
 * unique among live processes.
 *
 * A thousand is comfortably more than the number of copies of this that will
 * ever run at once, and two runs collide only if their pids are exactly a
 * thousand apart *and* both are alive. That is not zero. It is small enough to
 * be worth having, in a tool whose two cleverer answers were both wrong on a
 * machine I cannot reach.
 */
function ownPort() {
  return 9333 + (process.pid % 1000);
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
 * **Sixty seconds, not thirty**, and the reason is a fourth red run on this same
 * clock. Run 31747194527 failed here on a commit whose only change was one
 * markdown file, and the re-run of that identical commit fifteen hours later was
 * green — so nothing in the repository was the cause, and the machine was. The
 * browser's own first line of stderr is stamped **twenty seconds after it was
 * spawned**: twenty seconds to get as far as failing to find a session bus.
 * Thirty is not a comfortable margin over twenty. A budget costs a warm machine
 * nothing, because it returns the moment the thing it wants is true; the only
 * thing a larger one buys is that a slow machine stops being reported as a
 * broken one.
 *
 * It also stops waiting the moment the browser **dies**. Polling a port for the
 * remaining minute after the process has exited is a minute spent proving
 * something already known, and it reports a timeout for what was a crash.
 *
 * # Three failures wearing one sentence
 *
 * *The browser never opened a page* was true of all of these and useful about
 * none of them: nothing ever answered on the port; or something answered and it
 * was not a browser, which is what a colliding `ownPort` would look like from
 * here; or the browser answered and listed targets with no page among them.
 * Telling them apart meant reading stderr timestamps and guessing, which is how
 * the run above was diagnosed. `heard` keeps the last thing the port actually
 * did, so the sentence names it instead.
 *
 * The stderr it prints is raw, and on Linux it is mostly `Failed to connect to
 * the bus` — noise Chrome emits on any machine with no session D-Bus, and not
 * the reason for anything. Filtering it would mean deciding which of somebody
 * else's diagnostics matter, which is the mistake `trouble.ts` is about. It is
 * the browser's own words, printed whole, for a person to read.
 *
 * # The page, and not a page
 *
 * This took the **first** target of type `page` and handed it back, on the
 * assumption that a browser started with one URL on its command line has one
 * page. It does not always. A fresh profile can list an `about:blank` beside
 * the file it was told to open, and which of the two comes first in
 * `/json/list` is not a thing anything promises.
 *
 * When it came back blank, everything downstream did exactly what it should:
 * `settled` refused to measure, spent its whole sixty seconds waiting for a
 * `.pane-body` that was never going to appear in an empty document, and
 * reported *no .pane-body after 60s*. Which reads as a slow machine, and is why
 * this clock had already been raised twice — both times for a wait that was
 * real and once, at least, for a page that was never the right one. Three CI
 * runs in a row died here, and the sixty-second budget is not the finding.
 *
 * So the target is chosen **by its URL** where there is one to choose. A run
 * that lists pages and none of them ours says so and names what it did find —
 * and then takes one anyway, because the caller navigates it. Choosing by URL
 * alone left the failure exactly where it was on a runner that lists our URL
 * and serves an empty document; what fixes that is not picking better but
 * **opening the page ourselves** rather than trusting a command line. See
 * `main`.
 */
/**
 * Whether a target's URL is the file this script wrote.
 *
 * Exact first, because `pathToFileURL` and Chrome agree about a file URL almost
 * always. The fallback is the **file name**, which is enough here and is not a
 * loose match dressed up as a strict one: the name is `specimens.html` inside a
 * directory `mkdtemp` just minted, so nothing else on the machine can be
 * wearing it. It exists because a browser is entitled to normalise a URL it was
 * handed — a drive letter's case on Windows, a percent-encoding — and a check
 * that went red over that would be this same bug with the sides swapped.
 */
function isWanted(url, wanted) {
  if (typeof url !== "string") return false;
  return url === wanted || url.endsWith(`/${path.basename(wanted)}`);
}

async function pageOf(port, wanted, said = () => "", alive = () => true) {
  // The last thing the port did, not a transcript of it starting up: a failure
  // wants the state it gave up in. Every branch below overwrites this, so what
  // survives to the throw is the most informative thing that ever happened.
  let heard = `nothing ever answered on port ${port}`;
  for (let tries = 0; tries < 240; tries += 1) {
    try {
      const answer = await fetch(`http://127.0.0.1:${port}/json/list`);
      const list = await answer.json().catch(() => null);
      if (Array.isArray(list)) {
        const pages = list.filter((t) => t.webSocketDebuggerUrl && t.type === "page");
        // Ours by name, or any page — the caller navigates whichever it gets,
        // so a blank one is a usable handle rather than a wrong answer.
        const page = pages.find((t) => isWanted(t.url, wanted)) ?? pages[0];
        if (page) return page;
        const types = list.map((t) => t.type).join(", ");
        heard = list.length
          ? `it is listening on port ${port} and lists no page, only: ${types}`
          : `it is listening on port ${port} and lists no targets at all`;
      } else {
        heard = `something answered on port ${port}, and it was not a target list`;
      }
    } catch {
      // Not up yet, which is the ordinary case for the first second or so.
    }
    if (!alive()) break;
    await new Promise((r) => setTimeout(r, 250));
  }
  const why = said().trim();
  // A browser that exited has its own explanation on stderr, and the port never
  // had anything to say about a process that was not there to answer.
  const what = alive() ? `never opened a page — ${heard}` : "started and then exited";
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
 *
 * They differed for a while, and the paragraph above is the reason they were
 * allowed to. `pageOf` was raised to sixty because what it waits for is a
 * **process starting cold** — disk, dynamic linking, a profile directory that
 * does not exist yet — which is the part a loaded runner makes slow, and which
 * had gone red four times. This clock starts after all of that has already
 * happened: the browser is up and warm, and what it is waiting for is one local
 * file to parse and one stylesheet to apply. Thirty seconds was called an
 * enormous budget for that, on the stated grounds that no run had ever spent it,
 * and the note ended by saying that if one ever did, that would be evidence.
 *
 * **One did.** Actions run 31809481885, the `shell` job, on a commit that
 * touched no file under `app/src`: `no .pane-body after 30s`. Re-running that
 * same job on that same commit was green in 1m40s. So the reasoning was right
 * about the mechanism and wrong about the ceiling — parsing one file and
 * applying one stylesheet is all this waits for, and a runner loaded enough can
 * still take longer than thirty seconds to do it. There is no third thing to
 * find here. A warm browser on a contended machine is simply slow, which is the
 * same finding as the other four, one layer further in.
 *
 * Sixty, then, and the two clocks agree again — not for symmetry, which was
 * never a reason, but because each was moved by a red of its own. It costs a
 * healthy run nothing: the loop returns the moment the style is `auto`, so the
 * budget is only ever spent by a machine that needed it, or by a failure that
 * was going to fail regardless.
 *
 * The wait is one number here and printed from it. It was two — a loop counter
 * and the word `30s` typed into the message beside it — which is the same defect
 * this repository gates elsewhere, small enough to survive being noticed: had
 * only the loop been raised, the message would have kept saying thirty and the
 * next person to read a red would have been told a lie by the check itself.
 */
const SETTLED_TRIES = 600;
const SETTLED_EVERY = 100;

async function settled(eye) {
  for (let tries = 0; tries < SETTLED_TRIES; tries += 1) {
    const found = await eye.look(`(() => {
      if (document.readyState !== 'complete') return 'loading';
      const body = document.querySelector('.pane-body');
      return body ? getComputedStyle(body).overflowY : 'no .pane-body yet';
    })()`);
    if (found === "auto") return found;
    await new Promise((r) => setTimeout(r, SETTLED_EVERY));
  }
  // Whatever it last saw, so the failure names the state rather than the wait.
  //
  // **And what state.** This used to say only `no .pane-body after 60s`, which
  // is a sentence about a clock, and three CI runs in a row were read as a slow
  // machine because of it — the tool spent its whole budget looking at a
  // document it could have described in one evaluation. A wait that ends in
  // nothing should hand back the page it was waiting on: where the browser
  // actually is, whether it finished loading, and how much markup arrived. A
  // blank document and a slow one do not look alike once anybody says so.
  const waited = (SETTLED_TRIES * SETTLED_EVERY) / 1000;
  return eye.look(`(() => {
    const body = document.querySelector('.pane-body');
    if (body) return getComputedStyle(body).overflowY;
    const html = document.documentElement?.innerHTML?.length ?? 0;
    return 'no .pane-body after ${waited}s — at ' + location.href +
      ', readyState ' + document.readyState + ', ' + html + ' characters of markup';
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
  const port = ownPort();
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
    const wanted = pathToFileURL(page).href;
    const target = await pageOf(port, wanted, () => complained, alive);
    const socket = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve);
      socket.addEventListener("error", reject);
    });
    const eye = new Eye(socket);
    await eye.send("Runtime.enable");
    await eye.send("Page.enable");

    // **Open the page, rather than trusting that the browser did.**
    //
    // The URL is on the command line and this is the same URL again, so on a
    // healthy machine it is a reload costing a few milliseconds. It is here
    // because on a CI runner it was not a reload: the target listed our file
    // and served an empty document, and `settled` then spent sixty seconds
    // proving that an empty document has no `.pane-body`. Whatever the runner
    // does with a URL argument — a first-run interstitial, a restored session,
    // a navigation that lost a race with the debugger attaching — asking for
    // the page explicitly makes it not matter. This tool wrote the file; it can
    // open it.
    await eye.send("Page.navigate", { url: wanted });

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
        const box = pane.getBoundingClientRect();
        const strays = [...document.getElementById('narrow-tools').children]
          .map((c) => ({ said: c.textContent.trim(), r: c.getBoundingClientRect() }))
          .filter((c) => c.r.left < box.left - 1 || c.r.right > box.right + 1);
        return {
          title: Math.round(t.getBoundingClientRect().width),
          note: Math.round(note.getBoundingClientRect().width),
          noteFits: note.scrollWidth <= note.getBoundingClientRect().width + 1,
          lastButton: Math.round(last.getBoundingClientRect().width),
          lastFits: last.scrollWidth <= last.getBoundingClientRect().width + 1,
          strays: strays.map((c) => c.said),
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
      // Finding 8, and the reason the three assertions above did not catch it:
      // every one of them is a **width**, and a control that has left the pane
      // is the right width in the wrong place. `סגור` at x = -168 measures 33px
      // and fits its own label perfectly.
      seen(
        `no control has left the pane ${where}`,
        seenAt.strays.length === 0,
        `${seenAt.strays.length} control(s) sat outside the pane: ${seenAt.strays.join(", ")}`,
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
    // ------------------------------------ what a divider can do (finding 9)
    //
    // > *"Tabs should be splittable in any way and movable, like we want in
    // > ksav."*
    //
    // Two controls that live at `opacity: 0` until the pointer is on the line
    // they hang off. Everything a string search can ask about them — the class
    // exists, the rule exists, the button is built — is true of a control that
    // never becomes visible, which is the first bug at the head of this file.
    // So: nothing before, something after, and inside the line either way.
    //
    // `async`, and it waits: `.divider-controls` carries a 120ms opacity
    // transition, so the computed value one statement after the class goes on
    // is still the value it is animating *from*. Read straight through, this
    // assertion measures the animation and reports the affordance broken.
    const split = await eye.look(`(async () => {
      const divider = document.getElementById('split-divider');
      const controls = document.getElementById('split-controls');
      const before = getComputedStyle(controls).opacity;
      divider.classList.add('is-dragging');
      await new Promise((r) => setTimeout(r, 300));
      const after = getComputedStyle(controls).opacity;
      const line = divider.getBoundingClientRect();
      const box = controls.getBoundingClientRect();
      const buttons = [...controls.children].map((b) => {
        const r = b.getBoundingClientRect();
        return { w: Math.round(r.width), h: Math.round(r.height),
                 named: (b.getAttribute('aria-label') || '').length };
      });
      divider.classList.remove('is-dragging');
      return {
        before, after,
        lineWidth: Math.round(line.width),
        centred: Math.abs((box.left + box.right) / 2 - (line.left + line.right) / 2) < 2,
        buttons,
      };
    })()`);

    seen(
      "a divider's controls are invisible until it is touched",
      split.before === "0",
      `they sat at opacity ${split.before} on a divider nobody was pointing at`,
    );
    seen(
      "and visible once it is",
      split.after === "1",
      `hovering the divider left them at opacity ${split.after}`,
    );
    // The line is 5px and the buttons are about 20px. They must hang **off** it
    // without widening it, or every turn of a split shifts both panes.
    seen(
      "the controls do not widen the line they sit on",
      split.lineWidth <= 6,
      `the divider measured ${split.lineWidth}px around a 5px rule`,
    );
    seen(
      "and they sit on it rather than beside it",
      split.centred,
      "the control box was not centred on the divider",
    );
    for (const [at, button] of split.buttons.entries()) {
      seen(
        `divider control ${at} is big enough to hit`,
        button.w >= 14 && button.h >= 14,
        `it measured ${button.w}×${button.h}px`,
      );
      seen(
        `divider control ${at} has a name`,
        button.named > 0,
        "a glyph with no aria-label is a button nothing can read out",
      );
    }

    // ----------------------------------------------------- the find bar (Ctrl+F)
    //
    // The four defects the `.find-here` block in `styles.css` was written to fix,
    // as four measurements. All four were read off a screenshot of the running
    // window by a person; this is the same reading, by a machine, on every push.
    //
    // The header is measured at **two** widths, because the whole argument for
    // `--under-head` is that a constant cannot be right: `.pane-head` wraps, so
    // at 560px it is one row and at 260px it is two, and a bar pinned at
    // `2.4rem` clears the first and lands on top of the second.
    const find = await eye.look(`(() => {
      const pane = document.getElementById('find-pane');
      const head = document.getElementById('find-head');
      const bar = document.getElementById('find-bar');
      // The same rule findhere.ts:openOn applies — the header's measured height
      // and four pixels — so what this asks is whether that rule is sufficient,
      // not whether somebody typed the same number in two places.
      const place = (px) => {
        pane.style.width = px + 'px';
        bar.style.setProperty('--under-head', (head.offsetHeight + 4) + 'px');
        const h = head.getBoundingClientRect();
        const b = bar.getBoundingClientRect();
        const p = pane.getBoundingClientRect();
        return {
          rows: Math.round(h.height),
          clears: Math.round(b.top - h.bottom),
          bar: Math.round(b.width),
          pane: Math.round(p.width),
          // How far past either edge of the pane the bar reaches. The pane clips
          // nothing, so a negative number here is the bar drawn over whatever is
          // next to it.
          spills: Math.round(Math.max(p.left - b.left, b.right - p.right)),
        };
      };
      const out = { wide: place(560), narrow: place(260) };
      place(560);
      const count = document.getElementById('find-count');
      const walk = document.getElementById('find-up');
      const bare = document.getElementById('bare-button');
      const chip = document.getElementById('find-chip');
      const face = (el) => {
        const cs = getComputedStyle(el);
        return {
          background: cs.backgroundColor,
          border: cs.borderTopWidth,
          radius: cs.borderTopLeftRadius,
        };
      };
      // A choice on an open menu, and whether it looks like one you can take.
      const look = (el) => {
        const cs = getComputedStyle(el);
        const r = el.getBoundingClientRect();
        return {
          opacity: Number(cs.opacity),
          cursor: cs.cursor,
          width: Math.round(r.width),
          fits: el.scrollWidth <= r.width + 1,
        };
      };
      // Does the hidden attribute hide it? That attribute works by the
      // browser's own rule setting display to none, which any author display
      // beats. So this asks the browser rather than reading the stylesheet:
      // set the attribute the way FindHere.close does, and put it back.
      // (No backticks in here. This comment is inside a template literal and
      // one of them ends it, which is how the last hour went.)
      bar.hidden = true;
      const whenHidden = getComputedStyle(bar).display;
      bar.hidden = false;
      const whenShown = getComputedStyle(bar).display;
      return {
        ...out,
        whenHidden,
        whenShown,
        direction: getComputedStyle(count).direction,
        figures: getComputedStyle(count).fontVariantNumeric,
        walk: face(walk),
        bare: face(bare),
        chip: face(chip),
        can: look(document.getElementById('find-can')),
        cannot: look(document.getElementById('find-cannot')),
      };
    })()`);

    // **The fifth defect in this bar, and the first one a screenshot of NixOS
    // found rather than a screenshot of Windows.**
    //
    // `.find-here` sets `display: flex`, and an author's `display` beats the
    // browser's `[hidden] { display: none }`. So `element.hidden = true` did
    // nothing to it: the bar was drawn over the toolbar of a window with no
    // sefer open, in the first picture ever taken of Girsa on NixOS.
    //
    // Every other panel in `styles.css` that is toggled this way carries its
    // own `[hidden]` rule — `.picker`, `.shelf`, `.find`, `.lane`, `.settings`
    // and three more. This one was the exception, and the whole family is one.
    //
    // Asserted as a pair. *Hidden means gone* is the defect; *shown means
    // flex* is the thing a careless fix breaks, and a rule that hides the bar
    // permanently would pass the first assertion on its own.
    seen(
      "hidden hides the find bar",
      find.whenHidden === "none",
      `with the hidden attribute set it computed to display: ${find.whenHidden} — ` +
        `an author's display beats the browser's [hidden] rule, so the bar is ` +
        `drawn whether or not anything opened it`,
    );
    seen(
      "and taking it off gives the bar back",
      find.whenShown === "flex",
      `without the attribute it computed to display: ${find.whenShown}`,
    );

    // `1 / 33` in a right-to-left window is laid out `33 / 1` — two numbers with
    // a neutral between them take the paragraph's direction, so the bar reported
    // the reader's place and the total the wrong way round.
    //
    // **This is half the guard and it is worth saying which half.** The specimen
    // writes `dir="ltr"` itself, so what this can catch is a rule in 4,000 lines
    // of stylesheet setting `direction` back — the attribute would still be on
    // the element, and only a browser can say it lost. What it cannot catch is
    // `findhere.ts` ceasing to set it, because then there is no attribute to
    // override and this specimen is not that document. That half is
    // `findhere.test.mjs`, which reads the element the class really builds.
    seen(
      "the find bar's count does not read backwards in a right-to-left window",
      find.direction === "ltr",
      `direction computed to ${find.direction} — 1 / 33 is drawn 33 / 1`,
    );
    seen(
      "and does not shiver as the number widens",
      find.figures.includes("tabular-nums"),
      `font-variant-numeric was ${find.figures}`,
    );

    // *"Ugly and unwieldy"*, and this half of it was a `min-width` on the box in
    // a flex row with nothing holding it back: an empty input took four hundred
    // pixels of a daf.
    //
    // The two questions are not the same question, which is what the first
    // version of this got wrong by asking one of them at both widths. **Staying
    // inside the pane** is absolute — the bar is absolutely positioned in a
    // column that clips nothing, so a bar wider than its column is drawn over
    // the sefer *beside* it, which is worse than a wide bar and is what 324px in
    // a 260px column meant. **Leaving the line room** is a question you can only
    // ask where there is room to leave: in a 260px column there is one bar's
    // worth of width and that is all there is.
    for (const [where, at] of Object.entries({ wide: find.wide, narrow: find.narrow })) {
      seen(
        `the find bar stays inside the ${where} pane`,
        at.spills <= 0,
        `the bar measured ${at.bar}px over a ${at.pane}px pane and reached ` +
          `${at.spills}px past its edge`,
      );
      // It floats, deliberately — a bar that pushed the text down would move the
      // line the reader is looking at, every time it opened. What it must not do
      // is float over the header, which is where *The chain* was covered up.
      seen(
        `and clears the header of the ${where} pane`,
        at.clears >= 0,
        `the bar's top was ${-at.clears}px above the bottom of a ${at.rows}px header`,
      );
    }
    seen(
      "and leaves a pane that has room most of its line",
      find.wide.bar <= find.wide.pane * 0.62,
      `the bar measured ${find.wide.bar}px over a ${find.wide.pane}px pane`,
    );
    seen(
      "the pane header really does wrap when the pane is narrow",
      find.narrow.rows > find.wide.rows,
      `${find.wide.rows}px at 560 and ${find.narrow.rows}px at 260 — the narrow ` +
        `case this is measuring does not happen, so it proves nothing`,
    );

    // `glyph()` sets no class of its own, so ↑ ↓ ✕ arrived as three grey browser
    // slabs in a window where nothing else has a raised edge. The control is the
    // same element outside the bar, still wearing the face the browser gives it.
    seen(
      "the find bar's glyphs are not browser buttons",
      find.walk.background !== find.bare.background && find.walk.border === "0px",
      `the walk button had background ${find.walk.background} and a ` +
        `${find.walk.border} border, against ${find.bare.background} and ` +
        `${find.bare.border} on a bare one`,
    );
    // And the rule that flattens them must not reach the chips, which have a
    // face of their own. Both are `.find-here button`; only one is a chip.
    seen(
      "and the chips keep their own face inside it",
      find.chip.border !== "0px",
      `a chip's border computed to ${find.chip.border}, the same as a glyph's`,
    );

    // The one choice this bar declines — *a mareh makom*, which is a jump out of
    // the sefer the bar is inside. It was on the row and it quietly found
    // nothing. Grey is only half the answer: a control that is grey and silent
    // is one a reader clicks twice, so the reason is on `title` and the pointer
    // says so before anybody hovers long enough to read it.
    seen(
      "a choice the find bar cannot honour looks like one",
      find.cannot.opacity < find.can.opacity,
      `the declined choice computed to opacity ${find.cannot.opacity}, against ` +
        `${find.can.opacity} on the one beside it`,
    );
    seen(
      "and the pointer says so before the tooltip does",
      find.cannot.cursor === "not-allowed" && find.can.cursor !== "not-allowed",
      `the declined choice's cursor was ${find.cannot.cursor} and the other's ` +
        `${find.can.cursor}`,
    );
    seen(
      "a greyed choice is still readable",
      find.cannot.fits && find.cannot.width > 0,
      `it measured ${find.cannot.width}px around its own label`,
    );

    // ------------------------------------------------- one sefer's row in links
    //
    // 280 rows from 61 seforim became 61 lines, and a line nobody can read is
    // not a line. The range is `flex: 0 0 auto` and a range is as long as its
    // citation, so this is the pane header's question one panel over: what gives
    // way first, the name of the sefer or the apparatus beside it?
    const links = await eye.look(`(() => {
      const drawer = document.getElementById('links-drawer');
      // The drawer opens over 0.12s, and an evaluation that adds the class and
      // measures in the same tick measures the first frame of that — 1px, which
      // reads exactly like a drawer that does not open. What is being asked
      // about here is the width it settles at, so the animation is turned off
      // rather than waited on.
      drawer.style.transition = 'none';
      drawer.classList.add('is-open');
      const measure = () => {
        const box = drawer.getBoundingClientRect();
        const t = document.getElementById('link-title');
        const c = document.getElementById('link-count');
        const s = document.getElementById('link-span');
        const r = t.getBoundingClientRect();
        return {
          drawer: Math.round(box.width),
          title: Math.round(r.width),
          count: Math.round(c.getBoundingClientRect().width),
          countFits: c.scrollWidth <= c.getBoundingClientRect().width + 1,
          span: Math.round(s.getBoundingClientRect().width),
          spanFits: s.scrollWidth <= s.getBoundingClientRect().width + 1,
          overflows: Math.round(document.querySelector('.link-sefer > summary').scrollWidth) >
            Math.round(document.querySelector('.link-sefer > summary').clientWidth) + 1,
        };
      };
      const open = measure();
      drawer.classList.remove('is-open');
      const shut = Math.round(drawer.getBoundingClientRect().width);
      return { open, shut };
    })()`);

    seen(
      "a sefer's name survives beside a long range of se'ifim",
      links.open.title >= 40,
      `the title measured ${links.open.title}px in a ${links.open.drawer}px drawer, ` +
        `beside a ${links.open.span}px range`,
    );
    seen(
      "the count column is a column",
      links.open.countFits && links.open.count > 0,
      `the count measured ${links.open.count}px around its own digits`,
    );
    seen(
      "and the range is not clipped either",
      links.open.spanFits,
      `the range measured ${links.open.span}px around text that needs more`,
    );
    seen(
      "nothing in the row runs off the side of the drawer",
      !links.open.overflows,
      `the summary's content is wider than the drawer holding it`,
    );

    // ------------------------------------------------ the arrangements drawer
    //
    // A `position: fixed` column down the whole height of the window. Closed, it
    // has to be nothing at all: a few stray pixels here are a strip over the
    // sefer on every screen in the application, forever, and the only thing
    // stopping that is `overflow: hidden` on a `width: 0` box.
    const desks = await eye.look(`(() => {
      const drawer = document.getElementById('desks-drawer');
      const shut = Math.round(drawer.getBoundingClientRect().width);
      // Off, for the reason the links drawer above gives.
      drawer.style.transition = 'none';
      drawer.classList.add('is-open');
      const row = document.getElementById('desk-row');
      const name = document.getElementById('desk-name');
      const open = document.getElementById('desk-open');
      const forget = document.getElementById('desk-forget');
      const out = {
        shut,
        drawer: Math.round(drawer.getBoundingClientRect().width),
        name: Math.round(name.getBoundingClientRect().width),
        forget: Math.round(forget.getBoundingClientRect().width),
        forgetFits: forget.scrollWidth <= forget.getBoundingClientRect().width + 1,
        // The X has to be inside the drawer, not pushed past its edge by a long
        // arrangement name. That is what min-width: 0 on .desk-open is for.
        inside:
          Math.round(forget.getBoundingClientRect().right) <=
            Math.round(drawer.getBoundingClientRect().right) + 1 &&
          Math.round(forget.getBoundingClientRect().left) >=
            Math.round(drawer.getBoundingClientRect().left) - 1,
        rail: getComputedStyle(row).boxShadow,
      };
      drawer.classList.remove('is-open');
      return out;
    })()`);

    seen(
      "a closed arrangements drawer is nothing at all",
      desks.shut === 0,
      `it measured ${desks.shut}px wide with no is-open — a strip over the sefer`,
    );
    seen(
      "an open one shows an arrangement's name",
      desks.name >= 40,
      `the name measured ${desks.name}px in a ${desks.drawer}px drawer`,
    );
    seen(
      "and its ✕ is inside the drawer",
      desks.inside && desks.forgetFits,
      `the ✕ measured ${desks.forget}px and sits outside the ${desks.drawer}px drawer`,
    );
    seen(
      "the desk you are sitting at is marked",
      desks.rail !== "none",
      `box-shadow on .desk.is-here computed to ${desks.rail}`,
    );

    seen(
      "an English header does not clip its first button",
      english.fits,
      `the first button measured ${english.width}px around its own label`,
    );

    // ------------------------------------------------------------------- paper
    //
    // **Last, because it changes the medium under the whole document.**
    //
    // A printer is the one output here that nobody can see before it is on
    // paper, and `@media print` is a second stylesheet that never runs while
    // anybody is watching. `printing was never sent to a printer` is an honest
    // row in the handoff and it stays one — a dialogue was never accepted, and
    // where a PDF writer puts a file is still unverified. What this closes is
    // the half that is CSS: whether the rules that fire when the medium changes
    // do what the block in `styles.css` says they do.
    const screen = await eye.look(`(() => {
      const sheet = document.getElementById('print-sheet');
      const r = sheet.getBoundingClientRect();
      return {
        left: Math.round(r.left),
        height: Math.round(r.height),
        display: getComputedStyle(sheet).display,
      };
    })()`);

    // `position: absolute; left: -10000px` and not `display: none`, because a
    // hidden element has no layout and a printer asked to lay out a page that
    // has never been laid out gets it wrong in a way that shows up only on
    // paper. That sentence is a claim about the box, so measure the box.
    seen(
      "the sheet is off the screen and still laid out",
      screen.display !== "none" && screen.height > 0 && screen.left < -1000,
      `display ${screen.display}, ${screen.height}px tall, at ${screen.left}px`,
    );

    await eye.send("Emulation.setEmulatedMedia", { media: "print" });
    const paper = await eye.look(`(() => {
      const sheet = document.getElementById('print-sheet');
      const cs = getComputedStyle(sheet);
      const body = getComputedStyle(document.body);
      const line = document.querySelector('.print-sheet .line');
      return {
        position: cs.position,
        left: cs.left,
        // Everything that is the application rather than the sefer.
        pane: getComputedStyle(document.getElementById('find-pane')).display,
        shelf: getComputedStyle(document.getElementById('shelf-box').parentElement ??
          document.getElementById('shelf-box')).display,
        drawer: getComputedStyle(document.getElementById('links-drawer')).display,
        ink: body.color,
        page: body.backgroundColor,
        address: getComputedStyle(document.querySelector('.print-sheet .line-address')).color,
        breaking: getComputedStyle(line).breakInside,
      };
    })()`);

    seen(
      "on paper the sheet is in the flow",
      paper.position === "static" && paper.left === "auto",
      `the sheet was ${paper.position} at ${paper.left} — off the side of the page`,
    );
    seen(
      "and the application is not on it",
      paper.pane === "none" && paper.drawer === "none",
      `a pane printed as ${paper.pane} and the links drawer as ${paper.drawer}`,
    );
    // A dark theme sent to a printer is a black page, which is unreadable and an
    // act of violence against a toner cartridge. The reader's theme is a
    // decision about a lit screen in a room and has nothing to say about paper.
    seen(
      "the ink is black on white, whatever the reader's theme is",
      paper.ink === "rgb(0, 0, 0)" && paper.page === "rgb(255, 255, 255)",
      `${paper.ink} on ${paper.page}`,
    );
    seen(
      "the margin address is apparatus rather than text",
      paper.address !== paper.ink,
      `the address printed in ${paper.address}, the same as the words`,
    );
    // A siman that runs over a page break should break between its se'ifim and
    // not through the middle of one.
    seen(
      "a se'if does not break across two sheets",
      paper.breaking === "avoid",
      `break-inside computed to ${paper.breaking}`,
    );
    await eye.send("Emulation.setEmulatedMedia", { media: "" });
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
