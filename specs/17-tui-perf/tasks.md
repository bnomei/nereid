# Tasks — 17-tui-perf

Meta:
- Spec: 17-tui-perf — Reduce TUI per-frame allocations
- Source list: `docs/audit.md`
- Depends on: spec:11-tui/T001
- Global scope:
  - src/tui/mod.rs

## In Progress

## Blocked

## Todo

## Done

- [x] T001: Cache visible xref indices (owner: mayor) (scope: src/tui/mod.rs) (depends: -)
  - Completed_at: 2026-02-08T01:05:59+00:00
  - Completion_note: Cached visible-xref indices in App and recompute on dangling-only toggle; visible_xref_indices() now returns &[usize] (cargo test --offline: ok; cargo clippy --offline: ok).
  - Started_at: 2026-02-08T01:01:01+00:00
  - Context: `visible_xref_indices()` allocates a new `Vec` and is called repeatedly (draw + navigation).
  - DoD:
    - Add `App.visible_xref_indices: Vec<usize>` + `recompute_visible_xref_indices`.
    - Change `visible_xref_indices()` to return `&[usize]` and update callers.
    - Preserve selection semantics across dangling-only toggle and navigation.
  - Validation: `cargo test --offline`
  - Escalate if: this requires broad App refactors; keep changes localized.

- [x] T002: Avoid cloning labels into list items and status (owner: mayor) (scope: src/tui/mod.rs) (depends: T001)
  - Started_at: 2026-02-08T01:05:59+00:00
  - Completed_at: 2026-02-08T01:05:59+00:00
  - Completion_note: Objects/XRefs list items now borrow label &str (no per-frame String clones); status bar uses &str (cargo test --offline: ok; cargo clippy --offline: ok).
  - Context: draw clones `SelectableObject.label` / `SelectableXRef.label` into `ListItem`.
  - DoD:
    - Use `ListItem::new(label.as_str())` for Objects and XRefs lists.
    - Avoid cloning selected xref label for the status bar (use `&str`).
  - Validation: `cargo test --offline`; `cargo clippy --offline`

