DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: open
Location: src/mcp/server/diagram.rs:974-1021,250-251 | Slug: diagram-apply-ops-stale-xref

# diagram.apply_ops does not refresh xref status after removing endpoints

## Finding

`diagram.delete` calls `refresh_xref_statuses` before save (`250-251`), but `diagram.apply_ops` persistent and non-persistent paths never refresh xref status after ops that remove referenced objects.

## Violated Invariant Or Contract

`xref.list` / `XRefSummary.status` must reflect whether endpoints still exist, consistent with post-delete behavior.

## Oracle

`diagram.delete` test expects refreshed xref status (`tests.rs` ~2806). `sync_state_with_session_folder` refreshes on disk load (`server.rs:212-213`). `xref.add` computes status only at creation.

## Counterexample

Xref from `d:d-flow/flow/node/n:a` to a sequence participant. `diagram.apply_ops` with `FlowRemoveNode { node_id: "n:a" }`. `xref.list` still reports `status: "ok"` while `object_ref_is_missing(xref.from()) == true`.

## Why It Might Matter

Agents relying on xref status for navigation or validation act on stale "ok" links after partial diagram edits.

## Proof

Control-flow trace: `diagram_apply_ops` commits via `save_session` (`1016`) without `refresh_xref_statuses`; contrast `diagram_delete` (`250-251`).

## Counterevidence Checked

TUI refreshes xrefs on its own edits (`tui/mod.rs:760-761`). Disk sync on next full reload would refresh, but in-memory MCP session stays stale until then.

## Suggested Next Step

Call `refresh_xref_statuses` on `candidate_session` before `save_session` in `diagram_apply_ops`, matching `diagram.delete`.

DEVANA-KEY: src/mcp/server/diagram.rs:974-1021 | P2 | diagram-apply-ops-stale-xref
DEVANA-SUMMARY: P2 high src/mcp/server/diagram.rs:974-1021 - diagram.apply_ops leaves xref status stale after ops remove referenced objects, unlike diagram.delete.