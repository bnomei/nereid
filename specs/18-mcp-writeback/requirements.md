# Requirements — 18-mcp-writeback

This spec persists MCP mutations back to the session folder on disk when running `nereid --mcp --session <dir>`.

## Requirements (EARS)

- WHEN the MCP server is started with a session folder (`--session <dir>`), THE SYSTEM SHALL persist successful mutations from `diagram.apply_ops` and `xref.add`/`xref.remove` to that session folder using the existing store format.
- THE SYSTEM SHALL keep read-only MCP tools free of filesystem side effects.
- IF persistence fails, THE SYSTEM SHALL return an MCP `INTERNAL_ERROR` and leave the in-memory session unchanged (retry-safe semantics).
