# 01 - Index Navigation To Marlin Sequence

## Metadata
- `id`: `PB-01`
- `goal`: verify that the AI can resolve a navigation xref from the demo index to the marlin sequence target.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `basic`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Ensure no prior prompt context is reused.

## User Prompt
`From the demo index, the story node for the marlin fight — where does its nav link go? Return target diagram_id and target object_ref.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `diagram.get_ast`
   - matcher: `diagram_id` `equals` `demo-00-index`
3. `xref.list`
   - matcher: `from_ref` `equals` `d:demo-00-index/flow/node/n:om_marlin`
   - acceptable alternative: `involves_ref` `equals` `d:demo-00-index/flow/node/n:om_marlin`

### Optional (acceptable alternatives)
- `diagram.list` for orientation.
- `object.read` on the discovered target object.

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must include `om-11-marlin` as target diagram.
- Must include `d:om-11-marlin/seq/participant/p:marlin` as target object ref.
- Should mention it is a nav/xref-style jump from the index node.

## Pass/Fail Checklist
- [ ] Required call was made with the expected reference.
- [ ] No forbidden mutating calls were made.
- [ ] Final answer includes the exact expected diagram id and object ref.
- [ ] No hallucinated IDs or tool names appear.

## Notes
- Wording can vary; IDs and refs are strict.
- If the AI uses `involves_ref` instead of `from_ref`, treat as pass.
