//! Your own seforim: a file dropped in becomes one.
//!
//! spec.md §5: *PDF, DOCX and TXT dropped in at any time — **not an onboarding
//! step, not a second-class attachment.** Searchable, ref'd, citable and
//! linkable like anything shipped.*
//!
//! So a dropped file goes through the same door as Shas: it is parsed into
//! [`RawSegment`]s, [`ImportedWork::assemble`] gives every one of them a
//! permanent id (spec.md §3), and [`crate::import::write`] puts it on disk in
//! the same two files as every other work. Nothing downstream — the shelf, the
//! panes, the link graph, later the index — needs to know which of the three
//! sources a work came from.
//!
//! # Two places it is deliberately not clever
//!
//! **A scan has no words.** A PDF becomes a work with one segment per page and
//! **no text at all** until it is OCR'd (W26). Guessing at the text with a
//! parser that does not know the font's encoding would put invented Hebrew
//! into a sefer, permanently, under a real segment id — the worst thing this
//! codebase can do. spec.md §9.7 already says what to do instead: the page is
//! on the shelf, addressable and citable, and search says *"this one is not
//! searchable yet"* rather than quietly returning nothing.
//!
//! **A paragraph is a paragraph.** A .txt is split where its author left blank
//! lines, and nothing here reads a line and decides it looks like a heading. A
//! .docx keeps the headings **Word was told about** — `w:pStyle` — and invents
//! none.
//!
//! # Where it goes
//!
//! Under the personal root, never the corpus root:
//!
//! ```text
//! personal/works/index.jsonl        your catalogue
//! personal/works/user/<slug>/…      work.json + segments.jsonl
//! personal/files/<slug>.<ext>       the file itself, copied in
//! ```
//!
//! The copy is not tidiness. The sefer has to still be there after the file is
//! moved off the desktop it was dropped from — and for a PDF the copy *is* the
//! sefer, since the scan is the text (§6.3).

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::import::{self, ImportError, ImportedWork, RawSegment, SegmentKind};
use crate::work::{self, Source, Version, Work};

/// The extensions this can read.
pub const ACCEPTS: [&str; 4] = ["txt", "docx", "pdf", "ksav"];

/// Why a file did not become a sefer.
#[derive(Debug, thiserror::Error)]
pub enum MineError {
    #[error("{0} is not a kind of file Girsa reads — it reads {kinds}", kinds = ACCEPTS.join(", "))]
    NotAKind(String),
    #[error("there is nothing to read in {0}")]
    Empty(String),
    #[error("{0} will not open as a PDF: {1}")]
    NotAPdf(String, String),
    #[error("{0} will not open as a DOCX: {1}")]
    NotADocx(String, String),
    #[error(transparent)]
    Import(#[from] ImportError),
    #[error("reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// What kind of file it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Plain text, in whatever encoding it turns out to be in.
    Text,
    Docx,
    /// A scan, or anything else a PDF is. Pages, no words.
    Pdf,
    /// **Your own writing** (spec.md §10.4). A Ksav document goes on the shelf
    /// like anything else: searchable, citable, and linkable — which is what
    /// makes the system compound over years instead of being a lookup tool.
    Ksav,
}

impl Kind {
    /// What this file is, by its extension.
    ///
    /// # Errors
    ///
    /// If it is not one of [`ACCEPTS`].
    pub fn of(file: &Path) -> Result<Self, MineError> {
        let extension = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match extension.as_str() {
            "txt" => Ok(Self::Text),
            "docx" => Ok(Self::Docx),
            "pdf" => Ok(Self::Pdf),
            "ksav" => Ok(Self::Ksav),
            _ => Err(MineError::NotAKind(file.display().to_string())),
        }
    }

    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Text => "txt",
            Self::Docx => "docx",
            Self::Pdf => "pdf",
            Self::Ksav => "ksav",
        }
    }
}

