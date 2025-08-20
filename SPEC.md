# `cocoa`: a conventional commit assistant

## 1. introduction

This document specifies the requirements and behavior of cocoa (conventional
commit assistant), a comprehensive toolset for managing conventional commits,
commit message linting, changelog generation, and version management.

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be
interpreted as described in [RFC 2119](https://www.ietf.org/rfc/rfc2119.txt).

**version:** 0.1.0\
**status:** draft\
**date:** August 2025

## 2. overview

`cocoa` is a unified tool that provides:

- AI-powered commit message generation
- Commit message linting and validation
- Automated changelog generation
- Semantic version management and git tagging

## 3. configuration

### 3.1 configuration file

- The configuration file SHALL be named `.cocoa.toml`
- The configuration file SHOULD be located in the repository root
- The configuration file MAY also be found in
  `$XDG_CONFIG_HOME/cocoa/cocoa.toml` or `~/.config/cocoa/cocoa.toml` or
  `/etc/cocoa/cocoa.toml`.
- The system SHALL support cascading configuration (repository → user → system)
- The configuration format MUST be valid TOML v1.0.0
- The system SHALL provide sensible defaults when no configuration is present
- Invalid configuration MUST result in a clear error message

### 3.2 configuration schema

The configuration file SHALL support the following format and sections:

```toml
[commit]
types = [
  "build",
  "chore",
  "ci",
  "docs",
  "feat",
  "fix",
  "perf",
  "refactor",
  "revert",
  "style",
  "test",
]
scopes = [] # optional: allowed scopes

[commit.rules]
enabled = true # whether to lint commit messages
max_subject_length = 72
max_body_length = 500
require_scope = false
require_body = false
require_type = true
require_breaking_change_footer = true
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_squash_commits = true
ignore_merge_commits = true
ignore_revert_commits = true
regex_patterns = [] # Optional: custom validation patterns

[ai]
provider = "openai" # or "anthropic", "ollama", "openrouter"
model = "gpt-4"
temperature = 0.7
max_tokens = 500

[ai.secret]
# ai.secret is an externally-tagged serde enum
env = "OPENAI_API_KEY" # secret can be provided via environment variable name...
file = "./path/to/file" # ...or file path, relative or absolute

[changelog]
output_file = "CHANGELOG.md"
include_merge_commits = false
include_reverts = true
date_format = "%Y-%m-%d"

[changelong.sections]
feat = "Features"
fix = "Bug fixes"
perf = "Performance"
docs = "Documentation"
# any commit types not specified here will not be included in the changelog at all

# breaking is a special category for breaking changes of any other type
breaking = "Breaking changes"

[version]
strategy = "semver" # or "calver"
tag_prefix = "v"
sign_tags = true

# files that will be searched for an old version string when bumping. old version strings will be replaced with the new version string.
commit_version_files = ["package.json", "Cargo.toml", "pyproject.toml"]
```

## 4. commit generation

### 4.1 ai-powered generation

- The system SHALL support multiple AI providers (OpenAI, Anthropic, local
  models with Ollama, OpenRouter)
- The system SHALL analyze staged changes, branch name, and recent commit
  messages to generate appropriate commit messages
- Generated messages SHALL conform to:
  - the Conventional Commits specification v1.0.0
  - the user's defined commit rules
- The system SHOULD cache API responses to minimize redundant calls
- The system MUST NOT expose API keys in logs or error messages
- Users MUST be able to edit generated messages before committing

### 4.2 interactive mode

- The system MUST provide an interactive CLI for commit creation
- The interactive mode MUST support:
  - Type selection from configured types
  - Scope input with autocomplete from configured scopes
  - Subject line composition with character counting
  - Optional body text entry
  - Breaking change annotation
  - Issue reference linking
- The system SHOULD provide real-time validation feedback

## 5. commit linting

### 5.1 validation rules

The linter SHALL validate:

- Commit type against allowed types
- Scope against allowed scopes (if configured)
- Subject line length
- Body length
- Conventional Commits format compliance
- Custom regex patterns (if configured)

### 5.2 git hook integration

- The system SHALL provide a `commit-msg` git hook
- The hook installation MUST be idempotent
- The system SHALL provide automatic hook installation via `cocoa hook`
- The system SHALL provide automatic hook uninstallation via `cocoa unhook`
- Hooks MUST exit with appropriate status codes (0 for success, non-zero for
  failure)

### 5.3 ci/cd integration

- The system MUST be executable in CI/CD environments
- The system MUST support linting a range of commits
- The system SHOULD provide machine-readable output formats (JSON)

## 6. changelog generation

### 6.1 generation process

- The system SHALL parse git history to extract conventional commits
- The system SHALL group commits by type and version
- Breaking changes MUST be prominently displayed
- The system SHOULD support custom changelog templates
- Generated changelogs MUST be deterministic (same input = same output)

### 6.2 version detection

- The system MUST detect existing version tags
- The system MUST support multiple tag formats via configuration
- The system SHOULD handle pre-release and build metadata

### 6.3 output formats

- The system MUST support Markdown output
- The system SHOULD support JSON output
- The system MAY support HTML, reStructuredText, and AsciiDoc

## 7. version management

### 7.1 semantic versioning

- The system MUST support Semantic Versioning 2.0.0
- Version bumps MUST follow these rules:
  - Breaking changes → major version
  - Features → minor version
  - Fixes → patch version
- The system MUST support pre-release versions

### 7.2 calendar versioning

- The system SHOULD support Calendar Versioning
- The system MUST allow custom CalVer formats

### 7.3 file updates

- The system MUST update version in configured files
- The system MUST preserve file formatting
- The system SHOULD support custom version update scripts
- File updates MUST be atomic (all succeed or all fail)

### 7.4 git tagging

- The system MUST create annotated git tags
- Tags MUST include the changelog for that version
- The system SHOULD support GPG signing of tags
- The system MUST verify tag uniqueness before creation

## 8. cli interface

### 8.1 commands

The CLI MUST provide the following commands:

```bash
cocoa init                 # initialize configuration
cocoa commit               # interactive commit creation
cocoa generate             # generate commit from staged changes
cocoa lint [range|msg]     # lint commit messages
cocoa changelog [range]    # generate changelog
cocoa bump [bump-type]     # bump version (major|minor|patch|auto)
cocoa tag                  # create version tag
cocoa release              # full release (version + tag + changelog)
```

### 8.2 global options

All commands MUST support:

- `--config <path>` - custom configuration file
- `--verbose` - verbose output
- `--quiet` - suppress non-error output
- `--no-color` - disable colored output
- `--json` - JSON output format
- `--help` - display help information
- `--version` - display version information

### 8.3 exit codes

The system MUST use consistent exit codes:

- 0: success
- 1: general error
- 2: configuration error
- 3: validation error
- 4: API/network error
- 5: git operation error

## 9. security considerations

- API keys MUST NEVER be stored in configuration files
- API keys MUST be read from environment variables or secure storage
- The system MUST NOT log sensitive information
- Generated commits MUST NOT include sensitive data from diffs
- The system SHOULD warn about potentially sensitive content

## 10. compatibility

- The system MUST support Git 2.25.0 or later
- The system SHOULD run on Linux, macOS, and Windows
- The system MUST support UTF-8 encoded commit messages
- The system SHOULD be compatible with major git hosting platforms (GitHub,
  GitLab, Bitbucket)

## 11. error handling

- All errors MUST include actionable, friendly error messages
- The system MUST provide suggestions for common errors
- Errors MUST include relevant context (file paths, line numbers)
- The system SHOULD provide error recovery mechanisms

## 12. localization

- The system SHOULD support internationalization
- Commit messages MUST support Unicode
- The system SHOULD detect system locale
- Documentation SHOULD be available in multiple languages

## 13. testing requirements

- The system MUST include comprehensive unit tests
- The system MUST include integration tests
- Test coverage MUST exceed 80%
- The system MUST support dry-run mode for all destructive operations

## 14. documentation

- The system SHALL provide comprehensive CLI help
- The system SHOULD include man pages
- The system SHOULD provide interactive tutorials
- API documentation MUST be auto-generated

## 15. migration

- The system MAY provide migration tools from:
  - commitlint
  - conventional-changelog
  - semantic-release
- Migration MUST preserve existing configuration where possible
- The system MUST provide rollback capabilities

## 16. glossary

- **Conventional Commits**: a specification for adding person- and
  machine-readable meaning to commit messages
- **Semantic Versioning**: a versioning scheme using MAJOR.MINOR.PATCH format
- **Calendar Versioning**: a versioning scheme based on calendar dates
- **breaking change**: a change that breaks backward compatibility
- **scope**: a noun describing a section of the codebase
- **type**: the category of a code change (feat, fix, chore, refactor, etc.)

## references

- [RFC 2119](https://www.ietf.org/rfc/rfc2119.txt) - key words for use in RFCs
- [Conventional Commits v1.0.0](https://www.conventionalcommits.org/)
- [Semantic Versioning 2.0.0](https://semver.org/)
- [Calendar Versioning](https://calver.org/)
- [TOML v1.0.0](https://toml.io/)

---

# copyright notice

This specification is released under the GPL v3 license.

# revision history

| version | date       | changes               | author   |
| ------- | ---------- | --------------------- | -------- |
| 0.1.0   | 2025-08-18 | initial specification | municorn |
