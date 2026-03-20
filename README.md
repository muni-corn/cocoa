# `cocoa`

hi! i'm cocoa, the **co**nventional **co**mmit **a**ssistant! ^u^

i can help you write good, conventional commits, generate beautiful changelogs, manage your
versioning scheme, and even draft commit messages for you with the power of artificial intelligence!

## what can i do?

- **interactive commits:** a cozy TUI that walks you through crafting a perfect commit
- **AI-powered generation:** let me read your staged changes and write the commit message for you
- **commit linting:** catch bad commit messages before they sneak into your history
- **changelog generation:** auto-generate release notes from your commits, beautifully
- **version management:** automatically bump versions based on what your commits say
- **git hooks:** let me validate or generate commits automatically as part of your workflow
- **CI/CD support:** i work great in GitHub Actions, GitLab CI, and more!

## quick start

### install

take your pick:

```bash
nix profile install github:muni-corn/cocoa
```

```bash
cargo install cocoa
```

or grab a pre-built binary from [GitHub Releases](https://github.com/musicaloft/cocoa/releases)!

### set up your project

```bash
cocoa init
```

this interactive wizard sets up a `.cocoa.toml` in your project root. you can also write it manually
if you prefer. see the [configuration guide](./docs/user-guide/configuration.md) for help with that.

### make your first commit

```bash
git add .
cocoa commit
```

i'll guide you through picking a type, scope, subject, and any optional details. the result is a
clean, conventional commit every time!

### or let me write it for you

```bash
git add .
cocoa generate
```

you can set up [AI providers](./docs/user-guide/ai-providers.md) to use this feature!

### lint your commits

lint the most recent commit:

```bash
cocoa lint
```

lint a range of commits:

```bash
cocoa lint HEAD~5...HEAD
```

### release a new version

```bash
cocoa bump auto

# or
cocoa release
```

i'll read your commits, figure out the right version bump, update your version files, generate a
changelog, and create a git tag. all in one command!

## installation

| method           | command                                                         |
| ---------------- | --------------------------------------------------------------- |
| cargo            | `cargo install cocoa`                                           |
| pre-built binary | [GitHub Releases](https://github.com/musicaloft/cocoa/releases) |
| from source      | `devenv build` or `cargo build --release`                       |

see the [installation guide](./docs/user-guide/installation.md) for full details, including
verifying your installation and updating it.

## documentation

the full user guide lives in [`docs/user-guide/`](./docs/user-guide/):

- **[quick start](./docs/user-guide/quick-start.md):** get up and running
- **[installation](./docs/user-guide/installation.md):** all install methods
- **[creating commits](./docs/user-guide/creating-commits.md):** interactive and generated commits
- **[linting commits](./docs/user-guide/linting-commits.md):** validation and team rules
- **[changelogs](./docs/user-guide/changelogs.md):** auto-generate release notes
- **[versioning](./docs/user-guide/versioning.md):** bump versions and create tags
- **[configuration](./docs/user-guide/configuration.md):** customize everything
- **[AI providers](./docs/user-guide/ai-providers.md):** set up commit generation
- **[git hooks](./docs/user-guide/git-hooks.md):** automate validation
- **[CI/CD integration](./docs/user-guide/ci-cd.md):** integrate with CI and CD
- **[troubleshooting](./docs/user-guide/troubleshooting.md):** if things go sideways

## license

`cocoa` is licensed under [GPL-3.0-or-later](./LICENSE).
