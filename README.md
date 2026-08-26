# גִּרְסָא · Girsa

**A Torah library that assumes you are going to write something.**

Girsa (גִּרְסָא, *the text as received*) is the page. **Ksav** (כְּתָב, *writing*) is the pen. The pairing is the idea: you find a mekor while learning, send it to what you are writing, and the citation in the finished PDF opens the page it names.

Nothing is retyped, because what gets stored is the **reference**, not a printed string that looks like one.

> **New here and not a Rust developer?** You do not need Rust to use Girsa. Begin with [`docs/start-here.md`](docs/start-here.md), then follow [`Getting it`](#getting-it) if you want to fill a local library. Rust, Cargo, and the source checkout are only needed for contributors and maintainers. You can help without programming by testing search and reading workflows, checking citations and metadata, improving documentation, or reporting a reproducible corpus problem with the title, reference, query, and expected result.

---

## Where to go

| You are here to | Start at |
|---|---|
| **use it** | [`docs/start-here.md`](docs/start-here.md) |
| **install it** | [Getting it](#getting-it) |
| **contribute without coding** | [`docs/your-first-change.md`](docs/your-first-change.md) and [`docs/troubleshooting.md`](docs/troubleshooting.md) |
| **contribute code** | [`CONTRIBUTING.md`](CONTRIBUTING.md) |
| **understand the design** | [`docs/architecture.md`](docs/architecture.md) |
| **fix something that is not working** | [`docs/troubleshooting.md`](docs/troubleshooting.md) |
| **build it from the spec** | [`spec.md`](spec.md), then [`BUILDER.md`](BUILDER.md) |

## What it does

**A library.** Sefaria and Otzaria on one shelf, arranged how you arrange it, offline.

**Search.** Torat Emet, literal, phrase, proximity, and regex search; citation lookup, gematria, roshei teivot, and dilug. Results show matching words highlighted inside the line.

**Reading and writing.** Open a mefaresh beside the text, follow links, create personal overlays and notes, and send a source packet to Ksav without retyping its citation.

**Important boundary.** Girsa is a research and study tool. It does not replace a qualified teacher or posek, and corpus text, OCR, links, and user corrections require verification.

## Getting it

An installer is attached to every `v*` tag on the releases page. The installer carries the application and tools but not the library. From a clone, the shelf-building flow is:

```sh
node tools/build-a-shelf.mjs corpus --download-otzaria
```

The full prerequisites, library sources, disk requirements, resumable stages, and troubleshooting are documented in [`docs/the-libraries.md`](docs/the-libraries.md), [`BUILDER.md`](BUILDER.md), and [`docs/troubleshooting.md`](docs/troubleshooting.md). A fresh user should read those pages before downloading the corpus because the completed shelf can require many gigabytes.

## Contributing without coding

To report a problem, include the release or commit, operating system, corpus/library source, exact query or reference, what appeared, what you expected, and whether the problem reproduces after rebuilding the relevant cache. For documentation or corpus work, preserve source provenance and do not silently overwrite imported text; use the correction/personal layer when the source itself must remain replaceable.

For code contributors, [`CONTRIBUTING.md`](CONTRIBUTING.md) explains the Rust workspace, the application, tools, test gates, and the smallest checks to run. [`docs/your-first-change.md`](docs/your-first-change.md) is the recommended path for a first patch.

## Documentation

- [`docs/start-here.md`](docs/start-here.md) — first five minutes
- [`docs/the-libraries.md`](docs/the-libraries.md) — source libraries and terms
- [`docs/architecture.md`](docs/architecture.md) — system design
- [`docs/troubleshooting.md`](docs/troubleshooting.md) — symptom-first diagnosis
- [`docs/tools.md`](docs/tools.md) — tool reference
- [`docs/the-record.md`](docs/the-record.md) — decisions and history
- [`HANDOFF.md`](HANDOFF.md) — current maintainer context
