//! Security utilities for detecting and sanitizing sensitive content.

use std::sync::LazyLock;

use regex::Regex;

/// A pattern match indicating potentially sensitive content in a diff.
pub struct SensitiveMatch {
    /// Human-readable name of the matched pattern.
    pub pattern_name: &'static str,
    /// One-indexed line number within the diff where the match was found.
    pub line_number: usize,
}

/// Named sensitive-content patterns paired with their regular expressions.
///
/// Patterns are intentionally specific to minimise false positives. Each entry
/// is a `(name, regex)` pair where the regex is compiled once at first access.
static SENSITIVE_PATTERNS: &[(&str, &str)] = &[
    ("AWS access key ID", r"\bAKIA[0-9A-Z]{16}\b"),
    ("GitHub personal access token", r"\bghp_[A-Za-z0-9]{36}\b"),
    (
        "GitHub fine-grained token",
        r"\bgithub_pat_[A-Za-z0-9_]{82}\b",
    ),
    ("Slack token", r"\bxox[baprs]-[0-9A-Za-z\-]{10,96}\b"),
    (
        "PEM private key",
        r"-----BEGIN (?:RSA |EC |OPENSSH |PGP )?PRIVATE KEY",
    ),
    (
        "JWT token",
        r"\beyJ[A-Za-z0-9_\-]{4,}\.[A-Za-z0-9_\-]{4,}\.[A-Za-z0-9_\-]{4,}\b",
    ),
    (
        "API key assignment",
        r"(?i)(?:api[_\-]?key|api[_\-]?secret|auth[_\-]?token|access[_\-]?token)\s*[=:]\s*['\x22]?[A-Za-z0-9_\-+/]{20,}",
    ),
    (
        "password assignment",
        r"(?i)(?:password|passwd)\s*=\s*['\x22]?[A-Za-z0-9!@#$%^&*()\-_+=]{8,}['\x22]?",
    ),
    (
        "HTTP bearer token",
        r"(?i)Authorization\s*[:=]\s*Bearer\s+[A-Za-z0-9_\-\.]{20,}",
    ),
    (
        "credentials in URL",
        r"https?://[A-Za-z0-9_%+\-.]+:[^@/\s]{4,}@",
    ),
];

/// Compiled patterns, lazily initialised on first access.
static COMPILED: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    SENSITIVE_PATTERNS
        .iter()
        .filter_map(|(name, pattern)| Regex::new(pattern).ok().map(|re| (*name, re)))
        .collect()
});

/// Scans added lines in a unified diff for sensitive content patterns.
///
/// Only inspects lines beginning with `+` (excluding `+++` file headers),
/// which represent newly introduced content. Returns one `SensitiveMatch` per
/// `(line, pattern)` pair discovered.
pub fn scan_diff(diff: &str) -> Vec<SensitiveMatch> {
    let compiled = &*COMPILED;
    let mut matches = Vec::new();

    for (line_number, line) in diff.lines().enumerate() {
        // only check added lines; skip file header lines ("+++ b/...")
        if !line.starts_with('+') || line.starts_with("+++") {
            continue;
        }

        // strip the leading `+` origin marker before checking
        let content = &line[1..];

        for (pattern_name, re) in compiled {
            if re.is_match(content) {
                matches.push(SensitiveMatch {
                    pattern_name,
                    line_number: line_number + 1,
                });
            }
        }
    }

    matches
}

/// Redacts sensitive content from an arbitrary string by replacing pattern
/// matches with `[REDACTED]`.
///
/// Intended for sanitising error messages before displaying them to users,
/// preventing accidental secret leakage in diagnostic output.
pub fn redact(s: &str) -> String {
    let compiled = &*COMPILED;
    let mut result = s.to_string();
    for (_, re) in compiled {
        result = re.replace_all(&result, "[REDACTED]").into_owned();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_diff_detects_aws_key() {
        let diff = "diff --git a/config.env b/config.env\n\
            +++ b/config.env\n\
            +AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n";

        let matches = scan_diff(diff);
        assert!(
            matches
                .iter()
                .any(|m| m.pattern_name == "AWS access key ID"),
            "should detect AWS access key ID"
        );
    }

    #[test]
    fn test_scan_diff_ignores_removed_lines() {
        let diff = "diff --git a/config.env b/config.env\n\
            --- a/config.env\n\
            +++ b/config.env\n\
            -AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n";

        let matches = scan_diff(diff);
        assert!(matches.is_empty(), "should not flag removed lines");
    }

    #[test]
    fn test_scan_diff_ignores_context_lines() {
        // context lines start with a space, not a `+`
        let diff = " AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n";
        let matches = scan_diff(diff);
        assert!(
            matches.is_empty(),
            "should not flag context (unchanged) lines"
        );
    }

    #[test]
    fn test_scan_diff_detects_github_token() {
        // 36-char alphanumeric suffix is required by the pattern
        let diff = "+token = \"ghp_abcdefghijklmnopqrstuvwxyz1234567890\"\n";
        let matches = scan_diff(diff);
        assert!(
            matches
                .iter()
                .any(|m| m.pattern_name == "GitHub personal access token"),
            "should detect GitHub personal access token"
        );
    }

    #[test]
    fn test_scan_diff_detects_pem_key() {
        let diff = "+-----BEGIN RSA PRIVATE KEY-----\n";
        let matches = scan_diff(diff);
        assert!(
            matches.iter().any(|m| m.pattern_name == "PEM private key"),
            "should detect PEM private key header"
        );
    }

    #[test]
    fn test_scan_diff_detects_bearer_token() {
        let diff = "+Authorization: Bearer eyJhbGciOiJSUzI1NiJ9.payload.signature\n";
        let matches = scan_diff(diff);
        assert!(
            matches
                .iter()
                .any(|m| m.pattern_name == "HTTP bearer token" || m.pattern_name == "JWT token"),
            "should detect bearer token or JWT"
        );
    }

    #[test]
    fn test_scan_diff_no_matches_for_clean_diff() {
        let diff = "diff --git a/src/lib.rs b/src/lib.rs\n\
            +++ b/src/lib.rs\n\
            +pub fn hello() -> &'static str {\n\
            +    \"hello\"\n\
            +}\n";
        let matches = scan_diff(diff);
        assert!(matches.is_empty(), "clean diff should produce no matches");
    }

    #[test]
    fn test_scan_diff_reports_correct_line_number() {
        let diff = "+clean line\n\
            +another clean line\n\
            +AWS_KEY=AKIAIOSFODNN7EXAMPLE\n";
        let matches = scan_diff(diff);
        let aws = matches
            .iter()
            .find(|m| m.pattern_name == "AWS access key ID");
        assert!(aws.is_some());
        assert_eq!(
            aws.unwrap().line_number,
            3,
            "line number should be 1-indexed"
        );
    }

    #[test]
    fn test_redact_removes_aws_key() {
        let s = "error calling API with AKIAIOSFODNN7EXAMPLE key";
        let redacted = redact(s);
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_leaves_clean_strings_unchanged() {
        let s = "a normal error message without secrets";
        assert_eq!(redact(s), s);
    }

    #[test]
    fn test_redact_removes_github_token() {
        let s = "failed with token ghp_abcdefghijklmnopqrstuvwxyz1234567890 attached";
        let redacted = redact(s);
        assert!(!redacted.contains("ghp_abcdefghijklmnopqrstuvwxyz1234567890"));
        assert!(redacted.contains("[REDACTED]"));
    }
}
