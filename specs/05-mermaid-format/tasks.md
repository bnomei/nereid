# Tasks — 05-mermaid-format

Meta:
- Spec: 05-mermaid-format — Mermaid subset parse/export
- Depends on: spec:03-model-core/T001
- Global scope:
  - src/format/

## In Progress

## Blocked

## Todo

## Done

- [x] T004: Harden sequence export for mutated ASTs (owner: worker:019c38a5-401d-7b63-a255-5ab38bd46276) (scope: src/format/) (depends: T001)
  - Started_at: 2026-02-07T15:07:48+00:00
  - Context: Export must not emit invalid `.mmd` when sequence message text contains newlines/CRs (e.g. mutated via ops or other tools). Flowchart export already rejects newline/CR in labels; sequence export should match that safety posture.
  - DoD: `export_sequence_diagram` rejects message text containing `\\n` or `\\r` with a typed `MermaidSequenceExportError` variant; unit test covers the error case.
  - Validation: `cargo test --offline`
  - Escalate if: enforcing this at export breaks existing invariants/tests; propose a minimal escaping scheme instead of silently truncating.
  - Completed_at: 2026-02-07T15:22:49+00:00
  - Completion note: Sequence export now rejects message text containing newline/CR with a dedicated `MermaidSequenceExportError` variant before emitting output; added a unit test covering both `\\n` and `\\r` cases.
  - Validation result: `cargo test --offline` (ok)

- [x] T003: Extend flowchart subset for edge labels + more node shapes (owner: worker:019c3757-b8a7-7c61-acf9-01eaebd06dd0) (scope: src/format/) (depends: spec:03-model-core/T005)
  - Started_at: 2026-02-07T09:03:49+00:00
  - DoD: parser/exporter round-trips edge labels (`A -->|label| B`) and at least one non-rect node shape; unit tests cover semantic round-trip.
  - Validation: `cargo test`
  - Escalate if: this needs broad style/class/subgraph support; keep it minimal and aligned to available model fields.
  - Completed_at: 2026-02-07T09:23:04+00:00
  - Completion note: Implemented Mermaid `flowchart` subset support for edge labels (`A -->|label| B`) and non-rect node shapes (`()` round, `{}` diamond) with canonical export + semantic round-trip tests; unsupported styling/subgraphs remain rejected.
  - Validation result: `cargo test` (ok)

- [x] T002: Parse/export modern `flowchart` subset (owner: worker:019c35da-ac50-7c21-8f80-00fd7bfd0cae) (scope: src/format/) (depends: spec:03-model-core/T001)
  - Started_at: 2026-02-07T02:08:07+00:00
  - DoD: parse nodes/edges into AST; export canonical `.mmd`; tests for key constructs.
  - Validation: `cargo test`
  - Escalate if: routing/layout concerns leak into parsing; keep parsing semantic only.
  - Completed_at: 2026-02-07T02:19:35+00:00
  - Completion note: Implemented std-only Mermaid `flowchart` subset parse/export (nodes with optional `[]` labels + `-->` edges) with canonical export and semantic round-trip tests; unsupported constructs are rejected with actionable errors.
  - Validation result: `cargo test` (ok)

- [x] T001: Parse/export `sequenceDiagram` subset (owner: worker:019c35c0-55d7-7052-aba0-ee734172e57c) (scope: src/format/) (depends: spec:03-model-core/T001)
  - Started_at: 2026-02-07T01:37:10+00:00
  - DoD: parse participants + messages into AST; export canonical `.mmd`; tests for round-trip semantics (semantic, not formatting).
  - Validation: `cargo test`
  - Escalate if: syntax scope creeps; document and freeze subset.
  - Completed_at: 2026-02-07T01:59:43+00:00
  - Completion note: Implemented std-only Mermaid `sequenceDiagram` subset parse/export (participants + messages) with canonical export and semantic round-trip tests.
  - Validation result: `cargo test` (ok)
