# Design — 09-session-store

Keep persistence logic in `src/store/`.

Design goals:
- File layout matches `docs/protocol-01.md` exactly.
- Metadata paths are relative to the session folder.
- Start with “new IDs only” reconciliation if needed; add structural matching later.

This spec will likely require a serialization dependency (owned by `02-dependencies`).

