# Tasks — DRAFT-37-inline-notes

Meta:
- Spec: DRAFT-37-inline-notes — Inline notes in object boxes + TUI toggle
- Depends on: spec:09-session-store, spec:13-diagram-renderers, spec:19-mcp-protocol-surface
- Global scope:
  - src/model/flow_ast.rs
  - src/model/seq_ast.rs
  - src/ops/mod.rs
  - src/store/session_folder.rs
  - src/render/diagram.rs
  - src/render/flowchart.rs
  - src/render/sequence.rs
  - src/tui/mod.rs
  - src/mcp/types.rs
  - src/mcp/server.rs

## In Progress

- (none)

## Blocked

- (none)

## Todo

## Done

- [x] T001: Model — add optional `note` fields for nodes/participants (owner: mayor) (scope: src/model/flow_ast.rs, src/model/seq_ast.rs) (depends: -)
  - Completed_at: 2026-02-09T23:36:16Z
  - Completion note: Added `note: Option<String>` fields with setters/getters for flow nodes and sequence participants; defaults remain `None` so baseline behavior is unchanged.
  - Validation result: `cargo test` (pass)

- [x] T002: Ops + MCP — set/clear notes via operations (owner: worker:019c44c4-2e28-7503-a3f9-34beb1de009d) (scope: src/ops/mod.rs, src/mcp/types.rs, src/mcp/server.rs) (depends: T001)
  - Completed_at: 2026-02-10T00:01:15Z
  - Completion note: Added explicit ops + MCP surface to set/clear flow-node and seq-participant notes and exposed notes in `diagram.get_ast`, including tests.
  - Validation result: `cargo test` (pass)

- [x] T003: Persistence — roundtrip notes across save/load (owner: worker:019c44c4-1c95-7a71-b874-8cab82d5a764) (scope: src/store/session_folder.rs) (depends: T001)
  - Completed_at: 2026-02-10T00:01:15Z
  - Completion note: Persisted flow-node and seq-participant notes into diagram sidecar meta and re-applied them on load, with a roundtrip regression test.
  - Validation result: `cargo test` (pass)

- [x] T004: Rendering — show notes inside boxes under labels (owner: mayor) (scope: src/render/diagram.rs, src/render/flowchart.rs, src/render/sequence.rs) (depends: T001)
  - Completed_at: 2026-02-10T00:01:15Z
  - Completion note: Added `RenderOptions { show_notes }` and options-aware diagram/flow/seq renderers; when enabled, boxes gain a note row under the label with deterministic clipping; added notes-on/off snapshots.
  - Validation result: `cargo test` (pass)

- [x] T005: TUI — toggle notes with `n` in diagram focus (owner: mayor) (scope: src/tui/mod.rs) (depends: T004)
  - Completed_at: 2026-02-10T00:10:11Z
  - Completion note: Added an app-local `show_notes` flag; when focused on Diagram and search is inactive, `n` toggles notes and re-renders; status help now shows `n notes:on/off`.
  - Validation result: `cargo test` (pass)
