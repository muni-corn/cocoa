# cocoa development guide

## build/test commands

### basic commands

- build: `cargo build`
- test all: `cargo test`
- single test: `cargo test test_name`
- lint: `cargo clippy`
- format: `cargo fmt`

### specific test categories

- unit tests only: `cargo test --lib`
- integration tests: `cargo test --test '*'`
- e2e tests: `cargo test --test 'e2e_*'`
- specific integration test: `cargo test --test git_integration_test`
- tests with output: `cargo test -- --nocapture`
- single-threaded (debugging): `cargo test -- --test-threads=1`

### running the cli

- lint commit: `cargo run -- lint --stdin` (provide message via stdin)
- generate commit: `cargo run -- generate` (requires ai configuration and staged
  changes)

## architecture overview

### dual build targets

cocoa is both a library (`src/lib.rs`) and binary (`src/main.rs`). the library
exports core functionality for potential external use, while the binary provides
the cli interface.

### async architecture

- uses tokio runtime for async/await operations
- ai client calls are async to avoid blocking on network i/o
- main entry point is `#[tokio::main]`

### trait-based abstractions

**git operations** (`src/git_ops.rs`):

- `GitOperations` trait abstracts git commands for testability
- `RealGitOps` executes actual git commands via `std::process::Command`
- `MockGitOps` (test-only) provides configurable mocks
- all git-dependent code accepts `&impl GitOperations` for dependency injection

**ai providers** (`src/ai/`):

- unified interface via `genai` crate (v0.3)
- supports multiple providers: openai, anthropic, ollama, openrouter
- provider abstraction in `ai/provider.rs`
- secure api key management via env vars or file paths
- async client in `ai/client.rs`

### parser architecture

**commit message parsing** (`src/commit.rs`):

- uses `nom` parser combinator library
- parses conventional commit format: `type(scope)!: subject`
- extracts type, scope, breaking change marker, subject, body, footers
- intentionally permissive for bodies while strict on headers
- returns structured `CommitMessage` with parsed components

### error handling

- uses `thiserror` for domain-specific error types
- uses `anyhow` for application-level errors
- each module defines its own error enum:
  - `GenerateError` for commit generation failures
  - `ParseError` for commit parsing failures
  - errors have descriptive messages for users
- exit codes: 0 (success), 1 (general), 2 (config), 3 (validation), 4 (api), 5
  (git)

## module structure

### core modules

- **commit** (`src/commit.rs`): nom-based parser for conventional commit
  messages
- **lint** (`src/lint.rs`): validation engine that checks messages against
  configured rules
- **generate** (`src/generate.rs`): ai-powered commit message generation with
  git analysis
- **config** (`src/config.rs`): toml configuration loading with cascading
  fallbacks
