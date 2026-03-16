//! Internationalization (i18n) infrastructure for cocoa.
//!
//! Sets up the message catalog backend used by the `t!` macro throughout the
//! codebase. Locale detection is handled by [`detect_locale`].

// initialize the global translation backend from the bundled locale catalog
rust_i18n::i18n!("locales");

pub use rust_i18n::{locale, set_locale, t};

/// Detects the preferred locale from standard environment variables.
///
/// Inspects `COCOA_LOCALE`, `LANG`, `LANGUAGE`, and `LC_ALL` in that order,
/// normalizing the result to a bare language code (e.g., `"en"` from
/// `"en_US.UTF-8"`). Falls back to `"en"` when no valid locale is found.
pub fn detect_locale() -> String {
    // placeholder — full detection implemented in 13.3
    "en".to_string()
}
