# Requirements — 21-mcp-persist-session-active

This spec ensures persistent MCP mode (`--mcp --session <dir>`) writes session-level mutations (active diagram/walkthrough ids) back to disk.

## Requirements (EARS)

- WHEN the MCP server is running in persistent mode, THE SYSTEM SHALL persist `session.set_active_diagram` to the session folder.
- WHEN the MCP server is running in persistent mode, THE SYSTEM SHALL persist `session.set_active_walkthrough` to the session folder.

