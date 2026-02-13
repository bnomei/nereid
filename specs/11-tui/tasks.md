# Tasks — 11-tui

Meta:
- Spec: 11-tui — ratatui UI
- Depends on: spec:02-dependencies/T002
- Global scope:
  - src/tui/

## In Progress

## Blocked

## Todo

## Done

- [x] T005: Diagram switching + cross-diagram XRef jump (owner: mayor) (scope: src/tui/) (depends: T004)
  - Started_at: 2026-02-07T15:07:48+00:00
  - Context: Current UI renders the active diagram only and the Objects list is per-diagram, so `f`/`t` jumps from XRefs can’t reach objects in other diagrams. Add deterministic diagram switching and make XRef jumps switch diagrams when needed.
  - DoD: Add keybindings to switch the active diagram (e.g. `[`/`]` previous/next) and refresh the rendered diagram + Objects list; XRef jump (`f`/`t`) switches to the target diagram when the endpoint is in a different diagram and selects the object when present; unit tests cover diagram switching and cross-diagram XRef jumps.
  - Validation: `cargo test --offline`; manual: `cargo run` launches and quits cleanly.
  - Escalate if: UX semantics conflict with existing focus/selection model; keep changes minimal and deterministic.
  - Completed_at: 2026-02-07T15:22:49+00:00
  - Completion note: Added global `[`/`]` keybindings to switch the active diagram (wrap-around) and refresh the rendered view + per-diagram Objects list. Updated XRef jumps (`f`/`t`) to switch to the endpoint’s diagram when needed and then select the object when present; added unit tests for diagram switching and cross-diagram XRef jump selection.
  - Validation result: `cargo test --offline` (ok); manual `cargo run` (ok)

- [x] T004: Session-backed viewer (load + render active diagram) (owner: worker:019c37e9-6049-75c2-b2d9-a422ce95f488) (scope: src/tui/) (depends: T001,spec:09-session-store/T005,spec:13-diagram-renderers/T004)
  - Started_at: 2026-02-07T11:42:54+00:00
  - DoD: viewer renders the active diagram from a loaded session instead of a demo buffer; object list derives from the session/AST.
  - Validation: `cargo test`; manual: `cargo run` launches the TUI and quits cleanly.
  - Completed_at: 2026-02-07T12:03:17+00:00
  - Completion note: Added `tui::run_with_session(Session)` (no file I/O in TUI) and updated `run()` to call the session-backed path with an internal demo `Session`. The UI now ensures an active diagram ID is set deterministically when missing, renders the active diagram via `render::render_diagram_unicode(&Diagram)`, and populates the Objects list from the active diagram AST using canonical `ObjectRef` categories (`seq/participant`, `seq/message`, `flow/node`, `flow/edge`). XRefs list is now session-derived as well.
  - Validation result: `cargo test` (ok); manual `cargo run` (ok)

- [x] T003: XRefs panel + dangling TODO view (owner: worker:019c37cd-ab80-7a51-bc90-fb37ac8d9884) (scope: src/tui/) (depends: T002,spec:03-model-core/T002)
  - Started_at: 2026-02-07T11:12:26+00:00
  - DoD: xref list panel + jump; filter to dangling-only (TODO); show missing endpoints.
  - Validation: manual checklist in DoD is met.
  - Escalate if: navigation semantics unclear; define “jump to ref” rules first.
  - Completed_at: 2026-02-07T11:25:01+00:00
  - Completion note: Added an XRefs list panel (demo data) with a `d` “dangling only” filter. Selecting an xref shows details (including missing endpoints) in the inspector; `f`/`t` jumps to the xref’s from/to by selecting the matching `ObjectRef` in the Objects list. Added unit tests for xref focus/selection/jump and the dangling filter.
  - Validation result: `cargo test` (ok)

- [x] T002: Inspector + selection + `ObjectRef` wiring (owner: worker:019c37c6-e555-7e92-899a-c5967d45612e) (scope: src/tui/) (depends: T001,spec:03-model-core/T001)
  - Started_at: 2026-02-07T11:05:02+00:00
  - DoD: selecting an item yields an `ObjectRef`; inspector shows details.
  - Validation: manual: navigate selection; verify inspector updates.
  - Escalate if: hit-testing is too hard; use list/jump selection first.
  - Completed_at: 2026-02-07T11:10:53+00:00
  - Completion note: Added a basic inspector + selection model in the ratatui UI. The UI now has a sidebar objects list whose selection yields a canonical `ObjectRef` and an inspector panel that shows the selected ref’s details. Diagram viewer remains functional with `Tab` focus switching; added unit tests for focus/selection; `cargo test` green.
  - Validation result: `cargo test` (ok)

- [x] T001: ratatui app shell + diagram viewer pane (owner: worker:019c37ba-a608-70c2-8520-efa91711fe2a) (scope: src/tui/) (depends: spec:02-dependencies/T002,spec:06-render-canvas/T001)
  - Started_at: 2026-02-07T10:51:38+00:00
  - DoD: TUI starts; displays a rendered buffer; supports scrolling.
  - Validation: manual: run app; verify no panics.
  - Escalate if: backend choice is unclear; keep it in `02-dependencies`.
  - Completed_at: 2026-02-07T11:01:34+00:00
  - Completion note: Implemented `tui::run()` ratatui+crossterm app shell in `src/tui/` with clean terminal init/teardown; renders a static diagram buffer with scrolling (arrows/`hjkl`); quit keys `q`/Esc; added unit tests for input handling; `cargo test` green.
  - Validation result: `cargo test` (ok)
