//! integration tests for git operations

mod helpers;

use cocoa::generate::{analyze_staged_changes_with_git, extract_git_context_with_git};
use cocoa::git_ops::RealGitOps;
use helpers::git_repo::TestRepo;

#[test]
fn test_analyze_staged_changes_with_real_repo() {
    let repo = TestRepo::new();

    // create and stage some files
    repo.create_and_stage_file("main.rs", "fn main() {}");
    repo.create_and_stage_file("lib.rs", "pub fn hello() {}");

    let git_ops = RealGitOps;
    let result = analyze_staged_changes_with_git(&git_ops);

    assert!(result.is_ok());
    let changes = result.unwrap();
    assert!(!changes.diff.is_empty());
    assert_eq!(changes.files_added.len(), 2);
}

#[test]
fn test_analyze_staged_changes_no_changes() {
    let repo = TestRepo::new();

    // create initial commit
    repo.create_commit("README.md", "# Test", "feat: initial commit");

    let git_ops = RealGitOps;
    let result = analyze_staged_changes_with_git(&git_ops);

    assert!(result.is_err());
}

#[test]
fn test_extract_git_context_with_real_repo() {
    let repo = TestRepo::new();

    // create some commits
    repo.create_commit("file1.txt", "content1", "feat: add file1");
    repo.create_commit("file2.txt", "content2", "fix: add file2");
    repo.create_commit("file3.txt", "content3", "docs: add file3");

    let git_ops = RealGitOps;
    let result = extract_git_context_with_git(&git_ops);

    assert!(result.is_ok());
    let context = result.unwrap();

    // default branch should be set
    assert!(context.branch_name.is_some());

    // should have recent commits
    assert!(!context.recent_commits.is_empty());
    assert!(context.recent_commits.len() <= 5);
}

#[test]
fn test_extract_context_with_branch_name() {
    let repo = TestRepo::new();

    // create initial commit on main
    repo.create_commit("initial.txt", "init", "feat: initial");

    // create and checkout a feature branch
    repo.create_branch("feature/test");
    repo.checkout("feature/test");

    let git_ops = RealGitOps;
    let result = extract_git_context_with_git(&git_ops);

    assert!(result.is_ok());
    let context = result.unwrap();

    assert_eq!(context.branch_name, Some("feature/test".to_string()));
}

#[test]
fn test_analyze_mixed_file_changes() {
    let repo = TestRepo::new();

    // create initial commit
    repo.create_commit("existing.txt", "old content", "feat: initial");

    // modify existing file
    repo.modify_file("existing.txt", "new content");
    repo.stage_file("existing.txt");

    // add new file
    repo.create_and_stage_file("new.txt", "new file");

    let git_ops = RealGitOps;
    let result = analyze_staged_changes_with_git(&git_ops);

    assert!(result.is_ok());
    let changes = result.unwrap();

    assert_eq!(changes.files_added.len(), 1);
    assert_eq!(changes.files_modified.len(), 1);
    assert_eq!(changes.files_deleted.len(), 0);
}

#[test]
fn test_git_context_with_repository_url() {
    let repo = TestRepo::new();

    // set a remote URL
    repo.set_remote("origin", "https://github.com/test/repo.git");

    // create a commit
    repo.create_commit("file.txt", "content", "feat: initial");

    let git_ops = RealGitOps;
    let result = extract_git_context_with_git(&git_ops);

    assert!(result.is_ok());
    let context = result.unwrap();

    assert_eq!(context.repository_name, Some("repo".to_string()));
}
