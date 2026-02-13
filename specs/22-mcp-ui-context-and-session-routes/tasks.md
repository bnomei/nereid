# Tasks — 22-mcp-ui-context-and-session-routes

Meta:
- Spec: 22-mcp-ui-context-and-session-routes — Add missing protocol tools
- Depends on: spec:12-mcp/T023
- Global scope:
  - src/mcp/server.rs
  - src/mcp/types.rs

## In Progress

## Blocked

## Todo

## Done

- [x] T002: Add MCP tools `ui.get_selection` and `ui.get_view_state` (owner: worker:019c3d36-2ad8-7100-94ff-5440c3cc9a9b) (scope: src/mcp/server.rs, src/mcp/types.rs) (depends: -)
  - Started_at: 2026-02-08T12:24:28+00:00
  - Completed_at: 2026-02-08T12:34:47+00:00
  - Completion note: Added typed MCP tools `ui.get_selection` and `ui.get_view_state` that return stable defaults in `--mcp` mode (no selection; view state derived from session active diagram id with default scroll/panes); updated server tool list string; added unit tests.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 212 passed)

- [x] T001: Add MCP tool `session.routes` (owner: worker:019c3d36-274f-77e3-b83f-c41e6d64f371) (scope: src/mcp/server.rs, src/mcp/types.rs) (depends: -)
  - Started_at: 2026-02-08T12:24:28+00:00
  - Completed_at: 2026-02-08T12:34:51+00:00
  - Completion note: Added typed MCP tool `session.routes(from_ref,to_ref,limit,max_hops)` backed by `query::session_routes::find_route`, returning 0–1 routes with deterministic behavior and honoring `limit=0` and `max_hops`; updated server tool list string; added unit tests.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 212 passed)
