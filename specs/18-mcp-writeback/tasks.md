# Tasks — 18-mcp-writeback

Meta:
- Spec: 18-mcp-writeback — Persist MCP mutations to session folder
- Depends on: spec:09-session-store/T006, spec:12-mcp/T023, spec:14-cli-entrypoint/T003
- Global scope:
  - src/mcp/
  - src/main.rs

## In Progress

## Blocked

## Todo

## Done

- [x] T001: Persist MCP mutations to the session folder (owner: worker:019c3cc6-cd8f-79f1-bb42-3f55647295ce) (scope: src/mcp/server.rs, src/main.rs) (depends: -)
  - Started_at: 2026-02-08T10:20:42+00:00
  - Completed_at: 2026-02-08T10:34:27+00:00
  - Completion note: Added a “persistent” MCP mode that writes successful `diagram.apply_ops` and `xref.add/remove` mutations back to the provided session folder via `SessionFolder::save_session`, using retry-safe semantics (apply to cloned session → save → commit in-memory state). Wired `nereid --mcp --session <dir>` to use this persistent mode and added unit tests proving rev/xref changes persist on disk.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 192 passed)
