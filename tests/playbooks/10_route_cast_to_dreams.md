# 10 - Route From Cast Lions To Dreams Lions

## Metadata
- `id`: `PB-10`
- `goal`: resolve a multi-hop cross-diagram route using xrefs (not visible in Mermaid source).
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`Starting from the "Lions" node in the cast map, find the shortest route to the Lions participant in the dreams sequence. Return the full route as object_refs in order.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `diagram.get_ast`
   - matcher: `diagram_id` `equals` `om-01-cast`
3. `route.find`
   - matcher: `from_ref` `equals` `d:om-01-cast/flow/node/n:lions`
   - matcher: `to_ref` `equals` `d:om-08-dreams/seq/participant/p:lions`
   - matcher: `ordering` `equals` `fewest_hops`

### Optional (acceptable alternatives)
- `diagram.get_ast` for `om-08-dreams`.
- `xref.neighbors` for intermediate validation.

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must return this route:
  - `d:om-01-cast/flow/node/n:lions`
  - `d:om-20-motifs/flow/node/n:lions`
  - `d:om-08-dreams/seq/participant/p:lions`

## Pass/Fail Checklist
- [ ] `diagram.get_ast` used to resolve the Lions node by label.
- [ ] `route.find` used the exact refs above.
- [ ] Output route matches the expected three-hop sequence.
- [ ] No forbidden mutating calls were made.

## Notes
- This route relies on xrefs and is not derivable from the Mermaid diagrams alone.