/// Put a file on your shelf.
///
/// # Errors
///
/// If the file is of no kind this reads, has nothing in it, will not parse, or
/// the personal layer cannot be written.
pub fn add(personal: &Path, file: &Path, title: Option<&str>) -> Result<ImportedWork, MineError> {
    let kind = Kind::of(file)?;
    let named = title.map(str::trim).filter(|t| !t.is_empty()).map_or_else(
        || {
            file.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("ספר")
                .trim()
                .to_string()
        },
        ToString::to_string,
    );
    let slug = unminted(personal, &named);

    let copied = copy_in(personal, &slug, file, kind)?;
    let (raw, note) = parse(&copied, kind)?;
    if raw.is_empty() {
        // A work with no segments is a defect everywhere else in this codebase
        // (`Counts::empty_works`), and it is one here too. Refused with the
        // file named, rather than put on the shelf as a sefer that opens empty.
        let _ = std::fs::remove_file(&copied);
        return Err(MineError::Empty(file.display().to_string()));
    }

    let work = Work {
        slug,
        he_title: named.clone(),
        en_title: named,
        // spec.md §5's *yours*, and [`crate::taxonomy`] shelves it there.
        categories: vec!["שלי".to_string()],
        source: Source::Mine,
        origin: copied,
        schema: None,
        author: None,
        era: None,
        comp_date: None,
        version: Some(Version {
            edition: note,
            // Where it came from, so that a sefer of yours can say where it
            // came from as precisely as one of Sefaria's does (spec.md §13).
            provenance: Some(file.display().to_string()),
            license: None,
        }),
        he_sections: Vec::new(),
        commentary_on: Vec::new(),
    };

    let imported = ImportedWork::assemble(work, raw);
    import::write(personal, &imported)?;
    import::catalogue(personal, &imported.work)?;
    Ok(imported)
}

/// Re-read one of your seforim from the file itself.
///
/// # Errors
///
/// If the file has gone, or will not parse the way it did when it was added.
pub fn read(work: &Work) -> Result<Vec<RawSegment>, ImportError> {
    let kind =
        Kind::of(&work.origin).map_err(|e| ImportError::malformed(&work.origin, e.to_string()))?;
    parse(&work.origin, kind)
        .map(|(raw, _)| raw)
        .map_err(|e| ImportError::malformed(&work.origin, e.to_string()))
}

/// The segments of a file, and one line about how it was read.
fn parse(file: &Path, kind: Kind) -> Result<(Vec<RawSegment>, String), MineError> {
    match kind {
        Kind::Text => {
            let bytes = std::fs::read(file).map_err(io(file))?;
            let (text, encoding) = decode(&bytes);
            Ok((
                paragraphs(&text),
                format!("your own copy, read as {encoding}"),
            ))
        }
        Kind::Docx => {
            let raw = from_docx(file)?;
            Ok((raw, "your own copy, from a Word document".to_string()))
        }
        Kind::Ksav => {
            let bytes = std::fs::read(file).map_err(io(file))?;
            let (markup, _) = decode(&bytes);
            // Read rather than compiled: Typst is the only thing that can say
            // what a document *renders* as, and the shelf does not need that —
            // it needs the words and their shape. `girsa-ksav` is the same
            // crate that wrote them, so what is indexed is what was written.
            let blocks = girsa_ksav::read(&markup);
            let raw = from_ksav(&blocks);
            let notes = blocks
                .iter()
                .filter(|b| matches!(b, girsa_ksav::Block::Note { .. }))
                .count();
            let rows = blocks
                .iter()
                .filter(|b| matches!(b, girsa_ksav::Block::Row { .. }))
                .count();
            Ok((raw, said(blocks.len(), notes, rows)))
        }
        Kind::Pdf => {
            let pages = pages_in(file)?;
            let raw = (1..=pages)
                .map(|n| RawSegment {
                    path: vec![n.to_string()],
                    kind: SegmentKind::Page,
                    text: String::new(),
                })
                .collect();
            Ok((
                raw,
                format!("your own copy, {pages} pages of scan — no text until it is OCR'd"),
            ))
        }
    }
}

