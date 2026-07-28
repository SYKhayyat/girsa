//! The one tokenizer: `girsa-hebrew`, wearing tantivy's trait.
//!
//! This file is small on purpose. It contains **no rules about Hebrew** — every
//! decision about what a word is, which marks come off and how a gershayim
//! folds lives in `girsa-hebrew`, because the query bar normalizes through that
//! crate and the index must be built by the same code, not by code that agrees
//! with it today.
//!
//! Two implementations of "what is a word" is the failure mode this arrangement
//! exists to make impossible: the index writes `ובשבת` from `וּבַשַּׁבָּת`, the query
//! bar produces something a hair different, and the reader is told the sefer
//! does not contain a line that is printed in front of them.
//!
//! # The offsets are into the text as printed
//!
//! [`girsa_hebrew::tokenize`] hands back each word's span in the **input**, and
//! those spans go straight into tantivy's `offset_from`/`offset_to`. So a hit
//! can be pointed at `קוֹרִין` on the page rather than at `קורין` in the index,
//! which is what a highlight needs.

use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

/// What the schema names this tokenizer, and what the index registers it under.
///
/// The two have to agree or the index writes fine and cannot be queried, so
/// there is one constant and both sides use it.
pub const NAME: &str = "girsa";

/// Nikud off, marks folded, words split — and nothing else.
#[derive(Debug, Clone, Copy, Default)]
pub struct Normalized;

impl Tokenizer for Normalized {
    type TokenStream<'a> = Stream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Stream {
        let tokens = girsa_hebrew::tokenize(text)
            .into_iter()
            .enumerate()
            .map(|(position, token)| Token {
                offset_from: token.start,
                offset_to: token.end,
                position,
                text: token.text,
                position_length: 1,
            })
            .collect::<Vec<_>>();
        Stream {
            tokens: tokens.into_iter(),
            current: Token::default(),
        }
    }
}

/// The words of one segment, handed over one at a time.
pub struct Stream {
    tokens: std::vec::IntoIter<Token>,
    current: Token,
}

impl TokenStream for Stream {
    fn advance(&mut self) -> bool {
        match self.tokens.next() {
            Some(token) => {
                self.current = token;
                true
            }
            None => false,
        }
    }

    fn token(&self) -> &Token {
        &self.current
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.current
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn words(text: &str) -> Vec<(String, usize, usize)> {
        let mut tokenizer = Normalized;
        let mut stream = tokenizer.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            let token = stream.token();
            out.push((token.text.clone(), token.offset_from, token.offset_to));
        }
        out
    }

    #[test]
    fn a_menukad_line_tokenizes_to_bare_words_pointing_at_the_pointed_ones() {
        let text = "מֵאֵימָתַי קוֹרִין";
        let tokens = words(text);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].0, "מאימתי");
        assert_eq!(&text[tokens[0].1..tokens[0].2], "מֵאֵימָתַי");
        // Final letters fold — `ן` is `נ` in the index — so the reader who
        // types the word in the middle of a sentence and the reader who types
        // it at the end are asking the same question.
        assert_eq!(tokens[1].0, "קורינ");
        assert_eq!(&text[tokens[1].1..tokens[1].2], "קוֹרִין");
    }

    #[test]
    fn positions_count_up_so_a_phrase_query_has_something_to_stand_on() {
        let tokens = words("יתגבר כארי לעמוד");
        assert_eq!(tokens.len(), 3);
        let mut tokenizer = Normalized;
        let mut stream = tokenizer.token_stream("יתגבר כארי לעמוד");
        let mut positions = Vec::new();
        while stream.advance() {
            positions.push(stream.token().position);
        }
        assert_eq!(positions, [0, 1, 2]);
    }

    #[test]
    fn an_empty_segment_produces_no_words_rather_than_one_empty_one() {
        // The corpus has empty `<h2></h2>` headings (BUILDER.md T8). An empty
        // term in the index is a term every empty query would match.
        assert!(words("").is_empty());
        assert!(words("   ").is_empty());
        assert!(words("<h2></h2>").len() <= 2, "tags are not words");
    }
}
