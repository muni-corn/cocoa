//! Calendar versioning engine.

use std::fmt;

use chrono::{Datelike, NaiveDate, Utc};
use regex::Regex;
use thiserror::Error;

/// Error from a calendar version operation.
#[derive(Debug, Error, PartialEq)]
pub enum CalVerError {
    /// The format string contains an unsupported pattern.
    #[error("unsupported calver format token in '{0}'")]
    UnsupportedFormat(String),

    /// The input string does not match the expected format.
    #[error("failed to parse '{s}' with calver format '{fmt}': {reason}")]
    Parse {
        s: String,
        fmt: String,
        reason: String,
    },
}

/// A calendar version with a configurable format string.
///
/// Supported format tokens (longest-match first in rendering):
///
/// | Token  | Description                        | Example |
/// |--------|------------------------------------|---------|
/// | `YYYY` | 4-digit year                       | `2024`  |
/// | `0Y`   | 2-digit year, zero-padded          | `24`    |
/// | `YY`   | 2-digit year, no leading zero      | `24`    |
/// | `0M`   | Month, zero-padded                 | `03`    |
/// | `MM`   | Month, no leading zero             | `3`     |
/// | `0D`   | Day, zero-padded                   | `07`    |
/// | `DD`   | Day, no leading zero               | `7`     |
/// | `MICRO`| Micro increment counter (0-based)  | `0`     |
///
/// Common formats: `"YYYY.0M.0D"`, `"YYYY.MM"`, `"YYYY.0M.MICRO"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalVer {
    /// Format string used to render and parse this version.
    pub format: String,
    /// Calendar year.
    pub year: i32,
    /// Calendar month (1–12).
    pub month: u32,
    /// Calendar day (1–31).
    pub day: u32,
    /// Micro increment counter for multiple releases on the same date.
    pub micro: u64,
}

impl CalVer {
    /// Create a CalVer for today's date with `micro = 0`.
    pub fn today(format: &str) -> Self {
        let now = Utc::now().date_naive();
        Self::from_date(format, now, 0)
    }

    /// Create a CalVer for a specific date.
    pub fn from_date(format: &str, date: NaiveDate, micro: u64) -> Self {
        Self {
            format: format.to_string(),
            year: date.year(),
            month: date.month(),
            day: date.day(),
            micro,
        }
    }

    /// Return a bumped version.
    ///
    /// If today's date matches the stored date, increments the micro counter.
    /// Otherwise, resets to today's date with `micro = 0`.
    pub fn bump(&self) -> Self {
        let today = Utc::now().date_naive();
        let current =
            NaiveDate::from_ymd_opt(self.year, self.month, self.day).unwrap_or(NaiveDate::MIN);

        if today == current {
            Self {
                micro: self.micro + 1,
                ..self.clone()
            }
        } else {
            Self {
                year: today.year(),
                month: today.month(),
                day: today.day(),
                micro: 0,
                ..self.clone()
            }
        }
    }

    /// Render the version string using the stored format.
    pub fn render(&self) -> String {
        // replace tokens in longest-first order to avoid partial matches
        self.format
            .replace("YYYY", &format!("{:04}", self.year))
            .replace("0Y", &format!("{:02}", self.year % 100))
            .replace("YY", &format!("{}", self.year % 100))
            .replace("0M", &format!("{:02}", self.month))
            .replace("MM", &format!("{}", self.month))
            .replace("0D", &format!("{:02}", self.day))
            .replace("DD", &format!("{}", self.day))
            .replace("MICRO", &format!("{}", self.micro))
    }

    /// Parse a version string back into a `CalVer` using the given format.
    ///
    /// The format string is compiled into a regex by replacing each token with
    /// a capture group. Literal characters are treated as fixed separators.
    pub fn parse(s: &str, format: &str) -> Result<Self, CalVerError> {
        let (pattern, tokens) = format_to_regex(format)?;

        let re = Regex::new(&pattern).map_err(|e| CalVerError::Parse {
            s: s.to_string(),
            fmt: format.to_string(),
            reason: format!("internal regex error: {e}"),
        })?;

        let caps = re.captures(s).ok_or_else(|| CalVerError::Parse {
            s: s.to_string(),
            fmt: format.to_string(),
            reason: "string does not match format".to_string(),
        })?;

        let mut year = 0i32;
        let mut month = 1u32;
        let mut day = 1u32;
        let mut micro = 0u64;
        let mut year_seen = false;

        for (i, token) in tokens.iter().enumerate() {
            let cap = &caps[i + 1];
            match token.as_str() {
                "YYYY" => {
                    year = cap.parse().map_err(|_| CalVerError::Parse {
                        s: s.to_string(),
                        fmt: format.to_string(),
                        reason: format!("invalid year '{cap}'"),
                    })?;
                    year_seen = true;
                }
                "YY" | "0Y" if !year_seen => {
                    let yy: i32 = cap.parse().map_err(|_| CalVerError::Parse {
                        s: s.to_string(),
                        fmt: format.to_string(),
                        reason: format!("invalid short year '{cap}'"),
                    })?;
                    // assume 2000s for 2-digit years
                    year = 2000 + yy;
                }
                "YY" | "0Y" => {}
                "MM" | "0M" => {
                    month = cap.parse().map_err(|_| CalVerError::Parse {
                        s: s.to_string(),
                        fmt: format.to_string(),
                        reason: format!("invalid month '{cap}'"),
                    })?;
                }
                "DD" | "0D" => {
                    day = cap.parse().map_err(|_| CalVerError::Parse {
                        s: s.to_string(),
                        fmt: format.to_string(),
                        reason: format!("invalid day '{cap}'"),
                    })?;
                }
                "MICRO" => {
                    micro = cap.parse().map_err(|_| CalVerError::Parse {
                        s: s.to_string(),
                        fmt: format.to_string(),
                        reason: format!("invalid micro '{cap}'"),
                    })?;
                }
                _ => {}
            }
        }

        Ok(Self {
            format: format.to_string(),
            year,
            month,
            day,
            micro,
        })
    }
}

