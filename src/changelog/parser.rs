//! Git history parsing for changelog generation.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::{
    changelog::{Changelog, ChangelogEntry, ChangelogError, ChangelogSection, ChangelogVersion},
    commit::CommitMessage,
    config::ChangelogConfig,
    git_ops::GitOperations,
};

/// Return the default human-readable section title for a conventional commit
/// type.
pub(crate) fn default_section_title(commit_type: &str) -> Option<&'static str> {
    match commit_type {
        "feat" => Some("Features"),
        "fix" => Some("Bug fixes"),
        "perf" => Some("Performance"),
        "docs" => Some("Documentation"),
        "test" => Some("Tests"),
        "build" => Some("Build system"),
        "ci" => Some("Continuous integration"),
        _ => None,
    }
}

/// Return the sort order for a section by commit type (lower = earlier in
/// output).
fn section_order(commit_type: &str) -> usize {
    match commit_type {
        "feat" => 0,
        "fix" => 1,
        "perf" => 2,
        "test" => 3,
        "build" => 4,
        "ci" => 5,
        "docs" => 6,
        "refactor" => 7,
        "chore" => 8,
        "style" => 9,
        _ => 999,
    }
}

/// Resolve the section title for a commit type using config overrides first,
/// then defaults.
fn get_section_title(commit_type: &str, config: &ChangelogConfig) -> Option<String> {
    if let Some(ref sections) = config.sections
        && let Some(title) = sections.get(commit_type)
    {
        return Some(title.clone());
    }
    default_section_title(commit_type).map(|s| s.to_string())
}

/// Format a Unix timestamp using the given strftime format string.
fn format_date(timestamp: i64, fmt: &str) -> String {
    let dt: DateTime<Utc> =
        DateTime::from_timestamp(timestamp, 0).unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
    dt.format(fmt).to_string()
}

/// Convert raw commit records into changelog entries, applying config filters.
fn build_entries(
    commits: &[crate::git_ops::CommitInfo],
    config: &ChangelogConfig,
) -> Vec<ChangelogEntry> {
    commits
        .iter()
        .filter_map(|c| {
            let msg = CommitMessage::parse(&c.summary).ok()?;

            // filter based on config flags
            if !config.include_merge_commits && msg.is_merge() {
                return None;
            }
            if !config.include_reverts && msg.is_revert() {
                return None;
            }

            let short_id = c.id.get(..8).unwrap_or(&c.id).to_string();
            let date = format_date(c.timestamp, &config.date_format);

            Some(ChangelogEntry {
                id: short_id,
                commit_type: msg.commit_type.unwrap_or_default(),
                scope: msg.scope.clone(),
                breaking: msg.breaking,
                subject: msg.subject.clone(),
                body: msg.body.clone(),
                author: c.author.clone(),
                timestamp: c.timestamp,
                date,
            })
        })
        .collect()
}

/// Assemble a `ChangelogVersion` from a list of raw entries.
///
/// Groups entries by commit type into sections, extracts breaking changes,
/// and sorts everything deterministically: sections follow a fixed type-order
/// (feat → fix → perf → …) and entries within each section are sorted
/// newest-first with SHA as a tiebreaker, so identical input always produces
/// identical output.
fn build_version(
    version: &str,
    date: Option<String>,
    entries: Vec<ChangelogEntry>,
    config: &ChangelogConfig,
) -> ChangelogVersion {
    // collect breaking changes from all entries (sorted newest-first, then by id)
    let mut breaking_changes: Vec<ChangelogEntry> =
        entries.iter().filter(|e| e.breaking).cloned().collect();
    breaking_changes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then(a.id.cmp(&b.id)));

    // group entries by commit type (skip empty types)
    let mut type_groups: HashMap<String, Vec<ChangelogEntry>> = HashMap::new();
    for entry in &entries {
        if entry.commit_type.is_empty() {
            continue;
        }
        // only include types with a known section title
        if get_section_title(&entry.commit_type, config).is_none() {
            continue;
        }
        type_groups
            .entry(entry.commit_type.clone())
            .or_default()
            .push(entry.clone());
    }

    // sort entries within each section deterministically: newest-first, then by SHA
    for section_entries in type_groups.values_mut() {
        section_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then(a.id.cmp(&b.id)));
    }

    // build and sort sections
    let mut sections: Vec<ChangelogSection> = type_groups
        .into_iter()
        .map(|(commit_type, entries)| {
            let title =
                get_section_title(&commit_type, config).unwrap_or_else(|| commit_type.clone());
            ChangelogSection {
                commit_type,
                title,
                entries,
            }
        })
        .collect();

    sections.sort_by(|a, b| {
        section_order(&a.commit_type)
            .cmp(&section_order(&b.commit_type))
            .then(a.commit_type.cmp(&b.commit_type))
    });

    ChangelogVersion {
        version: version.to_string(),
        date,
        breaking_changes,
        sections,
    }
}

