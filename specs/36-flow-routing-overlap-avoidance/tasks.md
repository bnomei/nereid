# Tasks — DRAFT-36-flow-routing-overlap-avoidance

Meta:
- Spec: DRAFT-36-flow-routing-overlap-avoidance — Flow routing correctness (avoid nodes, reduce overlaps)
- Depends on: spec:07-layout-engine, spec:13-diagram-renderers (related: specs/31-perf-flow-routing)
- Global scope:
  - src/layout/flowchart.rs
  - src/render/flowchart.rs
  - src/model/fixtures.rs
  - benches/flow.rs

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T001: Routing — constrain intermediate traversal to lanes (owner: worker:019c4530-80bb-71d2-ae0a-fd6e24849eaf) (scope: src/layout/flowchart.rs) (depends: -)
  - Started_at: 2026-02-10T01:34:52Z
  - Completed_at: 2026-02-10T01:43:54Z
  - Completion note: Updated orthogonal routing to forbid intermediate traversal through even/even “node cells” (streets-only), while still permitting `start`/`goal`, and added a regression test asserting intermediate path points remain in streets.
  - Validation result: `cargo test` (green)

- [x] T002: Routing — reduce connector overlap via soft occupancy (owner: mayor) (scope: src/layout/flowchart.rs) (depends: T001)
  - Started_at: 2026-02-10T01:45:17Z
  - Recovered_from_owner: worker:019c453a-17f9-71f3-b7b0-88467367a2ba
  - Recovered_at: 2026-02-10T02:07:44Z
  - Completed_at: 2026-02-10T02:18:09Z
  - Completion note: Verified stable edge ordering and soft-occupancy routing (density-gated) and added a regression test showing parallel edges prefer distinct detours once earlier routes mark segments as occupied.
  - Validation result: `cargo test` (green)

- [x] T003: Projection-aware tests — assert connectors don’t enter node boxes (owner: worker:019c453a-1d94-7b83-aeb4-f2f4a6a8c6ce) (scope: src/render/flowchart.rs, src/model/fixtures.rs) (depends: T001)
  - Started_at: 2026-02-10T01:45:17Z
  - Completed_at: 2026-02-10T02:18:09Z
  - Completion note: Added a projection-aware test helper that asserts routed connector spans never enter any node box interior cell, plus a dedicated regression fixture for overlap avoidance.
  - Validation result: `cargo test` (green)