/// A slug nothing on your shelf is using yet.
///
/// Two handouts called `חבורה` are two seforim, and the second one may not
/// land on top of the first: a segment id is permanent, and quietly reusing one
/// would point every note and link anchored to it at somebody else's words.
fn unminted(personal: &Path, title: &str) -> String {
    let base = work::hebrew_slug_of(title);
    let base = if base.is_empty() {
        "ספר".to_string()
    } else {
        base
    };
    // The catalogue is parsed **once**, into the set of slugs it holds.
    //
    // It used to be parsed inside `is_taken`, which is called from a
    // `for n in 2..u32::MAX` loop — so dropping the tenth file called `חבורה` on
    // a shelf of a thousand seforim deserialised a thousand `Work` records nine
    // times over, to answer nine questions of the form "is this string in this
    // set". The right shape for that question is a set.
    let catalogue = std::fs::read_to_string(personal.join("works/index.jsonl")).unwrap_or_default();
    let taken: std::collections::HashSet<String> = catalogue
        .lines()
        .filter_map(|l| serde_json::from_str::<Work>(l).ok())
        .map(|w| w.slug)
        .collect();
    // The `is_file` check stays per-candidate: it is the answer to a *different*
    // question — a work directory on disk that the catalogue does not know about,
    // which is exactly the case where reusing the slug would overwrite somebody's
    // sefer. One `stat` per candidate, and there is almost never more than one.
    let is_taken = |slug: &str| {
        taken.contains(slug) || import::work_dir(personal, slug).join("work.json").is_file()
    };

    let first = format!("user/{base}");
    if !is_taken(&first) {
        return first;
    }
    for n in 2..u32::MAX {
        let next = format!("user/{base}-{n}");
        if !is_taken(&next) {
            return next;
        }
    }
    first
}

/// Copy the file in, so the sefer survives the file being moved away.
fn copy_in(personal: &Path, slug: &str, file: &Path, kind: Kind) -> Result<PathBuf, MineError> {
    let dir = personal.join("files");
    std::fs::create_dir_all(&dir).map_err(io(&dir))?;
    let flat = slug.replace('/', "-");
    let into = import::slug_dir(&dir, &flat).with_extension(kind.extension());
    std::fs::copy(file, &into).map_err(io(file))?;
    Ok(into)
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> MineError + '_ {
    move |source| MineError::Io {
        path: path.display().to_string(),
        source,
    }
}

// ---------------------------------------------------------------------------
// Your own writing
// ---------------------------------------------------------------------------

