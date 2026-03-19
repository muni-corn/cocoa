# configuration guide

i can be customized to match your project's workflow!

## configuration file

### location

i'll search for configuration files in this order:

1. **command-line:** `cocoa --config /path/to/config`
2. **project root:** `.cocoa.toml`
3. **user config:** `$XDG_CONFIG_HOME/cocoa/cocoa.toml`
4. **home directory:** `~/.config/cocoa/cocoa.toml`
5. **system-wide:** `/etc/cocoa/cocoa.toml`
6. **defaults:** built-in defaults if nothing is found

the options i find first are the options i'll use first!

### format

configuration is written in TOML format. if you don't have a `.cocoa.toml` yet, get started with:

```bash
cocoa init  # interactive setup wizard
```

or, you know, create `.cocoa.toml` manually.

## complete configuration reference

### commit configuration

```toml
[commit]
# allowed commit types (one per line or comma-separated)
types = [
  "build",
  "chore",
  "ci",
  "docs",
  "feat",
  "fix",
  "perf",
  "refactor",
  "style",
  "test",
]

# optional: allowed scopes
# (if you leave this empty, i'll allow any scope)
scopes = [
  "auth",
  "api",
  "database",
  "ui",
]

[commit.rules]
# enable/disable commit message linting
enabled = true

# don't lint commits created by git
ignore_fixup_commits = true      # `git commit --fixup`
ignore_amend_commits = true      # `git commit --amend`
ignore_squash_commits = true     # `git commit --squash`
ignore_merge_commits = true      # `git merge`
ignore_revert_commits = true     # `git revert`

# warnings: show but don't fail
[commit.rules.warn]
subject_length = 72              # max characters in subject
body_length = 500                # max characters in body
no_scope = false                 # require scope?
no_body = false                  # require body?
no_type = true                   # require type?
regex_patterns = []              # custom patterns to warn about

# errors: show and fail the commit
[commit.rules.deny]
subject_length = 100
body_length = 1000
no_scope = false
no_body = false
no_type = true                   # type is required
regex_patterns = [
  "^(feat|fix|chore|docs|test|refactor|perf|style|build|ci)",
]
```

### AI configuration

```toml
[ai]
# AI provider: openai, anthropic, ollama, or openrouter
provider = "openai"

# model identifier (varies by provider)
model = "gpt-4"

# temperature: 0.0 = deterministic, 1.0 = creative
temperature = 0.7

# max tokens in response
max_tokens = 500

# API secret (see next section)
[ai.secret]
env = "OPENAI_API_KEY"
```

you can read [AI providers](./ai-providers.md) for a further guide.

### changelog configuration

```toml
[changelog]
# output file path (relative or absolute)
output_file = "CHANGELOG.md"

# include merge commits from `git merge`?
include_merge_commits = false

# include revert commits from `git revert`?
include_reverts = true

# date format for version headers
# uses chrono format strings: %Y-%m-%d, %d/%m/%Y, etc.
date_format = "%Y-%m-%d"

# section names for different commit types
[changelog.sections]
feat = "Features"
fix = "Bug fixes"
perf = "Performance improvements"
docs = "Documentation"
refactor = "Refactoring"
style = "Style"
test = "Tests"
chore = "Maintenance"
build = "Build"
ci = "CI/CD"

# Special section for breaking changes (appears first)
breaking = "Breaking changes"
```

### version configuration

```toml
[version]
# strategy: semver or calver
strategy = "semver"

# For calver, custom format
# YYYY, MM, DD, MINOR, PATCH, 0, 1, etc.
calver_format = "YYYY.MM.0"

# prefix for git tags
tag_prefix = "v"

# also sign tags with gpg?
sign_tags = false

# files to update with new version
# i'll search for the old version string and replace it
commit_version_files = [
  "package.json",
  "Cargo.toml",
  "pyproject.toml",
]
```

## common configurations

### minimal setup (just the basics)

```toml
[commit]
types = ["feat", "fix", "chore", "docs"]

[changelog]
output_file = "CHANGELOG.md"

[version]
strategy = "semver"
tag_prefix = "v"
```

### strict enterprise setup

```toml
[commit]
types = ["feat", "fix", "chore", "docs", "test", "refactor"]
scopes = ["auth", "api", "database", "ui", "infra"]

[commit.rules]
enabled = true

[commit.rules.warn]
subject_length = 50
no_body = false

[commit.rules.deny]
subject_length = 72
body_length = 500
no_type = true
no_scope = false

[changelog]
output_file = "CHANGELOG.md"
include_merge_commits = false

[version]
strategy = "semver"
tag_prefix = "release-"
sign_tags = true
commit_version_files = ["package.json", "Cargo.toml"]
```

