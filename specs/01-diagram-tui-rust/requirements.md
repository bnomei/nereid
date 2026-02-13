# Requirements — 01-diagram-tui-rust

This spec describes the requirements for a Rust TUI application that supports collaborative creation and reasoning over Mermaid-like diagrams using a stable AST, with ASCII/Unicode rendering and an MCP tool surface.

Normative protocol reference: `docs/protocol-01.md`

## Goals

- Human and agent collaborate on **sequence diagrams** and **flowcharts** without redrawing/re-reasoning each turn.
- The system is **local-only**, **100% Rust**, and renders to **ASCII/Unicode** (no browser/SVG/JS).
- Diagrams are **stateful artifacts**: queryable, explainable, regenerable.
- The system can create **walkthroughs** (shareable drill-down narratives) from the AST.

## Non-goals (initially)

- Supporting every Mermaid diagram type.
- Format-preserving Mermaid round-tripping (we output canonical `.mmd`).
- Styling/themes comparable to Mermaid in browsers.

## Requirements (EARS)

### Sessions and persistence

- THE SYSTEM SHALL represent work as a `Session` containing multiple diagrams and walkthroughs.
- THE SYSTEM SHALL persist a session as a folder with relative-path metadata so the folder can be renamed/moved without breaking references.
- WHEN a session is saved THE SYSTEM SHALL export each diagram to canonical Mermaid `.mmd`.
- WHEN a session is saved THE SYSTEM SHALL export each diagram to `.ascii.txt` (export-on-save).
- WHEN a session is saved THE SYSTEM SHALL persist stable IDs and mappings in a sidecar metadata file (e.g. `.meta.json`).

### Diagram types and scope

- THE SYSTEM SHALL support sequence diagrams as a first-class diagram kind.
- THE SYSTEM SHALL support flowcharts using modern Mermaid `flowchart` syntax only (not legacy `graph`).
- THE SYSTEM SHALL treat the diagram AST as the single source of truth for rendering and reasoning.

### Stable identity and incremental change

- THE SYSTEM SHALL assign stable IDs to all addressable diagram objects (participants/messages; nodes/edges; xrefs; walkthrough nodes/edges).
- THE SYSTEM SHALL expose a stable `ObjectRef` addressing scheme usable by both UI and MCP tools.
- THE SYSTEM SHALL track per-diagram revisions (`rev`) that increment on every applied mutation.
- WHEN a mutation is attempted with a stale `base_rev` THE SYSTEM SHALL reject it with a conflict response that enables recovery via digest/delta.
- THE SYSTEM SHALL provide digests, deltas, and slices so agents can refresh context without full AST/ASCII re-fetches.

### Rendering

- THE SYSTEM SHALL render diagrams to a fixed-width character grid using Unicode box-drawing characters by default.
- THE SYSTEM SHALL keep rendering separate from UI logic (UI consumes rendered buffers).
- WHEN rendering fails due to size constraints THE SYSTEM SHALL report an actionable error (e.g. canvas too small) rather than silently truncating.

### Query (“ask graph”) primitives

- THE SYSTEM SHALL provide typed query primitives for flowcharts including: reachable nodes, paths, cycle detection, and dead-end identification.
- THE SYSTEM SHALL provide typed query primitives for sequence diagrams including: message search, tracing before/after, and filtering by participant pair.
- THE SYSTEM SHALL provide cross-diagram query primitives via XRefs and a derived session meta-graph (routes).

### XRefs and TODOs

- THE SYSTEM SHALL support cross-diagram links (XRefs) between any two `ObjectRef`s.
- WHEN an XRef endpoint is missing THE SYSTEM SHALL mark it as dangling and keep it retrievable via a TODO-style query/filter.
- THE SYSTEM SHALL present XRefs as first-class UI elements (panel + jump list).

### Walkthroughs

- THE SYSTEM SHALL support walkthroughs as shareable artifacts stored in the session folder.
- THE SYSTEM SHALL allow walkthrough nodes to reference underlying `ObjectRef`s as evidence.
- WHEN a walkthrough is saved THE SYSTEM SHALL export a walkthrough snapshot to `.ascii.txt`.

### MCP collaboration

- THE SYSTEM SHALL expose an MCP-facing tool surface aligned with `docs/protocol-01.md`.
- THE SYSTEM SHALL expose structured mutation operations (`ops[]`) rather than editing ASCII or Mermaid text directly.
- THE SYSTEM SHALL allow the agent to operate on the AST using typed queries (digest/delta/slice) to avoid re-reasoning.

### Constraints

- THE SYSTEM SHALL be implemented in Rust without invoking external layout engines (Graphviz/ELK) or JS runtimes.
- THE SYSTEM SHALL not perform network calls.

