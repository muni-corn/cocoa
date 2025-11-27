//! Test helper for creating temporary git repositories.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

/// Helper struct for creating and managing test git repositories.
pub struct TestRepo {
    /// Temporary directory containing the repo.
    pub dir: TempDir,
    /// Path to the repository.
    pub path: PathBuf,
}

impl TestRepo {
    /// Create a new test git repository with basic configuration.
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();

        // initialize git repository
        Command::new("git")
            .arg("init")
            .current_dir(&path)
            .output()
            .expect("failed to initialize git repository");

        // configure git user
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&path)
            .output()
            .expect("failed to configure git user name");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&path)
            .output()
            .expect("failed to configure git user email");

        Self { dir, path }
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
        let output = Command::new("git")
            .args(["add", path.as_ref().to_str().unwrap()])
            .current_dir(&self.path)
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
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.path)
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
        Command::new("git")
            .args(["branch", name])
            .current_dir(&self.path)
            .output()
            .expect("failed to create branch");
    }

    /// Checkout a branch.
    pub fn checkout(&self, branch: &str) {
        Command::new("git")
            .args(["checkout", branch])
            .current_dir(&self.path)
            .output()
            .expect("failed to checkout branch");
    }

    /// Get the current branch name.
    pub fn current_branch(&self) -> String {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&self.path)
            .output()
            .expect("failed to get current branch");

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Get the last commit message.
    pub fn last_commit_message(&self) -> String {
        let output = Command::new("git")
            .args(["log", "-1", "--format=%s"])
            .current_dir(&self.path)
            .output()
            .expect("failed to get last commit message");

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Set a remote URL.
    pub fn set_remote(&self, name: &str, url: &str) {
        Command::new("git")
            .args(["remote", "add", name, url])
            .current_dir(&self.path)
            .output()
            .expect("failed to set remote");
    }

    /// Get staged diff.
    pub fn get_staged_diff(&self) -> String {
        let output = Command::new("git")
            .args(["diff", "--cached"])
            .current_dir(&self.path)
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

        let output = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(&repo.path)
            .output()
            .unwrap();

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(url, "https://github.com/test/repo.git");
    }
}
