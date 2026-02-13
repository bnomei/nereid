# Tasks — 21-mcp-persist-session-active

Meta:
- Spec: 21-mcp-persist-session-active — Persist active diagram/walkthrough setters
- Depends on: spec:18-mcp-writeback/T001, spec:20-walkthrough-mcp-mutation/T001
- Global scope:
  - src/mcp/server.rs

## In Progress

## Blocked

## Todo

## Done

- [x] T001: Persist session active setters in persistent MCP mode (owner: worker:019c3d1d-f82e-7451-95cc-2a58f60e49b0) (scope: src/mcp/server.rs) (depends: -)
  - Started_at: 2026-02-08T11:57:34+00:00
  - Completed_at: 2026-02-08T12:02:39+00:00
  - Completion note: In persistent MCP mode, `session.set_active_diagram` and `session.set_active_walkthrough` now write through to disk using retry-safe clone→save→commit semantics; added unit tests proving the active ids persist after reloading the session folder.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 206 passed)
