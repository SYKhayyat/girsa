#!/usr/bin/env node
// Turn an OtzarLib checkout into a library `girsa-import` can read.
//
// OtzarLib is a folder of seforim, not a library: its files sit under
// `ספרים/<its own categories>/`, three of them are `.docx`, one is a
// byte-identical duplicate of its neighbour under a slipped name, and there is
// nothing anywhere saying where any of it came from. `girsa-import` reads a
// tree with an `אוצריא/` in it, and puts a sefer on the shelf **in the folder
// it finds it in** — so where each file lands here is where a reader will look
// for it later.
//
//     node tools/lay-out-otzarlib.mjs <checkout> <destination> [--dry-run]
//
//     git clone --depth 1 https://github.com/gwngdwl/seforim.git otzarlib
//     node tools/lay-out-otzarlib.mjs otzarlib otzarlib-shelf
//     girsa-import      corpus <otzaria> otzarlib-shelf
//     girsa-link-import corpus <otzaria> otzarlib-shelf
//
// # The table below is the part that is judgement rather than mechanism
//
// Moving files is nothing. Knowing that פסקי הרי"ד is a rishon on Shas, that
// קרית מלך is a commentary on the Rambam, and that מאמרי המשגיח is mussar
// rather than the "acharonim" drawer it arrived in — that is the work, and it
// is per-source knowledge that cannot be derived from a filename. It is written
// down here so that it is reviewable, arguable and re-runnable, instead of
// living in whatever shell history laid the shelf out the first time.
//
// The categories are Otzaria's own, because that is the shelf these seforim are
// joining: `שות/ראשונים` already exists and already has seforim in it.
//
// # It never guesses silently
//
// A file the table does not cover is **reported**, not dropped and not swept
// into a default. The report is the point: an unplaced sefer is a decision
// somebody has to make, and the one thing this must not do is make it quietly.

import { createHash } from "node:crypto";
import { inflateRawSync } from "node:zlib";
import {
  cpSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, extname, join, relative } from "node:path";

/** Where a sefer belongs, by the folder OtzarLib filed it under. */
const BY_FOLDER = new Map([
  ["תשובות/גאונים", "שות/גאונים"],
  ["תשובות/ראשונים", "שות/ראשונים"],
  ["תשובות/תשובות אחרונים", "שות/אחרונים"],
  ["תשובות/אחרוני זמננו", "שות/מחברי זמננו"],
  ["ראשונים/תלמוד בבלי", "תלמוד בבלי/ראשונים"],
  ["ראשונים", "הלכה/ראשונים"],
  ["אחרונים/אחרוני זמנינו", "תלמוד בבלי/מחברי זמננו"],
  ["אחרונים", "הלכה/אחרונים"],
]);

/**
 * The ones the folder gets wrong, by filename.
 *
 * A drashos sefer is not halacha and a commentary on the Rambam is not a
 * teshuvos sefer, whatever drawer each arrived in.
 */
const BY_NAME = new Map([
  // Commentaries on the Mishneh Torah.
  ["רבינו חיים הלוי", "הלכה/משנה תורה/מפרשים"],
  ["קרית מלך", "הלכה/משנה תורה/מפרשים"],
  ["חידושי מרן ריז הלוי על הרמבם", "הלכה/משנה תורה/מפרשים"],
  ["שו''ת רדב''ז ללשונות הרמב''ם", "הלכה/משנה תורה/מפרשים"],
  ["מפתח ספה''מ להרמבם - לפי פרשיות התורה", "הלכה/ספרי מצוות"],
  // On the Gemara.
  ["ברכת שמואל על יבמות", "תלמוד בבלי/אחרונים"],
  ["כפות תמרים - ראש השנה", "תלמוד בבלי/אחרונים"],
  ["ספר הישר לר''ת חידושים", "תלמוד בבלי/ראשונים"],
  ["פסקי הרי''ד", "תלמוד בבלי/ראשונים"],
  ["קונטרס הראיות", "תלמוד בבלי/ראשונים"],
  // Mussar and machshava, which arrived filed under acharonim.
  ["מוסר ודעת", "ספרי מוסר/מחברי זמננו"],
  ["מאמרי המשגיח", "ספרי מוסר/מחברי זמננו"],
  ["מגן אבות למאירי", "מחשבת ישראל/ראשונים"],
  ["דרשות מהר''ח או''ז", "תנך/דרשות ודרושים"],
  // Tefillah.
  ["הגדה של פסח לריטב''א", "סדר התפילה/הגדה"],
  ["הגדה של פסח לרשב''ץ", "סדר התפילה/הגדה"],
  ["פירוש סידור תפילה לרוקח", "סדר התפילה/סידור"],
  // Reference.
  ["ערכי תנאים ואמוראים", "ספרות עזר/תולדות עם ישראל"],
  ["מנחת שי - על התורה", "ספרות עזר/דקדוק"],
  // Responsa filed under "rishonim" rather than under "teshuvos".
  ["אגרות הרמ''ה", "שות/ראשונים"],
]);

