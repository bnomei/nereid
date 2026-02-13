# Tasks — 25-render-helper-dedup

Meta:
- Spec: 25-render-helper-dedup — Deduplicate render text helpers
- Depends on: spec:06-render-canvas/T001, spec:13-diagram-renderers/T001
- Global scope:
  - src/render/

## In Progress


## Blocked

## Todo

## Done

- [x] T001: Add shared render helper module (owner: perf-agent) (scope: src/render/) (depends: -)
  - Started_at: 2026-02-08T16:34:29+00:00
  - Completed_at: 2026-02-08T16:38:13+00:00
  - Completion note: Added `src/render/text.rs` as the single home for truncation/len/canvas trim helpers and wired renderers to use it.
  - Validation result: `cargo test --offline` (ok)

- [x] T002: Refactor sequence renderer to use shared helpers (owner: perf-agent) (scope: src/render/sequence.rs) (depends: T001)
  - Completed_at: 2026-02-08T16:38:13+00:00
  - Completion note: Removed duplicated helper implementations from `src/render/sequence.rs` in favor of `render::text`.
  - Validation result: `cargo test --offline` (ok)

- [x] T003: Refactor flowchart renderer to use shared helpers (owner: perf-agent) (scope: src/render/flowchart.rs) (depends: T001)
  - Completed_at: 2026-02-08T16:38:13+00:00
  - Completion note: Removed duplicated helper implementations from `src/render/flowchart.rs` in favor of `render::text`.
  - Validation result: `cargo test --offline` (ok)

- [x] T004: Refactor walkthrough renderer to use shared helpers (owner: perf-agent) (scope: src/render/walkthrough.rs) (depends: T001)
  - Completed_at: 2026-02-08T16:38:13+00:00
  - Completion note: Removed duplicated helper implementations from `src/render/walkthrough.rs` in favor of `render::text`.
  - Validation result: `cargo test --offline` (ok)

- [x] T005: Add unit tests for helper semantics (owner: perf-agent) (scope: src/render/) (depends: T001)
  - Completed_at: 2026-02-08T16:38:13+00:00
  - Completion note: Added focused unit tests in `src/render/text.rs` for truncation and canvas trimming edge cases.
  - Validation result: `cargo test --offline` (ok)
