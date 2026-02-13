# Design — 04-ops-delta

Keep op definitions and application logic in `src/ops/`.

Design goals:
- Small, typed ops (add/update/remove) for sequence/flow objects, xrefs, and walkthrough nodes.
- A minimal delta schema first (added/removed/updated `ObjectRef`s), then refine later.

This spec should not require adding dependencies.

