# 02 - Sequence Lookup For DiMaggio Quote

## Metadata
- `id`: `PB-02`
- `goal`: verify that the AI can locate a specific sequence message and read it before answering.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `basic`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh conversation for this prompt.

## User Prompt
`In the terrace dialogue where the boy mentions the Yankees, find the DiMaggio line and return the message object_ref plus exact text.`

## Expected Tool Calls
### Required (order matters)
1. `diagram_list`
2. `seq_search`
   - matcher: `diagram_id` `equals` `om-10-dialogue`
   - matcher: `needle` `contains` `DiMaggio`
3. `object_read`
   - matcher: reads at least one object ref returned by `seq_search`

### Optional (acceptable alternatives)
- `diagram_read` for additional context.
- `seq_messages` before `seq_search`.

### Forbidden
- `diagram_apply_ops`
- `diagram_propose_ops`
- `diagram_create_from_mermaid`
- `selection_update`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must include an object ref starting with `d:om-10-dialogue/seq/message/`.
- Must include the message text `Think of DiMaggio`.
- Must attribute the quote to the correct diagram (`om-10-dialogue`).

## Pass/Fail Checklist
- [ ] `seq_search` was used for discovery.
- [ ] `object_read` was used to verify message content.
- [ ] Final answer includes both object ref and exact message text.
- [ ] No forbidden mutating calls were made.

## Notes
- Minor punctuation differences in the explanation are fine.
- If multiple hits are returned, any answer containing the correct DiMaggio quote passes.
