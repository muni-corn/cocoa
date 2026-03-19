# `cocoa` user guide

hi! i'm cocoa, the **co**nventional **co**mmit **a**ssistant! ^o^

i can help you write well-formed commits, generate nice changelogs, manage your versioning scheme,
and even generate commit messages through the power of artificial intelligence!

## documentation overview

this guide is organized into bite-sized sections so you can find what you need fast:

### getting started

- **[quick start](./quick-start.md):** meet me and let's get set up!
- **[installation](./installation.md):** get me running on your system!

### core features

- **[creating commits](./creating-commits.md):** write commits interactively or with assistance
- **[linting commits](./linting-commits.md):** validate commit messages and catch issues early
- **[generating changelogs](./changelogs.md):** auto-generate release notes as beautiful as me~
- **[version management](./versioning.md):** bump versions and create release tags

### configuration

- **[configuration guide](./configuration.md):** customize rules to fit your workflow
- **[AI providers](./AI-providers.md):** set up AI-powered commit generation

### advanced topics

- **[git hooks](./git-hooks.md):** integrate me into your git workflow automatically
- **[CI/CD integration](./ci-cd.md):** let me work in continuous integration pipelines
- **[migration guide](./migration.md):** let me replace your other tools
- **[troubleshooting](./troubleshooting.md):** work through some of my problems

## common tasks

what do you want to do?

- [**write your first commit?**](./quick-start.md#your-first-commit)
- [**set up AI commit generation?**](./ai-providers.md)
- [**enforce commit standards for your team?**](./linting-commits.md#setting-up-team-rules)
- [**generate a changelog?**](./changelogs.md)
- [**release a new version?**](./versioning.md#creating-a-release)
- [**integrate with GitHub Actions?**](./ci-cd.md#github-actions)

## key concepts

### conventional commits

i'm a big believer in the [**conventional commits**](https://www.conventionalcommits.org/)
specification—a simple format that makes commits readable by people, machines, and zebras alike:

```gitcommit
type(scope): subject

optional body

optional footer
```

**example:**

```gitcommit
feat(auth): add two-factor authentication support

Implement TOTP-based 2FA for user accounts. Users can now
enable 2FA in their account settings.

Closes #142
BREAKING CHANGE: Authentication endpoints now require 2FA
```

### semantic versioning

semantic versioning is a way to version your software that makes it easy for users to understand the
impact of updates:

given a version number like **1.2.3**,

- **1** is the major version (bumped when you make breaking changes)
- **2** is the minor version (bumped when you add new features)
- **3** is the patch version (bumped for bug fixes)

i can automatically bump versions based on your commits!

## need help?

- check **[troubleshooting](./troubleshooting.md)** for common issues we might run into together
- run `cocoa --help` to get command-line help from me
- run `cocoa <command> --help` for command-specific options
