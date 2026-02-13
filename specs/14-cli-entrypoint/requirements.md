# Requirements — 14-cli-entrypoint

This spec wires a runnable CLI entrypoint for Nereid.

Mayor-only protocol reference: `docs/protocol-01.md` (workers must not be sent to this doc; extract needed excerpts into task `Context:` blocks)

## Requirements (EARS)

- WHEN the binary is executed THE SYSTEM SHALL start the TUI (default mode) and exit cleanly on quit.
- WHEN the binary is executed with `--mcp --session <dir>` THE SYSTEM SHALL start the MCP server over stdio using the loaded session.
- THE SYSTEM SHALL keep argument parsing minimal (std-only) until requirements justify a CLI parser crate.
