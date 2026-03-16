# cocoa: Remaining Implementation Plan

## Phase 0: Cleanup and Debt Reduction

Small fixes to clean up the codebase before building new features.

**Commit:** `chore(lint): remove stale comment about regex fallback`

### 0.1 Use `regex` crate in `check_custom_patterns()`

The `regex` crate is already in `Cargo.toml` but unused. Replace `String::contains()` with proper
`Regex` matching.

**Commit:** `fix(lint): use regex crate for custom pattern matching`

### 0.2 Wire up `--verbose` global flag

Currently parsed by clap but never read. Thread it through to command handlers.

**Commit:** `feat(cli): wire up --verbose flag for detailed output`

### 0.3 Wire up `--no-color` global flag

Use `console::set_colors_enabled(false)` when `--no-color` is passed.

**Commit:** `feat(cli): wire up --no-color flag to disable terminal colors`

### 0.4 Add `--dry-run` global flag

Add to `Cli` struct, thread through to all command handlers. No-op for now but available for future
commands.

**Commit:** `feat(cli): add global --dry-run flag for non-destructive operations`

---

## Phase 1: git2 Migration

Migrate from `std::process::Command` to `git2` (libgit2) for robustness and no shell dependency.

### 1.1 Create `git2`-based `GitOperations` implementation

Add a `Git2Ops` struct implementing the existing `GitOperations` trait using the `git2` crate. Keep
`RealGitOps` for now as fallback.

**Commit:** `refactor(git_ops): add Git2Ops implementation using libgit2`

### 1.2 Add new trait methods needed for future features

Extend `GitOperations` with methods we'll need later:

- `get_commits_in_range(from, to) -> Vec<CommitInfo>`
- `get_tags() -> Vec<TagInfo>`
- `create_tag(name, message, sign)`
- `create_commit(message)`
- `get_hook_path() -> PathBuf`
- `get_repo_root() -> PathBuf`

**Commit:** `feat(git_ops): extend GitOperations trait with tag, range, and hook methods`

### 1.3 Implement new trait methods in `Git2Ops`

**Commit:** `feat(git_ops): implement extended methods in Git2Ops`

### 1.4 Update `MockGitOps` with new trait methods

**Commit:** `test(git_ops): update MockGitOps for extended trait methods`

### 1.5 Switch `RealGitOps` usages to `Git2Ops`

Replace all `RealGitOps` references in `generate.rs` and `main.rs`.

**Commit:** `refactor: switch from RealGitOps to Git2Ops across codebase`

### 1.6 Remove `RealGitOps` and the `std::process::Command` approach

**Commit:** `refactor(git_ops): remove RealGitOps in favor of Git2Ops`

### 1.7 Add integration tests for `Git2Ops`

Update `tests/git_integration_test.rs` to test the new implementation.

**Commit:** `test(git_ops): add integration tests for Git2Ops`

---

## Phase 2: Cascading Configuration

Implement proper config cascading per spec section 3.1.

### 2.1 Add config file discovery logic

Implement `Config::discover()` that searches: `.cocoa.toml` -> `$XDG_CONFIG_HOME/cocoa/cocoa.toml`
-> `~/.config/cocoa/cocoa.toml` -> `/etc/cocoa/cocoa.toml`.

**Commit:** `feat(config): add cascading config file discovery`

### 2.2 Add config merging logic

Implement deep-merge so repo config overrides user config overrides system config.

**Commit:** `feat(config): implement config merging for cascading fallback`

### 2.3 Add `ChangelogConfig` to configuration schema

Add the `[changelog]` and `[changelog.sections]` config sections from the spec.

**Commit:** `feat(config): add changelog configuration schema`

### 2.4 Add `VersionConfig` to configuration schema

Add the `[version]` config section: strategy, tag_prefix, sign_tags, commit_version_files.

**Commit:** `feat(config): add version management configuration schema`

### 2.5 Update `Config::load()` to use discovery and merging

**Commit:** `refactor(config): integrate discovery and merging into Config::load`

### 2.6 Add tests for cascading configuration

**Commit:** `test(config): add tests for cascading config discovery and merging`

---

## Phase 3: `cocoa init`

Interactive configuration file generation (spec section 8.1).

### 3.1 Add `init` module scaffold

Create `src/init.rs` with the `init()` function signature and error types.

**Commit:** `feat(init): add init module scaffold`

