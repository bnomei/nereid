# Requirements — 17-tui-perf

Context
- Source: `docs/audit.md` → "[P2] TUI per-frame allocation opportunities"
- Affected: `src/tui/mod.rs`

Goals
- Reduce avoidable per-frame allocations in the TUI, focusing on:
  - repeated allocation of visible-xref indices
  - cloning label `String`s into list widgets

Non-goals
- UI redesign, new TUI features, or broad refactors outside `src/tui/mod.rs`.
- Adding complex perf instrumentation frameworks.

## Requirements (EARS)

- While rendering frames, the TUI shall not clone `SelectableObject.label` solely to build the Objects list widget.
- While rendering frames, the TUI shall not clone `SelectableXRef.label` solely to build the XRefs list widget.
- When the dangling-only filter is unchanged, the TUI shall reuse the previously computed visible-xref index set.
- When toggling dangling-only mode, the TUI shall recompute the visible-xref index set and preserve the previously selected xref when it remains visible; otherwise it shall select the first visible xref (or none if empty).
- The TUI shall preserve existing navigation behaviors and tests for:
  - selection movement
  - diagram switching
  - xref jumping (from/to)
  - dangling-only filter semantics

## Validation

- `cargo test --offline`
- `cargo clippy --offline`
