{
  # Girsa on NixOS.
  #
  # ## Why a flake and not "the AppImage works"
  #
  # It mostly does, under `appimage-run`, and that is a fine way to *try* Girsa.
  # It is a poor way to build it, and building it is the thing NixOS makes hard
  # for a Tauri application: the shell links against WebKitGTK, GTK 3, libsoup
  # and OpenSSL, and on NixOS none of those are at a path a `pkg-config` run
  # inherited from anywhere else will find. Without a declared environment the
  # first `cargo build` fails inside `webkit2gtk-sys` with a message about a
  # `.pc` file, which reads as a Rust problem and is a packaging one.
  #
  # So this declares the environment, and nothing else. It is a `devShell`, not
  # a package: Girsa's build is `npm ci` then `cargo tauri build`, and wrapping
  # that in a derivation means vendoring both a `node_modules` and a Cargo
  # registry for a repository whose own `tools/verify.mjs` is the gate. The
  # shell gets a NixOS reader to exactly where a Debian reader stands after the
  # `apt-get install` in `.github/workflows/ci.yml`, and from there every
  # instruction in `docs/start-here.md` is the same on both.
  #
  # ## The list is the CI list
  #
  # Package for package, from the Linux row of the bundle matrix — that file is
  # the one that has been run — with the Nix names beside the Debian ones so a
  # change to either can be checked against the other:
  #
  # | Debian                          | Nix                        |
  # |---------------------------------|----------------------------|
  # | `libwebkit2gtk-4.1-dev`         | `webkitgtk_4_1`            |
  # | `libgtk-3-dev`                  | `gtk3`                     |
  # | `librsvg2-dev`                  | `librsvg`                  |
  # | `libssl-dev`                    | `openssl`                  |
  # | `libxdo-dev`                    | `xdotool`                  |
  # | `libayatana-appindicator3-dev`  | `libayatana-appindicator`  |
  # | `patchelf`                      | `patchelf`                 |
  #
  # `libappindicator3-dev` is deliberately absent here too, and for the reason
  # `ci.yml` gives at length: it and the ayatana package conflict, and Tauri v2
  # wants ayatana.
  #
  # `libsoup_3` is in the list and not in Debian's, because on Debian it arrives
  # as a dependency of `libwebkit2gtk-4.1-dev` and Nix does not hand you a
  # library's dependencies to link against.
  #
  # ## What proves any of this
  #
  # `.github/workflows/ci.yml` has a `nixos` job, and it runs **inside the
  # `nixos/nix` container** rather than on `ubuntu-latest` with Nix installed
  # beside apt. That distinction is the whole value of the job: Ubuntu has a
  # `/usr/lib`, so a build there can quietly link against a system library this
  # file never declared and pass, and then fail on a machine that has no
  # `/usr/lib` at all. The container has no FHS. Anything missing from the two
  # lists below has nowhere to come from, and the job says so.
  description = "Girsa — a Torah library that reads like a sefer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # What the shell links against. Kept in one list because it is needed
        # twice — once to build and once, through `LD_LIBRARY_PATH`, to run the
        # binary that was built, which is the half a reader hits second and
        # least expects.
        libraries = with pkgs; [
          webkitgtk_4_1
          gtk3
          cairo
          gdk-pixbuf
          glib
          dbus
          openssl
          librsvg
          libsoup_3
          libayatana-appindicator
        ];

        tools = with pkgs; [
          # Rust from nixpkgs rather than a pinned toolchain: the workspace
          # states no channel, and a reader who wants one has rustup.
          cargo
          rustc
          rustfmt
          clippy
          rust-analyzer

          # The window. `tools/verify.mjs` and every `npm` script want 22.
          nodejs_22

          pkg-config
          patchelf
          xdotool

          # A display, for a machine that has none.
          #
          # `docs/not-yet.md` has said since the job was written that *a
          # container has no display, so nothing there has opened a WebKitGTK
          # surface* — and that sentence contains its own answer. A container
          # has no display until somebody starts one. `xvfb-run` is the X server
          # that exists to be nobody's screen, and `imagemagick`'s `import`
          # reads back what was drawn on it.
          #
          # Which matters here more than it does on most platforms, because the
          # failure this is looking for is not a crash. `WEBKIT_DISABLE_COMPOSITING_MODE`
          # is a line every Tauri application on NixOS carries; without it the
          # window opens, stays up, exits cleanly — and is **white**. A process
          # that lives is not evidence. A picture of what it drew is.
          xvfb-run
          imagemagick

          # `appimagekit`, `dpkg` and `fakeroot` were here, on the reasoning
          # that `cargo tauri build` shells out to them for the AppImage and
          # the .deb. It does not, and the CI job caught it the way the note in
          # `ci.yml` predicted a break would arrive: `error: undefined variable
          # 'appimagekit'`, an evaluation failure and not a compiler one.
          # nixpkgs removed the package without an alias — at nixpkgs
          # `e5bdc4a` only `libappimage`, `appimageupdate`, `appimage-run` and
          # `appimageTools` remain — so the entry could not have been silently
          # doing nothing for much longer either way.
          #
          # What `tauri-bundler` actually runs, read out of its source rather
          # than guessed at (2.9.4, `src/bundle/linux/`): the .deb and the .rpm
          # are written in Rust, archive headers and all, and the only external
          # program either of them names is none. The AppImage is
          # `linuxdeploy`, which the bundler **downloads itself** from
          # `tauri-apps/binary-releases` into a tools directory, along with
          # `AppRun`, the GTK plugin and the AppImage plugin, and then runs with
          # `APPIMAGE_EXTRACT_AND_RUN=1`. Nothing on `PATH` participates.
          #
          # Which means the AppImage row of the bundle matrix is the one thing
          # this shell cannot give a NixOS reader: `linuxdeploy` is a prebuilt
          # glibc ELF naming `/lib64/ld-linux-x86-64.so.2`, the same interpreter
          # `npm ci`'s binaries name and the same one this machine does not
          # have — and unlike `node_modules`, it is fetched during the build and
          # so is not there to be patched beforehand. `--bundles deb` is what
          # the banner below therefore says.
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = libraries ++ tools;
          # `autoPatchelf` is a shell function this hook puts on the path. It is
          # here for `node_modules`, not for anything Nix builds — see the note
          # in `shellHook`.
          nativeBuildInputs = [ pkgs.autoPatchelfHook ];

          # WebKitGTK on NixOS composites through its own sandbox and cannot
          # reach the store paths it needs from inside one. Every Tauri
          # application on NixOS carries this line; without it the window opens
          # and stays white, which is not an error anybody can search for.
          WEBKIT_DISABLE_COMPOSITING_MODE = "1";
          WEBKIT_DISABLE_DMABUF_RENDERER = "1";

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
            export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS

            # ── The half that has nothing to do with Tauri ──────────────────
            #
            # `npm ci` downloads **prebuilt ELF executables** — esbuild's and
            # rollup's, which Vite runs — and every one of them names
            # `/lib64/ld-linux-x86-64.so.2` as its interpreter. That path does
            # not exist on NixOS. What a reader sees is `npm run build` failing
            # with `No such file or directory` about a file that is plainly
            # there, which is the least searchable error message in computing.
            #
            # So `npm ci` is wrapped: install, then rewrite the interpreter of
            # everything it just unpacked. Wrapped rather than printed as an
            # instruction, because an instruction in a shell banner is an
            # instruction somebody does not read.
            npm() {
              command npm "$@"
              status=$?
              case "$1" in
                ci|install|i)
                  if [ $status -eq 0 ] && [ -d node_modules ]; then
                    echo "girsa: patching node_modules for a machine with no /lib64"
                    autoPatchelf node_modules 2>/dev/null || true
                  fi
                  ;;
              esac
              return $status
            }

            echo "Girsa: cargo and node are here. Build the window with"
            echo "  cd app && npm ci && npm run tauri build -- --bundles deb"
            echo "and run the gate with"
            echo "  node tools/verify.mjs"
            echo ""
            echo "(--bundles deb because the AppImage step downloads a glibc"
            echo " linuxdeploy this machine has no loader for. See the note by"
            echo " the tools list.)"
          '';
        };
      });
}
