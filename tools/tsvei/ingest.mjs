// Put the text in the database.
//
// The shape is deliberately dull: read a JSONL file of records, say what is in
// it, write it with an upsert that is safe to run twice. Everything interesting
// (what a note means, where a verse starts) happened in `parse.mjs`; everything
// interesting about search (which words, which vectors) happens in `schema.sql`.
// This file is the pipe between them and nothing else, because a pipe that also
// decides things is the pipe nobody trusts twice.
//
//   node ingest.mjs verses chunks.jsonl [--dry]
//   node ingest.mjs chat export.json     [--dry]
//
// `DATABASE_URL` is where the rows go. `--embed fake` is for tests: it writes a
// deterministic vector so the vector column, the index and the query path are
// all exercised without a network call. See README.md.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

import { unpointed } from "./hebrew.mjs";

const EMBED_DIM = 1536;
const BATCH = 64;

// ── what the database is ─────────────────────────────────────────────────────

function databaseUrl() {
  const url = process.env.TSVEI_DATABASE_URL || process.env.DATABASE_URL;
  if (!url) {
    throw new Error(
      "no database: set TSVEI_DATABASE_URL (or DATABASE_URL) to the Postgres " +
        "connection string, e.g. the one `neon env pull` writes",
    );
  }
  return url;
}

async function connect() {
  const { default: pg } = await import("pg");
  const client = new pg.Client({
    connectionString: databaseUrl(),
    // Neon and most managed Postgres terminate TLS with a certificate the local
    // trust store does not hold. The connection is still encrypted; the
    // alternative is refusing to talk to the database at all.
    ssl: /neon\.tech|sslmode=require/.test(databaseUrl())
      ? { rejectUnauthorized: false }
      : undefined,
  });
  await client.connect();
  return client;
}

// ── embeddings ──────────────────────────────────────────────────────────────

/** A deterministic 1536-vector from the text. Tests only; see README. */
function fakeVector(text) {
  const out = new Array(EMBED_DIM);
  for (let i = 0; i < EMBED_DIM; i += 1) {
    const h = createHash("sha256").update(`${i}\u0000${text}`).digest();
    out[i] = (h.readUInt32BE(0) / 0xffffffff) * 2 - 1;
  }
  const norm = Math.hypot(...out) || 1;
  return out.map((x) => x / norm);
}

/**
 * The embedding for a batch of texts, or null when nothing is configured.
 *
 * One OpenAI-compatible endpoint is all this needs, which is the whole reason
 * the URL is a variable rather than a constant: Neon's AI Gateway, OpenAI and
 * anything else that answers `/embeddings` are the same three settings, and
 * choosing between them is not a code change.
 */
function embedder(kind) {
  if (kind === "fake") return async (texts) => texts.map(fakeVector);
  if (kind === "none") return null;
  const url = process.env.TSVEI_EMBED_URL;
  const key = process.env.TSVEI_EMBED_KEY;
  const model = process.env.TSVEI_EMBED_MODEL || "text-embedding-3-small";
  if (!url) return null;
  return async (texts) => {
    const res = await fetch(url.replace(/\/$/, "") + "/embeddings", {
      method: "POST",
      headers: {
        "content-type": "application/json",
        ...(key ? { authorization: `Bearer ${key}` } : {}),
      },
      body: JSON.stringify({ model, input: texts }),
    });
    if (!res.ok) {
      throw new Error(`embeddings ${res.status}: ${(await res.text()).slice(0, 400)}`);
    }
    const body = await res.json();
    const byIndex = new Map((body.data ?? []).map((d) => [d.index, d.embedding]));
    return texts.map((_, i) => {
      const v = byIndex.get(i);
      if (!v) throw new Error(`the embedder answered ${body.data?.length ?? 0} of ${texts.length}`);
      if (v.length !== EMBED_DIM) {
        throw new Error(
          `a vector of ${v.length} for a column of ${EMBED_DIM} — set TSVEI_EMBED_MODEL ` +
            `to one that answers ${EMBED_DIM}, or change schema.sql`,
        );
      }
      return v;
    });
  };
}

async function embedAll(embed, texts) {
  if (!embed) return texts.map(() => null);
  const out = [];
  for (let i = 0; i < texts.length; i += BATCH) {
    const slice = texts.slice(i, i + BATCH);
    out.push(...(await embed(slice)));
  }
  return out;
}

// ── verses ──────────────────────────────────────────────────────────────────

/**
 * One row per pasuk, the longest text winning.
 *
 * A vault can hold the same verse twice — two notes on one parsha, or a note
 * that quotes a passage its neighbour also covers. Both are the same verse, and
 * a database that stored two rows would answer a search for it twice and count
 * it twice in every report. So the merge happens here, before the write, the way
 * it would happen in the reader's head.
 */