/// A document of yours as segments — headings as levels of the address, and
/// everything else as a line under them (W29).
///
/// # The headings are the address
///
/// Otzaria's `<h1>/<h2>/<h3>` become a work's structure in
/// [`crate::import::otzaria`], and a `.ksav` is read the same way and for the
/// same reason: a chaburah with three chapters should be cited as
/// `girsa:note/חבורה/מבוא:2#4` and not as line 47. A heading closes every
/// section at its level or deeper and opens its own; the lines after it are
/// numbered within it.
///
/// # A note is not part of the sentence it hangs off
///
/// It gets its own segment, right after the paragraph that carried it, with the
/// marker still in that paragraph's words to say something hangs there. That is
/// the difference between a footnote and an interruption, and it is what makes
/// a note searchable, citable and correctable on its own.
///
/// **An editor's note is left out of the sefer entirely.** `#הערת_עורך` is a
/// remark *about* the text and was never part of it (the same distinction W20
/// draws between a correction and a girsa variant), and importing one as a line
/// of the sefer would put a note-to-self into the corpus as though the author
/// had written it.
#[must_use]
pub fn from_ksav(blocks: &[girsa_ksav::Block]) -> Vec<RawSegment> {
    use girsa_ksav::Block;

    let mut out: Vec<RawSegment> = Vec::new();
    // The open sections, outermost first: their level and the label they are
    // addressed by.
    let mut open: Vec<(u8, String)> = Vec::new();
    // How many lines each section has had, so a line's number is its number
    // *within* its section.
    let mut lines: Vec<usize> = vec![0];

    for block in blocks {
        if let Block::Heading { level, text } = block {
            let level = (*level).max(1);
            while open.last().is_some_and(|(open, _)| *open >= level) {
                open.pop();
                lines.pop();
            }
            let taken: Vec<&str> = open.iter().map(|(_, label)| label.as_str()).collect();
            let label = unique(&crate::work::section_label_of(text), &taken);
            open.push((level, label));
            lines.push(0);
            out.push(RawSegment {
                path: open.iter().map(|(_, label)| label.clone()).collect(),
                kind: SegmentKind::Heading,
                text: text.clone(),
            });
            continue;
        }

        let (kind, text) = match block {
            Block::Paragraph(text) => (SegmentKind::Text, text.clone()),
            Block::Quote(text) => (SegmentKind::Quote, text.clone()),
            Block::Item {
                ordinal,
                text,
                depth,
            } => (
                SegmentKind::Item,
                match ordinal {
                    // The marker as the document numbers it, and the depth as a
                    // margin — so a list that was nested still reads as nested
                    // in a pane that draws lines rather than lists.
                    Some(n) => format!("{}{n}. {text}", "\u{2003}".repeat(*depth as usize)),
                    None => format!("{}{text}", "\u{2003}".repeat(*depth as usize)),
                },
            ),
            // Tab between cells, which is what a column boundary is in every
            // plain rendering of a table and is one character a reader can see
            // columns in.
            Block::Row { cells, .. } => (SegmentKind::Row, cells.join("\t")),
            Block::Note { kind, marker, text } => {
                if !kind.is_the_text() {
                    continue;
                }
                (SegmentKind::Note, format!("{marker}. {text}"))
            }
            Block::Heading { .. } => continue,
        };
        if text.trim().is_empty() {
            continue;
        }
        let nth = lines.last_mut().map_or(1, |n| {
            *n += 1;
            *n
        });
        let mut path: Vec<String> = open.iter().map(|(_, label)| label.clone()).collect();
        path.push(nth.to_string());
        out.push(RawSegment { path, kind, text });
    }
    out
}

/// A label no sibling section is already using.
///
/// `_` and not `-`, for the reason [`crate::work::section_label_of`] gives: a
/// hyphen in an address level is how `girsa-ref` writes a span. A silent
/// collision would make one of two chapters unreachable.
fn unique(label: &str, taken: &[&str]) -> String {
    let label = if label.is_empty() { "_" } else { label };
    if !taken.contains(&label) {
        return label.to_string();
    }
    for n in 2..=u32::MAX {
        let candidate = format!("{label}_{n}");
        if !taken.contains(&candidate.as_str()) {
            return candidate;
        }
    }
    label.to_string()
}

/// The one line the shelf shows about how a document was read.
fn said(blocks: usize, notes: usize, rows: usize) -> String {
    let mut out = format!("your own writing, {blocks} blocks");
    if notes > 0 {
        out.push_str(&format!(", {notes} notes"));
    }
    if rows > 0 {
        out.push_str(&format!(", {rows} table rows"));
    }
    out
}

// ---------------------------------------------------------------------------
// Plain text
// ---------------------------------------------------------------------------

/// Split where the author left a blank line.
///
/// If there are none — one long file of single lines, which is how a good deal
/// of Hebrew plain text arrives — every line is its own paragraph instead. A
/// sefer whose every word is in segment 1 is not addressable, and addressable
/// is the whole point.
#[must_use]
pub fn paragraphs(text: &str) -> Vec<RawSegment> {
    let by_blank: Vec<&str> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let pieces: Vec<String> = if by_blank.len() > 1 {
        by_blank
            .iter()
            .map(|p| p.split('\n').map(str::trim).collect::<Vec<_>>().join(" "))
            .collect()
    } else {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(ToString::to_string)
            .collect()
    };

    pieces
        .into_iter()
        .enumerate()
        .map(|(i, text)| RawSegment {
            path: vec![(i + 1).to_string()],
            kind: SegmentKind::Text,
            text,
        })
        .collect()
}