/** The Encyclopedia, which is a set rather than a folder. */
const ENCYCLOPEDIA = "ספרות עזר/אנציקלופדיות/אנציקלופדיה תלמודית";

/** The Chazon Ish's letters, which arrive as `.docx` named for a drawer. */
const IGROS = "שות/מחברי זמננו";

/**
 * What this library says about itself.
 *
 * **No `license` field, deliberately.** OtzarLib carries no licence and its
 * README says parts of its contents may not be redistributed. A blank is a
 * thing a reader can act on; a guessed licence is not.
 */
const DECLARATION = {
  edition: "OtzarLib",
  provenance: "https://github.com/gwngdwl/seforim",
};

/** Every file under a directory, recursively. */
function* walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === ".git") continue;
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* walk(path);
    else yield path;
  }
}

/**
 * One entry out of a zip, inflated.
 *
 * A `.docx` is a zip and Node has no reader for one. This walks the local file
 * headers rather than the central directory, which is enough for a file written
 * by Word and is forty lines instead of a dependency.
 */
function unzipEntry(buffer, wanted) {
  let at = 0;
  while (at + 30 <= buffer.length) {
    if (buffer.readUInt32LE(at) !== 0x04034b50) break;
    const method = buffer.readUInt16LE(at + 8);
    const compressed = buffer.readUInt32LE(at + 18);
    const uncompressed = buffer.readUInt32LE(at + 22);
    const nameLength = buffer.readUInt16LE(at + 26);
    const extraLength = buffer.readUInt16LE(at + 28);
    const name = buffer.toString("utf8", at + 30, at + 30 + nameLength);
    const start = at + 30 + nameLength + extraLength;
    if (name === wanted) {
      const raw = buffer.subarray(start, start + compressed);
      return method === 0 ? raw : inflateRawSync(raw);
    }
    if (compressed === 0 && uncompressed === 0) break; // sizes are in a trailer
    at = start + compressed;
  }
  return null;
}

/** The paragraphs of a `.docx`, one per line. */
function docxLines(path) {
  const xml = unzipEntry(readFileSync(path), "word/document.xml");
  if (!xml) throw new Error(`${path}: no word/document.xml — is it really a .docx?`);
  const text = xml.toString("utf8");
  const out = [];
  for (const [, paragraph] of text.matchAll(/<w:p[ >]([\s\S]*?)<\/w:p>/g)) {
    const words = [...paragraph.matchAll(/<w:t[^>]*>([\s\S]*?)<\/w:t>/g)].map((m) => m[1]);
    const line = words
      .join("")
      .replaceAll("&amp;", "&")
      .replaceAll("&lt;", "<")
      .replaceAll("&gt;", ">")
      .replaceAll("&quot;", '"')
      .replaceAll("&apos;", "'")
      .trim();
    if (line) out.push(line);
  }
  return out;
}

/** The category for one source file, or `null` if nothing here decides. */
function destination(rel, stem) {
  if (rel.includes("אנציקלופדיה")) {
    return rel.includes("הערות על") ? `${ENCYCLOPEDIA}/הערות` : ENCYCLOPEDIA;
  }
  if (BY_NAME.has(stem)) return BY_NAME.get(stem);

  let folder = rel
    .split(/[\\/]/)
    .slice(0, -1)
    .filter((part) => part !== "ספרים" && part !== "ספרים שאינם מותאמים לאוצריא")
    .join("/");

  // The Vagshal Shas: the same vocalized Bavli Sefaria already supplies. Filed
  // where Otzaria files a masechta so the catalogue recognises the duplicate
  // and skips it rather than shelving a second copy.
  if (folder.startsWith("שס וגשל")) {
    const seder = folder.split("/").slice(1).join("/");
    return seder ? `תלמוד בבלי/${seder}` : "תלמוד בבלי";
  }

  while (folder) {
    if (BY_FOLDER.has(folder)) return BY_FOLDER.get(folder);
    const cut = folder.lastIndexOf("/");
    if (cut < 0) break;
    folder = folder.slice(0, cut);
  }
  return null;
}

