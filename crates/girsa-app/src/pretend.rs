//! A sefer to test against, without a corpus on the machine.
//!
//! # Why this is a module and not a `#[cfg(test)]` helper
//!
//! It was one: `sending::tests::shulchan_arukh`, reachable from `buffer.rs`
//! because `mod tests` was `pub(crate)`. That works exactly as far as the crate
//! boundary, and the moment the Ksav side moved out to `girsa-desk` the two
//! tests that need this sefer were on the other side of it — with the usual
//! two options, copy the fixture or weaken the test.
//!
//! So it is a real module behind an off-by-default feature. `cargo test -p
//! girsa-app` compiles it because of the `test` half of the `cfg`; anyone else
//! asks for it in `[dev-dependencies]`, which is where a fixture belongs and
//! where it cannot reach a reader's build:
//!
//! ```toml
//! [dev-dependencies]
//! girsa-app = { workspace = true, features = ["pretend"] }
//! ```
//!
//! [`girsa_fixture`] is the other half of this and stays what it is: it writes
//! a whole shelf to disk through the real import path. This is the small one —
//! an [`Open`] in memory, no files, for a test that is about what a quote says
//! rather than about what is on the shelf.

use std::path::PathBuf;

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_corpus::work::{Source, Version as WorkVersion, Work};

use crate::shelf::Open;

/// A sefer with the text given, addressed `1:1`, `1:2`, …
#[must_use]
pub fn sefer(slug: &str, he_title: &str, sections: &[&str], texts: &[&str]) -> Open {
    let work = Work {
        slug: slug.to_string(),
        he_title: he_title.to_string(),
        en_title: "A Sefer".to_string(),
        categories: Vec::new(),
        order: Vec::new(),
        source: Source::Sefaria,
        origin: PathBuf::new(),
        schema: None,
        he_sections: sections.iter().map(|s| (*s).to_string()).collect(),
        author: None,
        era: None,
        comp_date: None,
        version: Some(WorkVersion {
            edition: "Maginei Eretz: Shulchan Aruch Orach Chaim, Lemberg, 1893".into(),
            provenance: Some("https://www.sefaria.org/".into()),
            license: Some("Public Domain".into()),
        }),
        commentary_on: Vec::new(),
    };
    let segments = texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            #[allow(clippy::cast_possible_truncation)]
            let n = i as u32 + 1;
            Segment {
                id: SegmentId::new(slug, vec!["1".into(), n.to_string()], Ordinal::root(n)),
                kind: SegmentKind::Text,
                text: (*text).to_string(),
                anchors: Vec::new(),
            }
        })
        .collect();
    Open::new(work, segments)
}

/// Three se'ifim of שולחן ערוך אורח חיים, with nikud and one bit of markup.
///
/// The nikud and the `<b>` are the point: what a quote does with them is the
/// difference between the words a reader highlighted and the words the corpus
/// happens to store.
#[must_use]
pub fn shulchan_arukh() -> Open {
    sefer(
        "shulchan-arukh/orach-chayim",
        "שולחן ערוך, אורח חיים",
        &["סימן", "סעיף"],
        &[
            "יִתְגַּבֵּר כָּאֲרִי לַעֲמוֹד בַּבֹּקֶר לַעֲבוֹדַת בּוֹרְאוֹ",
            "<b>שִׁוִּיתִי</b> ה' לְנֶגְדִּי תָמִיד",
            "הַמַּשְׁכִּים לַעֲמוֹד",
        ],
    )
}
