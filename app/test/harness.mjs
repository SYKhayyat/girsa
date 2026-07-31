// A test harness small enough that nobody has to learn it.
//
// `app/src` was 6,476 lines of TypeScript with no test directory and no `test`
// script — zero assertions over the entire webview, in a repository whose Rust
// side carries 625. The thing standing between that and coverage was never a
// framework, so this is four functions and a counter, and the effort goes into
// the assertions. It is deliberately the same four functions as
// `Ksav/ksav/app/test/harness.mjs`: two sibling repositories with two different
// harnesses is one more thing to know than either needs.
//
// There is no `localStorage` or `indexedDB` here, because nothing in Girsa's
// frontend uses them — the session lives in Rust (`girsa-app/src/session.rs`).
// Add them when something needs them, not before.

let pass = 0;
let fail = 0;
const failures = [];

function record(name, ok, detail) {
  if (ok) {
    pass++;
  } else {
    fail++;
    failures.push(`FAIL ${name}${detail ? `\n  ${detail}` : ""}`);
    console.log(failures[failures.length - 1]);
  }
}

/** Deep equality by JSON shape — enough for the plain data these modules pass. */
export function check(name, got, want) {
  const g = JSON.stringify(got);
  const w = JSON.stringify(want);
  record(name, g === w, `got  ${g}\n  want ${w}`);
}

export function ok(name, value) {
  record(name, !!value, `got ${JSON.stringify(value)}, wanted something truthy`);
}

export function notOk(name, value) {
  record(name, !value, `got ${JSON.stringify(value)}, wanted something falsy`);
}

/** Assert that an async call rejects, and optionally with a particular name. */
export async function rejects(name, fn, errorName) {
  try {
    await fn();
    record(name, false, "it resolved; an error was expected");
  } catch (e) {
    record(
      name,
      !errorName || e?.name === errorName || e?.constructor?.name === errorName,
      `threw ${e?.name ?? e}, wanted ${errorName}`,
    );
  }
}

export function counts() {
  return { pass, fail };
}
