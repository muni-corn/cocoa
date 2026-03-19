# troubleshooting guide

as domesticated as i seem, i'm still a wild zebra sometimes.

hopefully this guide helps us to resolve some issues you might run into with me.

## installation issues

### "cocoa: command not found"

help! i'm lost! D:

please help your system to find me!

#### option 1: add `~/.cargo/bin` to `$PATH`

```bash
# add to ~/.bashrc or ~/.zshrc
export PATH="$PATH:$HOME/.cargo/bin"

# reload
source ~/.bashrc
cocoa --version
```

#### option 2: figure out where i was installed

```bash
find ~ -name cocoa -type f 2>/dev/null
# move that file to /usr/local/bin or add to PATH
```

#### option 3: reinstall me

```bash
cargo install cocoa
```

### "error: could not compile 'cocoa'"

if you're trying to build me from scratch, this may be due to missing dependencies.

first and foremost, i recommend using `devenv` if you want the best guarantee of building me
correctly.

```bash
devenv shell
```

or, just

```bash
devenv build
```

### "permission denied" after installation

this means i'm not allowed to run. :(

i'd love to just sit around all day, but i'd love to help you out even more.

```bash
chmod +x $(which cocoa)
cocoa --version
```

## configuration issues

### "could not load configuration"

does your configuration file have syntax errors?

**validate TOML:**

```bash
# Install TOML validator
cargo install toml-cli

# Check your config
toml-cli check .cocoa.toml
```

**Common TOML errors:**

```toml
# missing quotes around strings
types = [feat, fix]  # wrong

types = ["feat", "fix"]  # yay!

# unmatched brackets
[commit
types = ["feat"]  # wrong

[commit]
types = ["feat"]  # yippee!

# invalid arrays
scopes = "auth, api"  # wrong

scopes = ["auth", "api"]  # happiness!
```

### "type 'feature' not recognized"

perhaps the type name isn't included in your config. check your `.cocoa.toml` for what types you
have configured.

or, you can update your config:

```toml
[commit]
types = ["feature", "fix"]
```

### "no configuration found"

you skipped the [quick start guide](./quick-start.md), didn't you?

that's okay! i don't take it personally.

**let's check if a config exists:**

```bash
# in project root?
ls .cocoa.toml

# in home directory?
ls ~/.config/cocoa/cocoa.toml

# or, run with `--verbose` and i'll tell you where i'm getting config options from:
cocoa --verbose commit
```

**solution:**

create config in project root:

```bash
cocoa init
```

copy existing config:

```bash
cp ~/.config/cocoa/cocoa.toml .cocoa.toml
```

## commit issues

### "commit validation failed"

i'm telling you your commit message doesn't pass the rules you configured.

with `cocoa lint`, i'll tell you which rules are being violated.

### "commit already created (hook too late)"

the git hook didn't stop the commit.

is my git hook executable? try:

```bash
chmod +x .git/hooks/commit-msg
```

## linting issues

### "no commits found"

the range you specified with `cocoa lint` doesn't have any commits.

commands like this only check a single commit:

```bash
cocoa lint main       # lints single commit, not range
cocoa lint HEAD~5     # lints single commit
```

these commands will check ranges:

```bash
cocoa lint main...HEAD           # commits in this branch
cocoa lint origin/main...HEAD    # compared to main

cocoa lint -n 5                  # last 5 commits
cocoa lint HEAD~5..HEAD          # 5 commits from HEAD
```

### "regex pattern failed"

a custom regex you configured has syntax errors.

**test your regex:**

```bash
# install a regex tester
echo "feat(auth): add login" | grep -E "^(feat|fix)"

# if no output, regex failed
```

**fix the regex in your config:**

```toml
# wrong (unclosed parenthesis)
regex_patterns = ["^(feat|fix"]

# right!
regex_patterns = ["^(feat|fix)"]
```

### "pattern doesn't match anything"

maybe your regex is too strict?

i'll tell you which patterns matched and/or failed with:

```bash
cocoa lint --verbose
```

## AI and generation issues

### "API key not found"

ensure your environment variables are set correctly _and_ that i know to use them.

### "Rate limit exceeded"

you're making too many API calls and your AI provider needs a small break.

it's probably time for you to take a break, too! get some water, step outside for a bit, take some
time for self-care. you deserve it, you busy bee~!

## changelog issues

### "no commits since last version"

i've addressed all commits since your last release!

the solution: make more commits!

```bash
git add .
git commit -m "chore: make a new commit so cocoa will do things"
cocoa changelog
# should show the new feature
```

### "changelog looks empty"

