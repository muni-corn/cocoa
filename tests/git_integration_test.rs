//! Integration tests for git operations.

mod helpers;

use cocoa::{
    generate::{analyze_staged_changes_with_git, extract_git_context_with_git},
    git_ops::{Git2Ops, GitOperations},
};
use helpers::git_repo::TestRepo;

#[test]
fn test_analyze_staged_changes_with_real_repo() {
    let repo = TestRepo::new();

    // create and stage some files
    repo.create_and_stage_file("main.rs", "fn main() {}");
    repo.create_and_stage_file("lib.rs", "pub fn hello() {}");

    let changes = analyze_staged_changes_with_git(&repo).unwrap();

    assert!(!changes.diff.is_empty());
    assert_eq!(changes.files_added.len(), 2);
}

#[test]
fn test_analyze_staged_changes_no_changes() {
    let repo = TestRepo::new();

    // create initial commit
    repo.create_commit("README.md", "# Test", "feat: initial commit");

    let result = analyze_staged_changes_with_git(&repo);

    assert!(result.is_err());
}

#[test]
fn test_extract_git_context_with_real_repo() {
    let repo = TestRepo::new();

    // create some commits
    repo.create_commit("file1.txt", "content1", "feat: add file1");
    repo.create_commit("file2.txt", "content2", "fix: add file2");
    repo.create_commit("file3.txt", "content3", "docs: add file3");

    let result = extract_git_context_with_git(&repo);

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

    let result = extract_git_context_with_git(&repo);

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

    let result = analyze_staged_changes_with_git(&repo);

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

    let result = extract_git_context_with_git(&repo);

    assert!(result.is_ok());
    let context = result.unwrap();

    assert_eq!(context.repository_name, Some("repo".to_string()));
}

// --- Git2Ops integration tests ---

#[test]
fn test_git2ops_open_at_valid_repo() {
    let repo = TestRepo::new();
    let git2_ops = Git2Ops::open_at(&repo.path);
    assert!(git2_ops.is_ok(), "Git2Ops should open a valid repository");
}

#[test]
fn test_git2ops_open_at_invalid_path() {
    let tmp = tempfile::tempdir().unwrap();
    let git2_ops = Git2Ops::open_at(tmp.path());
    assert!(
        git2_ops.is_err(),
        "Git2Ops should fail on a non-git directory"
    );
}

#[test]
fn test_git2ops_get_current_branch() {
    let repo = TestRepo::new();
    repo.create_commit("init.txt", "init", "feat: initial commit");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let branch = git2_ops.get_current_branch().unwrap();
    assert!(!branch.is_empty());
}

#[test]
fn test_git2ops_get_current_branch_feature() {
    let repo = TestRepo::new();
    repo.create_commit("init.txt", "init", "feat: initial commit");
    repo.create_branch("feature/my-feature");
    repo.checkout("feature/my-feature");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let branch = git2_ops.get_current_branch().unwrap();
    assert_eq!(branch, "feature/my-feature");
}

#[test]
fn test_git2ops_get_recent_commit_messages() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: first");
    repo.create_commit("b.txt", "b", "fix: second");
    repo.create_commit("c.txt", "c", "docs: third");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let messages = git2_ops.get_recent_commit_messages(3).unwrap();

    // check all three commits are present (order may vary when timestamps are
    // equal)
    assert_eq!(messages.len(), 3);
    assert!(messages.contains(&"feat: first".to_string()));
    assert!(messages.contains(&"fix: second".to_string()));
    assert!(messages.contains(&"docs: third".to_string()));
}

#[test]
fn test_git2ops_get_recent_commit_messages_empty_repo() {
    let repo = TestRepo::new();

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let messages = git2_ops.get_recent_commit_messages(5).unwrap();
    assert!(messages.is_empty());
}

#[test]
fn test_git2ops_get_repository_name() {
    let repo = TestRepo::new();
    repo.set_remote("origin", "https://github.com/example/my-project.git");
    repo.create_commit("init.txt", "init", "feat: initial");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let name = git2_ops.get_repository_name().unwrap();
    assert_eq!(name, "my-project");
}

#[test]
fn test_git2ops_get_staged_diff_with_changes() {
    let repo = TestRepo::new();
    repo.create_and_stage_file("hello.rs", "fn hello() {}");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let diff = git2_ops.get_staged_diff().unwrap();

    assert!(!diff.is_empty());
    assert!(diff.contains("hello"));
}

#[test]
fn test_git2ops_get_staged_diff_no_changes() {
    let repo = TestRepo::new();
    repo.create_commit("init.txt", "init", "feat: initial");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let diff = git2_ops.get_staged_diff().unwrap();

    assert!(diff.trim().is_empty());
}

