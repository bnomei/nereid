# Repository Guidelines

## Project Structure & Module Organization
- `src/`: main Rust crate code.
  - `model/`: core data model + ASTs (sessions, diagrams, IDs).
  - `format/`: Mermaid-ish parse/export.
  - `render/`: ASCII/Unicode diagram rendering.
  - `layout/`: layout + routing.
  - `tui/`: `ratatui` terminal UI.
  - `mcp/`: MCP server + tool surface.
  - `store/`: session persistence (folder format).
- `benches/`: Criterion benchmarks (fixtures live in `benches/fixtures/`).
- `scripts/`: helper scripts (notably `scripts/bench-criterion` for baseline bench runs).
- `docs/`, `specs/`: local design notes (currently gitignored; don’t rely on them being present in PRs).
- `target/`: build output (gitignored).

## Build, Test, and Development Commands
- `cargo run`: start the TUI (loads session from `.` by default).
- `cargo run -- --session path/to/session`: load a persisted session folder (expects `nereid-session.meta.json`, `diagrams/*.mmd`, `walkthroughs/*.wt.json`, …).
- `cargo run -- --mcp`: run the MCP server over stdio with persistence (loads session from `.` by default).
- `cargo run -- --mcp --session path/to/session`: run the MCP server over stdio with persistence.
- `cargo test` (or `cargo test <filter>`): run unit tests.
- `cargo fmt`: format with rustfmt.
- `cargo clippy --all-targets --all-features`: lint (fix warnings before pushing).
- `./scripts/bench-criterion save|compare`: run Criterion benchmarks with repeatable baselines.

## Coding Style & Naming Conventions
- Rust 2021; follow rustfmt defaults (4-space indentation, trailing commas where rustfmt adds them).
- Naming: modules/files `snake_case`, types `PascalCase`, functions `snake_case`, constants `SCREAMING_SNAKE_CASE`.
- Keep changes scoped to the relevant module in `src/`; update docs/tests when behavior changes.

## Testing Guidelines
- Tests use Rust’s built-in test harness and live alongside code under `src/**`.
- Add regression tests close to the behavior you’re changing; prefer deterministic fixtures for rendering/layout changes.
- Benchmarks are Criterion-based; keep fixtures deterministic and avoid RNG.

## Commit & Pull Request Guidelines
- Git history is heavy on `wip`; prefer descriptive, imperative subjects, ideally `<area>: <summary>` (e.g. `mcp: persist active session`).
- PRs: include what/why, how to reproduce (a minimal `--session` folder), and screenshots/recordings for TUI/render output changes.
- Before review: run `cargo fmt` and `cargo test`.
