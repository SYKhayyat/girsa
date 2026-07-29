// Ask a PDF what its own words are, on a terminal — so that W26 can be seen
// without a window (BUILDER.md §0.3).
//
//   node app/tools/glyphs.mjs personal/files/user-berachos-combined.pdf 7 8 51
//   node app/tools/glyphs.mjs <file>            # every page
//
// It prints one JSON object per page, which is what `girsa-read` eats:
//
//   node app/tools/glyphs.mjs <file> 7 \
//     | cargo run -q -p girsa-app --bin girsa-read -- corpus personal user/<slug>
//
// The extraction itself is `app/src/glyphs.ts`, imported here rather than
// copied: this has to be the same code path the window runs or it is not a
// reproduction of anything. It is the same pdf.js too — the one bundled into
// the window, out of `app/node_modules`.

import { readFileSync } from "node:fs";
import { glyphsOf } from "../src/glyphs.ts";

const [file, ...want] = process.argv.slice(2);
if (!file) {
  console.error("usage: node app/tools/glyphs.mjs <file.pdf> [page…]");
  process.exit(2);
}

const pdfjs = await import("pdfjs-dist/legacy/build/pdf.mjs");
const doc = await pdfjs.getDocument({
  data: new Uint8Array(readFileSync(file)),
  // No worker: this is a script, and the point of a worker is to keep a window
  // responsive.
  useWorkerFetch: false,
  isEvalSupported: false,
}).promise;

const pages = want.length ? want.map(Number) : [...Array(doc.numPages).keys()].map((n) => n + 1);
let carrying = 0;
for (const page of pages) {
  const glyphs = await glyphsOf(doc, page);
  // A page with no text layer prints nothing: it is a page for the OCR queue,
  // and an empty glyph list would read as a page that was looked at and found
  // blank. The two are counted separately on stderr.
  if (glyphs === null) continue;
  carrying += 1;
  console.log(JSON.stringify(glyphs));
}
console.error(
  `${carrying} of ${pages.length} pages carry their own text; ` +
    `${pages.length - carrying} have none and want OCR`,
);
