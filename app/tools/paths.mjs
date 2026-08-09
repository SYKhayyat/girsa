// Where things are, resolved once and correctly.
//
// Four files built this by hand:
//
//     path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1"))
//
// and both halves of that expression are a bug. A `file://` URL's `pathname` is
// **percent-encoded**, so a checkout under `C:\Users\Some One\Girsa` resolves to
// `Some%20One` and every path built from it finds nothing — the suite dying at
// import time with a path nobody can read. The `.replace(…)` fixes the leading
// drive letter, which is the *other* half of the same problem and the half that
// shows up on a developer's own machine, so it got fixed and the encoding did
// not.
//
// `fileURLToPath` does both, and it is the standard library's answer to exactly
// this. Ksav forbids the hand-rolled form by name — `runner.test.mjs`:
// *"nothing hand-rolls a path from import.meta.url"* — and `run.mjs` here says
// it has *"the same shape as `Ksav/ksav/app/test/run.mjs`, for the same reason
// it has that shape"* while carrying the expression that file bans. Neither
// repository's guard could read the other's tree; `prohibitions.test.mjs` is in
// both now, and this is the one place that knows the answer.

import { fileURLToPath } from "node:url";
import path from "node:path";

/** The directory holding the module whose `import.meta.url` this is. */
export const dirOf = (url) => path.dirname(fileURLToPath(url));

/** `app/`. */
export const APP = path.resolve(dirOf(import.meta.url), "..");

/** The repository root. */
export const ROOT = path.resolve(APP, "..");

/** `app/src/`. */
export const SRC = path.join(APP, "src");

/** `app/test/`. */
export const TEST = path.join(APP, "test");
