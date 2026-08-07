//! Handing somebody a sefer with your corrections in it (spec.md §7.4, W22).
//!
//! *Base text + applied patches → a clean `.txt`/`.docx`. Falls out of §4.1 for
//! free.* It does, and this module is the proof of that claim rather than a
//! feature in its own right: the text is already text, the corrections are
//! already an overlay, and a sefer read through [`girsa_app::shelf::Open`] is
//! already corrected. What is left is writing it down.
//!
//! # What "clean" means here
//!
//! - the words as the page shows them — the corpus's inline markup gone, the
//!   nikud as you are reading it;
//! - headings still headings, so the file has the shape the sefer has;
//! - and **four lines at the top saying what this is**: which sefer, from
//!   where, how many corrections were applied and how many were not.
//!
//! That header is not decoration. A corrected sefer that does not say it was
//! corrected is a text somebody will quote as though it were the printed
//! edition, and the whole design of §7.1 is that a correction is a claim
//! somebody made rather than a fact about the sefer.
//!
//! # Why the `.docx` is written by hand
//!
//! A `.docx` is a zip with an XML part in it, and `girsa-corpus` already opens
//! them to read a Word file you dropped on the window. Writing one is the same
//! two files backwards, and taking a docx library for it would be a dependency
//! carrying a rendering model this app has no use for. What matters is that the
//! paragraphs are marked **right-to-left** — `w:bidi` and `w:rtl`, which is what
//! Word needs in order not to lay a Hebrew line out backwards — and that a
//! heading declares `w:pStyle`, which is exactly what the importer reads back.

use std::io::Write;
use std::path::{Path, PathBuf};

use girsa_fix::{Layer, Showing};

use girsa_app::display;
use girsa_app::shelf::Open;

/// What to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Txt,
    Docx,
}

impl Format {
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Docx => "docx",
        }
    }

    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        match word {
            "txt" => Some(Self::Txt),
            "docx" => Some(Self::Docx),
            _ => None,
        }
    }
}

/// What came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    pub path: PathBuf,
    pub segments: usize,
    /// Corrections that are in the words of the file.
    pub corrections: usize,
    /// Corrections that are **not**, because the text they were made against is
    /// no longer there (W20). Counted here and named in the file: exporting is
    /// the moment somebody would otherwise never find out.
    pub stale: usize,
    /// Variants noted rather than applied, under the setting the sefer was read
    /// with.
    pub noted: usize,
}

/// Why a sefer could not be written out.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("writing {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> ExportError + '_ {
    move |source| ExportError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Write a sefer out, with whatever corrections were applied when it was read.
///
/// The `Open` handed in **is** the corrected sefer — that is where the patches
/// were applied (see [`Open::corrected`]) — so this does not apply anything. The
/// layer is here only to count what did not land.
///
/// # Errors
///
/// If the file cannot be written.
pub fn export(
    sefer: &Open,
    fixes: &Layer,
    format: Format,
    nikud: bool,
    to: &Path,
) -> Result<Exported, ExportError> {
    let mut done = Exported {
        path: to.to_path_buf(),
        segments: sefer.segments.len(),
        corrections: 0,
        stale: 0,
        noted: 0,
    };
    for segment in &sefer.segments {
        if let Some(corrected) = sefer.correction(&segment.id) {
            done.corrections += corrected.applied.len();
            done.stale += corrected.stale.len();
            done.noted += corrected.noted.len();
        }
    }
    // A patch on a segment this sefer no longer has at all — an upstream
    // re-segmentation — never reaches `Open`, so it is counted from the layer.
    let anchored: usize = sefer.segments.iter().map(|s| fixes.on(&s.id).len()).sum();
    let all = fixes.in_work(sefer.slug()).count();
    done.stale += all.saturating_sub(anchored);

    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(io(dir))?;
    }
    let lines: Vec<Line> = sefer
        .segments
        .iter()
        .map(|segment| Line {
            heading: segment.kind == girsa_corpus::import::SegmentKind::Heading,
            words: display::Shown::of(&segment.text, nikud).text().to_string(),
        })
        .filter(|line| !line.words.trim().is_empty())
        .collect();

    match format {
        Format::Txt => write_txt(sefer, &done, &lines, to),
        Format::Docx => write_docx(sefer, &done, &lines, to),
    }?;
    Ok(done)
}

