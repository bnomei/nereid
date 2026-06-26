DEVANA-FINDING: v1
DEVANA-STATE: fixed
Priority: P2 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/query/session_routes.rs:191-196 | Slug: route-adjacency-includes-dangling-xrefs

# Route adjacency adds phantom nodes/edges from dangling xrefs, routing through deleted objects

## Finding

`derive_adjacency` (`src/query/session_routes.rs:191-196`) iterates every xref
and unconditionally inserts bidirectional edges between `xref.from()` and
`xref.to()`, without checking `xref.status()`/`is_dangling()` and without
verifying that the endpoints correspond to objects already materialized from the
diagram ASTs. Because `insert_edge` uses `entry(...).or_default()`
(`:77-83`), a stale endpoint that no longer exists as a real object is
materialized as a traversable graph node.

## Violated Invariant Or Contract

Route adjacency should contain only objects that exist in the session, so a route
returned to the user never traverses a deleted object. The flow/sequence builders
above (`:91-189`) honor this by deriving nodes only from real AST objects.

## Oracle

Neighboring-implementation contract: the AST-driven branches only ever insert
refs for present nodes/edges/participants/messages/blocks/sections. The model
deliberately keeps dangling xrefs and only marks status (`XRefStatus::is_dangling`,
`set_status` used in `tui/mod.rs`, `mcp/server/helpers.rs`,
`store/session_folder/helpers.rs`; the TUI even exposes an `xrefs_dangling_only`
filter), so dangling xrefs are a normal, persisted session state.

## Counterexample

- Flow diagram `d:flow` with a single node `n:a`.
- Stale xref `x:1`: `from = d:flow/flow/node/n:a`,
  `to = d:flow/flow/node/n:gone`, status `DanglingTo` (n:gone was deleted).
- Stale xref `x:2`: `from = d:flow/flow/node/n:gone`,
  `to = d:flow/flow/node/n:also_gone`.
- `find_route(session, d:flow/flow/node/n:a, d:flow/flow/node/n:also_gone)`
  returns `Some([n:a, n:gone, n:also_gone])` — a route through two objects that
  do not exist in the session.

## Why It Might Matter

Route/path queries (UI navigation, MCP route queries) report connections through
deleted objects, presenting stale relationships as live ones and making
"reachable" include non-existent endpoints. Wrong correctness result on normal
persisted state.

## Proof

Dataflow trace: `session.xrefs().values()` → unconditional `insert_edge` for both
directions (`:194-195`) → `or_default()` materializes phantom endpoints
(`:82`) → BFS/route search at `:242-355` traverses them → `reconstruct_path`
returns a path naming deleted objects.

## Counterevidence Checked

Searched all of `src/query/` for any `status`/`is_dangling`/existence filtering
inside route derivation — none exists; only test constructors set
`XRefStatus::Ok`. `is_dangling` is defined and used elsewhere but never in
`derive_adjacency`. The AST branches cannot reintroduce the deleted node, so the
phantom edge is the sole source of the bad route.

## Suggested Next Step

In `derive_adjacency`, skip xrefs whose status is dangling/unresolved, or only
insert an xref edge when both endpoints already exist in `adjacency`.

## Status Notes

2026-06-26: Marked fixed after static validation. `derive_adjacency` now materializes AST objects first and inserts xref bridge edges only when both endpoints already exist in the adjacency map. Because `insert_edge` is no longer called for missing endpoints, dangling xrefs cannot resurrect deleted objects as traversable route nodes. A stale dangling status on still-existing endpoints may still route, but that does not recreate the reported deleted-object path.

DEVANA-KEY: src/query/session_routes.rs:191-196 | P2 | route-adjacency-includes-dangling-xrefs
DEVANA-SUMMARY: fixed P2 high src/query/session_routes.rs:191-196 - Route derivation adds edges for dangling xrefs, so route queries traverse deleted objects.
