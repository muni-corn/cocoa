# quick start guide

no time to waste? let's go!

### step 1: install

choose your installation method:

#### using `cargo` (Rust)

```bash
cargo install cocoa
```

#### from source

```bash
git clone https://github.com/musicaloft/cocoa
cd cocoa
cargo build --release
./target/release/cocoa --version
```

see [installation](./installation.md) for more options.

### step 2: initialize configuration

in your project root, run:

```bash
cocoa init
```

this interactive wizard will guide you through:

- commit types you want to allow (feat, fix, chore, etc.)
- scope conventions (optional)
- line length preferences
- AI provider settings (optional)
- changelog configuration

the result is a `.cocoa.toml` file that looks like this:

```toml
[commit]
types = ["feat", "fix", "chore", "docs", "test"]

[commit.rules]
enabled = true

[commit.rules.warn]
subject_length = 72

[changelog]
output_file = "CHANGELOG.md"

[version]
strategy = "semver"
tag_prefix = "v"
```

**don't like the wizard?** my bad. :( but you can create `.cocoa.toml` manually in your project
root! see [configuration](./configuration.md) for all options.

### step 3: set up git hooks

want me to validate commits automatically before they're created?

```bash
# install my git hook
cocoa hook

# now every commit will be validated automatically!
git commit -m "whatever"
# will probably fail based on good validation rules
```

now you're good to go!!

want to use AI for generating commits? you fancy developer, you~ head over to
[AI providers](./ai-providers.md) to set that up!

## quick reference

### creating a first commit

#### option a: interactive mode

if you like having a fancy TUI to create commits:

```bash
# stage your changes first
git add .

# create your commit interactively
cocoa commit
```

i'll guide you through:

1. picking a type: feat, fix, chore, etc.
2. picking a scope: optional section of code (auth, database, etc.)
3. writing your own subject: what did you change? (max 72 chars)
4. adding more information: an optional body with a detailed explanation
5. and adding footers: optional breaking changes or issue references

result:

```
feat(auth): add password reset functionality

Users can now reset their password via email. An email with a secure
link is sent immediately.
```

#### option B: AI-powered generation

if you want to be super fancy, i can let AI draft a commit message from your staged changes:

```bash
# stage your changes
git add .

# generate a commit message
cocoa generate
```

make sure you have me setup with an [AI provider](./ai-providers.md) for this to work!

#### option C: generate from command line

if you want to make commits simply and quickly:

```bash
git commit -m "feat(auth): add password reset functionality"
```

i can still lint your message and warn you if it doesn't follow conventions!

## lint past commits

i can validate past commit messages to ensure they follow your rules:

```bash
# lint the most recent commit
cocoa lint

# lint multiple commits
cocoa lint HEAD~5...HEAD

# lint a specific message
echo "feat(auth): add 2FA" | cocoa lint --stdin
```

in CI, i will exit with an error if linting fails:

```bash
cocoa lint --json  # machine-readable output for CI/CD
```

### your first release

Once you've made commits following Conventional Commits, create a release:

```bash
# automatically bump version based on commits
cocoa bump auto

# this will automatically
# - read your commit history since last version
# - detect breaking changes (bumps major)
# - detect features (bumps minor)
# - detect fixes (bumps patch)
# - update version numbers in your files (package.json, Cargo.toml, etc.)
# - generate a changelog
# - create a git tag

# or, you can also manually bump
cocoa bump major    # v1.0.0 → v2.0.0
cocoa bump minor    # v1.0.0 → v1.1.0
cocoa bump patch    # v1.0.0 → v1.0.1
```

## questions?

### do i have to use conventional commits?

not for existing projects! but i really work best with them. check out [migration](./migration.md)
if you're converting an existing project.

### can i use `cocoa` without AI?

of course! i'm not gonna shove artificial stuff down your throat.

you can use interactive mode (`cocoa commit`) if you want a comfy experience in crafting your own
commits.

### what if i mess up a commit message?

it truly happens to the best of us.

a simple amending can remedy that: `git commit --amend`

i'll lint your amended commit message, too.

### how do i use cocoa in my CI/CD?

i've got a whole guide for that! see [CI/CD integration](./ci-cd.md) for GitHub Actions, GitLab CI,
and more!

## got more time?

now that you're up and running, i have more i can show you:

- read [creating commits](./creating-commits.md) for advanced commit techniques
- read [configuration](./configuration.md) to customize rules for your team
- read [CI/CD integration](./ci-cd.md) to automate releases
- read [AI providers](./ai-providers.md) to enable commit generation
