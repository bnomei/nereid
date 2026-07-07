# 23 - XRef Neighbors From Motif Node

## Metadata
- `id`: `PB-23`
- `goal`: require `xref_neighbors` using a human prompt and return outbound neighbors.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`From the DiMaggio motif node, list all outbound xref neighbors as object refs.`

## Expected Tool Calls
### Required (order matters)
1. `diagram_list`
2. `diagram_get_ast`
   - matcher: `diagram_id` `equals` `om-20-motifs`
3. `xref_neighbors`
   - matcher: `object_ref` `equals` `d:om-20-motifs/flow/node/n:di_maggio`
   - matcher: `direction` `equals` `out`

### Optional (acceptable alternatives)
- `xref_list` filtered by `from_ref`.

### Forbidden
- `diagram_apply_ops`
- `diagram_propose_ops`
- `diagram_create_from_mermaid`
- `selection_update`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must include `d:om-10-dialogue/seq/message/m:dimaggio`.
- Must include `d:om-06-baseball/seq/message/m:dimaggio_diff`.

## Pass/Fail Checklist
- [ ] `xref_neighbors` was called with the correct object_ref and direction.
- [ ] Output includes both expected neighbor refs.
- [ ] No forbidden mutating calls were made.

## Notes
- The motif node label is human-facing; the agent must resolve it via AST before calling neighbors.
