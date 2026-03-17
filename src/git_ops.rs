//! Git operations abstraction for testability.

use std::path::PathBuf;

use crate::generate::GenerateError;

/// Information about a git commit.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Full SHA of the commit.
    pub id: String,
    /// First line of the commit message (subject).
    pub message: String,
    /// Author name.
    pub author: String,
    /// Unix timestamp of the commit.
    pub timestamp: i64,
}

/// Information about a git tag.
#[derive(Debug, Clone)]
pub struct TagInfo {
    /// Tag name (without the `refs/tags/` prefix).
    pub name: String,
    /// Annotation message for annotated tags; `None` for lightweight tags.
    pub message: Option<String>,
    /// SHA of the object the tag points to.
    pub target: String,
}

/// Trait for git operations, allows mocking in tests.
pub trait GitOperations {
    fn get_current_branch(&self) -> Result<String, GenerateError>;
    fn get_recent_commit_messages(&self, count: usize) -> Result<Vec<String>, GenerateError>;
    fn get_repository_name(&self) -> Result<String, GenerateError>;
    fn is_merge_in_progress(&self) -> bool;
    fn is_rebase_in_progress(&self) -> bool;
    fn get_staged_diff(&self) -> Result<String, GenerateError>;
    fn get_staged_files_by_status(&self, status: &str) -> Result<Vec<String>, GenerateError>;

    /// Return commits reachable from `to` but not from `from`.
    ///
    /// Pass an empty string for `from` to get all commits up to `to`.
    fn get_commits_in_range(
        &self,
        _from: &str,
        _to: &str,
    ) -> Result<Vec<CommitInfo>, GenerateError> {
        unimplemented!("get_commits_in_range not yet implemented for this backend")
    }

    /// Return all tags in the repository.
    fn get_tags(&self) -> Result<Vec<TagInfo>, GenerateError> {
        unimplemented!("get_tags not yet implemented for this backend")
    }

    /// Create an annotated tag at HEAD.
    ///
    /// Set `sign` to `true` to GPG-sign the tag (requires a configured signing
    /// key).
    fn create_tag(&self, _name: &str, _message: &str, _sign: bool) -> Result<(), GenerateError> {
        unimplemented!("create_tag not yet implemented for this backend")
    }

    /// Create a commit from the current index with the given message.
    fn create_commit(&self, _message: &str) -> Result<(), GenerateError> {
        unimplemented!("create_commit not yet implemented for this backend")
    }

    /// Return the path to the repository's hooks directory.
    fn get_hook_path(&self) -> Result<PathBuf, GenerateError> {
        unimplemented!("get_hook_path not yet implemented for this backend")
    }

    /// Return the root of the working tree.
    fn get_repo_root(&self) -> Result<PathBuf, GenerateError> {
        unimplemented!("get_repo_root not yet implemented for this backend")
    }
}

/// Git operations using libgit2, providing a shell-free implementation.
pub struct Git2Ops {
    repo: git2::Repository,
}

impl Git2Ops {
    /// Open a git repository by discovering it from the current directory.
    pub fn open() -> Result<Self, GenerateError> {
        let repo = git2::Repository::discover(".").map_err(|e| {
            GenerateError::GitContext(format!("failed to open git repository: {}", e))
        })?;
        Ok(Self { repo })
    }

    /// Open a git repository at a specific path.
    pub fn open_at(path: &std::path::Path) -> Result<Self, GenerateError> {
        let repo = git2::Repository::discover(path).map_err(|e| {
            GenerateError::GitContext(format!("failed to open git repository: {}", e))
        })?;
        Ok(Self { repo })
    }
}

impl GitOperations for Git2Ops {
    fn get_current_branch(&self) -> Result<String, GenerateError> {
        let head = self
            .repo
            .head()
            .map_err(|e| GenerateError::GitContext(format!("failed to get HEAD: {}", e)))?;

        if head.is_branch() {
            let name = head.shorthand().ok_or_else(|| {
                GenerateError::GitContext("branch name is not valid UTF-8".to_string())
            })?;
            Ok(name.to_string())
        } else {
            Err(GenerateError::GitContext(
                "not on any branch (detached HEAD)".to_string(),
            ))
        }
    }

