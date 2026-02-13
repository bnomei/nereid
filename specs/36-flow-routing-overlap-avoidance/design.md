# Design — DRAFT-36-flow-routing-overlap-avoidance

## Overview

The current router treats only node anchor points as obstacles. This is fast and deterministic, but in dense graphs it can:
- choose routes that visually intersect node boxes (after canvas projection), and/or
- collapse many connectors into the same lanes, producing ambiguous overlaps.

This spec upgrades the routing model while preserving:
- deterministic output
- safe fallback behavior

## Strategy

### 1) Lane-only intermediate routing (node-box avoidance baseline)

In the flow routing grid, nodes live at even/even coordinates:
- `x = layer * 2`
- `y = index_in_layer * 2`

Baseline rule:
- Allow even/even coordinates only for the **start** and **goal** node anchors.
- Treat all other even/even cells (and optionally all even-x “node columns”) as blocked for intermediate traversal.

Effect:
- Routes stay in the “streets” between nodes (odd coordinates), which map naturally to canvas lanes between box columns/rows.
- This substantially reduces the chance of a route projecting into a node’s rendered box.

### 2) Soft occupancy to reduce overlaps (second-phase)

After routing each edge in deterministic order:
- mark its traversed grid segments as “occupied”
- prefer unoccupied segments for subsequent routes

Implementation options:
- weighted search (Dijkstra with small integer costs)
- multi-queue BFS approximation (e.g., expand unoccupied first, then occupied)

Keep deterministic behavior by:
- stable edge routing order
- stable neighbor expansion order (already exists)
- deterministic tie-breaking

### 3) Projection-aware validation (guardrail)

Add a debug-only (or test-only) validation:
- project routed spans to canvas spans (same adapter as renderer)
- assert no connector span overlaps node box spans

This ensures routing correctness is measured in the same coordinate system humans see.

## Testing approach

- Add a snapshot fixture that previously produced a “connector crosses a node box” failure.
- Add a dense-graph fixture where overlap is reduced by occupancy-aware routing.
- Add a property-style test:
  - For each node box span, assert the routed connector spans do not include any cell strictly inside the box border.

## Performance note

This is a correctness spec, but routing must remain usable.

If occupancy-aware routing regresses badly on `flow.route/*` benches, stage it behind:
- a conservative heuristic (only enable on “dense” graphs), or
- a limit on attempted bounds before falling back.

