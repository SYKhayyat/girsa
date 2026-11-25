//! Whether there is a newer Girsa, and the releases page.
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

#[tauri::command(async)]
pub(crate) fn check_for_update() -> Result<girsa_app::newer::Newer, String> {
    girsa_app::newer::check(env!("CARGO_PKG_VERSION"))
        .map_err(|e| girsa_app::trouble::refuse(girsa_app::trouble::Code::Offline, e.to_string()))
}

/// Open the releases page on the machine's own browser.
///
/// No argument, on purpose: [`girsa_app::newer::open_releases`] opens one
/// address, compiled in. A command that opened whatever URL it was handed is a
/// command that opens whatever a bug hands it.
#[tauri::command(async)]
pub(crate) fn open_releases() -> Result<(), String> {
    girsa_app::newer::open_releases()
        .map_err(|e| girsa_app::trouble::refuse(girsa_app::trouble::Code::Offline, e.to_string()))
}
