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

Tier 0 and Tier 1 are done, plus the two Tier 2 work orders everything else
waits on. 147 tests across the three repositories, all four verify commands
green in each.

| | What holds |
|---|---|
| **W1** · scaffolding | Three repos, pinned, dual-licensed. A breaking change to a shared crate fails in `sefer-crates` CI before it reaches either app — proven by breaking one. |
| **W2** · `girsa-hebrew` | The normalizer, and the line between what it will and will not do. 372-row regression corpus **harvested from 400 real seforim**, not written by hand. |
| **W3** · `girsa-ref` | The resolver. **100.00% exact on 2,970 real citations**, 0 wrong. Lexicon of 6,594 works and 24,731 spellings, built from Sefaria's schemas. |
| **W4** · `girsa-source` | The Source Packet. Ksav compiles it, and an arriving quote is put through the **real Typst compiler** rather than merely deserialized. |
| **W5** · fetch | 12,826 files, 3.4 GB on disk. Resumable — killed at 47%, resumed with nothing refetched. |
| **W6** · segment IDs | `girsa:mishnah-berurah/1:1#7`. One typo fix, 501 links: **line numbers moved 501, permanent ids moved 0.** |

Nothing draws a pixel yet. The shell (W9), the shelf (W10) and search (W11–W14)
are next, and the Ksav loop (W15–W19) is the milestone that makes the project
itself — `BUILDER.md` says to pull it as early as Tier 2 allows.

### Measurements that disagree with `spec.md`

Reported rather than coded around, per `BUILDER.md` Appendix B.5.

| | spec.md | measured |
|---|---|---|
| Sefaria download | ~2.2 GB | **3.4 GB** |
| schemas | 6,456 | **6,595** |
| Hebrew `merged.json` | 6,211 | 6,211 ✓ |
| link CSVs | 19 | 19 ✓ |

The two counts that matter are exact, so the spec's method was sound; the size
was under-sampled (40 titles) and the schema count has drifted up since.

`spec.md` §9.1 also says to strip `U+0591–U+05C7`. Four code points in that
range are *punctuation that separates words* — maqaf, paseq, sof pasuq, nun
hafukha. Deleting maqaf glues `אֶת־הַשָּׁמַיִם` into one token and the second verse of
the Torah stops being findable by either word in it. They become spaces.

## Licence

MIT OR Apache-2.0 — see [`LICENSE`](LICENSE). Forced by crate-sharing with
Ksav. No corpus text is committed here; texts are downloaded at first run and
each carries its own source and licence.
