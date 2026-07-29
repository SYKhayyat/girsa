# Third-party notices

Girsa itself is dual-licensed MIT OR Apache-2.0 (see `LICENSE`). This file
covers everything else that ships inside a Girsa installer.

It exists because it has to. What is listed here is **bundled into the
application**, not merely depended on at build time: it is inside every
installer the release workflow publishes, and the Apache-2.0 licence requires
its notice to accompany redistribution.

Rust dependencies are not listed here. They are compiled in, they are all
permissively licensed, and `cargo` records them in `Cargo.lock`; nothing in the
dependency tree carries a notice requirement. **No AGPL or GPL code is used
anywhere in this project** — Zayit, HebMorph and Sefaria-ElasticSearch were read
as prior art and copied from nowhere (`BUILDER.md` T7).

**No corpus text is committed or shipped.** Seforim are downloaded at first run
and each carries its own source and licence (`spec.md` §13).

---

## pdf.js (`pdfjs-dist`)

- Bundled into `app/dist` by `vite build`, and therefore into the installer.
- Copyright the Mozilla Foundation and contributors.
- Licence: Apache License, Version 2.0 — `licenses/Apache-2.0.txt`
- <https://github.com/mozilla/pdf.js>

It draws a page of a scan you brought (`spec.md` §6.3, `BUILDER.md` W25). It is
bundled rather than fetched because Girsa does not go to the network to read
(§14), and it is loaded the first time a scan is opened rather than at startup.
Nothing in Girsa is derived from it; it is used as a library, unmodified.

## Tauri, and the webviews it uses

- Tauri is MIT OR Apache-2.0.
- The rendering engine is the **operating system's** — WebView2 on Windows,
  WebKit on macOS, WebKitGTK on Linux — and is not redistributed here.