### 3.2 Implement interactive prompts for commit config

Use `dialoguer` for: type selection (multi-select from defaults), scope input, rule thresholds.

**Commit:** `feat(init): implement interactive commit configuration prompts`

### 3.3 Implement AI provider configuration prompts

Prompt for provider, model, API key source (env var name or file path).

**Commit:** `feat(init): implement interactive AI configuration prompts`

### 3.4 Implement TOML file generation and writing

Serialize the built `Config` to TOML, write to `.cocoa.toml` with `--dry-run` support.

**Commit:** `feat(init): generate and write .cocoa.toml config file`

### 3.5 Wire `cocoa init` into `main.rs`

**Commit:** `feat(init): wire init command into CLI handler`

### 3.6 Add e2e tests for `cocoa init`

**Commit:** `test(init): add e2e tests for init command`

---

## Phase 4: Lint Improvements

### 4.1 Implement git range linting

Parse `HEAD~5..HEAD` syntax, iterate commits in range, lint each message.

**Commit:** `feat(lint): implement git range linting`

### 4.2 Support linting from file path

Detect if input is a file path (e.g., `.git/COMMIT_EDITMSG`) and read contents.

**Commit:** `feat(lint): support reading commit message from file path`

### 4.3 Add `--dry-run` awareness to lint

In dry-run mode, lint but don't exit with error codes.

**Commit:** `feat(lint): add dry-run mode support`

### 4.4 Add tests for range linting

**Commit:** `test(lint): add tests for git range and file path linting`

---

## Phase 5: `cocoa hook` / `cocoa unhook`

Git hook integration (spec section 5.2).

### 5.1 Add `Hook` and `Unhook` CLI variants

Add the missing command variants to `Commands` enum.

**Commit:** `feat(cli): add hook and unhook command variants`

### 5.2 Create `hook` module

Create `src/hook.rs` with types and functions for hook management.

**Commit:** `feat(hook): add hook module scaffold`

### 5.3 Implement `commit-msg` hook installation

Write a shell script to `.git/hooks/commit-msg` that invokes `cocoa lint --stdin`. Must be
idempotent.

**Commit:** `feat(hook): implement commit-msg hook installation`

### 5.4 Implement hook uninstallation

Remove the cocoa-managed hook, restore backup if one existed.

**Commit:** `feat(hook): implement hook uninstallation with backup restore`

### 5.5 Wire hook/unhook into `main.rs`

With `--dry-run` support: show what would be written/removed.

**Commit:** `feat(hook): wire hook and unhook commands into CLI`

### 5.6 Add tests for hook management

**Commit:** `test(hook): add integration tests for hook install and uninstall`

---

## Phase 6: `cocoa commit` (Interactive Mode)

Interactive commit creation (spec section 4.2).

### 6.1 Create `interactive` module scaffold

Create `src/interactive.rs` with function signatures.

**Commit:** `feat(interactive): add interactive commit module scaffold`

### 6.2 Implement type selection

Use `dialoguer::Select` with configured commit types.

**Commit:** `feat(interactive): implement commit type selection prompt`

### 6.3 Implement scope input with autocomplete

Use `dialoguer::Input` with `FuzzySelect` or completion from configured scopes.

**Commit:** `feat(interactive): implement scope input with autocomplete`

### 6.4 Implement subject line composition

Input with real-time character counting, warn/deny thresholds displayed.

**Commit:** `feat(interactive): implement subject composition with char counting`

### 6.5 Implement optional body text entry

Multi-line text editor input for commit body.

**Commit:** `feat(interactive): implement optional body text entry`

### 6.6 Implement breaking change annotation

Yes/no prompt, then BREAKING CHANGE footer input.

**Commit:** `feat(interactive): implement breaking change annotation flow`

### 6.7 Implement issue reference linking

Optional footer for issue references (e.g., `Closes #123`).

**Commit:** `feat(interactive): implement issue reference linking`

### 6.8 Assemble message and execute commit

Build the full conventional commit message from parts, validate with linter, commit with `--dry-run`
support.

**Commit:** `feat(interactive): assemble message, validate, and commit`

### 6.9 Wire `cocoa commit` into `main.rs`

**Commit:** `feat(interactive): wire commit command into CLI handler`

### 6.10 Add tests for interactive commit flow

