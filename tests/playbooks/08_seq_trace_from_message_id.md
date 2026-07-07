# 08 - Sequence Trace From Message ID

## Metadata
- `id`: `PB-08`
- `goal`: trace forward from a known message id and return the next two messages with refs and text.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`In the dialogue where the boy asks to go fishing again, starting from message id m:ask_go, return the next two messages after it (object_ref + text).`

## Expected Tool Calls
### Required (order matters)
1. `diagram_list`
2. `seq_trace`
   - matcher: `diagram_id` `equals` `om-10-dialogue`
   - matcher: `from_message_id` `equals` `m:ask_go`
   - matcher: `direction` `equals` `after`
   - matcher: `limit` `equals` `2`
3. `object_read`
   - matcher: reads both refs returned by `seq_trace`

### Optional (acceptable alternatives)
- `seq_search` to confirm the starting message.

### Forbidden
- `diagram_apply_ops`
- `diagram_propose_ops`
- `diagram_create_from_mermaid`
- `selection_update`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must include `d:om-10-dialogue/seq/message/m:stay` with text `Stay with them`.
- Must include `d:om-10-dialogue/seq/message/m:we_will_go` with text `We will go`.

## Pass/Fail Checklist
- [ ] `seq_trace` used the exact message id and requested limit 2.
- [ ] `object_read` was used to retrieve message text.
- [ ] Output includes both expected message refs and texts.
- [ ] No forbidden mutating calls were made.

## Notes
- The order should match the sequence order in the AST.
