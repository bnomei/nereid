# 11 - Note-Driven Nav And Shortest Path

## Metadata
- `id`: `PB-11`
- `goal`: resolve a node via hidden note, follow its nav xref, then compute a shortest path.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`On the demo index, find the node whose note says "routing + tees". Use that node to follow its nav xref to the target diagram. Then answer: can n:start reach n:done, and return one shortest path as node refs. Include the source node object_ref and the target diagram_id.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `diagram.get_ast`
   - matcher: `diagram_id` `equals` `demo-00-index`
3. `xref.list`
   - matcher: `from_ref` `equals` `d:demo-00-index/flow/node/n:flow_route`
   - matcher: `kind` `equals` `nav`
4. `flow.paths`
   - matcher: `diagram_id` `equals` `demo-t-flow-routing`
   - matcher: `from_node_id` `equals` `n:start`
   - matcher: `to_node_id` `equals` `n:done`
   - matcher: `max_extra_hops` `equals` `0`

### Optional (acceptable alternatives)
 

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must include source node ref `d:demo-00-index/flow/node/n:flow_route`.
- Must include target diagram id `demo-t-flow-routing`.
- Must state that `n:start` can reach `n:done`.
- Must include one shortest path, and it must be one of:
  - `d:demo-t-flow-routing/flow/node/n:start`
  - `d:demo-t-flow-routing/flow/node/n:opts`
  - `d:demo-t-flow-routing/flow/node/n:plan`
  - `d:demo-t-flow-routing/flow/node/n:exec`
  - `d:demo-t-flow-routing/flow/node/n:render`
  - `d:demo-t-flow-routing/flow/node/n:out`
  - `d:demo-t-flow-routing/flow/node/n:done`
  - or
  - `d:demo-t-flow-routing/flow/node/n:start`
  - `d:demo-t-flow-routing/flow/node/n:opts`
  - `d:demo-t-flow-routing/flow/node/n:plan`
  - `d:demo-t-flow-routing/flow/node/n:exec`
  - `d:demo-t-flow-routing/flow/node/n:render`
  - `d:demo-t-flow-routing/flow/node/n:metrics`
  - `d:demo-t-flow-routing/flow/node/n:done`

## Pass/Fail Checklist
- [ ] `diagram.get_ast` was used to resolve the note-only node.
- [ ] `xref.list` and `flow.paths` were called with the expected params.
- [ ] Output includes the node ref, target diagram id, and a valid shortest path.
- [ ] No forbidden mutating calls were made.

## Notes
- The routing note is not present in Mermaid; it must be read from the AST.
