# Tasks — 06-render-canvas

Meta:
- Spec: 06-render-canvas — Canvas + Unicode primitives
- Depends on: -
- Global scope:
  - src/render/

## In Progress

## Blocked

## Todo

## Done

- [x] T003: Add line intersection/junction merge rules (owner: worker:019c3743-bf33-7351-a06c-e7f38e58c097) (scope: src/render/) (depends: T002)
  - Started_at: 2026-02-07T08:42:02+00:00
  - DoD: drawing overlapping h/v lines yields consistent junction characters (e.g. `┼`, `├`, `┤`, `┬`, `┴`) instead of last-writer overwrites; unit tests cover core cases.
  - Validation: `cargo test`
  - Escalate if: this starts forcing layout/router decisions; keep it purely local to `Canvas` char-merging.
  - Completed_at: 2026-02-07T09:01:47+00:00
  - Completion note: Updated `Canvas` to merge Unicode box-drawing edges on overlap and render stable junction glyphs (`┼`, `├`, `┤`, `┬`, `┴`) instead of last-writer overwrites; added unit tests for core h/v intersection and tee cases.
  - Validation result: `cargo test` (ok)

- [x] T002: Add Unicode box/line primitives (owner: worker:019c35c0-4cef-7c42-b4c5-ff7626b6814a) (scope: src/render/) (depends: T001)
  - Started_at: 2026-02-07T01:37:10+00:00
  - DoD: helper fns draw boxes and orthogonal lines; tests validate output.
  - Validation: `cargo test`
  - Escalate if: terminal rendering differences appear; keep output purely text-based.
  - Completed_at: 2026-02-07T01:59:43+00:00
  - Completion note: Added Unicode line and box drawing helpers (`draw_hline`, `draw_vline`, `draw_box`) with tests for expected output and bounds behavior.
  - Validation result: `cargo test` (ok)

- [x] T001: Implement `Canvas` grid + basic drawing ops (owner: worker:019c35b5-9858-70c2-86eb-6bc2b38bdc11) (scope: src/render/) (depends: -)
  - Started_at: 2026-02-07T01:25:47+00:00
  - DoD: `Canvas` supports set/get, bounds checks, and exporting to `String`; tests cover basics.
  - Validation: `cargo test`
  - Escalate if: overlap rules become complex; pick a simple deterministic rule and document it.
  - Completed_at: 2026-02-07T01:35:45+00:00
  - Completion note: Implemented a bounds-checked `Canvas` fixed-size char grid with deterministic overwrite behavior and `String` export; added unit tests for core behavior + errors.
  - Validation result: `cargo test` (ok)
