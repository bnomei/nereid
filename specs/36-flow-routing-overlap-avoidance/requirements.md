# Requirements — DRAFT-36-flow-routing-overlap-avoidance

This spec improves flowchart edge routing correctness for “discussion-quality” diagrams:
- prevent connectors from running through node boxes
- reduce ambiguous connector overlaps in dense graphs where feasible

Normative protocol reference: `docs/protocol-01.md`

Checklist mapping: `docs/mm-as.md` item **#26**.

## Non-goals

- Changing flowchart semantics (nodes/edges/groups).
- Perfect aesthetics (this is still a baseline deterministic router).
- Implementing flow direction/orientation (Checklist #13 is explicitly out of scope).
- Performance-only optimizations (covered by `specs/31-perf-flow-routing/`).

## Requirements (EARS)

### Routing correctness

- WHEN `route_flowchart_edges_orthogonal(ast, layout)` is called, THE SYSTEM SHALL compute routes that avoid traversing through rendered node boxes (as projected into the routing model).
- WHEN a route cannot be found under the configured constraints, THE SYSTEM SHALL fall back deterministically to the existing baseline behavior (no panic/crash).
- WHEN multiple edges are routed, THE SYSTEM SHALL prefer routes that minimize connector overlap, using deterministic tie-breaking.

### Determinism

- WHEN routing is run multiple times on identical inputs, THE SYSTEM SHALL return identical routes.
- WHEN edges are routed, THE SYSTEM SHALL use a stable deterministic edge order (e.g., by `(from_node_id, to_node_id, edge_id)`).

### Testing

- THE SYSTEM SHALL include regression tests that assert connectors do not draw inside node box spans for representative fixtures.
- THE SYSTEM SHALL include at least one “dense routing” fixture demonstrating improved readability (fewer overlaps) while remaining deterministic.

