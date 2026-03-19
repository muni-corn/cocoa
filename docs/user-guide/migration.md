# migration guide

i'll be happy to replace your other tools~ >:3

## migrating from Commitlint

Commitlint validates commit messages against conventional commit rules.

with me, you'll also get:

- commit generation (interactive and AI-powered)
- changelog generation
- version management
- a single unified tool

### configuration migration

**Commitlint config:**

```js
// commitlintrc.js
module.exports = {
  extends: ["@commitlint/config-conventional"],
  rules: {
    "type-enum": [2, "always", ["feat", "fix", "docs"]],
    "subject-case": [2, "always", "lowercase"],
  },
};
```

**`cocoa` equivalent:**

```toml
# .cocoa.toml
[commit]
types = ["feat", "fix", "docs"]

[commit.rules.deny]
no_type = true
subject_case = "lowercase"
```

### migration steps

1. **create `.cocoa.toml`:**

```bash
cocoa init
# or manually create based on your commitlint config
```

2. **install `cocoa` hooks:**

```bash
cocoa hook
```

3. **update CI/CD:**

```yaml
# old
- run: npx commitlint --from origin/main

# new
- run: cocoa lint origin/main...HEAD
```

4. once everything works okay, **remove commitlint:**

```bash
npm uninstall commitlint
rm commitlintrc.js
```

## migrating from Conventional Changelog

Conventional Changelog generates changelogs from conventional commits.

with me, you also get:

- simpler configuration
- better integration with versioning
- AI-powered commit generation
- one single tool instead of multiple

### configuration migration

**.changelog.json:**

```json
{
  "types": [
    { "type": "feat", "section": "Features" },
    { "type": "fix", "section": "Bug fixes" }
  ]
}
```

**.cocoa.toml:**

```toml
[changelog.sections]
feat = "Features"
fix = "Bug fixes"
```

### migration steps

1. **create `.cocoa.toml`:**

```bash
cocoa init
# configure changelog sections to match
```

2. **generate new changelog:**

```bash
cocoa changelog > CHANGELOG_NEW.md
# compare with old CHANGELOG.md
```

3. **update CI/CD:**

```yaml
# old
- run: npx conventional-changelog -p angular -i CHANGELOG.md -s

# new
- run: cocoa changelog
```

4. once everything works correctly, **remove the old tool:**

```bash
npm uninstall conventional-changelog
rm .changelog.json
# all changelog tasks now use cocoa
```

## migrating from Semantic Release

Semantic Release automates versioning and releases.

with me, you'll also get:

- simpler, self-contained
- better control over process
- AI-powered commit generation
- no complex plugin system

### configuration migration

**.releaserc:**

```json
{
  "branches": ["main"],
  "plugins": [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    "@semantic-release/changelog",
    "@semantic-release/npm",
    "@semantic-release/git",
    "@semantic-release/github"
  ]
}
```

**.cocoa.toml:**

```toml
[commit]
types = ["feat", "fix", "chore"]

[changelog]
output_file = "CHANGELOG.md"

[version]
strategy = "semver"
tag_prefix = "v"
sign_tags = true
commit_version_files = ["package.json"]
```

### migration steps

1. **create `.cocoa.toml`**

```bash
cocoa init
# configure to match your semantic-release setup
```

2. **test release process**

```bash
cocoa release --dry-run
# preview what would happen
```

3. **update CI/CD**

```yaml
# Old
- run: npx semantic-release

# New
- run: cocoa release
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

4. once everything looks okay, **remove semantic-release:**

```bash
npm uninstall semantic-release
rm .releaserc
```
