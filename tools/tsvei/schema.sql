-- The text store: verses, and the conversations that were exported as JSON.
--
-- Two tables and the same four things on both: the text itself, a place in the
-- source (book/chapter/verse, or thread/role/time), a full-text index for words
-- a reader can spell, and a vector for the ones they cannot. Postgres answers
-- both questions in one language, which is why this is a table rather than a
-- file and a second service.
--
-- Applied more than once without harm: every statement is `if not exists`.
-- `psql -f schema.sql` is the whole install.

create extension if not exists vector;

-- ── verses ────────────────────────────────────────────────────────────────────
--
-- `book` is the Hebrew name a sefer is called by, because that is what a writer
-- types and what the vault says. `book_en` is the name every index and URL
-- wants, kept beside it rather than derived, since the two are not a
-- translation of each other (במדבר is "Numbers" and also "Bamidbar").
create table if not exists tsvei_verse (
  book        text    not null,
  book_en     text,
  chapter     integer not null,
  verse       integer not null,
  ref_he      text    not null,
  -- The verse as it is printed, with its vowel points.
  text        text    not null,
  -- The same verse with the points taken off, which is what an index can hold:
  -- a reader searching `בראשית` must find `בְּרֵאשִׁית`, and a tsvector built
  -- from the pointed text holds one token that the query does not match. See
  -- `hebrew.mjs` for the sweep and for the maqaf.
  text_plain  text    not null,
  -- Every note that carried this verse. Two vault files covering one pasuk are
  -- one row: the text is the longest of them and both sources are named, so
  -- nothing is silently dropped and nothing is counted twice.
  sources     text[]  not null default '{}',
  fts         tsvector generated always as
                (to_tsvector('simple', coalesce(text_plain, ''))) stored,
  embedding   vector(1536),
  embedded_at timestamptz,
  updated_at  timestamptz not null default now(),
  primary key (book, chapter, verse)
);

create index if not exists tsvei_verse_fts on tsvei_verse using gin (fts);
create index if not exists tsvei_verse_ref on tsvei_verse (book, chapter, verse);
-- Cosine, because the embedder's vectors are unit-length and cosine is the
-- measure that does not care how long the passage is.
create index if not exists tsvei_verse_embedding on tsvei_verse
  using hnsw (embedding vector_cosine_ops);

-- ── conversations ─────────────────────────────────────────────────────────────
--
-- A chat export is a list of turns, and the same export imported twice is the
-- same rows: `digest` is a hash of (thread, role, time, text), so a re-import
-- updates nothing and an edited export adds only what changed.
create table if not exists chat_message (
  id          bigserial primary key,
  thread      text,
  role        text not null,
  said_at     timestamptz,
  text        text not null,
  text_plain  text not null,
  digest      text not null unique,
  fts         tsvector generated always as
                (to_tsvector('simple', coalesce(text_plain, ''))) stored,
  embedding   vector(1536),
  embedded_at timestamptz,
  updated_at  timestamptz not null default now()
);

create index if not exists chat_message_fts on chat_message using gin (fts);
create index if not exists chat_message_thread on chat_message (thread, said_at);
create index if not exists chat_message_embedding on chat_message
  using hnsw (embedding vector_cosine_ops);
