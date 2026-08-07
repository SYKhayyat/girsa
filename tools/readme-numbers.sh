#!/usr/bin/env bash
# Re-count the numbers `README.md` marks, and write them back.
#
#   tools/readme-numbers.sh
#
# The other half of `the_numbers_in_the_readme_are_measurements`, which fails
# when a marked number has gone stale. That test is the gate; this is the fix,
# so nobody hand-edits a count and gets it wrong in the other direction.
#
# Same shape as `check-card.sh` and `check-ksav-fixture.sh`: the tree is the
# source, the file is the copy, and a copy nothing regenerates is a copy that
# rots.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here"

# 4686 → 4,686, so a reader takes it in at a glance. The test strips the commas
# before comparing, so either spelling passes and this is the one that reads.
#
# Not `printf "%'d"`: that needs a locale git-bash on Windows does not have, and
# returns the number unpunctuated without saying so.
thousands() {
  echo "$1" | sed -e :a -e 's/\(.*[0-9]\)\([0-9]\{3\}\)/\1,\2/;ta'
}

set_marked() {
  local name="$1" value
  value="$(thousands "$2")"
  # `[0-9]` first, so the pattern cannot match the empty string between the
  # markdown emphasis and the marker and insert a second copy of the number
  # beside the one already there. It did exactly that on its first run.
  #
  # No `perl -i` either: on Windows it refuses an in-place edit without a backup
  # suffix, and fails quietly enough that this script reported success while
  # changing nothing. Also found by running it.
  sed -i "s/[0-9][0-9,]*\(\**\)<!--=$name-->/$value\1<!--=$name-->/g" README.md
}

set_marked commands       "$(grep -c '#\[tauri::command\]' app/src-tauri/src/lib.rs)"
set_marked shell-lines    "$(wc -l < app/src-tauri/src/lib.rs)"
set_marked crates         "$(ls -d crates/*/ | wc -l)"
set_marked bins           "$(ls crates/*/src/bin/*.rs | wc -l)"
set_marked window-modules "$(ls app/src/*.ts | wc -l)"
set_marked styles-lines   "$(wc -l < app/src/styles.css)"

echo "README numbers re-counted."
echo "The check is: cargo test -p girsa-app --test the_numbers_in_the_readme_are_measurements"
