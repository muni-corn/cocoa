# version management

harness the powers of semantic or calendar versioning with me!

## versioning strategies

### semantic versioning (SemVer)

version format: `MAJOR.MINOR.PATCH`

- **MAJOR** (1.0.0 → 2.0.0) — breaking changes
- **MINOR** (1.0.0 → 1.1.0) — new features (backward compatible)
- **PATCH** (1.0.0 → 1.0.1) — bug fixes only

**examples:**

```
1.0.0  → 1.0.1  (bug fix)
1.0.0  → 1.1.0  (new feature)
1.0.0  → 2.0.0  (breaking change)
```

**best for:** libraries, APIs, tools where compatibility matters.

### calendar versioning (CalVer)

version format: `YYYY.MM.PATCH` or custom patterns

- **year:** `2024`
- **month:** `03`
- **patch:** sequential number

**examples:**

```
2024.01.0  → 2024.01.1  (patch within month)
2024.01.0  → 2024.02.0  (new month = new release)
```

**best for:** applications, products with regular release cadences.

## Configure Your Strategy

In `.cocoa.toml`:

### Semantic Versioning

```toml
[version]
strategy = "semver"
tag_prefix = "v"
sign_tags = true
commit_version_files = ["package.json", "Cargo.toml"]
```

### Calendar Versioning

```toml
[version]
strategy = "calver"
calver_format = "YYYY.MM.0"  # Year.Month.Patch
tag_prefix = "v"
sign_tags = true
```

## bumping versions

### automatic bumping

i analyze commits since last version and bump appropriately:

```bash
cocoa bump auto
```

i look at commits and decide:

| commits found    | version change | example       |
| ---------------- | -------------- | ------------- |
| breaking changes | major          | 1.0.0 → 2.0.0 |
| features (feat:) | minor          | 1.0.0 → 1.1.0 |
| fixes (fix:)     | patch          | 1.0.0 → 1.0.1 |
| chores/docs only | patch          | 1.0.0 → 1.0.1 |

```bash
# example scenario:
# current version: 1.0.0
# commits since 1.0.0:
#   feat(auth): add password reset
#   feat(ui): redesign dashboard
#   fix(api): timeout issue
#   BREAKING CHANGE: API v1 removed

$ cocoa bump auto
✓ detected breaking changes → bumping major
✓ updating to version 2.0.0
```

### manual bumping

force a specific bump:

```bash
cocoa bump major    # 1.0.0 → 2.0.0
cocoa bump minor    # 1.0.0 → 1.1.0
cocoa bump patch    # 1.0.0 → 1.0.1
```

### pre-release versions

```bash
cocoa bump patch --pre-release alpha    # 1.0.0 → 1.0.1-alpha
cocoa bump patch --pre-release beta     # 1.0.1-alpha → 1.0.1-beta
cocoa bump patch --pre-release rc       # 1.0.1-beta → 1.0.1-rc
cocoa bump patch                         # 1.0.1-rc → 1.0.1 (release)
```

great for testing releases before final push!

## creating release tags

### create a tag

```bash
cocoa tag
```

this does the following:

1. reads current version from files or tags
2. generates changelog for this version
3. creates an annotated git tag with changelog
4. signs tag (if `sign_tags = true`)

output:

```
$ cocoa tag
creating tag v1.2.0
changelog:
    - add password reset (feat)
    - fix timeout issue (fix)
tag created and signed
```

### tag already exists?

```bash
cocoa tag --force
```

**warning:** only do this if you haven't pushed yet!

### push tags

```bash
git push --tags
```

or push everything:

```bash
git push --all --tags
```

## full release workflow

### one-command release

```bash
cocoa release

# output:
detected 1 fix
bumping: 1.5.2 → 1.5.3 (PATCH)
creating tag v1.5.3
ready to push
```

does everything automatically:

1. analyzes commits since last tag
2. decides version bump (major/minor/patch)
3. updates version files
4. generates changelog
5. creates git commit with changes
6. creates signed git tag
7. ready to push!

output:

```
$ cocoa release
bumping version: 1.0.0 → 1.1.0
updated: package.json, Cargo.toml
generating changelog...
committing changes
creating tag v1.1.0
ready to push! run: git push --all --tags
```

now push to remote:

```bash
git push --all --tags
```

### step-by-step release

for more control, preview before releasing:

```bash
# 1. check what would happen
cocoa release --dry-run

# 2. review the output (version, files, changelog)

# 3. actually release
cocoa release

# 4. push when ready
git push --all --tags
```

## version files

i automatically update version in your files:

```toml
[version]
commit_version_files = [
  "package.json",
  "Cargo.toml",
  "pyproject.toml",
  "src/version.rs"
]
```

### supported formats

JSON (package.json):

```json
{
  "version": "1.0.0"
}
```

↓ (after bump)

