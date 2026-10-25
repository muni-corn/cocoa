//! Reporting of version tags that were ignored because HEAD cannot reach them.

use rust_i18n::t;

use crate::{
    git_ops::GitOperations,
    style::{print_info, print_warning},
    version,
};

/// Warn about version tags newer than the current version that HEAD cannot
/// reach, and return their names.
///
/// Such tags are usually left behind by a `git reset` or rebase. The warning
/// is skipped when `quiet` is set (for machine-readable output), but the names
/// are still returned so callers can include them in their own output. Scan
/// failures are ignored here because the command itself reports git errors.
pub fn report_ignored_tags(ops: &dyn GitOperations, prefix: &str, quiet: bool) -> Vec<String> {
    let Ok(found) = version::scan_version_tags(ops, prefix) else {
        return Vec::new();
    };

    let names: Vec<String> = found
        .ignored_newer()
        .iter()
        .map(|t| t.name.clone())
        .collect();

    if !names.is_empty() && !quiet {
        print_warning(t!("main.git.ignored_tags", tags = names.join(", ")));
        print_info(t!("main.git.ignored_tags_hint", tags = names.join(" ")));
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_ops::{MockGitOps, TagInfo};

    fn tag(name: &str, target: &str) -> TagInfo {
        TagInfo {
            name: name.to_string(),
            message: None,
            target: target.to_string(),
        }
    }

    #[test]
    fn test_report_ignored_tags_returns_unreachable_newer_tags() {
        let ops = MockGitOps {
            tags: Ok(vec![
                tag("v0.5.1", "a"),
                tag("v0.5.2", "b"),
                tag("v0.5.3", "c"),
            ]),
            unreachable_revs: vec!["b".to_string(), "c".to_string()],
            ..Default::default()
        };
        assert_eq!(report_ignored_tags(&ops, "v", true), ["v0.5.2", "v0.5.3"]);
    }

    #[test]
    fn test_report_ignored_tags_empty_when_all_reachable() {
        let ops = MockGitOps {
            tags: Ok(vec![tag("v1.0.0", "a")]),
            ..Default::default()
        };
        assert!(report_ignored_tags(&ops, "v", true).is_empty());
    }
}
