fn main() {
    tauri_build::build();
    refuse_a_release_build_that_would_open_on_a_dev_server();
}

/// The build that produced a window with no application in it.
///
/// > *"`cargo build --release -p girsa-shell` — the command this document's own
/// > appendix recommended, and the command used for every measurement in Part 2
/// > — does not produce a window that contains Girsa. It produces a window that
/// > navigates to `http://localhost:5174`."*
///
/// With the dev server up you cannot tell. With it down the application is a
/// Chromium *this site can't be reached* page inside a window titled
/// `גִּרְסָא · Girsa`. Every measurement anybody in this repository had ever
/// taken came from a development server, and the way that survived is that the
/// wrong command **succeeds** — it prints `Finished` and writes an executable,
/// and the executable looks like the product right up until you unplug the
/// thing it was quietly leaning on.
///
/// So the wrong command stops succeeding. `tauri` decides `dev` from whether
/// `custom-protocol` is on, which only the Tauri CLI turns on, and publishes the
/// answer to build scripts as `DEP_TAURI_DEV` — the same one bit the window
/// itself is compiled against, so this cannot disagree with what the binary
/// would have done. In a debug profile it is silent: `cargo check`, `cargo
/// clippy` and `tauri dev` all want exactly that binary, and CI's shell job runs
/// two of them.
///
/// `GIRSA_DEV_RELEASE=1` builds it anyway, for whoever is profiling the release
/// profile and does not care what the webview points at. It is an env var rather
/// than a cargo feature because it is a statement about this one invocation, and
/// the person who needs it is standing at the terminal.
fn refuse_a_release_build_that_would_open_on_a_dev_server() {
    if std::env::var_os("GIRSA_DEV_RELEASE").is_some() {
        return;
    }
    if std::env::var("PROFILE").as_deref() != Ok("release") {
        return;
    }
    if !tauri_build::is_dev() {
        return;
    }
    panic!(
        "\n\
         This release build would embed no frontend and open on the Vite dev\n\
         server at http://localhost:5174 — a `this site can't be reached` page\n\
         in a window called Girsa. It is not a build of this application.\n\
         \n\
         Build it with the Tauri CLI, which turns on `custom-protocol` and puts\n\
         `app/dist` inside the binary:\n\
         \n\
             cd app && npx tauri build              # with an installer\n\
             cd app && npx tauri build --no-bundle  # just the executable\n\
         \n\
         See docs/the-second-sitting.md, finding 16. If you meant it — profiling\n\
         the release profile, say — set GIRSA_DEV_RELEASE=1.\n"
    );
}
