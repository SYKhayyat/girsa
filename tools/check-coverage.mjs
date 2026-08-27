// The coverage manifest, checked against the tree.
//
//     node tools/check-coverage.mjs
//
// # What it is
//
// `tools/coverage-manifest.json` is the map of every surface of the tree —
// every crate, every tool, the shell, the window — to where it is tested, and
// of every highest-risk invariant to the test that pins it. A map nobody
// checks is a map that rots, and the whole argument of this repository is that
// prose that is not read becomes wrong. So the gate runs this file, and this
// file reads the tree and demands that the manifest's claims be true:
//
// * every surface's named test file or directory exists,
// * every invariant's pinning test exists, and
// * the pinning test's **body** actually mentions the thing it pins — the
//   `keyword` — so a test that names its subject in its title and then tests
//   something else entirely is caught as the vacuous pass it is.
//
// The strongest form of that last check is `tools/mutation.mjs`, which breaks
// each guard and demands the test fail. It is not run by the gate because it
// recompiles — it is the dynamic half, run when a mutation of the invariant is
// suspected or when this manifest is edited.
//
// # What it reports
//
// Two sections the reader of a green run is owed, printed every run:
//
// * the **uncovered** rows — surfaces the audits said were sampled or thin,
//   with an honest status, and
// * the **platform-gated** rows — things that need a Mac, a browser, a real
//   tesseract or Ksav's pen, reported as *skipped with what they need* rather
//   than as passed. Issue #5's acceptance criterion is exactly that: an
//   unsupported path is a row that says what it needs, never a silent green.
//
// # Exit
//
// 0 when the manifest matches the tree and every pinning test mentions its
// subject. 1 when a claim has rotted — a named file gone, a test renamed, a
// guard rewritten so the mutation in the manifest no longer applies, or a
// pinning test whose body has stopped mentioning what it pins.

import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");

const manifest = JSON.parse(
  await readFile(path.join(HERE, "coverage-manifest.json"), "utf8"),
);
const prose = await readFile(path.join(ROOT, "docs", "coverage.md"), "utf8");

const problems = [];
const named = { files: 0, invariants: 0 };

/** The body of `fn <name>(` — from its line to the next top-level `fn `. */
async function bodyOf(file, name) {
  const source = await readFile(file, "utf8");
  const at = source.indexOf(`fn ${name}(`);
  if (at < 0) return null;
  const rest = source.slice(at);
  const next = rest.search(/\n[ \t]*fn /);
  return next < 0 ? rest : rest.slice(0, next);
}

for (const surface of manifest.surfaces) {
  for (const entry of surface.tests) {
    const at = path.join(ROOT, entry);
    try {
      await stat(at);
      named.files += 1;
    } catch {
      problems.push(`${surface.name}: tests entry gone — ${entry}`);
    }
  }
}

for (const invariant of manifest.invariants) {
  named.invariants += 1;
  const source = path.join(ROOT, invariant.source);
  let sourceText;
  try {
    sourceText = await readFile(source, "utf8");
  } catch {
    problems.push(`${invariant.name}: source gone — ${invariant.source}`);
    continue;
  }
  // The mutation in the manifest must still apply: a guard that was rewritten
  // is a mutation the manifest does not understand, and that has to be said
  // rather than silently dropping the invariant from the table.
  if (!sourceText.includes(invariant.find)) {
    problems.push(
      `${invariant.name}: its guard is not in ${invariant.source} — ` +
        `find ${JSON.stringify(invariant.find)}`,
    );
  }
  const testFile = path.join(ROOT, invariant.test.file);
  const body = await bodyOf(testFile, invariant.test.fn);
  if (body === null) {
    problems.push(
      `${invariant.name}: pinning test gone — ${invariant.test.file}::` +
        `${invariant.test.fn}`,
    );
    continue;
  }
  // The body, not the title: a keyword that only lives in the fn name is a
  // test that does not mention its subject where it tests it.
  if (!body.includes(invariant.test.keyword)) {
    problems.push(
      `${invariant.name}: pinning test body no longer mentions ` +
        `${JSON.stringify(invariant.test.keyword)} — ` +
        `${invariant.test.file}::${invariant.test.fn}`,
    );
  }
  // And the prose must still name it: docs/coverage.md and this manifest are
  // two renderings of one table, and the one nobody edits is the one that
  // drifts.
  if (!prose.includes(invariant.name)) {
    problems.push(
      `${invariant.name}: docs/coverage.md no longer names this invariant`,
    );
  }
}
for (const surface of manifest.surfaces) {
  if (!prose.includes(surface.name)) {
    problems.push(
      `${surface.name}: docs/coverage.md no longer names this surface`,
    );
  }
}

const lines = [];
lines.push(`coverage manifest — ${manifest.surfaces.length} surfaces, ` +
  `${manifest.invariants.length} invariants, ${named.files} named test files`);
lines.push("");

if (manifest.uncovered.length > 0) {
  lines.push("not swept, and said so:");
  for (const row of manifest.uncovered) {
    lines.push(`  - ${row.surface}: ${row.status}`);
  }
  lines.push("");
}
if (manifest.platform_gated.length > 0) {
  lines.push("platform-gated — skipped, with what they need:");
  for (const row of manifest.platform_gated) {
    lines.push(`  - ${row.surface}: ${row.status} (needs: ${row.requires})`);
  }
  lines.push("");
}

if (problems.length > 0) {
  for (const problem of problems) {
    lines.push(`✗ ${problem}`);
  }
  lines.push(`\n✗ the coverage manifest has rotted — ${problems.length} claim(s) do not match the tree`);
  console.log(lines.join("\n"));
  process.exit(1);
}

lines.push("✓ every surface is where the manifest says, and every pinning test mentions what it pins");
lines.push("  the dynamic half — break each guard, demand the test fail — is `node tools/mutation.mjs`");
console.log(lines.join("\n"));
