DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: open
Location: src/tui/mod.rs:1467-1482,723-724 | Slug: tui-pending-sync-dropped

# TUI drops pending diagram sync after a single failed flush

## Finding

`flush_pending_diagram_sync` unconditionally `take()`s `pending_diagram_sync` before attempting persistence. On any failure it shows an error toast but never restores pending state, so auto-sync for that edit is abandoned after one attempt.

## Violated Invariant Or Contract

A diagram edit marked sync-pending must keep retrying until persisted or explicitly abandoned; `pending_diagram_sync` is the only auto-persist path for external editor edits when `session_folder` is set.

## Oracle

`apply_edited_mermaid_to_diagram` sets `pending_diagram_sync` with toast "sync pending". Main loop calls `flush_pending_diagram_sync` every tick. `sync_from_ui_state` only reloads from disk when `pending_diagram_sync.is_none()` (`723-724`).

## Counterexample

1. User edits active diagram via external editor; `pending_diagram_sync = Some({ expected_disk_rev: 1 })`. 2. First flush: `take()` clears pending; `persist_pending_diagram_sync` fails (disk full, rev conflict, diagram removed). 3. Error toast shown; pending stays `None`. 4. MCP concurrently bumps `session_rev`; next tick `sync_from_ui_state` reloads disk and replaces `self.session`, reverting the unsynced in-memory edit.

## Why It Might Matter

Transient I/O or rev conflicts cause permanent loss of user edits in the default TUI+MCP concurrent workflow.

## Proof

Control-flow trace: `take()` at `1468` before `match`; error arm at `1479-1481` never restores pending; no other retry path in the main loop.

## Counterevidence Checked

`persist_pending_diagram_sync` rev guard (`1503-1509`) prevents bad write but does not retain pending. Tests cover happy-path apply without `session_folder`; no flush-failure or reload-after-failure coverage.

## Suggested Next Step

Restore `pending_diagram_sync` on retriable errors; keep blocking disk reload while sync is pending.

DEVANA-KEY: src/tui/mod.rs:1467-1482 | P1 | tui-pending-sync-dropped
DEVANA-SUMMARY: P1 high src/tui/mod.rs:1467-1482 - Failed diagram sync flush permanently clears pending state, allowing MCP reload to clobber unsynced TUI edits.