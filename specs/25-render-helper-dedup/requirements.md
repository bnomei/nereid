# Requirements — 25-render-helper-dedup

This spec removes copy/paste render helper code that is duplicated across the Unicode renderers:

- `truncate_with_ellipsis(text, max_len)`
- `text_len(text)`
- `canvas_to_string_trimmed(canvas)`

Currently duplicated in:
- `src/render/sequence.rs`
- `src/render/flowchart.rs`
- `src/render/walkthrough.rs`

## Requirements (EARS)

- THE SYSTEM SHALL define the render text helpers in exactly one place under `src/render/` and reuse them from all three renderers.
- THE SYSTEM SHALL keep rendered output byte-for-byte identical for all existing unit tests/snapshots.
- THE SYSTEM SHALL keep helper behavior deterministic and platform-independent.
- THE SYSTEM SHALL keep the helpers crate-internal (`pub(crate)`), avoiding changes to the public render API.

## Non-goals

- This spec does not change the definition of “text length” (currently `chars().count()`), and does not introduce display-width/grapheme-width handling.