- **git_ops** (`src/git_ops.rs`): trait-based git command abstraction
- **ai/** (`src/ai/mod.rs`): multi-provider ai client abstraction
  - `client.rs`: generic ai client wrapper with prompt building
  - `provider.rs`: provider enum and validation
  - `config.rs`: ai configuration and secret management
- **style** (`src/style.rs`): terminal output formatting and user messaging
- **cli** (`src/cli.rs`): clap-based command line argument parsing

### data flow for commit generation

1. cli parses `generate` command → `handle_generate()`
1. check for ai configuration in `.cocoa.toml`
1. `generate::generate_commit_message()` orchestrates:
   - `analyze_staged_changes_with_git()` extracts diff and file stats
   - `extract_git_context_with_git()` gets branch, recent commits, repo name
   - `AiClient::generate_commit_message()` builds prompt and calls ai api
   - `Linter::lint()` validates generated message
1. present message to user for confirmation
1. optionally commit with `git commit -m`

## configuration system

### cascading configuration

config is loaded from first found:

1. `.cocoa.toml` in repository root
1. `$XDG_CONFIG_HOME/cocoa/cocoa.toml`
1. `~/.config/cocoa/cocoa.toml`
1. `/etc/cocoa/cocoa.toml`
1. built-in defaults if none found

### configuration structure

- **commit**: types, scopes, validation rules
- **commit.rules.warn**: warning thresholds (non-blocking)
- **commit.rules.deny**: error thresholds (blocking)
- **ai**: provider, model, temperature, max_tokens
- **ai.secret**: api key via env var or file path
- **changelog**: output format and sections (not yet implemented)
- **version**: semver/calver strategy and tag settings (not yet implemented)

## testing infrastructure

### test helpers (`tests/helpers/`)

**TestRepo** (`tests/helpers/git_repo.rs`):

- creates temporary git repositories for integration tests
- provides methods: `new()`, `create_file()`, `stage_file()`, `commit()`,
  `create_branch()`, `checkout()`, `set_remote()`
- auto-cleans up on drop

**MockGitOps** (in `src/git_ops.rs` with `#[cfg(test)]`):

- configurable mock for `GitOperations` trait
- use for unit testing git-dependent logic without real git

### test fixtures (`tests/fixtures/`)

- `configs/`: minimal.toml, with_custom_rules.toml
- `commit_messages/`: valid.txt, invalid.txt, with_breaking.txt

### test coverage

- 76 tests passing (93 defined)
- unit tests: 53 across 10 modules
- integration tests: 14 (config loading, git operations)
- e2e tests: 9 (cli commands)
- target: 80%+ coverage overall

## code style and conventions

### rust conventions

- rust 2024 edition
- `snake_case` for variables, functions, modules
- `PascalCase` for types, structs, enums
- `SCREAMING_SNAKE_CASE` for constants
- prefer `Result<T, E>` over panics
- semantic imports (avoid glob imports like `use module::*`)
- keep functions small and focused

### string styling

- all user-facing strings lowercase; do not capitalize sentences
- example: "checking commit message..." not "Checking commit message..."
- exception: proper nouns, acronyms, code identifiers

### error messages

- friendly and actionable
- provide context: "couldn't connect to 'users_db' at localhost:5432"
- suggest solutions: "use `git add <files>` to stage changes first"
- lowercase throughout

### commit conventions

- follow conventional commits spec strictly
- types: build, chore, ci, docs, feat, fix, perf, refactor, revert, style, test
- make small, atomic commits
- no co-author footers

## implementation status

### completed features

- commit message parsing (nom-based)
- commit message linting with configurable rules
- ai client abstraction with multi-provider support
- commit generation with git context analysis
- cli with lint and generate commands
- configuration loading with cascading fallbacks
- comprehensive test suite (76 passing tests)

### not yet implemented

- interactive commit creation (`cocoa commit`)
- changelog generation (`cocoa changelog`)
- version bumping (`cocoa bump`)
- git tagging (`cocoa tag`)
- full release workflow (`cocoa release`)
- git hook management (`cocoa hook`, `cocoa unhook`)
- git range linting (`cocoa lint HEAD~5..HEAD`)

## key dependencies

### runtime

- `clap` (v4): cli argument parsing with derive macros
- `nom` (v8): parser combinators for commit message parsing
- `genai` (v0.3): unified ai provider interface
- `tokio` (v1): async runtime with "full" features
- `git2` (v0.20): git operations (currently unused, using `Command` instead)
- `serde`/`toml`: configuration parsing
- `thiserror`: error type derivation
- `anyhow`: application error handling
- `console`/`dialoguer`: terminal ui

### dev/test

- `assert_cmd` (v2): cli testing
- `predicates` (v3): assertions for cli output
- `tempfile` (v3): temporary directories for test repos

## spec compliance

cocoa follows its formal specification in `SPEC.md`. key requirements:

- conventional commits v1.0.0 compliance
- toml v1.0.0 configuration format
- semantic versioning 2.0.0 support (for future versioning features)
- git 2.25.0+ compatibility
- utf-8 commit message support
- exit codes: 0 (success), 1 (general), 2 (config), 3 (validation), 4 (api), 5
  (git)
