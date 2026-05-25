# Lockfile handler design rationale

Notes on the design decisions behind cocoa's lockfile version update system, for future maintainers
and contributors.

## The original problem

The original `update_version_files` function used `str::replace` to replace every occurrence of the
old version string in each listed file. This caused problems with lockfiles:

- `Cargo.lock` contains `[[package]]` entries for every transient dependency, not just workspace
  members. If a transient dependency happened to share the same version string (e.g. `"0.2.2"`), it
  would be incorrectly updated.
- Similarly, `package-lock.json` contains nested `version` fields for every installed
  `node_modules/*` package.

## Solution: structured handlers

Instead of plain text replacement, cocoa now dispatches each file to a dedicated handler based on
its format. The handler knows which fields are "ours" and which belong to dependencies.

### Cargo

**`CargoManifestHandler` (`cargo_manifest.rs`)** uses `toml_edit` to parse `Cargo.toml` and update
only `[package].version` and/or `[workspace.package].version`. The `[dependencies]` table is never
touched.

**`CargoLockHandler` (`cargo_lock.rs`)** parses `Cargo.lock` with `toml_edit` and discovers which
`[[package]]` entries belong to the current workspace by:

1. Reading `[package].name` from the root `Cargo.toml`.
2. Expanding `[workspace.members]` glob patterns to find sub-crate `Cargo.toml` files and reading
   their `[package].name` values.

Only those entries whose `name` is in the discovered member set are updated. A safety guard
additionally checks that the entry's current `version` matches `old_version` before updating.

**Why not `cargo update --workspace`?** Running `cargo update --workspace` requires `cargo` to be
installed, may fail in offline CI environments, and technically touches more than just the version
field (it re-resolves dependency ranges). The in-process handler is more predictable, works without
a Rust toolchain installed, and is faster. It is, however, not the default for all users — you can
opt into the command approach with `strategy = "command"` and
`command = ["cargo", "update", "--workspace"]` if you prefer.

### npm

**`NpmManifestHandler` (`npm.rs`)** parses `package.json` with `serde_json` and updates only the
root-level `"version"` field.

**`NpmLockHandler` (`npm.rs`)** updates:

- The root `"version"` field (lockfile v1 format).
- `packages[""].version` (lockfile v2/v3 format).

Neither handler ever touches `dependencies`, `devDependencies`, or any `node_modules/*` entry.

### pnpm and yarn

These formats are complex enough that in-process editing is risky:

- pnpm's lockfile format has changed significantly between major versions (v6 → v9 → recent).
- yarn classic uses a custom non-standard format; yarn berry uses a structured YAML-like format that
  changes between OnP modes.

The `PnpmLock` and `YarnLock` handlers therefore default to the command strategy:

- pnpm: `pnpm install --lockfile-only --ignore-scripts`
- yarn: `yarn install --mode=update-lockfile`

These run after the manifest version has been updated and regenerate only the minimal lockfile
changes. Users who do not have the toolchain installed can set `strategy = "skip"` to avoid the
command entirely.

### Python (pyproject.toml)

**`PyprojectHandler` (`pyproject.rs`)** handles both:

1. PEP 621 (`[project].version`) — preferred.
2. Poetry (`[tool.poetry].version`) — fallback.

No lockfile handler is provided yet for `uv.lock` or `poetry.lock`. Add a `[[version.files]]` entry
with `strategy = "command"` to shell out to `poetry lock --no-update` or `uv lock` if needed.

## Handler selection

The dispatch order for any file is:

1. If the `[[version.files]]` entry has an explicit `kind` (not `auto`), that handler is used
   unconditionally.
2. If `kind = "auto"` (the default, including all `commit_version_files` entries), the handler is
   inferred from the file basename via `detect::infer_kind` in `version/detect.rs`.
3. Unknown basenames fall back to `PlainHandler` (global string replace), preserving historical
   behavior.

## Atomicity

All handlers work in two phases:

1. **Prepare**: each handler reads the file and computes the updated content without writing
   anything. Command-based handlers snapshot the file bytes before running the command.
2. **Apply**: `handlers::apply_updates` writes files in order. On the first write failure, all
   previously written files are restored from their pre-update snapshots.

This means the set of version files is either fully updated or left completely unchanged, regardless
of how many files are in the list.

## Adding a new handler

1. Create `src/version/<name>.rs` implementing `handlers::Handler`.
2. Add a new `FileKind` variant in `version.rs`.
3. Add a new `FileEntryKind` variant in `config.rs` and update `detect::infer_kind` in
   `version/detect.rs` to recognize the relevant basenames.
4. Wire the handler into `update_version_files_rich` in `version.rs`.
5. Add tests for: normal update, no-op (wrong file content), parse error, missing-field error.
