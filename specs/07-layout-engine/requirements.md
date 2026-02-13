# Requirements — 07-layout-engine

This spec computes deterministic layout for:
- sequence diagrams (participants/columns; messages/rows)
- flowcharts (layered layout + orthogonal edge routing)

Normative protocol reference: `docs/protocol-01.md`

## Requirements (EARS)

- THE SYSTEM SHALL compute deterministic layouts from the AST (no randomness).
- THE SYSTEM SHALL keep layout independent from TUI concerns.
- THE SYSTEM SHALL prefer readability (reduced crossings) over optimality.

