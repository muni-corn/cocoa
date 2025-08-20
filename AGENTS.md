# build/test commands

- build: `cargo build`
- test: `cargo test`
- single test: `cargo test test_name`
- lint: `cargo clippy`
- format: `cargo fmt`

# code style & conventions

- follow Rust 2024 edition conventions
- use `snake_case` for variables, functions, modules
- use `PascalCase` for types, structs, enums
- use `SCREAMING_SNAKE_CASE` for constants
- prefer explicit error handling with `Result<T, E>`
- use semantic imports (avoid glob imports)
- keep functions small and focused
- stylize all strings as lowercase; do not capitalize sentences

# project structure

- main binary: `src/main.rs`
- modules in `src/` directory
- tests alongside source files (#[cfg(test)])

# general development

- prioritize separation of concerns and small file sizes
- make small, atomic, incremental git commits
- create unit tests for all new features
- follow conventional commits spec (feat, fix, docs, etc.)
- no co-author footers in commit messages
