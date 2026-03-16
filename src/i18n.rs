//! Internationalization (i18n) infrastructure for cocoa.
//!
//! The translation backend is initialized at the crate root via
//! `rust_i18n::i18n!("locales")` in `lib.rs`. This module re-exports the
//! runtime helpers and provides locale detection via [`detect_locale`].

pub use rust_i18n::{locale, set_locale, t};

/// Detects the preferred locale from standard environment variables.
///
/// Inspects the following variables in priority order and returns the first
/// usable language code:
///
/// 1. `COCOA_LOCALE` — app-specific override
/// 2. `LC_ALL` — POSIX locale override
/// 3. `LANG` — primary POSIX locale
/// 4. `LANGUAGE` — GNU locale preference list (first entry used)
///
/// The value is normalized to a bare two-letter language code by stripping any
/// region suffix and encoding (e.g., `"en_US.UTF-8"` → `"en"`). Returns
/// `"en"` when no valid locale can be determined.
///
/// # Example
///
/// ```
/// // with LANG=fr_FR.UTF-8
/// // detect_locale() → "fr"
/// ```
pub fn detect_locale() -> String {
    // env var priority: app-specific → posix override → primary → gnu list
    let candidates = [
        std::env::var("COCOA_LOCALE"),
        std::env::var("LC_ALL"),
        std::env::var("LANG"),
        std::env::var("LANGUAGE"),
    ];

    for val in candidates.into_iter().flatten() {
        let lang = normalize_locale(&val);
        if !lang.is_empty() && lang != "c" && lang != "posix" {
            return lang;
        }
    }

    "en".to_string()
}

/// Strips region, encoding, and modifier suffixes from a POSIX locale string,
/// returning just the lowercase language code.
///
/// For example:
/// - `"en_US.UTF-8"` → `"en"`
/// - `"fr_FR"` → `"fr"`
/// - `"de"` → `"de"`
/// - `"zh_CN.UTF-8@hans"` → `"zh"`
fn normalize_locale(locale: &str) -> String {
    // take the first entry of a colon-separated LANGUAGE list
    let first = locale.split(':').next().unwrap_or(locale);

    // strip modifier (@...)
    let without_modifier = first.split('@').next().unwrap_or(first);

    // strip encoding (.UTF-8, .utf8, etc.)
    let without_encoding = without_modifier
        .split('.')
        .next()
        .unwrap_or(without_modifier);

    // take just the language portion before any region code (_XX or -XX)
    let lang = without_encoding
        .split(['_', '-'])
        .next()
        .unwrap_or(without_encoding);

    lang.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_strips_region_and_encoding() {
        assert_eq!(normalize_locale("en_US.UTF-8"), "en");
    }

    #[test]
    fn test_normalize_strips_region_only() {
        assert_eq!(normalize_locale("fr_FR"), "fr");
    }

    #[test]
    fn test_normalize_bare_lang() {
        assert_eq!(normalize_locale("de"), "de");
    }

    #[test]
    fn test_normalize_strips_modifier() {
        assert_eq!(normalize_locale("zh_CN.UTF-8@hans"), "zh");
    }

    #[test]
    fn test_normalize_language_list_uses_first() {
        assert_eq!(normalize_locale("en_US:fr_FR"), "en");
    }

    #[test]
    fn test_normalize_handles_dash_separator() {
        assert_eq!(normalize_locale("pt-BR"), "pt");
    }
}
