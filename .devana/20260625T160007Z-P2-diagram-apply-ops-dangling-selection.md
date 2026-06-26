DEVANA-FINDING: v1
DEVANA-STATE: fixed
Priority: P2 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/mcp/server/diagram.rs:1008-1016,250 | Slug: diagram-apply-ops-dangling-selection

# diagram.apply_ops persists dangling selected_object_refs after removals

## Finding

The persistent `diagram.apply_ops` path loads meta selection and saves the session without calling `retain_existing_selected_object_refs`, unlike `diagram.delete` which prunes before save.

## Violated Invariant Or Contract

Persisted `selected_object_refs` should only contain refs where `session.object_ref_exists` holds.

## Oracle

`diagram.delete` calls `retain_existing_selected_object_refs` (`250`). `selection.read`/`selection.update` prune in memory (`collaboration.rs:133-134`, `199`). `sync_state_with_session_folder` prunes on disk load (`server.rs:212`).

## Counterexample

1. `selection.update` selects `d:d-flow/flow/node/n:a`. 2. `diagram.apply_ops` removes `n:a`. 3. Persistent save reloads meta selection and writes it back without pruning (`1014-1016`). 4. On-disk meta still lists the removed ref until another prune path runs.

## Why It Might Matter

Stale selection in persisted meta survives restarts and can confuse TUI reload, collaboration tools, and agents reading selection from disk-backed meta.

## Proof

Cross-entry mismatch: `diagram_delete` prunes selection before save; `diagram_apply_ops` loads meta selection verbatim and saves without `retain_existing_selected_object_refs`.

## Counterevidence Checked

`selection.read` filters in-memory for display. `sync_state_with_session_folder` prunes on next disk sync, but apply path itself writes dangling refs to meta.

## Suggested Next Step

Call `retain_existing_selected_object_refs(&mut candidate_session)` before `save_session` in `diagram_apply_ops`.

## Status Notes

2026-06-26: Marked fixed after static validation. Persistent and in-memory `diagram.apply_ops` branches now call `retain_existing_selected_object_refs` after object-removing ops and before save/state exposure. The helper retains only refs for which `session.object_ref_exists` is true, blocking the persisted dangling-selection counterexample.

DEVANA-KEY: src/mcp/server/diagram.rs:1008-1016 | P2 | diagram-apply-ops-dangling-selection
DEVANA-SUMMARY: fixed P2 high src/mcp/server/diagram.rs:1008-1016 - diagram.apply_ops saves meta selection without pruning refs to removed objects.
