# Design — 05-mermaid-format

Keep Mermaid parsing/export in `src/format/mermaid/`.

Design goals:
- Deterministic parser for a deliberately limited subset.
- Canonical exporter (stable formatting) for `.mmd`.
- Unit tests using small embedded fixtures (std-only for now).

