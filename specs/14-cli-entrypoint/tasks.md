# Tasks — 14-cli-entrypoint

Meta:
- Spec: 14-cli-entrypoint — CLI entrypoint + argument parsing
- Depends on: spec:11-tui/T001
- Global scope:
  - src/main.rs

## In Progress

## Blocked

## Todo

## Done

- [x] T003: Add `--mcp --session <dir>` mode to run MCP server over stdio (owner: worker:019c3ca6-98b4-7930-8bf2-260d0a378923) (scope: src/main.rs) (depends: spec:12-mcp,spec:09-session-store)
  - Started_at: 2026-02-08T09:48:15+00:00
  - DoD: `cargo run -- --mcp --session <dir>` starts an MCP stdio server backed by the loaded session; existing TUI modes remain unchanged.
  - Validation: `cargo test --offline`
  - Completed_at: 2026-02-08T09:57:23+00:00
  - Completion note: Added `--mcp --session <dir>` mode in `src/main.rs` to load the session folder and serve `NereidMcp` over stdio on a local current-thread tokio runtime; invalid args print usage and exit 2.
  - Validation result: `cargo test --offline` (ok)

- [x] T002: Add optional `--session <dir>` to load a session folder (owner: worker:019c37fd-5a90-7980-b050-9f0eb1d701bf) (scope: src/main.rs) (depends: spec:09-session-store/T005,spec:11-tui/T004)
  - Started_at: 2026-02-07T12:04:43+00:00
  - DoD: `cargo run -- --session ./my-session` loads session and starts TUI with that session active.
  - Validation: manual: run; `cargo test`
  - Completed_at: 2026-02-07T12:09:08+00:00
  - Completion note: Implemented std-only argument parsing in `src/main.rs` for `--session <dir>`: loads with `store::SessionFolder::new(dir).load_session()` and launches `tui::run_with_session(session)`. Unknown/invalid args print a short usage and exit non-zero; no-arg path keeps demo mode.
  - Validation result: `cargo test` (ok); manual `cargo run -- --session <dir>` (ok)

- [x] T001: Wire `main` to launch `tui::run()` (owner: worker:019c37c6-e9ab-77f1-b4f2-7dddf29a2107) (scope: src/main.rs) (depends: spec:11-tui/T001)
  - Started_at: 2026-02-07T11:05:02+00:00
  - DoD: `cargo run` launches the TUI; quit returns cleanly (restores terminal state).
  - Validation: manual: `cargo run` and quit; `cargo test`
  - Escalate if: `tui::run()` API needs changes; keep main minimal and add a separate task for API refactors.
  - Completed_at: 2026-02-07T11:07:18+00:00
  - Completion note: Wired `main` to call `nereid::tui::run()`; errors now print to stderr and exit non-zero. Validated with `cargo test` and manual `cargo run` + quit.
  - Validation result: manual `cargo run` + `cargo test` (ok)
