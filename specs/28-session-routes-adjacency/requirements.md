# Requirements — 28-session-routes-adjacency

`session.routes` and `session.find_route` rely on `crate::query::session_routes::SessionRouteAdjacency`. Today, derived adjacency only models:

- flowcharts: `flow/node` objects connected by flow edge direction
- sequences: `seq/message` objects connected by chronological adjacency
- plus bidirectional xrefs

As a result, routes that start/end at `seq/participant` or `flow/edge` can appear “missing” unless xrefs connect them.

This spec expands derived adjacency to cover the remaining diagram object categories and their structural relationships, while preserving determinism and existing route behavior.

## Requirements (EARS)

- THE SYSTEM SHALL include `seq/participant` and `flow/edge` objects in the derived session adjacency graph.
- THE SYSTEM SHALL connect:
  - each `flow/edge` to its endpoint `flow/node`s
  - each `seq/participant` to the `seq/message`s it sends/receives
  - (existing) `seq/message` chronological adjacency
  - (existing) `flow/node` adjacency following edge directions
  - (existing) bidirectional xref adjacency
- THE SYSTEM SHALL keep route discovery deterministic for identical inputs.
- THE SYSTEM SHALL keep existing `find_route` unit tests passing unchanged and add new tests covering routes that start/end at `seq/participant` and `flow/edge`.

## Non-goals

- This spec does not implement multi-route enumeration (`session.routes` returning k routes); see `specs/24-mcp-query-extensions/T002`.

