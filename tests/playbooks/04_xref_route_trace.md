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
1. `diagram.list`
2. `diagram.get_ast`
   - matcher: `diagram_id` `equals` `om-20-motifs`
3. `seq.search`
   - matcher: `diagram_id` `equals` `om-06-baseball`
   - matcher: `needle` `contains` `difference`
4. `route.find`
   - matcher: `from_ref` `equals` `d:om-20-motifs/flow/node/n:di_maggio`
   - matcher: `to_ref` `equals` `d:om-06-baseball/seq/message/m:dimaggio_diff`
5. `object.read`
   - matcher: includes `d:om-06-baseball/seq/message/m:dimaggio_diff`

### Optional (acceptable alternatives)
- `xref.neighbors` as a pre-check.
- `xref.list` filtered by `from_ref` or `involves_ref`.

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must include a route that starts at `d:om-20-motifs/flow/node/n:di_maggio`.
- Must include destination `d:om-06-baseball/seq/message/m:dimaggio_diff`.
- Must quote the final message text `DiMaggio makes the difference`.

## Pass/Fail Checklist
- [ ] `route.find` used the exact start and destination refs.
- [ ] `object.read` was used to retrieve destination object details.
- [ ] Final answer includes route evidence and the exact quote text.
- [ ] No forbidden mutating calls were made.

## Notes
- Route can be direct (single hop via xref) or longer if the AI expands context.
- Exact quote text should match object payload, not paraphrase only.
