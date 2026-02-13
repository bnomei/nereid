# Tasks — 13-diagram-renderers

Meta:
- Spec: 13-diagram-renderers — Diagram renderers
- Depends on: spec:06-render-canvas/T001
- Global scope:
  - src/render/

## In Progress

## Blocked

## Todo

## Done

- [x] T004: Add `render_diagram_unicode(&Diagram)` helper (owner: worker:019c37ba-acb0-7690-a3a8-99a55ecec52e) (scope: src/render/) (depends: T001,T002,spec:07-layout-engine/T001,spec:07-layout-engine/T002)
  - Started_at: 2026-02-07T10:51:38+00:00
  - DoD: helper selects renderer by diagram kind, computes layout, returns Unicode text; unit tests cover both kinds.
  - Validation: `cargo test`
  - Escalate if: error type gets too broad; keep a single enum wrapping underlying errors.
  - Completed_at: 2026-02-07T10:58:26+00:00
  - Completion note: Added `render_diagram_unicode(&Diagram)` helper that selects by AST (Sequence/Flowchart), computes layout via `layout_sequence`/`layout_flowchart`, and delegates to existing Unicode renderers; added snapshot tests for both variants; `cargo test` ok.
  - Validation result: `cargo test` (ok)

- [x] T003: Flowchart renderer uses routed polylines when available (owner: worker:019c37a1-b5a7-76a2-a636-d86ead54468a) (scope: src/render/) (depends: T002,spec:07-layout-engine/T003)
  - Started_at: 2026-02-07T10:24:25+00:00
  - DoD: render uses `route_flowchart_edges_orthogonal` output to draw edges; snapshot test updated/added.
  - Validation: `cargo test`
  - Escalate if: router coordinate system mismatch; keep renderer baseline and add an adapter layer.
  - Completed_at: 2026-02-07T10:48:01+00:00
  - Completion note: Flowchart renderer now consumes `route_flowchart_edges_orthogonal` and renders routed orthogonal polylines by adapting `GridPoint` routes into deterministic canvas lane coordinates, drawing segments plus stubs into node boxes; canvas height expands to fit routed detours. Added a snapshot test covering obstacle-avoidance routing. Validation: `cargo test` (ok).
  - Validation result: `cargo test` (ok)

- [x] T002: Flowchart renderer baseline (owner: worker:019c3791-baa7-7ab1-9c2d-746ecf7906f5) (scope: src/render/) (depends: spec:07-layout-engine/T002,spec:06-render-canvas/T001,spec:03-model-core/T001)
  - Started_at: 2026-02-07T10:06:57+00:00
  - DoD: render node boxes + basic orthogonal edges using layered layout output; deterministic snapshot tests for a small DAG.
  - Validation: `cargo test`
  - Escalate if: routing is required; allow edges as straight connectors for the baseline and defer routing to `07-layout-engine/T003`.
  - Completed_at: 2026-02-07T10:18:07+00:00
  - Completion note: Added baseline deterministic Unicode flowchart renderer in `src/render/` consuming `FlowchartAst` + `FlowchartLayout`; renders per-layer node boxes and minimal orthogonal connectors into `Canvas`; includes a snapshot-style test for a small DAG.
  - Validation result: `cargo test` (ok)

- [x] T001: Sequence diagram renderer baseline (owner: worker:019c3783-8f4c-7f70-8f1f-f8eeb108e20e) (scope: src/render/) (depends: spec:07-layout-engine/T001,spec:06-render-canvas/T001,spec:03-model-core/T001)
  - Started_at: 2026-02-07T09:51:31+00:00
  - DoD: render participants + lifelines + basic message arrows to Unicode text; deterministic snapshot tests for 1–2 fixtures.
  - Validation: `cargo test`
  - Escalate if: layout/render boundaries get unclear; keep renderer consuming `SequenceLayout` output only.
  - Completed_at: 2026-02-07T10:03:43+00:00
  - Completion note: Added baseline deterministic Unicode sequence renderer in `src/render/` consuming `SequenceLayout` + AST labels; renders participant boxes, lifelines, and message arrows; includes 2 snapshot-style tests locking output.
  - Validation result: `cargo test` (ok)