**Commit:** `test(interactive): add tests for interactive commit creation`

---

## Phase 7: Changelog Generation

Spec section 6.

### 7.1 Create `changelog` module scaffold

Create `src/changelog.rs` (or `src/changelog/mod.rs` with submodules).

**Commit:** `feat(changelog): add changelog module scaffold`

### 7.2 Define `ChangelogEntry` and `ChangelogVersion` types

Structured types for parsed commits grouped by version and type.

**Commit:** `feat(changelog): define entry and version data types`

### 7.3 Implement git history parsing

Walk commits between tags/ranges, parse each with `CommitMessage::parse()`, group by type.

**Commit:** `feat(changelog): implement git history parsing and grouping`

### 7.4 Implement Markdown renderer

Generate Markdown changelog from grouped entries, with breaking changes prominent.

**Commit:** `feat(changelog): implement markdown output renderer`

### 7.5 Implement JSON output

Serialize changelog structure to JSON.

**Commit:** `feat(changelog): implement JSON output format`

### 7.6 Implement HTML output

**Commit:** `feat(changelog): implement HTML output format`

### 7.7 Implement reStructuredText output

**Commit:** `feat(changelog): implement reStructuredText output format`

### 7.8 Implement AsciiDoc output

**Commit:** `feat(changelog): implement AsciiDoc output format`

### 7.9 Add template support

Allow custom Handlebars/Tera templates for changelog rendering.

**Commit:** `feat(changelog): add custom template support`

### 7.10 Wire `cocoa changelog` into `main.rs`

