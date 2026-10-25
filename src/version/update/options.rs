//! Options that control how version files are updated.

/// Options for updating version files.
///
/// The default is the strictest behavior: handlers verify that the files hold
/// the version they are about to replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateOptions {
    /// Whether handlers must find `old_version` in the files they update.
    ///
    /// Turn this off when `old_version` is only a placeholder, such as the
    /// `0.0.0` used before the first release has been tagged.
    pub verify_old_version: bool,
}

impl Default for UpdateOptions {
    fn default() -> Self {
        Self {
            verify_old_version: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_verifies_old_version() {
        assert!(UpdateOptions::default().verify_old_version);
    }
}