/// Read bytes as text, and say how.
///
/// UTF-8 first, with or without a BOM, then UTF-16 if it says so, then
/// **windows-1255** — which is what a Hebrew `.txt` off a Windows machine is,
/// and which as UTF-8 is not a string at all. Which one was used goes into the
/// work's `edition`, because a reader looking at a mangled word deserves to be
/// able to see what it was read as.
#[must_use]
pub fn decode(bytes: &[u8]) -> (String, &'static str) {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        if let Ok(text) = std::str::from_utf8(rest) {
            return (text.to_string(), "utf-8");
        }
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return (text.to_string(), "utf-8");
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let wide: Vec<u16> = rest
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        return (String::from_utf16_lossy(&wide), "utf-16");
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let wide: Vec<u16> = rest
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        return (String::from_utf16_lossy(&wide), "utf-16");
    }
    (from_1255(bytes), "windows-1255")
}

/// Windows-1255, the Hebrew code page.
///
/// Written out rather than taken from a crate: it is one table, and the letters
/// and the nikud are the whole of what matters here. A byte the code page does
/// not define becomes `U+FFFD` — **visible**, because a byte silently turned
/// into a plausible letter is a corrupted sefer that nobody will ever catch.
fn from_1255(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| match byte {
            0x00..=0x7F => char::from(*byte),
            // The nikud block, U+05B0 sheva through U+05C3 sof pasuq.
            0xC0..=0xC9 => point(0x05B0 + u32::from(byte - 0xC0)),
            0xCB..=0xD3 => point(0x05BB + u32::from(byte - 0xCB)),
            // The ligatures and the geresh/gershayim, U+05F0..U+05F4.
            0xD4..=0xD8 => point(0x05F0 + u32::from(byte - 0xD4)),
            // א through ת.
            0xE0..=0xFA => point(0x05D0 + u32::from(byte - 0xE0)),
            0xA0 => ' ',
            0xA4 => '₪',
            0xAA => '×',
            0xBA => '÷',
            0x96 => '–',
            0x97 => '—',
            0x91 | 0x92 => '\'',
            0x93 | 0x94 => '"',
            0x85 => '…',
            _ => char::REPLACEMENT_CHARACTER,
        })
        .collect()
}

fn point(code: u32) -> char {
    char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
}

// ---------------------------------------------------------------------------
// Word
// ---------------------------------------------------------------------------

/// The paragraphs of a .docx, with the headings Word declared.
fn from_docx(file: &Path) -> Result<Vec<RawSegment>, MineError> {
    let handle = std::fs::File::open(file).map_err(io(file))?;
    let mut zip = zip::ZipArchive::new(handle)
        .map_err(|e| MineError::NotADocx(file.display().to_string(), e.to_string()))?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .map_err(|e| MineError::NotADocx(file.display().to_string(), e.to_string()))?
        .read_to_string(&mut xml)
        .map_err(io(file))?;
    Ok(docx_paragraphs(&xml))
}

/// `word/document.xml`, as segments.
///
/// Scanned rather than parsed with an XML library, because the whole of what is
/// wanted is: where does a `w:p` start, what `w:pStyle` did it declare, and
/// what is in its `w:t` runs.
#[must_use]
pub fn docx_paragraphs(xml: &str) -> Vec<RawSegment> {
    let mut out = Vec::new();
    let mut n = 0usize;
    for block in xml.split("<w:p ").flat_map(|b| b.split("<w:p>")).skip(1) {
        let block = block.split("</w:p>").next().unwrap_or_default();
        let heading = attribute(block, "<w:pStyle", "w:val")
            .is_some_and(|style| style.starts_with("Heading") || style == "Title");

        let mut text = String::new();
        for run in block.split("<w:t").skip(1) {
            let Some((_, rest)) = run.split_once('>') else {
                continue;
            };
            let Some((body, _)) = rest.split_once("</w:t>") else {
                continue;
            };
            text.push_str(&unescape(body));
        }
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        n += 1;
        out.push(RawSegment {
            path: vec![n.to_string()],
            kind: if heading {
                SegmentKind::Heading
            } else {
                SegmentKind::Text
            },
            text: text.to_string(),
        });
    }
    out
}