### relaxed setup (maximum flexibility)

```toml
[commit]
types = ["feat", "fix", "chore", "docs", "test", "refactor", "perf", "style", "build", "ci"]

[commit.rules]
enabled = true

[commit.rules.warn]
subject_length = 100

[commit.rules.deny]
subject_length = 200
no_type = true

[changelog]
output_file = "CHANGELOG.md"

[version]
strategy = "semver"
tag_prefix = "v"
```

### emoji-friendly setup

```toml
[commit]
types = ["feat", "fix", "docs", "style", "refactor", "test", "chore", "perf"]

[changelog]
output_file = "CHANGELOG.md"

[changelog.sections]
feat = "✨ New features"
fix = "🐛 Bug fixes"
docs = "📚 Documentation"
style = "🎨 Styling"
refactor = "♻️ Refactoring"
test = "✅ Tests"
perf = "⚡ Performance"
chore = "🔧 Maintenance"
breaking = "💥 Breaking changes"

[version]
strategy = "semver"
tag_prefix = "v"
```

## best practices for teams

### 1. communicate rules to your team

create a `COMMIT_GUIDELINES.md`:

````markdown
## Commit guidelines

Our project uses `cocoa` to enforce conventional commits as configured in `.cocoa.toml`.

### Allowed types

- feat: new features
- fix: bug fixes
- docs: documentation
- chore: maintenance

### Scopes

Optional, but when used:

- auth: authentication
- api: API endpoints
- db: database
- ui: user interface

### Examples

```gitcommit
feat(auth): add password reset functionality
```

```gitcommit
fix(api): prevent timeout in report endpoint
```

```gitcommit
docs: update README with setup instructions
```

### Setup

```bash
cocoa init    # set up locally
cocoa hook    # enable git hooks
```
````

### 3. use hooks to enforce rules

```bash
cocoa hook
```

now i'll validate commits automatically!

### 4. version control your config

you can (and should!) commit `.cocoa.toml` to version control!

```bash
git add .cocoa.toml
git commit -m "docs: configure conventional commits"
git push
```

then, new team members automatically get your config!

### 5. per-project vs. global config

**project-specific:**

- `.cocoa.toml` in project root
- good for team-specific rules
- committed to git

**user/system config:**

- `~/.config/cocoa/cocoa.toml` or `/etc/cocoa/cocoa.toml`
- good for personal preferences
- not checked into version control

### 6. override config on command line

for special cases:

```bash
# use custom config file
cocoa --config /path/to/special.toml commit

# disable color globally (for one command)
cocoa --no-color lint

# run without config (use defaults)
cocoa init
```

## configuration validation

check if your config is valid:

```bash
# verbose mode shows loaded config
cocoa --verbose init

# test with dry-run
cocoa --dry-run init

# check with a specific command
cocoa lint --verbose
# should show: "using config from .cocoa.toml"
```

## troubleshooting configuration

### "could not load configuration"

is your TOML syntax correct?

example for checking:

```bash
# validate TOML (requires external tool)
toml-check .cocoa.toml
```

or, check manually for common issues:

- missing quotes around strings
- unmatched brackets
- invalid special characters

### "type 'feature' not recognized"

perhaps the type name isn't included in your config. check your `.cocoa.toml` for what types you
have configured.

or, you can update your config:

```toml
[commit]
types = ["feature", "fix"]
```

### "configuration not found"

of course, start by checking that your configuration file exists in the correct location.

you can create a new one:

```bash
cocoa init
```

or specify a config file path on the command line:

```bash
cocoa --config ~/.config/cocoa/cocoa.toml commit
```

## environment-specific configuration

you can use different configs for different scenarios:

```bash
# development (relaxed rules)
cocoa --config .cocoa.dev.toml commit

# production (strict rules)
cocoa --config .cocoa.prod.toml lint

# CI/CD (different settings)
cocoa --config .cocoa.ci.toml release
```

create multiple `.cocoa.*.toml` files for each environment!

## viewing your config

see what config i loaded:

```bash
cocoa --verbose commit
# output includes: "using config from: .cocoa.toml"

# view the actual config values (careful with secrets!)
cat .cocoa.toml
```

## resetting to defaults

remove your config to use defaults:

```bash
# backup first!
cp .cocoa.toml .cocoa.toml.backup

# delete to use defaults
rm .cocoa.toml

# start fresh
cocoa init
```

## next steps

- set up AI: [AI providers](./ai-providers.md)
- enforce rules: [linting commits](./linting-commits.md)
- automate validation: [git hooks](./git-hooks.md)
- create releases: [version management](./versioning.md)