struct Line {
    heading: bool,
    words: String,
}

/// The four lines at the top: which sefer, from where, and what was done to it.
fn header(sefer: &Open, done: &Exported) -> Vec<String> {
    let mut out = vec![sefer.work.he_title.clone()];
    if !sefer.work.en_title.is_empty() && sefer.work.en_title != sefer.work.he_title {
        out.push(sefer.work.en_title.clone());
    }
    let mut from = format!("מקור: {}", sefer.work.source.as_str());
    if let Some(version) = sefer.work.version.as_ref() {
        // Which printed edition this is, and under what terms — spec.md §13
        // says every text carries them, and a file leaving the app is the one
        // place that has to be true outside it.
        from.push_str(&format!(" · {}", version.edition));
        if let Some(license) = version.license.as_deref() {
            from.push_str(&format!(" · {license}"));
        }
    }
    out.push(from);
    if done.corrections > 0 || done.stale > 0 || done.noted > 0 {
        out.push(what_was_done(done));
    }
    out
}

/// The one sentence that keeps a corrected sefer honest.
///
/// Counted in words rather than digits for the small numbers, because this is a
/// line a person reads at the top of a sefer and `1 תיקונים` is not Hebrew.
fn what_was_done(done: &Exported) -> String {
    let mut said = format!("הוחלו {}", how_many(done.corrections, "תיקון", "תיקונים"));
    if done.noted > 0 {
        said.push_str(&format!(
            " · {} שנרשמו ולא הוחלו",
            how_many(done.noted, "גרסה", "גרסאות")
        ));
    }
    if done.stale > 0 {
        said.push_str(&format!(
            " · {} שלא חל, משום שהטקסט שתוקן אינו שם עוד",
            how_many(done.stale, "תיקון", "תיקונים")
        ));
    }
    said
}

fn how_many(n: usize, one: &str, many: &str) -> String {
    match n {
        1 => format!("{one} אחד"),
        2 => format!("שני {many}"),
        _ => format!("{n} {many}"),
    }
}

fn write_txt(sefer: &Open, done: &Exported, lines: &[Line], to: &Path) -> Result<(), ExportError> {
    let mut body = String::new();
    for line in header(sefer, done) {
        body.push_str(&line);
        body.push('\n');
    }
    body.push('\n');
    for line in lines {
        if line.heading {
            body.push('\n');
        }
        body.push_str(&line.words);
        body.push('\n');
    }
    std::fs::write(to, body).map_err(io(to))
}

// ---------------------------------------------------------------------------
// .docx
// ---------------------------------------------------------------------------

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

fn write_docx(sefer: &Open, done: &Exported, lines: &[Line], to: &Path) -> Result<(), ExportError> {
    let mut document = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
    );
    for (n, line) in header(sefer, done).iter().enumerate() {
        // The title is a heading, so the file opens looking like a sefer; the
        // provenance under it is quiet, so it does not look like the text.
        document.push_str(&paragraph(line, n == 0, n > 0));
    }
    for line in lines {
        document.push_str(&paragraph(&line.words, line.heading, false));
    }
    document.push_str("</w:body></w:document>");

    let file = std::fs::File::create(to).map_err(io(to))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, body) in [
        ("[Content_Types].xml", CONTENT_TYPES),
        ("_rels/.rels", RELS),
        ("word/document.xml", document.as_str()),
    ] {
        zip.start_file(name, options)
            .map_err(|e| zipped(to, &e.to_string()))?;
        zip.write_all(body.as_bytes()).map_err(io(to))?;
    }
    zip.finish().map_err(|e| zipped(to, &e.to_string()))?;
    Ok(())
}

