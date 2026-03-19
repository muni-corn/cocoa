# creating commits

let's make some beautiful commits together!

## commit structure

a well-formed commit has three parts:

```
type(scope): subject

body (optional)

footer (optional)
```

### type

the category of your change. common types:

| Type       | Use For                  | Example                              |
| ---------- | ------------------------ | ------------------------------------ |
| `feat`     | New features             | `feat: add dark mode toggle`         |
| `fix`      | Bug fixes                | `fix: resolve memory leak in parser` |
| `docs`     | Documentation            | `docs: update API reference`         |
| `test`     | Tests                    | `test: add tests for auth module`    |
| `chore`    | Maintenance              | `chore: update dependencies`         |
| `refactor` | Code restructuring       | `refactor: simplify user validation` |
| `perf`     | Performance improvements | `perf: optimize database queries`    |
| `style`    | Code formatting          | `style: format with prettier`        |
| `ci`       | CI/CD changes            | `ci: add GitHub Actions workflow`    |
| `build`    | Build system             | `build: upgrade webpack`             |

your `.cocoa.toml` defines which types are allowed.

### scope (optional)

a noun indicating what part of the codebase changed:

```gitcommit
feat(auth): add two-factor authentication
#    ^^^^
```

```gitcommit
feat(database): add user migration
#    ^^^^^^^^
```

```gitcommit
feat: add user validation
# (no scope)
```

good scopes are:

- component/module names: `auth`, `api`, `ui`, `database`
- feature areas: `payment`, `notifications`, `search`
- layers: `controller`, `service`, `repository`

### subject

a short description (≤72 characters recommended):

good:

```gitcommit
feat(auth): add password reset via email
```

```gitcommit
fix(ui): prevent duplicate form submission
```

```gitcommit
docs: update installation instructions
```

bad:

```gitcommit
feat(auth): add password reset via email which allows users to securely reset their password through an email verification link
# (too long!)
```

```gitcommit
feat(auth): ADDED PASSWORD RESET
# (too shouty!)
```

```gitcommit
feat: wip
# (too vague!)
```

### body (optional)

explain _why_ and _how_, not just what:

good:

```gitcommit
feat(auth): add password reset functionality

Users can now securely reset forgotten passwords via email. When a user
clicks "Forgot Password", they receive an email with a time-limited reset
link. This link is valid for 24 hours.

Implementation uses bcrypt for token hashing and includes rate limiting
to prevent brute force attacks.
```

bad:

```gitcommit
added reset button and email code
# (too brief, no context)
```

```gitcommit
this PR adds a password reset feature. it's really useful.
# (vague benefits, no technical details)
```

### footer (optional)

reference issues, breaking changes, or other metadata:

```gitcommit
feat(auth): add 2FA support

Implement TOTP-based two-factor authentication.

Closes #142
Fixes #156
Related-to: #200

BREAKING CHANGE: Authentication endpoints now require 2FA setup
```

**common footers:**

- `Closes #123` — link to issue (auto-closes it!)
- `Refs #123` — reference without closing
- `BREAKING CHANGE:` — indicate breaking changes
- `Co-authored-by:` — credit collaborators

## three ways to create commits

### method 1: interactive mode (best for learning)

perfect for beginners or when you want guided input:

```bash
cocoa commit
```

you'll be prompted for each part of the commit.

### method 2: AI-powered generation (best for speed)

let AI draft your commit from your changes:

```bash
# stage your changes
git add .

# generate commit
cocoa generate
```

i'll analyze staged file changes, current branch name, recent commits, and your configuration.

then, i'll suggest something like:

```gitcommit
feat(database): add user caching with Redis

implement redis-backed caching for user queries to reduce database load.
cache entries expire after 1 hour and are invalidated on user updates.

Closes #234
```

edit if needed, then approve to commit!

**note:** this requires some API key from OpenAI, Anthropic, or another compatible provider. see
[AI providers](./ai-providers.md).

### method 3: command line (best for scripts)

for automation or if you prefer typing:

```bash
git add .
git commit -m "feat(auth): add password reset"
```

or with body and footer:

```bash
git commit -m "feat(auth): add password reset" -m "users can reset forgotten passwords via email link." -m "Closes #142"
```

**note:** i'll still lint your message if hooks are installed.

## real-world examples

### fixing a bug

```gitcommit
fix(payment): prevent duplicate charge on retry

When a payment request times out and the user retries, the system
was attempting to charge twice. Now we use idempotency keys to
ensure only one charge occurs per transaction.

Tested with manual retry scenario. All payment tests pass.

Fixes #567
```

### adding a feature

```gitcommit
feat(notifications): add email digest

users can now opt into daily/weekly email digests of their
notifications instead of receiving individual emails. digests
are sent at 9 AM local time.

includes:
- digest scheduling logic
- email template for digest format
- user preference settings UI
- tests covering edge cases (timezone changes, daylight saving)

Closes #123
```

### deprecating API

```gitcommit
feat(api): deprecate v1 endpoints

the v1 API endpoints are now deprecated in favor of v2. they will
be removed in v3.0.0 (6 months from release).

all v1 endpoints now return a deprecation warning in the response
headers and log a warning server-side.

migration guide: https://docs.example.com/migration-v1-to-v2

BREAKING CHANGE: v1 endpoints will be removed in v3.0.0
```

## tips and tricks

### imperative mood

write subjects as commands, like you're telling git what to do:

good: `add user validation` (imperative)

bad: `added user validation` (past tense)

bad: `adds user validation` (present tense)

this matches git's own commit messages (`Merge branch...`, `Revert...`).

### commit often, refine later

make commits as you work, even if they're not perfect. if you need to change commits later, you can:

- amend: `git commit --amend`
- fixup: `git commit --fixup`
- rebase: `git rebase -i` (interactive rebase)

safer than one giant commit at the end!

### reference related work

link commits to issues:

```
Closes #123        # auto-closes the issue
Refs #123         # references without closing
See #123          # mentions for context
```

GitHub and GitLab will auto-link these!

### break complex changes into multiple commits

instead of one large commit:

```
feat(auth): implement oauth, 2fa, password reset, account linking
```

split into multiple:

```
1. feat(auth): add oauth provider integration
2. feat(auth): add two-factor authentication (TOTP)
3. feat(auth): add password reset flow
4. feat(auth): add account linking
```

each commit is:

- easier to review
- easier to revert if needed
- easier to bisect for bugs
- better for changelog

### use scopes for team scale

large teams benefit from consistent scopes:

```example
team agreement:
- frontend scopes: ui, components, pages, styling
- backend scopes: api, database, auth, workers
- devops scopes: ci, deploy, infra, docker
```

these can be configured in `.cocoa.toml`:

```toml
[commit]
scopes = [
  "ui", "components", "pages", "styling",
  "api", "database", "auth", "workers",
  "ci", "deploy", "infra", "docker"
]
```

then i can autocomplete and verify scopes!

## linting your commits

after creating a commit, i can check it:

```bash
cocoa lint          # check latest commit
cocoa lint HEAD~2   # check older commit
```

or if hooks are installed, i can automatically validate before each commit is made. ^u^

see [linting commits](./linting-commits.md) for details.

## next steps

- **set up AI:** [AI Providers](./ai-providers.md)
- **validate commits:** [Linting Commits](./linting-commits.md)
- **automate validation:** [Git Hooks](./git-hooks.md)
- **generate releases:** [Version Management](./versioning.md)
