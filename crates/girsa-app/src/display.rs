//! Showing text, as against indexing it.
//!
//! These are not the same job and the difference has bitten this project once
//! already. spec.md §9.1 strips `U+0591–U+05C7` for the **index**, and the
//! README records why four of those code points become a space there: maqaf
//! separates words, so deleting it glues `אֶת־הַשָּׁמַיִם` into one token and the
//! second pasuk of the Torah stops being findable by either word in it.
//!
//! On the page none of that applies. A maqaf is printed, and turning it into a
//! space would rewrite the sefer in front of the reader. So the nikud toggle
//! takes off **marks only** — the nikud and the te'amim — and leaves every
//! letter and every mark of punctuation exactly where the corpus has it.

/// The same text with its nikud and te'amim taken off, for a reader who has
/// them switched off.
///
/// Idempotent, and safe on text that never had any: most of the corpus is bare
/// already and Berakhot is fully menukad (spec.md §2.1), so both arrive at the
/// same window.
#[must_use]
pub fn without_marks(text: &str) -> String {
    text.chars()
        .filter(|c| !girsa_hebrew::is_mark(*c))
        .collect()
}

/// How a run of words is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Plain,
    /// The dibur hamatchil — the words being commented on, which Sefaria marks
    /// `<b>`, `<strong>` or `<big>`. Losing it turns Rashi into one grey block
    /// and you can no longer see where each comment starts.
    Opening,
    /// `<i>`, `<em>`, `<small>` — an aside, a source, a footnote.
    Quiet,
    /// A line break inside a segment. Carries no text.
    Break,
}

/// A stretch of text and how it is set.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Run {
    pub text: String,
    pub style: Style,
}

/// Split a segment into runs, reading the markup the corpus carries.
///
/// # Why this is not a matter of taste
///
/// Sefaria's text is not plain text. It has inline HTML in it — 43,890 `</i>`
/// and 747 `<b>` in Berakhot alone — and a reader shown the raw string sees
/// `<big><strong>מאימתי</strong></big>` sitting in the middle of the first line
/// of Shas. Stripping the tags instead loses the dibur hamatchil, which is how
/// you find where one Rashi ends and the next begins.
///
/// # Runs, not HTML
///
/// The answer is **not** to put the corpus's markup into the page. The text is
/// local and the shell's CSP would stop a script, but building a document out of
/// a string that came from a file is the habit that costs you later, and there
/// is no need for it here: the six things this markup says are all sayable as a
/// list of runs, which the window turns into elements it made itself.
///
/// Tags outside the list are dropped and **their text is kept** — a `<span>`
/// around a masoretic פ is markup around a letter, and the letter is part of the
/// pasuk.
#[must_use]
pub fn runs(text: &str) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    let mut style = Style::Plain;
    let mut depth = 0usize;
    let mut buffer = String::new();
    let mut rest = text;

    let flush = |buffer: &mut String, style: Style, out: &mut Vec<Run>| {
        if buffer.is_empty() {
            return;
        }
        match out.last_mut() {
            Some(last) if last.style == style => last.text.push_str(buffer),
            _ => out.push(Run {
                text: std::mem::take(buffer),
                style,
            }),
        }
        buffer.clear();
    };

    while let Some(at) = rest.find('<') {
        buffer.push_str(&entities(&rest[..at]));
        let Some(end) = rest[at..].find('>') else {
            // An unclosed `<` is a `<`, not the start of markup.
            buffer.push_str(&entities(&rest[at..]));
            rest = "";
            break;
        };
        let tag = &rest[at + 1..at + end];
        rest = &rest[at + end + 1..];

        let closing = tag.starts_with('/');
        let name = tag
            .trim_start_matches('/')
            .split([' ', '\t', '\n'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        if name == "br" {
            flush(&mut buffer, style, &mut out);
            out.push(Run {
                text: String::new(),
                style: Style::Break,
            });
            continue;
        }
        let Some(marks) = style_of(&name) else {
            // Not one of ours. The tag goes, the words stay.
            continue;
        };
        if closing {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                flush(&mut buffer, style, &mut out);
                style = Style::Plain;
            }
        } else {
            if depth == 0 {
                flush(&mut buffer, style, &mut out);
                style = marks;
            }
            depth += 1;
        }
    }
    buffer.push_str(&entities(rest));
    flush(&mut buffer, style, &mut out);
    out
}

fn style_of(name: &str) -> Option<Style> {
    match name {
        "b" | "strong" | "big" => Some(Style::Opening),
        "i" | "em" | "small" | "sup" => Some(Style::Quiet),
        _ => None,
    }
}

/// The two entities the corpus actually uses, and the three every HTML-ish
/// string carries.
fn entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    text.replace("&nbsp;", "\u{00A0}")
        .replace("&thinsp;", "\u{2009}")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The words of a segment with the markup taken out, for anything that wants
/// text rather than a page.
#[must_use]
pub fn plain(text: &str) -> String {
    runs(text).iter().map(|r| r.text.as_str()).collect()
}

/// Whether this text has any nikud at all.
///
/// The toggle is pointless on a sefer that has none, and an app that offers a
/// switch which visibly does nothing teaches its reader that the switches lie.
#[must_use]
pub fn has_marks(text: &str) -> bool {
    text.chars().any(girsa_hebrew::is_mark)
}

