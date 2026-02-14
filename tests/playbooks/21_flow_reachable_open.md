# 21 - Open And Reachable List

## Metadata
- `id`: `PB-21`
- `goal`: require `diagram.open` and `flow.reachable` on a discovered diagram.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`Switch to the routing demo with crossings, then list every node reachable from Start.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `diagram.open`
   - matcher: `diagram_id` `equals` `demo-t-flow-routing`
3. `flow.reachable`
   - matcher: `diagram_id` `equals` `demo-t-flow-routing`
   - matcher: `from_node_id` `equals` `n:start`

### Optional (acceptable alternatives)
- `diagram.get_ast` for validation.

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must include all reachable node refs:
  - `d:demo-t-flow-routing/flow/node/n:start`
  - `d:demo-t-flow-routing/flow/node/n:ingest`
  - `d:demo-t-flow-routing/flow/node/n:opts`
  - `d:demo-t-flow-routing/flow/node/n:parse`
  - `d:demo-t-flow-routing/flow/node/n:ast`
  - `d:demo-t-flow-routing/flow/node/n:diag`
  - `d:demo-t-flow-routing/flow/node/n:analyze`
  - `d:demo-t-flow-routing/flow/node/n:plan`
  - `d:demo-t-flow-routing/flow/node/n:exec`
  - `d:demo-t-flow-routing/flow/node/n:render`
  - `d:demo-t-flow-routing/flow/node/n:out`
  - `d:demo-t-flow-routing/flow/node/n:metrics`
  - `d:demo-t-flow-routing/flow/node/n:done`

## Pass/Fail Checklist
- [ ] `diagram.open` and `flow.reachable` were called with expected params.
- [ ] Output includes every reachable node listed above.
- [ ] No forbidden mutating calls were made.

## Notes
- This playbook forces diagram discovery before open; the prompt does not name the diagram id.
