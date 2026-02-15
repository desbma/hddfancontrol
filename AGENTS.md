# AGENTS.md

## Commands

- Build: `cargo build`
- Check: `cargo clippy --all-targets --all-features -- -D warnings`
- Test all: `cargo test --all-features`
- Test single: `cargo test --all-features <test_name>` (e.g. `cargo test --all-features test_parse_temp`)
- Format: `cargo fmt`

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
- Clippy pedantic enabled; no `unwrap`/`expect`/`panic`/`todo` outside tests.
- Every module and item must have a doc comment (`//!` or `///`); `missing_docs` is warned.
- Doc comments do not end with a dot, unless it separates sentences
- Imports: group std, then external crates, then local modules. Use `_` suffix for unused trait imports.
- Prefer `default-features = false` for dependencies.
- In tests: use `use super::*;` to import from the parent module
- In tests: prefer `unwrap()` over `expect()` for conciseness
- In tests: do not add custom messages to `assert!`/`assert_eq!`/`assert_ne!` — the test name is sufficient
- When moving or refactoring code, never remove comment lines — preserve all comments and move them along with the code they document

## Version control

- This repository uses the jujutsu VCS. **Never use any `jj` command that modifies the repository**.
- You can also use read-only git commands for inspecting repository state. **Never use any git command that modifies the repository**.
