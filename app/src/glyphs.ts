// What a PDF says its own words are — asked of the file, not of a picture of it.
//
// spec.md §6.3 and §9.7, W26. A scan arrives on the shelf with one segment per
// page and no words, because the importer will not invent Hebrew it cannot read
// (`girsa_corpus::import::mine`). There are two ways to fill those pages in, and
// this is the one that is exact: **a PDF that was typeset rather than
// photographed carries its own text**, and asking it needs no model, no
// process and no guess. OCR is what happens to the pages that have none.
//
// # Why the words are not simply read off
//
// Because a PDF does not have words. It has drawing instructions, and a Hebrew
// sefer typeset properly positions every letter and every nikud mark separately
// so that the marks sit where the typesetter wanted them. Ask such a file for
// its text and it answers `ֵמ ֵא יָמ ַת י` — a space between the halves of every
// letter — because the extractor puts one wherever the pen jumped, and half of
// those jumps are inside a word.
//
// So this hands back **glyphs and rectangles**, and the words are worked out
// from the geometry in `girsa-scan`, where the rule is measured and tested. The
// spaces the file supplies are ignored entirely, which is what makes the same
// code group a text layer and an engine that returns loose glyphs.
//
// # And why it is here rather than in Rust
//
// The same reason `scanview.ts` draws the page rather than the Rust half doing
// it: pdf.js is one renderer on all three platforms, it is already bundled, and
// it is the thing that decides what a page of this file looks like. A second
// PDF stack in Rust would be a second opinion about the same file — which is
// exactly what W25 refused when it declined the webview's built-in viewer.

import type { PDFDocumentProxy } from "pdfjs-dist";

/** One drawn glyph, in pixels of a page rendered at scale 1. */
export interface Glyph {
  text: string;
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** A page's glyphs, with the size of the page they were measured on. */
export interface PageGlyphs {
  page: number;
  width: number;
  height: number;
  glyphs: Glyph[];
}

/**
 * Every glyph on a page, or `null` where the page carries no text at all.
 *
 * `null` is the answer that matters: it is a page that has to be OCR'd, and it
 * is a different thing from a page that was read and turned out to be blank.
 * Handing back an empty list for both would make a scan of a photographed sefer
 * look like a sefer of blank pages — searched, found empty, and never queued.
 */
export async function glyphsOf(
  doc: PDFDocumentProxy,
  page: number,
): Promise<PageGlyphs | null> {
  const it = await doc.getPage(page);
  const viewport = it.getViewport({ scale: 1 });
  // `includeMarkedContent` off: the marked-content spans are structure, not
  // ink, and they arrive interleaved with the glyphs carrying no position.
  const content = await it.getTextContent({ includeMarkedContent: false });

  const glyphs: Glyph[] = [];
  for (const item of content.items) {
    if (!("str" in item)) continue;
    if (item.str === "") continue;
    // transform is [a, b, c, d, e, f] with (e, f) the baseline origin in PDF
    // space, which counts up from the bottom of the page. The window draws
    // top-down, and so does everything downstream of here.
    const [, , , d, e, f] = item.transform;
    const height = Math.abs(item.height || d) || Math.abs(d);
    const top = viewport.height - f - height;
    glyphs.push({
      text: item.str,
      left: e,
      top,
      right: e + item.width,
      bottom: top + height,
    });
  }
  if (glyphs.length === 0) return null;
  return { page, width: viewport.width, height: viewport.height, glyphs };
}
