import { defineConfig } from "vite";

// Port 5174 so it never collides with Ksav's dev server on 5173 — the two are
// meant to be running at the same time (spec.md §10).
//
// `publicDir` is off for a production build on purpose. The only thing in
// `public/` is `dev/`, the browser fixtures — a few megabytes of real Gemara
// written there by `cargo run -p girsa-app --example dev-fixtures`. They exist
// so the page can be looked at outside the shell; shipping them inside the
// installed app would mean every reader carries a second, frozen copy of
// Berakhot that nothing ever updates.
export default defineConfig(({ command }) => ({
  clearScreen: false,
  server: { port: 5174, strictPort: true },
  publicDir: command === "build" ? false : "public",
  build: { target: "es2022", sourcemap: true },
}));
