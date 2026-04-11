# Changelog

## **v0.2.2**

### Bug fixes

- **release:** remove range from changelog generation

---

## **v0.2.1**

### Features

- **changelog:** add customizable next version label
- **version:** handle 0.x.x versions in bump operations
- **config:** increase default body length limits
- **prompt:** emphasize reasoning in commit message guidelines
- **cmd:** support message source from environment variable
- **devenv:** add devenv module for cocoa git hooks configuration

### Bug fixes

- **commit:** align fixup/squash/revert detection with git conventions
- **commit:** make commit_type optional to handle non-conventional commits
- **generate:** remove debug eprintln statements
- **hook:** fix generate hook invocation
- **devenv:** disable filename passing for cocoa commit hook
- **cmd:** fix indentation for release command help
- make breaking changes header sentence case in markdown

### Tests

- **helpers:** suppress dead_code warnings for test utilities
- **helpers:** simplify mock ai imports and update provider config
- correct version bump expectations in test cases
- move helpers module declaration to fix structure issues
- **prompt:** move build_prompt tests to prompt module

### Documentation

- **user-guide:** lowercase section headers in linting guide
- standardize punctuation in comments and messages

---

## **v0.2.0**

### Features

- make commit message generation configurable via CommitConfig
- **generate:** increase recent commits context from 5 to 10
- add hook kind selection and generate hook support
- add body to generated commit messages

### Tests

- update assertions to match current behavior
- expand hook tests for lint, generate, and all kinds

### Documentation

- simplify README description
- add comprehensive README

---

## **v0.1.0**

### Features

- **cli:** reintroduce help subcommand
- **cli:** modify help messages a bit
- **migrate:** implement conventional-changelog migration
- **migrate:** implement commitlint configuration migration
- **migrate:** add rollback support with config backup
- **migrate:** add migration module scaffold
- **migrate:** implement semantic-release migration
- **release:** add release module with orchestration logic
- **i18n:** add English baseline translations
- implement system locale detection
- **release:** wire release command into CLI handler
- add i18n infrastructure with message catalog
- **generate:** add sensitive content warning for staged diffs
- **tag:** wire tag command into CLI handler
- **tag:** implement GPG signing for tags
- **tag:** implement tag uniqueness verification
- **tag:** implement annotated git tag creation
- **bump:** wire bump command into CLI handler
- **version:** implement atomic version file updates
- **version:** implement automatic bump type detection from commits
- **version:** implement version detection from git tags
- **version:** implement calendar versioning engine
- **version:** implement semantic versioning engine
- **version:** add version module scaffold
- **changelog:** wire changelog command into CLI handler
- **changelog:** add custom template support
- **changelog:** implement AsciiDoc output format
- **changelog:** implement reStructuredText output format
- **changelog:** implement HTML output format
- **changelog:** implement JSON output format
- **changelog:** implement markdown output renderer
- **changelog:** define entry and version data types
- **changelog:** add changelog module scaffold
- **interactive:** wire commit command into CLI handler
- **interactive:** assemble message, validate, and commit
- **interactive:** implement issue reference linking
- **interactive:** implement breaking change annotation flow
- **interactive:** implement optional body text entry
- **interactive:** implement subject composition with char counting
- **interactive:** implement scope input with autocomplete
- **interactive:** implement commit type selection prompt
- **interactive:** add interactive commit module scaffold
- **hook:** wire hook and unhook commands into CLI
- **hook:** implement hook uninstallation with backup restore
- **hook:** implement commit-msg hook installation
- **hook:** add hook module
- **cli:** add hook and unhook command variants
- **lint:** implement git range linting, file path input, and dry-run mode
- **init:** wire init command into CLI handler
- **init:** add init module scaffold
- **config:** add version management configuration schema
- **config:** add changelog configuration schema
- **config:** implement config merging for cascading fallback
- **config:** add cascading config file discovery
- **git_ops:** implement extended methods in Git2Ops
- **git_ops:** extend GitOperations trait with tag, range, and hook methods
- **cli:** add global --dry-run flag for non-destructive operations
- **cli:** wire up --no-color flag to disable terminal colors
- **cli:** wire up --verbose flag for detailed output
- **ai:** add Provider struct with serde support for AI adapter configuration
- **ai:** add provider-specific model mapping and auth resolver to AI client
- **cli:** add conditional help templates for subcommands without arguments
- **git_ops:** add git operations abstraction for testability
- implement cli integration for generate command
- implement commit generation logic
- integrate genai for real ai provider support
- add core ai module structure
- add goodbye functions and improve error messaging with better UX
- **flake:** add bacon package to development shell for Rust development
- add welcome function and add styling with Unicode symbols
- add style module with colored console output utility functions
- improve console output with colored icons and lowercase
- add Display trait implementation for LintViolation
- **config:** update commit message validation rules with stricter warn defaults
- implement warn/deny rule structure for linting
- **spec:** replace single-level commit rules with warn/deny rule structure
- implement main application and lint command
- implement CLI interface structure
- implement lint validation engine
- implement conventional commit parser
- implement configuration module
- add core dependencies for CLI and configuration

### Bug fixes

- **commit:** strip git context and comments from commit messages
- ensure CI/CD compatibility across all commands
- audit and sanitize error messages to prevent secret leakage
- **changelog:** ensure deterministic output ordering
- **lint:** use regex crate for custom pattern matching
- **flake:** update project description from generic Rust to cocoa
- **commit:** fix footer line parsing with "BREAKING CHANGE" support
- **commit:** allow `is_ident_char` to take hyphens
- change JSON output from pretty-printed to compact unstyled format

### Tests

- **fixtures:** add `full_commit_message.txt`
- add tests for commits with comments and git context
- **commit:** add test for commit messages with git context
- add missing tests to reach 80% coverage target
- **migrate:** add tests for all migration paths
- **tag:** add integration tests for tag creation and signing
- **release:** add e2e tests for full release workflow
- **bump:** add e2e tests for bump command
- **version:** add tests for semver, calver, detection, and file updates
- **changelog:** add unit and integration tests
- **interactive:** add tests for interactive commit creation
- **hook:** add integration tests for hook install and uninstall
- **lint:** add tests for git range and file path linting
- **init:** add e2e tests for init command
- **config:** add tests for cascading config discovery and merging
- **git_ops:** add integration tests for Git2Ops
- **git_ops:** update MockGitOps for extended trait methods
- **ai:** add comprehensive deserialization tests for Provider enum
- isolate git configuration in test environment
- add test infrastructure for helpers and fixtures
- add integration and e2e test suites for config, git, and lint
- add comprehensive unit tests for cli, style, and ai modules
- add dependencies and structure for CLI integration testing

### Documentation

- add user guide
- add man page generation with clap_mangen
- **cli:** enhance help text with examples and detailed descriptions
- add plan to implement remaining features
- **comments:** apply sentence case to all doc comments
- restructure AGENTS.md into comprehensive cocoa development guide
- **TESTING:** add comprehensive testing documentation
- **commit:** improve documentation formatting and capitalize sentence beginnings
- add comments for commit module
- add development guidelines and conventions

---
