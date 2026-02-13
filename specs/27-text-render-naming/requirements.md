# Requirements — 27-text-render-naming

The system renders **deterministic text diagrams** that include Unicode box-drawing and symbols (e.g. `┌─┐`, `▶`, `…`), but several APIs and exports label this output as “ascii” (e.g. MCP `diagram.render_ascii`, files `*.ascii.txt`).

This spec aligns terminology so “ascii” is not used to describe Unicode output, while preserving backwards compatibility for existing clients and session exports.

## Requirements (EARS)

- THE SYSTEM SHALL provide a clearly-named “text” render surface that accurately describes the output as deterministic text (Unicode allowed).
- THE SYSTEM SHALL preserve backwards compatibility for existing MCP tools `diagram.render_ascii` and `walkthrough.render_ascii` and for existing exported `.ascii.txt` files, or provide a documented migration strategy.
- THE SYSTEM SHALL avoid claiming ASCII-only output unless the renderer is actually ASCII-only.
- THE SYSTEM SHOULD update tool descriptions and docs to explicitly state that the output may contain Unicode box-drawing and symbols.

## Non-goals

- This spec does not implement an ASCII-only renderer.

