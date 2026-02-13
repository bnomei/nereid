# Design — 01-diagram-tui-rust

This document describes the intended architecture for the Rust TUI app.

Normative protocol reference: `docs/protocol-01.md`

## High-level architecture

Data flow:

1. Load session folder → parse `.mmd` + reconcile `.meta.json` → AST
2. Layout AST → render to grid buffer (Unicode)
3. TUI shows rendered buffer + inspector panels
4. Agent uses MCP tools to query AST slices/deltas and propose structured ops
5. Apply ops → bump `rev` → compute `delta` → re-render → export on save

## Rust module layout (initial, single-crate)

Start as a single crate with explicit module boundaries; split into a workspace later if needed.

- `src/model/`
  - session/diagram/walkthrough/xref types
  - stable IDs + `ObjectRef`
- `src/ops/`
  - typed `ops[]` apply, revisioning, delta generation
- `src/query/`
  - typed primitives: flow reachability/paths/cycles, seq trace/search, session routes
- `src/format/mermaid/`
  - subset parser + canonical exporter for `sequenceDiagram` and `flowchart`
- `src/layout/`
  - sequence layout (deterministic timeline)
  - flowchart layout (layered/Sugiyama-style) + orthogonal routing (heuristics)
- `src/render/`
  - `Canvas` grid + Unicode drawing primitives
  - diagram renderers consume layout output
- `src/store/`
  - session folder IO (`nereid-session.meta.json`, `diagrams/*`, `walkthroughs/*`)
  - `.meta.json` reconciliation
- `src/tui/`
  - `ratatui` application, panes, selection, xref panel, walkthrough view
- `src/mcp/`
  - tool handlers matching `docs/protocol-01.md` (digest/delta/slice, apply ops, queries)

## Persistence format

Session folder:
- `nereid-session.meta.json` (session id, active diagram/walkthrough, UI hints)
- `diagrams/*.mmd`, `diagrams/*.ascii.txt`, `diagrams/*.meta.json`
- `walkthroughs/*.wt.json`, `walkthroughs/*.ascii.txt`

All paths in metadata are relative to the session folder.

## Revisioning, deltas, and conflicts

- Every diagram has `rev: u64`.
- All mutations provide `base_rev`; stale rev returns conflict.
- Agent refreshes via `delta` (preferred) rather than full AST.

## Walkthroughs

Walkthroughs are narrative maps built from AST objects and typed queries.

Design goals:
- short “step” nodes in-diagram + rich inspector content
- evidence-first (`refs: [ObjectRef]`)
- incremental refinement: add nodes/edges as users drill down

## TUI interaction model (sketch)

Panes:
- Diagram view (active diagram or active walkthrough)
- Inspector (details for selected `ObjectRef` or walkthrough node)
- XRefs panel (jump list; dangling TODO filter)
- Logs / agent chat (optional)

Selection:
- selecting an object yields a canonical `ObjectRef`
- UI drives agent context by exposing selection via MCP (`ui.get_selection`)

## MCP tool surface

Implement the minimal set in `docs/protocol-01.md`, prioritizing:
- session/diagram/walkthrough digests + deltas
- typed queries
- structured ops apply
- xref TODO retrieval
