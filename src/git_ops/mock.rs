//! In-memory `GitOperations` implementation for tests.

use std::{collections::HashMap, path::PathBuf};

use super::{CommitInfo, GitOperations, TagInfo};
use crate::generate::GenerateError;

/// Mock git operations for testing.
///
/// Every field is returned as-is by the matching trait method, so tests set
/// only what they care about and take the rest from `Default`.
pub struct MockGitOps {
    pub current_branch: Result<String, GenerateError>,
    pub recent_commits: Result<Vec<String>, GenerateError>,
    pub repository_name: Result<String, GenerateError>,
    pub is_merge: bool,
    pub is_rebase: bool,
    pub staged_diff: Result<String, GenerateError>,
    pub staged_files: HashMap<String, Vec<String>>,
    pub commits_in_range: Result<Vec<CommitInfo>, GenerateError>,
    pub tags: Result<Vec<TagInfo>, GenerateError>,
    pub hook_path: Result<PathBuf, GenerateError>,
    pub repo_root: Result<PathBuf, GenerateError>,
    /// Revisions that `is_reachable_from_head` reports as unreachable.
    pub unreachable_revs: Vec<String>,
}

impl Default for MockGitOps {
    fn default() -> Self {
        Self {
            current_branch: Ok("main".to_string()),
            recent_commits: Ok(Vec::new()),
            repository_name: Ok("test-repo".to_string()),
            is_merge: false,
            is_rebase: false,
            staged_diff: Ok(String::new()),
            staged_files: HashMap::new(),
            commits_in_range: Ok(Vec::new()),
            tags: Ok(Vec::new()),
            hook_path: Ok(PathBuf::from(".git/hooks")),
            repo_root: Ok(PathBuf::from(".")),
            unreachable_revs: Vec::new(),
        }
    }
}

impl GitOperations for MockGitOps {
    fn get_current_branch(&self) -> Result<String, GenerateError> {
        self.current_branch.clone()
    }

    fn get_recent_commit_messages(&self, _count: usize) -> Result<Vec<String>, GenerateError> {
        self.recent_commits.clone()
    }

    fn get_repository_name(&self) -> Result<String, GenerateError> {
        self.repository_name.clone()
    }

    fn is_merge_in_progress(&self) -> bool {
        self.is_merge
    }

    fn is_rebase_in_progress(&self) -> bool {
        self.is_rebase
    }

    fn get_staged_diff(&self) -> Result<String, GenerateError> {
        self.staged_diff.clone()
    }

    fn get_staged_files_by_status(&self, status: &str) -> Result<Vec<String>, GenerateError> {
        Ok(self
            .staged_files
            .get(status)
            .cloned()
            .unwrap_or_else(Vec::new))
    }

    fn get_commits_in_range(
        &self,
        _from: &str,
        _to: &str,
    ) -> Result<Vec<CommitInfo>, GenerateError> {
        self.commits_in_range.clone()
    }

    fn get_tags(&self) -> Result<Vec<TagInfo>, GenerateError> {
        self.tags.clone()
    }

    fn is_reachable_from_head(&self, rev: &str) -> Result<bool, GenerateError> {
        Ok(!self.unreachable_revs.iter().any(|r| r == rev))
    }

    fn create_tag(&self, _name: &str, _message: &str, _sign: bool) -> Result<(), GenerateError> {
        Ok(())
    }

    fn create_commit(&self, _message: &str) -> Result<(), GenerateError> {
        Ok(())
    }

    fn get_hook_path(&self) -> Result<PathBuf, GenerateError> {
        self.hook_path.clone()
    }

    fn get_repo_root(&self) -> Result<PathBuf, GenerateError> {
        self.repo_root.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_git_ops_default() {
        let mock = MockGitOps::default();
        assert_eq!(mock.get_current_branch().unwrap(), "main");
        assert!(mock.get_recent_commit_messages(5).unwrap().is_empty());
        assert_eq!(mock.get_repository_name().unwrap(), "test-repo");
        assert!(!mock.is_merge_in_progress());
        assert!(!mock.is_rebase_in_progress());
        assert!(mock.get_tags().unwrap().is_empty());
        assert!(mock.get_commits_in_range("", "HEAD").unwrap().is_empty());
        assert!(mock.get_hook_path().is_ok());
        assert!(mock.get_repo_root().is_ok());
    }

    #[test]
    fn test_mock_git_ops_custom() {
        let mock = MockGitOps {
            current_branch: Ok("feature/test".to_string()),
            recent_commits: Ok(vec!["feat: add feature".to_string()]),
            is_merge: true,
            ..Default::default()
        };

        assert_eq!(mock.get_current_branch().unwrap(), "feature/test");
        assert_eq!(mock.get_recent_commit_messages(5).unwrap().len(), 1);
        assert!(mock.is_merge_in_progress());
    }

    #[test]
    fn test_mock_git_ops_error() {
        let mock = MockGitOps {
            current_branch: Err(GenerateError::GitContext("test error".to_string())),
            ..Default::default()
        };

        assert!(mock.get_current_branch().is_err());
    }

    #[test]
    fn test_mock_git_ops_tags() {
        let mock = MockGitOps {
            tags: Ok(vec![TagInfo {
                name: "v1.0.0".to_string(),
                message: Some("release v1.0.0".to_string()),
                target: "abc123".to_string(),
            }]),
            ..Default::default()
        };

        let tags = mock.get_tags().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.0.0");
        assert_eq!(tags[0].message, Some("release v1.0.0".to_string()));
    }

    #[test]
    fn test_mock_git_ops_commits_in_range() {
        let mock = MockGitOps {
            commits_in_range: Ok(vec![
                CommitInfo {
                    id: "abc123".to_string(),
                    summary: "feat: add feature".to_string(),
                    author: "Alice".to_string(),
                    timestamp: 1000,
                },
                CommitInfo {
                    id: "def456".to_string(),
                    summary: "fix: fix bug".to_string(),
                    author: "Bob".to_string(),
                    timestamp: 900,
                },
            ]),
            ..Default::default()
        };

        let commits = mock.get_commits_in_range("v0.9.0", "HEAD").unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "feat: add feature");
        assert_eq!(commits[1].author, "Bob");
    }

    #[test]
    fn test_mock_git_ops_reports_unreachable_revs() {
        let mock = MockGitOps {
            unreachable_revs: vec!["abc".to_string()],
            ..Default::default()
        };
        assert!(!mock.is_reachable_from_head("abc").unwrap());
        assert!(mock.is_reachable_from_head("def").unwrap());
    }
}