export function mergeVerses(records) {
  const seen = new Map();
  for (const r of records) {
    for (const field of ["book", "chapter", "verse", "text"]) {
      if (r[field] === undefined || r[field] === null || r[field] === "") {
        throw new Error(`a verse record has no ${field}: ${JSON.stringify(r).slice(0, 300)}`);
      }
    }
    const chapter = Number(r.chapter);
    const verse = Number(r.verse);
    if (!Number.isInteger(chapter) || chapter < 1) {
      throw new Error(`chapter ${r.chapter} is not a chapter number (${r.book} ${r.verse})`);
    }
    if (!Number.isInteger(verse) || verse < 1) {
      throw new Error(`verse ${r.verse} is not a verse number (${r.book} ${r.chapter})`);
    }
    const key = `${r.book}\u0000${chapter}\u0000${verse}`;
    const source = r.source ?? r.sources?.[0] ?? null;
    const had = seen.get(key);
    if (!had) {
      seen.set(key, {
        book: r.book,
        book_en: r.book_en ?? null,
        chapter,
        verse,
        ref_he: r.ref_he ?? `${r.book} ${chapter}:${verse}`,
        text: r.text.trim(),
        text_plain: unpointed(r.text),
        sources: source ? [source] : [],
        from: 1,
      });
    } else {
      had.from += 1;
      if (r.text.trim().length > had.text.length) {
        had.text = r.text.trim();
        had.text_plain = unpointed(r.text);
      }
      if (source && !had.sources.includes(source)) had.sources.push(source);
      if (!had.book_en && r.book_en) had.book_en = r.book_en;
    }
  }
  return [...seen.values()].sort(
    (a, b) => a.book.localeCompare(b.book, "he") || a.chapter - b.chapter || a.verse - b.verse,
  );
}

async function writeVerses(client, rows, embed, { dry }) {
  const vectors = await embedAll(
    embed,
    rows.map((r) => `${r.ref_he}\n${r.text}`),
  );
  if (dry) return rows.length;
  const statement = `
    insert into tsvei_verse
      (book, book_en, chapter, verse, ref_he, text, text_plain, sources, embedding, embedded_at)
    values ($1, $2, $3, $4, $5, $6, $7, $8, $9::vector,
            case when $9::text is null then null else now() end)
    on conflict (book, chapter, verse) do update set
      book_en   = coalesce(excluded.book_en, tsvei_verse.book_en),
      ref_he    = excluded.ref_he,
      text      = case when length(excluded.text) > length(tsvei_verse.text)
                       then excluded.text else tsvei_verse.text end,
      text_plain = case when length(excluded.text) > length(tsvei_verse.text)
                        then excluded.text_plain else tsvei_verse.text_plain end,
      sources   = array(select distinct unnest(tsvei_verse.sources || excluded.sources)),
      embedding = coalesce(excluded.embedding, tsvei_verse.embedding),
      embedded_at = case when excluded.embedding is null then tsvei_verse.embedded_at
                         else now() end,
      updated_at = now()`;
  for (let i = 0; i < rows.length; i += BATCH) {
    const slice = rows.slice(i, i + BATCH);
    await client.query("begin");
    try {
      for (let j = 0; j < slice.length; j += 1) {
        const r = slice[j];
        const v = vectors[i + j];
        await client.query(statement, [
          r.book,
          r.book_en,
          r.chapter,
          r.verse,
          r.ref_he,
          r.text,
          r.text_plain,
          r.sources,
          v ? `[${v.join(",")}]` : null,
        ]);
      }
      await client.query("commit");
    } catch (e) {
      await client.query("rollback");
      throw e;
    }
  }
  return rows.length;
}

// ── conversations ───────────────────────────────────────────────────────────

/**
 * Every turn of an exported conversation, as rows.
 *
 * The export formats differ — one JSON array of {role, content}, one object per
 * thread, one with `messages` nested inside a `mapping` — so this reads the two
 * shapes that carry the same three facts (who, when, what) rather than guessing
 * a vendor. `digest` is what makes a second import a no-op.
 */
export function turnsOf(exported) {
  const threads = [];
  const push = (thread, messages) => {
    if (Array.isArray(messages)) threads.push({ thread, messages });
  };
  if (Array.isArray(exported)) {
    push(exported.title ?? exported.name ?? null, exported);
  } else if (Array.isArray(exported?.messages)) {
    push(exported.title ?? exported.name ?? exported.id ?? null, exported.messages);
  } else if (Array.isArray(exported?.threads)) {
    for (const t of exported.threads) push(t.title ?? t.id ?? null, t.messages);
  } else if (Array.isArray(exported?.conversations)) {
    for (const t of exported.conversations) push(t.title ?? t.id ?? null, t.messages);
  } else if (exported?.mapping && typeof exported.mapping === "object") {
    // The ChatGPT export: a tree of nodes, of which the message-bearing ones in
    // `create_time` order are the conversation as anybody read it.
    const nodes = Object.values(exported.mapping)
      .map((n) => n?.message)
      .filter((m) => m && m.author && Array.isArray(m.content?.parts))
      .sort((a, b) => (a.create_time ?? 0) - (b.create_time ?? 0));
    push(
      exported.title ?? null,
      nodes.map((m) => ({
        role: m.author.role,
        content: m.content.parts.filter((p) => typeof p === "string").join("\n"),
        created_at: m.create_time,
      })),
    );
  }
  const out = [];
  for (const { thread, messages } of threads) {
    for (const m of messages) {
      const text = typeof m === "string" ? m : textOf(m.content ?? m.text ?? m.parts);
      if (!text || !text.trim()) continue;
      const role = m.role ?? m.author?.role ?? "unknown";
      const at = m.created_at ?? m.create_time ?? m.timestamp ?? m.time ?? null;
      out.push({
        thread,
        role,
        said_at: at === null ? null : new Date(typeof at === "number" ? at * 1000 : at),
        text: text.trim(),
        text_plain: unpointed(text),
        digest: createHash("sha256")
          .update(`${thread ?? ""}\u0000${role}\u0000${at ?? ""}\u0000${text.trim()}`)
          .digest("hex"),
      });
    }
  }
  return out;
}

