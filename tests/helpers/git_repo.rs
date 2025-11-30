//! Test helper for creating temporary git repositories.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use cocoa::{generate::GenerateError, git_ops::GitOperations};
use tempfile::TempDir;

/// Helper struct for creating and managing test git repositories.
pub struct TestRepo {
    /// Temporary directory containing the repo.
    pub dir: TempDir,
    /// Path to the repository.
    pub path: PathBuf,
    /// Temporary home directory for isolated git config.
    pub home_dir: TempDir,
}

impl TestRepo {
    /// Create a new test git repository with basic configuration.
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let home_dir = tempfile::tempdir().unwrap();

        let repo = Self {
            dir,
            path: path.clone(),
            home_dir,
        };

        // initialize git repository
        repo.create_git_command(&["init"])
            .output()
            .expect("failed to initialize git repository");

        // configure git user
        repo.create_git_command(&["config", "user.name", "Test User"])
            .output()
            .expect("failed to configure git user name");

        repo.create_git_command(&["config", "user.email", "test@example.com"])
            .output()
            .expect("failed to configure git user email");

        repo
    }

    /// Create a git command with isolated environment variables.
    ///
    /// This prevents personal git configuration from interfering with tests by:
    /// - Setting GIT_CONFIG_NOSYSTEM=1 to ignore system-wide config
    /// - Setting HOME to an isolated temporary directory
    /// - Setting XDG_CONFIG_HOME to an isolated config directory
    pub fn create_git_command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new("git");
        cmd.args(args)
            .current_dir(&self.path)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", self.home_dir.path())
            .env("XDG_CONFIG_HOME", self.home_dir.path().join(".config"));
        cmd
    }

    /// Create a new file with given content.
    pub fn create_file<P: AsRef<Path>>(&self, path: P, content: &str) {
        let file_path = self.path.join(path);

        // create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("failed to create parent directories");
        }

        std::fs::write(&file_path, content).expect("failed to write file");
    }

    /// Stage a file (must already exist).
    pub fn stage_file<P: AsRef<Path>>(&self, path: P) {
        let output = self
            .create_git_command(&["add", path.as_ref().to_str().unwrap()])
            .output()
            .expect("failed to stage file");

        if !output.status.success() {
            panic!(
                "failed to stage file: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Create and stage a file in one operation.
    pub fn create_and_stage_file<P: AsRef<Path>>(&self, path: P, content: &str) {
        self.create_file(path.as_ref(), content);
        self.stage_file(path);
    }

    /// Modify an existing file.
    pub fn modify_file<P: AsRef<Path>>(&self, path: P, content: &str) {
        self.create_file(path, content);
    }

    /// Delete a file.
    pub fn delete_file<P: AsRef<Path>>(&self, path: P) {
        let file_path = self.path.join(path);
        std::fs::remove_file(&file_path).expect("failed to delete file");
    }

    /// Commit staged changes with given message.
    pub fn commit(&self, message: &str) {
        let output = self
            .create_git_command(&["commit", "-m", message])
            .output()
            .expect("failed to commit");

        if !output.status.success() {
            panic!(
                "failed to commit: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Create a file, stage it, and commit in one operation.
    pub fn create_commit<P: AsRef<Path>>(&self, path: P, content: &str, message: &str) {
        self.create_and_stage_file(path, content);
        self.commit(message);
    }

    /// Create a new branch.
    pub fn create_branch(&self, name: &str) {
        self.create_git_command(&["branch", name])
            .output()
            .expect("failed to create branch");
    }

    /// Checkout a branch.
    pub fn checkout(&self, branch: &str) {
        self.create_git_command(&["checkout", branch])
            .output()
            .expect("failed to checkout branch");
    }

    /// Get the current branch name.
    pub fn current_branch(&self) -> String {
        let output = self
            .create_git_command(&["branch", "--show-current"])
            .output()
            .expect("failed to get current branch");

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Get the last commit message.
    pub fn last_commit_message(&self) -> String {
        let output = self
            .create_git_command(&["log", "-1", "--format=%s"])
            .output()
            .expect("failed to get last commit message");

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Set a remote URL.
    pub fn set_remote(&self, name: &str, url: &str) {
        self.create_git_command(&["remote", "add", name, url])
            .output()
            .expect("failed to set remote");
    }

    /// Get staged diff.
    pub fn get_staged_diff(&self) -> String {
        let output = self
            .create_git_command(&["diff", "--cached"])
            .output()
            .expect("failed to get staged diff");

        String::from_utf8_lossy(&output.stdout).to_string()
    }

    /// Check if there are staged changes.
    pub fn has_staged_changes(&self) -> bool {
        !self.get_staged_diff().trim().is_empty()
    }
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement GitOperations for TestRepo to allow using it directly in tests.
///
/// This provides a clean way to test git operations without needing to modify
/// global environment variables or the working directory.
impl GitOperations for TestRepo {
    fn get_current_branch(&self) -> Result<String, GenerateError> {
        let output = self
            .create_git_command(&["branch", "--show-current"])
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
        let output = self
            .create_git_command(&["log", &format!("-{}", count), "--oneline", "--format=%s"])
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
        let output = self
            .create_git_command(&["config", "--get", "remote.origin.url"])
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
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .strip_suffix(".git")
            .unwrap_or_else(|| url.rsplit('/').next().unwrap_or("unknown"))
            .to_string();

        Ok(repo_name)
    }

    fn is_merge_in_progress(&self) -> bool {
        self.create_git_command(&["rev-parse", "--verify", "MERGE_HEAD"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn is_rebase_in_progress(&self) -> bool {
        self.path.join(".git/rebase-merge").exists() || self.path.join(".git/rebase-apply").exists()
    }

    fn get_staged_diff(&self) -> Result<String, GenerateError> {
        let output = self
            .create_git_command(&["diff", "--cached"])
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
        let output = self
            .create_git_command(&["diff", "--cached", "--name-status"])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_creation() {
        let repo = TestRepo::new();
        assert!(repo.path.exists());
        assert!(repo.path.join(".git").exists());
    }

    #[test]
    fn test_create_and_stage_file() {
        let repo = TestRepo::new();
        repo.create_and_stage_file("test.txt", "hello world");

        assert!(repo.path.join("test.txt").exists());
        assert!(repo.has_staged_changes());
    }

    #[test]
    fn test_commit() {
        let repo = TestRepo::new();
        repo.create_commit("test.txt", "hello", "feat: add test file");

        assert_eq!(repo.last_commit_message(), "feat: add test file");
    }

    #[test]
    fn test_branch_operations() {
        let repo = TestRepo::new();
        repo.create_commit("initial.txt", "content", "feat: initial commit");

        repo.create_branch("feature");
        repo.checkout("feature");

        assert_eq!(repo.current_branch(), "feature");
    }

    #[test]
    fn test_remote_operations() {
        let repo = TestRepo::new();
        repo.set_remote("origin", "https://github.com/test/repo.git");

        let output = repo
            .create_git_command(&["remote", "get-url", "origin"])
            .output()
            .unwrap();

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(url, "https://github.com/test/repo.git");
    }
}
