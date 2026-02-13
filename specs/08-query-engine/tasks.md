# Tasks — 08-query-engine

Meta:
- Spec: 08-query-engine — Typed query primitives
- Depends on: spec:03-model-core/T001
- Global scope:
  - src/query/

## In Progress

## Blocked

## Todo

## Done

- [x] T003: Session routes via derived meta-graph (owner: worker:019c376c-aa29-7233-a4e0-fbdc7143a73a) (scope: src/query/) (depends: T001,T002,spec:03-model-core/T002)
  - Started_at: 2026-02-07T09:26:42+00:00
  - DoD: route finder across diagrams using xrefs + adjacency; tests cover simple cross-diagram route.
  - Validation: `cargo test`
  - Escalate if: route semantics unclear; align strictly to `docs/protocol-01.md` and document assumptions.
  - Completed_at: 2026-02-07T09:38:51+00:00
  - Completion note: Implemented deterministic session route finding via a derived meta-graph combining flow node edge adjacency, sequence message order adjacency, and bidirectional xref neighbors; added a unit test verifying a simple cross-diagram route.
  - Validation result: `cargo test` (ok)

- [x] T002: Flow query primitives (owner: worker:019c3757-bd1a-7d33-87b6-0af329957e96) (scope: src/query/) (depends: spec:03-model-core/T001)
  - Started_at: 2026-02-07T09:03:49+00:00
  - DoD: reachable/paths/cycles/dead-ends; tests cover small graphs.
  - Validation: `cargo test`
  - Escalate if: path enumeration explodes; implement shortest-path + capped alternates first.
  - Completed_at: 2026-02-07T09:23:04+00:00
  - Completion note: Implemented FlowchartAst query primitives: forward reachable, bounded shortest+alternate path enumeration, SCC-based cycle groups, and dead-end detection, with deterministic unit tests on small graphs.
  - Validation result: `cargo test` (ok)

- [x] T001: Sequence query primitives (owner: worker:019c3734-04fa-7580-a33e-a1aa05b71820) (scope: src/query/) (depends: spec:03-model-core/T001)
  - Started_at: 2026-02-07T08:24:28+00:00
  - DoD: message search + trace before/after + pair filtering; tests cover deterministic output.
  - Validation: `cargo test`
  - Escalate if: sequence blocks (`alt/loop`) required; ship message-only queries first.
  - Completed_at: 2026-02-07T08:35:43+00:00
  - Completion note: Added deterministic `SequenceAst` query primitives for substring message search, ordered traces before/after a message, and filtering by `(from,to)`, with unit tests using small fixtures to lock stable ordering.
  - Validation result: `cargo test` (ok)