function main() {
  const words = process.argv.slice(2).filter((a) => !a.startsWith("--"));
  const dryRun = process.argv.includes("--dry-run");
  if (words.length !== 2) {
    console.error(
      "usage: node tools/lay-out-otzarlib.mjs <checkout> <destination> [--dry-run]\n\n" +
        "  <checkout>     a clone of https://github.com/gwngdwl/seforim\n" +
        "  <destination>  written fresh; anything already there is removed\n\n" +
        "  Read that repository's README before you run this. It states that parts\n" +
        "  of its contents may not be redistributed. Girsa neither fetches nor ships\n" +
        "  them; what you put on your own shelf is between you and those terms.",
    );
    process.exit(2);
  }
  const [source, out] = words;
  const books = join(source, "ספרים");
  try {
    statSync(books);
  } catch {
    console.error(`${books} is not there — is ${source} an OtzarLib checkout?`);
    process.exit(1);
  }

  const placed = [];
  const unplaced = [];
  const duplicates = [];
  const seen = new Map();

  for (const path of walk(books)) {
    const extension = extname(path);
    if (extension !== ".txt" && extension !== ".docx") continue;
    const rel = relative(books, path).split(/[\\/]/).join("/");
    let stem = basename(path, extension);

    let body;
    if (extension === ".docx") {
      body = `${docxLines(path).join("\n")}\n`;
      stem = `קובץ אגרות חזון איש${stem.replace("קובץ אגרות", "").replaceAll("  ", " ").trimEnd()}`;
    } else {
      // LF, whatever the file arrived as. A carriage return is not part of
      // anybody's text, both readers strip it anyway, and leaving it in makes
      // two runs of this produce files that differ without differing.
      body = readFileSync(path, "utf8").replaceAll("\r\n", "\n");
    }

    // Byte-identical to a file already taken, under a name that is plainly a
    // slip — `בעלי התוספות11` beside `בעלי התוספות`. Both would import as
    // separate works with separate permanent ids and nothing would notice.
    const digest = createHash("sha256").update(body).digest("hex");
    if (seen.has(digest)) {
      duplicates.push([stem, seen.get(digest)]);
      continue;
    }
    seen.set(digest, stem);

    const category = extension === ".docx" ? IGROS : destination(rel, stem);
    if (!category) {
      unplaced.push(rel);
      continue;
    }
    placed.push({ category, stem, body, bytes: Buffer.byteLength(body) });
  }

  const links = [...walk(books)].filter((p) => p.endsWith("_links.json"));

  if (!dryRun) {
    rmSync(out, { recursive: true, force: true });
    for (const { category, stem, body } of placed) {
      const dir = join(out, "אוצריא", ...category.split("/"));
      mkdirSync(dir, { recursive: true });
      writeFileSync(join(dir, `${stem}.txt`), body, "utf8");
    }
    const linksDir = join(out, "links");
    mkdirSync(linksDir, { recursive: true });
    for (const path of links) cpSync(path, join(linksDir, basename(path)));
    writeFileSync(
      join(out, "library.json"),
      `${JSON.stringify(DECLARATION, null, 2)}\n`,
      "utf8",
    );
  }

  const byCategory = new Map();
  for (const item of placed) {
    if (!byCategory.has(item.category)) byCategory.set(item.category, []);
    byCategory.get(item.category).push(item);
  }
  const megabytes = placed.reduce((n, p) => n + p.bytes, 0) / 1048576;
  console.log(
    `${dryRun ? "would place" : "placed"} ${placed.length} seforim in ` +
      `${byCategory.size} categories, ${links.length} links sidecars, ` +
      `${megabytes.toFixed(1)} MB`,
  );
  for (const category of [...byCategory.keys()].sort()) {
    console.log(`\n${category}  (${byCategory.get(category).length})`);
    for (const item of byCategory.get(category).sort((a, b) => a.stem.localeCompare(b.stem))) {
      console.log(`    ${(item.bytes / 1024).toFixed(0).padStart(7)} KB  ${item.stem}`);
    }
  }
  for (const [copy, original] of duplicates) {
    console.log(`\nskipped ${copy} — byte-identical to ${original}`);
  }
  if (unplaced.length > 0) {
    console.log(
      `\n${unplaced.length} file(s) this script has no category for. Nothing was ` +
        `written for them; add a row to BY_NAME or BY_FOLDER and run it again:`,
    );
    for (const rel of unplaced) console.log(`    ${rel}`);
    process.exitCode = 1;
  }
}

main();
