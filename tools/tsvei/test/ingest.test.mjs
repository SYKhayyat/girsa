// The rows, against a real Postgres.
//
// The subject of this file is *the database*, so a mock would test nothing: the
// generated tsvector, the vector column and the upsert are all Postgres
// behaviour. `TSVEI_TEST_DB` names a scratch database — see README.md for the
// one-line `nix-shell` that provides one — and without it the whole file reports
// itself skipped rather than passing on nothing.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const tool = join(here, "..");
const db = process.env.TSVEI_TEST_DB;

const skip = db
  ? false
  : "set TSVEI_TEST_DB to a scratch database (see README.md) — the subject here is Postgres itself";

let client;

function run(args) {
  return execFileSync(process.execPath, [join(tool, "ingest.mjs"), ...args], {
    encoding: "utf8",
    env: { ...process.env, TSVEI_DATABASE_URL: db, DATABASE_URL: "" },
  });
}

async function pg() {
  if (!client) {
    const { default: Pg } = await import("pg");
    client = new Pg.Client({ connectionString: db });
    await client.connect();
    await client.query(readFileSync(join(tool, "schema.sql"), "utf8"));
    await client.query("truncate tsvei_verse, chat_message");
  }
  return client;
}

test("the pointed text is findable by an unpointed query", { skip }, async () => {
  await pg();
  await run(["verses", join(here, "..", "fixtures", "verses.jsonl"), "--embed", "fake"]);
  const { rows } = await (
    await pg()
  ).query(
    "select ref_he from tsvei_verse where fts @@ plainto_tsquery('simple', $1) order by ref_he",
    ["אלהים"],
  );
  assert.equal(rows.length, 1, "`אלהים` should find `אֱלֹהִים`");
  assert.match(rows[0].ref_he, /^בראשית/);
});

test("the same verse in two notes is one row, the longer text, and both sources", { skip }, async () => {
  const { rows } = await (
    await pg()
  ).query(
    "select text, sources, text_plain, embedding is not null as has_vector " +
      "from tsvei_verse where book = $1 and chapter = 1 and verse = 1",
    ["בראשית"],
  );
  assert.equal(rows.length, 1, "one verse is one row");
  assert.match(rows[0].text, /ועוד דברים/, "the longer of the two notes wins");
  assert.deepEqual([...rows[0].sources].sort(), [
    "vault/parsha/bereshit-2.md",
    "vault/parsha/bereshit.md",
  ]);
  assert.equal(rows[0].text_plain, "בראשית ברא אלהים את השמים ואת הארץ. ועוד דברים שלא היו בטעות הקודמת.");
  assert.equal(rows[0].has_vector, true);
});

test("importing twice changes nothing", { skip }, async () => {
  const before = (await (await pg()).query("select count(*)::int as n from tsvei_verse")).rows[0].n;
  await run(["verses", join(here, "..", "fixtures", "verses.jsonl"), "--embed", "fake"]);
  const after = (await (await pg()).query("select count(*)::int as n from tsvei_verse")).rows[0].n;
  assert.equal(after, before, "a second import is a no-op");
});

test("the nearest verse to a verse is itself", { skip }, async () => {
  const { rows } = await (
    await pg()
  ).query(
    `select ref_he
       from tsvei_verse
      order by embedding <=> (select embedding from tsvei_verse
                               where book = $1 and chapter = 1 and verse = 1)
      limit 1`,
    ["בראשית"],
  );
  assert.match(rows[0].ref_he, /א׳, א׳$/, `nearest was ${rows[0].ref_he}`);
});

test("a conversation becomes turns, blank ones left out, and a re-import adds nothing", { skip }, async () => {
  await run(["chat", join(here, "..", "fixtures", "chat.json"), "--embed", "fake"]);
  const first = (await (await pg()).query("select count(*)::int as n from chat_message")).rows[0].n;
  assert.equal(first, 2, "the blank assistant turn is not a turn");
  await run(["chat", join(here, "..", "fixtures", "chat.json"), "--embed", "fake"]);
  const second = (await (await pg()).query("select count(*)::int as n from chat_message")).rows[0].n;
  assert.equal(second, first);
});

test("an empty file is a failure and not a successful import of nothing", { skip }, async () => {
  const empty = join(here, "empty.jsonl");
  const { writeFileSync, unlinkSync } = await import("node:fs");
  writeFileSync(empty, "\n");
  try {
    assert.throws(() => run(["verses", empty, "--embed", "fake"]), /held no verses/);
  } finally {
    unlinkSync(empty);
  }
});

test.after(async () => {
  if (client) await client.end();
});
