DEVANA-FINDING: v1
Priority: P3 | Confidence: high | Security-sensitive: no | Status: open
Location: src/query/session_routes.rs:254-256,318-320 | Slug: find-route-self-fabricates-ghost

# find_routes/find_route fabricate a single-node route for an unknown id when from == to

## Finding

`find_routes_with_adjacency` (`src/query/session_routes.rs:254-256`) and
`find_route_with_adjacency` (`:318-320`) short-circuit on `from == to` and return
a path of `[from]` *before* consulting the adjacency map. There is no
`adjacency.contains_key(from)` guard, so an id that does not exist anywhere in the
session yields a fabricated success path naming that non-existent object.

## Violated Invariant Or Contract

A route query whose endpoint does not exist in the session must return "no route"
(`None` / empty), not a success containing a fabricated object.

## Oracle

The sibling implementation `flow::paths` (`src/query/flow.rs:184-190`) guards the
`from == to` case with `outgoing.contains_key(from)` first, so it correctly
rejects unknown ids. That establishes the intended contract; `session_routes`
diverges from it.

## Counterexample

Empty session (no diagrams, no xrefs). `let x: ObjectRef =
"d:none/none/node/n:ghost".parse().unwrap();`
- `find_route(&session, &x, &x)` returns `Some([n:ghost])`.
- `find_routes(&session, &x, &x, 1, None, _)` returns `[[n:ghost]]`.

## Why It Might Matter

Callers receive a "route exists" answer for an object that is not in the session,
fabricating a relationship out of an invalid query. Low impact (self-to-self,
unknown id) but a concrete wrong result.

## Proof

Control-flow trace: both entry points hit the `from == to` branch at
`:254-256` / `:318-320` and return `vec![from.clone()]` / `Some(vec![from])`
unconditionally; `adjacency` is never queried for membership of `from`.

## Counterevidence Checked

`find_route`/`find_routes` derive adjacency internally and are the public entry
points; no caller pre-validates ids. Distinct from the filed
`flow-reachable-missing-node` (different file `src/query/flow.rs`, different
behavior: that returns empty success; this fabricates a node). The `flow::paths`
analog shows the containment guard is the intended contract.

## Suggested Next Step

Return empty/`None` when `from == to` but `!adjacency.contains_key(from)`,
mirroring `flow::paths`.

DEVANA-KEY: src/query/session_routes.rs:254-256 | P3 | find-route-self-fabricates-ghost
DEVANA-SUMMARY: P3 high src/query/session_routes.rs:254-256 - Self-to-self route query for a non-existent id fabricates a single-node success path.
