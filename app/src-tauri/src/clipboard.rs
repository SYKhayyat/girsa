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
use girsa_app::trouble::{refuse, Code};
use girsa_source::CLIPBOARD_MIME;
use serde::Serialize;

/// What reached the clipboard.
///
/// # Three fields, and one fact
///
/// This used to promise that what reached the clipboard is *"reported rather
/// than assumed"*, and it was not: `put` sets all three from a single `Ok(())`.
/// The sentence and the code disagreed, and the sentence was the one making a
/// claim a reader could act on.
///
/// The code is right and the sentence was wrong. `clipboard-rs` sets the whole
/// list **inside one open** — which is the reason it is the dependency rather
/// than two smaller ones, and the module note above says so — so the platform
/// either took the list or it did not. There is no partial answer to report and
/// no way to ask for one.
///
/// So the three fields are one fact in three places, and they stay three
/// because the *window* has three things to say: a paste into Ksav that arrives
/// as plain text is a different disappointment from one into Word that arrives
/// unformatted, and a future clipboard backend that does put flavours down one
/// at a time would fill these in separately without changing anything that
/// reads them. What has gone is the claim that they are already independent.
#[derive(Debug, Default, Serialize)]
pub struct Put {
    /// `text/plain` — WhatsApp, a terminal, anything.
    pub plain: bool,
    /// `text/html` — Word, an email, a browser.
    pub html: bool,
    /// `application/x-girsa-source+json` — Ksav.
    pub packet: bool,
    /// Why something is missing — **coded**, so the window says it in the
    /// reader's language rather than showing this.
    ///
    /// These three were the last user-facing sentences this crate composed in
    /// English, and one of them reached a Hebrew right-to-left toast as
    /// `the clipboard refused it: Empty clipboard error, code = OSError(1418):
    /// Thread does not have a clipboard open.` — a raw Windows error number, in
    /// English, as the reader's whole explanation. `girsa_app::trouble::refuse`
    /// puts a name in front of the machine's words; `app/src/trouble.ts` turns
    /// the name into a sentence and keeps the machine's words on the hover,
    /// which is where they belong.
    pub trouble: Option<String>,
}

/// Put the three flavours down.
pub fn put(sent: &Sent) -> Put {
    let json = match sent.packet.to_json() {
        Ok(json) => json,
        Err(e) => {
            return Put {
                trouble: Some(refuse(Code::WillNotSerialize, e)),
                ..Put::default()
            }
        }
    };

    let context = match ClipboardContext::new() {
        Ok(context) => context,
        Err(e) => {
            return Put {
                trouble: Some(refuse(Code::NoClipboard, e)),
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
        // All three or none: one `set`, one open, one answer. See the note on
        // `Put` for why this is honest rather than lazy.
        Ok(()) => Put {
            plain: true,
            html: true,
            packet: true,
            trouble: None,
        },
        Err(e) => Put {
            trouble: Some(refuse(Code::ClipboardRefused, e)),
            ..Put::default()
        },
    }
}
