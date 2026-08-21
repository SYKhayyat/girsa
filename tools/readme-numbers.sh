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

# The pages that carry markers. `docs/tools.md` joined `README.md` when the
# word *fifteen* on it went stale for the second time; the test reads the same
# list, in the same order, as `PAGES`.
PAGES="README.md docs/tools.md"

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
  # Every page, because a marker's name is not owned by one of them: `bins` is
  # claimed in both, and a name that moves between pages must not need this
  # script edited to follow it.
  for page in $PAGES; do
    sed -i "s/[0-9][0-9,]*\(\**\)<!--=$name-->/$value\1<!--=$name-->/g" "$page"
  done
}

# **The prefix, not the whole attribute**, and this line said the whole
# attribute for as long as the test's did not.
#
# The day every command in the shell became `#[tauri::command(async)]` — so one
# blocked call could not hold the window still — the exact pattern stopped
# matching 135 of the 138. The test was fixed and its comment says so; this
# script is the *other half of that pair* and was left counting 3. So the one
# command both documents name as the way to repair a stale number would have
# written `3` into the README's `commands`, and the gate's next run would have
# failed on a number a reader had just been told to trust.
#
# Nothing caught it because a generator is only run when a number moves, and
# the numbers that moved since — the two line counts — are spelled the same way
# on both sides. Two lists again, one of them nobody edits.
set_marked commands       "$(grep -cE '^[[:space:]]*#\[tauri::command' app/src-tauri/src/lib.rs)"
set_marked shell-lines    "$(wc -l < app/src-tauri/src/lib.rs)"
# `*/Cargo.toml`, which is what the test counts: a directory under `crates/`
# with no manifest in it is not a crate, and the two halves should not be able
# to disagree about that either.
set_marked crates         "$(ls -d crates/*/Cargo.toml | wc -l)"
set_marked bins           "$(ls crates/*/src/bin/*.rs | wc -l)"
set_marked examples       "$(ls crates/*/examples/*.rs | wc -l)"
set_marked window-modules "$(ls app/src/*.ts | wc -l)"
set_marked styles-lines   "$(wc -l < app/src/styles.css)"

echo "Numbers re-counted in: $PAGES"
echo "The check is: cargo test -p girsa-app --test the_numbers_in_the_readme_are_measurements"
