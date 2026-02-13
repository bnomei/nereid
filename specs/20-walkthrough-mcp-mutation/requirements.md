# Requirements — 20-walkthrough-mcp-mutation

This spec completes the walkthrough MCP tool surface from `docs/protocol-01.md` §8.7 by adding delta + structured mutation tools.

Mayor-only protocol reference: `docs/protocol-01.md` (workers must not be sent to this doc; extract needed excerpts into task `Context:` blocks)

## Requirements (EARS)

- THE SYSTEM SHALL expose `walkthrough.get_delta` as a typed MCP tool with a bounded delta history window.
- THE SYSTEM SHALL expose `walkthrough.apply_ops` as a typed MCP tool gated by `base_rev` that returns `{ new_rev, applied, delta }`.
- WHEN the MCP server is running in persistent mode (started with `--session <dir>`), THE SYSTEM SHALL persist successful walkthrough mutations to the session folder.

