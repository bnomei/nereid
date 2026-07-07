# 06 - Nav XRefs To Sequence Targets

## Metadata
- `id`: `PB-06`
- `goal`: resolve all nav xrefs that land on sequence diagrams and return their targets.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`From the demo index, list every nav xref that lands in a sequence diagram (not flowchart). Return target diagram_id and target object_ref for each.`

## Expected Tool Calls
### Required (order matters)
1. `diagram_list`
2. `xref_list`
   - matcher: `kind` `equals` `nav`

### Optional (acceptable alternatives)
- `diagram_read` for the index diagram.

### Forbidden
- `diagram_apply_ops`
- `diagram_propose_ops`
- `diagram_create_from_mermaid`
- `selection_update`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must include all five sequence targets:
  - `om-10-dialogue` -> `d:om-10-dialogue/seq/participant/p:manolin`
  - `om-11-marlin` -> `d:om-11-marlin/seq/participant/p:marlin`
  - `om-12-sharks` -> `d:om-12-sharks/seq/participant/p:mako`
  - `demo-seq` -> `d:demo-seq/seq/participant/p:alice`
  - `demo-t-seq-blocks` -> `d:demo-t-seq-blocks/seq/participant/p:client`
- Must not include flowchart targets.

## Pass/Fail Checklist
- [ ] `diagram_list` and `xref_list` were used.
- [ ] All five expected sequence targets are present.
- [ ] No flowchart targets are listed.
- [ ] No forbidden mutating calls were made.

## Notes
- Ordering in the response does not matter.
