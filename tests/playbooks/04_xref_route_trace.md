# 04 - XRef Route To Baseball Quote

## Metadata
- `id`: `PB-04`
- `goal`: verify route traversal across diagrams and final object read for evidence.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `intermediate`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`From the DiMaggio motif in the motifs diagram, find the baseball quote that says "makes the difference" and read it. Return the route and the quote.`

## Expected Tool Calls
### Required (order matters)
1. `diagram_list`
2. `diagram_get_ast`
   - matcher: `diagram_id` `equals` `om-20-motifs`
3. `seq_search`
   - matcher: `diagram_id` `equals` `om-06-baseball`
   - matcher: `needle` `contains` `difference`
4. `route_find`
   - matcher: `from_ref` `equals` `d:om-20-motifs/flow/node/n:di_maggio`
   - matcher: `to_ref` `equals` `d:om-06-baseball/seq/message/m:dimaggio_diff`
5. `object_read`
   - matcher: includes `d:om-06-baseball/seq/message/m:dimaggio_diff`

### Optional (acceptable alternatives)
- `xref_neighbors` as a pre-check.
- `xref_list` filtered by `from_ref` or `involves_ref`.

### Forbidden
- `diagram_apply_ops`
- `diagram_propose_ops`
- `diagram_create_from_mermaid`
- `selection_update`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must include a route that starts at `d:om-20-motifs/flow/node/n:di_maggio`.
- Must include destination `d:om-06-baseball/seq/message/m:dimaggio_diff`.
- Must quote the final message text `DiMaggio makes the difference`.

## Pass/Fail Checklist
- [ ] `route_find` used the exact start and destination refs.
- [ ] `object_read` was used to retrieve destination object details.
- [ ] Final answer includes route evidence and the exact quote text.
- [ ] No forbidden mutating calls were made.

## Notes
- Route can be direct (single hop via xref) or longer if the AI expands context.
- Exact quote text should match object payload, not paraphrase only.
