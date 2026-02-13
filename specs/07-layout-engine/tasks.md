# Tasks — 07-layout-engine

Meta:
- Spec: 07-layout-engine — Layout + routing
- Depends on: spec:03-model-core/T001
- Global scope:
  - src/layout/

## In Progress

## Blocked

## Todo

## Done

- [x] T004: Remove panic paths from flowchart routing (owner: worker:019c3871-9267-7f42-a9b5-d99660be8160) (scope: src/layout/) (depends: T003)
  - Started_at: 2026-02-07T14:10:36+00:00
  - DoD: Routing no longer panics in normal library use; failures yield a deterministic fallback polyline (or a typed error plumbed to callers if already `Result`-based); unit tests cover at least one forced “no path” case without panicking.
  - Validation: `cargo test --offline`
  - Completed_at: 2026-02-07T14:52:41+00:00
  - Completion note: Removed panic-based crash paths from flowchart edge routing by returning deterministic fallback polylines when endpoints are missing or no route is found after bounded search; added unit tests for both the “no path” fallback and missing-placement cases.
  - Validation result: `cargo test --offline` (ok)

- [x] T003: Orthogonal edge routing baseline (owner: worker:019c3791-b4ec-70e1-aed8-cdc772192a5d) (scope: src/layout/) (depends: T002)
  - Started_at: 2026-02-07T10:06:57+00:00
  - DoD: simple router avoids nodes; supports labeled edges later; tests for basic paths.
  - Validation: `cargo test`
  - Escalate if: router overlap dominates; keep minimal “good enough” router and document limitations.
  - Completed_at: 2026-02-07T10:20:02+00:00
  - Completion note: Implemented deterministic orthogonal edge routing baseline for flowcharts (`route_flowchart_edges_orthogonal`) that consumes `FlowchartAst` + `FlowchartLayout`, routes on an integer grid derived from `(layer,index)` (nodes as obstacles, endpoints allowed), and returns per-edge polyline points. Added `GridPoint` + `FlowchartLayout::node_grid_point` helper, exported via `layout::mod`, and added unit tests covering straight routing and obstacle avoidance.
  - Validation result: `cargo test` (ok)

- [x] T002: Flowchart layered layout baseline (owner: worker:019c377d-274f-7a80-804e-6606f65ba103) (scope: src/layout/) (depends: spec:03-model-core/T001,spec:05-mermaid-format/T002)
  - Started_at: 2026-02-07T09:44:30+00:00
  - DoD: layer assignment + node ordering for simple DAGs; tests for predictable output.
  - Validation: `cargo test`
  - Escalate if: cycles complicate baseline; ship DAG-first and add cycle handling later.
  - Completed_at: 2026-02-07T09:54:37+00:00
  - Completion note: Implemented deterministic flowchart layered layout (`layout_flowchart`) with edge validation (unknown-node errors), cycle detection, longest-path layer assignment, and deterministic per-layer ordering (single barycenter sweep). Added unit tests locking predictable layer/order output and error cases.
  - Validation result: `cargo test` (ok)

- [x] T001: Sequence layout baseline (owner: worker:019c376c-a3fa-7df3-b685-3a04f9a14cfd) (scope: src/layout/) (depends: spec:03-model-core/T001,spec:05-mermaid-format/T001,spec:06-render-canvas/T001)
  - Started_at: 2026-02-07T09:26:42+00:00
  - DoD: deterministic participant/message positioning data structure; unit tests for small cases.
  - Validation: `cargo test`
  - Escalate if: rendering concerns leak into layout; keep output as coordinates only.
  - Completed_at: 2026-02-07T09:38:51+00:00
  - Completion note: Implemented deterministic coordinates-only sequence layout. Participants are assigned columns by `ObjectId` order; messages are assigned rows by `(order_key, message_id)` order. Added unit tests locking deterministic ordering and verifying unknown participant errors.
  - Validation result: `cargo test` (ok)