function textOf(content) {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content
      .map((p) => (typeof p === "string" ? p : p?.text ?? ""))
      .filter(Boolean)
      .join("\n");
  }
  if (content && typeof content === "object") return content.text ?? "";
  return "";
}

async function writeTurns(client, rows, embed, { dry }) {
  const vectors = await embedAll(embed, rows.map((r) => r.text));
  if (dry) return rows.length;
  const statement = `
    insert into chat_message
      (thread, role, said_at, text, text_plain, digest, embedding, embedded_at)
    values ($1, $2, $3, $4, $5, $6, $7::vector,
            case when $7::text is null then null else now() end)
    on conflict (digest) do update set
      embedding = coalesce(chat_message.embedding, excluded.embedding),
      embedded_at = case when chat_message.embedding is null and excluded.embedding is not null
                         then now() else chat_message.embedded_at end`;
  for (let i = 0; i < rows.length; i += BATCH) {
    const slice = rows.slice(i, i + BATCH);
    await client.query("begin");
    try {
      for (let j = 0; j < slice.length; j += 1) {
        const r = slice[j];
        const v = vectors[i + j];
        await client.query(statement, [
          r.thread,
          r.role,
          r.said_at,
          r.text,
          r.text_plain,
          r.digest,
          v ? `[${v.join(",")}]` : null,
        ]);
      }
      await client.query("commit");
    } catch (e) {
      await client.query("rollback");
      throw e;
    }
  }
  return rows.length;
}

// ── the command ─────────────────────────────────────────────────────────────

function arg(name, fallback) {
  const i = process.argv.indexOf(`--${name}`);
  return i === -1 ? fallback : process.argv[i + 1];
}

function flag(name) {
  return process.argv.includes(`--${name}`);
}

function jsonl(path) {
  return readFileSync(path, "utf8")
    .split("\n")
    .filter((l) => l.trim())
    .map((l, i) => {
      try {
        return JSON.parse(l);
      } catch (e) {
        throw new Error(`${path}:${i + 1} is not JSON: ${e.message}`);
      }
    });
}

async function main() {
  const kind = process.argv[2];
  const file = process.argv[3];
  if (!kind || !file) {
    console.error(
      "usage: node ingest.mjs verses <chunks.jsonl> | chat <export.json> [--dry] [--embed fake|none]",
    );
    process.exit(2);
  }
  const dry = flag("dry");
  const embed = embedder(arg("embed", "env"));
  const schema = flag("schema");

  const rows =
    kind === "verses"
      ? mergeVerses(jsonl(file))
      : kind === "chat"
        ? turnsOf(JSON.parse(readFileSync(file, "utf8")))
        : null;
  if (!rows) {
    console.error(`unknown kind ${kind}: verses or chat`);
    process.exit(2);
  }
  if (rows.length === 0) {
    // A silent success is how an import of nothing looks like an import.
    console.error(`${file} held no ${kind === "verses" ? "verses" : "turns"}`);
    process.exit(1);
  }

  const what = embed ? "with embeddings" : "without embeddings";
  if (!embed && !dry) {
    console.error(
      "note: no embedder configured (TSVEI_EMBED_URL), so the text is loaded and " +
        "searchable by word but not by meaning",
    );
  }
  if (dry) {
    console.log(`${rows.length} rows from ${file} ${what} — nothing written (--dry)`);
    return;
  }

  const client = await connect();
  try {
    if (schema) await client.query(readFileSync(new URL("./schema.sql", import.meta.url), "utf8"));
    const written =
      kind === "verses"
        ? await writeVerses(client, rows, embed, { dry })
        : await writeTurns(client, rows, embed, { dry });
    const table = kind === "verses" ? "tsvei_verse" : "chat_message";
    const { rows: counted } = await client.query(
      `select count(*)::int as n from ${table}` +
        (kind === "verses" ? "" : " where true"),
    );
    console.log(`${written} rows from ${file} ${what}; ${table} now holds ${counted[0].n}`);
  } finally {
    await client.end();
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
