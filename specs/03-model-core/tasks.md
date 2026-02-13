# Tasks — 03-model-core

Meta:
- Spec: 03-model-core — Core model types
- Depends on: spec:01-diagram-tui-rust/T002
- Global scope:
  - src/model/

## In Progress

## Blocked

## Todo

## Done

- [x] T006: Add `SequenceParticipant` setters (owner: worker:019c38c0-4959-71b1-b3e4-31e49f0551ce) (scope: src/model/) (depends: T001)
  - Started_at: 2026-02-07T15:37:39+00:00
  - Context: `SeqOp::UpdateParticipant` currently reconstructs `SequenceParticipant` and drops unrelated fields (e.g. `role`). Add public setters on `SequenceParticipant` so ops can patch in place and preserve metadata (aligning with `FlowNode`/`FlowEdge` APIs).
  - DoD: `SequenceParticipant` exposes `set_mermaid_name(...)` and `set_role(...)` (or equivalent) without exposing fields; unit tests cover basic update behavior.
  - Validation: `cargo test --offline`
  - Escalate if: API naming/ownership is unclear; keep it consistent with `FlowNode`/`FlowEdge` setter patterns.
  - Completed_at: 2026-02-07T15:42:27+00:00
  - Completion note: Added in-place `SequenceParticipant` setter APIs (`set_mermaid_name`, `set_role`) and a unit test verifying role preservation across name updates and explicit role clearing.
  - Validation result: `cargo test --offline` (ok)

- [x] T005: Add Flowchart node/edge constructors for richer Mermaid semantics (owner: worker:019c3743-b5fb-7882-b749-0e53c64f8652) (scope: src/model/) (depends: T001)
  - Started_at: 2026-02-07T08:42:02+00:00
  - DoD: `FlowNode`/`FlowEdge` can be constructed/updated with `mermaid_id`, `shape`, edge `label`, and edge `style` via public APIs (no direct field access); unit tests cover basic construction.
  - Validation: `cargo test`
  - Escalate if: API design becomes unclear; keep constructors minimal and defer enums for shape/style until parser needs them.
  - Completed_at: 2026-02-07T09:01:47+00:00
  - Completion note: Added `FlowNode`/`FlowEdge` public constructors + setters for Mermaid semantics (`mermaid_id`, `shape`, edge `label`/`style`) without exposing fields; added unit tests for construction/update behavior.
  - Validation result: `cargo test` (ok)

- [x] T004: Add O(1) diagram mutation helper(s) for ops apply (owner: worker:019c3733-f824-7232-b394-1d4474d41334) (scope: src/model/) (depends: T001)
  - Started_at: 2026-02-07T08:24:28+00:00
  - DoD: `Diagram` exposes a safe way to replace its AST without reconstructing the whole struct; ops can bump `rev` once per apply without looping.
  - Validation: `cargo test`
  - Escalate if: this requires exposing internal fields; keep encapsulation and add narrow APIs instead.
  - Completed_at: 2026-02-07T08:35:43+00:00
  - Completion note: Added `Diagram::replace_ast`/`set_ast` (kind-checked) plus `DiagramAstKindMismatch` so ops can replace a diagram AST in O(1) while preserving `diagram_id`/`name`/`kind` and keeping `rev` untouched; added unit tests.
  - Validation result: `cargo test` (ok)

- [x] T003: Define `Walkthrough` model shells (owner: worker:019c35da-a7e0-7b02-9d29-101474df7191) (scope: src/model/) (depends: T001)
  - Started_at: 2026-02-07T02:08:07+00:00
  - DoD: Walkthrough types compile (`Walkthrough`, nodes, edges) and reference `ObjectRef`s as evidence.
  - Validation: `cargo test`
  - Escalate if: walkthrough model implies rendering/layout choices; keep it data-only for now.
  - Completed_at: 2026-02-07T02:19:35+00:00
  - Completion note: Implemented core walkthrough model shells (`Walkthrough`, `WalkthroughNode`, `WalkthroughEdge`) with evidence-first `refs: Vec<ObjectRef>`, added `WalkthroughNodeId`, exported types, and updated `Session` to store walkthroughs as values.
  - Validation result: `cargo test` (ok)

- [x] T002: Define `XRef` + dangling status model (owner: worker:019c35c0-481b-7f90-a609-65bfd7ed6736) (scope: src/model/) (depends: T001)
  - Started_at: 2026-02-07T01:37:10+00:00
  - DoD: XRef type matches protocol (`from/to/kind/label?/status`); dangling endpoints representable.
  - Validation: `cargo test`
  - Escalate if: status semantics drift from `docs/protocol-01.md`; update spec or code to match.
  - Completed_at: 2026-02-07T01:59:43+00:00
  - Completion note: Added `XRef` model + `XRefStatus` and integrated xref storage into `Session`; unit tests cover status parsing/formatting.
  - Validation result: `cargo test` (ok)

- [x] T001: Define `Session`, `Diagram`, `DiagramKind`, and AST shells (owner: worker:019c35b5-930d-7212-8ddd-0d6bb993cf27) (scope: src/model/) (depends: spec:01-diagram-tui-rust/T002)
  - Started_at: 2026-02-07T01:25:47+00:00
  - DoD: Core types compile; include `rev` fields where required by protocol; no serde yet.
  - Validation: `cargo test`
  - Escalate if: boundaries between model/ops/query/store become unclear; stop and propose a refactor plan.
  - Completed_at: 2026-02-07T01:35:45+00:00
  - Completion note: Added core `Session`/`Diagram` model types (with `Diagram.rev: u64`) and minimal Sequence/Flowchart AST shells in `src/model/`, aligned with `docs/protocol-01.md`.
  - Validation result: `cargo test` (ok)
