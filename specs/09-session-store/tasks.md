# Tasks — 09-session-store

Meta:
- Spec: 09-session-store — Session folder persistence
- Depends on: spec:02-dependencies/T001
- Global scope:
  - src/store/

## In Progress

## Blocked

## Todo

## Done

- [x] T006: Store hardening + completeness (xrefs + rev + atomic writes + symlink defense) (owner: worker:019c3871-8698-7540-b90e-812e224be86b) (scope: src/store/,src/model/) (depends: -)
  - Started_at: 2026-02-07T14:10:36+00:00
  - DoD: Loading a saved session reproduces xrefs/active walkthrough/diagram revs; save paths are atomic; unsafe symlinked paths are rejected with a typed store error; unit tests cover round-trip + symlink defense behavior.
  - Validation: `cargo test --offline`
  - Completed_at: 2026-02-07T14:52:41+00:00
  - Completion note: Persisted and loaded `Session.xrefs`, `Session.active_walkthrough_id`, and `Diagram.rev` via `nereid-session.meta.json` with backwards-compatible serde defaults; load now refreshes xref status based on loaded endpoints. Hardened store writes with atomic temp-write+rename and best-effort symlink refusal under the session root, plus unit tests for xref/rev/active walkthrough round-trips and the symlink defense.
  - Validation result: `cargo test --offline` (ok)

- [x] T005: Implement `SessionFolder::load_session` (diagrams + walkthroughs) (owner: worker:019c37ba-a10f-7280-b505-59ffc85fa47f) (scope: src/store/) (depends: T004,spec:05-mermaid-format/T001,spec:05-mermaid-format/T002,spec:10-walkthroughs/T001)
  - Started_at: 2026-02-07T10:51:38+00:00
  - DoD: load `nereid-session.meta.json`, parse each diagram `.mmd` into AST, load walkthroughs from `walkthroughs/*.wt.json`; unit tests for a save→load round-trip.
  - Validation: `cargo test`
  - Escalate if: stable-id reconciliation becomes required; keep “new IDs only” and defer reconciliation to a separate task.
  - Completed_at: 2026-02-07T11:00:53+00:00
  - Completion note: Implemented `SessionFolder::load_session()` to read `nereid-session.meta.json`, parse each referenced diagram `.mmd` into the correct AST by `kind`, and load walkthroughs from `walkthroughs/*.wt.json`; added Mermaid-parse store errors and a save→load round-trip unit test (diagrams + walkthroughs + `active_diagram_id`).
  - Validation result: `cargo test` (ok)

- [x] T004: Use real renderer for diagram `.ascii.txt` export-on-save (owner: worker:019c37a1-bb80-7df0-9298-d17a393e7881) (scope: src/store/) (depends: T002,spec:13-diagram-renderers/T001,spec:13-diagram-renderers/T002)
  - Started_at: 2026-02-07T10:24:25+00:00
  - DoD: `save_session` writes `.ascii.txt` using Unicode renderers for sequence + flowcharts (no placeholder for diagrams).
  - Validation: `cargo test`
  - Escalate if: renderer needs new configuration surface; keep a minimal default config in store and defer knobs to a follow-up task.
  - Completed_at: 2026-02-07T10:33:05+00:00
  - Completion note: Replaced `SessionFolder::save_session` diagram `.ascii.txt` export placeholder with real Unicode renders using `layout_sequence`+`render_sequence_unicode` and `layout_flowchart`+`render_flowchart_unicode`; added store-level error variants for layout/render failures and updated unit tests to assert rendered output.
  - Validation result: `cargo test` (ok)

- [x] T002: Diagram export-on-save (`.mmd` + `.ascii.txt`) (owner: worker:019c377d-2c48-7242-87fb-86739cc88143) (scope: src/store/) (depends: T001,spec:05-mermaid-format/T001,spec:06-render-canvas/T001)
  - Started_at: 2026-02-07T09:44:30+00:00
  - DoD: saving a session writes canonical `.mmd` and `.ascii.txt` for each diagram.
  - Validation: `cargo test`
  - Escalate if: renderer/layout not available; stub ASCII export with placeholder and document.
  - Completed_at: 2026-02-07T09:58:09+00:00
  - Completion note: Implemented `SessionFolder::save_session` export-on-save: writes canonical Mermaid `.mmd` via existing exporters and writes `.ascii.txt` as a deterministic placeholder (renderer not implemented yet). Added unit tests asserting session-relative paths in `nereid-session.meta.json` and verifying exported files exist under `diagrams/`.
  - Validation result: `cargo test` (ok)

- [x] T003: `.meta.json` sidecar persistence stub (owner: worker:019c376c-aee4-77a3-b66a-a29bff5b871f) (scope: src/store/) (depends: T001,spec:03-model-core/T001)
  - Started_at: 2026-02-07T09:26:42+00:00
  - DoD: write/read stable id mappings and xrefs; reconciliation can be “new IDs only” initially.
  - Validation: `cargo test`
  - Escalate if: reconciliation is too complex; keep it as a stub and add follow-up tasks.
  - Completed_at: 2026-02-07T09:38:51+00:00
  - Completion note: Implemented per-diagram `diagrams/*.meta.json` sidecar persistence including session-relative `mmd_path`, stable-id mapping stub, and xrefs schema; added unit tests for round-trip and relative-path enforcement.
  - Validation result: `cargo test` (ok)

- [x] T001: Implement session folder load/save skeleton (owner: worker:019c35db-1122-77d0-8d6a-8a6e496e83ab) (scope: src/store/) (depends: spec:02-dependencies/T001,spec:03-model-core/T001)
  - Started_at: 2026-02-07T02:08:07+00:00
  - DoD: load/save `nereid-session.meta.json` + diagram lists; relative-path handling; tests for path behavior.
  - Validation: `cargo test`
  - Escalate if: serialization choice is not ready; block on `02-dependencies/T001`.
  - Completed_at: 2026-02-07T02:19:35+00:00
  - Completion note: Implemented session folder meta persistence skeleton with load/save of minimal `nereid-session.meta.json`, storing paths relative to the session folder on disk and resolving them on load; added unit tests for relative-path behavior + JSON roundtrip.
  - Validation result: `cargo test` (ok)
