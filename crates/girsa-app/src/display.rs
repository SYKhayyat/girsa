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
    /// These are the words that answered a search (W39).
    ///
    /// > *"the search result is not clear (the actual hit)."*
    ///
    /// A field beside the style rather than another value of it, because a match
    /// inside a dibur hamatchil is **both** — and a `Style::Hit` would have to
    /// choose, which is how the bold goes missing from exactly the rows a reader
    /// is looking hardest at.
    ///
    /// Which words those are is the search's own answer (`bar::Marker`), not a
    /// re-search of the drawn text: on a menukad page, looking for what the
    /// reader typed finds nothing.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hit: bool,
}

/// The text as the pane drew it, and where every character of it came from.
///
/// # Why this exists
///
/// A reader highlights four letters and asks for them to be corrected. The
/// window counts that highlight in **characters of the text it drew** — markup
/// already off, nikud already applied — and a patch has to name characters of
/// the segment **as it stands on disk**, because that is the only text that is
/// still there tomorrow (W20). The two disagree by however much markup and
/// however many nikud points are in the line, which in Berakhot is most of it.
///
/// So the scan that takes the markup off records what it took, and this is that
/// record. Nothing else in the project may work the offset out by arithmetic.
#[derive(Debug, Clone)]
pub struct Shown {
    text: String,
    /// Per character of `text`, the half-open span of base characters it came
    /// from. An entity is several base characters and one shown one.
    from: Vec<(usize, usize)>,
}

impl Shown {
    /// Draw a segment the way the pane draws it.
    #[must_use]
    pub fn of(text: &str, nikud: bool) -> Self {
        let mut shown = String::new();
        let mut from = Vec::new();
        for bit in bits(text) {
            let Bit::Letter { ch, at, len, .. } = bit else {
                continue;
            };
            if !nikud && girsa_hebrew::is_mark(ch) {
                continue;
            }
            shown.push(ch);
            from.push((at, at + len));
        }
        Self { text: shown, from }
    }

    /// The words, as drawn.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// How many characters the reader is looking at.
    #[must_use]
    pub fn len(&self) -> usize {
        self.from.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.from.is_empty()
    }

    /// The span of the segment on disk that a highlight names.
    ///
    /// `None` when the highlight is empty or off the end — never a nearby span:
    /// a correction that lands on letters the reader did not select is the
    /// failure this whole module exists to prevent.
    ///
    /// A highlight that runs across markup gives a span **containing** that
    /// markup, which is the honest answer: the reader is asking for those words
    /// to read differently, and the tags are between them.
    #[must_use]
    pub fn base_span(&self, from_char: usize, to_char: usize) -> Option<std::ops::Range<usize>> {
        let to = to_char.min(self.from.len());
        if from_char >= to {
            return None;
        }
        let start = self.from.get(from_char)?.0;
        let end = self.from.get(to - 1)?.1;
        (start < end).then_some(start..end)
    }
}

/// One character of the drawn text, or a line break, and where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bit {
    Letter {
        ch: char,
        style: Style,
        /// Where in the base text, counted in characters.
        at: usize,
        /// How many base characters it took — one, or the length of an entity.
        len: usize,
    },
    Break,
}

