# Where this tree is tested, and where it is not

The audits said where they were thin. [`AUDIT-CORRECTNESS.md`](../AUDIT-CORRECTNESS.md),
*Coverage, and what this audit did not reach*, named the surfaces that were read
for shape only and the suites that were sampled rather than swept — and then
said the question worth asking of every test was *which of these tests would
still pass if the thing they name were broken?*

This page is the answer to the first of those, written as a map, and the two
tools beside it are the answer to the second.

* **`tools/coverage-manifest.json`** — the map, machine-readable: every
  surface, where it is tested, the six highest-risk invariants and the test
  that pins each one, and the honest rows that are not swept.
* **`node tools/check-coverage.mjs`** — the gate's tenth step. It reads the
  map and the tree and demands they still agree: a named test file that has
  been deleted, a pinning test that no longer mentions what it pins, a guard
  the manifest's mutation no longer applies to — any of those is a red step.
  It also prints the uncovered and platform-gated rows **every run**, so a
  green gate is a gate that has said out loud what it did not reach.
* **`node tools/mutation.mjs`** — the negative-control half. It breaks each
  guard and demands the pinning test fail. A mutation the test does not catch
  is a red row. Not in the gate — each mutation recompiles — but the record
  asks for it when a guard drifts.

If this page and the manifest ever disagree, **the manifest is right and this
page has rotted** — the checker is what enforces it, and it is the one that
runs.

---

## The surfaces

| Surface | What it owns | Where it is tested |
|---|---|---|
| girsa-plain | plaintext reading and normalization | unit tests in the crate |
| girsa-personal | the personal layer: append-only logs, tombstones, compaction, six stores | unit tests in the crate, including M2/M3 |
| girsa-corpus | import, continuity, sections, taxonomy, work, redirects | six integration files, including the re-import invariants |
| girsa-search | the index, citation, facets, the widening ladder, proximity | thirteen integration files |
| girsa-link | the link graph, inbound caches, repair | four integration files |
| girsa-fix | corrections and their application | unit and integration |
| girsa-note | notes, marks, saved queries, two people's layers | one integration file and unit tests |
| girsa-scan | OCR, paging schemes, anchors, the daf a page is | three integration files and unit tests |
| girsa-lane | the semantic lane: vectors, signatures, coverage | unit tests in the crate |
| girsa-app | the library the shell calls: sending, held, workspace, luach, arrangement, session | twenty-two integration files, including the gates that read the whole tree |
| girsa-export | fixed seforim out to text and word files | one integration file |
| girsa-nearby | adjacency and what you have not seen | one integration file and unit tests |
| girsa-desk | the desk: sessions, refreshing, citing, documents | unit tests in the crate |
| girsa-mcp | the MCP server and its tool surface | two integration files and unit tests |
| girsa-fixture | the test shelf every other crate stands on | unit tests in the crate |
| app/src-tauri | the Tauri shell: commands, refusal codes, post, clipboard | no test files; held by shell clippy, shell fmt, the wire tests and the twenty-two gates |
| app/src (the window) | thirty-five TypeScript modules | twenty-four window test files, about five hundred and thirty-eight tests |

## The highest-risk invariants, and what pins each

| Invariant | The guard | The test that must fail if the guard breaks |
|---|---|---|
| Near is a per-gap contract | `crates/girsa-search/src/proximity.rs` — `step: gap.saturating_add(1)` | `near_is_a_per_gap_contract_for_three_words_too` |
| OCR structural rows are not words | `crates/girsa-scan/src/engine.rs` — the empty-text skip | `the_structural_rows_are_not_words_and_the_boxes_are_fractions_of_the_page` |
| Repairs attribute by time, not by key order | `crates/girsa-link/src/repair.rs` — `records.sort_by_key(\|record\| record.when)` | `attribution_after_a_restart_goes_to_the_latest_action_not_the_kind_that_sorts_last` |
| An anchor counts for the page it is on, and never a page before it | `crates/girsa-scan/src/paging.rs` — `a.page <= page` | `an_anchor_counts_for_the_page_it_is_on` |
| A small layer is never rewritten | `crates/girsa-personal/src/log.rs` — `const FLOOR: usize = 64` | `a_small_layer_is_never_rewritten_for_a_stray_deletion` |
| A read-only MCP server refuses writes at the door | `crates/girsa-mcp/src/tools.rs` — `if !server.is_writable() =>` | `a_write_against_a_read_only_server_is_refused_at_the_door` |

`node tools/mutation.mjs --list` prints the exact break each mutation applies,
and `node tools/mutation.mjs` applies them, one at a time, and demands the
failure. The six are the invariants that have a single-line guard; the rest of
the tree is pinned by the suites in the table above and the audit's regression
tests, which is a different and weaker claim than a mutation run, and this page
does not pretend otherwise.

## Not swept, and said so

* **girsa-corpus fetch** — the downloader's path handling is pinned by the
  finding-12 fixes; the network half cannot be exercised without the libraries.
* **girsa-search scoring** — order is pinned by `offered_not_applied.rs` and
  the typo-queue ranking; the score formula itself is asserted only through
  those orders.
* **girsa-lane nearest-neighbour arithmetic** — `vectors.rs` pins persistence,
  signatures and restarts; the distance formula is asserted through ranked
  outcomes only.
* **girsa-nearby** — the smallest crate in the tree and the least swept: one
  integration file and the unseen unit tests.
* **girsa-desk** — unit-tested inside the crate; no integration file exercises
  it against a real shelf.
* **app/src-tauri** — 7,554 lines of shell commands held by clippy, fmt, the
  wire tests and the twenty-two gates, not by tests that drive the commands.

## Platform-gated — skipped, never silently green

The gate runs the eyes step with `EYES_REQUIRED=1`, so a machine with no
browser gets a red step whose own output says how to fix it — a skipped eyes
can never read as a pass. The rest of these are reported by the checker on
every run, with what they need:

* **The eyes step** — needs a browser (Edge on Windows, or `EYES_BROWSER`).
* **OCR against real tesseract** — the engine's absence is pinned as a
  sentence; a real run needs tesseract with the `heb` traineddata.
* **WebKitGTK and macOS WebKit** — never seen; the CI matrix builds and lints
  them, it does not look at them.
* **The semantic lane on a real shelf** — needs the model weights and the
  two-library shelf.
* **Ksav interop** — CI checks out Ksav beside this tree and catches drift; a
  live pen is a machine this desk does not have.
