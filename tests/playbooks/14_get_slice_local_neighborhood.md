# 14 - Local Neighborhood Slice

## Metadata
- `id`: `PB-14`
- `goal`: validate `diagram.get_slice` on a flow node.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`In the flowchart demo with alpha/beta labels, give me the local neighborhood (radius 1) around the node labeled "A", including node refs and edge refs.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `diagram.get_ast`
   - matcher: `diagram_id` `equals` `demo-flow`
3. `diagram.get_slice`
   - matcher: `diagram_id` `equals` `demo-flow`
   - matcher: `center_ref` `equals` `d:demo-flow/flow/node/n:a`
   - matcher: `radius` `equals` `1`

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
- Must include node ref `d:demo-flow/flow/node/n:a`.
- Must include node ref `d:demo-flow/flow/node/n:b`.
- Must include node ref `d:demo-flow/flow/node/n:c`.
- Must include edge ref `d:demo-flow/flow/edge/e:ab`.
- Must include edge ref `d:demo-flow/flow/edge/e:ac`.
- Must not include edge refs `d:demo-flow/flow/edge/e:bd` or `d:demo-flow/flow/edge/e:cd`.

## Pass/Fail Checklist
- [ ] `diagram.get_slice` used the expected center ref and radius.
- [ ] Output includes the expected nodes and edges.
- [ ] Output excludes edges outside the radius.
- [ ] No forbidden mutating calls were made.

## Notes
- The slice is computed from AST adjacency, not Mermaid text scanning.
