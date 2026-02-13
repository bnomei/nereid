# Design — 22-mcp-ui-context-and-session-routes

Implement the missing protocol tools in:
- `src/mcp/server.rs` (tool handlers + server info tool list)
- `src/mcp/types.rs` (typed params/responses)

Implementation notes:
- `session.routes` can be backed by the existing `crate::query::session_routes::find_route` logic, returning either 0 or 1 routes initially.
- `ui.get_selection` / `ui.get_view_state` are best-effort in `--mcp` mode (there is no interactive TUI); return stable defaults derived from the session where possible (e.g. `active_diagram_id`).

