#!/usr/bin/env bash
#
# Open Girsa's window on NixOS and photograph it.
#
#     nix develop --command bash tools/nixos-window.sh
#
# # Why this exists
#
# `docs/not-yet.md` has carried the same sentence since the `nixos` job was
# written: *a container has no display, so nothing there has opened a WebKitGTK
# surface, and `WEBKIT_DISABLE_COMPOSITING_MODE` is a line every Tauri
# application on NixOS carries rather than a line anybody here has watched
# work.*
#
# The first half of that sentence contains the answer to the second. A container
# has no display **until somebody starts one**, and `xvfb-run` is the X server
# that exists to be nobody's screen. What was missing was not a machine.
#
# # Why a picture and not an exit code
#
# The failure this is looking for is not a crash, and that is the whole
# difficulty. Without `WEBKIT_DISABLE_COMPOSITING_MODE`, WebKitGTK on NixOS
# composites through its own sandbox, cannot reach the store paths it needs from
# inside one, and draws **nothing** — the window opens, sits there, and closes
# cleanly when asked. A process that lived for thirty seconds is evidence of
# nothing at all. So this counts the colours on the screen: an empty root window
# is one, a white window is one or two, and a drawn page of Hebrew is hundreds.
#
# # What it still does not settle
#
# That the window is *right*. Nobody is reading a sefer off this picture and
# this script does not know what Girsa looks like. The claim it supports is that
# something was drawn, on a machine with no FHS — one rung above *the code
# compiles* and several below *somebody used it*. `docs/not-yet.md` says which.

set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
cd "$here"

shell=target/debug/girsa-shell
shot=${GIRSA_WINDOW_SHOT:-/tmp/girsa-window.png}
log=/tmp/girsa-window.log

# ImageMagick 7 is one binary with the old names beside it, and which of those
# names a build installs is a decision the packager made. Ask, rather than
# assume, so that a missing `import` is reported as a missing tool instead of as
# a window that did not draw.
if command -v import >/dev/null 2>&1; then
  snap() { import -window root "$1"; }
  count() { identify -format %k "$1"; }
elif command -v magick >/dev/null 2>&1; then
  snap() { magick import -window root "$1"; }
  count() { magick identify -format %k "$1"; }
else
  echo "no imagemagick on the path — flake.nix puts it in the devShell, so this"
  echo "was probably not run through 'nix develop'."
  exit 1
fi

