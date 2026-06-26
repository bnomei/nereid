DEVANA-FINDING: v1
DEVANA-STATE: fixed
Priority: P3 | Confidence: medium | Security-sensitive: no | Status: fixed
Location: src/mcp/server/queries.rs:338-340 | Slug: flow-reachable-missing-node

# flow.reachable returns empty success for unknown from_node_id

## Finding

`flow.reachable` returns `Ok({ nodes: [] })` when `from_node_id` is not in the diagram, while sibling tools `flow.paths` and `flow.unreachable` return `resource_not_found` for the same missing node.

## Violated Invariant Or Contract

Invalid node IDs on flow query tools should surface consistently. Empty reachable set is indistinguishable from a valid node with no outgoing reachability.

## Oracle

`flow.paths` at `src/mcp/server/queries.rs:412-419` returns `resource_not_found` for missing `from_node_id`. `flow.reachable` early-returns empty at 338-340.

## Counterexample

Call `flow.reachable` with `from_node_id: "n:does-not-exist"` on a valid flowchart → success with `nodes: []`. Call `flow.paths` with same id → `resource_not_found`.

## Why It Might Matter

Agents cannot distinguish "node missing" from "node exists but nothing reachable" without a second tool call, increasing silent mis-navigation on bad refs.

## Proof

**Cross-entry mismatch:** Same `from_node_id` validation semantics differ between `flow.reachable` (empty OK) and `flow.paths` (error).

**Counterexample value:** `from_node_id` syntactically valid `ObjectId` not present in `ast.nodes()`.

## Counterevidence Checked

May be intentional API design for empty-graph ergonomics; no schema documents the distinction. Still inconsistent with sibling flow query handlers.

## Suggested Next Step

Return `resource_not_found` from `flow.reachable` when `from_node_id` is absent, matching `flow.paths`.

## Status Notes

2026-06-26: Marked fixed after static validation. MCP `flow.reachable` now checks `ast.nodes().contains_key(&from_node_id_parsed)` and returns `resource_not_found` when the source node is missing, matching `flow.paths`. The lower-level query helper still returns empty for unknown ids, but the reported MCP tool path is blocked.

DEVANA-KEY: src/mcp/server/queries.rs:338-340 | P3 | flow-reachable-missing-node
DEVANA-SUMMARY: fixed P3 medium src/mcp/server/queries.rs:338-340 - flow.reachable returns empty success for missing from_node_id while flow.paths errors, hiding invalid node refs.
