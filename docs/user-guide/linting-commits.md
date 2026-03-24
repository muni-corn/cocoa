# linting commits

ensure commit quality with automated validation!

## what is linting?

linting is automatic checking of commit messages to ensure they follow your rules. think of it as a
helpful reviewer who catches issues _before_ they're pushed.

i can catch:

- wrong commit type
- invalid scope
- subject line too long
- missing body
- malformed conventional commits
- violations of custom patterns
- missing issue references

## basic linting

### lint the latest commit

```bash
cocoa lint
```

output if valid:

```
✓ commit message is valid
```

output if invalid:

```
✗ commit validation failed

error: subject line is 85 characters (max 72)
warning: no body found (recommended for complex changes)

commit message:
feat(auth): add two-factor authentication support with time-based one-time passwords which is really great
```

### lint specific commits

```bash
# last 3 commits
cocoa lint HEAD~2...HEAD

# entire branch compared to main
cocoa lint main...HEAD

# specific commit
cocoa lint abc1234

# by count
cocoa lint -n 5    # last 5 commits
```

### lint from stdin

useful for testing messages or CI/CD:

```bash
echo "feat(auth): add 2FA" | cocoa lint --stdin

# or pipe from file
cat commit-msg.txt | cocoa lint --stdin
```

## understanding validation rules

### configure rules in `.cocoa.toml`

```toml
[commit.rules]
enabled = true
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_merge_commits = true

# Warnings: shown but don't fail commit
[commit.rules.warn]
subject_length = 72
body_length = 500
no_scope = false
no_body = false
no_type = false
regex_patterns = []

# Denials: shown and fail commit
[commit.rules.deny]
subject_length = 100
body_length = 1000
no_scope = false
no_body = false
no_type = true
regex_patterns = [
  "^(feat|fix|chore|docs|test)",  # Enforces type
  "TODO|FIXME|XXX"                # Disallow development markers
]
```

### rule types

| rule             | what it checks                  | example                          |
| ---------------- | ------------------------------- | -------------------------------- |
| `subject_length` | max characters in subject       | 72 (github default)              |
| `body_length`    | max characters in body          | 500 or 1000                      |
| `no_type`        | requires type (feat, fix, etc.) | must have `feat(...)`            |
| `no_scope`       | requires scope                  | must have `feat(auth):`          |
| `no_body`        | requires body text              | complex changes need explanation |
| `regex_patterns` | custom regex validation         | custom rules                     |

### warnings vs. errors

**warnings:** advisory, doesn't block commit:

```toml
[commit.rules.warn]
subject_length = 72
```

if you write an 80-character subject:

```
warning: subject line is 80 characters (recommended max 72)
```

commit still succeeds!

**errors:** blocks commit if violated:

```toml
[commit.rules.deny]
subject_length = 100
```

if you write a 105-character subject:

```
error: subject line is 105 characters (max 100)
```

commit is rejected!

## setting up team rules

### example: strict enterprise standards

```toml
[commit]
types = ["feat", "fix", "chore", "docs", "test", "refactor", "perf"]
scopes = [
  "auth",
  "api",
  "database",
  "ui",
  "docs"
]

[commit.rules]
enabled = true
ignore_fixup_commits = false
ignore_amend_commits = false

[commit.rules.warn]
subject_length = 50
body_length = 500
no_scope = false       # Scope is optional but recommended
no_body = false

[commit.rules.deny]
subject_length = 72
body_length = 1000
no_type = true         # Type is required
no_scope = false
no_body = true         # Body is required for features
regex_patterns = [
  "^(feat|fix|chore|docs|test|refactor|perf)",
  "^(feat|fix)\\(",     # feat/fix must have scope
  "Closes #\\d+|Fixes #\\d+"  # Issues must be referenced
]
```

now all commits must:

- have a type from the allowed list
- have a body (required for all commits)
- have 72 or fewer characters in subject
- reference an issue (closes/fixes)

### example: relaxed standards

```toml
[commit.rules]
enabled = true

[commit.rules.warn]
subject_length = 100
body_length = 1000

[commit.rules.deny]
subject_length = 200
no_type = true
```

allows:

- longer subjects (up to 200)
- optional scopes
- optional bodies
- warns if subject exceeds 100 chars

## git hook integration

automatically validate on every commit:

### install hooks

```bash
cocoa hook
```

now every `git commit` is validated:

```bash
git commit -m "feat(auth): add 2FA"

# if valid:
✓ commit is valid
[main abc1234] feat(auth): add 2FA

# if invalid:
✗ commit validation failed
error: no body found

# commit is rejected!
```

### uninstall hooks

```bash
cocoa hook --uninstall
```

or manually in `.git/hooks/commit-msg`.

## CI/CD integration

lint all commits on every push:

