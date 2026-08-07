#!/usr/bin/env bash
# The generator against the generated: is `docs/shortcuts.md` still what
# `girsa-card` prints?
#
#   tools/check-card.sh            # check, exit 1 on a diff
#   tools/check-card.sh --write    # accept the new card
#
# # Why this is a gate and not a sentence in the README
#
# The README says the shortcut card is *"generated from the source, so it cannot
# drift."* That was a claim about a command nobody ran. `girsa-card` reads
# `girsa_app::keys::ACTIONS` — the table the window really resolves a key press
# against — so the card is wrong only if the application is; but only if
# somebody re-runs it. Rebind an action, or add one, and `docs/shortcuts.md`
# keeps saying what the keys used to be, with a header at the top asserting it
# cannot.
#
# The repository already owned this shape. `tools/check-ksav-fixture.sh`
# generates a packet and diffs it against the fixture Ksav asserts on, for
# exactly this reason and after exactly this failure — and it was applied to the
# other application's fixture and not to this one's card.
#
# It happens to be in sync today. That is the moment to add the gate, not the
# argument against one.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
card="$here/docs/shortcuts.md"

write=0
[[ "${1:-}" == "--write" ]] && write=1

if [[ ! -f "$card" ]]; then
  # Not a skip. A check that passes because it could not find what it checks is
  # the exact failure this script exists to end.
  echo "FAILED: no card at $card"
  echo
  echo "Write one: tools/check-card.sh --write"
  exit 1
fi

printed="$(mktemp)"
trap 'rm -f "$printed"' EXIT

if ! cargo run -q --manifest-path "$here/Cargo.toml" \
       -p girsa-app --bin girsa-card > "$printed"; then
  echo "FAILED: girsa-card could not print a card at all"
  exit 1
fi

if diff -u "$card" "$printed" > /dev/null 2>&1; then
  echo "OK: docs/shortcuts.md is what girsa-card prints."
  exit 0
fi

if [[ $write -eq 1 ]]; then
  cp "$printed" "$card"
  echo "WROTE: $card"
  exit 0
fi

echo "FAILED: docs/shortcuts.md and girsa-card disagree."
echo
echo "  - docs/shortcuts.md (what the reader is told)"
echo "  + girsa-card        (what the window will do)"
echo
diff -u "$card" "$printed"
echo
echo "The keys changed. Re-run with --write and commit the card with them:"
echo
echo "    tools/check-card.sh --write"
exit 1
