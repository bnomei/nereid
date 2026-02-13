# Requirements — 26-flow-reachability-unification

This spec removes duplicated flow reachability logic between:

- MCP tool handler `flow.reachable` in `src/mcp/server.rs`
- Query primitive `crate::query::flow::reachable` in `src/query/flow.rs`

The goal is to make the query engine the single source of truth for reachability and have MCP call into it, while preserving existing behavior and determinism.

## Requirements (EARS)

- THE SYSTEM SHALL expose a query-engine reachability API that supports `out`, `in`, and `both` reachability on a `FlowchartAst`.
- THE SYSTEM SHALL preserve existing `crate::query::flow::reachable` semantics (it SHALL continue to mean `out` reachability and include the start node when present).
- THE SYSTEM SHALL implement MCP tool `flow.reachable` in terms of the query-engine API (no local BFS implementation in `src/mcp/server.rs`).
- THE SYSTEM SHALL keep `flow.reachable` responses deterministic and unchanged for existing tests (lexicographically-sorted `ObjectRef` strings; includes start when present; empty list when `from_node_id` is missing).

## Non-goals

- This spec does not change the protocol surface of `flow.reachable` (parameter names and response shape stay the same).
- This spec does not refactor other flow tools (`flow.unreachable`, `flow.paths`, etc.) unless it is a necessary byproduct of unifying reachability.

