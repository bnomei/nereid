DEVANA-FINDING: v1
DEVANA-STATE: fixed
Priority: P1 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/mcp/server/walkthrough.rs:424-459, src/mcp/server/helpers.rs:572-888 | Slug: walkthrough-ops-partial-commit

# Non-persistent walkthrough.apply_ops commits partial batch on mid-batch failure

## Finding

When MCP runs without a `SessionFolder` (e.g. `--demo --mcp`), `walkthrough.apply_ops` mutates the live `Walkthrough` in place. If a later op in the same batch fails, earlier ops remain applied even though the tool returns an error and `rev` is not bumped.

## Violated Invariant Or Contract

A failed `walkthrough.apply_ops` must leave walkthrough content unchanged, matching the transactional behavior of `diagram.apply_ops` (clone, apply, commit only on full success).

## Oracle

Persistent branch clones `candidate_session` before apply (`walkthrough.rs:336-377`); `diagram.apply_ops` uses `apply_ops` on a cloned diagram AST (`ops/mod.rs:221-258`). Non-persistent walkthrough path applies directly to `state.session`.

## Counterexample

`base_rev: 0`, ops: `[AddNode { node_id: "wn:new", ... }, AddNode { node_id: "wn:new", ... }]`. First op succeeds and pushes `wn:new`; second op returns `"node_id already exists"`. Session now contains `wn:new` at `rev == 0` while the tool reports failure.

## Why It Might Matter

Agents retrying or reconciling from `rev` see an inconsistent state: error response but partial mutation. Demo/in-memory MCP sessions can diverge from agent expectations and from `walkthrough.stat`/`walkthrough.read` contracts.

## Proof

Control-flow trace: `apply_walkthrough_ops` loops over ops mutating `&mut Walkthrough` and returns `Err` mid-loop (`helpers.rs:579-607`). Non-persistent handler calls it on the live walkthrough (`walkthrough.rs:459`) and only calls `bump_rev()` after `?` succeeds (`460-461`). No clone or rollback on error path.

## Counterevidence Checked

Persistent branch clones first; failed apply discards clone. No test covers failed multi-op walkthrough batches in non-persistent mode.

## Suggested Next Step

Mirror the persistent pattern: clone walkthrough (or full session), apply on clone, commit only after all ops succeed.

## Status Notes

2026-06-26: Marked fixed after static validation. The non-persistent `walkthrough.apply_ops` path now clones the walkthrough, applies the batch to the clone, bumps rev, and assigns the clone back only after `apply_walkthrough_ops` succeeds. A mid-batch error can still mutate the candidate clone, but the live walkthrough and rev remain unchanged.

DEVANA-KEY: src/mcp/server/walkthrough.rs:424-459 | P1 | walkthrough-ops-partial-commit
DEVANA-SUMMARY: fixed P1 high src/mcp/server/walkthrough.rs:424-459 - Non-persistent walkthrough.apply_ops leaves partial mutations when a later op in the batch fails.