### github actions example

```yaml
name: Lint Commits
on: [pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0

      - name: Install cocoa
        run: cargo install cocoa

      - name: Lint commits
        run: cocoa lint main...HEAD --json
```

### gitlab CI example

```yaml
lint_commits:
  stage: validate
  script:
    - cargo install cocoa
    - cocoa lint main...HEAD
```

see [CI/CD integration](./ci-cd.md) for more platforms.

## JSON output for tooling

for automated processing:

```bash
cocoa lint --json
```

output:

```json
{
  "valid": false,
  "errors": [
    {
      "rule": "subject_length",
      "message": "subject line is 85 characters (max 72)",
      "commit": "abc1234",
      "severity": "error"
    }
  ],
  "warnings": [
    {
      "rule": "no_body",
      "message": "body is recommended but not required",
      "commit": "abc1234",
      "severity": "warning"
    }
  ]
}
```

use this in:

- IDE plugins
- pre-commit hooks
- build systems
- custom scripts

## custom regex patterns

use regex for advanced validation:

```toml
[commit.rules.deny]
regex_patterns = [
  "TODO|FIXME|XXX|HACK",        # disallow development markers
  "password|secret|key",        # warn about secrets
  "\\b\\d{4}-\\d{4}\\b",        # disallow credit card-like patterns
  "^[^a-z]",                    # enforce lowercase start
]
```

now commits with `TODO` or `password` are rejected!

examples that would fail:

```
fix(auth): TODO implement 2FA    (contains TODO)
feat(api): add secret key param  (contains secret)
FIX: uppercase start             (starts with uppercase)
```

## ignoring certain commits

skip validation for maintenance commits:

```toml
[commit.rules]
ignore_fixup_commits = true      # git commit --fixup
ignore_amend_commits = true      # git commit --amend
ignore_squash_commits = true     # git commit --squash
ignore_merge_commits = true      # merge commits from git merge
ignore_revert_commits = true     # git revert
```

so these don't get linted:

```bash
git commit --fixup main       # squashed during rebase, no lint needed
git commit --amend            # fine-tuning previous commit
git merge feature-branch      # git generates message, no lint needed
```

## troubleshooting lint issues

### "subject line too long"

```
✗ error: subject line is 95 characters (max 72)
```

solution: keep subjects concise. move details to body:

before:
`feat(auth): add password reset functionality with email verification and security token expiration`

after: `feat(auth): add password reset via email`

then explain in body:

```
users can reset forgotten passwords via email. password reset
tokens expire after 24 hours for security.
```

### "no type specified"

```
✗ error: missing commit type (feat, fix, chore, etc.)
```

**solution:** add a type:

```
❌ auth: add 2FA
✅ feat(auth): add 2FA
```

### "invalid scope"

```
✗ error: scope 'authentication' not in allowed list: [auth, api, ui]
```

solution: use configured scope or update `.cocoa.toml`:

option 1: use an allowed scope

```
feat(auth): add 2FA
```

option 2: add to allowed scopes in `.cocoa.toml`:

```toml
[commit]
scopes = ["auth", "api", "ui", "authentication"]
```

### custom regex failing

```
✗ error: commit message doesn't match required pattern
```

check your regex:

```bash
cocoa lint --verbose
# shows which pattern failed and why
```

debug the pattern:

```bash
# test your regex
echo "feat(auth): add 2FA" | grep -E "^(feat|fix)"
# if no output, regex failed
```

## best practices

### 1. start lenient, tighten over time

**new project:**

```toml
[commit.rules.deny]
no_type = true
```

**established project:**

```toml
[commit.rules.deny]
subject_length = 72
no_type = true
no_body = true
regex_patterns = ["Closes #\\d+"]  # require issue reference
```

### 2. communicate rules to team

```markdown
## commit guidelines

all commits must:

- type: feat, fix, chore, docs, test
- optional scope: [auth, api, ui, database]
- subject: ≤72 chars, imperative mood
- body: explain why for complex changes
- footer: reference issues with "closes #123"

example: feat(auth): add password reset

users can reset passwords via email. tokens expire after 24 hours for security.

Closes #142
```

### 3. enforce in CI

block merging if commits don't follow rules:

```bash
cocoa lint main...HEAD || exit 1
```

### 4. make exceptions explicit

allow fixup commits but nothing else:

```toml
[commit.rules]
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_merge_commits = true
ignore_revert_commits = true
```

## next steps

- **learn commit styles:** [creating commits](./creating-commits.md)
- **automate validation:** [git hooks](./git-hooks.md)
- **enforce in CI:** [CI/CD integration](./ci-cd.md)
- **generate releases:** [version management](./versioning.md)

---

**remember:** linting ensures consistency, which makes changelogs better, releases cleaner, and
history more readable!
