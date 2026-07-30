#!/usr/bin/env bash
# The producer against the consumer: does the packet Girsa builds still match
# the fixture Ksav's tests assert on?
#
#   tools/check-ksav-fixture.sh            # check, exit 1 on a diff
#   tools/check-ksav-fixture.sh --write    # accept the new packet
#
# Ksav is found as a sibling checkout, or at $KSAV.
#
# # Why this is a gate and not a comment
#
# `Ksav/ksav/engine/tests/from_girsa.rs` is the only check anywhere that spans
# both applications. Its fixture is real output of a real Girsa command rather
# than a hand-written shape, deliberately — and its own module note predicted
# how that would go wrong:
#
#   a fixture nobody can reproduce is a fixture that will be wrong quietly.
#
# Which is what happened. The import path started dropping the printed edition
# (grade finding N-1); the fixture still carried it; and the one test whose job
# was to notice spent that whole time comparing Ksav against a Girsa that no
# longer existed, going green on every run. Being right in a comment is not the
# same as being caught.
#
# It could not be regenerated in CI because regenerating it needed the 2.2 GB
# corpus. `--example fixture-packet` removes that: it builds the single work the
# fixture quotes in a temp directory and takes the real
# `Shelf::open` → `Shelf::read` → `send` path over it, byte for byte the same
# output as the full corpus. So the check is now a `cargo run` and a `diff`.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
siblings="$(cd "$here/.." && pwd)"
ksav="${KSAV:-$siblings/Ksav}"
fixture="$ksav/ksav/engine/tests/fixtures/girsa-packet.json"

write=0
[[ "${1:-}" == "--write" ]] && write=1

if [[ ! -f "$fixture" ]]; then
  # Not a skip. A check that passes because it could not find what it checks is
  # the exact failure this script exists to end.
  echo "FAILED: no Ksav fixture at $fixture"
  echo
  echo "Ksav is looked for as a sibling checkout, or at \$KSAV. Clone it beside"
  echo "this repository (github.com/SYKhayyat/ksav), or set KSAV=/path/to/Ksav."
  exit 1
fi

produced="$(mktemp)"
trap 'rm -f "$produced"' EXIT

if ! cargo run -q --manifest-path "$here/Cargo.toml" \
       -p girsa-app --example fixture-packet > "$produced"; then
  echo "FAILED: girsa could not produce a packet at all"
  exit 1
fi

if diff -u "$fixture" "$produced" > /dev/null 2>&1; then
  echo "OK: the packet Girsa produces is the one Ksav asserts against."
  exit 0
fi

if [[ $write -eq 1 ]]; then
  cp "$produced" "$fixture"
  echo "WROTE: $fixture"
  echo
  echo "The packet changed on purpose, so Ksav's fixture now says so too."
  echo "Commit both repositories — the pen and the library disagree until you do."
  exit 0
fi

echo "FAILED: Girsa's packet and Ksav's fixture disagree."
echo
echo "  - Ksav's fixture (what the pen expects)"
echo "  + Girsa's packet  (what the library sends)"
echo
diff -u "$fixture" "$produced"
echo
echo "If the packet changed on purpose, re-run with --write and commit both"
echo "repositories. If it did not, the library is dropping something on the"
echo "way to the pen — which is grade finding N-1, and it is a blocker."
exit 1