commits aren't matching your config.

**check commit types:**

```bash
# list recent commits
git log --oneline | head -10

# check types match config
cat .cocoa.toml | grep -A 15 "\[changelog.sections\]"
```

**examples:**

config says:

```toml
[changelog.sections]
feat = "Features"
fix = "Bug fixes"
```

but commits have:

```bash
git log --grep "feature"  # wrong type!
git log --grep "feat"     # right type
```

**solutions:**

you may need to add the missing sections to your config:

```toml
[changelog.sections]
chore = "Maintenance"  # add if you have chore commits
```

or exclude unwanted types:

```toml
[changelog]
exclude_types = ["chore", "test"]
```

### "changelog not updating"

possibly a config issue or missing output file.

**check config:**

```bash
cat .cocoa.toml | grep output_file
# should show: output_file = "changelog.md"
```

check that the file exists:

```bash
ls CHANGELOG.md
```

if not, create it:

```bash
touch CHANGELOG.md
```

**check permissions:**

```bash
ls -la CHANGELOG.md
# should be writable by current user
```

## version/release issues

### "no version found"

there aren't any existing tags in your git commit history yet.

**create first version:**

```bash
# check current version in files
cat package.json | grep version

# create first tag
git tag -a v1.0.0 -m "Initial release"
cocoa bump auto
```

### "version not updating in files"

maybe the file paths are wrong or the format is unsupported.

**Check config:**

```bash
cat .cocoa.toml | grep -A 5 "commit_version_files"
```

**Verify files exist:**

```bash
cat package.json  # should exist and have version
cat Cargo.toml    # should exist and have version
```

**`cocoa` only supports these formats:**

```json
// package.json
{ "version": "1.0.0" }
```

```toml
# Cargo.toml or pyproject.toml
[project]
version = "1.0.0"
```

```rust
// src/version.rs
pub const VERSION: &str = "1.0.0";
```

### "can't push: tag already exists"

maybe a tag exists locally but with different content.

```bash
git tag -l | grep v1.0.0
# tag exists locally

git ls-remote origin v1.0.0
# tag exists on remote
```

#### solution 1: use different tag

```bash
cocoa tag --force
git push --tags
```

#### solution 2: delete and recreate

**_only if you haven't pushed!_**

```bash
git tag -d v1.0.0
git push origin :refs/tags/v1.0.0
cocoa tag
git push --tags
```

#### option 3: force push if you know what you're doing

**_be careful!! this can be destructive, and it can make your team upset!_**

```bash
git push --tags --force
```

### "git operation failed"

there was a `git` error during release.

#### problem 1: dirty working directory

```bash
git status
# should show "working tree clean"

# if dirty:
git add .
git commit -m "chore: add such and such"
```

#### problem 2: branch tracking issue

```bash
git branch -vv
# should show origin/main tracking

git branch --set-upstream-to=origin/main main
```

#### problem 3: permission issues

```bash
# check git credentials
git config user.name
git config user.email

# if missing, set them
git config --global user.name "Your Name"
git config --global user.email "you@example.com"
```

## platform-specific issues

### macOS: "security will ask for password"

Keychain is asking for permission.

**solution:** allow it permanently

- when prompted, click "Always Allow"
- or disable keychain for git: `git config --global credential.helper osxkeychain`

### Windows: "command not recognized"

PowerShell can't find me. ugh.

**solutions:**

1. restart terminal after installation
2. run PowerShell as Administrator
3. check PATH: `$env:PATH` should include `%APPDATA%\.cargo\bin`

### Linux: "permission denied" on git push

perhaps you don't have SSH keys configured?

**setup SSH:**

```bash
ssh-keygen -t ed25519
cat ~/.ssh/id_ed25519.pub  # Copy to GitHub/GitLab
git remote set-url origin git@github.com:user/repo.git
```

## getting help

### enable verbose output

you can see my thought process and what i'm doing:

```bash
cocoa --verbose commit
# shows more details
```

### check logs

```bash
# some commands output to stderr
cocoa generate 2>&1 | tee output.log
cat output.log
```

### run dry-run mode

you can preview actions without making changes:

```bash
cocoa --dry-run release
# shows what would happen
```

### debug specific command with structured output

```bash
cocoa --verbose --dry-run release --json > debug.json
cat debug.json  # structured output
```

### ask for help

- check existing issues: https://github.com/muni-corn/cocoa/issues
- search documentation
- create a new issue with:
  - output of `cocoa --version`
  - your `.cocoa.toml` (don't include secrets, you goof!)
  - the exact error message
  - steps to reproduce
