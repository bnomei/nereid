# Requirements — 19-mcp-protocol-surface

This spec completes the remaining MCP tools listed in `docs/protocol-01.md` §8 that are still missing from the current implementation.

Mayor-only protocol reference: `docs/protocol-01.md` (workers must not be sent to this doc; extract needed excerpts into task `Context:` blocks)

## Requirements (EARS)

- THE SYSTEM SHALL expose `seq.messages` as a typed MCP tool for listing sequence messages (deterministic ordering).
- THE SYSTEM SHALL expose `flow.unreachable` as a typed MCP tool for listing unreachable flow nodes (deterministic ordering).
- THE SYSTEM SHALL expose `diagram.get_ast` as an explicit MCP tool that returns a JSON-friendly AST representation (potentially large).
- THE SYSTEM SHALL expose `diagram.propose_ops` as a non-mutating MCP tool that validates ops against `base_rev` and returns the predicted delta/rev change.

