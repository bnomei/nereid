# Tasks — DRAFT-35-mermaid-ascii-parity-rendering

Meta:
- Spec: DRAFT-35-mermaid-ascii-parity-rendering — Render parity subset (flow + sequence)
- Depends on: spec:13-diagram-renderers, spec:05-mermaid-format, spec:03-model-core
- Global scope:
  - src/model/seq_ast.rs
  - src/format/mermaid/sequence.rs
  - src/render/flowchart.rs
  - src/render/sequence.rs
  - src/render/text.rs

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T001: Flow — render Unicode arrowheads for directed edges (owner: mayor) (scope: src/render/flowchart.rs) (depends: -)
  - Completed_at: 2026-02-09T21:14:48Z
  - Completion note: Implemented deterministic flow edge arrowheads for routed + non-routed connectors using a render-time overlay (so box/junction merging stays intact), updated annotated highlights to include the arrowhead cell, and refreshed snapshot coverage.
  - Validation result: `cargo fmt` + `cargo test` (green)

- [x] T002: Flow — render edge labels on connectors (owner: mayor) (scope: src/render/flowchart.rs, src/render/text.rs) (depends: T001 optional)
  - Completed_at: 2026-02-09T21:14:48Z
  - Completion note: Added deterministic edge-label placement on connector paths (preferring merged horizontal segments, clipping with ellipsis, and avoiding arrowhead collisions) plus snapshot coverage; annotated highlights include an explicit label span.
  - Validation result: `cargo fmt` + `cargo test` (green)

- [x] T003: Flow — render `shape == \"round\"` as rounded-corner rectangles (owner: mayor) (scope: src/render/flowchart.rs) (depends: -)
  - Completed_at: 2026-02-09T21:14:48Z
  - Completion note: Render `shape == \"round\"` flow nodes with `╭╮╰╯` rounded corners (overlay), keeping sizing/layout unchanged; added a mixed-shape snapshot.
  - Validation result: `cargo fmt` + `cargo test` (green)

- [x] T004: Sequence — parse/export participant aliases (`participant A as Alice`) (owner: mayor) (scope: src/model/seq_ast.rs, src/format/mermaid/sequence.rs) (depends: -)
  - Completed_at: 2026-02-09T21:14:48Z
  - Completion note: Extended `SequenceParticipant` with an optional display-label override; added `participant <ident> as <label>` parse/export support (with conflict errors) and semantic round-trip tests; updated sequence rendering to display `label()` for aliased participants.
  - Validation result: `cargo fmt` + `cargo test` (green)

- [x] T005: Sequence — render dotted return arrows for `-->>` (owner: mayor) (scope: src/render/sequence.rs) (depends: -)
  - Completed_at: 2026-02-09T21:14:48Z
  - Completion note: Render return messages with a dashed Unicode stroke (`┈`) via post-processing the rendered text for the return message span (keeps geometry stable), and updated the return snapshot.
  - Validation result: `cargo fmt` + `cargo test` (green)

- [x] T006: MCP — expose participant labels + snapshot preserves aliases (owner: mayor) (scope: src/mcp/types.rs, src/mcp/server.rs) (depends: T004)
  - Completed_at: 2026-02-09T21:30:00Z
  - Completion note: Added `label` to `diagram.get_ast` sequence participants and updated `diagram.get_snapshot` Mermaid output to emit `participant <id> as <label>` plus canonical async arrow token (`-)`) so agent-facing outputs match the rendered diagram.
  - Validation result: `cargo test` (green)
