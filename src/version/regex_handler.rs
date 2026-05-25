//! Regex-based version handler.
//!
//! Updates version strings in arbitrary file formats by targeting a named
//! capture group `v` inside a user-supplied regex pattern. This lets users
//! anchor replacements to surrounding context so that only the right
//! occurrence is changed, regardless of what other version-like strings the
//! file may contain.
//!
//! # Configuration example
//! ```toml
//! [[version.files]]
//! path = "README.md"
//! kind = "regex"
//! pattern = 'cocoa = "(?P<v>[^"]+)"'
//! occurrences = "first"
//! ```

use regex::Regex;

use crate::{
    config::{Occurrences, OccurrencesNamed},
    version::{
        FileKind, UpdatedFile, VersionError,
        handlers::{Handler, PendingUpdate, read_text},
    },
};

/// Handler that replaces text matched by a named capture group `v`.
///
/// The pattern must contain exactly one named group called `v`. Every match
/// (or a configured subset) replaces the content of that group with the new
/// version string, leaving the surrounding context unchanged.
#[derive(Debug)]
pub struct RegexHandler {
    /// The compiled pattern. Must contain a `(?P<v>...)` group.
    pub pattern: String,
    /// How many matches to replace.
    pub occurrences: Occurrences,
}

impl RegexHandler {
    /// Create a new handler, compiling the pattern immediately.
    ///
    /// Returns `VersionError::PatternInvalid` when the regex fails to compile.
    pub fn new(path: &str, pattern: &str, occurrences: Occurrences) -> Result<Self, VersionError> {
        // validate the regex compiles (we compile it again in prepare, but
        // fail-fast here so config errors surface before any files are read)
        Regex::new(pattern).map_err(|e| VersionError::PatternInvalid {
            pattern: pattern.to_owned(),
            path: path.to_owned(),
            source: e,
        })?;

        Ok(Self {
            pattern: pattern.to_owned(),
            occurrences,
        })
    }
}

impl Handler for RegexHandler {
    fn prepare(
        &self,
        path: &str,
        _old_version: &str,
        new_version: &str,
    ) -> Result<Option<PendingUpdate>, VersionError> {
        let re = Regex::new(&self.pattern).map_err(|e| VersionError::PatternInvalid {
            pattern: self.pattern.clone(),
            path: path.to_owned(),
            source: e,
        })?;

        // ensure the pattern has a capture group named `v`
        if re.capture_names().all(|n| n != Some("v")) {
            return Err(VersionError::PatternMissingGroup {
                pattern: self.pattern.clone(),
                path: path.to_owned(),
            });
        }

        let original_text = read_text(path)?;
        let original_bytes = original_text.as_bytes().to_vec();

        // count total matches so we can check for zero
        let total_matches = re.captures_iter(&original_text).count();
        if total_matches == 0 {
            return Err(VersionError::PatternNoMatch {
                pattern: self.pattern.clone(),
                path: path.to_owned(),
            });
        }

        let limit = match &self.occurrences {
            Occurrences::Named(OccurrencesNamed::All) => usize::MAX,
            Occurrences::Named(OccurrencesNamed::First) | Occurrences::Count(1) => 1,
            Occurrences::Count(n) => *n,
        };

        let (updated_text, replacements) =
            replace_in_group(&re, &original_text, new_version, limit);

        Ok(Some(PendingUpdate {
            path: path.to_owned(),
            original: original_bytes,
            updated: updated_text.into_bytes(),
            updated_file: UpdatedFile {
                path: path.to_owned(),
                kind: FileKind::Regex,
                replacements,
            },
        }))
    }
}