/// An era, as a reader says it rather than as Sefaria codes it.
///
/// The catalogue records `AH`, `RI`, `T`, `A`, `GN`, `CO` — 4,812 of the 7,189
/// works carry one — and a shelf row that says `A` beside `ברכות` is telling
/// the reader nothing. A code nobody here has a word for is **shown as it is**,
/// not dropped and not guessed at: an unknown era is a thing to notice.
#[must_use]
pub fn era_said(code: &str) -> &str {
    match code {
        "T" => "תנאים",
        "A" => "אמוראים",
        "GN" => "גאונים",
        "RI" => "ראשונים",
        "AH" => "אחרונים",
        "CO" => "מחברי זמננו",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_era_is_said_in_words_and_an_unknown_code_is_shown_rather_than_dropped() {
        assert_eq!(era_said("RI"), "ראשונים");
        assert_eq!(era_said("A"), "אמוראים");
        assert_eq!(era_said("XX"), "XX");
    }

    #[test]
    fn the_toggle_takes_off_nikud_and_leaves_the_letters() {
        assert_eq!(without_marks("וּבַשַּׁבָּת"), "ובשבת");
        assert_eq!(without_marks("בראשית"), "בראשית");
    }

    #[test]
    fn a_maqaf_survives_being_shown_even_though_the_index_replaces_it() {
        // The index turns it into a space so that both words are findable; the
        // page prints it, because it is what is printed. Two rules, on purpose.
        assert_eq!(without_marks("אֶת־הַשָּׁמַיִם"), "את־השמים");
        assert!(without_marks("אֶת־הַשָּׁמַיִם").contains('\u{05BE}'));
    }

    #[test]
    fn stripping_twice_is_stripping_once() {
        let once = without_marks("בְּרֵאשִׁית");
        assert_eq!(without_marks(&once), once);
    }

    fn shown(text: &str) -> Vec<(&'static str, String)> {
        runs(text)
            .into_iter()
            .map(|r| {
                (
                    match r.style {
                        Style::Plain => "plain",
                        Style::Opening => "opening",
                        Style::Quiet => "quiet",
                        Style::Break => "break",
                    },
                    r.text,
                )
            })
            .collect()
    }

    #[test]
    fn the_first_line_of_shas_is_not_shown_with_its_tags_in_it() {
        // Verbatim off the shelf. Shown raw, a reader sees
        // `<big><strong>מאימתי</strong></big>` in the middle of the Gemara.
        let line = "<big><strong>מאימתי</strong></big> קורין את שמע";
        assert_eq!(
            shown(line),
            [
                ("opening", "מאימתי".to_string()),
                ("plain", " קורין את שמע".to_string()),
            ]
        );
    }

    #[test]
    fn the_dibur_hamatchil_is_kept_rather_than_stripped() {
        // Stripping the tags is the other easy answer and it costs the thing
        // that makes a page of Rashi readable — where each comment starts.
        let rashi = "<b>משעה שהכהנים נכנסים</b> – כהנים שנטמאו";
        assert_eq!(
            shown(rashi),
            [
                ("opening", "משעה שהכהנים נכנסים".to_string()),
                ("plain", " – כהנים שנטמאו".to_string()),
            ]
        );
    }

    #[test]
    fn markup_we_do_not_know_loses_its_tags_and_keeps_its_letters() {
        // `<span class="mam-spi-pe">פ</span>` is markup around a letter of the
        // pasuk. Dropping the letter with the tag would take a masoretic mark
        // out of the Torah.
        assert_eq!(
            shown("בראשית <span class=\"mam-spi-pe\">פ</span> ברא"),
            [("plain", "בראשית פ ברא".to_string())]
        );
    }

    #[test]
    fn nested_markup_does_not_end_early() {
        // `<b>א <i>ב</i> ג</b>` — the inner close must not end the bold.
        assert_eq!(
            shown("<b>א <i>ב</i> ג</b>ד"),
            [("opening", "א ב ג".to_string()), ("plain", "ד".to_string())]
        );
    }

    #[test]
    fn a_break_inside_a_segment_is_a_break() {
        assert_eq!(
            shown("ראשון<br>שני"),
            [
                ("plain", "ראשון".to_string()),
                ("break", String::new()),
                ("plain", "שני".to_string()),
            ]
        );
    }

    #[test]
    fn the_two_entities_the_corpus_uses_come_out_as_spaces() {
        assert_eq!(plain("א&nbsp;ב&thinsp;ג"), "א\u{00A0}ב\u{2009}ג");
    }

    #[test]
    fn a_stray_angle_bracket_is_a_character_and_not_a_tag() {
        assert_eq!(plain("שווה < משהו"), "שווה < משהו");
    }

    #[test]
    fn plain_text_comes_through_as_one_run() {
        assert_eq!(shown("בראשית ברא"), [("plain", "בראשית ברא".to_string())]);
        assert!(runs("").is_empty());
    }

    #[test]
    fn a_sefer_with_no_nikud_says_so() {
        assert!(has_marks("בְּרֵאשִׁית"));
        assert!(!has_marks("משנה ברורה"));
    }
}
