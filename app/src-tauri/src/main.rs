// The console window on Windows is hidden for a release build and kept for a
// debug one, where the messages about a missing shelf are worth seeing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    girsa_shell_lib::run();
}
