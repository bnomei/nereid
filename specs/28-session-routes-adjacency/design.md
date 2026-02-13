# Design — 28-session-routes-adjacency

## Current state

`src/query/session_routes.rs` derives an adjacency graph over `ObjectRef` nodes:
- Flowcharts: nodes are `flow/node/*`, edges are flow edge direction (`from -> to`).
- Sequences: nodes are `seq/message/*`, edges are chronological adjacency (`m[i] -> m[i+1]`).
- XRefs are added bidirectionally.

Valid `ObjectRef` categories also include `flow/edge/*` and `seq/participant/*`, but these are currently absent from the adjacency graph, so endpoints of those kinds can be unreachable even within the same diagram.

## Proposed adjacency expansion

### Flowcharts

Keep existing `flow/node -> flow/node` edges (directed).

Add:
- nodes for each `flow/edge/<edge_id>`
- structural adjacency between an edge and its endpoints

Direction choice:
- Recommended: add edge↔node adjacency *bidirectionally* so routes can start/end at either kind.
- Note: this introduces reverse connectivity between nodes via `node <- edge -> node`. If that is undesirable, restrict the structural edges (e.g. only `from_node -> edge` and `edge -> to_node`) and add explicit edges for “edge-to-endpoints” without enabling “node-to-edge” in the reverse direction.

Decision:
- Use bidirectional structural edges (edge↔endpoint nodes, participant↔messages). This optimizes for navigation/use as a meta-graph, even though it introduces reverse connectivity between `flow/node`s via `flow/edge` nodes.

### Sequences

Keep existing `seq/message -> seq/message` chronological adjacency (directed).

Add:
- nodes for each `seq/participant/<participant_id>`
- structural adjacency between participants and messages they send/receive

Recommended: add participant↔message adjacency bidirectionally so routes can start/end at either kind.

### XRefs

Keep existing behavior: bidirectional adjacency between xref endpoints.

## Validation

- Add unit tests for:
  - a route from a `flow/edge` to its `flow/node` endpoints
  - a route from a `seq/participant` to a related `seq/message` (and vice versa)
  - a cross-diagram route involving an endpoint that is a `flow/edge` or `seq/participant`
- Ensure the existing cross-diagram route test stays unchanged (same returned path).
