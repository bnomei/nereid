# Tasks — DRAFT-38-sequence-blocks

Meta:
- Spec: DRAFT-38-sequence-blocks — Sequence diagram blocks (alt/opt/loop/par)
- Depends on: spec:05-mermaid-format, spec:07-layout-engine, spec:13-diagram-renderers
- Global scope:
  - src/model/seq_ast.rs
  - src/format/mermaid/sequence.rs
  - src/render/sequence.rs
  - src/mcp/types.rs
  - src/mcp/server.rs

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T001: Model — add block/section objects to `SequenceAst` (owner: mayor) (scope: src/model/seq_ast.rs) (depends: -)
  - Completed_at: 2026-02-09T23:36:16Z
  - Completion note: Added `SequenceBlock`/`SequenceSection` tree types and a `blocks` field on `SequenceAst`, plus deterministic ID helpers for block/section allocation.
  - Validation result: `cargo test` (pass)

- [x] T002: Format — parse + export `alt/opt/loop/par` blocks (owner: worker:019c44c4-0ad0-72d3-81ef-4b2a0ab5cdda) (scope: src/format/mermaid/sequence.rs) (depends: T001)
  - Completed_at: 2026-02-10T00:11:17Z
  - Completion note: Added sequence block parsing/export (`alt/else`, `opt`, `loop`, `par/and`) with nesting/validation and canonical export ordering, plus tests.
  - Validation result: `cargo test` (pass)

- [x] T003: Render — draw block decorations on gap rows (owner: mayor) (scope: src/render/sequence.rs) (depends: T001)
  - Completed_at: 2026-02-10T01:18:04Z
  - Completion note: Added deterministic Unicode block decorations (frames + section separators) and made header labels non-destructive by overlaying them after canvas rendering to preserve lifeline connectivity; added snapshots for `alt/else`, `opt`, `loop`, `par/and`, plus a nested example.
  - Validation result: `cargo test` (pass)

- [x] T004: Annotated render — highlight spans for blocks/sections (owner: mayor) (scope: src/render/sequence.rs) (depends: T003)
  - Completed_at: 2026-02-10T01:18:04Z
  - Completion note: Added `seq/block` + `seq/section` highlight spans covering decoration cells (borders + label rows) and a regression test asserting block highlights include the header label text.
  - Validation result: `cargo test` (pass)

- [x] T005: MCP surface — include blocks in AST payloads (owner: worker:019c44e4-2f5d-7602-920a-0e364c6ab77a) (scope: src/mcp/types.rs, src/mcp/server.rs) (depends: T001)
  - Completed_at: 2026-02-10T01:18:04Z
  - Completion note: Extended `diagram.get_ast` sequence payload to include blocks/sections (including nested blocks) with deterministic ordering and `message_ids` as strings; verified via MCP server tests.
  - Validation result: `cargo test` (pass)
