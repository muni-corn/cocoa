# Version management

Harness the powers of semantic or calendar versioning with cocoa!

## Versioning strategies

### Semantic versioning (SemVer)

Version format: `MAJOR.MINOR.PATCH`

- **MAJOR** (1.0.0 → 2.0.0): breaking changes
- **MINOR** (1.0.0 → 1.1.0): new features (backward compatible)
- **PATCH** (1.0.0 → 1.0.1): bug fixes only

**Best for:** libraries, APIs, tools where compatibility matters.

### Calendar versioning (CalVer)

Version format: `YYYY.MM.PATCH` or custom patterns.

**Best for:** applications with regular release cadences.

## Bumping versions

### Automatic bumping

Cocoa analyzes commits since the last version tag and bumps appropriately:

```bash
cocoa bump auto
```

| Commits found    | Version change | Example       |
| ---------------- | -------------- | ------------- |
| Breaking changes | major          | 1.0.0 → 2.0.0 |
| Features (feat:) | minor          | 1.0.0 → 1.1.0 |
| Fixes (fix:)     | patch          | 1.0.0 → 1.0.1 |
| Chores/docs only | patch          | 1.0.0 → 1.0.1 |

### Manual bumping

Force a specific bump:

```bash
cocoa bump major    # 1.0.0 → 2.0.0
cocoa bump minor    # 1.0.0 → 1.1.0
cocoa bump patch    # 1.0.0 → 1.0.1
```

## Full release workflow

```bash
# Preview without making any changes
cocoa release --dry-run

# Run the full release
cocoa release

# Push to remote
git push --all --tags
```

`cocoa release` does everything automatically:

1. Analyzes commits since the last tag.
2. Decides the version bump (major/minor/patch).
3. Updates version files (see below).
4. Generates the changelog.
5. Creates a git commit with all changes.
6. Creates an annotated git tag.

## Version files

Cocoa can update version strings in your files as part of every release. There are two ways to
configure this.

### Simple form (`commit_version_files`)

List file paths and cocoa will update them automatically. The handler is chosen by file basename:

```toml
[version]
commit_version_files = ["Cargo.toml", "Cargo.lock", "package.json"]
```

**Auto-detected handlers by basename:**

| File basename       | Handler                                                            |
| ------------------- | ------------------------------------------------------------------ |
| `Cargo.toml`        | Structured: updates `[package].version` only                       |
| `Cargo.lock`        | Workspace-aware: updates only workspace member entries             |
| `package.json`      | Structured: updates top-level `"version"` only                     |
| `package-lock.json` | Structured: updates root entry only, not `node_modules/*`          |
| `pnpm-lock.yaml`    | Command: runs `pnpm install --lockfile-only`                       |
| `yarn.lock`         | Command: runs `yarn install --mode=update-lockfile`                |
| `pyproject.toml`    | Structured: updates `[project].version` or `[tool.poetry].version` |
| Any other file      | Plain: replaces every occurrence of the version string             |

### Rich form (`[[version.files]]`)

For precise control, use the `[[version.files]]` array:

```toml
[[version.files]]
path = "Cargo.toml"
kind = "cargo"          # optional; auto-detected from basename if omitted

[[version.files]]
path = "Cargo.lock"
kind = "cargo-lock"

[[version.files]]
path = "package.json"
kind = "npm"

[[version.files]]
path = "package-lock.json"
kind = "npm-lock"
```

The `kind` field accepts: `auto` (default), `cargo`, `cargo-lock`, `npm`, `npm-lock`, `pnpm-lock`,
`yarn-lock`, `pyproject`, `regex`, `plain`.

Both forms can coexist. Rich `[[version.files]]` entries take precedence for any path they declare;
`commit_version_files` entries fill in the rest.

### Regex handler

For files cocoa doesn't natively understand (badges in READMEs, helm charts, install scripts, etc.)
use `kind = "regex"`:

```toml
[[version.files]]
path = "README.md"
kind = "regex"
pattern = 'cocoa = "(?P<v>[^"]+)"'
occurrences = "first"   # or "all" or a number; defaults to "first"
```

The pattern must contain exactly one named capture group `v`. Cocoa replaces only the content of
that group, leaving all surrounding context untouched.

```toml
[[version.files]]
path = "helm/Chart.yaml"
kind = "regex"
pattern = 'appVersion: "(?P<v>[^"]+)"'
```

### Plain handler

Use `kind = "plain"` to force the historical behavior: all occurrences of the old version string are
replaced with the new one.

```toml
[[version.files]]
path = "src/version.rs"
kind = "plain"
occurrences = 1   # limit to the first match (optional)
```

### Command strategy

For any file, you can shell out to a toolchain command instead of editing in-process. This is the
default for `pnpm-lock.yaml` and `yarn.lock`:

```toml
[[version.files]]
path = "Cargo.lock"
kind = "cargo-lock"
strategy = "command"
command = ["cargo", "update", "--workspace"]
```

The command runs in the repository root after the manifest version has already been updated.

Available strategies: `in-process` (default), `command`, `skip`.

### Global toolchain preferences

Override the default lockfile strategy for a whole toolchain without listing every file:

```toml
[version.toolchains]
cargo  = { lockfile = "command" }   # use cargo update --workspace
npm    = { lockfile = "skip" }      # do not touch package-lock.json
pnpm   = { lockfile = "command" }
```

## Cargo workspace example

A Rust workspace with multiple crates:

```toml
# .cocoa.toml
[version]
strategy = "semver"
tag_prefix = "v"

[[version.files]]
path = "Cargo.toml"
kind = "cargo"        # updates [workspace.package].version

[[version.files]]
path = "Cargo.lock"
kind = "cargo-lock"   # updates only workspace member entries
```

After `cocoa release`:

- `Cargo.toml` → only `[workspace.package].version` changes.
- `Cargo.lock` → only `[[package]]` entries whose `name` matches a workspace member change.
  Transient dependencies are untouched.

## npm / Node.js example

```toml
[version]
[[version.files]]
path = "package.json"
kind = "npm"

[[version.files]]
path = "package-lock.json"
kind = "npm-lock"   # updates root "version" and packages[""].version only
```

## Opting out of auto-detection

If you need the historical "replace everything" behavior for a file that would otherwise be
auto-detected (for example, a `Cargo.toml` that you maintain manually), use `kind = "plain"`:

```toml
[[version.files]]
path = "Cargo.toml"
kind = "plain"   # disables structured handler; replaces all occurrences
```

## Tips

### Preview before releasing

```bash
# Show what would change without touching any files
cocoa release --dry-run
```

### Sign releases (GPG)

```toml
[version]
sign_tags = true
```

### Custom tag prefix

```toml
[version]
tag_prefix = "release-"   # creates release-1.0.0, release-1.1.0, etc.
```

## Next steps

- **Automate releases:** [CI/CD integration](./ci-cd.md)
- **Better commits:** [creating commits](./creating-commits.md)
- **Generate changelogs:** [changelogs](./changelogs.md)
- **Configure everything:** [configuration](./configuration.md)
