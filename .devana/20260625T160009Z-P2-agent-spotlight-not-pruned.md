DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: open
Location: src/mcp/server/diagram.rs:974-1081, src/mcp/server/collaboration.rs:36-54 | Slug: agent-spotlight-not-pruned

# Agent spotlight not cleared when apply_ops removes the target object

## Finding

`agent_highlights` is pruned only on `diagram.delete` (`diagram.rs:276-277`), not after `diagram.apply_ops` removes individual objects. `attention.agent.read` returns the stored ref without validating it still exists.

## Violated Invariant Or Contract

`attention.agent.read` should not return refs for missing objects; `attention.agent.set` validates existence on write (`collaboration.rs:67-72`).

## Oracle

`diagram.delete` test expects agent attention cleared when deleted diagram held spotlight (`tests.rs` ~2802-2804). `sync_state_with_session_folder` does not prune `agent_highlights` on disk reload (`server.rs:204-235`).

## Counterexample

1. `attention.agent.set` on flow node `n:a`. 2. `diagram.apply_ops` removes `n:a` (diagram kept). 3. `attention.agent.read` still returns that ref. 4. `object_ref_is_missing(session, ref) == true`; a new `attention.agent.set` on the same ref would fail with `resource_not_found`.

## Why It Might Matter

Agents and follow-AI TUI mode can target removed objects; stale spotlight persists until manual clear or whole-diagram delete.

## Proof

State lifecycle mismatch: `agent_highlights` outlives removed objects because only `diagram.delete` calls `agent_highlights.retain`; `diagram_apply_ops` has no analogous prune.

## Counterevidence Checked

`follow_agent_highlight` skips jump when object missing (`tui/mod.rs:777-778`), but spotlight remains in `agent_highlights` set. `attention.agent.set` validates on write only.

## Suggested Next Step

After successful `diagram.apply_ops`, prune `agent_highlights` refs that `object_ref_is_missing` reports; mirror `diagram.delete` retention logic.

DEVANA-KEY: src/mcp/server/diagram.rs:974-1081 | P2 | agent-spotlight-not-pruned
DEVANA-SUMMARY: P2 high src/mcp/server/diagram.rs:974-1081 - diagram.apply_ops leaves agent_highlights pointing at removed objects; attention.agent.read returns stale refs.