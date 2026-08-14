// What a photograph costs, measured — because nobody has photographed a sefer.
//
//     node tools/degraded-ocr.mjs personal 151 152 153
//
// # Why this exists
//
// `docs/record/scans.md` reports tesseract at 90.6% on this shelf's Berachos,
// and then says the thing that matters about that number: **every page it was
// measured on is born-digital.** A 300-dpi render of a typeset PDF is the best
// input an OCR engine will ever see. A photograph of a Vilna Shas is a
// different picture — out of focus in one corner, not flat, grey where the
// paper has aged, and JPEG'd by the phone that took it — and the record says
// the engine will do worse on one *by an unknown amount*.
//
// Unknown is the honest word and it is a bad place to stop. This bounds it.
//
// # It is a proxy, and it says so in every sentence it prints
//
// **This is not a photograph.** It is a born-digital page put through named
// degradations, one at a time and then all together, and it can only ever be a
// *floor*: a real photograph brings things nothing here simulates — uneven
// lighting across the page, the gutter shadow of a bound sefer, show-through
// from the other side of thin paper, a lens that is sharp in the middle and
// soft at the edges, and print that was never crisp to begin with because it
// was set in 1880.
//
// So what comes out of this is not *how well OCR reads a photographed sefer*.
// It is *how much of tesseract's accuracy each of five things costs it*, which
// is a real number and is strictly better than nothing. Read the table as a
// lower bound on the damage.
//
// # The ground truth is the file's own text, not a second OCR run
//
// The scan on this shelf is born-digital, so its PDF carries the words the
// typesetter put there. `personal/words/…/pages.jsonl` holds them, read by
// `embedded` — *the file said so*, which needs no model and cannot invent a
// word. Scoring against that rather than against a clean tesseract run is the
// difference between measuring degradation and measuring tesseract's agreement
// with itself.
//
// # What it needs
//
// Tesseract with a Hebrew model, and a browser. The browser is not decoration:
// this repository has exactly one PDF renderer — pdf.js, in the window — and
// adding a second in a measuring tool would mean measuring a page the
// application never draws.

// A tool that prints a report.
/* eslint-disable no-console */