/// Parse git history and produce a `Changelog`.
///
/// When `range` is `Some("from..to")`, returns a single "next" version
/// containing the commits in that range. When `range` is `None`, tags are used
/// to split history into per-version sections.
pub fn parse_history<G: GitOperations>(
    git_ops: &G,
    range: Option<&str>,
    config: &ChangelogConfig,
    next_version: Option<&str>,
) -> Result<Changelog, ChangelogError> {
    if let Some(r) = range {
        parse_range_history(git_ops, r, config, next_version)
    } else {
        build_versioned_history(git_ops, config, next_version)
    }
}

/// Build a single-version changelog from an explicit git range.
fn parse_range_history<G: GitOperations>(
    git_ops: &G,
    range: &str,
    config: &ChangelogConfig,
    next_version: Option<&str>,
) -> Result<Changelog, ChangelogError> {
    let (from, to) = range.split_once("..").ok_or_else(|| {
        ChangelogError::Git(format!("invalid range '{}': expected from..to", range))
    })?;

    let commits = git_ops
        .get_commits_in_range(from, to)
        .map_err(|e| ChangelogError::Git(e.to_string()))?;

    let entries = build_entries(&commits, config);
    let version = build_version(
        next_version.unwrap_or("Next version"),
        None,
        entries,
        config,
    );
    Ok(Changelog {
        versions: vec![version],
    })
}

