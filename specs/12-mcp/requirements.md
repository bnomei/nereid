# Requirements — 12-mcp

This spec implements the MCP server tool surface for agent collaboration.

Mayor-only protocol reference: `docs/protocol-01.md` (workers must not be sent to this doc; extract needed excerpts into task `Context:` blocks)

## Requirements (EARS)

- THE SYSTEM SHALL expose typed read tools (digest/delta/slice) for diagrams and walkthroughs.
- THE SYSTEM SHALL expose structured mutation tools (`apply_ops`) gated by `base_rev`.
- THE SYSTEM SHALL keep agent operations AST-first (no ASCII direct edits).
- THE SYSTEM SHALL implement the MCP tool surface using `rmcp` v0.14.0.
