# Tasks — 10-walkthroughs

Meta:
- Spec: 10-walkthroughs — Walkthrough artifacts
- Depends on: spec:03-model-core/T003
- Global scope:
  - src/model/
  - src/store/
  - src/render/
  - src/tui/
  - src/mcp/

## In Progress

## Blocked

## Todo

## Done

- [x] T003: Use walkthrough renderer for `.ascii.txt` export-on-save (owner: worker:019c395e-01a6-7a93-b828-87de8ab52744) (scope: src/store/) (depends: T002,spec:09-session-store/T006)
  - Started_at: 2026-02-07T18:29:33+00:00
  - Context (worker-facing; do not read `docs/protocol-01.md`):
    - `SessionFolder::save_walkthrough` still writes a placeholder `.ascii.txt`. Wire it to the real walkthrough renderer and update store tests accordingly.
  - DoD:
    - Walkthrough `.ascii.txt` export uses `render_walkthrough_unicode(...)` (no placeholder).
    - Store tests updated for deterministic rendered output.
    - `cargo test --offline` remains green.
  - Validation: `cargo test --offline`
  - Escalate if: this requires changes outside `src/store/`; keep it isolated.
  - Completed_at: 2026-02-07T18:36:27+00:00
  - Completion note: Updated `SessionFolder::save_walkthrough` to export `.ascii.txt` via `render_walkthrough_unicode` (with a trailing newline), added `StoreError::WalkthroughRender` for renderer failures, updated the store unit test to assert the real render output, and removed the placeholder helper; `cargo test --offline` is green.
  - Validation result: `cargo test --offline` (ok, 132 tests)

- [x] T002: Walkthrough ASCII renderer baseline (owner: worker:019c3950-0f02-79f3-b12e-5129709ddca9) (scope: src/render/) (depends: T001,spec:06-render-canvas/T001)
  - Started_at: 2026-02-07T18:14:40+00:00
  - Context (worker-facing; do not read `docs/protocol-01.md`):
    - Walkthrough `.ascii.txt` export is currently a placeholder; implement a minimal deterministic renderer using existing `Canvas` primitives.
    - Keep it minimal: boxes + simple arrows; no advanced layout required.
  - DoD:
    - Add `render_walkthrough_unicode(&Walkthrough) -> Result<String, WalkthroughRenderError>` in `src/render/` and export it from `src/render/mod.rs`.
    - Unit tests snapshot a simple walkthrough (2 nodes + 1 edge) and assert deterministic output (boxes + arrow).
    - Keep this task isolated to `src/render/` (store wiring is a follow-up task).
  - Validation: `cargo test --offline`
  - Escalate if: this requires changes outside `src/render/`; keep it isolated.
  - Completed_at: 2026-02-07T18:29:33+00:00
  - Completion note: Added baseline walkthrough Unicode renderer `render_walkthrough_unicode` using existing `Canvas` primitives (boxes + simple arrows), exported it from `src/render/mod.rs`, and added snapshot unit tests; `cargo test --offline` is green.
  - Validation result: `cargo test --offline` (ok, 132 tests)

- [x] T001: Walkthrough persistence (`*.wt.json` + ascii snapshot) (owner: worker:019c3791-bef8-7372-afa4-9464c2c211d1) (scope: src/store/) (depends: spec:03-model-core/T003,spec:09-session-store/T001)
  - Started_at: 2026-02-07T10:06:57+00:00
  - DoD: walkthroughs load/save; export `.ascii.txt` snapshot.
  - Validation: `cargo test`
  - Escalate if: cross-scope conflicts arise; split into smaller specs.
  - Completed_at: 2026-02-07T10:16:16+00:00
  - Completion note: Implemented walkthrough artifact persistence under `walkthroughs/` (`<id>.wt.json` + `<id>.ascii.txt`), added `SessionFolder` load/save APIs, and unit tests for round-trip + move-safe loading; `cargo test` green.
  - Validation result: `cargo test` (ok)