/// Build a multi-version changelog by walking git history and splitting on
/// tags.
fn build_versioned_history<G: GitOperations>(
    git_ops: &G,
    config: &ChangelogConfig,
    next_version: Option<&str>,
) -> Result<Changelog, ChangelogError> {
    let all_commits = git_ops
        .get_commits_in_range("", "HEAD")
        .map_err(|e| ChangelogError::Git(e.to_string()))?;

    if all_commits.is_empty() {
        return Ok(Changelog { versions: vec![] });
    }

    let tags = git_ops
        .get_tags()
        .map_err(|e| ChangelogError::Git(e.to_string()))?;

    // build commit-SHA → tag-name map for fast lookup
    let tag_map: HashMap<String, String> = tags.into_iter().map(|t| (t.target, t.name)).collect();

    let mut versions = Vec::new();
    let mut current_commits: Vec<crate::git_ops::CommitInfo> = Vec::new();
    let mut current_version: Option<&str> = None;
    let mut current_date: Option<String> = None;

    for commit in all_commits {
        if let Some(tag_name) = tag_map.get(&commit.id) {
            // this commit is tagged; close the running bucket and start a new version
            if !current_commits.is_empty() || current_version.is_some() {
                let entries = build_entries(&current_commits, config);
                if !entries.is_empty() || current_version.is_some() {
                    versions.push(build_version(
                        current_version.or(next_version).unwrap_or("Next version"),
                        current_date.clone(),
                        entries,
                        config,
                    ));
                }
            }

            current_version = Some(tag_name);
            current_date = Some(format_date(commit.timestamp, &config.date_format));
            current_commits = vec![commit];
        } else {
            current_commits.push(commit);
        }
    }

    // close the final bucket
    if !current_commits.is_empty() {
        let entries = build_entries(&current_commits, config);
        if !entries.is_empty() {
            versions.push(build_version(
                current_version.or(next_version).unwrap_or("Next version"),
                current_date,
                entries,
                config,
            ));
        }
    } else if let Some(current_version) = current_version {
        // tagged version with no parseable entries
        versions.push(build_version(current_version, current_date, vec![], config));
    }

    Ok(Changelog { versions })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ChangelogConfig,
        git_ops::{CommitInfo, MockGitOps},
    };

    fn make_commit(id: &str, message: &str, timestamp: i64) -> CommitInfo {
        CommitInfo {
            id: id.to_string(),
            summary: message.to_string(),
            author: "Test User".to_string(),
            timestamp,
        }
    }

    fn default_config() -> ChangelogConfig {
        ChangelogConfig::default()
    }

    #[test]
    fn test_parse_history_empty() {
        let mock = MockGitOps {
            commits_in_range: Ok(vec![]),
            ..Default::default()
        };
        let config = default_config();
        let cl = parse_history(&mock, None, &config, None).unwrap();
        assert!(cl.versions.is_empty());
    }

    #[test]
    fn test_parse_history_no_tags_single_version() {
        let mock = MockGitOps {
            commits_in_range: Ok(vec![
                make_commit("aaaa1111bbbb2222", "feat: add login", 2000),
                make_commit("cccc3333dddd4444", "fix: fix crash", 1000),
            ]),
            tags: Ok(vec![]),
            ..Default::default()
        };
        let config = default_config();
        let cl = parse_history(&mock, None, &config, None).unwrap();
        assert_eq!(cl.versions.len(), 1);
        assert_eq!(cl.versions[0].version, "Next version"); // Unreleased
        assert_eq!(cl.versions[0].sections.len(), 2); // feat + fix
    }

    #[test]
    fn test_parse_history_with_range() {
        let mock = MockGitOps {
            commits_in_range: Ok(vec![make_commit(
                "aaaa1111bbbb2222",
                "feat: add feature",
                1000,
            )]),
            ..Default::default()
        };
        let config = default_config();
        let cl = parse_history(&mock, Some("v1.0.0..HEAD"), &config, None).unwrap();
        assert_eq!(cl.versions.len(), 1);
        assert_eq!(cl.versions[0].version, "Next version"); // range output is unreleased
    }

    #[test]
    fn test_parse_history_invalid_range() {
        let mock = MockGitOps::default();
        let config = default_config();
        assert!(parse_history(&mock, Some("notarange"), &config, None).is_err());
    }

    #[test]
    fn test_parse_history_with_tags() {
        let v1_sha = "aaaa1111bbbb2222cccc3333dddd4444eeee5555".to_string();
        let mock = MockGitOps {
            commits_in_range: Ok(vec![
                make_commit("ffff6666gggg7777", "feat: add v1.1 feature", 3000),
                make_commit(&v1_sha, "chore: release v1.0.0", 2000),
                make_commit("hhhh8888iiii9999", "fix: initial fix", 1000),
            ]),
            tags: Ok(vec![crate::git_ops::TagInfo {
                name: "v1.0.0".to_string(),
                message: None,
                target: v1_sha.clone(),
            }]),
            ..Default::default()
        };
        let config = default_config();
        let cl = parse_history(&mock, None, &config, None).unwrap();

        // should produce: Unreleased[feat] and v1.0.0[chore, fix]
        assert_eq!(cl.versions.len(), 2);
        assert_eq!(cl.versions[0].version, "Next version"); // Unreleased
        assert_eq!(cl.versions[1].version, "v1.0.0");
    }

    #[test]
    fn test_breaking_changes_extracted() {
        let mock = MockGitOps {
            commits_in_range: Ok(vec![
                make_commit("aaaa1111bbbb2222", "feat!: breaking api change", 1000),
                make_commit("cccc3333dddd4444", "fix: normal fix", 900),
            ]),
            tags: Ok(vec![]),
            ..Default::default()
        };
        let config = default_config();
        let cl = parse_history(&mock, None, &config, None).unwrap();
        assert_eq!(cl.versions[0].breaking_changes.len(), 1);
        assert_eq!(
            cl.versions[0].breaking_changes[0].subject,
            "breaking api change"
        );
    }

    #[test]
    fn test_merge_commits_excluded_by_default() {
        let mock = MockGitOps {
            commits_in_range: Ok(vec![
                make_commit("aaaa1111bbbb2222", "Merge branch 'feature'", 2000),
                make_commit("cccc3333dddd4444", "feat: real feature", 1000),
            ]),
            tags: Ok(vec![]),
            ..Default::default()
        };
        let config = default_config();
        let cl = parse_history(&mock, None, &config, None).unwrap();
        // merge commit filtered out, only feat remains
        assert_eq!(cl.versions[0].sections.len(), 1);
        assert_eq!(cl.versions[0].sections[0].commit_type, "feat");
    }

    #[test]
    fn test_section_order_is_deterministic() {
        let mock = MockGitOps {
            commits_in_range: Ok(vec![
                make_commit("aaaa1111bbbb2222", "chore: update deps", 4000),
                make_commit("cccc3333dddd4444", "fix: fix thing", 3000),
                make_commit("eeee5555ffff6666", "docs: update docs", 2000),
                make_commit("gggg7777hhhh8888", "feat: add feature", 1000),
            ]),
            tags: Ok(vec![]),
            ..Default::default()
        };
        let config = default_config();
        let cl = parse_history(&mock, None, &config, None).unwrap();
        let types: Vec<&str> = cl.versions[0]
            .sections
            .iter()
            .map(|s| s.commit_type.as_str())
            .collect();
        // feat(0) < fix(1) < docs(4) < chore(9)
        assert_eq!(types, vec!["feat", "fix", "docs"]);
    }
}
