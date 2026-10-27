//! Custom minijinja filters available to changelog templates.

use minijinja::{Error, ErrorKind};

/// Returns `text`, a newline, and a line of `fill` repeated once per character.
///
/// Used for reStructuredText-style headings, for example
/// `{{ "Features"|heading("~") }}`.
///
/// # Errors
///
/// Returns an invalid-operation error unless `fill` is exactly one character.
pub fn heading(text: &str, fill: &str) -> Result<String, Error> {
    let mut fill_chars = fill.chars();
    let (Some(fill_char), None) = (fill_chars.next(), fill_chars.next()) else {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("heading needs a single fill character, got '{fill}'"),
        ));
    };

    let width = text.chars().count();
    let mut out = String::with_capacity(text.len() + 1 + width * fill_char.len_utf8());
    out.push_str(text);
    out.push('\n');
    out.extend(std::iter::repeat_n(fill_char, width));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_ascii() {
        assert_eq!(heading("Features", "~").unwrap(), "Features\n~~~~~~~~");
    }

    #[test]
    fn test_heading_counts_characters_not_bytes() {
        // the book emoji is one character but four bytes
        assert_eq!(
            heading("\u{1f4da} Docs", "-").unwrap(),
            "\u{1f4da} Docs\n------"
        );
    }

    #[test]
    fn test_heading_empty_text() {
        assert_eq!(heading("", "=").unwrap(), "\n");
    }

    #[test]
    fn test_heading_rejects_bad_fill() {
        assert!(heading("x", "").is_err());
        assert!(heading("x", "ab").is_err());
    }
}