/// The one scan of a segment's markup.
///
/// [`runs`] groups these by style to draw a line; [`Shown`] keeps their
/// positions so a highlight can be turned back into a span of the file. Two
/// implementations of *what this markup says* is how a correction ends up four
/// letters to the left of the typo it was made on.
fn bits(text: &str) -> Vec<Bit> {
    let letters: Vec<char> = text.chars().collect();
    let mut out = Vec::with_capacity(letters.len());
    let mut style = Style::Plain;
    let mut depth = 0usize;
    let mut i = 0usize;

    let plain = |out: &mut Vec<Bit>, letters: &[char], from: usize, style| {
        for (n, ch) in letters.iter().skip(from).enumerate() {
            out.push(Bit::Letter {
                ch: *ch,
                style,
                at: from + n,
                len: 1,
            });
        }
    };

    while i < letters.len() {
        let ch = letters[i];
        if ch == '<' {
            let Some(end) = (i..letters.len()).find(|j| letters[*j] == '>') else {
                // An unclosed `<` is a `<`, not the start of markup.
                plain(&mut out, &letters, i, style);
                break;
            };
            let tag: String = letters[i + 1..end].iter().collect();
            i = end + 1;

            let closing = tag.starts_with('/');
            let name = tag
                .trim_start_matches('/')
                .split([' ', '\t', '\n'])
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if name == "br" {
                out.push(Bit::Break);
                continue;
            }
            let Some(marks) = style_of(&name) else {
                // Not one of ours. The tag goes, the words stay.
                continue;
            };
            if closing {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    style = Style::Plain;
                }
            } else {
                if depth == 0 {
                    style = marks;
                }
                depth += 1;
            }
            continue;
        }
        if ch == '&' {
            if let Some((decoded, len)) = entity_at(&letters, i) {
                out.push(Bit::Letter {
                    ch: decoded,
                    style,
                    at: i,
                    len,
                });
                i += len;
                continue;
            }
        }
        out.push(Bit::Letter {
            ch,
            style,
            at: i,
            len: 1,
        });
        i += 1;
    }
    out
}

/// The two entities the corpus actually uses, and the three every HTML-ish
/// string carries.
fn entity_at(letters: &[char], at: usize) -> Option<(char, usize)> {
    const ENTITIES: [(&str, char); 6] = [
        ("&nbsp;", '\u{00A0}'),
        ("&thinsp;", '\u{2009}'),
        ("&quot;", '"'),
        ("&lt;", '<'),
        ("&gt;", '>'),
        ("&amp;", '&'),
    ];
    for (spelling, decoded) in ENTITIES {
        let len = spelling.chars().count();
        if letters
            .get(at..at + len)
            .is_some_and(|found| found.iter().copied().eq(spelling.chars()))
        {
            return Some((decoded, len));
        }
    }
    None
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
    runs_marking(text, &[])
}

/// The same, with the words that answered a search in runs of their own (W39).
///
/// `marks` are **byte** ranges into `text` — what `girsa_search::bar::Marker`
/// hands back. They are converted to characters here because that is what a
/// [`Bit`] is counted in, and getting that wrong would put the mark a nikud point
/// or two to the left of the word, which is worse than no mark.
///
/// A range that does not line up with the text is ignored rather than clamped: a
/// mark in the wrong place is a lie about which word matched, and no mark is only
/// a missing hint.
#[must_use]
pub fn runs_marking(text: &str, marks: &[(usize, usize)]) -> Vec<Run> {
    let hit = hit_chars(text, marks);
    let mut out: Vec<Run> = Vec::new();
    for bit in bits(text) {
        match bit {
            Bit::Letter { ch, style, at, len } => {
                // A whole entity counts as marked if any of it is.
                let marked = (at..at + len.max(1)).any(|n| hit.contains(&n));
                match out.last_mut() {
                    Some(last) if last.style == style && last.hit == marked => last.text.push(ch),
                    _ => out.push(Run {
                        text: ch.to_string(),
                        style,
                        hit: marked,
                    }),
                }
            }
            Bit::Break => out.push(Run {
                text: String::new(),
                style: Style::Break,
                hit: false,
            }),
        }
    }
    out
}

/// Which character positions of `text` the byte ranges cover.
fn hit_chars(text: &str, marks: &[(usize, usize)]) -> std::collections::BTreeSet<usize> {
    let mut out = std::collections::BTreeSet::new();
    if marks.is_empty() {
        return out;
    }
    // One walk of the text, so a segment with forty marks costs one pass.
    let mut byte_to_char = std::collections::BTreeMap::new();
    for (nth, (byte, _)) in text.char_indices().enumerate() {
        byte_to_char.insert(byte, nth);
    }
    for (from, to) in marks {
        // `>= to` and not `> to`: a range whose end is the end of the text has no
        // character starting at it, and dropping those would lose every match on
        // the last word of a line.
        let (Some(start), Some(end)) = (
            byte_to_char.get(from).copied(),
            byte_to_char
                .range(to..)
                .next()
                .map(|(_, nth)| *nth)
                .or(Some(text.chars().count())),
        ) else {
            continue;
        };
        for n in start..end {
            out.insert(n);
        }
    }
    out
}

