# Requirements — 02-dependencies

This spec owns dependency and feature-flag decisions in `Cargo.toml`, so other specs can be worked in parallel without conflicting edits to the crate manifest.

Mayor-only protocol reference: `docs/protocol-01.md` (workers must not be sent to this doc; extract needed excerpts into task `Context:` blocks)

## Requirements (EARS)

- THE SYSTEM SHALL build with deterministic dependency versions recorded in `Cargo.lock`.
- WHEN dependencies are added THE SYSTEM SHALL keep `cargo test` green.
- THE SYSTEM SHALL avoid adding dependencies unless they are required by a spec and have a clear validation path.
