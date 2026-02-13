# Requirements — 03-model-core

This spec defines the core in-memory model types that back the protocol (`Session`, `Diagram`, ASTs, XRefs, Walkthroughs).

Normative protocol reference: `docs/protocol-01.md`

## Requirements (EARS)

- THE SYSTEM SHALL represent work as a `Session` containing multiple diagrams, walkthroughs, and xrefs.
- THE SYSTEM SHALL assign stable IDs to all addressable objects and expose canonical `ObjectRef`s.
- THE SYSTEM SHALL track per-diagram revisions (`rev`) that increment on each applied mutation.

