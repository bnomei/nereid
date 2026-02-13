# Requirements — 13-diagram-renderers

This spec renders diagrams to deterministic ASCII/Unicode text using `Canvas`.

Normative protocol reference: `docs/protocol-01.md`

## Requirements (EARS)

- THE SYSTEM SHALL render a diagram from its AST (via layout output) into a text buffer deterministically.
- THE SYSTEM SHALL keep rendering independent from TUI concerns (no ratatui types).
- THE SYSTEM SHALL support Unicode output as the primary mode (ASCII-only can be added later).

