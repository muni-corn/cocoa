# CI/CD Integration

i can live in your CI pipeline to validate commit messages and releases!

## overview

i can integrate with CI/CD systems to:

- validate commit messages on every PR
- lint commit history across branches
- automatically generate releases
- update versions and changelogs
- create release tags

the sky's the limit!

## GitHub Actions

### basic commit validation

to validate commits in every pull request:

```yaml
# .github/workflows/lint-commits.yml
name: Lint commits with cocoa

on:
  pull_request:
    types: [opened, synchronize, reopened]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0 # Need full history for commit range

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install cocoa
        run: cargo install cocoa

      - name: Lint commits
        run: cocoa lint origin/main...HEAD
```

now every PR is linted!

### automatic release on tag

Create releases automatically when you push a version tag:

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
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install cocoa
        run: cargo install cocoa

      - name: Create release
        run: cocoa release --json

      - name: Generate release notes
        run: cocoa changelog > RELEASE_NOTES.md

      - name: Create GitHub release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          body_path: RELEASE_NOTES.md
          draft: false
          prerelease: false
```

### full CI pipeline

a complete setup with validation, testing, and releasing:

```yaml
# .github/workflows/ci.yml
name: All the cocoa

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install cocoa
        run: cargo install cocoa

      - name: Lint commits
        run: |
          if [ "${{ github.base_ref }}" != "" ]; then
            cocoa lint origin/${{ github.base_ref }}...HEAD
          else
            cocoa lint
          fi

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: cargo test

  release:
    runs-on: ubuntu-latest
    needs: [validate, test]
    if: startsWith(github.ref, 'refs/tags/v')
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install cocoa
        run: cargo install cocoa

      - name: Generate changelog
        run: cocoa changelog --json > changelog.json

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          body_path: CHANGELOG.md
          files: |
            target/release/cocoa
```

## GitLab CI

### basic validation

```yaml
# .gitlab-ci.yml
image: rust:latest

stages:
  - lint
  - test
  - release

lint_commits:
  stage: lint
  script:
    - cargo install cocoa
    - cocoa lint $CI_COMMIT_BEFORE_SHA...$CI_COMMIT_SHA

test:
  stage: test
  script:
    - cargo test

release:
  stage: release
  script:
    - cargo install cocoa
    - cocoa release
    - git push --all --tags
  only:
    - tags
```

## Bitbucket pipelines

```yaml
# bitbucket-pipelines.yml
image: rust:latest

pipelines:
  branches:
    main:
      - step:
          name: Lint commits
          script:
            - cargo install cocoa
            - cocoa lint origin/main...HEAD

tags:
  "v*":
    - step:
        name: Release
        script:
          - cargo install cocoa
          - cocoa release
```

## common CI patterns

### validate on every commit

Reject PRs with invalid commits:

```yaml
- name: Lint commits
  run: cocoa lint origin/main...HEAD || exit 1
```

the exit code signals CI to fail.

### JSON output for reports

get structured output for dashboards:

```yaml
- name: Lint commits (JSON)
  run: cocoa lint origin/main...HEAD --json > lint-report.json

- name: Upload report
  uses: actions/upload-artifact@v3
  with:
    name: lint-report
    path: lint-report.json
```

### allow warnings, fail on errors

Only hard errors fail the build:

```yaml
- name: Lint commits
  run: |
    cocoa lint origin/main...HEAD --json > report.json

    # check for errors (exit if found)
    if grep -q '"severity":"error"' report.json; then
      exit 1
    fi

    # warnings are OK
    exit 0
```

### automatic version bumping

Auto-bump and release after successful tests:

```yaml
- name: Check if release needed
  id: check
  run: |
    cocoa bump auto --verbose
    echo "version=$(cocoa --version)" >> $GITHUB_OUTPUT

- name: Create release
  if: steps.check.outcome == 'success'
  run: cocoa release
```

## environment variables

### API Keys in CI

Store secrets securely:

**GitHub Actions:**

```yaml
- name: Generate commit with AI
  env:
    OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
  run: cocoa generate
```

**GitLab:**

```yaml
variables:
  OPENAI_API_KEY: $CI_JOB_TOKEN

script:
  - cocoa generate
```

**Bitbucket:**

```yaml
script:
  - cocoa generate
env:
  - OPENAI_API_KEY=secure_value_from_ui
```

### git credentials

For pushing releases:

**GitHub Actions (built-in):**

```yaml
- uses: actions/checkout@v4
  with:
    token: ${{ secrets.GITHUB_TOKEN }}
```

**GitLab:**

```yaml
script:
  - git push --all --tags
# GitLab CI automatically uses its token
```

## Troubleshooting

### "cocoa: command not found"

make sure i'm installed!

```yaml
- name: Install cocoa
  run: cargo install cocoa
```

### "no commits to lint"

maybe you've configured the range incorrectly?

```yaml
# wrong :(
- run: cocoa lint origin/main

# correct :)
- run: cocoa lint origin/main...HEAD
```

### "API key error"

have you passed secrets to my environment?

```yaml
# no :(
run: cocoa generate

# yes! :D
env:
  OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
run: cocoa generate
```

### "permission denied" on git push

do you have write permissions?

**GitHub:**

```yaml
- uses: actions/checkout@v4
  with:
    token: ${{ secrets.GITHUB_TOKEN }}
# now git push should work!
```

**GitLab:**

```yaml
script:
  - git config user.name "CI"
  - git config user.email "ci@example.com"
  - git push --all --tags
```

### slow `cargo install`

Cache the installation:

```yaml
- uses: actions/cache@v3
  with:
    path: ~/.cargo/bin/cocoa
    key: cocoa-${{ runner.os }}

- name: Install cocoa
  run: cargo install cocoa --locked
```

## my recommendations

### 1. validate early

lint commits on every push, not just releases:

```yaml
on: [push, pull_request]
```

### 2. gate on validation

block merging until commits are valid:

```yaml
- name: Lint
  run: cocoa lint origin/main...HEAD || exit 1
```

### 3. document CI rules

help developers understand requirements:

```markdown
## CI Rules

- All commits must follow Conventional Commits
- Scopes must be one of: [auth, api, db, ui]
- Breaking changes must be documented
```

### 4. automate releases

let CI handle version bumps and tags:

```yaml
- name: Auto-release
  if: github.ref == 'refs/heads/main'
  run: cocoa release
```

### 5. separate concerns

different jobs for different tasks:

```yaml
jobs:
  lint: # validate
    ...
  test: # test
    ...
  release: # release
    ...
```

## where to now?

- set up locally: [quick start](./quick-start.md)
- configure rules: [configuration](./configuration.md)
- learn git hooks: [git hooks](./git-hooks.md)