/// Replace up to `limit` occurrences of the `v` capture group with `new_value`.
///
/// Returns the modified string and the number of replacements made.
fn replace_in_group(re: &Regex, text: &str, new_value: &str, limit: usize) -> (String, usize) {
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;
    let mut count = 0;

    for caps in re.captures_iter(text) {
        if count >= limit {
            break;
        }

        let full_match = caps.get(0).unwrap();
        let v_match = caps.name("v").unwrap();

        // append everything from last_end to the start of the `v` group
        result.push_str(&text[last_end..v_match.start()]);
        // substitute the new version
        result.push_str(new_value);
        // advance past the full match so surrounding context is preserved
        last_end = v_match.end();
        count += 1;

        // if this is the last replacement we'll make, we need to append up to
        // end of full match then break; the remainder is handled after the loop
        let _ = full_match; // full match used implicitly via last_end tracking
    }

    // append everything after the last replacement
    result.push_str(&text[last_end..]);
    (result, count)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn write_tmp(content: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, content).unwrap();
        (dir, path.to_string_lossy().into_owned())
    }

    fn all() -> Occurrences {
        Occurrences::Named(OccurrencesNamed::All)
    }

    fn first() -> Occurrences {
        Occurrences::Named(OccurrencesNamed::First)
    }

    #[test]
    fn test_regex_replaces_named_group() {
        let (_dir, path) = write_tmp(r#"version = "1.0.0""#);
        let h = RegexHandler::new(&path, r#"version = "(?P<v>[^"]+)""#, all()).unwrap();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(
            String::from_utf8(update.updated).unwrap(),
            r#"version = "2.0.0""#
        );
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_regex_replaces_all_occurrences() {
        let (_dir, path) = write_tmp("pkg@1.0.0 and pkg@1.0.0");
        let h = RegexHandler::new(&path, r"pkg@(?P<v>[0-9.]+)", all()).unwrap();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(
            String::from_utf8(update.updated).unwrap(),
            "pkg@2.0.0 and pkg@2.0.0"
        );
        assert_eq!(update.updated_file.replacements, 2);
    }

    #[test]
    fn test_regex_replaces_first_only() {
        let (_dir, path) = write_tmp("pkg@1.0.0 and pkg@1.0.0");
        let h = RegexHandler::new(&path, r"pkg@(?P<v>[0-9.]+)", first()).unwrap();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(
            String::from_utf8(update.updated).unwrap(),
            "pkg@2.0.0 and pkg@1.0.0"
        );
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_regex_replaces_n_occurrences() {
        let (_dir, path) = write_tmp("v1.0.0 v1.0.0 v1.0.0");
        let h = RegexHandler::new(&path, r"v(?P<v>[0-9.]+)", Occurrences::Count(2)).unwrap();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(
            String::from_utf8(update.updated).unwrap(),
            "v2.0.0 v2.0.0 v1.0.0"
        );
        assert_eq!(update.updated_file.replacements, 2);
    }

    #[test]
    fn test_regex_preserves_surrounding_context() {
        let (_dir, path) = write_tmp(r#"<Badge text="v1.0.0" color="blue" />"#);
        let h = RegexHandler::new(&path, r#"text="v(?P<v>[^"]+)""#, first()).unwrap();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(
            String::from_utf8(update.updated).unwrap(),
            r#"<Badge text="v2.0.0" color="blue" />"#
        );
    }

    #[test]
    fn test_regex_no_match_returns_error() {
        let (_dir, path) = write_tmp("nothing here");
        let h = RegexHandler::new(&path, r"version-(?P<v>[^-]+)-", all()).unwrap();
        let err = h.prepare(&path, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::PatternNoMatch { .. }));
    }

    #[test]
    fn test_regex_missing_v_group_returns_error() {
        let (_dir, path) = write_tmp("version = 1.0.0");
        let h = RegexHandler::new(&path, r"version = ([0-9.]+)", all()).unwrap();
        let err = h.prepare(&path, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::PatternMissingGroup { .. }));
    }

    #[test]
    fn test_regex_invalid_pattern_returns_error() {
        let (_dir, path) = write_tmp("x");
        let err = RegexHandler::new(&path, r"(?P<v>", all()).unwrap_err();
        assert!(matches!(err, VersionError::PatternInvalid { .. }));
    }

    #[test]
    fn test_regex_missing_file_returns_error() {
        let h = RegexHandler::new("/nonexistent/path.txt", r"v(?P<v>[0-9.]+)", all()).unwrap();
        let err = h
            .prepare("/nonexistent/path.txt", "1.0.0", "2.0.0")
            .unwrap_err();
        assert!(matches!(err, VersionError::File { .. }));
    }

    #[test]
    fn test_regex_kind_is_regex() {
        let (_dir, path) = write_tmp(r#"v = "1.0.0""#);
        let h = RegexHandler::new(&path, r#"v = "(?P<v>[^"]+)""#, all()).unwrap();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(update.updated_file.kind, FileKind::Regex);
    }
}
