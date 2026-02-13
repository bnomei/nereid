# Tasks — 04-ops-delta

Meta:
- Spec: 04-ops-delta — Structured ops + revision/delta
- Depends on: spec:03-model-core/T001
- Global scope:
  - src/ops/

## In Progress

## Blocked

## Todo

## Done

- [x] T006: Preserve seq participant role on update (owner: worker:019c38c5-40d4-7663-a288-dc29429e6390) (scope: src/ops/) (depends: spec:03-model-core/T006)
  - Started_at: 2026-02-07T15:42:27+00:00
  - Context: `SeqOp::UpdateParticipant` currently reconstructs `SequenceParticipant` and drops unrelated metadata (e.g. `role`). After `03-model-core/T006` adds `SequenceParticipant` setters, update ops apply to patch participants in place and preserve existing role.
  - DoD: Updating a participant’s `mermaid_name` via ops preserves its existing `role`; unit test covers the preservation behavior.
  - Validation: `cargo test --offline`
  - Escalate if: model APIs are insufficient after `03-model-core/T006`; propose the minimal additional model API instead of using field access.
  - Completed_at: 2026-02-07T15:49:01+00:00
  - Completion note: `SeqOp::UpdateParticipant` now patches `SequenceParticipant` in place using the new setter APIs so participant `role` is preserved across name updates; added a unit test covering the preservation behavior.
  - Validation result: `cargo test --offline` (ok)

- [x] T005: Validate seq message endpoints exist (owner: worker:019c38a5-3a6b-7113-b87a-cfa9c2307f45) (scope: src/ops/) (depends: T001)
  - Started_at: 2026-02-07T15:07:48+00:00
  - Context: Prevent invalid sequence diagrams from being created via ops. `SeqOp::AddMessage` and `SeqOp::UpdateMessage` must reject `from_participant_id`/`to_participant_id` that do not exist in `SequenceAst.participants`.
  - DoD: Applying seq message add/update ops fails fast when either endpoint participant is missing, returning an `ApplyError::NotFound { kind: SeqParticipant, object_id: <missing> }` (no new error type required); unit tests cover both AddMessage and UpdateMessage invalid endpoints.
  - Validation: `cargo test --offline`
  - Escalate if: this requires changing error types outside `src/ops/`; keep it within existing `ApplyError` variants.
  - Completed_at: 2026-02-07T15:22:49+00:00
  - Completion note: `apply_seq_op` now validates `from_participant_id`/`to_participant_id` for `SeqOp::AddMessage` and `SeqOp::UpdateMessage` and returns `ApplyError::NotFound { kind: SeqParticipant, object_id }` when either endpoint participant is missing; added unit tests covering missing-from and missing-to for both ops.
  - Validation result: `cargo test --offline` (ok)

- [x] T004: Extend flow ops for node shape + edge labels/styles (owner: worker:019c3757-b3df-7e63-95e3-cf00629b0989) (scope: src/ops/) (depends: T001,spec:03-model-core/T005)
  - Started_at: 2026-02-07T09:03:49+00:00
  - DoD: Flow ops support adding/updating node `shape` and edge `label`/`style` without breaking existing behavior; unit tests cover at least one of each field.
  - Validation: `cargo test`
  - Escalate if: model changes are required beyond `03-model-core/T005`; stop and propose the minimal additional model API.
  - Completed_at: 2026-02-07T09:23:04+00:00
  - Completion note: Extended flow ops to support node `shape` and edge `label`/`style` on add/update; updated apply logic to patch-in-place (and rebuild edges with preserved label/style) so unrelated fields aren’t reset; added unit tests covering node shape updates and edge label/style updates + preservation on endpoint changes.
  - Validation result: `cargo test` (ok)

- [x] T003: Make `apply_ops` O(1) in `rev` (owner: worker:019c3743-bad8-7a20-9d3c-8e2509ae6c76) (scope: src/ops/) (depends: T001,spec:03-model-core/T004)
  - Started_at: 2026-02-07T08:42:02+00:00
  - DoD: `apply_ops` updates a diagram without rebuilding and without looping `bump_rev()` up to `new_rev`; unit tests still pass.
  - Validation: `cargo test`
  - Escalate if: additional model APIs are needed; propose the minimal change in `03-model-core` instead of hacking around privacy.
  - Completed_at: 2026-02-07T09:01:47+00:00
  - Completion note: Updated `apply_ops` to replace the diagram AST via `Diagram::set_ast` and bump `rev` exactly once, eliminating the Diagram rebuild and O(rev) `bump_rev()` loop while preserving base_rev conflict gating, kind mismatch checks, and delta recording.
  - Validation result: `cargo test` (ok)

- [x] T002: Implement minimal delta schema (owner: worker:019c3734-006c-7a52-bf27-ca583c28a2c0) (scope: src/ops/) (depends: T001)
  - Started_at: 2026-02-07T08:24:28+00:00
  - DoD: delta payload returned from apply includes added/removed/updated refs (coarse ok).
  - Validation: `cargo test`
  - Escalate if: delta granularity blocks progress; ship coarse delta first.
  - Completed_at: 2026-02-07T08:35:43+00:00
  - Completion note: Implemented minimal delta schema returned from `apply_ops` (`added/removed/updated` `ObjectRef`s) including cascading removals for flow-node incident edges and seq-participant messages; added unit tests.
  - Validation result: `cargo test` (ok)

- [x] T001: Define op enums + apply skeleton (owner: worker:019c35c0-51b7-73e0-aa5f-702059710203) (scope: src/ops/) (depends: spec:03-model-core/T001)
  - Started_at: 2026-02-07T01:37:10+00:00
  - DoD: `ops[]` types exist for core mutations; apply function updates `rev`.
  - Validation: `cargo test`
  - Escalate if: op set grows too large; freeze to minimal add/update/remove set.
  - Completed_at: 2026-02-07T01:59:43+00:00
  - Completion note: Implemented minimal typed ops for sequence/flow and an `apply_ops` skeleton with `base_rev` conflict gating; added unit tests for rev bump and stale-rev conflicts.
  - Validation result: `cargo test` (ok)