    fn get_recent_commit_messages(&self, count: usize) -> Result<Vec<String>, GenerateError> {
        // return empty list if repo has no commits
        if self.repo.head().is_err() {
            return Ok(Vec::new());
        }

        let mut revwalk = self
            .repo
            .revwalk()
            .map_err(|e| GenerateError::GitContext(format!("failed to create revwalk: {}", e)))?;

        revwalk.push_head().map_err(|e| {
            GenerateError::GitContext(format!("failed to push HEAD to revwalk: {}", e))
        })?;

        revwalk
            .set_sorting(git2::Sort::TIME)
            .map_err(|e| GenerateError::GitContext(format!("failed to set sort order: {}", e)))?;

        let commits = revwalk
            .take(count)
            .filter_map(|oid_result| {
                let oid = oid_result.ok()?;
                let commit = self.repo.find_commit(oid).ok()?;
                Some(commit.summary()?.to_string())
            })
            .collect();

        Ok(commits)
    }

    fn get_repository_name(&self) -> Result<String, GenerateError> {
        let config = self
            .repo
            .config()
            .map_err(|e| GenerateError::GitContext(format!("failed to get git config: {}", e)))?;

        let url = config
            .get_string("remote.origin.url")
            .map_err(|_| GenerateError::GitContext("no remote origin found".to_string()))?;

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
        matches!(self.repo.state(), git2::RepositoryState::Merge)
    }

    fn is_rebase_in_progress(&self) -> bool {
        matches!(
            self.repo.state(),
            git2::RepositoryState::Rebase
                | git2::RepositoryState::RebaseInteractive
                | git2::RepositoryState::RebaseMerge
        )
    }

