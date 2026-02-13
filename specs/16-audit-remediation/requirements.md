# Requirements — 16-audit-remediation

This spec closes the remaining issues listed in `docs/audit.md` and brings `cargo clippy --offline` to zero warnings.

## Requirements (EARS)

- THE SYSTEM SHALL keep `cargo test --offline` green while implementing audit fixes.
- THE SYSTEM SHALL reduce `cargo clippy --offline` to **zero warnings**.
- WHEN saving a session folder THE SYSTEM SHALL not write outside the session root, even if symlinks exist under the session directory.
- WHEN saving a session folder THE SYSTEM SHALL write JSON and exported artifacts atomically (temp-write + rename).
- WHEN loading a session folder THE SYSTEM SHALL restore persisted session state (diagram revs, active ids, xrefs) without loss.
- WHEN a walkthrough is removed and the session is saved THE SYSTEM SHALL not resurrect the removed walkthrough on subsequent loads.
- WHEN loading walkthroughs from disk THE SYSTEM SHALL restore `Walkthrough.rev` in O(1) time and reject/cap pathological `rev` values.
- WHEN persisting diagrams and walkthroughs THE SYSTEM SHALL use OS-portable file names while keeping model/protocol IDs unchanged.
- WHEN applying sequence ops THE SYSTEM SHALL reject messages that reference missing participants.
- WHEN syncing via MCP delta polling THE SYSTEM SHALL provide a recoverable sync path (snapshot + bounded delta history).
- WHEN routing flowchart edges THE SYSTEM SHALL not panic and SHALL provide deterministic fallback behavior when routing fails.
- WHERE performance hot paths are identified (routing, TUI, session routing) THE SYSTEM SHALL avoid avoidable allocations/recomputation while preserving deterministic results.

