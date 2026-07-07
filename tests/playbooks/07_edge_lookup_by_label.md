# 07 - Edge Lookup By Label

## Metadata
- `id`: `PB-07`
- `goal`: resolve a labeled flow edge via AST, returning its object_ref and endpoints.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`In the flowchart demo with alpha/beta edges, find the edge labeled "beta" and return its edge object_ref plus from/to node refs.`

## Expected Tool Calls
### Required (order matters)
1. `diagram_list`
2. `diagram_get_ast`
   - matcher: `diagram_id` `equals` `demo-flow`

### Optional (acceptable alternatives)
- `object_read` for the resolved edge ref.
- `diagram_read` for additional confirmation.

### Forbidden
- `diagram_apply_ops`
- `diagram_propose_ops`
- `diagram_create_from_mermaid`
- `selection_update`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must include edge ref `d:demo-flow/flow/edge/e:ac`.
- Must include from node ref `d:demo-flow/flow/node/n:a`.
- Must include to node ref `d:demo-flow/flow/node/n:c`.

## Pass/Fail Checklist
- [ ] `diagram_get_ast` was used to resolve the labeled edge.
- [ ] Output includes the exact edge ref and both endpoint refs.
- [ ] No forbidden mutating calls were made.

## Notes
- This edge ID is not present in the Mermaid source; it must be read from the AST.