```json
{
  "version": "1.1.0"
}
```

TOML (Cargo.toml):

```toml
[package]
version = "1.0.0"
```

↓ (after bump)

```toml
[package]
version = "1.1.0"
```

Python (pyproject.toml):

```toml
[project]
version = "1.0.0"
```

↓ (after bump)

```toml
[project]
version = "1.1.0"
```

Rust (src/version.rs):

```rust
pub const VERSION: &str = "1.0.0";
```

↓ (after bump)

```rust
pub const VERSION: &str = "1.1.0";
```

### add custom files

add any file where version appears:

```toml
[version]
commit_version_files = [
  "package.json",
  "docs/version.txt",
  "README.md",
  "_config.yml"
]
```

i'll find and replace version strings!

## Real-World Release Examples

### Example 1: Simple Bug Fix

```bash
# Make and commit fixes
git add .
git commit -m "fix(api): resolve timeout issue"

# Release
cocoa release

# Output:
✓ Detected 1 fix
✓ Bumping: 1.5.2 → 1.5.3 (PATCH)
✓ Creating tag v1.5.3
✓ Ready to push
```

### Example 2: Feature Release

```bash
# Make and commit features
git add .
git commit -m "feat(auth): add password reset"
git commit -m "feat(dashboard): redesign UI"

cocoa release

# Output:
Detected 2 features, 0 breaking changes
Bumping: 1.5.2 → 1.6.0 (MINOR)
Creating tag v1.6.0
Ready to push
```

### Example 3: Major Release (Breaking Change)

```bash
# Make and commit changes
git add .
git commit -m "refactor(api): redesign endpoints

BREAKING CHANGE: Old /auth endpoints removed in favor of /api/v2/auth"

cocoa release

# Output:
Detected 1 breaking change
Bumping: 1.5.2 → 2.0.0 (MAJOR)
Creating tag v2.0.0
Ready to push
```

## Pre-Release Workflow

Release a beta/alpha/RC version:

```bash
# 1. Create a pre-release tag
cocoa tag --pre-release beta

# Tags: v2.0.0-beta, v2.0.0-beta.1, v2.0.0-beta.2, etc.

# 2. Test extensively
# ... run tests, manual testing, etc.

# 3. Release final version
cocoa bump patch        # v2.0.0-beta → v2.0.0
cocoa tag
git push --tags
```

## Troubleshooting

### "No version found"

```bash
cocoa release
# Error: No version tag found in repository
```

**Cause:** No tags exist yet.

**Solution:** Create first tag manually:

```toml
# In .cocoa.toml, update version first
```

Then:

```bash
cocoa tag
```

Or create manually:

```bash
git tag -a v1.0.0 -m "Initial release"
git push --tags
```

### "Version not updated in files"

```bash
cocoa release
# Says it updated, but files didn't change
```

**Cause:** File path is wrong or format isn't recognized.

**Debug:**

```bash
# Check what cocoa sees
cocoa release --verbose

# Verify file exists
cat package.json | grep version
```

**Fix:** Update paths in `.cocoa.toml`:

```toml
[version]
commit_version_files = ["package.json", "src/lib.rs"]  # Correct paths
```

### "Can't push because local tags differ"

```bash
git push --tags
# ERROR: refs/tags/v1.2.0 exists but has different object id
```

**Cause:** Tag was created locally but pushed differently.

**Solution:** Use `--force` carefully!

```bash
# Delete local tag
git tag -d v1.2.0

# Pull from remote
git fetch origin refs/tags/*:refs/tags/*

# Or force if you own the repo
git push --tags --force
```

## Tips and Tricks

### 1. Automate in CI

Release automatically on tag push:

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags:
      - "v*"

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo install cocoa
      - run: cocoa release
      - run: git push
```

### 2. Validate Before Release

```bash
# Preview release without committing
cocoa release --dry-run

# Check what version would be bumped
cocoa bump auto --verbose

# Lint recent commits
cocoa lint HEAD~10...HEAD
```

### 3. Sign Releases (GPG)

```toml
[version]
sign_tags = true
```

Requires GPG key setup. Tags are cryptographically signed!

### 4. Custom Tag Prefix

```toml
[version]
tag_prefix = "release-"
```

Creates tags: `release-1.0.0`, `release-1.1.0`, etc.

### 5. Track Multiple Version Formats

Some projects need multiple version files:

```toml
[version]
commit_version_files = [
  "package.json",         # npm version
  "package-lock.json",    # npm lockfile
  "Cargo.toml",           # Rust version
  "docs/CHANGELOG.md",    # Documentation version
]
```

all updated atomically!

## next steps

- **automate releases:** [CI/CD integration](./ci-cd.md)
- **better commits:** [creating commits](./creating-commits.md)
- **generate changelogs:** [changelogs](./changelogs.md)
- **configure everything:** [configuration](./configuration.md)
