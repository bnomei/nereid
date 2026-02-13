# Requirements — 11-tui

This spec implements the `ratatui` UI (diagram view, inspector, XRefs panel, walkthrough view).

Mayor-only protocol reference: `docs/protocol-01.md` (workers must not be sent to this doc; extract needed excerpts into task `Context:` blocks)

## Requirements (EARS)

- THE SYSTEM SHALL provide an interactive terminal UI to view rendered diagrams.
- THE SYSTEM SHALL expose selection context for agent collaboration (selection → `ObjectRef`).
- THE SYSTEM SHALL present XRefs as first-class UI elements, including a dangling TODO filter.
