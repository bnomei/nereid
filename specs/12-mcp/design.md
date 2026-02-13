# Design — 12-mcp

Keep MCP integration in `src/mcp/`.

This spec depends on picking an MCP library in `02-dependencies/T005` and on core query/ops/model types.

Start with a minimal tool set:
- `session.list_diagrams`
- `diagram.get_digest`
- `diagram.get_delta`
- `diagram.apply_ops` (minimal)

Transport note:
- Use `rmcp` v0.14.0 and its stdio transport for a real MCP implementation (no temporary JSON harness).
