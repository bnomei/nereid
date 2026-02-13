# Tasks — 28-session-routes-adjacency

Meta:
- Spec: 28-session-routes-adjacency — Expand session route adjacency beyond nodes/messages
- Depends on: spec:08-query-engine/T003
- Related: spec:24-mcp-query-extensions/T002
- Global scope:
  - src/query/session_routes.rs

## In Progress

## Blocked

## Todo

- (none)

## Done

- [x] T001: Decide direction semantics for structural edges (owner: perf-agent) (scope: specs/28-session-routes-adjacency/) (depends: -)
  - Completed_at: 2026-02-08T17:45:35+00:00
  - Completion note: Chose bidirectional structural edges (edge↔endpoint nodes, participant↔messages) for navigation semantics; recorded the decision in `design.md`.
  - Validation result: n/a (decision)

- [x] T002: Add flow edge and seq participant nodes to adjacency (owner: perf-agent) (scope: src/query/session_routes.rs) (depends: T001)
  - Completed_at: 2026-02-08T17:45:35+00:00
  - Completion note: Extended derived adjacency to include `flow/edge/*` and `seq/participant/*` nodes and bidirectional structural edges to their related nodes/messages, while keeping existing directed flow/node and seq/message adjacency.
  - Validation result: `cargo test --offline` (ok)

- [x] T003: Add unit tests for new adjacency behavior (owner: perf-agent) (scope: src/query/session_routes.rs) (depends: T002)
  - Completed_at: 2026-02-08T17:45:35+00:00
  - Completion note: Added unit tests for edge↔endpoint connectivity, participant↔message connectivity, and a cross-diagram route involving these endpoint kinds; existing route test remains unchanged.
  - Validation result: `cargo test --offline` (ok)
