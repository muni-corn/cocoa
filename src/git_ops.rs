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
            // GPG signing via libgit2 requires a custom signing callback; defer to a future
            // phase
            return Err(GenerateError::GitCommand(
                "GPG-signed tags are not yet supported by Git2Ops".to_string(),
            ));
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
}
