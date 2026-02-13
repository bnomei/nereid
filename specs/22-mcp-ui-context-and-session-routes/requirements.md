# Requirements — 22-mcp-ui-context-and-session-routes

This spec closes remaining gaps between the implemented MCP server and `docs/protocol-01.md` §8:

- Session tool: `session.routes(from_ref, to_ref, limit, max_hops) -> [Route]`
- Optional UI context tools:
  - `ui.get_selection() -> { object_ref?, diagram_id?, mode? }`
  - `ui.get_view_state() -> { active_diagram_id, scroll, panes }`

Mayor-only protocol reference: `docs/protocol-01.md` (workers must not be sent to this doc; extract needed excerpts into task `Context:` blocks)

## Requirements (EARS)

- THE SYSTEM SHALL expose `session.routes` as a typed MCP tool for finding 0+ routes between two `ObjectRef`s in the current session.
- THE SYSTEM SHALL accept optional `limit` and `max_hops` parameters for `session.routes` and return a deterministic result.
- THE SYSTEM SHALL expose `ui.get_selection` and `ui.get_view_state` as typed MCP tools. In `--mcp` mode, these tools SHALL return stable defaults (no selection, default view state) rather than crashing.