# ── Under a display, or start one ────────────────────────────────────────────
#
# Re-running this same file inside `xvfb-run` rather than passing it a string of
# shell: a quoted script inside a quoted script inside a YAML block is three
# levels of escaping and one of them is always wrong. `-a` picks a free display
# number instead of fighting over `:99`, which is the number every other tool on
# a runner also picks.
if [ "${1:-}" != "--under-a-display" ]; then
  # GTK wants somewhere to put its sockets and the container has not made one.
  # Absent, GTK prints `XDG_RUNTIME_DIR is not set` and carries on, which is a
  # warning in the log a person reads after a failure and not a cause.
  export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/tmp/girsa-runtime}
  mkdir -p "$XDG_RUNTIME_DIR"
  chmod 700 "$XDG_RUNTIME_DIR"

  # ── Rendering, on a machine with no graphics card ──────────────────────────
  #
  # The first run that got this far said
  #
  #     Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
  #
  # and photographed one colour. WebKitGTK has required EGL since 2.42, an
  # Xvfb screen has no GPU behind it, and `WEBKIT_DISABLE_COMPOSITING_MODE`
  # does not cover it — that variable is about compositing, and this fails one
  # step earlier, at opening a display at all. `flake.nix` has `libglvnd` and
  # `mesa` in the devShell now; these three lines are what point them at the
  # CPU.
  #
  # Set here rather than in the flake because the flake's shell is also what a
  # person developing on NixOS enters, and telling their machine to render in
  # software would be a rude thing to do to somebody with a graphics card. This
  # script only ever runs where there is none.
  export LIBGL_ALWAYS_SOFTWARE=1
  export GALLIUM_DRIVER=llvmpipe
  export GDK_BACKEND=x11

  # And the half that a package on `LD_LIBRARY_PATH` does not settle.
  # `libEGL.so.1` belongs to `libglvnd`, which is a dispatcher: it picks a
  # driver by reading vendor manifests out of `/usr/share/glvnd/egl_vendor.d`,
  # and this container has no `/usr/share` because it has no FHS. Having mesa
  # installed and having it *findable* are two different things, and the second
  # needs the store path — which `flake.nix` passes down as `GIRSA_MESA` rather
  # than setting these itself, so that a NixOS desktop with an NVIDIA card
  # entering the same shell is not told to render through llvmpipe.
  #
  # Globbed rather than named: `50_mesa.json` is the filename today and the
  # number in front of it is a priority somebody may renumber. Reported either
  # way, because *EGL was configured* and *EGL was not* have to be
  # distinguishable in the log without a second run.
  if [ -n "${GIRSA_MESA:-}" ]; then
    vendor=$(ls "$GIRSA_MESA"/share/glvnd/egl_vendor.d/*.json 2>/dev/null | head -1)
    if [ -n "$vendor" ]; then
      export __EGL_VENDOR_LIBRARY_FILENAMES="$vendor"
    fi
    if [ -d "$GIRSA_MESA/lib/dri" ]; then
      export LIBGL_DRIVERS_PATH="$GIRSA_MESA/lib/dri"
    fi
  fi

  # The two the flake sets, defaulted rather than assumed. The failure message
  # further down tells a reader that a shell entered any other way will not
  # have them, which was true and is a strange thing for a script to say about
  # itself when it could simply carry them.
  export WEBKIT_DISABLE_COMPOSITING_MODE=${WEBKIT_DISABLE_COMPOSITING_MODE:-1}
  export WEBKIT_DISABLE_DMABUF_RENDERER=${WEBKIT_DISABLE_DMABUF_RENDERER:-1}

  if [ ! -x "$shell" ]; then
    echo "no $shell. Build it first:"
    echo "  cd app && npm ci && npm run build"
    echo "  cd app/src-tauri && cargo build --features tauri/custom-protocol"
    exit 1
  fi

  # **The build that would make this measure the wrong thing.**
  #
  # `--features tauri/custom-protocol` is what puts `app/dist` *inside* the
  # binary. Without it the window navigates to the Vite dev server at
  # `http://localhost:5174` and, with no server there, draws the webview's *this
  # site can't be reached* — a page, in colour, which would pass the count below
  # while proving the opposite of what this claims.
  #
  # `app/src-tauri/build.rs` refuses that build in the release profile and is
  # deliberately silent in debug, because `cargo check` and `tauri dev` both
  # want exactly that binary. Debug is therefore the one profile where the
  # mistake is possible, and this is the only place it can be caught.
  #
  # # What to look for, which took two tries
  #
  # This asked for `find-here-box`, a class in `app/src/styles.css`, and failed
  # a correctly built binary in CI on 17 August. **Tauri brotli-compresses the
  # embedded assets**, so no word out of the CSS or the HTML is a literal string
  # in the executable, and the check was testing the compressor.
  #
  # `index.html` *is* in there, and is worse than useless: measured on both
  # builds of this binary, it is present either way — it is the asset resolver's
  # own path constant, compiled in whether or not anything was embedded.
  #
  # What discriminates is Vite's **hashed** filenames, which are the keys of the
  # embedded map and are stored uncompressed. Measured, same tree, minutes
  # apart: `core-DhEqZVGG.js` present with the feature, absent without. Reading
  # the name off `app/dist` rather than writing it here also means a binary that
  # embedded *last week's* frontend fails, which is the same mistake wearing a
  # better disguise.
  assets=app/dist/assets
  # Rule 7: a check that cannot find its input says so, and does not report the
  # absence of its input as a fault in the thing being checked.
  if [ ! -d "$assets" ]; then
    echo "no $assets — the frontend was never built, so there is nothing that"
    echo "could have been embedded. Build it first:"
    echo "  cd app && npm ci && npm run build"
    exit 1
  fi
  embedded=$(ls "$assets" | grep -v '\.map$' | head -1)
  if [ -z "$embedded" ]; then
    echo "$assets is empty. 'npm run build' wrote no assets, so this cannot tell"
    echo "an embedded binary from one that was not."
    exit 1
  fi
  if ! grep -aqF "$embedded" "$shell"; then
    echo "$shell does not carry the frontend: '$embedded' is in $assets and its"
    echo "name is not in this binary. Built without"
    echo "--features tauri/custom-protocol, the window opens on the dev server"
    echo "and this would photograph a browser error page."
    exit 1
  fi
  echo "the frontend is in the binary ($embedded)"

  # What it is about to run with. Four lines in the log, and they are the
  # difference between a next failure that answers a question and one that
  # raises it — the run above spent two minutes proving something was wrong
  # with the graphics and could not say whether the variables meant to prevent
  # it had even arrived.
  echo "the window will open with:"
  for named in WEBKIT_DISABLE_COMPOSITING_MODE WEBKIT_DISABLE_DMABUF_RENDERER \
    LIBGL_ALWAYS_SOFTWARE GALLIUM_DRIVER GDK_BACKEND XDG_RUNTIME_DIR \
    GIRSA_MESA __EGL_VENDOR_LIBRARY_FILENAMES LIBGL_DRIVERS_PATH; do
    eval "value=\${$named:-<unset>}"
    echo "  $named=$value"
  done

  echo "opening the window under Xvfb"
  exec xvfb-run -a --server-args="-screen 0 1360x900x24" \
    bash "$0" --under-a-display
fi

# ── From here down there is a screen ─────────────────────────────────────────

"./$shell" >"$log" 2>&1 &
pid=$!

# Is it *actually* still going?
#
# `kill -0 "$pid"` is the obvious test and it is wrong here: a child that has
# died and not been reaped is a zombie, the process still exists, and signal 0
# is delivered to it happily. The first run to reach this loop watched a
# process that had aborted in the first second, polled it for the full ninety,
# and then reported *the window opened and drew nothing* — true, and the least
# useful of the three true things it could have said.
#
# `/proc/<pid>/stat` knows. The state is the field after the last `)`, which is
# how it has to be read: the field before it is the executable's name, in
# parentheses, and a name may contain spaces and parentheses of its own.
still_running() {
  [ -r "/proc/$1/stat" ] || return 1
  [ "$(sed 's/.*) //' "/proc/$1/stat" | cut -d' ' -f1)" != "Z" ]
}

# The window has to exist before it can be photographed, and how long that takes
# on a cold container is not a number anybody here knows. So this waits for the
# thing it wants rather than for a duration, and gives up after ninety seconds —
# a budget rather than an expectation, since the loop returns the moment the
# screen is not blank and only a sick run spends the rest of it.
drawn=0
died=no
for _ in $(seq 90); do
  if ! still_running "$pid"; then
    died=yes
    echo "the window exited on its own, before it drew anything — so what it"
    echo "printed below is the failure, and the picture is only the proof."
    break
  fi
  snap "$shot" 2>/dev/null || true
  if [ -f "$shot" ]; then
    colours=$(count "$shot" 2>/dev/null || echo 0)
    if [ "${colours:-0}" -gt 8 ]; then
      drawn=$colours
      break
    fi
  fi
  sleep 1
done

# One more after it has settled, so what is kept is the window rather than the
# first frame of it.
if [ "$drawn" -gt 0 ]; then
  sleep 2
  snap "$shot" 2>/dev/null || true
fi

kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true

said() {
  if [ -s "$log" ]; then
    echo
    echo "--- the window said ---"
    cat "$log"
  fi
}

if [ ! -f "$shot" ]; then
  echo "nothing was photographed at all — the screen could not be read"
  said
  exit 1
fi

colours=$(count "$shot")
echo "$colours distinct colours on the screen"

# Eight is not a threshold anybody tuned, and it does not need to be: an empty
# Xvfb root is **one** colour, a white window is one or two, and a drawn page of
# antialiased Hebrew is in the hundreds. Anything in between is a state nobody
# has seen. The number is printed above either way, so the next person to
# disagree with this line is arguing with a measurement.
if [ "$colours" -le 8 ]; then
  echo
  # Two different failures reach this line and they want different sentences.
  # Saying *the process stayed perfectly healthy* about a process that aborted
  # in the first second sends the next reader looking in the wrong place, which
  # is what the 17 August run did.
  if [ "$died" = yes ]; then
    echo "The window did not stay up. The blank screen is a consequence and the"
    echo "lines below are the cause — read those first."
  else
    echo "The window opened, stayed up, and drew nothing. That is the failure"
    echo "this script exists for, because a process that lived is evidence of"
    echo "nothing: WebKitGTK on NixOS composites through its own sandbox and"
    echo "cannot reach the store paths it needs from inside one."
  fi
  echo
  echo "What it was given is printed above, before 'opening the window'. All of"
  echo "WEBKIT_DISABLE_COMPOSITING_MODE, WEBKIT_DISABLE_DMABUF_RENDERER and"
  echo "LIBGL_ALWAYS_SOFTWARE are set by this script whether or not the shell"
  echo "supplied them, so an unset one there is a bug in this file."
  said
  exit 1
fi

echo "a WebKitGTK surface was opened and drawn on, on a machine with no FHS"
