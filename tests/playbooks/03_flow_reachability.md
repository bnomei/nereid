# 03 - Flow Reachability And Path

## Metadata
- `id`: `PB-03`
- `goal`: verify that the AI can answer a reachability question with a concrete node-ref path.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `intermediate`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Run in a clean prompt context.

## User Prompt
`In the routing demo with crossings, can Start reach Done? Return yes/no and one shortest path as node refs.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `flow.paths`
   - matcher: `diagram_id` `equals` `demo-t-flow-routing`
   - matcher: `from_node_id` `equals` `n:start`
   - matcher: `to_node_id` `equals` `n:done`
   - matcher: `max_extra_hops` is `0` or omitted

### Optional (acceptable alternatives)
- `flow.reachable` to double-check reachability.
- `diagram.get_ast` for additional validation.

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must clearly state that `n:start` can reach `n:done`.
- Must include a path that starts with `d:demo-t-flow-routing/flow/node/n:start`.
- Must include a path that ends with `d:demo-t-flow-routing/flow/node/n:done`.
- Path must be valid node refs from this diagram.

## Pass/Fail Checklist
- [ ] `flow.paths` call used the expected source/target nodes.
- [ ] Final answer includes a valid reachable conclusion.
- [ ] Final answer includes at least one concrete node-ref path.
- [ ] No forbidden mutating calls were made.

## Notes
- Multiple shortest paths can be valid in this graph; accept any valid shortest path.
- If the AI reports one valid path and a reachable verdict, that is sufficient.
