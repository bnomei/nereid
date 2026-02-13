# Requirements — 23-diagram-meta-roundtrip

This spec makes diagram `.meta.json` sidecars **authoritative** for round-tripping:
- stable internal IDs for objects not directly represented in Mermaid (notably: **sequence messages** and **flow edges**)
- non-Mermaid fields that would otherwise be lost on export/import (notably: **flow edge style**)

Protocol reference (mayor-only): `docs/protocol-01.md` §7.

## Requirements (EARS)

- WHEN a session is saved THE SYSTEM SHALL write a `.meta.json` sidecar for each diagram alongside the exported `.mmd` and `.ascii.txt`.
- WHEN loading a session AND a diagram sidecar exists THE SYSTEM SHALL reconcile the parsed AST with the sidecar so that stable IDs are preserved for:
  - sequence messages
  - flow edges
- WHEN a parsed object cannot be matched to any persisted sidecar entry THE SYSTEM SHALL assign a new stable ID (and MUST NOT reuse an existing stable ID).
- THE SYSTEM SHALL preserve `FlowEdge.style` across save/load via the diagram sidecar.
- THE SYSTEM SHALL remain backwards compatible with sessions missing sidecars (load succeeds; IDs are generated as today).

