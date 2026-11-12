# tsvei — the verses, in a database

Two things arrive here as files and leave as rows: **verses** (a pasuk, the sefer
and chapter it belongs to, and the note it came from) and **conversations** (a
turn, who said it, when). Both are useful by word and by meaning, so both tables
carry a full-text index and a vector, and Postgres answers both questions.

```
tools/tsvei/
  schema.sql     the two tables, their indexes, and the extensions they need
  hebrew.mjs     the vowel points come off; gematria for a reference
  ingest.mjs     JSONL (or a chat export) → rows, in batches, twice-safe
  fixtures/      a four-line verse file and a three-turn conversation
  test/          11 tests; the database ones run against real Postgres
```

## Why two columns of text

A sefer's text is written `בְּרֵאשִׁית` and read `בראשית`. A tsvector built from
the pointed text holds one token that an unpointed query does not match, so every
Hebrew search silently returns nothing — the failure looks exactly like a
database with no rows in it. So `text` is the verse as printed and `text_plain`
is the same verse with nikkud and te'amim removed, and the index is on
`text_plain`. The maqaf (`־`) is in the same Unicode block but is *not* a
diacritic: it becomes a space, because a space is what a reader searching
`בית־המקדש` types.

## Setting a database up

A scratch one for the tests, without touching Neon:

```
nix-shell -p 'postgresql_16.withPackages (ps: [ ps.pgvector ])' --run '
  initdb -D /tmp/tsvei-pg -A trust -U postgres
  pg_ctl -D /tmp/tsvei-pg -o "-p 55432 -k /tmp" -l /tmp/tsvei-pg.log start
  psql -h /tmp -p 55432 -U postgres -c "create database tsvei"
  psql -h /tmp -p 55432 -U postgres -d tsvei -f tools/tsvei/schema.sql'

TSVEI_TEST_DB="postgresql://postgres@localhost:55432/tsvei" npm test
```

The real one is whatever `neon env pull` writes:

```
export TSVEI_DATABASE_URL="$(...)"     # or DATABASE_URL
node ingest.mjs verses chunks.jsonl --schema --embed env
```

## Embeddings

One OpenAI-compatible endpoint, three variables, because the choice between Neon's
AI Gateway, OpenAI and anything else that answers `/embeddings` should not be a
code change:

| variable | meaning |
|---|---|
| `TSVEI_EMBED_URL` | the base URL; `/embeddings` is appended |
| `TSVEI_EMBED_KEY` | bearer token, if the endpoint wants one |
| `TSVEI_EMBED_MODEL` | default `text-embedding-3-small` |

The column is `vector(1536)`. A model that answers a different width is refused
rather than truncated, with the two ways out in the message.

With `TSVEI_EMBED_URL` unset the text still loads and is still searchable by
word; `ingest.mjs` says so rather than pretending the vectors are there.
`--embed fake` writes a deterministic vector from the text — for tests, and for
exercising the vector column and the HNSW index without a network call.

## The JSONL a verse is

One object per line, and only `book`, `chapter`, `verse` and `text` are required:

```json
{"book":"בראשית","book_en":"Bereshit","chapter":1,"verse":1,
 "ref_he":"בראשית א׳, א׳","text":"בְּרֵאשִׁית בָּרָא אֱלֹהִים…",
 "source":"vault/parsha/bereshit.md"}
```

Two notes covering one pasuk are **one row**: the longer text wins and both
sources are kept in `sources`, so a search does not answer twice and a report
does not count twice. `ref_he` is `hebrew.mjs`'s `gematria`, not a copy pasted
from somewhere.

## What is not here yet

`parse.mjs` — the reading of the vault itself — is the one piece that waits for
the notes. It needs one real file: which tag marks a book, which marks a verse,
and where a verse's text stops are questions the vault answers and a guess does
not. Everything downstream of it is `chunks.jsonl`, which the fixtures in
`fixtures/` already stand in for.