import { spawn, spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");
const APP = path.join(ROOT, "app");

/** 300 dpi against a 72-dpi PDF page: what `girsa-scan/src/engine.rs` measured
 *  tesseract at, so the clean row here is comparable with the record's. */
const OCR_SCALE = 300 / 72;

/**
 * The degradations, each named for the thing about a photograph it stands in
 * for.
 *
 * `filter` is a CSS filter applied while drawing, `turn` is degrees of
 * rotation, `grain` is the amplitude of per-pixel noise, `jpeg` re-encodes at
 * that quality, and `dpi` is the resolution the page is handed over at. One at
 * a time, so the table says which one costs what, and then all together —
 * because the last row is the only one trying to be a photograph and the others
 * are trying to explain it.
 *
 * # The first draft of this table measured nothing, and the table said so
 *
 * It had a 1.4-pixel blur and a 1.6-degree turn against a 300-dpi render, which
 * is 2,550 pixels wide with letters forty tall. Every row came back within a
 * point of clean and the "photograph" row came back **above** it — a result
 * that is not a finding, it is a tool reporting its own noise. A degradation
 * has to be proportional to the letter or it is not a degradation.
 *
 * So the blur is a fraction of the render's height, and `dpi` is here at all:
 * the largest thing a photograph takes away is **pixels per letter**. A 300-dpi
 * scan is what a flatbed gives; a phone held over a sefer gives perhaps 120
 * once focus and perspective have had their share, and no amount of sharpening
 * puts back a serif that was never sampled.
 */
const DEGRADED = [
  { name: "clean", why: "the 300-dpi render, as the record measured it" },
  {
    name: "150 dpi",
    why: "half the sampling — a phone held over the sefer, focus perfect",
    dpi: 150,
  },
  {
    name: "100 dpi",
    why: "a third, which is a photograph taken from further back",
    dpi: 100,
  },
  {
    name: "soft",
    why: "focus that is right in the middle of the page and not at its edge",
    blur: 0.0012,
  },
  {
    name: "aged",
    why: "grey paper and ink that is no longer black",
    filter: "contrast(0.55) brightness(1.15) sepia(0.3)",
  },
  {
    name: "askew",
    why: "a page held by hand rather than laid flat",
    turn: 2.5,
  },
  {
    name: "grainy",
    why: "a sensor in the light a beis medrash actually has",
    grain: 34,
  },
  {
    name: "compressed",
    why: "what a photograph is once the phone has saved it",
    dpi: 150,
    jpeg: 0.3,
  },
  {
    name: "a photograph",
    why: "all of it at once, which is the only row pretending to be one",
    dpi: 150,
    blur: 0.0012,
    filter: "contrast(0.55) brightness(1.15) sepia(0.3)",
    turn: 2.5,
    grain: 34,
    jpeg: 0.3,
  },
];

/** Where Edge or Chrome is. The same list `app/tools/eyes.mjs` carries. */
const BROWSERS = [
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
  "/usr/bin/microsoft-edge",
  "/usr/bin/google-chrome",
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
];

async function findBrowser() {
  const { access } = await import("node:fs/promises");
  const said = process.env.EYES_BROWSER;
  for (const candidate of said ? [said] : BROWSERS) {
    try {
      await access(candidate);
      return candidate;
    } catch {
      // The next one.
    }
  }
  return null;
}

/** Where tesseract is. */
function findTesseract() {
  const said = process.env.GIRSA_TESSERACT;
  const candidates = said
    ? [said]
    : [
        "C:/Program Files/Tesseract-OCR/tesseract.exe",
        "/usr/bin/tesseract",
        "/usr/local/bin/tesseract",
        "/opt/homebrew/bin/tesseract",
      ];
  return candidates.find((c) => existsSync(c)) ?? null;
}

/**
 * The words a Hebrew comparison is made of.
 *
 * Nikud off and final letters folded — the same normalizing `girsa-hebrew`
 * does, spelled here because this tool is node and that crate is Rust. A
 * comparison of two spellings of one word is the whole measurement, so it may
 * not be a comparison of two ways of writing a shin.
 */
function words(text) {
  const FINAL = { "\u05DA": "\u05DB", "\u05DD": "\u05DE", "\u05DF": "\u05E0", "\u05E3": "\u05E4", "\u05E5": "\u05E6" };
  return text
    .replace(/[\u0591-\u05C7]/gu, "")
    .split(/[^\u05D0-\u05EA]+/u)
    .filter(Boolean)
    .map((word) => [...word].map((c) => FINAL[c] ?? c).join(""));
}

/** A multiset difference, which is what "how many of these words are there" is. */
function kept(truth, got) {
  const have = new Map();
  for (const word of got) have.set(word, (have.get(word) ?? 0) + 1);
  let found = 0;
  for (const word of truth) {
    const n = have.get(word) ?? 0;
    if (n > 0) {
      have.set(word, n - 1);
      found += 1;
    }
  }
  return found;
}

/** The page's own words, as the PDF's text layer gave them. */
async function truthOf(personal, slug, page) {
  const file = path.join(personal, "words", ...slug.split("/"), "pages.jsonl");
  const body = await readFile(file, "utf8");
  let found = null;
  for (const line of body.split("\n")) {
    if (!line.trim()) continue;
    let read;
    try {
      read = JSON.parse(line);
    } catch {
      continue;
    }
    // The last line for a page wins: this is a log, not a table.
    if (read.page === page && read.by === "embedded") found = read;
  }
  return found?.words?.map((w) => w.text).join(" ") ?? null;
}

/** A page that renders one page of a PDF and hands back each variant as a data URL. */
function renderer(pdf, worker, file, page) {
  return `<!doctype html><meta charset="utf-8"><title>render</title>
<body><script type="module">
import * as pdfjs from ${JSON.stringify(pdf)};
pdfjs.GlobalWorkerOptions.workerSrc = ${JSON.stringify(worker)};
window.render = async (page, scale, variants) => {
  const doc = await pdfjs.getDocument({ url: ${JSON.stringify(file)} }).promise;
  const sheet = await doc.getPage(page);
  const viewport = sheet.getViewport({ scale });
  const base = document.createElement("canvas");
  base.width = Math.floor(viewport.width);
  base.height = Math.floor(viewport.height);
  await sheet.render({ canvas: base, canvasContext: base.getContext("2d"), viewport }).promise;

  const out = {};
  for (const v of variants) {
    // Fewer pixels per letter, which is the largest thing a photograph takes
    // away. Done by drawing smaller and handing the smaller image over — not by
    // shrinking and growing back, which would be simulating a resample rather
    // than a camera.
    const shrink = v.dpi ? v.dpi / 300 : 1;
    const w = Math.max(1, Math.round(base.width * shrink));
    const h = Math.max(1, Math.round(base.height * shrink));
    const c = document.createElement("canvas");
    // A rotation needs room, or the corners of the page go off the edge — and a
    // measurement that cropped the first word of every line would be measuring
    // the crop.
    const pad = v.turn ? Math.ceil(w * Math.abs(Math.sin((v.turn * Math.PI) / 180))) + 8 : 0;
    c.width = w + pad * 2;
    c.height = h + pad * 2;
    const ctx = c.getContext("2d");
    // White, because a photograph of a page is a page and not a transparency —
    // and because a rotation leaves triangles of nothing in the corners.
    ctx.fillStyle = "#fff";
    ctx.fillRect(0, 0, c.width, c.height);
    // Blur as a fraction of the page's height, so it is the same blur relative
    // to a letter at every resolution. A pixel count would be a different
    // degradation on every row.
    const blur = v.blur ? "blur(" + (v.blur * h).toFixed(2) + "px)" : "";
    const filter = [blur, v.filter ?? ""].filter(Boolean).join(" ");
    if (filter) ctx.filter = filter;
    ctx.translate(c.width / 2, c.height / 2);
    if (v.turn) ctx.rotate((v.turn * Math.PI) / 180);
    ctx.drawImage(base, -w / 2, -h / 2, w, h);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.filter = "none";
    if (v.grain) {
      const px = ctx.getImageData(0, 0, c.width, c.height);
      for (let i = 0; i < px.data.length; i += 4) {
        const n = (Math.random() - 0.5) * 2 * v.grain;
        px.data[i] = Math.max(0, Math.min(255, px.data[i] + n));
        px.data[i + 1] = Math.max(0, Math.min(255, px.data[i + 1] + n));
        px.data[i + 2] = Math.max(0, Math.min(255, px.data[i + 2] + n));
      }
      ctx.putImageData(px, 0, 0);
    }
    out[v.name] = v.jpeg ? c.toDataURL("image/jpeg", v.jpeg) : c.toDataURL("image/png");
  }
  return out;
};
window.ready = true;
</script></body>`;
}

/** The CDP plumbing, in the shape `app/tools/eyes.mjs` settled on. */
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
      }, 120_000);
    });
  }

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