fn zipped(to: &Path, why: &str) -> ExportError {
    ExportError::Io {
        path: to.display().to_string(),
        source: std::io::Error::other(why.to_string()),
    }
}

/// One paragraph, right to left.
///
/// `w:bidi` on the paragraph and `w:rtl` on the run: without them Word lays a
/// Hebrew line out left to right, which puts the punctuation at the wrong end
/// and reads as a broken file rather than as a setting.
fn paragraph(text: &str, heading: bool, quiet: bool) -> String {
    let mut properties = String::from("<w:pPr><w:bidi/>");
    if heading {
        // The style the importer reads back (`girsa_corpus::import::mine`), and
        // the one Word knows.
        properties.push_str(r#"<w:pStyle w:val="Heading1"/>"#);
    }
    properties.push_str("</w:pPr>");

    let mut run_properties = String::from("<w:rPr><w:rtl/>");
    if heading {
        run_properties.push_str("<w:b/><w:sz w:val=\"32\"/>");
    }
    if quiet {
        run_properties.push_str("<w:i/><w:sz w:val=\"18\"/>");
    }
    run_properties.push_str("</w:rPr>");

    format!(
        "<w:p>{properties}<w:r>{run_properties}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        girsa_app::markup::text(text)
    )
}

/// A file name for a sefer, for the window's save dialog to start from.
#[must_use]
pub fn suggested_name(sefer: &Open, format: Format) -> String {
    let title = if sefer.work.he_title.trim().is_empty() {
        sefer.work.slug.replace('/', "-")
    } else {
        sefer.work.he_title.trim().to_string()
    };
    let clean: String = title
        .chars()
        .map(|c| {
            if ['<', '>', ':', '"', '\\', '|', '?', '*', '/'].contains(&c) {
                '-'
            } else {
                c
            }
        })
        .collect();
    format!("{clean}.{}", format.extension())
}

/// Which corrections the sefer was read with, said in words for the window.
#[must_use]
pub fn showing_said(showing: Showing) -> &'static str {
    match showing {
        Showing::AsPrinted => "כפי שנדפס",
        Showing::Fixed => "מתוקן",
        Showing::FixedWithVariants => "מתוקן, עם גרסאות",
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn what_was_done_reads_as_hebrew_for_the_small_numbers() {
        let done = |corrections, stale, noted| Exported {
            path: PathBuf::new(),
            segments: 0,
            corrections,
            stale,
            noted,
        };
        assert_eq!(what_was_done(&done(1, 0, 0)), "הוחלו תיקון אחד");
        assert_eq!(what_was_done(&done(2, 0, 0)), "הוחלו שני תיקונים");
        assert_eq!(what_was_done(&done(7, 0, 0)), "הוחלו 7 תיקונים");
        assert!(what_was_done(&done(1, 1, 0)).contains("תיקון אחד שלא חל"));
        assert!(what_was_done(&done(1, 0, 3)).contains("3 גרסאות שנרשמו"));
    }

    #[test]
    fn a_paragraph_is_right_to_left_and_a_heading_says_so() {
        let plain = paragraph("שלום", false, false);
        assert!(plain.contains("<w:bidi/>"), "{plain}");
        assert!(plain.contains("<w:rtl/>"), "{plain}");
        assert!(!plain.contains("pStyle"), "{plain}");

        let heading = paragraph("סימן א", true, false);
        assert!(
            heading.contains(r#"<w:pStyle w:val="Heading1"/>"#),
            "{heading}"
        );
    }

    #[test]
    fn the_markup_of_a_word_file_cannot_be_broken_by_the_text_in_it() {
        // A sefer really does contain `<`, and a segment carrying one would end
        // the run element and produce a file Word refuses to open.
        let escaped = paragraph("שווה < משהו & עוד", false, false);
        assert!(escaped.contains("שווה &lt; משהו &amp; עוד"), "{escaped}");
    }
}
