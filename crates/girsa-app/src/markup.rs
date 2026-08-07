//! Putting Hebrew into markup without it stopping being Hebrew.
//!
//! # Why there are two of these and not one
//!
//! `"` and `'` are how Hebrew writes gershayim. `שו"ע או"ח סימן א'` is a
//! citation, not a quotation, and escaping those marks as `&quot;` and `&#39;`
//! turns a mekor into noise in anything that shows markup rather than rendering
//! it — a plain-text paste, a diff, a `.docx` opened in a text editor.
//!
//! So [`text`] escapes the three characters that would otherwise **be** markup,
//! and [`attr`] escapes the quote as well, because inside `"…"` a quote ends the
//! value and there is nothing to argue about.
//!
//! # Why there are two of these and not six
//!
//! There were six. `sending.rs` and `scanning.rs` each carried an `escape_text`
//! and an `escape_attr`, **byte-identical, including a five-line comment about
//! gershayim** — so the paragraph explaining the rule existed twice and could
//! rot in one. `export.rs` carried a third, spelled as a chain of `replace`
//! calls, escaping the quote in element content where nothing requires it.
//!
//! Three copies of a rule about Hebrew punctuation, in one crate. The comment
//! was right and there were two of it.
//!
//! # Why the corpus needs this at all
//!
//! A quote from a sefer is arbitrary text, and Sefaria's own files carry `<`,
//! `>` and `&` inside segments — **43,890 `</i>` in Berakhot alone**. While
//! [`crate::display::plain`] takes the tags off, a stray `<` in the corpus is a
//! character a reader typed and has to arrive as one.

/// The three characters that would otherwise be read as markup.
///
/// **Not the quote marks** — see the module note.
#[must_use]
pub fn text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// The same, inside a `"…"` attribute, where a quote mark would end the value.
#[must_use]
pub fn attr(raw: &str) -> String {
    text(raw).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_gershayim_survives_text_and_is_escaped_in_an_attribute() {
        // The whole reason there are two. A mekor pasted as plain text has to
        // still read as a mekor.
        let mekor = "שו\"ע או\"ח סימן א'";
        assert_eq!(text(mekor), mekor, "the gershayim was escaped in text");
        assert_eq!(
            attr(mekor),
            "שו&quot;ע או&quot;ח סימן א'",
            "a quote inside an attribute ends the value"
        );
    }

    #[test]
    fn the_three_that_would_be_markup_are_escaped_in_both() {
        // 43,890 `</i>` in Berakhot alone, and `display::plain` takes the tags
        // off — but a `<` that is a character in the sefer stays a character.
        for escaped in [text("a < b & c > d"), attr("a < b & c > d")] {
            assert_eq!(escaped, "a &lt; b &amp; c &gt; d");
        }
    }

    #[test]
    fn the_ampersand_goes_first() {
        // `<` → `&lt;` and then `&` → `&amp;` gives `&amp;lt;`. One pass over
        // the characters cannot make that mistake, which is why this is a loop
        // and not a chain of `replace` calls — `export.rs`'s copy was the chain,
        // in the right order, by luck rather than by construction.
        assert_eq!(text("&lt;"), "&amp;lt;");
        assert_eq!(text("<&>"), "&lt;&amp;&gt;");
    }
}
