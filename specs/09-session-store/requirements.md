# Requirements — 09-session-store

This spec implements session folder persistence (`nereid-session.meta.json`, `diagrams/*`, `walkthroughs/*`) with relative-path metadata.

Normative protocol reference: `docs/protocol-01.md`

## Requirements (EARS)

- THE SYSTEM SHALL load/save a session as a folder with relative-path metadata.
- WHEN a session is saved THE SYSTEM SHALL export `.mmd` and `.ascii.txt` for each diagram (export-on-save).
- THE SYSTEM SHALL preserve stable IDs via `.meta.json` sidecars.
