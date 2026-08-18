// Paper.
//
// > *Print the daf for the shiur.*
//
// Before this the answer was: export to `.docx`, open Word, print from there.
// Three applications and a file on a disk for the thing a bachur wants at seven
// in the morning.
//
// # Why the page prints itself
//
// The window already knows how to draw a line of a sefer — the corrections
// applied, the pointing the reader chose, the shemos written the way they
// asked, the dibur hamatchil bold. A printer path that built its own idea of a
// line would be a second answer to *what does this sefer say*, and the two
// would drift. So this draws the same `Line`s with the same function the pane
// uses, into a sheet the print stylesheet is the only thing that shows.
//
// `window.print()` and not a PDF writer: the webview has a print dialogue on
// all three platforms, it is the one the reader's own printer is set up in, and
// *print to PDF* is a destination in it. Writing a PDF here would mean shipping
// a typesetter to get worse output than the one already installed.
//
// # What is on the sheet besides the words
//
// The sefer, the printed edition and the terms. spec.md §13 asks every text to
// carry its provenance and a page leaving the application is where that has to
// be true outside it — `girsa-export` puts the same four lines at the head of a
// `.docx`, and a sheet of paper is a file leaving by another road.

import { api, type Sheet } from "./api.ts";
import { lineElement } from "./pane.ts";

/** Where the sheet is built. One, reused, and empty the rest of the time. */
let sheet: HTMLElement | null = null;

/**
 * The address on the sheet: one when it is one line, two when it is a run.
 *
 * The whole of the decision, and the reason it is worth naming is the case in
 * the middle. `printSection` reads the first line's address and the last line's
 * address off a run, and a section of **one** se'if has the same address twice
 * — so the unguarded form prints `סימן א׳ סעיף א׳ — סימן א׳ סעיף א׳`, which is
 * not wrong so much as it is a sheet that looks like a mistake. It is also what
 * comes back for the section that is a single line, which is not rare: a siman
 * of one se'if, a perek of one mishnah.
 *
 * It is returned to the caller as well as printed, because a print dialogue
 * opening over the window tells the reader *something* happened and not *what*
 * — and on a machine whose printer is a PDF writer, the file lands somewhere
 * with a name nobody chose.
 */
export function sheetWhere(address: string, toAddress: string): string {
  return address === toAddress ? address : `${address} — ${toAddress}`;
}

function surface(): HTMLElement {
  if (sheet) return sheet;
  sheet = document.createElement("article");
  sheet.className = "print-sheet";
  // Out of the reading order and out of the tab order while it is not being
  // printed. `hidden` would be simpler and `@media print` cannot unhide it.
  sheet.setAttribute("aria-hidden", "true");
  document.body.append(sheet);
  return sheet;
}

/**
 * Put the section a line is in onto paper.
 *
 * Returns what it printed, so the caller can say it out loud — a print dialogue
 * that opens over the window tells the reader *something* happened and not
 * *what*, and on a machine whose printer is a PDF writer the file lands
 * somewhere with a name nobody chose.
 */
export async function printSection(at: string): Promise<string> {
  const found: Sheet = await api.seferSheet(at, false);
  const page = surface();
  page.replaceChildren();

  const head = document.createElement("header");
  head.className = "print-head";
  for (const [nth, line] of found.title.entries()) {
    const row = document.createElement("p");
    // The first line is the sefer and the rest is the apparatus — the edition
    // and the terms — which are set small so they are on the page and not in
    // the way of it.
    row.className = nth === 0 ? "print-title" : "print-provenance";
    row.textContent = line;
    head.append(row);
  }
  const where = document.createElement("p");
  where.className = "print-where";
  where.textContent = sheetWhere(found.address, found.to_address);
  head.append(where);
  page.append(head);

  for (const line of found.lines) page.append(...lineElement(line));

  // The class is what the print stylesheet keys off, and it comes off again
  // whether the dialogue was accepted or dismissed — `print()` returns when the
  // dialogue closes, both ways.
  document.body.classList.add("is-printing");
  try {
    window.print();
  } finally {
    document.body.classList.remove("is-printing");
  }
  return where.textContent;
}

/** Forget the sheet. For a language switch, which rebuilds every panel. */
export function forgetSheet(): void {
  sheet?.remove();
  sheet = null;
}
