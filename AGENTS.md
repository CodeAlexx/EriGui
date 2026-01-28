# Repository Guidelines

## Project Structure & Module Organization
- Workspace crates live in `erigui-core` (core types), `erigui-rendering` (OpenGL backend), `erigui-widgets` (widget implementations), and `erigui-examples` (example apps).  
- Example binaries are defined under `erigui-examples/examples` and runnable via Cargo.  
- Shared assets live in `assets/` and crate-specific assets under `*/assets`.  
- A small placeholder binary sits in `src/`; use it only if you need a root crate entry point.

## Build, Test, and Development Commands
- `cargo build --workspace --release` — full optimized build of all crates.  
- `cargo test --workspace` — run all unit/integration tests. Add targeted tests with `cargo test -p erigui-widgets`.  
- `cargo fmt --all` — format Rust code with rustfmt.  
- `cargo clippy --workspace --all-targets -D warnings` — lint with warnings treated as errors.  
- `cargo run -p erigui-examples --example widget_gallery` — launch the widget showcase. Other examples are listed in `erigui-examples/Cargo.toml`.  
- `./test_widget.sh` — quick smoke run of the widget gallery (spawns then exits).

## Coding Style & Naming Conventions
- Follow Rust 2021 idioms; prefer 4-space indent and rustfmt defaults.  
- Naming: `snake_case` for functions/vars/modules, `PascalCase` for types/traits, `SCREAMING_SNAKE_CASE` for consts/static vars.  
- Keep modules focused; re-export public surface in each crate’s `lib.rs` as already patterned.  
- Avoid panics in library code; prefer `Result` with `thiserror`/`anyhow`.

## Testing Guidelines
- Use `cargo test --workspace` before sending changes; add integration tests under `crate/tests/` when wiring new widgets or rendering paths.  
- Prefer deterministic rendering tests (logic/state assertions) over screenshot baselines.  
- Add doc-tests for API examples that already appear in guides when possible.

## Commit & Pull Request Guidelines
- Use short, imperative commit messages (e.g., “Add slider layout constraints”).  
- PRs should describe intent, list major changes, and note user-facing behavior or API changes.  
- Link related issues and include screenshots or CLI output for visual/rendering changes.  
- Ensure lint, fmt, and tests pass; mention any intentionally skipped checks and why.

## Environment & Platform Notes
- Targeted for Wayland with OpenGL 2.1+; verify you have matching system deps before reporting render issues.  
- Run `RUST_LOG=info cargo run -p erigui-examples --example <name>` to debug with logs enabled.