/// The value of one attribute of the first `tag` in a block.
fn attribute<'a>(block: &'a str, tag: &str, name: &str) -> Option<&'a str> {
    let start = block.find(tag)?;
    let element = &block[start..];
    let element = &element[..element.find('>').unwrap_or(element.len())];
    let at = element.find(&format!("{name}=\""))? + name.len() + 2;
    let rest = &element[at..];
    rest.find('"').map(|end| &rest[..end])
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

// ---------------------------------------------------------------------------
// PDF
// ---------------------------------------------------------------------------

/// How many pages a PDF has.
///
/// Counted through the page tree rather than by looking for `/Type /Page` in
/// the bytes: a PDF written this decade keeps its page tree inside a compressed
/// object stream, where looking finds nothing.
fn pages_in(file: &Path) -> Result<usize, MineError> {
    let document = lopdf::Document::load(file)
        .map_err(|e| MineError::NotAPdf(file.display().to_string(), e.to_string()))?;
    Ok(document.get_pages().len())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_file_with_no_blank_lines_is_still_addressable() {
        let by_line = paragraphs("ראשון\nשני\nשלישי");
        assert_eq!(by_line.len(), 3);
        assert_eq!(by_line[2].path, vec!["3".to_string()]);

        // And where there are blank lines, they are what counts — the lines
        // inside one paragraph are one segment.
        let by_blank = paragraphs("ראשון\nהמשך\n\nשני");
        assert_eq!(by_blank.len(), 2);
        assert_eq!(by_blank[0].text, "ראשון המשך");
    }

    #[test]
    fn windows_1255_letters_and_nikud_come_back_as_themselves() {
        // בְּרֵאשִׁית in windows-1255: letters interleaved with nikud.
        let bytes = [
            0xE1, 0xC7, 0xCC, 0xF8, 0xC5, 0xE0, 0xF9, 0xC4, 0xD1, 0xE9, 0xFA,
        ];
        let (text, encoding) = decode(&bytes);
        assert_eq!(encoding, "windows-1255");
        assert_eq!(girsa_hebrew::normalize(&text), "בראשית");

        // A byte the code page does not define is not quietly turned into a
        // letter.
        let (odd, _) = decode(&[0xE0, 0xCA, 0xE1]);
        assert!(odd.contains(char::REPLACEMENT_CHARACTER), "{odd}");
    }

    #[test]
    fn utf8_wins_and_a_bom_is_not_part_of_the_first_word() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("שלום".as_bytes());
        let (text, encoding) = decode(&bytes);
        assert_eq!(encoding, "utf-8");
        assert_eq!(text, "שלום");
    }

    #[test]
    fn word_says_which_paragraphs_are_headings_and_nothing_else_decides() {
        let xml = r#"<w:body>
            <w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>סימן א</w:t></w:r></w:p>
            <w:p><w:r><w:t>ומה </w:t></w:r><w:r><w:t>שכתב</w:t></w:r></w:p>
            <w:p w:rsidR="00"><w:pPr><w:pStyle w:val="Normal"/></w:pPr><w:r><w:t>ועוד &amp; עוד</w:t></w:r></w:p>
        </w:body>"#;
        let raw = docx_paragraphs(xml);
        assert_eq!(raw.len(), 3);
        assert_eq!(raw[0].kind, SegmentKind::Heading);
        assert_eq!(raw[0].text, "סימן א");
        // Two runs of one paragraph are one segment, not two.
        assert_eq!(raw[1].kind, SegmentKind::Text);
        assert_eq!(raw[1].text, "ומה שכתב");
        // A declared style that is not a heading is not one.
        assert_eq!(raw[2].kind, SegmentKind::Text);
        assert_eq!(raw[2].text, "ועוד & עוד");
    }
}