/// The same runs with the nikud taken off, keeping what each run is.
///
/// Stripped **after** the marks are placed, not before: `without_marks` shortens
/// the text, so a byte range worked out against the pointed text would land two
/// or three letters left of the word it meant.
#[must_use]
pub fn unpointed(runs: Vec<Run>) -> Vec<Run> {
    runs.into_iter()
        .map(|run| Run {
            text: without_marks(&run.text),
            ..run
        })
        .filter(|run| run.style == Style::Break || !run.text.is_empty())
        .collect()
}

fn style_of(name: &str) -> Option<Style> {
    match name {
        "b" | "strong" | "big" => Some(Style::Opening),
        "i" | "em" | "small" | "sup" => Some(Style::Quiet),
        _ => None,
    }
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
///
/// The table is [`girsa_corpus::era`]'s, because W28 made the same six codes
/// load-bearing: they are the axis a transmission chain runs along, and a
/// second copy of them here could disagree with the one doing the ordering.
#[must_use]
pub fn era_said(code: &str) -> &str {
    match girsa_corpus::era::Era::from_code(code) {
        Some(era) => era.he(),
        None => code,
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
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
    fn what_was_drawn_is_what_the_pane_would_have_drawn() {
        // The two consumers of the scan have to agree, or a correction lands
        // beside the word it was made on. They agree by construction — `runs`
        // and `Shown` are the same scan — and this is the assertion that keeps
        // it that way if one of them is ever rewritten.
        for line in [
            "<big><strong>מאימתי</strong></big> קורין את שמע",
            "<b>משעה שהכהנים נכנסים</b> – כהנים שנטמאו",
            "בראשית <span class=\"mam-spi-pe\">פ</span> ברא",
            "ראשון<br>שני",
            "א&nbsp;ב&thinsp;ג &amp; ד",
            "שווה < משהו",
            "בְּרֵאשִׁית בָּרָא",
            "",
        ] {
            for nikud in [true, false] {
                let drawn = plain(line);
                let drawn = if nikud { drawn } else { without_marks(&drawn) };
                assert_eq!(
                    Shown::of(line, nikud).text(),
                    drawn,
                    "{line} · nikud {nikud}"
                );
            }
        }
    }

    #[test]
    fn a_highlight_names_the_letters_it_covers_in_the_file() {
        // The reader has nikud off and is looking at `ובשבת`. The file has
        // `<b>וּבַשַּׁבָּת</b>` — thirteen characters where the reader sees five, plus
        // three of markup in front. Counting is not going to work; the scan
        // has to say.
        let base = "<b>וּבַשַּׁבָּת</b> הזה";
        let shown = Shown::of(base, false);
        assert_eq!(shown.text(), "ובשבת הזה");
        let span = shown.base_span(0, 5).expect("a span");
        let letters: Vec<char> = base.chars().collect();
        let covered: String = letters[span.clone()].iter().collect();
        assert_eq!(
            covered, "וּבַשַּׁבָּת",
            "the pointed word, and not the tags around it"
        );

        // And the word after it, which is where an off-by-the-nikud would show.
        let span = shown.base_span(6, 9).expect("a span");
        assert_eq!(letters[span].iter().collect::<String>(), "הזה");
    }

    #[test]
    fn a_highlight_across_markup_covers_the_markup_between_the_words() {
        // The honest answer: the reader is asking for those words to read
        // differently, and the tags are in between them. A correction made
        // here replaces the lot, which is visible in what it says it will do.
        let base = "א<b>ב</b>ג";
        let shown = Shown::of(base, true);
        assert_eq!(shown.text(), "אבג");
        assert_eq!(shown.base_span(0, 3), Some(0..base.chars().count()));
        assert_eq!(
            shown.base_span(1, 2),
            Some(4..5),
            "just the ב, inside its tag"
        );
    }

    #[test]
    fn a_highlight_of_nothing_is_not_a_span() {
        let shown = Shown::of("אבג", true);
        assert_eq!(shown.base_span(1, 1), None);
        assert_eq!(shown.base_span(3, 9), None);
        assert_eq!(
            shown.base_span(0, 99),
            Some(0..3),
            "past the end is the end"
        );
        assert!(Shown::of("", true).is_empty());
    }

    #[test]
    fn an_entity_is_one_character_to_the_reader_and_six_in_the_file() {
        let shown = Shown::of("א&nbsp;ב", true);
        assert_eq!(shown.len(), 3);
        assert_eq!(shown.base_span(1, 2), Some(1..7));
    }

    #[test]
    fn a_sefer_with_no_nikud_says_so() {
        assert!(has_marks("בְּרֵאשִׁית"));
        assert!(!has_marks("משנה ברורה"));
    }
}

#[cfg(test)]
mod hit_tests {
    // A panic in a test is a failure report.
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    /// The byte range of `word` in `text`, the way `bar::Marker` gives them.
    fn at(text: &str, word: &str) -> (usize, usize) {
        let from = text.find(word).expect("the word is in the text");
        (from, from + word.len())
    }

    // ── W39: which words answered the search ─────────────────────────────────
    //
    // > *"the search result is not clear (the actual hit)."*
    //
    // The engine already knew: `bar::Marker::marks` gives byte ranges, and the
    // window was handed the paragraph without them.

    #[test]
    fn the_words_that_matched_come_back_in_their_own_run() {
        let text = "מאימתי קורין את שמע בערבית";
        let marked = runs_marking(text, &[at(text, "שמע")]);
        let hit: Vec<&str> = marked
            .iter()
            .filter(|r| r.hit)
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(hit, vec!["שמע"], "{marked:?}");
        // And nothing is lost putting it in its own element.
        let whole: String = marked.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(whole, text);
    }

    #[test]
    fn a_match_inside_a_dibur_hamatchil_is_still_bold() {
        // The reason `hit` is a field and not a `Style::Hit`. Sefaria marks the
        // words being commented on with `<b>`, and a search for one of them would
        // otherwise arrive with the bold silently removed.
        let text = "<b>מאימתי קורין</b> מכאן ואילך";
        let marked = runs_marking(text, &[(3, 3 + "מאימתי".len())]);
        let both: Vec<&Run> = marked
            .iter()
            .filter(|r| r.hit && r.style == Style::Opening)
            .collect();
        assert_eq!(both.len(), 1, "{marked:?}");
        assert_eq!(both[0].text, "מאימתי");
    }

    #[test]
    fn a_match_on_the_last_word_of_a_line_is_marked() {
        // The range ends at the end of the string, so there is no character
        // starting at its end. An off-by-one here loses every hit that happens to
        // be the last word, which is a class of result nobody would think to
        // check.
        let text = "קורין את שמע";
        let marked = runs_marking(text, &[at(text, "שמע")]);
        assert!(
            marked.iter().any(|r| r.hit && r.text == "שמע"),
            "{marked:?}"
        );
    }

    #[test]
    fn nothing_marked_draws_what_it_always_drew() {
        let text = "<b>מאימתי</b> קורין";
        assert_eq!(runs_marking(text, &[]), runs(text));
        assert!(!runs(text).iter().any(|r| r.hit));
    }

    #[test]
    fn a_range_that_does_not_fit_the_text_marks_nothing() {
        // Rather than clamping. A mark in the wrong place says *this word
        // matched* about a word that did not.
        let text = "קורין את שמע";
        let marked = runs_marking(text, &[(900, 950)]);
        assert!(!marked.iter().any(|r| r.hit));
        let whole: String = marked.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(whole, text, "and the words are all still there");
    }

    #[test]
    fn taking_the_nikud_off_keeps_the_marks_where_they_were() {
        // The order this has to happen in: mark the pointed text, then strip. A
        // byte range worked out against the pointed text and applied to the
        // stripped one lands letters away from the word it meant.
        let text = "שְׁמַע יִשְׂרָאֵל";
        let marked = unpointed(runs_marking(text, &[at(text, "שְׁמַע")]));
        let hit: Vec<&str> = marked
            .iter()
            .filter(|r| r.hit)
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(hit, vec!["שמע"], "{marked:?}");
    }
}
