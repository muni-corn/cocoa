# git hooks integration

automate commit validation with git hooks!

## what's a git hook?

git hooks are scripts that run automatically when you perform git operations. i provide a
`commit-msg` hook that validates every commit before it's created.

**without hooks:**

```bash
git commit -m "wip"
# commit succeeds (bad D:)
```

**with hooks:**

```bash
git commit -m "wip"
# hook runs, validation fails
# commit is rejected (good!)
```

## installing hooks

### one-command installation

```bash
cocoa hook
```

this installs my validation hook into `.git/hooks/commit-msg`.

verify installation:

```bash
ls -la .git/hooks/commit-msg
# should exist and be executable
```

### what gets installed

i'll install a hook that:

- reads your commit message
- loads your `.cocoa.toml` configuration
- validates against your rules
- shows warnings or errors
- allows or rejects the commit

## using hooks

once installed, hooks run automatically:

### successful commit

```bash
git add .
git commit -m "feat(auth): add password reset"
```

```gitcommit
# hook validates and succeeds
# commit is created
[main abc1234] feat(auth): add password reset
```

### failed commit

```bash
git commit -m "wip"

# hook validates and fails
# error shown
commit message validation failed:
- missing commit type (feat, fix, chore, etc.)

# commit is rejected, try again
```

### amending failed commits

if your commit fails validation, fix it and try again:

```bash
git commit -m "wip"
# validation fails

# fix the message
git commit -m "feat: implement new feature"
# validation passes
```

or amend the last commit:

```bash
git commit --amend
# edit the message in your editor
# hook validates the new message
```

## uninstalling hooks

remove the hook when you no longer want automatic validation:

```bash
cocoa hook --uninstall
```

or manually:

```bash
rm .git/hooks/commit-msg
```

## hook behavior

### what hooks validate

hooks use your configuration from `.cocoa.toml`:

```toml
[commit.rules]
enabled = true

[commit.rules.deny]
no_type = true              # enforce type
subject_length = 72
```

hooks will:

- check commit type exists
- check subject length
- check body length
- check scope validity
- run custom regex patterns
- etc.

### what hooks ignore

hooks can skip certain commits:

```toml
[commit.rules]
ignore_fixup_commits = true     # git commit --fixup
ignore_amend_commits = true     # git commit --amend
ignore_squash_commits = true    # git commit --squash
ignore_merge_commits = true     # merge commits
ignore_revert_commits = true    # git revert
```

these commits skip validation entirely.

## troubleshooting

### "hook not found"

hooks aren't installed.

**solution:**

```bash
cocoa hook
# installs the hook

# verify
ls .git/hooks/commit-msg
```

### "permission denied" when committing

```bash
git commit -m "feat: test"
# permission denied: .git/hooks/commit-msg
```

**cause:** hook isn't executable.

**solution:**

```bash
chmod +x .git/hooks/commit-msg
git commit -m "feat: test"
```

### "hook seems outdated"

if you update me, reinstall hooks:

```bash
cocoa hook --uninstall
cocoa hook
```

### bypassing hooks temporarily

sometimes you need to skip validation (not recommended!!):

```bash
# skip git hooks for ONE commit
git commit -m "wip" --no-verify

# WARNING: this disables all git hooks!
```

use `--no-verify` only when absolutely necessary.

### hooks don't run in different shells

if you work in multiple shells (bash, zsh, fish), the hook should still work:

```bash
# these all use the installed hook
bash -c "git commit -m 'feat: test'"
zsh -c "git commit -m 'feat: test'"
fish -c "git commit -m 'feat: test'"
```

## team setup

### sharing hooks across team

hooks are stored in `.git/`, which is not shared. to standardize hooks:

**option 1: install script**

create `scripts/install-hooks.sh`:

```bash
#!/bin/bash
cocoa hook
echo "hooks installed"
```

team runs:

```bash
./scripts/install-hooks.sh
```

**option 2: setup instructions**

add to your README:

```markdown
## development setup

1. clone repo
2. install dependencies
3. set up git hooks: cocoa hook
```

**option 3: git config local**

configure hooks path (Git 2.9+):

```bash
git config core.hooksPath .githooks
cocoa hook --path .githooks
```

commit `.githooks/` to git:

```bash
git add .githooks/
git commit -m "docs: add git hooks"
```

now all clones automatically use them!

## hook configuration

### disable hooks for one command

```bash
git -c core.hooksPath=/dev/null commit -m "wip"
# hook doesn't run
```

### custom hook path

by default, i use `.git/hooks/commit-msg`.

to use a different location:

```bash
cocoa hook --path /custom/path/commit-msg
```

then configure git:

```bash
git config core.hooksPath /custom/path
```

### multiple hooks

if you have other hooks (pre-commit, etc.), they coexist:

```bash
.git/hooks/
  pre-commit       # runs before staging
  commit-msg       # my hook (runs after message entered)
  pre-push         # runs before push
```

each runs independently!

## CI/CD vs local hooks

### local hooks

hooks on your machine validate locally:

```bash
git commit -m "wip"
# fails locally, doesn't push
```

good for:

- developer feedback
- catching issues early
- preventing bad commits

### CI/CD validation

CI also validates:

```bash
cocoa lint main...HEAD
# validates all commits in PR
```

good for:

- team standards
- preventing bad merges
- auditing all work

### best practice

use both:

1. local hooks catch issues before push
2. CI/CD ensures nothing slips through

```
developer -> local hook -> push -> CI/CD -> merge
```

## advanced: customizing hook behavior

### view hook content

```bash
cat .git/hooks/commit-msg
```

shows the installed hook script.

### modify hook

you can edit the hook script, but:

- updates to me won't update your hook
- better to uninstall and reinstall

```bash
cocoa hook --uninstall
cocoa hook
```

## performance

hooks add minimal overhead:

```bash
git commit -m "feat: test"
# typically adds <100ms
```

if hooks are slow:

1. check network (if loading remote config)
2. check disk I/O
3. reduce validation rules in `.cocoa.toml`

## frequently asked questions

**Q: do hooks slow down my workflow?** A: no, validation is very fast (<100ms per commit).

**Q: can i disable hooks temporarily?** A: yes: `git commit --no-verify` (use sparingly!)

**Q: do hooks work in CI/CD?** A: hooks are local. use `cocoa lint` in CI instead.

**Q: what if i'm using pre-commit?** A: my hooks coexist with pre-commit. both run independently.

**Q: can i use my hooks and commitlint?** A: not together (they conflict). choose one.

## next steps

- learn linting: [linting commits](./linting-commits.md)
- use in CI: [CI/CD integration](./ci-cd.md)
- configure rules: [configuration](./configuration.md)
