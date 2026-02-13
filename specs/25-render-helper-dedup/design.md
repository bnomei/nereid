# Design — 25-render-helper-dedup

## Context

Three renderers implement identical private helpers:
- `truncate_with_ellipsis`
- `text_len`
- `canvas_to_string_trimmed`

This is low-risk duplication, but it is easy for behavior to drift over time and creates “fix-in-three-places” failure modes.

## Proposed design

1. Introduce a single shared helper module in `src/render/` (e.g. `src/render/text.rs`).
2. Move the three helpers into that module:
   - `pub(crate) fn truncate_with_ellipsis(text: &str, max_len: usize) -> String`
   - `pub(crate) fn text_len(text: &str) -> usize`
   - `pub(crate) fn canvas_to_string_trimmed(canvas: &Canvas) -> String`
3. Update the renderers to import and call the shared helpers and delete the duplicated definitions.

## Compatibility / invariants

- Keep the exact semantics:
  - `truncate_with_ellipsis` uses Unicode ellipsis (`…`) and counts `chars()`.
  - `canvas_to_string_trimmed` trims trailing spaces on each line and drops trailing empty lines at the bottom.
- Do not change any renderer public function signatures (`render_*_unicode`).

## Validation

- Existing renderer snapshot tests in:
  - `src/render/sequence.rs`
  - `src/render/flowchart.rs`
  - `src/render/walkthrough.rs`
  should remain byte-for-byte identical.
- Add focused helper unit tests for truncation edge cases and trimming behavior.

