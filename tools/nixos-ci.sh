#!/bin/sh
#
# Everything the `nixos` job does inside the container.
#
#     docker run --rm -v "$PWD:/w" -w /w nixos/nix:latest sh -eu /w/tools/nixos-ci.sh
#
# # Why this is a file and not a line in the workflow
#
# It was a line in the workflow, and the line was `sh -euc '...'` with the whole
# script inside the quotes. The job failed on the word `job's` in one of the
# comments: the apostrophe closed the string, the two commands after it ran on
# the host instead of in the container, and the host has no Nix. `nix: command
# not found`, after a fifteen-minute build that had already succeeded.
#
# `tools/nixos-window.sh` had the right idea one level down and wrote it in its
# own header — a quoted script inside a quoted script inside a YAML block is
# three levels of escaping and one of them is always wrong. This is that lesson
# applied to the outermost level. In a file, an apostrophe is an apostrophe.
#
# `sh` and not `bash`: the image is Alpine, and what it ships is busybox. The
# tools that want bash are started through `nix develop`, which has one.

# ── The repository is not ours, and Nix wants to read it as git ──────────────
#
# `repository path "/w" is not owned by current user`, which is what this job
# said on its second push. A flake inside a git working tree is a **git input**,
# so Nix asks libgit2 to open the repository — and libgit2 refuses a repository
# owned by somebody other than the process, which here is the host runner user
# against root in the container. Saying the directory is safe is the documented
# remedy and keeps the git input, which is what makes Nix respect `.gitignore`
# and not copy `target/` into the store.
#
# Written rather than set with `git config`, because the image ships Nix and is
# not obliged to ship a git binary — and Nix reads this through libgit2, which
# reads the file and not the command.
printf '[safe]\n\tdirectory = /w\n' > "$HOME/.gitconfig"

# First, and on its own, so that a package renamed out of nixpkgs — which is how
# this file will most likely break — is reported as an evaluation failure and
# not as a compiler one.
nix flake check --no-build

nix develop --command bash -c 'cd app && npm ci && npm test && npm run build'
nix develop --command cargo build --workspace

# ── And the window, which had never been opened here ─────────────────────────
#
# This job's own row in `docs/not-yet.md` said *a container has no display, so
# nothing there has opened a WebKitGTK surface* — and the first half of that
# sentence answers the second. A container has no display until somebody starts
# one. `xvfb-run` is in the devShell now and `tools/nixos-window.sh` starts the
# binary on it, waits for the screen to stop being blank, and counts the colours
# on it.
#
# A picture rather than an exit code, because the failure this is for is not a
# crash: without `WEBKIT_DISABLE_COMPOSITING_MODE` the window opens, draws
# nothing and closes cleanly, and a process that lived is evidence of nothing.
#
# `--features tauri/custom-protocol` is what embeds `app/dist`. Without it the
# window opens on the Vite dev server and draws a browser error page, which is
# in colour and would pass; the script refuses a binary that does not carry the
# frontend rather than measuring one.
nix develop --command bash -c 'cd app/src-tauri && cargo build --features tauri/custom-protocol'
nix develop --command bash tools/nixos-window.sh
