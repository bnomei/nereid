# 09 - Route From Node To Edge

## Metadata
- `id`: `PB-09`
- `goal`: verify shortest route computation between a flow node and a flow edge object.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`Find the shortest route from d:demo-flow/flow/node/n:a to d:demo-flow/flow/edge/e:cd. Return the full route as object_refs in order.`

## Expected Tool Calls
### Required (order matters)
1. `route_find`
   - matcher: `from_ref` `equals` `d:demo-flow/flow/node/n:a`
   - matcher: `to_ref` `equals` `d:demo-flow/flow/edge/e:cd`
   - matcher: `ordering` `equals` `fewest_hops`
   - matcher: `limit` `equals` `1`

### Optional (acceptable alternatives)
- `diagram_get_ast` for validation.

### Forbidden
- `diagram_apply_ops`
- `diagram_propose_ops`
- `diagram_create_from_mermaid`
- `selection_update`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must return the shortest route:
  - `d:demo-flow/flow/node/n:a`
  - `d:demo-flow/flow/node/n:c`
  - `d:demo-flow/flow/edge/e:cd`

## Pass/Fail Checklist
- [ ] `route_find` used fewest-hops ordering with the exact refs.
- [ ] Output route matches the shortest path above.
- [ ] No forbidden mutating calls were made.

## Notes
- Any longer path should be treated as a failure.
