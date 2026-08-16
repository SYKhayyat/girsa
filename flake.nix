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

          # `cargo tauri build` produces the AppImage and the .deb with these.
          # Absent, the build succeeds and the bundle step fails at the end,
          # which is the worst place to find out.
          appimagekit
          dpkg
          fakeroot
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = libraries ++ tools;

          # WebKitGTK on NixOS composites through its own sandbox and cannot
          # reach the store paths it needs from inside one. Every Tauri
          # application on NixOS carries this line; without it the window opens
          # and stays white, which is not an error anybody can search for.
          WEBKIT_DISABLE_COMPOSITING_MODE = "1";
          WEBKIT_DISABLE_DMABUF_RENDERER = "1";

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
            export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS
            echo "Girsa: cargo and node are here. Build the window with"
            echo "  cd app && npm ci && npm run tauri build"
            echo "and run the gate with"
            echo "  node tools/verify.mjs"
          '';
        };
      });
}
