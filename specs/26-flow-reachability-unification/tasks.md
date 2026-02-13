# Tasks — 26-flow-reachability-unification

Meta:
- Spec: 26-flow-reachability-unification — Unify flow reachability between MCP and query engine
- Depends on: spec:08-query-engine/T002, spec:12-mcp/T020
- Global scope:
  - src/query/flow.rs
  - src/mcp/server.rs

## In Progress

## Blocked

## Todo

## Done

- [x] T001: Add direction-aware reachability in query engine (owner: perf-agent) (scope: src/query/flow.rs) (depends: -)
  - Started_at: 2026-02-08T16:39:25+00:00
  - Completed_at: 2026-02-08T16:43:20+00:00
  - Completion note: Added `ReachDirection` + `reachable_with_direction` to `src/query/flow.rs` with deterministic sorted output.
  - Validation result: `cargo test --offline` (ok)

- [x] T002: Refactor query `reachable` to delegate (owner: perf-agent) (scope: src/query/flow.rs) (depends: T001)
  - Completed_at: 2026-02-08T16:43:20+00:00
  - Completion note: Implemented `reachable` as a wrapper over `reachable_with_direction(..., ReachDirection::Out)` and added `in|both` unit tests.
  - Validation result: `cargo test --offline` (ok)

- [x] T003: Refactor MCP `flow.reachable` to call query engine (owner: perf-agent) (scope: src/mcp/server.rs) (depends: T001)
  - Completed_at: 2026-02-08T16:43:20+00:00
  - Completion note: Removed duplicated BFS/adjacency logic from `flow.reachable` and delegated to `crate::query::flow::reachable_with_direction`.
  - Validation result: `cargo test --offline` (ok)

- [x] T004: Add regression tests for `in|both` parity (owner: perf-agent) (scope: src/query/flow.rs, src/mcp/server.rs) (depends: T001,T003)
  - Completed_at: 2026-02-08T16:43:20+00:00
  - Completion note: Added query-engine tests for `ReachDirection::In` and `ReachDirection::Both`; existing MCP reachability tests remain unchanged.
  - Validation result: `cargo test --offline` (ok)
