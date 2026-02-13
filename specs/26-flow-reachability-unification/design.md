# Design — 26-flow-reachability-unification

## Current state

- `src/query/flow.rs` already implements `reachable(ast, from_node_id)` (forward/outgoing reachability).
- `src/mcp/server.rs` implements `flow.reachable` with a local BFS plus hand-built outgoing/incoming adjacency maps to support `direction=out|in|both`.

This duplication is an easy drift point: behavior changes in one place won’t automatically be reflected in the other.

## Proposed design

### Query engine API

Add a direction-aware reachability function to `src/query/flow.rs`:

- `pub enum ReachDirection { Out, In, Both }` (crate-internal or public within `crate::query`)
- `pub fn reachable_with_direction(ast: &FlowchartAst, from: &ObjectId, direction: ReachDirection) -> Vec<ObjectId>`

Behavior:
- Return an empty list if `from` is missing.
- Include `from` in the result when present.
- Return node ids in a deterministic order (sorted).

Implementation sketch:
- Reuse existing `outgoing_adjacency` and `incoming_adjacency` helpers.
- Add a single internal BFS helper that takes an adjacency map and a start node and returns a visited set.
- Keep the existing `reachable(...)` as a small wrapper calling `reachable_with_direction(..., ReachDirection::Out)`.

### MCP handler refactor

In `src/mcp/server.rs` `flow.reachable`:
- Parse the `direction` string into `ReachDirection`.
- Call `crate::query::flow::reachable_with_direction(...)`.
- Map to canonical `flow/node` `ObjectRef` strings and keep existing sorting by string.

## Validation

- `cargo test --offline`
- Existing `flow.reachable` MCP unit tests must remain unchanged.
- Add/extend query engine tests to cover `in` and `both` semantics and ensure output ordering is deterministic.

