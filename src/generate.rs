use thiserror::Error;

use crate::{
    ai::client::{Client as AiClient, CommitContext},
    config::Config,
    git_ops::{GitOperations, RealGitOps},
    lint::Linter,
};

#[derive(Error, Debug, Clone)]
pub enum GenerateError {
    #[error("no staged changes found - use `git add` to stage files first")]
    NoStagedChanges,

    #[error("failed to extract git context: {0}")]
    GitContext(String),

    #[error("failed to analyze staged changes: {0}")]
    StagedChanges(String),

    #[error("ai generation failed: {0}")]
    AiGeneration(String),

    #[error("generated message failed validation: {0}")]
    Validation(String),

    #[error("git command failed: {0}")]
    GitCommand(String),
}

#[derive(Debug)]
pub struct StagedChanges {
    pub diff: String,
    pub files_added: Vec<String>,
    pub files_modified: Vec<String>,
    pub files_deleted: Vec<String>,
    pub total_additions: usize,
    pub total_deletions: usize,
}

pub async fn generate_commit_message(config: &Config) -> Result<String, GenerateError> {
    generate_commit_message_with_git(config, &RealGitOps).await
}

pub async fn generate_commit_message_with_git<G: GitOperations>(
    config: &Config,
    git_ops: &G,
) -> Result<String, GenerateError> {
    let staged_changes = analyze_staged_changes_with_git(git_ops)?;

    if staged_changes.diff.trim().is_empty() {
        return Err(GenerateError::NoStagedChanges);
    }

    let context = extract_git_context_with_git(git_ops)?;

    let ai_config = config
        .ai
        .as_ref()
        .ok_or_else(|| GenerateError::AiGeneration("ai configuration not found".to_string()))?;

    let ai_client = AiClient::new(ai_config.clone())
        .map_err(|e| GenerateError::AiGeneration(format!("failed to create ai client: {}", e)))?;

    let generated_message = ai_client
        .generate_commit_message(&staged_changes.diff, &context)
        .await
        .map_err(|e| GenerateError::AiGeneration(e.to_string()))?;

    validate_generated_message(&generated_message, config)?;

    Ok(generated_message)
}

pub fn extract_git_context() -> Result<CommitContext, GenerateError> {
    extract_git_context_with_git(&RealGitOps)
}

pub fn extract_git_context_with_git<G: GitOperations>(
    git_ops: &G,
) -> Result<CommitContext, GenerateError> {
    let branch_name = git_ops.get_current_branch().ok();
    let recent_commits = git_ops.get_recent_commit_messages(5)?;
    let repository_name = git_ops.get_repository_name().ok();
    let is_merge = git_ops.is_merge_in_progress();
    let is_rebase = git_ops.is_rebase_in_progress();

    Ok(CommitContext {
        branch_name,
        recent_commits,
        repository_name,
        is_merge,
        is_rebase,
    })
}

pub fn analyze_staged_changes() -> Result<StagedChanges, GenerateError> {
    analyze_staged_changes_with_git(&RealGitOps)
}

pub fn analyze_staged_changes_with_git<G: GitOperations>(
    git_ops: &G,
) -> Result<StagedChanges, GenerateError> {
    let diff = git_ops.get_staged_diff()?;

    if diff.trim().is_empty() {
        return Err(GenerateError::NoStagedChanges);
    }

    let files_added = git_ops.get_staged_files_by_status("A")?;
    let files_modified = git_ops.get_staged_files_by_status("M")?;
    let files_deleted = git_ops.get_staged_files_by_status("D")?;

    let (total_additions, total_deletions) = count_diff_changes(&diff);

    Ok(StagedChanges {
        diff,
        files_added,
        files_modified,
        files_deleted,
        total_additions,
        total_deletions,
    })
}

fn count_diff_changes(diff: &str) -> (usize, usize) {
    let mut additions = 0;
    let mut deletions = 0;

    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }

    (additions, deletions)
}

