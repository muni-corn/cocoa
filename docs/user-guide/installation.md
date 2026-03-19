# installation guide

introduce me to your system! i promise i'll be a good roommate!

## prerequisites

you'll need:

- **Git 2.25.0 or later** (for git hooks)
- **Linux, macOS, or Windows** (any modern OS)
- **Rust 1.70+** (only if building from source)

```bash
git --version
# git version 2.40.1

rustc --version
# rustc 1.75.0
```

## installation methods

### method 1: cargo (easiest)

got Rust? then this is your move:

```bash
cargo install cocoa
```

verify it worked:

```bash
cocoa --version
# cocoa 0.1.0
```

### method 2: pre-built binaries

download me from [GitHub Releases](https://github.com/musicaloft/cocoa/releases):

```bash
# for macOS (Apple Silicon)
curl -L https://github.com/musicaloft/cocoa/releases/latest/download/cocoa-aarch64-apple-darwin.tar.gz | tar xz
sudo mv cocoa /usr/local/bin/

# for macOS (Intel)
curl -L https://github.com/musicaloft/cocoa/releases/latest/download/cocoa-x86_64-apple-darwin.tar.gz | tar xz
sudo mv cocoa /usr/local/bin/

# for Linux
curl -L https://github.com/musicaloft/cocoa/releases/latest/download/cocoa-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv cocoa /usr/local/bin/

# for Windows, grab the .exe or use Cargo above!
```

### method 3: build from source

i recommend using the devenv shell for this:

```bash
devenv build
```

or, you can also build me from scratch:

```bash
git clone https://github.com/musicaloft/cocoa.git
cd cocoa
cargo build --release
sudo mv target/release/cocoa /usr/local/bin/
```

or just use the binary directly:

```bash
./target/release/cocoa --version
```

## verify installation

```bash
cocoa --version
cocoa --help
```

you should see the version and help without errors.

## updating

**via `cargo`:**

```bash
cargo install cocoa --force
```

**from releases:**

1. download the latest
2. replace the binary in your PATH

## uninstalling

**via `cargo`:**

```bash
cargo uninstall cocoa
```

**from source/releases:**

```bash
rm /usr/local/bin/cocoa
# or wherever you put my binary
```

## troubleshooting installation

### "cocoa: command not found"

can't find me? your PATH might need help:

```bash
# where am i?
which cocoa
# or if using cargo:
ls ~/.cargo/bin/cocoa

# add to PATH (add to ~/.bashrc, ~/.zshrc, or equivalent)
export PATH="$PATH:~/.cargo/bin"
```

## next steps

good to go? head to the [quick start guide](./quick-start.md) to set me up!

**stuck?** see [troubleshooting](./troubleshooting.md) for more help.
