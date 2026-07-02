# AGENTS.md

## Build & Test Commands

- Build: `cargo build`
- Check/Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Format: `cargo +nightly fmt --check -- --config imports_granularity=Crate --config group_imports=StdExternalCrate`
- Test: `cargo test --all-features`
- Single test: `cargo test --all-features <test_name>` (e.g. `cargo test --all-features test_parse_temp`)

## Architecture

Single-binary Rust daemon (`hddfancontrol`) that regulates Linux fan speed based on HDD temps.

- `src/main.rs` — entry point, main control loop
- `src/cl.rs` — CLI parsing (clap derive)
- `src/device/` — drive and hwmon device abstractions
- `src/probe/` — temperature probing backends (drivetemp, hddtemp, hdparm, smartctl)
- `src/fan/` — fan speed control logic
  - `mod.rs` — Fan trait, Speed newtype, Thresholds struct, target_speed function
  - `pwm_fan.rs` — PWM-based fan control via sysfs
  - `cmd_fan.rs` — command-based fan control via external command invocation
- `src/pwm.rs` — PWM sysfs interface
- `src/sysfs.rs` — sysfs file helpers
- `src/exit.rs` — exit hook for PWM restore
- `src/tests.rs` — shared test utilities

## Code Style

- Rust 2024 edition, MSRV 1.87. Errors via `anyhow`/`thiserror`.
- Clippy pedantic enabled plus many restriction lints; no `unwrap`/`expect`/`panic`/`todo` outside tests.
- Every module and item must have a doc comment (`//!` or `///`); `missing_docs` is warned.
- Imports:
  - Place all `use` statements at the top of the file; do not put them inside functions, `impl` blocks, or other inner scopes, except inside `#[cfg(...)]` modules such as `mod tests`
  - Group std imports first, then external crates, then local modules
  - Never use fully-qualified paths in code; always import namespaces via `use` statements and refer to symbols by their short name
  - Import deep `std` namespaces aggressively, except for namespaces like `io` or `fs` whose symbols have very common names that may collide
  - For third-party crates, prefer importing at the crate or module level rather than deeply importing individual symbols, unless needed to avoid very long fully-qualified namespaces
- In format strings, never mix positional placeholders (`{}`) with named ones; for expression arguments, use named arguments (`{id}` … `id = loc.id`)
- When formatting paths in error messages or logs, always use debug formatting (`:?`) rather than `.display()` to preserve non-UTF-8 safety and show quoting
- Prefer `log` macros for logging; no `dbg!` or `todo!`
- Prefer `default-features = false` for dependencies.
- Do not add `derive` traits unless they are required by the current code or actively used by tests/runtime behavior
- Comments and doc comments should be concise, and single-sentence comments should omit trailing periods
- Doc comments do not end with a dot, unless it separates sentences
- In tests: use `use super::*;` to import from the parent module
- In tests: prefer `unwrap()` over `expect()` for conciseness
- In tests: do not add custom messages to `assert!`/`assert_eq!`/`assert_ne!` — the test name is sufficient
- In tests: prefer full type comparisons with `assert_eq!` over selectively checking nested attributes or unpacking; tag types with `#[cfg_attr(test, derive(Eq, PartialEq))]` if needed
- In tests: do not add section-separator comments — test names are descriptive enough
- When moving or refactoring code, never remove comment lines — preserve all comments and move them along with the code they document

## Version control

- This repository uses the jujutsu VCS. **Never use any `jj` command that modifies the repository**.
- You can also use read-only git commands for inspecting repository state. **Never use any git command that modifies the repository**.
