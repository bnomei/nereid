# 15 - Flow Analytics Bundle

## Metadata
- `id`: `PB-15`
- `goal`: cover flow analytics tools (cycles, dead ends, degrees, unreachable).
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`On the flowchart demo with alpha/beta edges, tell me: cycles (if any), dead-end nodes, the top out-degree node, and unreachable nodes when starting from n:b.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `flow.cycles`
   - matcher: `diagram_id` `equals` `demo-flow`
3. `flow.dead_ends`
   - matcher: `diagram_id` `equals` `demo-flow`
4. `flow.degrees`
   - matcher: `diagram_id` `equals` `demo-flow`
   - matcher: `sort_by` `equals` `out`
   - matcher: `top` `equals` `1`
5. `flow.unreachable`
   - matcher: `diagram_id` `equals` `demo-flow`
   - matcher: `start_node_id` `equals` `n:b`

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
- Must state that cycles are empty.
- Must include dead-end node `d:demo-flow/flow/node/n:d`.
- Must report top out-degree node `d:demo-flow/flow/node/n:a` with out-degree `2`.
- Must include unreachable nodes `d:demo-flow/flow/node/n:a` and `d:demo-flow/flow/node/n:c`.

## Pass/Fail Checklist
- [ ] All four flow analytics tools were called with expected params.
- [ ] Output matches the expected cycle/dead-end/degree/unreachable results.
- [ ] No forbidden mutating calls were made.

## Notes
- This playbook is deterministic for the demo-flow DAG.