function ownPort() {
  return 10_000 + Math.floor(Math.random() * 40_000);
}

async function pageOf(port, wanted, alive = () => true) {
  for (let tries = 0; tries < 240; tries += 1) {
    try {
      const answer = await fetch(`http://127.0.0.1:${port}/json/list`);
      const list = await answer.json().catch(() => null);
      if (Array.isArray(list)) {
        const pages = list.filter((t) => t.webSocketDebuggerUrl && t.type === "page");
        const page = pages.find((t) => t.url === wanted) ?? pages[0];
        if (page) return page;
      }
    } catch {
      // Not up yet.
    }
    if (!alive()) break;
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error("the browser never opened a page");
}

/** Run tesseract over one image and hand back what it read. */
function readWith(tesseract, image, tessdata) {
  const args = [image, "stdout", "-l", "heb", "--psm", "6"];
  if (tessdata) args.push("--tessdata-dir", tessdata);
  const out = spawnSync(tesseract, args, { encoding: "utf8" });
  if (out.status !== 0) {
    throw new Error(`tesseract: ${out.stderr?.trim() || out.status}`);
  }
  return out.stdout ?? "";
}

async function main() {
  const [personalArg, ...pageArgs] = process.argv.slice(2);
  const personal = path.resolve(personalArg ?? path.join(ROOT, "personal"));
  const pages = pageArgs.length ? pageArgs.map(Number) : [151];
  const slug = process.env.GIRSA_SCAN_SLUG ?? "user/berachos-combined";
  const file = path.join(personal, "files", `${slug.replace(/\//gu, "-")}.pdf`);

  const tesseract = findTesseract();
  if (!tesseract) {
    console.log("no tesseract — set GIRSA_TESSERACT. Nothing measured.");
    return 0;
  }
  const tessdata = existsSync(path.join(personal, "tessdata", "heb.traineddata"))
    ? path.join(personal, "tessdata")
    : null;
  const browser = await findBrowser();
  if (!browser) {
    console.log("no browser — set EYES_BROWSER. Nothing measured.");
    return 0;
  }
  if (!existsSync(file)) {
    console.log(`no scan at ${file}. Nothing measured.`);
    return 0;
  }

  const room = await mkdtemp(path.join(tmpdir(), "girsa-degraded-"));
  const html = path.join(room, "render.html");
  await writeFile(
    html,
    renderer(
      pathToFileURL(path.join(APP, "node_modules", "pdfjs-dist", "build", "pdf.mjs")).href,
      pathToFileURL(path.join(APP, "node_modules", "pdfjs-dist", "build", "pdf.worker.mjs")).href,
      pathToFileURL(file).href,
      pages[0],
    ),
    "utf8",
  );
  const wanted = pathToFileURL(html).href;
  const port = ownPort();
  const child = spawn(
    browser,
    [
      "--headless=new",
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${path.join(room, "profile")}`,
      "--no-first-run",
      "--disable-gpu",
      "--no-sandbox",
      "--disable-dev-shm-usage",
      "--allow-file-access-from-files",
      wanted,
    ],
    { stdio: ["ignore", "ignore", "ignore"] },
  );

  const scored = [];
  try {
    const target = await pageOf(port, wanted, () => child.exitCode === null);
    const socket = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve);
      socket.addEventListener("error", reject);
    });
    const eye = new Eye(socket);
    await eye.send("Runtime.enable");
    await eye.send("Page.enable");
    await eye.send("Page.navigate", { url: wanted });
    for (let tries = 0; tries < 200; tries += 1) {
      if (await eye.look("window.ready === true")) break;
      await new Promise((r) => setTimeout(r, 100));
    }

    for (const page of pages) {
      const truth = await truthOf(personal, slug, page);
      if (!truth) {
        console.log(`page ${page}: the file's own text is not on this shelf — skipped`);
        continue;
      }
      const want = words(truth);
      const urls = await eye.look(
        `window.render(${page}, ${OCR_SCALE}, ${JSON.stringify(DEGRADED)})`,
      );
      for (const variant of DEGRADED) {
        const url = urls[variant.name];
        if (!url) continue;
        const [head, body] = url.split(",");
        const image = path.join(room, `${page}-${variant.name.replace(/\s/gu, "-")}.${head.includes("jpeg") ? "jpg" : "png"}`);
        await writeFile(image, Buffer.from(body, "base64"));
        const read = words(readWith(tesseract, image, tessdata));
        scored.push({
          page,
          variant: variant.name,
          why: variant.why,
          of: want.length,
          found: kept(want, read),
          read: read.length,
        });
      }
    }
  } finally {
    child.kill();
    await rm(room, { recursive: true, force: true }).catch(() => undefined);
  }

  if (scored.length === 0) {
    console.log("nothing measured.");
    return 1;
  }

  console.log("\nA PROXY, NOT A PHOTOGRAPH. A born-digital page put through named");
  console.log("degradations. A real photograph brings uneven lighting, a gutter shadow,");
  console.log("show-through from thin paper and print that was never crisp — so read");
  console.log("this as a floor on the damage, not as a measurement of a photographed sefer.\n");
  console.log("ground truth: the PDF's own text layer, which needs no model.\n");

  const rows = new Map();
  for (const row of scored) {
    const held = rows.get(row.variant) ?? { of: 0, found: 0, read: 0, why: row.why, pages: 0 };
    held.of += row.of;
    held.found += row.found;
    held.read += row.read;
    held.pages += 1;
    rows.set(row.variant, held);
  }
  const clean = rows.get("clean");
  const width = Math.max(...[...rows.keys()].map((k) => k.length));
  console.log(`${"".padEnd(width)}  words  found   of what clean found`);
  for (const [name, held] of rows) {
    const share = held.of === 0 ? 0 : (held.found / held.of) * 100;
    const against =
      clean && clean.found > 0 ? `${((held.found / clean.found) * 100).toFixed(0)}%` : "—";
    console.log(
      `${name.padEnd(width)}  ${String(held.read).padStart(5)}  ${share.toFixed(1).padStart(5)}%  ${against.padStart(4)}   ${held.why}`,
    );
  }
  console.log(`\nover ${clean?.pages ?? 0} page(s), ${clean?.of ?? 0} words of ground truth.`);
  return 0;
}

process.exitCode = await main();