/// Convert a CalVer format string into a regex pattern with named capture
/// groups.
///
/// Returns `(pattern, token_order)` where `token_order` lists the format
/// tokens in the order they appear (matching the capture group indices).
fn format_to_regex(format: &str) -> Result<(String, Vec<String>), CalVerError> {
    let mut pattern = String::from("^");
    let mut tokens: Vec<String> = Vec::new();
    let mut remaining = format;

    // tokens ordered longest-first so we match greedily
    const TOKEN_LIST: &[(&str, &str)] = &[
        ("YYYY", r"(\d{4})"),
        ("MICRO", r"(\d+)"),
        ("0M", r"(\d{2})"),
        ("MM", r"(\d{1,2})"),
        ("0D", r"(\d{2})"),
        ("DD", r"(\d{1,2})"),
        ("0Y", r"(\d{2})"),
        ("YY", r"(\d{1,2})"),
    ];

    while !remaining.is_empty() {
        let mut matched = false;
        for (token, regex_part) in TOKEN_LIST {
            if remaining.starts_with(token) {
                pattern.push_str(regex_part);
                tokens.push(token.to_string());
                remaining = &remaining[token.len()..];
                matched = true;
                break;
            }
        }
        if !matched {
            // literal character: escape it for use in regex
            let ch = remaining.chars().next().unwrap();
            pattern.push_str(&regex::escape(&ch.to_string()));
            remaining = &remaining[ch.len_utf8()..];
        }
    }

    pattern.push('$');
    Ok((pattern, tokens))
}

impl fmt::Display for CalVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_full_date() {
        let v = CalVer {
            format: "YYYY.0M.0D".to_string(),
            year: 2024,
            month: 3,
            day: 7,
            micro: 0,
        };
        assert_eq!(v.render(), "2024.03.07");
    }

    #[test]
    fn test_render_no_padding() {
        let v = CalVer {
            format: "YYYY.MM.DD".to_string(),
            year: 2024,
            month: 3,
            day: 7,
            micro: 0,
        };
        assert_eq!(v.render(), "2024.3.7");
    }

    #[test]
    fn test_render_with_micro() {
        let v = CalVer {
            format: "YYYY.0M.MICRO".to_string(),
            year: 2024,
            month: 3,
            day: 1,
            micro: 2,
        };
        assert_eq!(v.render(), "2024.03.2");
    }

    #[test]
    fn test_render_short_year() {
        let v = CalVer {
            format: "YY.MM".to_string(),
            year: 2024,
            month: 3,
            day: 1,
            micro: 0,
        };
        assert_eq!(v.render(), "24.3");
    }

    #[test]
    fn test_parse_full_date() {
        let v = CalVer::parse("2024.03.07", "YYYY.0M.0D").unwrap();
        assert_eq!(v.year, 2024);
        assert_eq!(v.month, 3);
        assert_eq!(v.day, 7);
        assert_eq!(v.micro, 0);
    }

    #[test]
    fn test_parse_with_micro() {
        let v = CalVer::parse("2024.03.2", "YYYY.0M.MICRO").unwrap();
        assert_eq!(v.year, 2024);
        assert_eq!(v.month, 3);
        assert_eq!(v.micro, 2);
    }

    #[test]
    fn test_parse_short_year() {
        let v = CalVer::parse("24.3", "YY.MM").unwrap();
        assert_eq!(v.year, 2024);
        assert_eq!(v.month, 3);
    }

    #[test]
    fn test_parse_no_match() {
        let result = CalVer::parse("not-a-date", "YYYY.0M.0D");
        assert!(result.is_err());
    }

    #[test]
    fn test_display() {
        let v = CalVer::from_date(
            "YYYY.0M.0D",
            NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(),
            0,
        );
        assert_eq!(v.to_string(), "2024.12.25");
    }

    #[test]
    fn test_bump_different_day() {
        // set date far in the past so bump always triggers a date change
        let v = CalVer {
            format: "YYYY.MM.DD".to_string(),
            year: 2000,
            month: 1,
            day: 1,
            micro: 5,
        };
        let bumped = v.bump();
        // today is not 2000-01-01, so micro should reset
        assert_eq!(bumped.micro, 0);
        assert_ne!(bumped.year, 2000);
    }

    #[test]
    fn test_bump_same_day() {
        let today = Utc::now().date_naive();
        let v = CalVer {
            format: "YYYY.MM.DD.MICRO".to_string(),
            year: today.year(),
            month: today.month(),
            day: today.day(),
            micro: 3,
        };
        let bumped = v.bump();
        assert_eq!(bumped.micro, 4);
        assert_eq!(bumped.year, today.year());
    }
}