With `--dry-run` support (print but don't write file).

**Commit:** `feat(changelog): wire changelog command into CLI handler`

### 7.11 Ensure deterministic output

Same input always produces same output (sort commits deterministically).

**Commit:** `fix(changelog): ensure deterministic output ordering`

### 7.12 Add tests for changelog generation

**Commit:** `test(changelog): add unit and integration tests`

---

## Phase 8: Version Management

Spec section 7.

### 8.1 Create `version` module scaffold

Create `src/version/mod.rs` with submodules for semver and calver.

**Commit:** `feat(version): add version module scaffold`

### 8.2 Implement semantic versioning engine

Parse and bump semver versions: major, minor, patch, pre-release, build metadata.

**Commit:** `feat(version): implement semantic versioning engine`

### 8.3 Implement calendar versioning engine

Support CalVer with configurable format strings (YYYY.MM.DD, etc.).

**Commit:** `feat(version): implement calendar versioning engine`

### 8.4 Implement version detection from git tags

Scan existing tags, parse versions, determine latest.

**Commit:** `feat(version): implement version detection from git tags`

### 8.5 Implement automatic bump type detection

Analyze commits since last tag: breaking -> major, feat -> minor, fix -> patch.

**Commit:** `feat(version): implement automatic bump type detection from commits`

### 8.6 Implement version file updates

Search configured files for old version string, replace with new. Must be atomic (all succeed or all
fail).

**Commit:** `feat(version): implement atomic version file updates`

### 8.7 Add tests for version management

**Commit:** `test(version): add tests for semver, calver, detection, and file updates`

---

## Phase 9: `cocoa bump`

### 9.1 Wire `cocoa bump` into `main.rs`

Accept bump_type arg (major|minor|patch|auto), use version module. `--dry-run` shows what would
change.

**Commit:** `feat(bump): wire bump command into CLI handler`

### 9.2 Add e2e tests for `cocoa bump`

**Commit:** `test(bump): add e2e tests for bump command`

---

## Phase 10: `cocoa tag`

Spec section 7.4.

### 10.1 Implement annotated tag creation

Create git tags with the changelog for that version as the message.

**Commit:** `feat(tag): implement annotated git tag creation`

### 10.2 Implement tag uniqueness verification

Check tag doesn't already exist before creating.

**Commit:** `feat(tag): implement tag uniqueness verification`

### 10.3 Implement GPG signing support

If `version.sign_tags = true`, sign the tag with GPG.

**Commit:** `feat(tag): implement GPG signing for tags`

### 10.4 Wire `cocoa tag` into `main.rs`

With `--dry-run` support.

**Commit:** `feat(tag): wire tag command into CLI handler`

### 10.5 Add tests for tagging

**Commit:** `test(tag): add integration tests for tag creation and signing`

---

## Phase 11: `cocoa release`

Full orchestrated release workflow.

### 11.1 Create `release` module

Orchestrate: bump version -> update files -> generate changelog -> commit -> tag.

**Commit:** `feat(release): add release module with orchestration logic`

### 11.2 Wire `cocoa release` into `main.rs`

With `--dry-run` support (shows the full plan without executing).

**Commit:** `feat(release): wire release command into CLI handler`

### 11.3 Add e2e tests for full release workflow

**Commit:** `test(release): add e2e tests for full release workflow`

---

## Phase 12: Security

Spec section 9.

### 12.1 Add sensitive content detection for diffs

Scan diffs for patterns that look like API keys, passwords, tokens. Warn the user.

**Commit:** `feat(generate): add sensitive content warning for staged diffs`

### 12.2 Audit logging to ensure no secrets leak

Review all error messages and log output paths to ensure API keys are never exposed.

**Commit:** `fix: audit and sanitize error messages to prevent secret leakage`

---

## Phase 13: Localization / i18n

Spec section 12.

### 13.1 Add i18n infrastructure

Add a string table / message catalog system. Could use `fluent` or `rust-i18n` crate.

**Commit:** `feat: add i18n infrastructure with message catalog`

### 13.2 Extract all user-facing strings to message catalog

**Commit:** `refactor: extract user-facing strings to i18n message catalog`

### 13.3 Add locale detection

Detect system locale and load appropriate translations.

**Commit:** `feat: implement system locale detection`

### 13.4 Add initial translations (English baseline)

**Commit:** `feat(i18n): add English baseline translations`

---

## Phase 14: Documentation

Spec section 14.

### 14.1 Add man page generation

Use `clap_mangen` to generate man pages from CLI definitions.

**Commit:** `docs: add man page generation with clap_mangen`

### 14.2 Enhance CLI help text

Make all `--help` output comprehensive with examples.

**Commit:** `docs(cli): enhance help text with examples and detailed descriptions`

---

## Phase 15: Migration Tools

Spec section 15.

### 15.1 Create `migrate` module scaffold

**Commit:** `feat(migrate): add migration module scaffold`

### 15.2 Implement commitlint config migration

Parse `.commitlintrc.*` and convert to `.cocoa.toml`.

**Commit:** `feat(migrate): implement commitlint configuration migration`

### 15.3 Implement conventional-changelog migration

Parse `conventional-changelog` config and convert.

**Commit:** `feat(migrate): implement conventional-changelog migration`

### 15.4 Implement semantic-release migration

**Commit:** `feat(migrate): implement semantic-release migration`

### 15.5 Add rollback support

Save backup of existing config before migration, support `--undo`.

**Commit:** `feat(migrate): add rollback support with config backup`

### 15.6 Add tests for migration tools

**Commit:** `test(migrate): add tests for all migration paths`

---

## Phase 16: Final Polish

### 16.1 Remove unused `dialoguer` dependency (if not used by this point)

Or confirm it's being used by `cocoa commit` / `cocoa init`.

**Commit:** `chore: clean up unused dependencies`

### 16.2 Ensure 80%+ test coverage

Add any missing tests to reach the spec's requirement.

**Commit:** `test: add missing tests to reach 80% coverage target`

### 16.3 Final CI/CD compatibility audit

Ensure all commands work headlessly, `--json` output is consistent, exit codes are correct.

**Commit:** `fix: ensure CI/CD compatibility across all commands`

---

## Summary

| Phase     | Feature                      | Commits         |
| --------- | ---------------------------- | --------------- |
| 0         | Cleanup and debt             | 5               |
| 1         | git2 migration               | 7               |
| 2         | Cascading config             | 6               |
| 3         | `cocoa init`                 | 6               |
| 4         | Lint improvements            | 4               |
| 5         | `cocoa hook`/`unhook`        | 6               |
| 6         | `cocoa commit` (interactive) | 10              |
| 7         | Changelog generation         | 12              |
| 8         | Version management           | 7               |
| 9         | `cocoa bump`                 | 2               |
| 10        | `cocoa tag`                  | 5               |
| 11        | `cocoa release`              | 3               |
| 12        | Security                     | 2               |
| 13        | i18n                         | 4               |
| 14        | Documentation                | 2               |
| 15        | Migration tools              | 6               |
| 16        | Final polish                 | 3               |
| **Total** |                              | **~90 commits** |
