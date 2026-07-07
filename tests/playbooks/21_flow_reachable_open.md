# 21 - Open And Reachable List

## Metadata
- `id`: `PB-21`
- `goal`: require `diagram_open` and `flow_reachable` on a discovered diagram.
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
1. `diagram_list`
2. `diagram_open`
   - matcher: `diagram_id` `equals` `demo-t-flow-routing`
3. `flow_reachable`
   - matcher: `diagram_id` `equals` `demo-t-flow-routing`
   - matcher: `from_node_id` `equals` `n:start`

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
- [ ] `diagram_open` and `flow_reachable` were called with expected params.
- [ ] Output includes every reachable node listed above.
- [ ] No forbidden mutating calls were made.

## Notes
- This playbook forces diagram discovery before open; the prompt does not name the diagram id.