    fn get_staged_diff(&self) -> Result<String, GenerateError> {
        let index = self
            .repo
            .index()
            .map_err(|e| GenerateError::StagedChanges(format!("failed to get index: {}", e)))?;

        // returns None for empty repositories with no commits yet
        let head_tree = self.repo.head().and_then(|h| h.peel_to_tree()).ok();

        let diff = self
            .repo
            .diff_tree_to_index(head_tree.as_ref(), Some(&index), None)
            .map_err(|e| {
                GenerateError::StagedChanges(format!("failed to compute staged diff: {}", e))
            })?;

        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            // add prefix character for context, addition, and deletion lines
            match line.origin() {
                '+' | '-' | ' ' => diff_text.push(line.origin()),
                _ => {}
            }
            if let Ok(s) = std::str::from_utf8(line.content()) {
                diff_text.push_str(s);
            }
            true
        })
        .map_err(|e| {
            GenerateError::StagedChanges(format!("failed to format diff output: {}", e))
        })?;

        Ok(diff_text)
    }

    fn get_staged_files_by_status(&self, status: &str) -> Result<Vec<String>, GenerateError> {
        let index = self
            .repo
            .index()
            .map_err(|e| GenerateError::StagedChanges(format!("failed to get index: {}", e)))?;

        let head_tree = self.repo.head().and_then(|h| h.peel_to_tree()).ok();

        let diff = self
            .repo
            .diff_tree_to_index(head_tree.as_ref(), Some(&index), None)
            .map_err(|e| {
                GenerateError::StagedChanges(format!("failed to compute staged diff: {}", e))
            })?;

        let files: Vec<String> = diff
            .deltas()
            .filter_map(|delta| {
                let delta_status = match delta.status() {
                    git2::Delta::Added => "A",
                    git2::Delta::Modified => "M",
                    git2::Delta::Deleted => "D",
                    git2::Delta::Renamed => "R",
                    git2::Delta::Copied => "C",
                    _ => return None,
                };

                if delta_status != status {
                    return None;
                }

                // use old file path for deletions, new file path otherwise
                let path = if delta.status() == git2::Delta::Deleted {
                    delta.old_file().path()
                } else {
                    delta.new_file().path()
                };

                path.and_then(|p| p.to_str()).map(|s| s.to_string())
            })
            .collect();

        Ok(files)
    }

    fn get_commits_in_range(&self, from: &str, to: &str) -> Result<Vec<CommitInfo>, GenerateError> {
        let mut revwalk = self
            .repo
            .revwalk()
            .map_err(|e| GenerateError::GitContext(format!("failed to create revwalk: {}", e)))?;

        let to_obj = self
            .repo
            .revparse_single(to)
            .map_err(|e| GenerateError::GitContext(format!("failed to resolve '{}': {}", to, e)))?;

        revwalk.push(to_obj.id()).map_err(|e| {
            GenerateError::GitContext(format!("failed to push revision to walk: {}", e))
        })?;

        if !from.is_empty() {
            let from_obj = self.repo.revparse_single(from).map_err(|e| {
                GenerateError::GitContext(format!("failed to resolve '{}': {}", from, e))
            })?;
            revwalk.hide(from_obj.id()).map_err(|e| {
                GenerateError::GitContext(format!("failed to hide revision: {}", e))
            })?;
        }

        revwalk
            .set_sorting(git2::Sort::TIME)
            .map_err(|e| GenerateError::GitContext(format!("failed to set sort order: {}", e)))?;

        let commits = revwalk
            .filter_map(|oid_result| {
                let oid = oid_result.ok()?;
                let commit = self.repo.find_commit(oid).ok()?;
                let message = commit.summary().unwrap_or("").to_string();
                let author = commit.author().name().unwrap_or("").to_string();
                let timestamp = commit.time().seconds();
                Some(CommitInfo {
                    id: oid.to_string(),
                    message,
                    author,
                    timestamp,
                })
            })
            .collect();

        Ok(commits)
    }

    fn get_tags(&self) -> Result<Vec<TagInfo>, GenerateError> {
        let tag_names = self
            .repo
            .tag_names(None)
            .map_err(|e| GenerateError::GitContext(format!("failed to list tags: {}", e)))?;

        let mut tags = Vec::new();
        for opt_name in tag_names.iter() {
            let name = match opt_name {
                Some(n) => n,
                // skip tags with non-UTF-8 names
                None => continue,
            };

            let refname = format!("refs/tags/{}", name);
            let obj = match self.repo.revparse_single(&refname) {
                Ok(o) => o,
                Err(_) => continue,
            };

            let (message, target) = match obj.into_tag() {
                Ok(tag) => {
                    // annotated tag: extract message and the object it points to
                    let msg = tag.message().map(|s| s.to_string());
                    let target_id = tag.target_id().to_string();
                    (msg, target_id)
                }
                Err(obj) => {
                    // lightweight tag: the ref points directly to the target object
                    (None, obj.id().to_string())
                }
            };

            tags.push(TagInfo {
                name: name.to_string(),
                message,
                target,
            });
        }

        Ok(tags)
    }

    fn create_tag(&self, name: &str, message: &str, sign: bool) -> Result<(), GenerateError> {
        if sign {
            // libgit2 signing requires a custom callback; fall back to the git CLI which
            // uses the user's configured GPG key transparently
            let workdir = self.repo.workdir().ok_or_else(|| {
                GenerateError::GitContext(
                    "repository has no working directory (bare repo)".to_string(),
                )
            })?;

            let output = std::process::Command::new("git")
                .args(["tag", "-s", "-a", name, "-m", message])
                .current_dir(workdir)
                .output()
                .map_err(|e| {
                    GenerateError::GitCommand(format!("failed to invoke git for signing: {}", e))
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(GenerateError::GitCommand(format!(
                    "failed to create signed tag '{}': {}",
                    name,
                    stderr.trim()
                )));
            }

            return Ok(());
        }

        let head_commit = self
            .repo
            .head()
            .and_then(|h| h.peel_to_commit())
            .map_err(|e| {
                GenerateError::GitContext(format!("failed to resolve HEAD commit: {}", e))
            })?;

        let sig = self
            .repo
            .signature()
            .map_err(|e| GenerateError::GitContext(format!("failed to build signature: {}", e)))?;

        self.repo
            .tag(name, head_commit.as_object(), &sig, message, false)
            .map_err(|e| {
                GenerateError::GitCommand(format!("failed to create tag '{}': {}", name, e))
            })?;

        Ok(())
    }

    fn create_commit(&self, message: &str) -> Result<(), GenerateError> {
        let sig = self
            .repo
            .signature()
            .map_err(|e| GenerateError::GitContext(format!("failed to build signature: {}", e)))?;

        let mut index = self
            .repo
            .index()
            .map_err(|e| GenerateError::GitContext(format!("failed to get index: {}", e)))?;

        let tree_id = index
            .write_tree()
            .map_err(|e| GenerateError::GitContext(format!("failed to write tree: {}", e)))?;

        let tree = self
            .repo
            .find_tree(tree_id)
            .map_err(|e| GenerateError::GitContext(format!("failed to find tree: {}", e)))?;

        let parents = match self.repo.head().and_then(|h| h.peel_to_commit()) {
            Ok(parent) => vec![parent],
            // no parent for the initial commit
            Err(_) => vec![],
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

        self.repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .map_err(|e| GenerateError::GitCommand(format!("failed to create commit: {}", e)))?;

        Ok(())
    }

    fn get_hook_path(&self) -> Result<PathBuf, GenerateError> {
        // repo.path() returns the .git directory
        Ok(self.repo.path().join("hooks"))
    }

    fn get_repo_root(&self) -> Result<PathBuf, GenerateError> {
        self.repo.workdir().map(|p| p.to_path_buf()).ok_or_else(|| {
            GenerateError::GitContext("repository has no working directory (bare repo)".to_string())
        })
    }
}

/// Mock git operations for testing.
#[cfg(test)]
pub struct MockGitOps {
    pub current_branch: Result<String, GenerateError>,
    pub recent_commits: Result<Vec<String>, GenerateError>,
    pub repository_name: Result<String, GenerateError>,
    pub is_merge: bool,
    pub is_rebase: bool,
    pub staged_diff: Result<String, GenerateError>,
    pub staged_files: std::collections::HashMap<String, Vec<String>>,
    pub commits_in_range: Result<Vec<CommitInfo>, GenerateError>,
    pub tags: Result<Vec<TagInfo>, GenerateError>,
    pub hook_path: Result<PathBuf, GenerateError>,
    pub repo_root: Result<PathBuf, GenerateError>,
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
            commits_in_range: Ok(Vec::new()),
            tags: Ok(Vec::new()),
            hook_path: Ok(PathBuf::from(".git/hooks")),
            repo_root: Ok(PathBuf::from(".")),
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
                    message: "feat: add feature".to_string(),
                    author: "Alice".to_string(),
                    timestamp: 1000,
                },
                CommitInfo {
                    id: "def456".to_string(),
                    message: "fix: fix bug".to_string(),
                    author: "Bob".to_string(),
                    timestamp: 900,
                },
            ]),
            ..Default::default()
        };

        let commits = mock.get_commits_in_range("v0.9.0", "HEAD").unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "feat: add feature");
        assert_eq!(commits[1].author, "Bob");
    }

    // ── Git2Ops unit tests ────────────────────────────────────────────────────

    /// Create a minimal git2 repo in a temp dir and return the path.
    ///
    /// The repo has an initial empty commit so HEAD is valid.
    fn make_temp_repo() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();

        let repo = git2::Repository::init(&path).unwrap();

        // configure identity so commit creation works
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();
        drop(cfg);

        // create a root commit so HEAD is valid
        let sig = repo
            .signature()
            .or_else(|_| git2::Signature::now("Test", "test@example.com"))
            .unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "chore: initial commit",
            &tree,
            &[],
        )
        .unwrap();

        (dir, path)
    }

    #[test]
    fn test_git2ops_open_at_valid_repo() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path);
        assert!(ops.is_ok(), "expected Ok but open_at failed");
        drop(dir);
    }

    #[test]
    fn test_git2ops_open_at_invalid_path() {
        let tmp = tempfile::tempdir().unwrap();
        // point to a dir that is NOT a git repo
        let non_repo = tmp.path().join("not_a_git_repo");
        std::fs::create_dir_all(&non_repo).unwrap();
        let result = Git2Ops::open_at(&non_repo);
        assert!(result.is_err());
    }

    #[test]
    fn test_git2ops_get_current_branch_on_main() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        let branch = ops.get_current_branch();
        // may be "main" or "master" depending on git version defaults
        assert!(branch.is_ok(), "expected Ok, got {:?}", branch);
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_recent_commit_messages_returns_list() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        let msgs = ops.get_recent_commit_messages(10).unwrap();
        assert!(!msgs.is_empty());
        assert_eq!(msgs[0], "chore: initial commit");
        drop(dir);
    }

    #[test]
    fn test_git2ops_is_merge_not_in_progress() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        assert!(!ops.is_merge_in_progress());
        drop(dir);
    }

    #[test]
    fn test_git2ops_is_rebase_not_in_progress() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        assert!(!ops.is_rebase_in_progress());
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_staged_diff_empty_index() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        // nothing staged → empty diff
        let diff = ops.get_staged_diff().unwrap();
        assert!(diff.is_empty());
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_staged_files_by_status_empty() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        let added = ops.get_staged_files_by_status("A").unwrap();
        assert!(added.is_empty());
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_repo_root_is_correct() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        let root = ops.get_repo_root().unwrap();
        // the root should be (or contain) our temp path
        assert!(root.exists());
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_hook_path_exists_under_dotgit() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        let hook_path = ops.get_hook_path().unwrap();
        // hooks dir is always inside .git/
        let hook_str = hook_path.to_string_lossy();
        assert!(
            hook_str.contains("hooks"),
            "unexpected hook path: {}",
            hook_str
        );
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_tags_empty_repo() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        let tags = ops.get_tags().unwrap();
        assert!(tags.is_empty());
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_staged_diff_with_staged_file() {
        let (dir, path) = make_temp_repo();
        // write a new file and stage it
        let file_path = path.join("test.txt");
        std::fs::write(&file_path, "hello world\n").unwrap();
        let repo = git2::Repository::open(&path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let ops = Git2Ops::open_at(&path).unwrap();
        let diff = ops.get_staged_diff().unwrap();
        assert!(!diff.is_empty());
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_staged_files_by_status_added() {
        let (dir, path) = make_temp_repo();
        let file_path = path.join("new_file.rs");
        std::fs::write(&file_path, "fn main() {}\n").unwrap();
        let repo = git2::Repository::open(&path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("new_file.rs")).unwrap();
        index.write().unwrap();

        let ops = Git2Ops::open_at(&path).unwrap();
        let added = ops.get_staged_files_by_status("A").unwrap();
        assert!(added.contains(&"new_file.rs".to_string()));
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_commits_in_range_all() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        // empty string for `from` returns all commits up to HEAD
        let commits = ops.get_commits_in_range("", "HEAD").unwrap();
        assert!(!commits.is_empty());
        assert_eq!(commits[0].summary, "chore: initial commit");
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_commits_in_range_with_since_tag() {
        let (dir, path) = make_temp_repo();
        // add a tag on the initial commit
        let repo = git2::Repository::open(&path).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.tag_lightweight("v0.0.0", head.as_object(), false)
            .unwrap();

        // add a second commit after the tag
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let new_file = path.join("second.txt");
        std::fs::write(&new_file, "second").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("second.txt")).unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "feat: second commit",
            &tree,
            &[&head],
        )
        .unwrap();

        let ops = Git2Ops::open_at(&path).unwrap();
        let commits = ops.get_commits_in_range("v0.0.0", "HEAD").unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "feat: second commit");
        drop(dir);
    }

    #[test]
    fn test_git2ops_create_commit_on_staged_file() {
        let (dir, path) = make_temp_repo();
        let file_path = path.join("commit_me.txt");
        std::fs::write(&file_path, "content").unwrap();
        let repo = git2::Repository::open(&path).unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("commit_me.txt"))
            .unwrap();
        index.write().unwrap();

        let ops = Git2Ops::open_at(&path).unwrap();
        let result = ops.create_commit("feat: add commit_me file");
        assert!(result.is_ok(), "create_commit failed: {:?}", result);
        // verify the commit was actually created
        let commits = ops.get_recent_commit_messages(1).unwrap();
        assert_eq!(commits[0], "feat: add commit_me file");
        drop(dir);
    }

    #[test]
    fn test_git2ops_create_tag_lightweight() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        let result = ops.create_tag("v1.0.0", "release v1.0.0", false);
        assert!(result.is_ok(), "create_tag failed: {:?}", result);
        let tags = ops.get_tags().unwrap();
        assert!(tags.iter().any(|t| t.name == "v1.0.0"));
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_tags_with_annotated_tag() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        ops.create_tag("v2.0.0", "release notes for v2", false)
            .unwrap();
        let tags = ops.get_tags().unwrap();
        let tag = tags.iter().find(|t| t.name == "v2.0.0").unwrap();
        assert_eq!(tag.name, "v2.0.0");
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_repository_name_no_remote() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        // no remote configured → should return an error
        let result = ops.get_repository_name();
        assert!(result.is_err());
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_staged_files_modified_status() {
        let (dir, path) = make_temp_repo();
        // create and commit a file first
        let repo = git2::Repository::open(&path).unwrap();
        let file_path = path.join("existing.txt");
        std::fs::write(&file_path, "original\n").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("existing.txt"))
            .unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "chore: add file",
            &tree,
            &[&parent],
        )
        .unwrap();

        // now modify the file and stage it
        std::fs::write(&file_path, "modified\n").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("existing.txt"))
            .unwrap();
        index.write().unwrap();

        let ops = Git2Ops::open_at(&path).unwrap();
        let modified = ops.get_staged_files_by_status("M").unwrap();
        assert!(modified.contains(&"existing.txt".to_string()));
        drop(dir);
    }

    #[test]
    fn test_git2ops_get_commits_in_range_invalid_to_returns_error() {
        let (dir, path) = make_temp_repo();
        let ops = Git2Ops::open_at(&path).unwrap();
        let result = ops.get_commits_in_range("", "nonexistent-ref");
        assert!(result.is_err());
        drop(dir);
    }
}
