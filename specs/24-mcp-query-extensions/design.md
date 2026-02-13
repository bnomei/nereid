# Design — 24-mcp-query-extensions

## Context (extracted from `docs/protocol-01.md`)

- MCP tools should be composable and “small context by default” (digests, deltas, slices).
- Session routes are defined over a derived meta-graph (flow edges, sequence adjacency, xrefs).
- `seq.search` is specified as `text|regex`.

## R1 — `xref.list` filters

Implement filters in MCP server handler (do not push this burden onto clients).

Determinism:
- Apply filters, then sort deterministically (e.g. by `xref_id`), then apply `limit`.

## R2 — `session.routes` multi-route semantics

User decision: **k-shortest simple routes** (bounded by `limit` and `max_hops`).

Implementation candidates:
- A path-enumerating BFS with pruning (simpler, but needs a safety cap to avoid path explosion).
- Yen’s k-shortest loopless paths algorithm (more predictable “k-shortest” semantics).

Ordering:
- Add an output ordering param (default: hops/fewest edges).
- Provide deterministic tie-breaking (e.g. lexicographic by `ObjectRef` string).

## R3 — `seq.search` regex + case-insensitive

Add `regex` crate and expose:
- `mode: substring|regex` (default substring)
- `case_insensitive: bool` (default `true`)

In regex mode:
- compile once, then test each message’s text.
- `invalid_params` on compile error.

## R4 — Flow fan-in/fan-out stats

Two viable tool shapes:

### Option A: `flow.degrees`
`flow.degrees(diagram_id?, top?, sort_by?, direction?) -> [{ node_ref, in, out, label? }]`

Pros:
- Single tool covers “fan-in” and “fan-out”.
- Easy to keep bounded (`top`).

Cons:
- Needs careful params to avoid returning “too much” by default.

### Option B: `flow.stats`
`flow.stats(diagram_id?, top?) -> { starts[], terminals[], top_fan_in[], top_fan_out[], branch_nodes[] }`

Pros:
- More directly answers protocol-style questions (“starts”, “terminal nodes”, “main branches”).
- Still bounded by `top` (and fixed-size lists).

Cons:
- Larger response schema; slightly less composable than primitives.

Decision: **Option A (`flow.degrees`)**.
