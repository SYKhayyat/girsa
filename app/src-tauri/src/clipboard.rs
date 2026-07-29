//! Putting three flavours down in one Ctrl+C (spec.md §10.2, BUILDER.md W15).
//!
//! **What** is put down is decided one directory up, in
//! [`girsa_app::sending`], where it can be tested. This module is the part
//! that touches the operating system, and it is here for one reason: the
//! flavour Ksav takes has to be a **real clipboard format**, registered under
//! `application/x-girsa-source+json`, so that a native application reading the
//! clipboard finds it.
//!
//! A webview cannot do that. `navigator.clipboard.write` will take a custom
//! type, but Chromium puts it down as a *web custom format* — a private
//! encoding that another browser tab can read and a native application cannot.
//! Written from the webview, Ksav would see the plain text and nothing else,
//! and the pairing would look like it worked.
//!
//! # All three, in one open
//!
//! On Windows a clipboard write is: open, empty, then one `SetClipboardData`
//! per format. Two libraries taking turns means the second one empties what
//! the first put down, and the failure looks like the rich flavour silently
//! not being there. `clipboard-rs` sets the whole list inside one open, which
//! is why it is the dependency rather than two smaller ones.

use clipboard_rs::{Clipboard, ClipboardContent, ClipboardContext};
use girsa_app::sending::Sent;
use girsa_source::CLIPBOARD_MIME;
use serde::Serialize;

/// What actually reached the clipboard.
///
/// Reported rather than assumed. A copy that put down two flavours out of
/// three is a paste into Ksav that arrives as plain text — which looks like
/// Ksav ignoring the source, and is the kind of thing a reader would never
/// think to check.
#[derive(Debug, Default, Serialize)]
pub struct Put {
    /// `text/plain` — WhatsApp, a terminal, anything.
    pub plain: bool,
    /// `text/html` — Word, an email, a browser.
    pub html: bool,
    /// `application/x-girsa-source+json` — Ksav.
    pub packet: bool,
    /// Why something is missing, in words the window can show.
    pub trouble: Option<String>,
}

/// Put the three flavours down.
pub fn put(sent: &Sent) -> Put {
    let json = match sent.packet.to_json() {
        Ok(json) => json,
        Err(e) => {
            return Put {
                trouble: Some(format!("the source packet would not serialize: {e}")),
                ..Put::default()
            }
        }
    };

    let context = match ClipboardContext::new() {
        Ok(context) => context,
        Err(e) => {
            return Put {
                trouble: Some(format!("no clipboard here: {e}")),
                ..Put::default()
            }
        }
    };

    let contents = vec![
        ClipboardContent::Text(sent.plain.clone()),
        ClipboardContent::Html(sent.html.clone()),
        ClipboardContent::Other(CLIPBOARD_MIME.to_string(), json.into_bytes()),
    ];
    match context.set(contents) {
        Ok(()) => Put {
            plain: true,
            html: true,
            packet: true,
            trouble: None,
        },
        Err(e) => Put {
            trouble: Some(format!("the clipboard refused it: {e}")),
            ..Put::default()
        },
    }
}
