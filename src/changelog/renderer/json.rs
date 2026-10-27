//! JSON changelog renderer.

use crate::changelog::{Changelog, ChangelogError};

/// Serializes the changelog to pretty-printed JSON.
///
/// JSON stays a serializer rather than a template so that escaping is always
/// handled by serde.
pub fn render_json(changelog: &Changelog) -> Result<String, ChangelogError> {
    serde_json::to_string_pretty(changelog).map_err(|e| ChangelogError::Render(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changelog::ChangelogVersion;

    #[test]
    fn test_render_json_is_valid() {
        let cl = Changelog {
            versions: vec![ChangelogVersion {
                version: "v1.0.0".to_string(),
                date: Some("2024-01-01".to_string()),
                breaking_changes: vec![],
                sections: vec![],
            }],
        };
        let out = render_json(&cl).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(parsed["versions"].is_array());
        assert_eq!(parsed["versions"][0]["version"], "v1.0.0");
    }
}
