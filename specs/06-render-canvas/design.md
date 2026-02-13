# Design — 06-render-canvas

Keep generic rendering primitives in `src/render/`.

Design goals:
- A `Canvas` type with bounds-checked drawing.
- Clear behavior for overlaps (e.g. last-writer-wins or merge rules).
- Unit tests for primitives (box, h/v lines, corners).