#[test]
fn test_git2ops_get_staged_files_by_status() {
    let repo = TestRepo::new();
    repo.create_commit("existing.txt", "old content", "feat: initial");

    // stage an addition and a modification
    repo.create_and_stage_file("new.rs", "fn new() {}");
    repo.modify_file("existing.txt", "new content");
    repo.stage_file("existing.txt");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let added = git2_ops.get_staged_files_by_status("A").unwrap();
    let modified = git2_ops.get_staged_files_by_status("M").unwrap();

    assert_eq!(added.len(), 1);
    assert_eq!(added[0], "new.rs");
    assert_eq!(modified.len(), 1);
    assert_eq!(modified[0], "existing.txt");
}

#[test]
fn test_git2ops_get_tags_empty() {
    let repo = TestRepo::new();
    repo.create_commit("init.txt", "init", "feat: initial");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let tags = git2_ops.get_tags().unwrap();
    assert!(tags.is_empty());
}

#[test]
fn test_git2ops_get_tags_lightweight() {
    let repo = TestRepo::new();
    repo.create_commit("init.txt", "init", "feat: initial");
    repo.create_lightweight_tag("v0.0.1");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let tags = git2_ops.get_tags().unwrap();

    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "v0.0.1");
    assert!(
        tags[0].message.is_none(),
        "lightweight tags have no message"
    );
}

#[test]
fn test_git2ops_get_tags_annotated() {
    let repo = TestRepo::new();
    repo.create_commit("init.txt", "init", "feat: initial");
    repo.create_annotated_tag("v1.0.0", "release v1.0.0");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let tags = git2_ops.get_tags().unwrap();

    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "v1.0.0");
    assert!(
        tags[0].message.is_some(),
        "annotated tags should have a message"
    );
    assert!(tags[0].message.as_ref().unwrap().contains("release v1.0.0"));
}

#[test]
fn test_git2ops_get_commits_in_range() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: alpha");
    repo.create_lightweight_tag("v0.0.1");
    repo.create_commit("b.txt", "b", "fix: beta");
    repo.create_commit("c.txt", "c", "docs: gamma");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let commits = git2_ops.get_commits_in_range("v0.0.1", "HEAD").unwrap();

    // should include beta and gamma but not alpha (which is at the tag)
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].summary, "docs: gamma");
    assert_eq!(commits[1].summary, "fix: beta");
}

#[test]
fn test_git2ops_get_commits_in_range_all() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: alpha");
    repo.create_commit("b.txt", "b", "fix: beta");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    // empty from → include everything up to HEAD
    let commits = git2_ops.get_commits_in_range("", "HEAD").unwrap();

    assert_eq!(commits.len(), 2);
}

#[test]
fn test_git2ops_create_tag() {
    let repo = TestRepo::new();
    repo.create_commit("init.txt", "init", "feat: initial");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    git2_ops
        .create_tag("v2.0.0", "release v2.0.0", false)
        .unwrap();

    let tags = git2_ops.get_tags().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "v2.0.0");
    assert!(tags[0].message.is_some());
}

#[test]
fn test_git2ops_create_commit() {
    let repo = TestRepo::new();

    // stage a file manually
    repo.create_and_stage_file("hello.txt", "hello world");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    git2_ops.create_commit("feat: initial commit").unwrap();

    // verify the commit was created
    assert_eq!(repo.last_commit_message(), "feat: initial commit");
}

#[test]
fn test_git2ops_get_hook_path() {
    let repo = TestRepo::new();
    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let hook_path = git2_ops.get_hook_path().unwrap();

    assert!(hook_path.to_string_lossy().contains("hooks"));
    // the path should be inside the .git directory
    assert!(hook_path.to_string_lossy().contains(".git"));
}

#[test]
fn test_git2ops_get_repo_root() {
    let repo = TestRepo::new();
    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    let root = git2_ops.get_repo_root().unwrap();

    // the canonical paths should match
    let expected = repo.path.canonicalize().unwrap();
    let actual = root.canonicalize().unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_git2ops_is_merge_in_progress_false() {
    let repo = TestRepo::new();
    repo.create_commit("init.txt", "init", "feat: initial");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    assert!(!git2_ops.is_merge_in_progress());
}

#[test]
fn test_git2ops_is_rebase_in_progress_false() {
    let repo = TestRepo::new();
    repo.create_commit("init.txt", "init", "feat: initial");

    let git2_ops = Git2Ops::open_at(&repo.path).unwrap();
    assert!(!git2_ops.is_rebase_in_progress());
}
