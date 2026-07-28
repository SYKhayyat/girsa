//! Reading Sefaria's link CSVs.
//!
//! Small, and here rather than in a dependency for one reason: the fields
//! contain the commas.
//!
//! ```text
//! "A Dictionary of the Talmud, אֱגוֹד 1",Mishnah Peah 6:6,quotation,…
//! ```
//!
//! A title split on the comma alone tears in half, and every row scores as
//! unresolvable for a reason that has nothing to do with the resolver. That is
//! the whole of the format's difficulty; the files are otherwise plain.

/// Split one line into fields, honouring quotes.
#[must_use]
pub fn fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => fields.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    fields.push(current);
    fields
}

/// The columns of `links*.csv`, by the names in its header row.
///
/// **`Conection Type` is misspelled in the file** (T2), in Sefaria's export and
/// in Otzaria's conversion of it both. Reading it correctly spelled finds
/// nothing and silently types every link in the corpus as the catch-all.
pub mod link_columns {
    pub const CITATION_1: usize = 0;
    pub const CITATION_2: usize = 1;
    pub const CONECTION_TYPE: usize = 2;
    pub const TEXT_1: usize = 3;
    pub const TEXT_2: usize = 4;
    /// The header exactly as it appears, so a changed format is noticed rather
    /// than read into the wrong columns.
    pub const HEADER: &str =
        "Citation 1,Citation 2,Conection Type,Text 1,Text 2,Category 1,Category 2";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_citation_containing_a_comma_survives() {
        let line = "\"A Dictionary of the Talmud, אֱגוֹד 1\",Mishnah Peah 6:6,quotation";
        let got = fields(line);
        assert_eq!(got[0], "A Dictionary of the Talmud, אֱגוֹד 1");
        assert_eq!(got[1], "Mishnah Peah 6:6");
        assert_eq!(got[2], "quotation");
    }

    #[test]
    fn a_blank_type_is_an_empty_field_and_not_a_missing_one() {
        // T5: 74% of rows look like this, and a reader that treated a blank as
        // a short row would drop three quarters of the graph.
        let got = fields("\"A Dictionary of the Talmud, Abbreviations 140\",Exodus 1:1-6:1,,x,y");
        assert_eq!(got.len(), 5);
        assert_eq!(got[link_columns::CONECTION_TYPE], "");
    }

    #[test]
    fn the_header_is_the_misspelling_it_actually_is() {
        assert!(link_columns::HEADER.contains("Conection Type"));
        assert!(!link_columns::HEADER.contains("Connection Type"));
    }
}
