# גִּרְסָא · Girsa

**A Torah library that assumes you are going to write something.**

Girsa (גִּרְסָא, "the text as received") is the page. **Ksav** (כְּתָב, "writing")
is the pen. The pairing is the idea.

- **[`spec.md`](spec.md)** — what Girsa is.
- **[`BUILDER.md`](BUILDER.md)** — what to do on day one: work orders, binding
  rules, the verified traps in the data, and what may not be decided alone.

Read `spec.md` §2 (ground truth), §3 (the landmine) and §16 (settled decisions)
first. They are what shape everything else.

## Where things are

```
Videos/
  Girsa/          this repository — the library app
  Ksav/           the writing app          github.com/SYKhayyat/ksav
  sefer-crates/   the shared contract      github.com/SYKhayyat/sefer-crates
```

| Crate | Purpose |
|---|---|
| `girsa-corpus` | Storage, ingest, schemas, permanent segment IDs |
| `girsa-search` | tantivy indices, the five modes, the relaxation ladder |
| `girsa-link` | The typed link graph, repair, later mining |

plus `girsa-source`, `girsa-ref`, `girsa-hebrew` and `girsa-cite` from
`sefer-crates`, pinned to an exact version and resolved from the sibling
checkout during development.

**The sibling checkout has to be present.** Until `sefer-crates` is published,
cloning Girsa alone will not build.

## Build

```sh
cargo build --all-targets
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
```

## Status

**W1 — scaffolding.** Three repositories, wired, dual-licensed, with the
cross-repo check working: a breaking change to a shared crate fails in
`sefer-crates` CI before it can reach either application.

Nothing reads a sefer yet. `crates/` holds the shape of each component and the
invariants that must hold, not their implementations — W2 onward fill them in,
in the order `BUILDER.md` sets out.

## Licence

MIT OR Apache-2.0 — see [`LICENSE`](LICENSE). Forced by crate-sharing with
Ksav. No corpus text is committed here; texts are downloaded at first run and
each carries its own source and licence.
