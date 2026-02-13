# Requirements — DRAFT-35-mermaid-ascii-parity-rendering

Implement a focused subset of `mermaid-ascii` feature parity that directly improves **human+agent discussion** over diagrams (Protocol 01).

Checklist items (from `docs/mm-as.md`):
- #14 Flow: labeled edges rendering
- #22 Flow: arrowheads (Unicode)
- #23 Flow: edge labels on connectors
- #20 Flow: shapes — **only** render `round` as rounded-corner rectangles (no diamonds/circles)
- #32 Seq: dotted return arrows (`-->>`) rendering
- #35 Seq: participant aliases (`participant A as Alice`)

## Protocol constraints (excerpt)

The implementation must preserve these Protocol 01 properties:
- AST is the source of truth; text is a projection.
- Stable `ObjectRef` identifiers remain valid across edits.
- Rendering is deterministic for identical inputs.
- Unicode output is the primary mode (ASCII-only can be added later).

## Non-goals

- Add an mmd-from-stdin/file CLI like `mermaid-ascii`.
- Add ASCII-only rendering mode.
- Add Mermaid `graph` header support (Protocol 01 locked decision: modern `flowchart` only).
- Add nested flow `subgraph` support.
- Add flow node shapes beyond “rounded corners on rectangles”.
- Add sequence activation bars.
- Add sequence `note`, `loop`, `alt`, `opt`, `par` blocks.

## Requirements (EARS)

### Flowchart rendering

- WHEN rendering a flowchart edge `from → to`, THE SYSTEM SHALL render a Unicode arrowhead indicating the direction of the edge.
- WHEN rendering a flowchart edge with a label, THE SYSTEM SHALL render the label text on the connector path in a deterministic position.
- WHEN a flowchart edge label cannot fit on the chosen connector segment, THE SYSTEM SHALL clip the label deterministically (e.g. ellipsis) without overlapping node boxes.
- WHEN rendering a flowchart node whose model shape is `round`, THE SYSTEM SHALL render its box using rounded-corner Unicode glyphs; OTHERWISE it SHALL use the existing hard-corner box rendering.

### Sequence parsing/export

- WHEN parsing a `sequenceDiagram` participant alias declaration `participant <alias> as <display>`, THE SYSTEM SHALL create/patch a participant keyed by `<alias>` and store `<display>` as the participant’s rendered label.
- WHEN exporting a sequence diagram to canonical `.mmd`, THE SYSTEM SHALL emit `participant <alias> as <display>` for aliased participants and SHALL use `<alias>` identifiers in message lines.

### Sequence rendering

- WHEN rendering a return message (`-->>`), THE SYSTEM SHALL render the message arrow using a dotted/dashed Unicode stroke while preserving message direction and label placement.

### Determinism + highlighting

- WHEN rendering flowcharts and sequences with these features, THE SYSTEM SHALL keep output deterministic for identical AST+layout inputs.
- WHEN producing an annotated render, THE SYSTEM SHALL include arrowheads and edge/message label glyphs in the highlight spans for the owning object and SHALL clamp spans to the returned text.

