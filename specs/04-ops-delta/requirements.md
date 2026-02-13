# Requirements — 04-ops-delta

This spec defines structured mutation operations (`ops[]`) plus revision/delta plumbing used by the UI and MCP tool surface.

Normative protocol reference: `docs/protocol-01.md`

## Requirements (EARS)

- THE SYSTEM SHALL apply structured ops to mutate a diagram/walkthrough without editing ASCII or Mermaid text directly.
- THE SYSTEM SHALL reject mutations with stale `base_rev` and return conflict info enabling recovery via digest/delta.
- THE SYSTEM SHALL produce a delta payload describing what changed since a revision.