fn validate_generated_message(message: &str, config: &Config) -> Result<(), GenerateError> {
    // use existing lint module to validate the message
    let linter = Linter::new(config);
    let result = linter.lint(message);

    if result.is_valid {
        Ok(())
    } else {
        let error_messages: Vec<String> = result
            .violations
            .iter()
            .map(|v| format!("{}: {}", v.rule, v.message))
            .collect();
        Err(GenerateError::Validation(error_messages.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::git_ops::MockGitOps;

    #[test]
    fn test_count_diff_changes() {
        let diff = r#"
diff --git a/src/main.rs b/src/main.rs
index 1234567..abcdefg 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,5 @@
 fn main() {
+    println!("hello");
     println!("world");
-    // old comment
+    // new comment
 }
"#;

        let (additions, deletions) = count_diff_changes(diff);
        assert_eq!(additions, 2);
        assert_eq!(deletions, 1);
    }

    #[test]
    fn test_staged_changes_empty_diff() {
        // test that empty diff is handled properly
        let (additions, deletions) = count_diff_changes("");
        assert_eq!(additions, 0);
        assert_eq!(deletions, 0);
    }

    #[test]
    fn test_extract_repository_name() {
        // test the URL parsing logic directly
        let test_urls = vec![
            ("https://github.com/user/repo.git", "repo"),
            ("git@github.com:user/repo.git", "repo"),
            ("https://github.com/user/my-project", "my-project"),
        ];

        for (url, expected) in test_urls {
            let repo_name = url
                .rsplit('/')
                .next()
                .unwrap()
                .strip_suffix(".git")
                .unwrap_or_else(|| url.rsplit('/').next().unwrap())
                .to_string();

            assert_eq!(repo_name, expected);
        }
    }

    #[test]
    fn test_commit_context_creation() {
        let context = CommitContext {
            branch_name: Some("feature/new-feature".to_string()),
            recent_commits: vec![
                "fix: update dependencies".to_string(),
                "feat: add new component".to_string(),
            ],
            repository_name: Some("cocoa".to_string()),
            is_merge: false,
            is_rebase: false,
        };

        assert_eq!(context.branch_name, Some("feature/new-feature".to_string()));
        assert_eq!(context.recent_commits.len(), 2);
        assert!(!context.is_merge);
        assert!(!context.is_rebase);
    }

    #[test]
    fn test_staged_changes_creation() {
        let changes = StagedChanges {
            diff: "test diff".to_string(),
            files_added: vec!["new_file.rs".to_string()],
            files_modified: vec!["existing_file.rs".to_string()],
            files_deleted: vec!["old_file.rs".to_string()],
            total_additions: 10,
            total_deletions: 5,
        };

        assert_eq!(changes.files_added.len(), 1);
        assert_eq!(changes.files_modified.len(), 1);
        assert_eq!(changes.files_deleted.len(), 1);
        assert_eq!(changes.total_additions, 10);
        assert_eq!(changes.total_deletions, 5);
    }

    #[test]
    fn test_extract_git_context_with_mock() {
        let mock = MockGitOps {
            current_branch: Ok("feature/test".to_string()),
            recent_commits: Ok(vec![
                "feat: add feature".to_string(),
                "fix: bug fix".to_string(),
            ]),
            repository_name: Ok("test-repo".to_string()),
            ..Default::default()
        };

        let context = extract_git_context_with_git(&mock).unwrap();

        assert_eq!(context.branch_name, Some("feature/test".to_string()));
        assert_eq!(context.recent_commits.len(), 2);
        assert_eq!(context.repository_name, Some("test-repo".to_string()));
        assert!(!context.is_merge);
        assert!(!context.is_rebase);
    }

    #[test]
    fn test_analyze_staged_changes_with_mock() {
        let mock = MockGitOps {
            staged_diff: Ok(r#"
diff --git a/test.rs b/test.rs
+++ test.rs
+fn new_function() {}
"#
            .to_string()),
            staged_files: HashMap::from([("A".to_string(), vec!["test.rs".to_string()])]),
            ..Default::default()
        };

        let changes = analyze_staged_changes_with_git(&mock).unwrap();

        assert!(!changes.diff.is_empty());
        assert_eq!(changes.files_added.len(), 1);
        assert_eq!(changes.total_additions, 1);
    }

    #[test]
    fn test_analyze_staged_changes_empty_diff() {
        let mock = MockGitOps {
            staged_diff: Ok("".to_string()),
            ..Default::default()
        };

        let result = analyze_staged_changes_with_git(&mock);
        assert!(matches!(result, Err(GenerateError::NoStagedChanges)));
    }
}
