//! git operations abstraction for testability

use std::process::Command;

use crate::generate::GenerateError;

/// trait for git operations, allows mocking in tests
pub trait GitOperations {
    fn get_current_branch(&self) -> Result<String, GenerateError>;
    fn get_recent_commit_messages(&self, count: usize) -> Result<Vec<String>, GenerateError>;
    fn get_repository_name(&self) -> Result<String, GenerateError>;
    fn is_merge_in_progress(&self) -> bool;
    fn is_rebase_in_progress(&self) -> bool;
    fn get_staged_diff(&self) -> Result<String, GenerateError>;
    fn get_staged_files_by_status(&self, status: &str) -> Result<Vec<String>, GenerateError>;
}

/// real git operations using actual git commands
pub struct RealGitOps;

impl GitOperations for RealGitOps {
    fn get_current_branch(&self) -> Result<String, GenerateError> {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .output()
            .map_err(|e| GenerateError::GitCommand(format!("failed to run git branch: {}", e)))?;

        if !output.status.success() {
            return Err(GenerateError::GitCommand(
                "git branch command failed".to_string(),
            ));
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            return Err(GenerateError::GitContext(
                "not on any branch (detached HEAD)".to_string(),
            ));
        }

        Ok(branch)
    }

    fn get_recent_commit_messages(&self, count: usize) -> Result<Vec<String>, GenerateError> {
        let output = Command::new("git")
            .args(["log", &format!("-{}", count), "--oneline", "--format=%s"])
            .output()
            .map_err(|e| GenerateError::GitCommand(format!("failed to run git log: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let commits: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        Ok(commits)
    }

    fn get_repository_name(&self) -> Result<String, GenerateError> {
        let output = Command::new("git")
            .args(["config", "--get", "remote.origin.url"])
            .output()
            .map_err(|e| GenerateError::GitCommand(format!("failed to get remote url: {}", e)))?;

        if !output.status.success() {
            return Err(GenerateError::GitContext(
                "no remote origin found".to_string(),
            ));
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // extract repository name from URL
        let repo_name = url
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .strip_suffix(".git")
            .unwrap_or_else(|| url.split('/').next_back().unwrap_or("unknown"))
            .to_string();

        Ok(repo_name)
    }

    fn is_merge_in_progress(&self) -> bool {
        Command::new("git")
            .args(["rev-parse", "--verify", "MERGE_HEAD"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn is_rebase_in_progress(&self) -> bool {
        std::path::Path::new(".git/rebase-merge").exists()
            || std::path::Path::new(".git/rebase-apply").exists()
    }

    fn get_staged_diff(&self) -> Result<String, GenerateError> {
        let output = Command::new("git")
            .args(["diff", "--cached"])
            .output()
            .map_err(|e| GenerateError::GitCommand(format!("failed to run git diff: {}", e)))?;

        if !output.status.success() {
            return Err(GenerateError::StagedChanges(
                "git diff --cached failed".to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_staged_files_by_status(&self, status: &str) -> Result<Vec<String>, GenerateError> {
        let output = Command::new("git")
            .args(["diff", "--cached", "--name-status"])
            .output()
            .map_err(|e| {
                GenerateError::GitCommand(format!("failed to run git diff --name-status: {}", e))
            })?;

        if !output.status.success() {
            return Err(GenerateError::StagedChanges(
                "git diff --name-status failed".to_string(),
            ));
        }

        let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[0] == status {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(files)
    }
}

/// mock git operations for testing
#[cfg(test)]
pub struct MockGitOps {
    pub current_branch: Result<String, GenerateError>,
    pub recent_commits: Result<Vec<String>, GenerateError>,
    pub repository_name: Result<String, GenerateError>,
    pub is_merge: bool,
    pub is_rebase: bool,
    pub staged_diff: Result<String, GenerateError>,
    pub staged_files: std::collections::HashMap<String, Vec<String>>,
}

#[cfg(test)]
impl Default for MockGitOps {
    fn default() -> Self {
        Self {
            current_branch: Ok("main".to_string()),
            recent_commits: Ok(Vec::new()),
            repository_name: Ok("test-repo".to_string()),
            is_merge: false,
            is_rebase: false,
            staged_diff: Ok(String::new()),
            staged_files: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
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
}
