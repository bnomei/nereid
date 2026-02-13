# Tasks — 23-diagram-meta-roundtrip

Meta:
- Spec: 23-diagram-meta-roundtrip — Diagram sidecar round-trip (stable IDs + non-Mermaid fields)
- Depends on: spec:09-session-store/T006
- Global scope:
  - src/store/

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T004: Regression tests — xrefs remain valid across save→load for message/edge endpoints (owner: mayor) (scope: src/store/) (depends: T002,T003)
  - Started_at: 2026-02-08T18:00:00+00:00
  - Completed_at: 2026-02-08T18:02:01+00:00
  - Completion note: Added a regression test ensuring xrefs targeting `flow/edge` and `seq/message` remain `ok` after save→load (no dangling status due to ID renumbering).
  - Validation result: `cargo test --offline` (ok)

- [x] T003: Sequence reconciliation — stable message IDs (owner: mayor) (scope: src/store/) (depends: T001)
  - Started_at: 2026-02-08T17:52:47+00:00
  - Completed_at: 2026-02-08T17:58:47+00:00
  - Completion note: Extended diagram sidecar schema to persist sequence message fingerprints → stable `message_id`, reconciled parsed sequence diagrams on load to restore stable message IDs (even when parse order changes), and ensured new messages never reuse sidecar IDs; added focused unit tests (parse-order change + no ID reuse).
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok)

- [x] T002: Flowchart reconciliation — stable edge IDs + restore non-Mermaid `FlowEdge.style` (owner: mayor) (scope: src/store/) (depends: T001)
  - Started_at: 2026-02-08T16:14:15+00:00
  - Recovered_from_owner: worker:019c3e09-3763-71a0-8421-73a832ca74d9
  - Recovered_at: 2026-02-08T16:49:33+00:00
  - Completed_at: 2026-02-08T17:45:35+00:00
  - Completion note: Extended diagram sidecar schema to persist flow edge fingerprints → stable `edge_id` + `style`, reconciled parsed flowcharts on load to restore stable edge IDs and `FlowEdge.style`, and ensured new edges never reuse sidecar IDs; added focused unit tests (style round-trip + no ID reuse).
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok)

- [x] T001: Wire diagram sidecar save/load into `SessionFolder::{save_session,load_session}` (owner: worker:019c3df6-26a1-7a60-b6a6-d92ca8b5d7b1) (scope: src/store/) (depends: spec:09-session-store/T006)
  - Started_at: 2026-02-08T15:51:39+00:00
  - Completed_at: 2026-02-08T16:06:30+00:00
  - Completion note: Wired per-diagram `diagrams/<stem>.meta.json` sidecar writes into `SessionFolder::save_session` (atomic) and optional loads into `SessionFolder::load_session` (missing sidecars are ignored for backwards compatibility); added unit tests for sidecar presence and load behavior (missing/invalid).
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 219 passed)
